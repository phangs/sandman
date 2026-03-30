use sqlx::{sqlite::{SqliteConnectOptions, SqlitePoolOptions}, SqlitePool};
use std::path::PathBuf;
use std::fs;

pub async fn init_db(project_path: &str) -> Result<SqlitePool, String> {
    let mut db_dir = PathBuf::from(project_path);
    db_dir.push(".sandman");

    // Create the hidden .sandman directory if it doesn't exist
    if !db_dir.exists() {
        fs::create_dir_all(&db_dir)
            .map_err(|e| format!("Failed to create .sandman directory: {}", e))?;
    }

    let mut db_path = db_dir.clone();
    db_path.push("sandman.db");

    let options = SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .map_err(|e| format!("Failed to connect to SQLite: {}", e))?;

    // 1. Core tables (Standard SQL)
    let core_schema = r#"
        CREATE TABLE IF NOT EXISTS stories (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            status TEXT NOT NULL,
            ai_ready INTEGER NOT NULL DEFAULT 0,
            agent TEXT,
            state TEXT
        );

        CREATE TABLE IF NOT EXISTS files (
            path TEXT PRIMARY KEY,
            hash TEXT NOT NULL,
            last_idx_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS chunks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path TEXT NOT NULL,
            content TEXT NOT NULL,
            start_line INTEGER NOT NULL,
            end_line INTEGER NOT NULL,
            FOREIGN KEY(file_path) REFERENCES files(path) ON DELETE CASCADE
        );
    "#;

    sqlx::query(core_schema)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to initialize core schema: {}", e))?;

    // 2. Vector table (Requires sqlite-vec extension)
    // We attempt this separately so the app doesn't crash if the extension isn't loaded
    let vec_schema = "CREATE VIRTUAL TABLE IF NOT EXISTS vec_chunks USING vec0(chunk_id INTEGER PRIMARY KEY, embedding FLOAT[1536]);";
    let _ = sqlx::query(vec_schema).execute(&pool).await;

    Ok(pool)
}
