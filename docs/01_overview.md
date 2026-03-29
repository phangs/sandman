# Sandman Overview

## Vision
Sandman is an AI-powered software factory that transforms product requirements into working code through an autonomous, state-driven Kanban workflow.

## Core Idea
- **User-Defined Requirements:** Input raw ideas into a Kanban backlog.
- **Multi-Agent Orchestration:** Specialized AI agents (Story, Builder, Reviewer) handle the SDLC stages.
- **Local Execution:** A dedicated runner applies code and executes tests on your machine.
- **Autonomous Feedback:** Agents self-heal by analyzing build/test failures and retrying.

## Key Differentiators
- **State-Driven Workflow:** Moves beyond simple chat-based coding into an automated pipeline.
- **Model Agnostic & Local-First:** Supports Claude, Gemini, and GPT-4o, with **Ollama** as a first-class option for any agent to ensure zero-cost, private execution.
- **Zero-Cost Infrastructure:** Built on PostgreSQL (pgvector), Redis (BullMQ), and Node.js.

## Tagline
"Build software while you sleep."