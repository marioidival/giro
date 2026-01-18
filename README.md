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

## API Documentation

### Health Check
- `GET /health` - Health check endpoint
  - Returns: `{"status": "ok", "version": "0.1.0", "database": "ok"}`

### Authentication
- `POST /api/auth/register` - Register new user
  - Body: `{"username": "user", "email": "user@example.com", "password": "pass"}`
  - Returns: `{"success": true, "message": "...", "user_id": "..."}`
- `POST /api/auth/login` - Login user
  - Body: `{"username": "user", "password": "pass"}`
  - Returns: `{"success": true, "message": "...", "user_id": "...", "session_token": "..."}`
- `POST /api/auth/logout` - Logout user

### Loops
- `GET /loops` - Loop list page (HTML)
- `GET /loops/new` - Loop creation form (HTML)
- `GET /loops/:id` - Loop detail page (HTML)
- `GET /api/loops` - List all loops for authenticated user
- `POST /api/loops` - Create a new loop
  - Body: Loop object with name, prd, provider, model, etc.
- `GET /api/loops/:id` - Get a specific loop
- `DELETE /api/loops/:id` - Delete a loop
- `POST /api/loops/:id/start` - Start a loop
- `POST /api/loops/:id/pause` - Pause a running loop
- `POST /api/loops/:id/resume` - Resume a paused loop
- `POST /api/loops/:id/stop` - Stop a loop
- `GET /api/loops/:id/stream` - WebSocket for real-time loop updates

### Tasks
- `GET /loops/:id/tasks/new` - Task creation form (HTML)
- `GET /api/loops/:id/tasks` - List tasks for a loop
- `POST /api/loops/:id/tasks` - Create a task for a loop
  - Body: `{"title": "Task name", "description": "...", "priority": 5}`
- `GET /api/tasks/:id` - Get a specific task
- `DELETE /api/tasks/:id` - Delete a task

## Frontend Usage

### Web Interface

The application provides a web interface built with HTMX and Tailwind CSS:

1. **Register/Login** - Access at `http://localhost:3000`
2. **Loop Dashboard** - View all your loops with status indicators
3. **Create Loop** - Click "New Loop" to create a new Ralph loop
4. **Loop Detail** - Click on a loop to view PRD, tasks, and controls
5. **Task Management** - Add, view, and delete tasks for each loop
6. **Real-time Updates** - WebSocket connections provide live progress updates

### WebSocket Integration

Connect to real-time loop updates:

```javascript
// Connect to WebSocket
const ws = new WebSocket(`ws://localhost:3000/api/loops/${loopId}/stream`);

ws.onmessage = (event) => {
    const message = JSON.parse(event.data);

    switch (message.type) {
        case "loop_status":
            // Update loop status indicator
            updateStatus(message.data.status);
            break;
        case "iteration_complete":
            // Update iteration counter
            updateIteration(message.data.iteration_number);
            break;
        case "loop_error":
            // Show error notification
            showError(message.data.error);
            break;
    }
};

ws.onclose = () => {
    console.log("WebSocket disconnected");
};
```

## Demo Scripts

### Create and Run a Simple Loop

```bash
#!/bin/bash
# demo-simple-loop.sh

# 1. Register user
curl -X POST http://localhost:3000/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "demouser",
    "email": "demo@example.com",
    "password": "password123"
  }'

# 2. Login and save session
RESPONSE=$(curl -s -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "demouser",
    "password": "password123"
  }')

SESSION_TOKEN=$(echo $RESPONSE | jq -r '.session_token')

# 3. Create a loop
LOOP_RESPONSE=$(curl -s -X POST http://localhost:3000/api/loops \
  -H "Content-Type: application/json" \
  -H "session: $SESSION_TOKEN" \
  -H "x-csrf-token: your-csrf-token" \
  -d '{
    "name": "Hello World Loop",
    "prd": "# Hello World\n\nBuild a simple HTML page that says hello world.",
    "provider": "mock",
    "model": "mock"
  }')

LOOP_ID=$(echo $LOOP_RESPONSE | jq -r '.loop_id')

# 4. Add a task
curl -X POST http://localhost:3000/api/loops/$LOOP_ID/tasks \
  -H "Content-Type: application/json" \
  -H "session: $SESSION_TOKEN" \
  -H "x-csrf-token: your-csrf-token" \
  -d '{
    "title": "Create HTML file",
    "description": "Create index.html with hello world message"
  }'

echo "Loop created with ID: $LOOP_ID"
echo "Open http://localhost:3000/loops/$LOOP_ID to view details"
```

### Monitor Loop Execution

```bash
#!/bin/bash
# demo-monitor-loop.sh

LOOP_ID=$1

# Monitor loop status
while true; do
  STATUS=$(curl -s -H "session: $SESSION_TOKEN" \
    http://localhost:3000/api/loops/$LOOP_ID | jq -r '.loop_.status')

  echo "Loop status: $STATUS"

  if [ "$STATUS" = "completed" ] || [ "$STATUS" = "error" ]; then
    echo "Loop finished!"
    break
  fi

  sleep 2
done
```

Run the demos:
```bash
chmod +x demo-simple-loop.sh demo-monitor-loop.sh
./demo-simple-loop.sh
```

## Database Migration: SQLite to PostgreSQL

### Current Setup

The application currently uses SQLite by default (`DATABASE_URL=sqlite:ralph.db`).

### Migration Path

To migrate to PostgreSQL:

1. **Update Database URL** in `.env`:
   ```bash
   # .env
   DATABASE_URL=postgresql://user:password@localhost:5432/ralph
   ```

2. **No Code Changes Required**
   - SQLx works with both SQLite and PostgreSQL
   - All queries use `query!()` macros which are type-safe
   - Database-specific syntax differences handled by SQLx

3. **Migrate Existing Data** (optional):
   ```bash
   # Export from SQLite
   sqlite3 ralph.db .dump > backup.sql
   
   # Import to PostgreSQL
   psql -U user -d ralph -f backup.sql
   ```

4. **Benefits of PostgreSQL**:
   - Better concurrency handling
   - Superior performance for read-heavy workloads
   - Full-text search extensions
   - Advanced indexing options
   - Production-ready replication and clustering

### Future Sprint 4 Enhancements

Planned features in future Sprint 4 include:
- PostgreSQL-specific optimizations
- Connection pooling configuration
- Advanced query performance monitoring
- Database migration tooling

## Documentation

- [PRD](PRD-ralph-loop-management.md) - Product Requirements Document
- [Sprint Breakdown](2026-01-17-ralph-loop-manager-sprint-breakdown.md) - Implementation tasks
- [GitHub Issues](GITHUB_ISSUES_README.md) - Issue tracking

## License

MIT
