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
use portable_pty::MasterPty;
use std::io::Write;

pub struct AppState {
    pub db: Mutex<Option<SqlitePool>>,
    pub project_path: Mutex<Option<String>>,
    pub terminal_pid: Mutex<Option<u32>>,
    pub pty_master: Mutex<Option<Box<dyn MasterPty + Send>>>,
    pub pty_writer: Mutex<Option<Box<dyn Write + Send>>>,
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
    pub reviewer_feedback: Option<String>,
    pub skip_clarification: i32,
}

#[tauri::command]
async fn get_stories(state: tauri::State<'_, AppState>) -> Result<Vec<Story>, String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    };

    if let Some(pool) = pool {
        let stories = sqlx::query_as::<_, Story>("SELECT id, title, description, status, ai_ready, ai_hold, agent, state, reviewer_feedback, skip_clarification FROM stories")
            .fetch_all(&pool)
            .await
            .map_err(|e| format!("Database error: {}", e))?;
        Ok(stories)
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
async fn create_story(title: String, skip_clarification: bool, state: tauri::State<'_, AppState>) -> Result<Story, String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    };

    if let Some(pool) = pool {
        let id_suffix = uuid::Uuid::new_v4().to_string().chars().take(6).collect::<String>();
        let id = format!("S-{}", id_suffix.to_uppercase());
        let status = "Raw Requirements".to_string();
        let ai_ready = 1; // Always ready for AI when in Raw Requirements

        let skip_val = if skip_clarification { 1i64 } else { 0i64 };
        sqlx::query("INSERT INTO stories (id, title, status, ai_ready, skip_clarification) VALUES (?, ?, ?, ?, ?)")
            .bind(&id)
            .bind(&title)
            .bind(&status)
            .bind(ai_ready)
            .bind(skip_val)
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
            reviewer_feedback: None,
            skip_clarification: if skip_clarification { 1 } else { 0 },
        })
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
async fn update_story_ready(id: String, ready: bool, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("Database not initialized")?;

    sqlx::query("UPDATE stories SET ai_ready = ?, state = 'idle' WHERE id = ?")
        .bind(if ready { 1i64 } else { 0i64 })
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(())
}
#[tauri::command]
async fn update_story_status(id: String, status: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("Database not initialized")?;

    sqlx::query("UPDATE stories SET status = ?, state = 'idle' WHERE id = ?")
        .bind(&status)
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn get_story_tasks(story_id: String, state: tauri::State<'_, AppState>) -> Result<Vec<serde_json::Value>, String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;

    use sqlx::Row;
    let rows = sqlx::query("SELECT id, title, completed FROM story_tasks WHERE story_id = ?")
        .bind(&story_id)
        .fetch_all(&pool)
        .await
        .map_err(|e: sqlx::Error| e.to_string())?;

    Ok(rows.into_iter().map(|row| {
        let id: i64 = row.try_get("id").unwrap_or(0);
        let title: String = row.try_get("title").unwrap_or_default();
        let completed: i64 = row.try_get("completed").unwrap_or(0);
        serde_json::json!({
            "id": id,
            "title": title,
            "completed": completed != 0
        })
    }).collect())
}

#[tauri::command]
async fn create_story_task(story_id: String, title: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;

    sqlx::query("INSERT INTO story_tasks (story_id, title) VALUES (?, ?)")
        .bind(story_id)
        .bind(title)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn update_story_task(task_id: i64, completed: bool, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;

    sqlx::query("UPDATE story_tasks SET completed = ? WHERE id = ?")
        .bind(if completed { 1 } else { 0 })
        .bind(task_id)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;
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

    sqlx::query("UPDATE stories SET ai_hold = ?, state = 'idle' WHERE id = ?")
        .bind(new_hold)
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(new_hold)
}

#[tauri::command]
async fn clear_column_state(status: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("Database not initialized")?;

    sqlx::query("UPDATE stories SET state = 'idle', ai_ready = 1, ai_hold = 0 WHERE status = ?")
        .bind(status)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;
        
    Ok(())
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
    call_llm(&app, messages, None).await
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

async fn apply_patch_internal(path: &str, old_content: &str, new_content: &str, state: &tauri::State<'_, AppState>) -> Result<(), String> {
    let current_content = read_file_internal(path, state).await?;
    
    if !current_content.contains(old_content) {
        return Err("Target content not found in file. Please ensure old_content matches exactly.".to_string());
    }

    let patched_content = current_content.replace(old_content, new_content);
    write_file_internal(path, &patched_content, state).await
}

async fn run_project_command_internal(full_command: &str, state: &tauri::State<'_, AppState>) -> Result<String, String> {
    let project_path = {
        let guard = state.project_path.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;

    #[cfg(target_os = "windows")]
    let (shell, arg) = ("cmd", "/C");
    #[cfg(not(target_os = "windows"))]
    let (shell, arg) = ("sh", "-c");

    let child = tokio::process::Command::new(shell)
        .arg(arg)
        .arg(full_command)
        .current_dir(project_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Failed to spawn command: {}", e))?;

    let timeout_duration = std::time::Duration::from_secs(30);
    
    match tokio::time::timeout(timeout_duration, child.wait_with_output()).await {
        Ok(result) => {
            let output = result.map_err(|e| e.to_string())?;
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            if output.status.success() {
                Ok(stdout)
            } else {
                Err(format!("Command failed (Exit {}):\nSTDOUT: {}\nSTDERR: {}", 
                    output.status.code().unwrap_or(-1), stdout, stderr))
            }
        },
        Err(_) => {
            Err("Command TIMEOUT (30s): The command was killed because it took too long. If you were trying to start a dev server, please run a BUILD command (like 'npm run build') instead, as it must terminate for the agent to continue.".to_string())
        }
    }
}

use std::io::{BufRead, BufReader};

#[tauri::command]
async fn kill_terminal_command(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let pid = {
        let mut guard = state.terminal_pid.lock().unwrap();
        guard.take()
    };

    if let Some(pid) = pid {
        #[cfg(not(target_os = "windows"))]
        {
            use std::process::Command;
            let _ = Command::new("kill").arg("-9").arg(pid.to_string()).spawn();
        }
        #[cfg(target_os = "windows")]
        {
            use std::process::Command;
            let _ = Command::new("taskkill").arg("/F").arg("/T").arg("/PID").arg(pid.to_string()).spawn();
        }
        Ok(())
    } else {
        Err("No active process to kill".to_string())
    }
}

#[tauri::command]
async fn run_terminal_command(command: String, app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let project_path = {
        let guard = state.project_path.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;

    let app_clone = app.clone();
    
    tauri::async_runtime::spawn(async move {
        let state = app_clone.state::<AppState>();

        #[cfg(target_os = "windows")]
        let (shell, arg) = ("cmd", "/C");
        #[cfg(not(target_os = "windows"))]
        let (shell, arg) = ("sh", "-c");

        let child = std::process::Command::new(shell)
            .arg(arg)
            .arg(&command)
            .current_dir(project_path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn();

        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                let _ = app_clone.emit("terminal-stdout", format!("\x1b[31mError spawning command: {}\x1b[0m\n", e));
                return;
            }
        };

        // Store the PID for killing
        {
            let mut guard = state.terminal_pid.lock().unwrap();
            *guard = Some(child.id());
        }

        let stdout = child.stdout.take().expect("Failed to open stdout");
        let stderr = child.stderr.take().expect("Failed to open stderr");

        let app_stdout = app_clone.clone();
        let app_stderr = app_clone.clone();

        // Handle stdout
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(l) = line {
                    let _ = app_stdout.emit("terminal-stdout", format!("{}\n", l));
                }
            }
        });

        // Handle stderr
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(l) = line {
                    let _ = app_stderr.emit("terminal-stdout", format!("\x1b[31m{}\x1b[0m\n", l));
                }
            }
        });

        match child.wait() {
            Ok(status) => {
                // Clear the PID after completion
                {
                    let mut guard = state.terminal_pid.lock().unwrap();
                    if *guard == Some(child.id()) {
                        *guard = None;
                    }
                }
                let _ = app_clone.emit("terminal-stdout", format!("\n\x1b[32mProcess finished with exit code: {}\x1b[0m\n", status));
            },
            Err(e) => {
                let _ = app_clone.emit("terminal-stdout", format!("\n\x1b[31mProcess error: {}\x1b[0m\n", e));
            }
        }
    });

    Ok(())
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
                // Hallucination Defense: Strip XML-like wrappers if agents use them inside tool calls
                let mut args = after_name[..end_tag_idx].to_string();
                args = args.trim().to_string();
                
                // Helper to strip ALL XML-like tags and keep only the content
                fn strip_xml(input: &str) -> String {
                    let mut output = input.to_string();
                    while let Some(start) = output.find('<') {
                        if let Some(end) = output[start..].find('>') {
                            let end_abs = start + end;
                            output.replace_range(start..=end_abs, "");
                        } else {
                            break;
                        }
                    }
                    output.trim().to_string()
                }

                if tool_name == "run_command" || tool_name == "read_file" || tool_name == "search_code" || tool_name == "create_task" {
                     args = strip_xml(&args);
                }
                
                return Some((tool_name.to_string(), args));
            }
        }
    }
    None
}


