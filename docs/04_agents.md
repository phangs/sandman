# AI Agents Specification

## Agent Roles & Model Mapping
Users can assign different LLMs to different roles in `sandman.config.json`:

1. **Story Agent (Refiner)**
   - **Default:** Gemini 1.5 Pro (Large context for planning).
   - **Task:** Convert raw text to JSON (Title, Story, AC).

2. **Builder Agent (Executor)**
   - **Default:** Claude 3.5 Sonnet (State-of-the-art coding).
   - **Task:** Perform RAG search, plan file changes, and generate an execution script.

3. **Reviewer Agent (Gatekeeper)**
   - **Default:** GPT-4o or Ollama/Codestral.
   - **Task:** Compare code diff against AC. Issue PASS/FAIL.

## Self-Healing Protocol
The Builder Agent prompt includes a conditional section:
- "If `last_failure_logs` exists, analyze the error and prioritize fixing the regression in this attempt."