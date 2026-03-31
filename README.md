# Sandman IDE

Sandman is a groundbreaking, open-source IDE built for the AI-first development era. It integrates autonomous agentic workflows seamlessly into your software development lifecycle (SDLC) using a multi-provider LLM infrastructure.

Instead of writing code yourself, you define requirements in a Kanban board. Sandman's AI agents autonomously brainstorm, plan, implement, trace, and document your stories directly within your local repository.

## ✨ Core Features

- **Autonomous Agentic Loop**: Assign stories to an AI agent that loops through thinking, planning, and executing filesystem changes autonomously.
- **Rule-Based SDLC Routing**: Configure specific AI models for specific Kanban stages. E.g., Use **Ollama** (Local) for fast requirement gathering, **Anthropic (Claude-3.5-Sonnet)** for precision coding, **XAI (Grok)** for rigorous code review, and **Google Gemini** for high-throughput documentation.
- **PTY Terminal Integration**: A persistent background terminal reader piped natively to the frontend ensures agents can run real build commands and react to compiler outputs.
- **RAG Semantic Search**: Agents have programmatic access to your entire codebase and existing `docs/` artifacts, preventing hallucinated implementations.
- **Monaco Editor & Explorer**: Fallback to manual intervention at any time through a fully-featured code editor and file explorer.
- **100% Local Privacy**: Sandman runs natively on your machine (Tauri/Rust). API keys and conversation histories are stored in an encrypted local SQLite database (`.sandman`).

---

## 🚀 Architecture

- **Frontend**: React 18, TypeScript, Vite, Tailwind CSS, Lucide React, Monaco Editor.
- **Backend**: Rust, Tauri v2.
- **Database**: SQLite (via `sqlx`) stored directly inside your project folder (`.sandman/sandman.db`).
- **Terminal Integration**: `portable-pty` for persistent terminal sessions piped via IPC.
- **AI Infrastructure**: Custom `llm.rs` dispatcher supporting local (Ollama) and cloud APIs (OpenAI, Anthropic, Gemini, XAI).

---

## 🛠️ Development Guide

### Prerequisites

You need the standard [Tauri Prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites) installed on your machine:
- Node.js (v18+)
- Rust (`rustup`, `cargo`)
- OS-specific build tools (`build-essential`, `libgtk-3-dev`, `libwebkit2gtk-4.0-dev` for Linux).

### Local Setup

1. **Clone the repository:**
   ```bash
   git clone https://github.com/your-org/sandman.git
   cd sandman
   ```

2. **Install Node dependencies:**
   ```bash
   npm install
   ```

3. **Start the Development Server:**
   This command starts the Vite dev server and compiles the Rust backend concurrently.
   ```bash
   npm run tauri dev
   ```

### Project Structure Overview

#### Frontend (`src/`)
- `components/KanbanBoard.tsx`: The heart of the SDLC where stories are managed and agents are dispatched.
- `components/SettingsView.tsx`: Multi-LLM API key configuration and SDLC column routing strategy.
- `components/TerminalPanel.tsx`: The PTY terminal interface listening to IPC events from Rust.
- `components/CodeEditor.tsx`: Monaco editor integration for manual interventions.
- `App.tsx`: The primary resizable IDE layout.

#### Backend (`src-tauri/src/`)
- `lib.rs`: The Core Tauri builder, IPC command registration, and global state (database pool, PTY session, active project path).
- `agent.rs`: The autonomous AI orchestration loop. Handles sequential/parallel tool execution, tool output parsing, hallucination guards, and conversation management.
- `llm.rs`: The multi-provider LLM dispatcher handling payload transformations for Ollama, OpenAI, Gemini, Anthropic, and XAI.
- `tools.rs`: The filesystem tool-belt consumed by the agent (grep search, recursive file listing, bash execution).
- `db.rs`: SQLite initialization and `sqlx` schemas.
- `config.rs`: Local workspace configuration loading and default provider aggregation.
- `prompts.rs`: The foundational system prompts driving the agentic persona.

### Architectural Philosophies

When contributing to Sandman, adhere to these design principles:
1. **Zero-Stall Agentic Loop**: Agents must always receive actionable error feedback rather than silently failing. E.g., if a file does not exist, return an error string to the prompt history so the LLM can correct itself.
2. **Parallel Tool Execution**: LLM responses containing multiple XML `<tool>` tags must be parsed holistically within a single turn to minimize latency.
3. **No Central Telemetry**: API keys and prompt history must remain on the user's local disk. Do not introduce remote telemetry loggers.

---

## 🤝 Contributing

We welcome pull requests for new LLM integrations, enhanced agentic tools (e.g., browser-automation tools), and frontend polish. Please run `cargo clippy --fix` in `src-tauri` and `npm run lint` before committing.
