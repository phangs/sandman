# Local Execution Tasks (Rust)

## The Execution Protocol
- [ ] **Local Indexer:** A `tokio` thread to scan project files, generate embeddings via the configured LLM API (Cloud or Ollama), and sync to Embedded SQLite via `sqlx`.
- [ ] **Job Queue Manager:** Build a lightweight async queue worker loop in Rust.
- [ ] **Git Manager:** Use `std::process::Command` (or `git2-rs`) to auto-create branches (`feat/sandman-story-id`).
- [ ] **Sandbox:** Execute commands defined in `sandman.config.json` whitelisted scripts.

## Reporting
- [ ] Pipe `stdout`/`stderr` from standard shell commands directly to Tauri's Event Emitter.
- [ ] Package failure logs into the SQLite Job Result for the next agent in the loop.