# Backend Tasks (Rust & Tauri)

## Manual Setup
- [ ] Initialize Tauri workspace (`cargo tauri init`).
- [ ] Setup `rust-sqlite` / `sqlx` and load the `sqlite-vec` extension in the Tauri state.
- [ ] Setup `.env` for the Rust environment to store Cloud API Keys (e.g. `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`).

## Feature: Multi-Project Database Setup
- [ ] Setup `sqlx` and create Database migrations for:
  - `projects` (id, name, absolute_path)
  - `stories` (with `project_id` foreign key)
  - `settings` (to store `active_project_id`)
  - `embeddings` (with `project_id` foreign key)
- [ ] Create Tauri Commands (IPC endpoints): `get_projects`, `add_project`, and `switch_project`.

## Feature: The "Model Switcher"
- [ ] Implement Rust logic using `reqwest` to check if `OLLAMA_HOST` is reachable.
- [ ] If reachable, allow agents to fallback to local models.

## Feature: Live Streaming logs
- [ ] Setup Tauri `Window::emit` to push real-time task execution logs to the Web UI (Replaces Socket.io).