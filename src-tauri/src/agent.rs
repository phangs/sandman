use crate::{AppState, Story, Message, call_llm, load_config};
use crate::tools::*;
use crate::prompts::get_agent_prompt;
use sqlx::SqlitePool;
use tauri::{Emitter, Manager};

pub async fn dispatch_agent_internal(
    id: String,
    story: Story,
    pool: SqlitePool,
    app: tauri::AppHandle,
    discovery_context: String,
    rag_context: String,
    additional_context: Option<String>
) {
    let (system_prompt, mut user_msg) = get_agent_prompt(&story, &pool).await;

    if let Some(ctx) = additional_context {
        user_msg = format!("{}\n\nADDITIONAL CONTEXT FROM USER:\n{}", user_msg, ctx);
    }
    
    user_msg = format!("{}\n{}", user_msg, discovery_context);
    if !rag_context.is_empty() {
        user_msg = format!("{}\n{}", user_msg, rag_context);
    }
    
    let mut conversation = vec![
        Message { role: "system".to_string(), content: system_prompt },
        Message { role: "user".to_string(), content: user_msg },
    ];

    let app_clone = app.clone();
    let id_clone = id.clone();
    let pool_clone = pool.clone();

    tauri::async_runtime::spawn(async move {
        let state = app_clone.state::<AppState>();
        let config = load_config(&app_clone);
        let preferred_provider = config.column_strategies.get(&story.status).map(|s| s.as_str());

        let mut loop_count = 0;
        let mut consecutive_non_action_turns = 0;
        let mut turns_since_task_update = 0;
        let mut tool_history: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut final_response = String::new();
        let initial_status = story.status.clone();
        
        let max_loops = if initial_status == "In Progress" { 40 } else { 20 };

        while loop_count < max_loops {
            let current_story: Result<Story, _> = sqlx::query_as("SELECT * FROM stories WHERE id = ?").bind(&id_clone).fetch_one(&pool_clone).await;
            if let Ok(s) = current_story {
                if s.status != initial_status {
                    let _ = app_clone.emit("log", format!("\x1b[90m[Agent]\x1b[0m Story {} moved manually. Aborting turn.", id_clone));
                    return;
                }
            }

            let _ = app_clone.emit("log", format!("\x1b[90m[Agent]\x1b[0m Thinking (Story {} - Turn {})...", id_clone, loop_count + 1));
            match call_llm(&app_clone, conversation.clone(), preferred_provider).await {
                Ok(response) => {
                    final_response = response.clone();
                    let thoughts = response.chars().take(300).collect::<String>().replace('\n', " ");
                    let _ = app_clone.emit("log", format!("\x1b[90m[Agent]\x1b[0m Thoughts: \"{}...\"", thoughts));

                    if turns_since_task_update >= 10 && story.status == "In Progress" {
                        conversation.push(Message { role: "user".to_string(), content: "SYSTEM REMINDER: Please remember to use <tool:update_task> to check off subtasks you have completed.".to_string() });
                        turns_since_task_update = 0;
                    }

                    let tool_calls = parse_all_tool_calls(&response);
                    
                    if !tool_calls.is_empty() {
                        conversation.push(Message { role: "assistant".to_string(), content: response });
                        
                        let mut results = Vec::new();
                        for (tool_name, args) in tool_calls {
                            let tool_key = format!("{}({})", tool_name, args.chars().take(200).collect::<String>());
                            if tool_history.contains(&tool_key) && ["list_files", "search_code", "read_file"].contains(&tool_name.as_str()) {
                                results.push(format!("TOOL_RESULT: REDUNDANT {} call ignored.", tool_name));
                                continue;
                            }
                            
                            if ["write_file", "apply_patch", "run_command", "manage_fs"].contains(&tool_name.as_str()) { tool_history.clear(); }
                            if tool_name == "update_task" { turns_since_task_update = 0; } else { turns_since_task_update += 1; }
                            
                            tool_history.insert(tool_key);
                            let _ = app_clone.emit("log", format!("\x1b[32m[Agent]\x1b[0m Executing \x1b[1m{}\x1b[0m: {}", tool_name, args.chars().take(100).collect::<String>()));
                            
                            let result = match tool_name.as_str() {
                                "read_file" => read_file_internal(&sanitize_args(&args), &state).await,
                                "write_file" => handle_write_file(&sanitize_args(&args), &state, &app_clone).await,
                                "apply_patch" => handle_apply_patch(&sanitize_args(&args), &state, &app_clone).await,
                                "run_command" => run_project_command_internal(&sanitize_args(&args), &state).await,
                                "grep_search" => grep_search_internal(&sanitize_args(&args), &state).await,
                                "list_files" => list_files_recursive_internal(&sanitize_args(&args), &state).await,
                                "manage_fs" => handle_manage_fs(&sanitize_args(&args), &state, &app_clone).await,
                                "search_code" => search_code_internal(&sanitize_args(&args), &state).await,
                                "create_task" => handle_create_task(&sanitize_args(&args), &id_clone, &pool_clone, &app_clone).await,
                                "create_story" => handle_create_story(&sanitize_args(&args), &pool_clone, &app_clone).await,
                                "update_task" => handle_update_task(&sanitize_args(&args), &id_clone, &pool_clone, &app_clone).await,
                                "manage_artifact" => handle_manage_artifact(&sanitize_args(&args), &id_clone, &pool_clone, &app_clone).await,
                                "update_story" => handle_update_story(&sanitize_args(&args), &id_clone, &pool_clone, &app_clone).await,
                                _ => Err("Unknown tool".to_string()),
                            };

                            match result {
                                Ok(r) => {
                                    let _ = app_clone.emit("log", format!("\x1b[90m[Agent]\x1b[0m Result: {}", r.chars().take(100).collect::<String>()));
                                    results.push(format!("TOOL_RESULT ({}): {}", tool_name, r));
                                },
                                Err(e) => {
                                    let _ = app_clone.emit("log", format!("\x1b[31m[Agent]\x1b[0m Error: {}", e));
                                    results.push(format!("TOOL_ERROR ({}): {}", tool_name, e));
                                },
                            }
                        }
                        
                        let combined_results = results.join("\n\n");
                        conversation.push(Message { role: "user".to_string(), content: combined_results });
                        loop_count += 1;
                        consecutive_non_action_turns = 0;
                    } else {
                        consecutive_non_action_turns += 1;
                        if consecutive_non_action_turns <= 2 {
                             let _ = app_clone.emit("log", format!("\x1b[33m[Agent]\x1b[0m Turn {} lack of action. Nudging...", loop_count + 1));
                             conversation.push(Message { role: "assistant".to_string(), content: response });
                             conversation.push(Message { role: "user".to_string(), content: "NO TOOL CALL DETECTED. PERFORM AN ACTION.".to_string() });
                             loop_count += 1;
                        } else {
                            let _ = app_clone.emit("log", "\x1b[31m[Agent]\x1b[0m Stalled. Aborting.".to_string());
                            break;
                        }
                    }
                },
                Err(e) => {
                    let _ = app_clone.emit("log", format!("\x1b[31m[Agent]\x1b[0m AI Error: {}", e));
                    let _ = sqlx::query("UPDATE stories SET state = 'failed' WHERE id = ?").bind(&id_clone).execute(&pool_clone).await;
                    return;
                }
            }
        }
        
        let _ = app_clone.emit("log", format!("\x1b[90m[Agent]\x1b[0m Story {} turn sequence finished.", id_clone));
        finalize_post_agent(&final_response, &story, &id_clone, &pool_clone, &app_clone).await;
    });
}

