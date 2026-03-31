use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri::Manager;

use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub providers: Vec<ProviderConfig>,
    pub selected_model_group: String,
    #[serde(default)]
    pub column_strategies: HashMap<String, String>, // story status -> provider id
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub api_key: Option<String>,
    pub endpoint: String,
    pub active: bool,
    pub model: String,
}

impl Default for Config {
    fn default() -> Self {
        let mut column_strategies = HashMap::new();
        column_strategies.insert("Raw Requirements".to_string(), "ollama".to_string());
        column_strategies.insert("Clarification Required".to_string(), "ollama".to_string());
        column_strategies.insert("To Do".to_string(), "ollama".to_string());
        column_strategies.insert("In Progress".to_string(), "ollama".to_string());
        column_strategies.insert("Review".to_string(), "ollama".to_string());

        Self {
            selected_model_group: "Local & Efficient".to_string(),
            column_strategies,
            providers: vec![
                ProviderConfig {
                    id: "ollama".to_string(),
                    name: "Ollama (Local)".to_string(),
                    api_key: None,
                    endpoint: "http://localhost:11434".to_string(),
                    active: true,
                    model: "llama3".to_string(), // Safer default
                },
                ProviderConfig {
                    id: "openai".to_string(),
                    name: "OpenAI".to_string(),
                    api_key: None,
                    endpoint: "https://api.openai.com/v1".to_string(),
                    active: false,
                    model: "gpt-4o".to_string(),
                },
                ProviderConfig {
                    id: "anthropic".to_string(),
                    name: "Anthropic (Claude)".to_string(),
                    api_key: None,
                    endpoint: "https://api.anthropic.com/v1".to_string(),
                    active: false,
                    model: "claude-3-5-sonnet-latest".to_string(),
                },
                ProviderConfig {
                    id: "gemini".to_string(),
                    name: "Google Gemini".to_string(),
                    api_key: None,
                    endpoint: "https://generativelanguage.googleapis.com".to_string(),
                    active: false,
                    model: "gemini-1.5-flash".to_string(),
                },
                ProviderConfig {
                    id: "xai".to_string(),
                    name: "XAI Grok".to_string(),
                    api_key: None,
                    endpoint: "https://api.x.ai/v1".to_string(),
                    active: false,
                    model: "grok-beta".to_string(),
                },
            ],
        }
    }
}

pub fn get_config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let mut config_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
    }
    
    config_dir.push("config.json");
    Ok(config_dir)
}

pub fn load_config(app: &AppHandle) -> Config {
    let path = match get_config_path(app) {
        Ok(p) => p,
        Err(_) => return Config::default(),
    };

    if !path.exists() {
        return Config::default();
    }

    let data = match fs::read_to_string(path) {
        Ok(d) => d,
        Err(_) => return Config::default(),
    };

    let mut config: Config = serde_json::from_str(&data).unwrap_or_else(|_| Config::default());
    
    // Merge new providers from default if missing
    let default_config = Config::default();
    for dp in default_config.providers {
        if !config.providers.iter().any(|p| p.id == dp.id) {
            config.providers.push(dp);
        }
    }

    config
}

pub fn save_config(app: &AppHandle, config: &Config) -> Result<(), String> {
    let path = get_config_path(app)?;
    let data = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(path, data).map_err(|e| e.to_string())?;
    Ok(())
}
