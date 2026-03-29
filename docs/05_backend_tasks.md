# Backend Tasks (Local Dev)

## Manual Setup
- [ ] Install PostgreSQL + pgvector locally.
- [ ] Install Redis locally (ensure default port 6379).
- [ ] Setup `.env` with local credentials (DB_USER, DB_PASS, REDIS_HOST=localhost).

## Feature: The "Model Switcher"
- [ ] Implement logic to check if `OLLAMA_HOST` is reachable.
- [ ] If reachable, allow agents to use local models; otherwise, fallback to Cloud APIs.

## Feature: Socket.io
- [ ] Setup Socket.io server to bridge the Local Runner's terminal output to the Web UI.