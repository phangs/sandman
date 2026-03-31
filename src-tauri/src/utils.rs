use crate::AppState;
use std::io::{BufRead, BufReader};
use tauri::{Emitter, Manager};

#[cfg(not(target_os = "windows"))]
use std::process::Command;
#[cfg(target_os = "windows")]
use std::process::Command;

pub async fn kill_terminal_command_internal(state: &tauri::State<'_, AppState>) -> Result<(), String> {
    let pid = {
        let mut guard = state.terminal_pid.lock().unwrap();
        guard.take()
    };

    if let Some(pid) = pid {
        #[cfg(not(target_os = "windows"))]
        {
            let _ = Command::new("kill").arg("-9").arg(pid.to_string()).spawn();
        }
        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("taskkill").arg("/F").arg("/T").arg("/PID").arg(pid.to_string()).spawn();
        }
        Ok(())
    } else {
        Err("No active process to kill".to_string())
    }
}

pub async fn run_terminal_command_internal(command: String, app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
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

        {
            let mut guard = state.terminal_pid.lock().unwrap();
            *guard = Some(child.id());
        }

        let stdout = child.stdout.take().expect("Failed to open stdout");
        let stderr = child.stderr.take().expect("Failed to open stderr");

        let app_stdout = app_clone.clone();
        let app_stderr = app_clone.clone();

        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for l in reader.lines().flatten() {
                let _ = app_stdout.emit("terminal-stdout", format!("{}\n", l));
            }
        });

        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for l in reader.lines().flatten() {
                let _ = app_stderr.emit("terminal-stdout", format!("\x1b[31m{}\x1b[0m\n", l));
            }
        });

        match child.wait() {
            Ok(status) => {
                {
                    let mut guard = state.terminal_pid.lock().unwrap();
                    if *guard == Some(child.id()) {
                        *guard = None;
                    }
                }
                let _ = app_clone.emit("terminal-stdout", format!("\n\x1b[32mProcess finished which code: {}\x1b[0m\n", status));
            },
            Err(e) => {
                let _ = app_clone.emit("terminal-stdout", format!("\n\x1b[31mProcess error: {}\x1b[0m\n", e));
            }
        }
    });

    Ok(())
}
