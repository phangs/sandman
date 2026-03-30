# Product Requirements Document (PRD)

## Objective
Enable a Product Owner to manage an autonomous software development lifecycle where AI agents handle requirements refinement, coding, and quality assurance.

## Features
### 1. Workspace & Project Management
- **Multi-Project Support:** A single Sandman instance can manage multiple local codebases.
- **Seamless Switching:** Users can switch between projects/folders without losing history, stories, or embeddings.

### 2. Kanban Board
- Columns: Backlog, To Do, In Progress, Review, Testing, Done.
- Real-time updates via Tauri IPC Events.

### 2. Story Management
- **Ready for AI Toggle:** Explicit human-in-the-loop trigger to start automation.
- **Retry Logic:** Automatic re-queuing on failure with a "Circuit Breaker" at 3 attempts.
- **Log Streaming:** Real-time visibility into agent "thoughts" and terminal output.

### 3. Agent System
- **Story Agent:** Refines raw input into structured User Stories and Acceptance Criteria (AC).
- **Builder Agent:** Context-aware code generation using RAG (`sqlite-vec`).
- **Reviewer Agent:** Validates code against AC and build success.

## Success Metrics
- **Build Success Rate:** Percentage of stories that reach "Done" without human intervention.
- **Self-Healing Efficiency:** Rate at which agents fix their own build errors on retry.