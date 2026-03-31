use serde::{Deserialize, Serialize};
use crate::config::{ProviderConfig, load_config};
use tauri::AppHandle;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

pub async fn call_llm(app: &AppHandle, messages: Vec<Message>, preferred_provider_id: Option<&str>) -> Result<String, String> {
    let config = load_config(app);
    
    // 1. Try to find the preferred provider if specified
    // 2. Fall back to first active provider
    let provider = if let Some(pid) = preferred_provider_id {
        config.providers.iter().find(|p| p.id == pid)
            .ok_or(format!("Preferred provider '{}' not found in settings", pid))?
    } else {
        config.providers.iter().find(|p| p.active)
            .ok_or("No active LLM provider found in settings")?
    };

    match provider.id.as_str() {
        "ollama" => call_ollama(provider, messages).await,
        "openai" | "xai" => call_openai(provider, messages).await, // Grok is OpenAI compatible
        "gemini" | "google" => call_gemini(provider, messages).await,
        "anthropic" => call_anthropic(provider, messages).await,
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

async fn call_gemini(provider: &ProviderConfig, messages: Vec<Message>) -> Result<String, String> {
    let client = reqwest::Client::new();
    let api_key = provider.api_key.as_ref().ok_or("Gemini API Key is missing")?;
    
    // Gemini endpoint format: models/{model}:generateContent
    let model = if provider.model.is_empty() { "gemini-1.5-flash" } else { &provider.model };
    let url = format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}", model, api_key);

    let mut system_instruction = String::new();
    let mut merged_contents: Vec<serde_json::Value> = Vec::new();
    let mut last_role = String::new();
    let mut last_text = String::new();

    for m in messages {
        if m.role == "system" {
            if !system_instruction.is_empty() {
                system_instruction.push_str("\n\n");
            }
            system_instruction.push_str(&m.content);
            continue;
        }

        let current_role = if m.role == "assistant" { "model" } else { "user" };
        if current_role == last_role {
            last_text.push_str("\n\n");
            last_text.push_str(&m.content);
        } else {
            if !last_text.is_empty() {
                merged_contents.push(serde_json::json!({
                    "role": last_role,
                    "parts": [{ "text": last_text }]
                }));
            }
            last_role = current_role.to_string();
            last_text = m.content.clone();
        }
    }
    
    if !last_text.is_empty() {
        merged_contents.push(serde_json::json!({
            "role": last_role,
            "parts": [{ "text": last_text }]
        }));
    }

    let mut body = serde_json::json!({ "contents": merged_contents });
    
    if !system_instruction.is_empty() {
        body.as_object_mut().unwrap().insert(
            "system_instruction".to_string(), 
            serde_json::json!({ "parts": [{ "text": system_instruction }] })
        );
    }


    let res = client.post(url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Gemini request failed: {}", e))?;

    let json: serde_json::Value = res.json().await.map_err(|e| format!("Failed to parse Gemini JSON: {}", e))?;
    
    // Explicitly check for an error object in the response
    if let Some(error) = json.get("error") {
        let msg = error.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown Gemini Error");
        let status = error.get("status").and_then(|s| s.as_str()).unwrap_or("UNKNOWN_STATUS");
        return Err(format!("Gemini API Error ({}): {}", status, msg));
    }

    json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or(format!("Malformed response from Gemini. Candidates missing content. Raw: {:?}", json))
}

async fn call_anthropic(provider: &ProviderConfig, messages: Vec<Message>) -> Result<String, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/messages", provider.endpoint);
    let api_key = provider.api_key.as_ref().ok_or("Anthropic API Key is missing")?;

    let body = serde_json::json!({
        "model": &provider.model,
        "max_tokens": 4096,
        "messages": messages,
    });

    let res = client.post(url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Anthropic request failed: {}", e))?;

    let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    
    json["content"][0]["text"].as_str()
        .map(|s| s.to_string())
        .ok_or(format!("Malformed response from Anthropic. Raw: {:?}", json))
}
