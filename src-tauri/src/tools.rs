use crate::AppState;
// use std::io::{BufRead, BufReader};
use ignore::WalkBuilder;

pub async fn read_file_internal(path: &str, state: &tauri::State<'_, AppState>) -> Result<String, String> {
    let project_path = {
        let guard = state.project_path.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;

    let full_path = std::path::PathBuf::from(&project_path).join(path);
    if !full_path.starts_with(&project_path) {
        return Err("Access denied: Path outside project".to_string());
    }

    if !full_path.exists() {
        return Err(format!("File not found at '{}'. If this file is required for your mission, you MUST create it first using the <tool:write_file> tool.", path));
    }
    std::fs::read_to_string(full_path).map_err(|e| e.to_string())
}

pub async fn write_file_internal(path: &str, content: &str, state: &tauri::State<'_, AppState>) -> Result<(), String> {
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

pub async fn apply_patch_internal(path: &str, old_content: &str, new_content: &str, state: &tauri::State<'_, AppState>) -> Result<(), String> {
    let current_content = read_file_internal(path, state).await?;
    
    if !current_content.contains(old_content) {
        return Err("Target content not found in file. Please ensure old_content matches exactly.".to_string());
    }

    let patched_content = current_content.replace(old_content, new_content);
    write_file_internal(path, &patched_content, state).await
}

pub async fn run_project_command_internal(full_command: &str, state: &tauri::State<'_, AppState>) -> Result<String, String> {
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
            Err("Command TIMEOUT (30s): The command was killed because it took too long.".to_string())
        }
    }
}

pub async fn search_code_internal(query: &str, state: &tauri::State<'_, AppState>) -> Result<String, String> {
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

pub async fn grep_search_internal(query: &str, state: &tauri::State<'_, AppState>) -> Result<String, String> {
    let project_path = {
        let guard = state.project_path.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;

    let mut results = Vec::new();
    let walker = WalkBuilder::new(&project_path)
        .hidden(true)
        .git_ignore(true)
        .build();

    for entry in walker.flatten() {
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

    if results.is_empty() {
        Ok("No matches found.".to_string())
    } else {
        Ok(results.join("\n"))
    }
}

pub async fn list_files_recursive_internal(path: &str, state: &tauri::State<'_, AppState>) -> Result<String, String> {
    let project_path = {
        let guard = state.project_path.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;

    let target_path = if path.is_empty() || path == "void" {
        std::path::PathBuf::from(&project_path)
    } else {
        std::path::PathBuf::from(&project_path).join(path)
    };

    if !target_path.exists() {
        return Err(format!("Path not found: {}", path));
    }
    if !target_path.starts_with(&project_path) {
        return Err("Access denied".to_string());
    }

    let mut files = Vec::new();
    let walker = WalkBuilder::new(&target_path)
        .hidden(false)
        .git_ignore(true)
        .build();

    for entry in walker.flatten() {
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
        if files.len() > 150 { 
            files.push("... list truncated. Please narrow your search.".to_string());
            break; 
        }
    }

    Ok(files.join("\n"))
}

pub async fn manage_filesystem_internal(op: &str, path: &str, dest: Option<&str>, state: &tauri::State<'_, AppState>) -> Result<String, String> {
    let project_path = {
        let guard = state.project_path.lock().unwrap();
        guard.clone()
    }.ok_or("No project connected")?;

    let full_path = std::path::PathBuf::from(&project_path).join(path);
    if !full_path.starts_with(&project_path) {
        return Err("Access denied".to_string());
    }

    match op {
        "mkdir" => {
            std::fs::create_dir_all(&full_path).map_err(|e| e.to_string())?;
            Ok("Directory created".to_string())
        },
        "delete" => {
            if full_path.is_dir() {
                std::fs::remove_dir_all(&full_path).map_err(|e| e.to_string())?;
                Ok("Directory deleted".to_string())
            } else {
                std::fs::remove_file(&full_path).map_err(|e| e.to_string())?;
                Ok("File deleted".to_string())
            }
        },
        "move" => {
            if let Some(d) = dest {
                let full_dest = std::path::PathBuf::from(&project_path).join(d);
                if !full_dest.starts_with(&project_path) {
                    return Err("Access denied for destination".to_string());
                }
                if let Some(parent) = full_dest.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                std::fs::rename(&full_path, &full_dest).map_err(|e| e.to_string())?;
                Ok("Item moved".to_string())
            } else {
                Err("Move operation requires <dest> argument".to_string())
            }
        },
        _ => Err("Invalid operation".to_string())
    }
}
