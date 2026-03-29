# Architecture

## System Design
[Web UI] <-> [Backend (Orchestrator + BullMQ)] <-> [Local Runner] <-> [Codebase]

## Components

### 1. Backend (The Brain)
- **Queue Manager:** BullMQ (Redis) handles durable task handoffs between agents.
- **LLM Factory:** A provider-agnostic interface supporting OpenAI, Anthropic, Gemini, and Ollama.
- **Context Provider:** RAG engine using `pgvector` to inject relevant code snippets into agent prompts.

### 2. PostgreSQL + pgvector (Unified Store)
- Stores relational data (Stories, Logs) and vector embeddings (Codebase chunks).
- Enables $0 cost semantic search for the Builder Agent.

### 3. Local Runner (The Hands)
- A CLI tool that polls the `code-execution` queue.
- Executes Git branching, file writes, and shell commands (`npm test`, etc.).
- Reports logs and exit codes back to the Orchestrator.

## Failure & Retry Logic
- **Build Failure:** Runner captures `stderr` -> Increments `retry_count` -> Re-queues to Builder with error context.
- **Review Failure:** Reviewer adds comments -> Re-queues to Builder.
- **Circuit Breaker:** If `retry_count >= 3`, status moves to `STALLED` for human review.