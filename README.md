# Ralph Loop Manager

A web-based platform for managing, monitoring, and orchestrating Ralph loops - AI-driven development workflows.

## About

Ralph is a technique for AI-driven development based on an infinite loop:

```bash
while :; do cat PROMPT.md | npx --yes @sourcegraph/amp ; done
```

Ralph Loop Manager provides a complete web interface for:

- Creating and managing Ralph loops with PRDs and tasks
- Executing loops in isolated Docker containers
- Real-time monitoring and streaming output
- Full history and iteration tracking
- Integration with Git (auto PRs/MRs)
- Support for multiple LLM providers (Claude, OpenAI, Sourcegraph Amp)

## Architecture

**Stack:**
- **Backend:** Rust (Axum, Tokio, SQLx)
- **Database:** SQLite (with PostgreSQL migration path)
- **Frontend:** HTMX + Tailwind CSS
- **Containers:** Docker

**Workspace Structure:**
- `ralph-models` - Data models (User, Loop, Task, etc)
- `ralph-repositories` - Database repositories
- `ralph-agent` - LLM agent with provider adapters
- `ralph-services` - Business logic (auth, Docker, loop execution)
- `ralph-server` - HTTP server and web interface

## Prerequisites

- **Rust 1.85+** - For building the project
- **Docker 27.x+** - For containerized loop execution
- **API Keys** - At least one of:
  - Anthropic Claude API key (from https://console.anthropic.com/)
  - OpenAI API key (from https://platform.openai.com/)

## Development Setup

1. Clone the repository
2. Copy `.env.example` to `.env` and add your API keys
3. Run `cargo check` to verify workspace compiles
4. Run `cargo build` to build all crates

## Running

```bash
# Start the server
cargo run --bin ralph-server

# Server will be available at http://localhost:3000
```

## Documentation

- [PRD](PRD-ralph-loop-management.md) - Product Requirements Document
- [Sprint Breakdown](2026-01-17-ralph-loop-manager-sprint-breakdown.md) - Implementation tasks
- [GitHub Issues](GITHUB_ISSUES_README.md) - Issue tracking

## License

MIT
