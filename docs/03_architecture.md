# Architecture

## System Design
[React Web UI (Vite)] <-> [Tauri Backend (Rust)] <-> [Embedded SQLite] / [Local Codebase]

## Components

### 1. Tauri Backend (The Brain & Muscle)
- **Rust Engine:** Consolidates the UI host, the LLM orchestrator, and the command runner into a single high-performance binary.
- **Queue Manager:** Lightweight async queuing using `tokio` channels or a SQLite-backed job table (no Redis/Postgres needed).
- **LLM Factory:** Native Rust HTTP clients (`reqwest`) supporting seamless switching between Cloud APIs (OpenAI, Anthropic, Gemini) and Local APIs (Ollama/LM Studio).
- **Context Provider:** RAG engine using `sqlite-vec` (`rusqlite`/`sqlx`) to inject relevant code snippets into agent prompts.

### 2. Embedded SQLite + sqlite-vec (Unified Store)
- **Multi-Project Aware:** The `projects` table stores the absolute paths to different local codebases.
- Every `story`, codebase vector `embedding`, and `log` is isolated by a `project_id`.
- Stores relational data (Stories, Logs) and vector embeddings (Codebase chunks).
- Enables $0 cost semantic search for the Builder Agent.

### 3. Local Execution Engine (Rust Child Processes)
- A dedicated async Rust supervisor embedded inside Tauri.
- Uses `std::process::Command` to execute Git branching, file writes, and shell commands (`npm test`, etc.) securely without memory leaks.
- Streams highly-efficient live `stdout`/`stderr` logs back to the React UI via Tauri `Window::emit()`.

## Failure & Retry Logic
- **Build Failure:** Rust captures `stderr` -> Increments `retry_count` -> Re-queues the task internally with error context.
- **Review Failure:** Reviewer adds comments -> Re-queues to Builder.
- **Circuit Breaker:** If `retry_count >= 3`, status moves to `STALLED` for human review.