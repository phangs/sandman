mod db;
mod config;

use tauri::Emitter;
use std::sync::Mutex;
use sqlx::{SqlitePool, FromRow};
use serde::{Deserialize, Serialize};
use config::{Config, load_config, save_config};
use ignore::WalkBuilder;
use sha2::{Sha256, Digest};
use std::io::Read;
mod llm;
use llm::{call_llm, Message};

pub struct AppState {
    pub db: Mutex<Option<SqlitePool>>,
    pub project_path: Mutex<Option<String>>,
}

#[tauri::command]
fn get_config(app: tauri::AppHandle) -> Result<Config, String> {
    Ok(load_config(&app))
}

#[tauri::command]
fn save_global_config(config: Config, app: tauri::AppHandle) -> Result<(), String> {
    save_config(&app, &config)
}

#[tauri::command]
fn update_provider(id: String, active: bool, api_key: Option<String>, app: tauri::AppHandle) -> Result<(), String> {
    let mut config = load_config(&app);
    if let Some(p) = config.providers.iter_mut().find(|pr| pr.id == id) {
        p.active = active;
        if api_key.is_some() {
            p.api_key = api_key;
        }
        save_config(&app, &config)?;
        Ok(())
    } else {
        Err("Provider not found".to_string())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Option<Vec<FileEntry>>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Story {
    pub id: String,
    pub title: String,
    pub status: String,
    pub ai_ready: i32,
    pub agent: Option<String>,
    pub state: Option<String>,
}

#[tauri::command]
async fn get_stories(state: tauri::State<'_, AppState>) -> Result<Vec<Story>, String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    };

    if let Some(pool) = pool {
        let stories = sqlx::query_as::<_, Story>("SELECT id, title, status, ai_ready, agent, state FROM stories")
            .fetch_all(&pool)
            .await
            .map_err(|e| format!("Database error: {}", e))?;
        Ok(stories)
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
async fn create_story(title: String, state: tauri::State<'_, AppState>) -> Result<Story, String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    };

    if let Some(pool) = pool {
        let id_suffix = uuid::Uuid::new_v4().to_string().chars().take(6).collect::<String>();
        let id = format!("S-{}", id_suffix.to_uppercase());
        let status = "Backlog".to_string();
        let ai_ready = 0;

        sqlx::query("INSERT INTO stories (id, title, status, ai_ready) VALUES (?, ?, ?, ?)")
            .bind(&id)
            .bind(&title)
            .bind(&status)
            .bind(ai_ready)
            .execute(&pool)
            .await
            .map_err(|e| format!("Database error: {}", e))?;

        Ok(Story {
            id,
            title,
            status,
            ai_ready,
            agent: None,
            state: None,
        })
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
async fn update_story_status(id: String, status: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    };

    if let Some(pool) = pool {
        sqlx::query("UPDATE stories SET status = ? WHERE id = ?")
            .bind(&status)
            .bind(&id)
            .execute(&pool)
            .await
            .map_err(|e| format!("Database error: {}", e))?;
        Ok(())
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
async fn list_files(path: String) -> Result<Vec<FileEntry>, String> {
    let entries = std::fs::read_dir(&path)
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path_buf = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        
        // Skip hidden files/folders (like .git, .sandman)
        if name.starts_with('.') && name != "." {
            continue;
        }

        files.push(FileEntry {
            name,
            path: path_buf.to_string_lossy().into_owned(),
            is_dir: path_buf.is_dir(),
            children: None,
        });
    }

    // Sort: Directories first, then alphabetical
    files.sort_by(|a, b| {
        if a.is_dir != b.is_dir {
            b.is_dir.cmp(&a.is_dir)
        } else {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
    });

    Ok(files)
}

#[tauri::command]
async fn chat_with_agent(messages: Vec<Message>, app: tauri::AppHandle) -> Result<String, String> {
    call_llm(&app, messages).await
}

#[tauri::command]
async fn start_indexing(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let (pool, path) = {
        let guard_db = state.db.lock().unwrap();
        let guard_path = state.project_path.lock().unwrap();
        (guard_db.clone(), guard_path.clone())
    };

    let pool = pool.ok_or("No project connected")?;
    let path = path.ok_or("No project path set")?;

    let _ = app.emit("log", "\x1b[33m[Indexer]\x1b[0m Starting full project scan...");

    tauri::async_runtime::spawn(async move {
        let mut count = 0;
        let walker = WalkBuilder::new(&path)
            .hidden(false)
            .git_ignore(true)
            .build();

        for entry in walker {
            if let Ok(entry) = entry {
                let file_path = entry.path();
                if file_path.is_file() {
                    let relative_path = file_path.strip_prefix(&path).unwrap_or(file_path);
                    let relative_path_str = relative_path.to_string_lossy().into_owned();

                    // Skip the .sandman directory
                    if relative_path_str.starts_with(".sandman") {
                        continue;
                    }

                    // Compute hash
                    if let Ok(mut file) = std::fs::File::open(file_path) {
                        let mut hasher = Sha256::new();
                        let mut buffer = [0; 4096];
                        while let Ok(n) = file.read(&mut buffer) {
                            if n == 0 { break; }
                            hasher.update(&buffer[..n]);
                        }
                        let hash = format!("{:x}", hasher.finalize());

                        // Update DB
                        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                        let _ = sqlx::query("INSERT OR REPLACE INTO files (path, hash, last_idx_at) VALUES (?, ?, ?)")
                            .bind(&relative_path_str)
                            .bind(&hash)
                            .bind(now as i64)
                            .execute(&pool)
                            .await;

                        count += 1;
                        if count % 10 == 0 {
                            let _ = app.emit("log", format!("\x1b[90m[Indexer]\x1b[0m Scanned {} files...", count));
                        }
                    }
                }
            }
        }

        let _ = app.emit("log", format!("\x1b[32m[Indexer]\x1b[0m Project indexing complete. Total: {} files.", count));
        let _ = app.emit("log", "\x1b[90mReady for RAG-enhanced agent commands.\x1b[0m");
    });

    Ok(())
}

#[tauri::command]
async fn dispatch_agent(id: String, app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;

    // 1. Mark as processing
    sqlx::query("UPDATE stories SET state = 'processing', agent = 'Builder' WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;

    let _ = app.emit("log", format!("\x1b[33m[Agent]\x1b[0m Dispatching Builder to story: {}", id));

    // 2. Fetch the story
    let story: Story = sqlx::query_as("SELECT id, title, status, ai_ready, agent, state FROM stories WHERE id = ?")
        .bind(&id)
        .fetch_one(&pool)
        .await
        .map_err(|e| e.to_string())?;

    // 3. Simple static context for now (RAG indexing was already set up in background)
    let system_prompt = "You are Sandman, an autonomous IDE agent. You must analyze the requirements and provide a high-level implementation strategy. For now, just summarize the task and confirm receipt.";
    let user_msg = format!("Task: {}\nPlease analyze this story and confirm readiness.", story.title);

    let messages = vec![
        Message { role: "system".to_string(), content: system_prompt.to_string() },
        Message { role: "user".to_string(), content: user_msg },
    ];

    let app_clone = app.clone();
    let id_clone = id.clone();
    let pool_clone = pool.clone();

    tauri::async_runtime::spawn(async move {
        match call_llm(&app_clone, messages).await {
            Ok(response) => {
                let _ = app_clone.emit("log", format!("\x1b[32m[Agent]\x1b[0m AI Proposal received: \n{}", response));
                let _ = sqlx::query("UPDATE stories SET state = 'success' WHERE id = ?")
                    .bind(&id_clone)
                    .execute(&pool_clone)
                    .await;
            }
            Err(e) => {
                let _ = app_clone.emit("log", format!("\x1b[31m[Agent]\x1b[0m Agent crash: {}", e));
                let _ = sqlx::query("UPDATE stories SET state = 'failed' WHERE id = ?")
                    .bind(&id_clone)
                    .execute(&pool_clone)
                    .await;
            }
        }
    });

    Ok(())
}

#[tauri::command]
async fn switch_project(path: String, app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<String, String> {
    println!("Switched active project to: {}", path);
    
    // Connect to SQLite db and embed in .sandman/
    let pool = db::init_db(&path).await?;
    *state.db.lock().unwrap() = Some(pool.clone());
    *state.project_path.lock().unwrap() = Some(path.clone());

    // Emit connection logs
    let _ = app.emit("log", format!("\x1b[36m[System]\x1b[0m Switched active project to: {}", path));
    let _ = app.emit("log", format!("\x1b[32m[System]\x1b[0m SQLite Sandbox embedded at: {}/.sandman/sandman.db", path));
    
    // Auto-start indexing
    let _ = start_indexing(app.clone(), state).await;

    Ok(format!("Successfully connected to {}", path))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState { 
            db: Mutex::new(None),
            project_path: Mutex::new(None) 
        })
        .invoke_handler(tauri::generate_handler![
            switch_project,
            get_stories,
            create_story,
            update_story_status,
            list_files,
            get_config,
            save_global_config,
            update_provider,
            start_indexing,
            dispatch_agent,
            chat_with_agent
        ])
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
