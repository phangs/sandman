use sqlx::SqlitePool;

/// Split file content into overlapping chunks optimized for code.
/// Returns Vec<(start_line, end_line, chunk_text)>
pub fn chunk_file(content: &str, chunk_size: usize, overlap: usize) -> Vec<(usize, usize, String)> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return vec![];
    }

    let mut chunks = Vec::new();
    let mut start = 0;

    while start < lines.len() {
        let end = (start + chunk_size).min(lines.len());
        let chunk_text = lines[start..end].join("\n");
        if !chunk_text.trim().is_empty() {
            chunks.push((start + 1, end, chunk_text));
        }
        if end == lines.len() { break; }
        start += chunk_size.saturating_sub(overlap);
    }

    chunks
}

/// Call Ollama's embedding API (nomic-embed-text or similar).
/// Returns a Vec<f32> embedding.
pub async fn embed_text(text: &str, endpoint: &str) -> Result<Vec<f32>, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/embeddings", endpoint);

    let body = serde_json::json!({
        "model": "nomic-embed-text",
        "prompt": text
    });

    let res = client.post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("Embedding request failed: {}", e))?;

    let json: serde_json::Value = res.json().await
        .map_err(|e| format!("Failed to parse embedding response: {}", e))?;

    let embedding = json["embedding"]
        .as_array()
        .ok_or("No embedding field in response")?
        .iter()
        .map(|v| v.as_f64().unwrap_or(0.0) as f32)
        .collect();

    Ok(embedding)
}

/// Cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() { return 0.0; }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 { return 0.0; }
    dot / (mag_a * mag_b)
}

/// Serialize f32 vec to little-endian bytes for BLOB storage.
pub fn vec_to_blob(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Deserialize BLOB back to f32 vec.
pub fn blob_to_vec(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// Index a single file: chunk it, embed each chunk, upsert into DB.
pub async fn index_file(
    pool: &SqlitePool,
    file_path: &str,
    content: &str,
    embedding_endpoint: &str,
) -> Result<usize, String> {
    // Delete old chunks for this file
    let _ = sqlx::query("DELETE FROM rag_chunks WHERE file_path = ?")
        .bind(file_path)
        .execute(pool)
        .await;

    let chunks = chunk_file(content, 60, 10); // 60 lines, 10-line overlap
    let mut indexed = 0;

    for (start, end, text) in &chunks {
        let embedding = match embed_text(text, embedding_endpoint).await {
            Ok(e) => e,
            Err(_) => continue, // skip if embedding fails silently
        };
        let blob = vec_to_blob(&embedding);

        let _ = sqlx::query(
            "INSERT INTO rag_chunks (file_path, start_line, end_line, content, embedding) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(file_path)
        .bind(*start as i64)
        .bind(*end as i64)
        .bind(text)
        .bind(&blob)
        .execute(pool)
        .await;

        indexed += 1;
    }

    Ok(indexed)
}

/// Query: given a query string, return top-k most semantically similar chunks.
pub async fn search_chunks(
    pool: &SqlitePool,
    query: &str,
    embedding_endpoint: &str,
    top_k: usize,
) -> Result<Vec<(String, String, f32)>, String> {
    let query_vec = embed_text(query, embedding_endpoint).await?;

    use sqlx::Row;
    let rows = sqlx::query("SELECT file_path, content, embedding FROM rag_chunks")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    let mut scored: Vec<(String, String, f32)> = rows
        .into_iter()
        .filter_map(|row| {
            let file_path: String = row.try_get("file_path").ok()?;
            let content: String = row.try_get("content").ok()?;
            let blob: Vec<u8> = row.try_get("embedding").ok()?;
            let vec = blob_to_vec(&blob);
            let sim = cosine_similarity(&query_vec, &vec);
            Some((file_path, content, sim))
        })
        .collect();

    scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(top_k);

    Ok(scored)
}
