mod db;
mod config;

use tauri::{Emitter, Manager};
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
    pub description: Option<String>,
    pub status: String,
    pub ai_ready: i32,
    pub ai_hold: i32,
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
        let stories = sqlx::query_as::<_, Story>("SELECT id, title, description, status, ai_ready, ai_hold, agent, state FROM stories")
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
        let status = "Raw Requirements".to_string();
        let ai_ready = 1; // Always ready for AI when in Raw Requirements

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
            description: None,
            status,
            ai_ready,
            ai_hold: 0,
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
    }.ok_or("Database not initialized")?;

    sqlx::query("UPDATE stories SET status = ? WHERE id = ?")
        .bind(&status)
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn delete_story(id: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("Database not initialized")?;

    sqlx::query("DELETE FROM stories WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
    
    Ok(())
}

#[tauri::command]
async fn toggle_story_hold(id: String, state: tauri::State<'_, AppState>) -> Result<i32, String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("Database not initialized")?;

    let story: Story = sqlx::query_as("SELECT * FROM stories WHERE id = ?")
        .bind(&id)
        .fetch_one(&pool)
        .await
        .map_err(|e| e.to_string())?;

    let new_hold = if story.ai_hold == 1 { 0 } else { 1 };

    sqlx::query("UPDATE stories SET ai_hold = ? WHERE id = ?")
        .bind(new_hold)
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(new_hold)
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

async fn read_file_internal(path: &str, state: &tauri::State<'_, AppState>) -> Result<String, String> {
    let project_path = {
        let guard = state.project_path.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;

    let full_path = std::path::PathBuf::from(&project_path).join(path);
    if !full_path.starts_with(&project_path) {
        return Err("Access denied: Path outside project".to_string());
    }

    std::fs::read_to_string(full_path).map_err(|e| e.to_string())
}

async fn write_file_internal(path: &str, content: &str, state: &tauri::State<'_, AppState>) -> Result<(), String> {
    let project_path = {
        let guard = state.project_path.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;

    let full_path = std::path::PathBuf::from(&project_path).join(path);
    if !full_path.starts_with(&project_path) {
        return Err("Access denied: Path outside project".to_string());
    }

    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    std::fs::write(full_path, content).map_err(|e| e.to_string())
}

async fn run_project_command_internal(command: &str, args: Vec<String>, state: &tauri::State<'_, AppState>) -> Result<String, String> {
    let project_path = {
        let guard = state.project_path.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;

    let output = std::process::Command::new(command)
        .args(args)
        .current_dir(project_path)
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        Err(format!("Command failed: {}\n{}", stdout, stderr))
    }
}

async fn search_code_internal(query: &str, state: &tauri::State<'_, AppState>) -> Result<String, String> {
    let project_path = {
        let guard = state.project_path.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;

    let mut results = Vec::new();
    let walker = WalkBuilder::new(&project_path).build();

    for entry in walker {
        if let Ok(entry) = entry {
            if entry.path().is_file() {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    if content.contains(query) {
                        let rel_path = entry.path().strip_prefix(&project_path).unwrap_or(entry.path());
                        results.push(rel_path.to_string_lossy().into_owned());
                        if results.len() > 10 { break; } // Cap results
                    }
                }
            }
        }
    }
    
    if results.is_empty() {
        Ok("No matches found".to_string())
    } else {
        Ok(format!("Matches found in:\n{}", results.join("\n")))
    }
}

// Helper to parse <tool:NAME>ARGS</tool>
fn parse_tool_call(response: &str) -> Option<(String, String)> {
    if let Some(start_idx) = response.find("<tool:") {
        let rest = &response[start_idx + 6..];
        if let Some(end_name_idx) = rest.find('>') {
            let tool_name = &rest[..end_name_idx];
            let after_name = &rest[end_name_idx + 1..];
            if let Some(end_tag_idx) = after_name.find("</tool>") {
                let args = &after_name[..end_tag_idx];
                return Some((tool_name.to_string(), args.to_string()));
            }
        }
    }
    None
}

#[tauri::command]
async fn read_file(path: String, state: tauri::State<'_, AppState>) -> Result<String, String> {
    read_file_internal(&path, &state).await
}

#[tauri::command]
async fn write_file(path: String, content: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    write_file_internal(&path, &content, &state).await
}

#[tauri::command]
async fn run_project_command(command: String, args: Vec<String>, state: tauri::State<'_, AppState>) -> Result<String, String> {
    run_project_command_internal(&command, args, &state).await
}

#[tauri::command]
async fn search_code(query: String, state: tauri::State<'_, AppState>) -> Result<String, String> {
    search_code_internal(&query, &state).await
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
async fn dispatch_agent(id: String, additional_context: Option<String>, app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    println!("[Agent] Dispatching story: {} with context: {:?}", id, additional_context);
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
    let story: Story = sqlx::query_as("SELECT id, title, description, status, ai_ready, ai_hold, agent, state FROM stories WHERE id = ?")
        .bind(&id)
        .fetch_one(&pool)
        .await
        .map_err(|e| e.to_string())?;

    // 3. Simple static context for now (RAG indexing was already set up in background)
    let (system_prompt, mut user_msg) = if story.status == "Raw Requirements" || story.status == "Clarification Required" {
        let prompt = "You are Sandman, a senior product owner and software architect. Your job is to take raw requirements from a user and polish them into a professional, Jira-style user story.

The output MUST follow this format:
# Title: [Polished Story Title]
# Description: [Clear, detailed description of the feature or task]
# Acceptance Criteria:
- [Criteria 1]
- [Criteria 2]
...

If the requirements are still too vague after reviewing user answers, you may append a 'Clarifying Questions' section at the end. If you have clarifying questions, MOVE the story to 'Clarification Required'. 

CRITICAL: If the user has provided 'ADDITIONAL CONTEXT FROM USER' below, you MUST integrate those answers into a FINAL story. DO NOT repeat the same questions if the user has provided answers for them. Finalize the story into 'To-Do' if the context is sufficient.";
        
        let msg = if additional_context.is_some() {
            format!("Raw Requirement: {}", story.title)
        } else {
            format!("Raw Requirement: {}\n\n{}", story.title, story.description.as_deref().unwrap_or(""))
        };
        (prompt.to_string(), msg)
    } else if story.status == "To Do" {
        let prompt = "You are Sandman, a senior software architect. Analyze this story and provide a high-level implementation strategy. Identify key files to modify and the overall architecture.";
        let msg = format!("Story Title: {}\nDescription: {}", story.title, story.description.as_deref().unwrap_or(""));
        (prompt.to_string(), msg)
    } else if story.status == "In Progress" {
        let prompt = "You are Sandman Builder, a technology-agnostic engineering agent. 
You can implement any application (Web, Mobile, System, AI, etc.) by discovering the project's tech stack first.

AVAILABLE TOOLS:
- <tool:read_file>path</tool>
- <tool:write_file>path | content</tool>
- <tool:run_command>command args</tool> (use this to build/test or install deps)
- <tool:search_code>keyword</tool> (use this to find relevant files and understand existing code)

STRATEGY:
1. Scan for config files (package.json, go.mod, etc.) to understand the environment.
2. Develop the implementation plan.
3. Write the code and verify it.

STORY: {TITLE}\nDESCRIPTION: {DESC}";
        let msg = prompt
            .replace("{TITLE}", &story.title)
            .replace("{DESC}", story.description.as_deref().unwrap_or(""));
        (prompt.to_string(), msg)
    } else if story.status == "Review" {
        let prompt = "You are Sandman Reviewer. Analyze the code changes using <tool:read_file> or <tool:search_code> to verify quality and correctness for the target technology stack.
Story Title: {TITLE}
Description: {DESC}";
        let msg = prompt
            .replace("{TITLE}", &story.title)
            .replace("{DESC}", story.description.as_deref().unwrap_or(""));
        (prompt.to_string(), msg)
    } else {
        let prompt = "You are Sandman, an autonomous IDE agent. You must analyze the requirements and provide a high-level implementation strategy. For now, just summarize the task and confirm receipt.";
        let msg = format!("Task: {}\nPlease analyze this story and confirm readiness.", story.title);
        (prompt.to_string(), msg)
    };

    if let Some(ctx) = additional_context {
        user_msg = format!("{}\n\nADDITIONAL CONTEXT FROM USER:\n{}", user_msg, ctx);
    }
    
    let mut conversation = vec![
        Message { role: "system".to_string(), content: system_prompt },
        Message { role: "user".to_string(), content: user_msg },
    ];

    let app_clone = app.clone();
    let id_clone = id.clone();
    let pool_clone = pool.clone();

    tauri::async_runtime::spawn(async move {
        // Recover state inside the spawned task to avoid lifetime issues
        let state = app_clone.state::<AppState>();
        
        let mut loop_count = 0;
        let max_loops = 10;
        let mut final_response = String::new();

        while loop_count < max_loops {
            match call_llm(&app_clone, conversation.clone()).await {
                Ok(response) => {
                    final_response = response.clone();
                    
                    if let Some((tool_name, args)) = parse_tool_call(&response) {
                        let _ = app_clone.emit("log", format!("\x1b[35m[Agent]\x1b[0m Executing {}: {}", tool_name, args.chars().take(50).collect::<String>()));
                        
                        let result = match tool_name.as_str() {
                            "read_file" => read_file_internal(args.trim(), &state).await,
                            "write_file" => {
                                let parts: Vec<&str> = args.splitn(2, '|').collect();
                                if parts.len() == 2 {
                                    write_file_internal(parts[0].trim(), parts[1].trim(), &state).await
                                        .map(|_| "Success: File written".to_string())
                                } else {
                                    Err("Format error: use 'path | content'".to_string())
                                }
                            },
                            "run_command" => {
                                let parts: Vec<&str> = args.split_whitespace().collect();
                                if !parts.is_empty() {
                                    run_project_command_internal(parts[0], parts[1..].iter().map(|s| s.to_string()).collect(), &state).await
                                } else {
                                    Err("Empty command".to_string())
                                }
                            },
                            "search_code" => search_code_internal(args.trim(), &state).await,
                            _ => Err("Unknown tool".to_string()),
                        };

                        let tool_msg = match result {
                            Ok(r) => format!("TOOL_RESULT: {}", r),
                            Err(e) => format!("TOOL_ERROR: {}", e),
                        };

                        conversation.push(Message { role: "assistant".to_string(), content: response });
                        conversation.push(Message { role: "user".to_string(), content: tool_msg });
                        loop_count += 1;
                    } else {
                        break;
                    }
                },
                Err(e) => {
                    let _ = app_clone.emit("log", format!("\x1b[31m[Agent]\x1b[0m AI Error: {}", e));
                    let _ = sqlx::query("UPDATE stories SET state = 'failed' WHERE id = ?").bind(&id_clone).execute(&pool_clone).await;
                    return;
                }
            }
        }

        // Finalize story based on context
        let response = final_response;
        let _ = app_clone.emit("log", format!("\x1b[32m[Agent]\x1b[0m Task completed: {}", id_clone));
        
        if story.status == "Raw Requirements" || story.status == "Clarification Required" {
            let new_title = response.lines()
                .find(|l| l.contains("Title:"))
                .map(|l| l.replace("# Title:", "").trim().to_string())
                .unwrap_or(story.title.clone());
            
            let has_questions = response.to_lowercase().contains("clarifying questions") 
                && response.lines()
                    .skip_while(|l| !l.to_lowercase().contains("clarifying questions"))
                    .skip(1)
                    .any(|l| !l.trim().is_empty());

            let next_status = if has_questions { "Clarification Required" } else { "To Do" };

            let _ = sqlx::query("UPDATE stories SET title = ?, description = ?, status = ?, state = 'success', ai_ready = ? WHERE id = ?")
                .bind(&new_title).bind(&response).bind(next_status).bind(1).bind(&id_clone).execute(&pool_clone).await;
        } else if story.status == "To Do" {
            let _ = sqlx::query("UPDATE stories SET status = 'In Progress', agent = 'Builder', state = 'success' WHERE id = ?").bind(&id_clone).execute(&pool_clone).await;
        } else if story.status == "In Progress" {
            let _ = sqlx::query("UPDATE stories SET status = 'Review', agent = 'Reviewer', state = 'success' WHERE id = ?").bind(&id_clone).execute(&pool_clone).await;
        } else if story.status == "Review" {
            let _ = sqlx::query("UPDATE stories SET status = 'Done', agent = NULL, state = 'success' WHERE id = ?").bind(&id_clone).execute(&pool_clone).await;
        } else {
            let _ = sqlx::query("UPDATE stories SET state = 'success' WHERE id = ?").bind(&id_clone).execute(&pool_clone).await;
        }
    });

    Ok(())
}

fn setup_project_structure(path: &str) {
    let folders = ["src", "docs", "tests", ".sandman"];
    for folder in folders {
        let mut p = std::path::PathBuf::from(path);
        p.push(folder);
        if !p.exists() {
            let _ = std::fs::create_dir_all(&p);
        }
    }
}

#[tauri::command]
async fn switch_project(path: String, app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<String, String> {
    println!("Switched active project to: {}", path);
    *state.project_path.lock().unwrap() = Some(path.clone());

    // 1. Setup standard project structure
    setup_project_structure(&path);

    // 2. Connect to SQLite db and embed in .sandman/
    let pool = db::init_db(&path).await?;
    *state.db.lock().unwrap() = Some(pool.clone());

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
            delete_story,
            toggle_story_hold,
            list_files,
            read_file,
            write_file,
            run_project_command,
            search_code,
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
