# Local Runner Tasks

## The Execution Protocol
- [ ] **Local Indexer:** Scan project files, generate embeddings via Ollama, and sync to Backend pgvector.
- [ ] **Job Consumer:** Poll BullMQ for `code-execution` tasks.
- [ ] **Git Manager:** Auto-create branches (`feat/sandman-story-id`) and handle reverts on fatal errors.
- [ ] **Sandbox:** Execute commands defined in `sandman.config.json` whitelisted scripts.

## Reporting
- [ ] Capture `stdout`/`stderr` and stream to Backend.
- [ ] Package failure logs into the "Job Result" for the next agent in the loop.