// ALL HELPER FUNCTIONS MOVED HERE (handle_write_file, etc.)
// ...
async fn handle_write_file(args: &str, state: &tauri::State<'_, AppState>, _app: &tauri::AppHandle) -> Result<String, String> {
    // Parser logic from lib.rs
    let mut path = String::new();
    let mut content = String::new();
    if args.contains("<file_path>") {
        let p_start = args.find("<file_path>").unwrap() + 11;
        let p_end = args.find("</file_path>").unwrap();
        path = args[p_start..p_end].trim().to_string();
        if let Some(c_start) = args.find("<file_content>") {
            let c_end_opt = args.rfind("</file_content>");
            if let Some(c_end) = c_end_opt {
                content = args[c_start + 14..c_end].trim().to_string();
            } else {
                content = args[c_start + 14..].trim().to_string();
            }
        }
    } else {
        let parts: Vec<&str> = args.splitn(2, '|').collect();
        if parts.len() == 2 {
            path = parts[0].trim().to_string();
            content = parts[1].trim().to_string();
        }
    }
    if !path.is_empty() {
        write_file_internal(&path, &content, state).await.map(|_| "Success: File written".to_string())
    } else {
        Err("Format error".to_string())
    }
}

async fn handle_apply_patch(args: &str, state: &tauri::State<'_, AppState>, _app: &tauri::AppHandle) -> Result<String, String> {
    // Parser logic from lib.rs
    let mut path = String::new();
    let mut old_content = String::new();
    let mut new_content = String::new();
    if args.contains("<file_path>") {
        let p_end = args.find("</file_path>").unwrap();
        path = args[args.find("<file_path>").unwrap() + 11..p_end].trim().to_string();
        if let Some(o_start) = args.find("<file_old_content>") {
            if let Some(o_end) = args.rfind("</file_old_content>") {
                old_content = args[o_start + 18..o_end].to_string();
            }
        }
        if let Some(n_start) = args.find("<file_new_content>") {
            if let Some(n_end) = args.rfind("</file_new_content>") {
                new_content = args[n_start + 18..n_end].to_string();
            }
        }
    }
    if !path.is_empty() {
        apply_patch_internal(&path, &old_content, &new_content, state).await.map(|_| "Success: File patched".to_string())
    } else {
        Err("Format error".to_string())
    }
}

