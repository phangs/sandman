mod db;
mod config;
mod rag;
mod llm;
mod agent;
mod tools;
mod prompts;
mod utils;

use tauri::{Emitter, Manager};
use std::sync::Mutex;
use sqlx::{SqlitePool, FromRow};
use serde::{Deserialize, Serialize};
use config::{Config, load_config};
use portable_pty::MasterPty;
use std::io::Write;
use llm::{call_llm, Message};
// use rag::{index_file, search_chunks};
use agent::dispatch_agent_internal;
use utils::{kill_terminal_command_internal, run_terminal_command_internal};

#[derive(serde::Serialize, serde::Deserialize, Clone, sqlx::FromRow)]
pub struct Artifact {
    pub id: String,
    pub story_id: String,
    pub name: String,
    pub content: String,
    pub a_type: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub saved: i64,
}

pub struct AppState {
    pub db: Mutex<Option<SqlitePool>>,
    pub project_path: Mutex<Option<String>>,
    pub terminal_pid: Mutex<Option<u32>>,
    pub pty_master: Mutex<Option<Box<dyn MasterPty + Send>>>,
    pub pty_writer: Mutex<Option<Box<dyn Write + Send>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Option<Vec<FileEntry>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct ChatSession {
    pub id: i64,
    pub title: String,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct ChatMessage {
    pub id: i64,
    pub session_id: i64,
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Story {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub ai_ready: i64,
    pub ai_hold: i64,
    pub reviewer_feedback: Option<String>,
    pub skip_clarification: i64,
    pub agent: Option<String>,
    pub state: Option<String>,
}

#[tauri::command]
fn get_config(app: tauri::AppHandle) -> Result<Config, String> {
    Ok(load_config(&app))
}

#[tauri::command]
fn save_global_config(config: crate::config::Config, app: tauri::AppHandle) -> Result<(), String> {
    crate::config::save_config(&app, &config)
}

#[tauri::command]
fn set_column_strategy(status: String, provider_id: String, app: tauri::AppHandle) -> Result<(), String> {
    let mut config = crate::config::load_config(&app);
    config.column_strategies.insert(status, provider_id);
    crate::config::save_config(&app, &config)
}

#[tauri::command]
async fn switch_project(path: String, app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let pool = db::init_db(&path).await?;
    let pool_clone = pool.clone();
    
    {
        let mut db_guard = state.db.lock().unwrap();
        *db_guard = Some(pool);
    }
    {
        let mut path_guard = state.project_path.lock().unwrap();
        *path_guard = Some(path.clone());
    }
    
    // Initialize PTY for the Terminal tab
    let pty_system = portable_pty::native_pty_system();
    let pty_pair = pty_system.openpty(portable_pty::PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    }).map_err(|e: anyhow::Error| e.to_string())?;

    #[cfg(target_os = "windows")]
    let shell = "cmd.exe";
    #[cfg(not(target_os = "windows"))]
    let shell = "sh";

    let mut cmd = portable_pty::CommandBuilder::new(shell);
    cmd.cwd(path.clone());
    let _child = pty_pair.slave.spawn_command(cmd).map_err(|e: anyhow::Error| e.to_string())?;

    let writer = pty_pair.master.take_writer().map_err(|e: anyhow::Error| e.to_string())?;
    let mut reader = pty_pair.master.try_clone_reader().map_err(|e: anyhow::Error| e.to_string())?;
    
    {
        let mut master_guard = state.pty_master.lock().unwrap();
        *master_guard = Some(pty_pair.master);
    }
    {
        let mut writer_guard = state.pty_writer.lock().unwrap();
        *writer_guard = Some(writer);
    }

    // Set up background reader for PTY
    let app_handle = app.clone();
    std::thread::spawn(move || {
        let mut buffer = [0u8; 1024];
        loop {
            match reader.read(&mut buffer) {
                Ok(n) if n > 0 => {
                    let text = String::from_utf8_lossy(&buffer[..n]).to_string();
                    let _ = app_handle.emit("pty-stdout", text);
                }
                Ok(_) => break, // EOF
                Err(_) => break,
            }
        }
    });

    // Auto-index project in background
    let path_cloned = path.clone();
    tauri::async_runtime::spawn(async move {
         let _ = index_project_internal(&path_cloned, &pool_clone).await;
    });

    Ok(())
}

async fn index_project_internal(path: &str, pool: &SqlitePool) -> Result<(), String> {
    let mut files = Vec::new();
    let walker = ignore::WalkBuilder::new(path).hidden(true).git_ignore(true).build();
    for e in walker.flatten() {
        if e.path().is_file() {
            let rel = e.path().strip_prefix(path).unwrap_or(e.path());
            let name = rel.to_string_lossy().to_string();
            if name.ends_with(".ts") || name.ends_with(".tsx") || name.ends_with(".rs") || name.ends_with(".js") || name.ends_with(".py") {
                files.push((e.path().to_path_buf(), name));
            }
        }
    }

    // Load config for embedding endpoint
    // In a real app we'd need AppHandle here, but we can assume ollama default for now or pass it in
    // For now we'll assume http://localhost:11434 from common defaults
    let endpoint = "http://localhost:11434";

    for (full, rel) in files {
        if let Ok(content) = std::fs::read_to_string(&full) {
            let _ = crate::rag::index_file(pool, &rel, &content, endpoint).await;
        }
    }
    Ok(())
}

#[tauri::command]
async fn get_stories(state: tauri::State<'_, AppState>) -> Result<Vec<Story>, String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("Database not initialized")?;
    sqlx::query_as::<_, Story>("SELECT * FROM stories ORDER BY id DESC").fetch_all(&pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_story(title: String, status: String, state: tauri::State<'_, AppState>) -> Result<Story, String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("Database not initialized")?;

    let id = format!("S-{}", uuid::Uuid::new_v4().to_string()[..6].to_uppercase());
    let ai_ready = 1i64;

    sqlx::query("INSERT INTO stories (id, title, status, ai_ready, state) VALUES (?, ?, ?, ?, 'idle')")
        .bind(&id).bind(&title).bind(&status).bind(ai_ready).execute(&pool).await.map_err(|e| e.to_string())?;

    Ok(Story {
        id, title, status, ai_ready,
        description: None, ai_hold: 0, agent: None, state: Some("idle".into()), reviewer_feedback: None, skip_clarification: 0,
    })
}

#[tauri::command]
async fn delete_story(id: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("Database not initialized")?;
    sqlx::query("DELETE FROM stories WHERE id = ?").bind(id).execute(&pool).await.map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_story_tasks(story_id: String, state: tauri::State<'_, AppState>) -> Result<Vec<serde_json::Value>, String> {
    // Moved to agent logic or kept here as a command
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("Database not initialized")?;
    use sqlx::Row;
    let rows = sqlx::query("SELECT id, title, completed FROM story_tasks WHERE story_id = ?").bind(story_id).fetch_all(&pool).await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|row| {
        serde_json::json!({
            "id": row.get::<i64, _>("id"),
            "title": row.get::<String, _>("title"),
            "completed": row.get::<i64, _>("completed") != 0
        })
    }).collect())
}

#[tauri::command]
async fn set_column_ai_paused(status: String, paused: bool, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;
    
    let hold = if paused { 1 } else { 0 };
    sqlx::query("UPDATE stories SET ai_hold = ? WHERE status = ?")
        .bind(hold).bind(status).execute(&pool).await.map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_story_status(id: String, status: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("Database not initialized")?;
    sqlx::query("UPDATE stories SET status = ?, state = 'idle', ai_ready = 1 WHERE id = ?").bind(status).bind(id).execute(&pool).await.map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn dispatch_story(id: String, additional_context: Option<String>, app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("Database not initialized")?;

    let story: Story = sqlx::query_as("SELECT * FROM stories WHERE id = ?").bind(&id).fetch_one(&pool).await.map_err(|e| e.to_string())?;
    if story.state.as_deref() == Some("processing") {
         return Ok(());
    }
    
    // Clear stale reviewer feedback
    sqlx::query("UPDATE stories SET reviewer_feedback = NULL, state = 'processing' WHERE id = ?").bind(&id).execute(&pool).await.ok();
    app.emit("refresh_board", ()).ok();

    let _ = app.emit("log", format!("\x1b[33m[Agent]\x1b[0m Dispatching story: {}", id));

    // Automated discovery (eyes on project)
    let project_path = {
        let guard = state.project_path.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;

    let mut file_list = Vec::new();
    let walker = ignore::WalkBuilder::new(&project_path).max_depth(Some(3)).hidden(true).git_ignore(true).build();
    for e in walker.take(100).flatten() {
        let path = e.path().strip_prefix(&project_path).unwrap_or(e.path());
        if path.as_os_str().is_empty() { continue; }
        let name = path.to_string_lossy();
        if name.starts_with(".git") || name.starts_with(".sandman") || name.contains("node_modules") { continue; }
        file_list.push(format!("  {}{}", name, if e.file_type().unwrap().is_dir() { "/" } else { "" }));
    }
    let discovery_context = format!("\n### CURRENT PROJECT STRUCTURE\n{}\n", file_list.join("\n"));

    let rag_query = format!("{}\n{}", story.title, story.description.as_deref().unwrap_or(""));
    let mut rag_context = String::new();
    if matches!(story.status.as_str(), "To Do" | "In Progress" | "Review") {
        let config = load_config(&app);
        if let Some(ollama) = config.providers.iter().find(|p| p.id == "ollama" && p.active) {
            match crate::rag::search_chunks(&pool, &rag_query, &ollama.endpoint, 5).await {
                Ok(chunks) => {
                    let mut ctx = String::from("\n### SEMANTICALLY RELEVANT CODE CHUNKS (RAG)\n");
                    for (path, content, sim) in chunks {
                        ctx.push_str(&format!("-- From file: {} (Similarity: {:.2})\n{}\n\n", path, sim, content));
                    }
                    rag_context = ctx;
                },
                Err(e) => { let _ = app.emit("log", format!("\x1b[31m[Agent]\x1b[0m RAG indexing failed or silent: {}", e)); }
            }
        }
    }

    dispatch_agent_internal(id, story, pool, app, discovery_context, rag_context, additional_context).await;
    Ok(())
}

#[tauri::command]
async fn run_terminal_command(command: String, app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    run_terminal_command_internal(command, app, state).await
}

#[tauri::command]
async fn kill_terminal_command(state: tauri::State<'_, AppState>) -> Result<(), String> {
    kill_terminal_command_internal(&state).await
}

#[tauri::command]
async fn get_artifacts(story_id: String, state: tauri::State<'_, AppState>) -> Result<Vec<Artifact>, String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;
    sqlx::query_as::<_, Artifact>("SELECT * FROM artifacts WHERE story_id = ? ORDER BY created_at DESC")
        .bind(story_id).fetch_all(&pool).await.map_err(|e: sqlx::Error| e.to_string())
}

#[tauri::command]
async fn toggle_artifact_save(id: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;
    sqlx::query("UPDATE artifacts SET saved = 1 - saved WHERE id = ?").bind(id).execute(&pool).await.map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn purge_artifacts(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;
    sqlx::query("DELETE FROM artifacts WHERE saved = 0").execute(&pool).await.map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
fn pty_write(data: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.pty_writer.lock().unwrap();
    if let Some(writer) = guard.as_mut() {
        writer.write_all(data.as_bytes()).map_err(|e| e.to_string())
    } else {
        Err("PTY not initialized".into())
    }
}

#[tauri::command]
fn pty_resize(cols: u16, rows: u16, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let guard = state.pty_master.lock().unwrap();
    if let Some(master) = guard.as_ref() {
        master.resize(portable_pty::PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        }).map_err(|e| e.to_string())
    } else {
        Err("PTY not initialized".into())
    }
}

#[tauri::command]
async fn list_chat_sessions(state: tauri::State<'_, AppState>) -> Result<Vec<ChatSession>, String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;
    sqlx::query_as::<_, ChatSession>("SELECT * FROM chat_sessions ORDER BY updated_at DESC")
        .fetch_all(&pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_chat_messages(session_id: i64, state: tauri::State<'_, AppState>) -> Result<Vec<ChatMessage>, String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;
    sqlx::query_as::<_, ChatMessage>("SELECT * FROM chat_messages WHERE session_id = ? ORDER BY created_at ASC")
        .bind(session_id).fetch_all(&pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_chat_session(title: String, state: tauri::State<'_, AppState>) -> Result<i64, String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;
    let now = chrono::Utc::now().timestamp();
    let res = sqlx::query("INSERT INTO chat_sessions (title, updated_at) VALUES (?, ?)")
        .bind(title).bind(now).execute(&pool).await.map_err(|e| e.to_string())?;
    Ok(res.last_insert_rowid())
}

#[tauri::command]
async fn delete_chat_session(id: i64, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;
    sqlx::query("DELETE FROM chat_messages WHERE session_id = ?").bind(id).execute(&pool).await.ok();
    sqlx::query("DELETE FROM chat_sessions WHERE id = ?").bind(id).execute(&pool).await.map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn chat_with_agent(session_id: Option<i64>, messages_all: Vec<llm::Message>, app: tauri::AppHandle) -> Result<String, String> {
    let state = app.state::<AppState>();
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;

    let response = crate::llm::call_llm(&app, messages_all.clone(), Some("brainstorm")).await?;
    
    if let Some(sid) = session_id {
        if let Some(user_msg) = messages_all.last() {
            let now = chrono::Utc::now().timestamp();
            sqlx::query("INSERT INTO chat_messages (session_id, role, content, created_at) VALUES (?, 'user', ?, ?)")
                .bind(sid).bind(&user_msg.content).bind(now).execute(&pool).await.ok();
            sqlx::query("INSERT INTO chat_messages (session_id, role, content, created_at) VALUES (?, 'assistant', ?, ?)")
                .bind(sid).bind(&response).bind(now + 1).execute(&pool).await.ok();
            sqlx::query("UPDATE chat_sessions SET updated_at = ? WHERE id = ?").bind(now + 1).bind(sid).execute(&pool).await.ok();
        }
    }
    
    Ok(response)
}

#[tauri::command]
async fn create_story_from_chat(messages: Vec<llm::Message>, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;

    let content = messages.iter().map(|m| format!("{}: {}", m.role, m.content)).collect::<Vec<_>>().join("\n\n");
    let title = messages.iter().find(|m| m.role == "user").map(|m| m.content.chars().take(50).collect::<String>()).unwrap_or_else(|| "New Story from Chat".into());
    let id = format!("S-{}", uuid::Uuid::new_v4().to_string()[..6].to_uppercase());

    sqlx::query("INSERT INTO stories (id, title, description, status, ai_ready, state) VALUES (?, ?, ?, 'Raw Requirements', 1, 'idle')")
        .bind(&id).bind(&title).bind(&content).execute(&pool).await.map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_files(path: String, state: tauri::State<'_, AppState>) -> Result<Vec<FileEntry>, String> {
    let project_path = {
        let guard = state.project_path.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;

    let abs_path = std::path::PathBuf::from(&project_path).join(&path);
    if !abs_path.starts_with(&project_path) {
        return Err("Access denied".into());
    }

    let mut entries = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(abs_path) {
        for e in read_dir.flatten() {
            let rel = e.path().strip_prefix(&project_path).unwrap_or(&e.path()).to_string_lossy().to_string();
            let meta = e.metadata().map_err(|e| e.to_string())?;
            entries.push(FileEntry {
                name: e.file_name().to_string_lossy().to_string(),
                path: rel,
                is_dir: meta.is_dir(),
                children: None,
            });
        }
    }
    
    entries.sort_by(|a, b| {
        if a.is_dir != b.is_dir {
            b.is_dir.cmp(&a.is_dir)
        } else {
            a.name.cmp(&b.name)
        }
    });

    Ok(entries)
}

#[tauri::command]
async fn read_file(path: String, state: tauri::State<'_, AppState>) -> Result<String, String> {
    crate::tools::read_file_internal(&path, &state).await
}

#[tauri::command]
async fn write_file(path: String, content: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    crate::tools::write_file_internal(&path, &content, &state).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState {
            db: Mutex::new(None),
            project_path: Mutex::new(None),
            terminal_pid: Mutex::new(None),
            pty_master: Mutex::new(None),
            pty_writer: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            get_config, save_global_config, set_column_strategy,
            switch_project,
            get_stories, create_story, delete_story, update_story_status,
            set_column_ai_paused,
            get_story_tasks, dispatch_story, run_terminal_command, kill_terminal_command,
            get_artifacts, toggle_artifact_save, purge_artifacts,
            pty_write, pty_resize,
            list_chat_sessions, get_chat_messages, create_chat_session, delete_chat_session,
            chat_with_agent, create_story_from_chat,
            list_files, read_file, write_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