#[tauri::command]
async fn approve_guarded_action(id: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let pool = {
        let guard = state.db.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;

    sqlx::query("UPDATE action_auths SET status = 'approved' WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
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
async fn apply_patch(path: String, old_content: String, new_content: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    apply_patch_internal(&path, &old_content, &new_content, &state).await
}

#[tauri::command]
async fn run_project_command(command: String, args: Vec<String>, state: tauri::State<'_, AppState>) -> Result<String, String> {
    let full = if args.is_empty() { command } else { format!("{} {}", command, args.join(" ")) };
    run_project_command_internal(&full, &state).await
}

#[tauri::command]
async fn search_code(query: String, state: tauri::State<'_, AppState>) -> Result<String, String> {
    search_code_internal(&query, &state).await
}

async fn grep_search_internal(query: &str, state: &tauri::State<'_, AppState>) -> Result<String, String> {
    let project_path = {
        let guard = state.project_path.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;

    let mut results = Vec::new();
    let walker = WalkBuilder::new(&project_path)
        .hidden(true)
        .git_ignore(true)
        .build();

    for entry in walker {
        if let Ok(entry) = entry {
            if entry.path().is_file() {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    for (i, line) in content.lines().enumerate() {
                        if line.contains(query) {
                            let rel = entry.path().strip_prefix(&project_path).unwrap_or(entry.path());
                            results.push(format!("{}:{}: {}", rel.display(), i + 1, line.trim()));
                            if results.len() > 100 { 
                                results.push("... Too many results, please narrow your search.".to_string());
                                break; 
                            }
                        }
                    }
                }
            }
        }
    }

    if results.is_empty() {
        Ok("No matches found.".to_string())
    } else {
        Ok(results.join("\n"))
    }
}

async fn list_files_recursive_internal(state: &tauri::State<'_, AppState>) -> Result<String, String> {
    let project_path = {
        let guard = state.project_path.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;

    let mut files = Vec::new();
    let walker = WalkBuilder::new(&project_path)
        .hidden(false)
        .git_ignore(true)
        .build();

    for entry in walker {
        if let Ok(entry) = entry {
            let rel = entry.path().strip_prefix(&project_path).unwrap_or(entry.path());
            let path_str = rel.to_string_lossy().to_string();
            if path_str.is_empty() || path_str.starts_with(".git") || path_str.starts_with(".sandman") || path_str.contains("node_modules") || path_str.starts_with("target") {
                continue;
            }
            if entry.path().is_dir() {
                files.push(format!("{}/", path_str));
            } else {
                files.push(path_str);
            }
        }
    }
    
    Ok(files.join("\n"))
}

async fn manage_filesystem_internal(op: &str, path: &str, dest: Option<&str>, state: &tauri::State<'_, AppState>) -> Result<String, String> {
    let (pool, project_path) = {
        let guard_db = state.db.lock().unwrap();
        let guard_path = state.project_path.lock().unwrap();
        (guard_db.clone(), guard_path.clone())
    };

    let pool = pool.ok_or("No project connected")?;
    let project_path = project_path.ok_or("No project connected")?;

    let abs_path = std::path::PathBuf::from(&project_path).join(path);
    
    match op {
        "mkdir" => {
            std::fs::create_dir_all(&abs_path).map_err(|e| e.to_string())?;
            Ok(format!("Directory created: {}", path))
        },
        "move" => {
            if let Some(dest_p) = dest {
                let abs_dest = std::path::PathBuf::from(&project_path).join(dest_p);
                std::fs::rename(&abs_path, &abs_dest).map_err(|e| e.to_string())?;
                Ok(format!("Moved {} to {}", path, dest_p))
            } else {
                Err("Move destination required".to_string())
            }
        },
        "delete" => {
            // DESTRUCTIVE ACTION SAFETY CHECK
            let auth: Option<(String,)> = sqlx::query_as("SELECT status FROM action_auths WHERE action = 'delete' AND target = ? AND status = 'approved'")
                .bind(path)
                .fetch_optional(&pool)
                .await
                .map_err(|e| e.to_string())?;

            if auth.is_none() {
                let auth_id = format!("auth-{}", uuid::Uuid::new_v4());
                sqlx::query("INSERT INTO action_auths (id, action, target, status) VALUES (?, 'delete', ?, 'pending')")
                    .bind(&auth_id)
                    .bind(path)
                    .execute(&pool)
                    .await
                    .map_err(|e| e.to_string())?;

                return Err(format!("AUTH_REQUIRED: Deletion of {} requires human approval. I have created a request ({}). Please tell the user to APPROVE this in their terminal or chat before you try again.", path, auth_id));
            }

            // If approved, proceed
            if abs_path.is_dir() {
                std::fs::remove_dir_all(&abs_path).map_err(|e| e.to_string())?;
            } else {
                std::fs::remove_file(&abs_path).map_err(|e| e.to_string())?;
            }
            
            // Consume the auth
            let _ = sqlx::query("DELETE FROM action_auths WHERE action = 'delete' AND target = ?").bind(path).execute(&pool).await;
            
            Ok(format!("Deleted: {}", path))
        },
        _ => Err("Unknown operation".to_string()),
    }
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
                    let relative_path_str = relative_path.to_string_lossy();

                    // Skip common noise directories and build artifacts
                    if relative_path_str.contains("node_modules") || 
                       relative_path_str.starts_with(".git") || 
                       relative_path_str.starts_with(".sandman") ||
                       relative_path_str.starts_with("target") ||
                       relative_path_str.starts_with("dist") ||
                       relative_path_str.starts_with("build") {
                        continue;
                    }

                    // Skip binary and lock files
                    let ext = relative_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "ico"| "lock" | "yaml" | "exe" | "dll" | "so" | "dylib") {
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

    // 1. Fetch the story
    let story: Story = sqlx::query_as("SELECT id, title, description, status, ai_ready, ai_hold, agent, state, reviewer_feedback, skip_clarification FROM stories WHERE id = ?")
        .bind(&id)
        .fetch_one(&pool)
        .await
        .map_err(|e| e.to_string())?;

    // 2. Mark as processing
    let agent_name = if story.status == "Documentation" { "Writer" } else { "Builder" };
    sqlx::query("UPDATE stories SET state = 'processing', agent = ? WHERE id = ?")
        .bind(agent_name)
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;

    let _ = app.emit("log", format!("\x1b[33m[Agent]\x1b[0m Dispatching {} to story: {}", agent_name, id));

    // 3. Automated Context Discovery: Give the agent 'eyes' on the filesystem immediately
    let project_path = {
        let guard = state.project_path.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;

    let mut file_list = Vec::new();
    let walker = WalkBuilder::new(&project_path)
        .max_depth(Some(3))
        .hidden(false)
        .git_ignore(true)
        .build();

    for entry in walker.take(100) {
        if let Ok(e) = entry {
            let path = e.path().strip_prefix(&project_path).unwrap_or(e.path());
            if path.as_os_str().is_empty() { continue; }
            let name = path.to_string_lossy();
            if name.starts_with(".git") || name.starts_with(".sandman") || name.contains("node_modules") { continue; }
            file_list.push(format!("  {}{}", name, if e.file_type().unwrap().is_dir() { "/" } else { "" }));
        }
    }
    let discovery_context = format!("
### PROJECT ISOLATION POLICY
- Sandman IDE Port: 5173 (Reserved)
- Target Project Dev Port: 5180 (Standard for all target apps)

### CURRENT PROJECT STRUCTURE
(Relative to project root)
{}
", file_list.join("\n"));

    // 4. Simple static context for now (RAG indexing was already set up in background)
    let (system_prompt, mut user_msg) = if story.status == "Raw Requirements" || story.status == "Clarification Required" {
        let prompt = "You are Sandman, a senior product owner and software architect. Your job is to take raw requirements from a user and polish them into a professional, Jira-style user story.

DOCUMENTATION RESPONSIBILITIES:
- You MUST maintain 'docs/PRD.md' (Project Requirements Document) and 'docs/FEATURES.md' (High-level feature list).
- For every new requirement, you must ensure these files are updated or created using <tool:write_file> BEFORE finalizing the story.

The output MUST follow this format:
# Title: [Polished Story Title]
# Description: [Clear, detailed description of the feature or task]
# Acceptance Criteria:
- [Criteria 1]
- [Criteria 2]
...
# Tasks:
- [Task 1]
- [Task 2]
...

If the requirements are still too vague after reviewing user answers, you MUST append a '# Clarifying Questions' section at the end. 

CRITICAL: If you have ANY uncertainty or if the user's input is a single sentence bug report without steps to reproduce, DO NOT proceed to Backlog. Append your questions and move the story to 'Clarification Required'.

If the user has provided 'ADDITIONAL CONTEXT FROM USER' below, you MUST integrate those answers into a FINAL story. DO NOT repeat the same questions if the user has provided answers for them. Finalize the story into 'Backlog' ONLY if the context is sufficient for a real developer to implement it.
";
        
        let msg = if additional_context.is_some() {
            format!("Raw Requirement: {}", story.title)
        } else {
            format!("Raw Requirement: {}\n\n{}", story.title, story.description.as_deref().unwrap_or(""))
        };
        (prompt.to_string(), msg)
    } else if story.status == "Backlog" {
        let prompt = "You are Sandman Tester (Test Lead). Your role is to prepare the verification suite before implementation begins.
YOU MUST:
1. READ the story description and Acceptance Criteria.
2. Identify the target project language and testing framework.
3. Use <tool:write_file> to create detailed test scripts in the project's 'tests/' or equivalent directory.
4. MAINTAIN 'docs/TESTING.md': Document how to run the tests you created.
5. Once the test stubs/scripts are created, summarize the test plan and MOVE the story to 'To-Do' so the Architect can begin planning.
6. IF SYSTEM SETUP IS MISSING: Add a '**MANUAL INTERVENTION REQUIRED**' section with 'npm install' or 'cargo build'.

STORY: {TITLE}
DESCRIPTION: {DESC}";
        let msg = prompt
            .replace("{ID}", &story.id)
            .replace("{TITLE}", &story.title)
            .replace("{DESC}", story.description.as_deref().unwrap_or(""));
        (prompt.to_string(), msg)
    } else if story.status == "To Do" {
        let prompt = "You are Sandman Architect. Analyze the story and generate a formal implementation plan.

AVAILABLE TOOLS:
- <tool:create_task>title</tool>
- <tool:write_file>path | content</tool>

DOCUMENTATION RESPONSIBLITIES:
- You MUST maintain 'docs/ARCH.md' (Overall architecture overview).
- You MUST maintain 'docs/DEPLOY.md' (Deployment guide, Docker, CI/CD).
- You MUST update 'README.md' at the project root to reflect current features.
- You MUST create 'docs/{ID}_PLAN.md' with the high-level architecture and task breakdown for THIS specific story.

YOU MUST:
1. READ the story's '# Tasks' section carefully.
2. Register EVERY identified task into the story checklist using <tool:create_task>.
3. Update all documentation files mentioned above BEFORE moving the story to 'In Progress'.
4. Confirm that the implementation plan is ready and the documentation is up to date.

STORY: {TITLE}
DESCRIPTION: {DESC}";
        let msg = prompt
            .replace("{ID}", &story.id)
            .replace("{TITLE}", &story.title)
            .replace("{DESC}", story.description.as_deref().unwrap_or(""));
        (prompt.to_string(), msg)
    } else if story.status == "In Progress" {
        let prompt = "You are Sandman Builder, a tech-agnostic autonomous engineer. 
Follow the 'Antigravity Plan-Act-Verify' workflow.

AVAILABLE TOOLS:
- <tool:read_file>path</tool>
- <tool:write_file>path | content</tool>
- <tool:apply_patch>path | old | new</tool>
- <tool:run_command> [command] (Swiss army knife: use for git, complex bash, or one-off tests)
- <tool:search_code> [query] (Semantic/RAG search for high-level concepts)
- <tool:grep_search> [query] (Exact string search across all files)
- <tool:list_files> [void] (Recursively list all project files)
- <tool:manage_fs> [op | path | optional_dest] (Operations: mkdir, move, delete)
- <tool:update_task>task_id | completed_bool</tool>
- <tool:update_story>new_status | summary_of_work</tool>

YOU MUST:
1. Read 'docs/{ID}_PLAN.md' using <tool:read_file> to understand the mission.
2. Implement the changes using <tool:write_file> and <tool:apply_patch>.
3. After every major change, run a COMPILING build/test command (e.g., 'npm run build' or 'cargo check') using <tool:run_command> to verify stability. 
CRITICAL: DO NOT run 'npm run dev' or any command that starts a persistent server via <tool:run_command>, as it will hang the agent loop. Verification commands MUST terminate.
5. NEVER use placeholders for imports. If a file is missing, CREATE IT.
6. PROACTIVELY use <tool:update_story> to move the story to 'Testing' once the build succeeds and tasks are done.
7. Use <tool:update_task> to check off tasks as you complete them.
8. IF ANY STEP REQUIRES MANUAL USER ACTION (e.g. running 'npm install'), include a '**MANUAL INTERVENTION REQUIRED**' section at the end.

STORY: {TITLE}
DESCRIPTION: {DESC}
{FEEDBACK}";
        let feedback_text = if let Some(fb) = &story.reviewer_feedback {
            format!("\n### PREVIOUS REVIEWER FEEDBACK (FIX THESE ISSUES):\n{}", fb)
        } else {
            String::new()
        };
        let msg = prompt
            .replace("{ID}", &story.id)
            .replace("{TITLE}", &story.title)
            .replace("{DESC}", story.description.as_deref().unwrap_or(""))
            .replace("{FEEDBACK}", &feedback_text);
        (prompt.to_string(), msg)
    } else if story.status == "Review" {
        let prompt = "You are Sandman Reviewer. Your role is a Quality Gatekeeper.
YOU MUST:
1. READ the code changes and the planning artifact in docs/ to verify alignment.
2. RUN a verification command (e.g., 'npm test', 'cargo check', or a build command) using <tool:run_command>.
3. Only if the verification command succeeds and the code is perfect, use <tool:update_story> to move the story to 'Testing'.
4. IF VERIFICATION FAILS: Use <tool:update_story> to move the story back to 'In Progress' and provide detailed feedback.
5. IF SYSTEM SETUP IS MISSING (e.g. 'npm install', 'cargo build'): You MUST explicitly include a section titled '**MANUAL INTERVENTION REQUIRED**' at the end of your response with the exact command for the user to run in their terminal.

STORY: {TITLE}
DESCRIPTION: {DESC}";
        let msg = prompt
            .replace("{ID}", &story.id)
            .replace("{TITLE}", &story.title)
            .replace("{DESC}", story.description.as_deref().unwrap_or(""));
        (prompt.to_string(), msg)
    } else if story.status == "Testing" {
        let prompt = "You are Sandman Verification Auditor (QA). Your mission is to PROVE that the implementation is perfect before it reaches the user.
        
AVAILABLE TOOLS:
- <tool:run_command> (Use this to verify tests, build, or start the app)
- <tool:read_file> (Use this to audit the code)
- <tool:update_story> 'Documentation | [Success Summary]' OR 'In Progress | [Reason for rejection]'

YOU MUST:
1. READ 'docs/{ID}_PLAN.md' and compare it with the actual project files.
2. RUN a build command (e.g., 'npm run build' or 'cargo check') using <tool:run_command>. 
CRITICAL: DO NOT run persistent servers (npm run dev). Commands MUST terminate quickly.
3. IF YOU SEE COMPILATION OR IMPORT ERRORS (like 'Failed to resolve import' or 'File not found'): You MUST reject the implementation and move the story back to 'In Progress'.
4. Provide the exact error log in your feedback.
5. ONLY move to 'Documentation' if the build SUCCEEDS with no errors.

STORY: {TITLE}
DESCRIPTION: {DESC}";
        let msg = prompt
            .replace("{ID}", &story.id)
            .replace("{TITLE}", &story.title)
            .replace("{DESC}", story.description.as_deref().unwrap_or(""));
        (prompt.to_string(), msg)
    } else if story.status == "Documentation" {
        let prompt = "You are Sandman Technical Writer. Your mission is to ensure the project has high-fidelity documentation.
        
AVAILABLE TOOLS:
- <tool:read_file> (Read project files to understand the final implementation)
- <tool:write_file> (Maintain the docs/ folder)
- <tool:update_story> 'Done | [Documentation Summary]'

YOU MUST:
1. READ the code and the original 'docs/{ID}_PLAN.md'.
2. MAINTAIN 'docs/USER_GUIDE.md': Add or update instructions for this specific feature.
3. MAINTAIN 'docs/CHANGELOG.md': Append a professional entry describing what was added/fixed.
4. ENSURE the project 'README.md' at the root is clean and reflects the total feature set.
5. Once your documentation polish is complete, move the story to 'Done'.

STORY: {TITLE}
DESCRIPTION: {DESC}";
        let msg = prompt
            .replace("{ID}", &story.id)
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
    
    // Inject discovery result last so it's fresh in memory
    user_msg = format!("{}\n{}", user_msg, discovery_context);
    
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
        let config = load_config(&app_clone);
        let preferred_provider = config.column_strategies.get(&story.status).map(|s| s.as_str());

        let mut loop_count = 0;
        let max_loops = 10;
        let mut final_response = String::new();
        let initial_status = story.status.clone();

        while loop_count < max_loops {
            // Check for manual move override
            let current_story: Result<Story, _> = sqlx::query_as("SELECT * FROM stories WHERE id = ?").bind(&id_clone).fetch_one(&pool_clone).await;
            if let Ok(s) = current_story {
                if s.status != initial_status {
                    let _ = app_clone.emit("log", format!("\x1b[90m[Agent]\x1b[0m Story {} moved manually. Aborting AI turn.", id_clone));
                    return;
                }
            }

            let _ = app_clone.emit("log", format!("\x1b[90m[Agent]\x1b[0m Thinking (Story {} - Turn {})...", id_clone, loop_count + 1));
            match call_llm(&app_clone, conversation.clone(), preferred_provider).await {
                Ok(response) => {
                    final_response = response.clone();
                    
                    if let Some((tool_name, args)) = parse_tool_call(&response) {
                        let _ = app_clone.emit("log", format!("\x1b[35m[Agent]\x1b[0m Executing {}: {}", tool_name, args.chars().take(50).collect::<String>()));
                        
                        let result = match tool_name.as_str() {
                            "read_file" => {
                                let path = args.trim();
                                let _ = app_clone.emit("log", format!("\x1b[34m[Agent]\x1b[0m Reading file: {}", path));
                                read_file_internal(path, &state).await
                            },
                            "write_file" => {
                                let parts: Vec<&str> = args.splitn(2, '|').collect();
                                if parts.len() == 2 {
                                    let path = parts[0].trim();
                                    let _ = app_clone.emit("log", format!("\x1b[32m[Agent]\x1b[0m Writing file: {}", path));
                                    write_file_internal(path, parts[1].trim(), &state).await
                                        .map(|_| "Success: File written".to_string())
                                } else {
                                    Err("Format error: use 'path | content'".to_string())
                                }
                            },
                            "apply_patch" => {
                                let parts: Vec<&str> = args.splitn(3, '|').collect();
                                if parts.len() == 3 {
                                    let path = parts[0].trim();
                                    let _ = app_clone.emit("log", format!("\x1b[32m[Agent]\x1b[0m Patching file: {}", path));
                                    apply_patch_internal(path, parts[1].trim(), parts[2].trim(), &state).await
                                        .map(|_| "Success: File patched".to_string())
                                } else {
                                    Err("Format error: use 'path | old_content | new_content'".to_string())
                                }
                            },
                            "run_command" => {
                                if !args.is_empty() {
                                    let cmd = args.trim();
                                    let _ = app_clone.emit("log", format!("\x1b[33m[Agent]\x1b[0m Running command: {}", cmd));
                                    run_project_command_internal(cmd, &state).await
                                } else {
                                    Err("Empty command".to_string())
                                }
                            },
                            "grep_search" => {
                                let query = args.trim();
                                let _ = app_clone.emit("log", format!("\x1b[34m[Agent]\x1b[0m Grep search: {}", query));
                                grep_search_internal(query, &state).await
                            },
                            "list_files" => {
                                let _ = app_clone.emit("log", "\x1b[34m[Agent]\x1b[0m Listing all files...".to_string());
                                list_files_recursive_internal(&state).await
                            },
                            "manage_fs" => {
                                let parts: Vec<&str> = args.splitn(3, '|').collect();
                                if parts.len() >= 2 {
                                    let op = parts[0].trim();
                                    let path = parts[1].trim();
                                    let dest = parts.get(2).map(|s| s.trim());
                                    let _ = app_clone.emit("log", format!("\x1b[32m[Agent]\x1b[0m File Operation: {} on {}", op, path));
                                    manage_filesystem_internal(op, path, dest, &state).await
                                } else {
                                    Err("Format error: use 'op | path | (optional_dest)'".to_string())
                                }
                            },
                            "search_code" => {
                                let query = args.trim();
                                let _ = app_clone.emit("log", format!("\x1b[34m[Agent]\x1b[0m Searching code for: {}", query));
                                search_code_internal(query, &state).await
                            },
                            "create_task" => {
                                let title = args.trim();
                                let _ = app_clone.emit("log", format!("\x1b[36m[Agent]\x1b[0m Creating task: {}", title));
                                sqlx::query("INSERT INTO story_tasks (story_id, title) VALUES (?, ?)")
                                    .bind(&id_clone)
                                    .bind(title)
                                    .execute(&pool_clone)
                                    .await
                                    .map(|_| "Task created".to_string())
                                    .map_err(|e| e.to_string())
                            },
                            "update_task" => {
                                let parts: Vec<&str> = args.splitn(2, '|').collect();
                                if parts.len() == 2 {
                                    match parts[0].trim().parse::<i64>() {
                                        Ok(task_id) => {
                                            let comp = parts[1].trim() == "true";
                                            let _ = app_clone.emit("log", format!("\x1b[36m[Agent]\x1b[0m {}: Task ID {}", if comp { "Completing" } else { "Reopening" }, task_id));
                                            sqlx::query("UPDATE story_tasks SET completed = ? WHERE id = ?")
                                                .bind(if comp { 1i64 } else { 0i64 })
                                                .bind(task_id)
                                                .execute(&pool_clone)
                                                .await
                                                .map(|_| "Task updated".to_string())
                                                .map_err(|e| e.to_string())
                                        },
                                        Err(e) => Err(e.to_string()),
                                    }
                                } else {
                                    Err("Format error: use 'id | completed_bool'".to_string())
                                }
                            },
                            "update_story" => {
                                 let parts: Vec<&str> = args.splitn(2, '|').collect();
                                 if parts.len() == 2 {
                                     let new_status = parts[0].trim();
                                     let feedback = parts[1].trim();
                                     let _ = app_clone.emit("log", format!("\x1b[35m[Agent]\x1b[0m Lifecycle move: -> {}", new_status));
                                     sqlx::query("UPDATE stories SET status = ?, reviewer_feedback = ?, state = 'idle', ai_ready = 1 WHERE id = ?")
                                         .bind(new_status)
                                         .bind(feedback)
                                         .bind(&id_clone)
                                         .execute(&pool_clone)
                                         .await
                                         .map(|_| format!("Story status updated to: {}", new_status))
                                         .map_err(|e| e.to_string())
                                 } else {
                                     Err("Format error: use 'new_status | feedback_or_summary'".to_string())
                                 }
                             },
                            _ => Err("Unknown tool".to_string()),
                        };

                        let tool_msg = match result {
                            Ok(r) => {
                                let _ = app_clone.emit("log", format!("\x1b[90m[Agent]\x1b[0m Result: {}", r.chars().take(100).collect::<String>()));
                                format!("TOOL_RESULT: {}", r)
                            },
                            Err(e) => {
                                let _ = app_clone.emit("log", format!("\x1b[31m[Agent]\x1b[0m Error: {}", e));
                                format!("TOOL_ERROR: {}", e)
                            },
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
        
        // Final check for manual move override before committing state change
        let current_story: Result<Story, _> = sqlx::query_as("SELECT * FROM stories WHERE id = ?").bind(&id_clone).fetch_one(&pool_clone).await;
        if let Ok(s) = current_story {
            if s.status != initial_status {
                let _ = app_clone.emit("log", format!("\x1b[90m[Agent]\x1b[0m Story {} moved manually. Aborting commit.", id_clone));
                return;
            }
        }

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

            let next_status = if has_questions && story.skip_clarification == 0 { "Clarification Required" } else { "Backlog" };

            let _ = sqlx::query("UPDATE stories SET title = ?, description = ?, status = ?, state = 'idle', ai_ready = ? WHERE id = ?")
                .bind(&new_title).bind(&response).bind(next_status).bind(if has_questions { 0i64 } else { 1i64 }).bind(&id_clone).execute(&pool_clone).await;
        } else if story.status == "Backlog" {
            let _ = sqlx::query("UPDATE stories SET status = 'To Do', agent = 'Architect', state = 'idle' WHERE id = ?").bind(&id_clone).execute(&pool_clone).await;
        } else if story.status == "To Do" {
            let _ = sqlx::query("UPDATE stories SET status = 'In Progress', agent = 'Builder', state = 'idle' WHERE id = ?").bind(&id_clone).execute(&pool_clone).await;
        } else if story.status == "In Progress" {
            let _ = app_clone.emit("log", format!("\x1b[36m[Agent]\x1b[0m Builder finished work on {}. Submitting for Testing.", id_clone));
            let _ = sqlx::query("UPDATE stories SET status = 'Testing', agent = 'Reviewer', state = 'idle', reviewer_feedback = ? WHERE id = ?")
                .bind(&response)
                .bind(&id_clone)
                .execute(&pool_clone)
                .await;
        } else if story.status == "Testing" {
            // Check if Auditor reported failure
            let is_failure = response.to_lowercase().contains("verification failed") || 
                            response.to_lowercase().contains("requires more work") ||
                            response.to_lowercase().contains("rejection") ||
                            response.to_lowercase().contains("error") ||
                            response.to_lowercase().contains("failure") ||
                            response.to_lowercase().contains("rejected");

            if is_failure {
                let _ = app_clone.emit("log", "\x1b[31m[Auditor]\x1b[0m Verification failed. Returning to Builder for corrections.");
                let _ = sqlx::query("UPDATE stories SET status = 'In Progress', agent = 'Builder', state = 'idle', reviewer_feedback = ? WHERE id = ?")
                    .bind(&response)
                    .bind(&id_clone)
                    .execute(&pool_clone)
                    .await;
            } else {
                let _ = app_clone.emit("log", "\x1b[32m[Auditor]\x1b[0m Verification successful. Moving to DONE.");
                let _ = sqlx::query("UPDATE stories SET status = 'Done', agent = NULL, state = 'success', reviewer_feedback = ? WHERE id = ?")
                    .bind(&response)
                    .bind(&id_clone)
                    .execute(&pool_clone)
                    .await;
            }
        } else {
            let _ = sqlx::query("UPDATE stories SET state = 'success', reviewer_feedback = ? WHERE id = ?")
                .bind(&response)
                .bind(&id_clone)
                .execute(&pool_clone)
                .await;
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

#[allow(dead_code)]
#[tauri::command]
async fn pty_write(data: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    println!("PTY Write: {}", data);
    let mut guard = state.pty_writer.lock().unwrap();
    if let Some(writer) = guard.as_mut() {
        writer.write_all(data.as_bytes()).map_err(|e| e.to_string())?;
        writer.flush().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("PTY not initialized".to_string())
    }
}

#[tauri::command]
async fn pty_resize(cols: u16, rows: u16, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let guard = state.pty_master.lock().unwrap();
    if let Some(master) = guard.as_ref() {
        master.resize(portable_pty::PtySize {
            cols,
            rows,
            pixel_width: 0,
            pixel_height: 0,
        }).map_err(|e| e.to_string())
    } else {
        Err("PTY not initialized".to_string())
    }
}

#[tauri::command]
async fn init_pty(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let project_path = {
        let guard = state.project_path.lock().unwrap();
        guard.clone()
    }.unwrap_or_else(|| ".".to_string());

    let pty_system = portable_pty::native_pty_system();
    let pair = pty_system.openpty(portable_pty::PtySize {
        cols: 80,
        rows: 24,
        pixel_width: 0,
        pixel_height: 0,
    }).map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    let shell = "cmd.exe";
    #[cfg(not(target_os = "windows"))]
    let shell = "bash";

    let mut cmd = portable_pty::CommandBuilder::new(shell);
    cmd.cwd(&project_path);
    
    let mut _child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
    
    // Release the slave as we only need the master
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    {
        let mut master_guard = state.pty_master.lock().unwrap();
        *master_guard = Some(pair.master);
        let mut writer_guard = state.pty_writer.lock().unwrap();
        *writer_guard = Some(writer);
    }

    let app_clone = app.clone();
    let _ = app.emit("log", "\x1b[36m[System]\x1b[0m Initializing interactive terminal session...");
    
    std::thread::spawn(move || {
        let mut buffer = [0u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buffer[..n]).to_string();
                    let _ = app_clone.emit("pty-stdout", data);
                }
                Err(_) => break,
            }
        }
    });

    println!("PTY Initialized for: {}", project_path);
    Ok(())
}

#[tauri::command]
async fn switch_project(path: String, app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<String, String> {
    println!("Switched active project to: {}", path);
    *state.project_path.lock().unwrap() = Some(path.clone());

    // 1. Setup standard project structure
    setup_project_structure(&path);

    // 2. Connect to SQLite db and embed in .sandman/
    let pool = db::init_db(&path).await?;
    
    // Clear any ghost processing states from unexpected shutdowns
    let _ = sqlx::query("UPDATE stories SET state = 'idle' WHERE state = 'processing'").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE stories ADD COLUMN reviewer_feedback TEXT").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE stories ADD COLUMN skip_clarification INTEGER DEFAULT 0").execute(&pool).await;
    
    *state.db.lock().unwrap() = Some(pool.clone());

    // Emit connection logs
    let _ = app.emit("log", format!("\x1b[36m[System]\x1b[0m Switched active project to: {}", path));
    let _ = app.emit("log", format!("\x1b[32m[System]\x1b[0m SQLite Sandbox embedded at: {}/.sandman/sandman.db", path));
    
    // Auto-start indexing
    let _ = start_indexing(app.clone(), state.clone()).await;
    
    // Auto-init PTY
    let _ = init_pty(app.clone(), state).await;

    Ok(format!("Successfully connected to {}", path))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState { 
            db: Mutex::new(None),
            project_path: Mutex::new(None),
            terminal_pid: Mutex::new(None),
            pty_master: Mutex::new(None),
            pty_writer: Mutex::new(None)
        })
        .invoke_handler(tauri::generate_handler![
            switch_project,
            get_stories,
            get_story_tasks,
            create_story_task,
            update_story_task,
            create_story,
            update_story_status,
            update_story_ready,
            delete_story,
            toggle_story_hold,
            clear_column_state,
            list_files,
            read_file,
            write_file,
            apply_patch,
            run_project_command,
            search_code,
            set_column_strategy,
            get_config,
            save_global_config,
            update_provider,
            start_indexing,
            dispatch_agent,
            chat_with_agent,
            run_terminal_command,
            kill_terminal_command,
            pty_write,
            pty_resize,
            init_pty,
            approve_guarded_action
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
