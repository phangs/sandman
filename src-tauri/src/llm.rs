use serde::{Deserialize, Serialize};
use crate::config::{ProviderConfig, load_config};
use tauri::AppHandle;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

pub async fn call_llm(app: &AppHandle, messages: Vec<Message>) -> Result<String, String> {
    let config = load_config(app);
    
    // Find first active provider
    let provider = config.providers.iter()
        .find(|p| p.active)
        .ok_or("No active LLM provider found in settings")?;

    match provider.id.as_str() {
        "ollama" => call_ollama(provider, messages).await,
        "openai" => call_openai(provider, messages).await,
        _ => Err(format!("Provider {} not yet implemented", provider.id)),
    }
}

async fn call_ollama(provider: &ProviderConfig, messages: Vec<Message>) -> Result<String, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/chat", provider.endpoint);
    
    // Format for Ollama /api/chat
    let body = serde_json::json!({
        "model": &provider.model,
        "messages": messages,
        "stream": false
    });

    let res = client.post(url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Ollama connection failed: {}", e))?;

    let json: serde_json::Value = res.json().await.map_err(|e| format!("Failed to parse JSON: {}", e))?;
    
    if let Some(error) = json.get("error").and_then(|e| e.as_str()) {
        return Err(format!("Ollama Error: {}", error));
    }

    let content = json.get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str());

    match content {
        Some(s) => Ok(s.to_string()),
        None => {
            // Check if it's the legacy format (non-chat)
            if let Some(resp) = json.get("response").and_then(|r| r.as_str()) {
                Ok(resp.to_string())
            } else {
                Err(format!("Ollama malformed response. Raw: {:?}", json))
            }
        }
    }
}

async fn call_openai(provider: &ProviderConfig, messages: Vec<Message>) -> Result<String, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/chat/completions", provider.endpoint);
    let api_key = provider.api_key.as_ref().ok_or("OpenAI API Key is missing")?;

    let body = serde_json::json!({
        "model": &provider.model, 
        "messages": messages,
    });

    let res = client.post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("OpenAI request failed: {}", e))?;

    let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    
    json["choices"][0]["message"]["content"].as_str()
        .map(|s| s.to_string())
        .ok_or("Malformed response from OpenAI".to_string())
}