async fn handle_manage_fs(args: &str, state: &tauri::State<'_, AppState>, _app: &tauri::AppHandle) -> Result<String, String> {
    let parts: Vec<&str> = args.splitn(3, '|').collect();
    if parts.len() >= 2 {
        manage_filesystem_internal(parts[0].trim(), parts[1].trim(), parts.get(2).map(|s| s.trim()), state).await
    } else {
        Err("Format error".to_string())
    }
}

async fn handle_create_task(args: &str, id: &str, pool: &SqlitePool, _app: &tauri::AppHandle) -> Result<String, String> {
    sqlx::query("INSERT INTO story_tasks (story_id, title) VALUES (?, ?)")
        .bind(id)
        .bind(args.trim())
        .execute(pool)
        .await
        .map(|_| "Task created".to_string())
        .map_err(|e| e.to_string())
}

async fn handle_create_story(args: &str, pool: &SqlitePool, _app: &tauri::AppHandle) -> Result<String, String> {
    let parts: Vec<&str> = args.splitn(2, '|').collect();
    if parts.len() == 2 {
        let new_id = format!("S-{}", uuid::Uuid::new_v4().to_string()[..6].to_uppercase());
        sqlx::query("INSERT INTO stories (id, title, description, status, ai_ready, state) VALUES (?, ?, ?, 'Raw Requirements', 1, 'idle')")
            .bind(&new_id)
            .bind(parts[0].trim())
            .bind(parts[1].trim())
            .execute(pool)
            .await
            .map(|_| format!("Story created: {}", new_id))
            .map_err(|e| e.to_string())
    } else {
        Err("Format error".to_string())
    }
}

async fn handle_update_task(args: &str, id: &str, pool: &SqlitePool, app: &tauri::AppHandle) -> Result<String, String> {
    // Positional/XML parsing logic from lib.rs
    let raw_id;
    let mut comp = true;
    if args.contains("<id>") {
        raw_id = args.split("<id>").nth(1).unwrap().split("</id>").next().unwrap().trim().to_string();
        if let Some(c) = args.split("<completed>").nth(1) {
            let s = c.split("</completed>").next().unwrap().trim().to_lowercase();
            comp = s == "true" || s == "1" || s == "completed";
        }
    } else {
        let parts: Vec<&str> = args.splitn(2, '|').collect();
        raw_id = parts[0].trim().to_string();
        comp = parts.get(1).map(|s| s.trim().to_lowercase() == "true").unwrap_or(true);
    }

    let task_id: i64 = if let Ok(n) = raw_id.parse() { n } else {
        let pos = raw_id.trim_start_matches(|c: char| !c.is_ascii_digit()).parse::<i64>().unwrap_or(0);
        use sqlx::Row;
        sqlx::query("SELECT id FROM story_tasks WHERE story_id = ? ORDER BY id LIMIT 1 OFFSET ?").bind(id).bind(pos-1).fetch_one(pool).await.map(|r| r.get(0)).unwrap_or(0)
    };
    
    sqlx::query("UPDATE story_tasks SET completed = ? WHERE id = ?").bind(if comp { 1 } else { 0 }).bind(task_id).execute(pool).await
        .map(|_| { let _ = app.emit("refresh_board", ()); "Task updated".to_string() })
        .map_err(|e| e.to_string())
}

async fn handle_manage_artifact(args: &str, id: &str, pool: &SqlitePool, app: &tauri::AppHandle) -> Result<String, String> {
    let mut op = String::new();
    let mut name = String::new();
    let mut content = String::new();
    let mut a_type = String::new();
    if args.contains("<op>") {
        op = args.split("<op>").nth(1).unwrap().split("</op>").next().unwrap().trim().to_string();
        if args.contains("<name>") { name = args.split("<name>").nth(1).unwrap().split("</name>").next().unwrap().trim().to_string(); }
        if args.contains("<content>") { content = args.split("<content>").nth(1).unwrap().split("</content>").next().unwrap().trim().to_string(); }
        if args.contains("<type>") { a_type = args.split("<type>").nth(1).unwrap().split("</type>").next().unwrap().trim().to_string(); }
    }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    match op.as_str() {
        "create" | "update" => {
            let art_id = format!("{}-{}", id, uuid::Uuid::new_v4().to_string()[..6].to_lowercase());
            sqlx::query("INSERT INTO artifacts (id, story_id, name, content, type, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET content=excluded.content, updated_at=excluded.updated_at")
                .bind(&art_id).bind(id).bind(&name).bind(&content).bind(&a_type).bind(now).bind(now).execute(pool).await
                .map(|_| { let _ = app.emit("refresh_artifacts", ()); "Artifact updated".to_string() }).map_err(|e| e.to_string())
        },
        _ => Err("Invalid op".to_string())
    }
}

async fn handle_update_story(args: &str, id: &str, pool: &SqlitePool, app: &tauri::AppHandle) -> Result<String, String> {
    let mut status = String::new();
    let mut feedback = String::new();
    if args.contains("<status>") {
        status = args.split("<status>").nth(1).unwrap().split("</status>").next().unwrap().trim().to_string();
        if args.contains("<feedback>") { feedback = args.split("<feedback>").nth(1).unwrap().split("</feedback>").next().unwrap().trim().to_string(); }
    }
    // Normalization logic
    let norm = match status.to_lowercase().as_str() {
        "todo" => "To Do",
        "progress" | "in progress" => "In Progress",
        "review" => "Review",
        "testing" => "Testing",
        "documentation" => "Documentation",
        "done" => "Done",
        _ => &status
    };
    sqlx::query("UPDATE stories SET status = ?, reviewer_feedback = ?, state = 'idle', ai_ready = 1 WHERE id = ?").bind(norm).bind(&feedback).bind(id).execute(pool).await
        .map(|_| { let _ = app.emit("refresh_board", ()); format!("Story moved to {}", norm) }).map_err(|e| e.to_string())
}

async fn finalize_post_agent(response: &str, story: &Story, id: &str, pool: &SqlitePool, _app: &tauri::AppHandle) {
    if story.status == "Raw Requirements" {
         let clean_desc = response.to_string();
         // simple clean
         sqlx::query("UPDATE stories SET description = ?, ai_ready = 1 WHERE id = ?").bind(&clean_desc).bind(id).execute(pool).await.ok();
    }
}

pub fn parse_all_tool_calls(response: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let mut current_pos = 0;
    let lower_res = response.to_lowercase();
    
    while let Some(start_idx) = lower_res[current_pos..].find("<tool:") {
        let abs_start = current_pos + start_idx;
        let rest = &response[abs_start + 6..];
        let lower_rest = &lower_res[abs_start + 6..];
        
        if let Some(end_name_idx) = lower_rest.find('>') {
            let tool_name = lower_rest[..end_name_idx].trim().to_lowercase();
            let after_name = &rest[end_name_idx + 1..];
            let lower_after_name = &lower_rest[end_name_idx + 1..];
            
            if let Some(end_tag_idx) = lower_after_name.find("</tool") {
                let args = after_name[..end_tag_idx].to_string();
                results.push((tool_name, args.trim().to_string()));
                current_pos = abs_start + 6 + end_name_idx + 1 + end_tag_idx + 7;
            } else {
                // Incomplete tag, just push what we have as args and break
                results.push((tool_name, after_name.to_string()));
                break;
            }
        } else {
            break;
        }
    }
    results
}

fn sanitize_args(args: &str) -> String {
    // Some models hallucinate <path>docs/item.md</path> or <content>blah</content> inside the tool tag
    // We want to strip these common LLM-hallucinated meta-tags if they wrap the entire content or part of it
    let mut sanitized = args.to_string();
    
    let tags_to_check = ["path", "content", "id", "completed", "title", "description", "dest", "op"];
    for tag in tags_to_check {
        let open_tag = format!("<{}>", tag);
        let close_tag = format!("</{}>", tag);
        
        if sanitized.contains(&open_tag) && sanitized.contains(&close_tag) {
             if let Some(start) = sanitized.find(&open_tag) {
                 if let Some(end) = sanitized.find(&close_tag) {
                     let _inner = &sanitized[start + open_tag.len()..end];
                     // Only replace if it looks like a clean tag wrap
                     sanitized = sanitized.replace(&open_tag, "").replace(&close_tag, "");
                 }
             }
        }
    }
    
    sanitized.trim().to_string()
}
