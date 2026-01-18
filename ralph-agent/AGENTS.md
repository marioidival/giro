# Ralph Agent - Intent Layer

## Purpose

This crate provides the LLM (Large Language Model) orchestration layer for Ralph Loop Manager. It implements an abstract interface for multiple LLM providers (Claude, OpenAI, Sourcegraph Amp) and manages AI-assisted development task execution.

**What this area does:**
- Abstracts LLM calls for multiple providers (Claude, OpenAI, Amp)
- Manages AI task configuration and execution
- Parses structured responses (suggested tasks, commands)
- Executes commands in Docker containers via `ExecutionContext`
- Provides tool interface for file and command operations
- Manages context and iterations with automatic truncation
- Mock providers for testing

**What this area does NOT do:**
- Does not access database (that's `ralph-repositories`' responsibility)
- Does not manage Docker containers directly (uses `ExecutionContext`)
- Does not implement loop business logic (that's `ralph-services`' responsibility)
- Does not handle HTTP requests/responses (that's `ralph-server`' responsibility)

## General Architecture

### Module Structure

```
ralph-agent/
├── lib.rs          # Public re-exports
├── agent.rs        # CodeAgent, AgentConfig, AgentResult
├── provider.rs     # LLMProviderTrait, ClaudeProvider, OpenAIProvider
├── executor.rs     # ExecutionContext (container execution)
├── tools.rs        # Tool trait, FileTool, CommandTool
└── mocks.rs       # MockLLMProvider for tests
```

### Execution Flow

```
┌─────────────┐
│  ralph-     │
│  services   │ (Loop executor)
└──────┬──────┘
       │
       ▼
┌──────────────────────────────────────────────┐
│           CodeAgent                        │
│  - Manages configuration                  │
│  - Truncates context (MAX 100 entries)    │
│  - Sends requests to providers          │
│  - Applies timeout (default 60s)          │
└──────┬───────────────────────────────────┘
       │
       ├──────────────┬─────────────┐
       ▼              ▼             ▼
┌──────────┐   ┌──────────┐  ┌──────────┐
│  Claude  │   │  OpenAI  │  │   Amp    │
│ Provider │   │ Provider │  │ Provider │
│(anthropic│   │(async-   │  │(TODO)   │
│ _rust)   │   │ openai)  │  │          │
└──────────┘   └──────────┘  └──────────┘
       │              │
       └──────────────┴──────────────┐
                                     ▼
                     ┌────────────────────┐
                     │  LLMResponse     │
                     │ - content        │
                     │ - tokens_used   │
                     │ - suggested_tasks│
                     │ - commands      │
                     └────────────────────┘
                                     ▼
                     ┌────────────────────┐
                     │  ExecutionContext│
                     │  - Commands in   │
                     │    containers     │
                     │  - File I/O      │
                     └────────────────────┘
```

## Critical Invariants

### Provider Abstraction

**EVERY** provider must implement `LLMProviderTrait`:

```rust
#[async_trait]
pub trait LLMProviderTrait: Send + Sync {
    async fn complete(&self, request: &LLMRequest) -> anyhow::Result<LLMResponse>;
}
```

**Requirements:**
- Must be `Send + Sync` for async execution
- Receives `LLMRequest` (prd, task, context, max_tokens)
- Returns `LLMResponse` (content, tokens_used, suggested_tasks, commands)
- Propagates errors via `anyhow::Result`

### Context Truncation

**Context ALWAYS truncated to max 100 entries:**

```rust
pub const MAX_CONTEXT_ENTRIES: usize = 100;
```

- FIFO (First-In-First-Out) when exceeded
- Keeps 100 most recent entries
- Logs warning when truncation happens
- Order preserved after truncation

**Why this is critical:**
- Prevents context window overflow
- Keeps tokens under control
- Ensures consistent performance

### Timeout Enforcement

**ALWAYS** apply timeout to LLM requests:

```rust
let timeout_duration = tokio::time::Duration::from_secs(self.config.timeout_seconds);

let llm_response = tokio::time::timeout(timeout_duration, self.provider.complete(&request))
    .await
    .map_err(|_| AgentError::Timeout(self.config.timeout_seconds))??;
```

- Default: 60 seconds
- Configurable via `AgentConfig::timeout_seconds`
- Returns `AgentError::Timeout` if exceeded

### Response Parsing

**LLM MUST use specific tags for structured data:**

**Suggested tasks:**
```
<TASKS>[
  {"title": "Add tests", "description": "Write unit tests", "priority": 5}
]</TASKS>
```

**Commands:**
```
<CMD>cargo build</CMD>
<CMD>cargo test</CMD>
```

**Parsing:**
- `parse_suggested_tasks()` → Extracts JSON array between `<TASKS>...</TASKS>`
- `parse_commands()` → Extracts all `<CMD>...</CMD>` tags
- Returns `None` if tags not found
- Silent (doesn't fail if tags missing)

### Docker Execution Safety

**ALL** commands in containers MUST:

```rust
// 1. Timeout of 300 seconds (5 minutes)
timeout(Duration::from_secs(300), self.execute_in_container(&[command])).await

// 2. Check exit code != 0
if let Some(exit_code) = inspect.exit_code && exit_code != 0 {
    return Err(anyhow::anyhow!("Command failed with exit code {}", exit_code));
}

// 3. Decode output UTF-8 with error handling
String::from_utf8(stdout).map_err(|e| anyhow::anyhow!("Failed to decode output: {}", e))
```

### Serialization/Deserialization

**ALL** public structs MUST implement `Serialize`/`Deserialize`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig { ... }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest { ... }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse { ... }
```

**Why this is critical:**
- Configuration persistence
- Inter-service communication
- Debugging and logging

## Usage Patterns

### Create CodeAgent with Provider

```rust
use ralph_agent::{CodeAgent, AgentConfig, provider::ClaudeProvider};

// Create Claude provider
let claude = ClaudeProvider::new("sk-ant-...".to_string())?;

// Configure agent
let config = AgentConfig {
    max_iterations: 10,
    max_tokens_per_request: Some(4000),
    timeout_seconds: 60,
};

// Create agent (provider must be Box<dyn LLMProviderTrait>)
let agent = CodeAgent::new(Box::new(claude), config);
```

### Execute Task with Context

```rust
let prd = r#"
# Calculator App

Build a simple calculator with addition, subtraction, multiplication, and division.
"#.to_string();

let task = "Implement addition function".to_string();

let context = vec![
    "Iteration 1: Created project structure".to_string(),
    "Iteration 2: Added Cargo.toml with dependencies".to_string(),
];

let result = agent.execute_task(prd, task, context).await?;

println!("Content: {}", result.content);
println!("Tokens used: {}", result.tokens_used);

// Process suggested tasks
for suggested_task in result.suggested_tasks {
    println!("Suggested: {} (priority: {})", suggested_task.title, suggested_task.priority);
}

// Execute extracted commands
for command in result.commands_executed {
    println!("Command: {}", command);
}
```

### Use OpenAI Provider

```rust
use ralph_agent::provider::OpenAIProvider;

let openai = OpenAIProvider::new("sk-...".to_string())?;

let config = AgentConfig::default();
let agent = CodeAgent::new(Box::new(openai), config);

let result = agent.execute_task(
    "Build a CLI tool".to_string(),
    "Implement command parsing".to_string(),
    vec![],
).await?;
```

### Execute Commands in Container

```rust
use ralph_agent::ExecutionContext;
use std::collections::HashMap;

let ctx = ExecutionContext::new(
    "container-id-123".to_string(),
    "/workspace".to_string(),
    HashMap::new(), // no custom env vars
);

// Execute simple command
let output = ctx.execute_command("echo 'Hello, World!'").await?;
assert_eq!(output, "Hello, World!");

// Read file
let content = ctx.read_file("/workspace/config.toml").await?;

// Write file
ctx.write_file("/workspace/output.txt", "Generated content").await?;

// List files
let files = ctx.list_files(Some("/workspace/src")).await?;
```

### Use Tools for Operations

```rust
use ralph_agent::{Tool, FileTool, CommandTool};

// File tool
let file_tool = FileTool::new();

// Read file (placeholder - integrates with ExecutionContext in future)
let result = file_tool.execute(&["read", "/workspace/file.txt"]);
println!("{}", result.output);

// Write file
let result = file_tool.execute(&["write", "/workspace/file.txt", "Hello"]);
println!("{}", result.output);

// List files
let result = file_tool.execute(&["list", "/workspace"]);
println!("{}", result.output);

// Command tool
let cmd_tool = CommandTool::new();

// Execute command
let result = cmd_tool.execute(&["cargo", "build"]);
println!("{}", result.output);
```

### Test with Mock Provider

```rust
use ralph_agent::CodeAgent;
use ralph_agent::mocks::MockLLMProvider;

// Create mock provider
let mock = MockLLMProvider::new();

// Create agent with mock
let agent = CodeAgent::new(Box::new(mock), AgentConfig::default());

// Execute task (doesn't call real LLM)
let result = agent.execute_task(
    "Test PRD".to_string(),
    "Test task".to_string(),
    vec![],
).await?;

assert_eq!(result.content, "Mock LLM response - for testing purposes");
assert_eq!(result.tokens_used, 100);
```

## Anti-patterns

### NEVER DO

**1. Not truncate context**
```rust
// ❌ WRONG - Context can grow infinitely
pub async fn execute_task(&self, prd: String, task: String, context: Vec<String>) -> Result {
    let request = LLMRequest {
        context,  // uses all context without truncating!
        ...
    };
    ...
}

// ✅ CORRECT - Always truncate
let truncated_context = self.build_truncated_context(context);
let request = LLMRequest {
    context: truncated_context,
    ...
};
```

**2. Not apply timeout**
```rust
// ❌ WRONG - Request can block indefinitely
let llm_response = self.provider.complete(&request).await?;

// ✅ CORRECT - Apply timeout
let llm_response = tokio::time::timeout(
    Duration::from_secs(self.config.timeout_seconds),
    self.provider.complete(&request)
)
.await
.map_err(|_| AgentError::Timeout(self.config.timeout_seconds))??;
```

**3. Ignore command exit codes**
```rust
// ❌ WRONG - Doesn't check if command failed
self.execute_in_container(&[command]).await?;

// ✅ CORRECT - Check exit code
let inspect = self.docker.inspect_exec(&exec_id).await?;
if let Some(exit_code) = inspect.exit_code && exit_code != 0 {
    return Err(anyhow::anyhow!("Command failed with exit code {}", exit_code));
}
```

**4. Not serialize public structs**
```rust
// ❌ WRONG - Can't persist or communicate
pub struct AgentConfig {
    pub max_iterations: u32,
    ...
}

// ✅ CORRECT - Implement Serialize/Deserialize
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub max_iterations: u32,
    ...
}
```

**5. Use specific provider instead of trait**
```rust
// ❌ WRONG - Coupled to specific implementation
fn execute_with_claude(&self, claude: ClaudeProvider) -> Result { ... }

// ✅ CORRECT - Use abstract trait
fn execute_with_provider(&self, provider: &dyn LLMProviderTrait) -> Result { ... }
```

**6. Hardcode model IDs**
```rust
// ❌ WRONG - Hard to change model
let client = Client::builder()
    .model(Model::Claude35Sonnet20241022)  // hardcoded!
    .build()?;

// ✅ CORRECT - Pass as parameter or config
impl ClaudeProvider {
    pub fn new(api_key: String, model: Model) -> anyhow::Result<Self> {
        let client = Client::builder()
            .model(model)  // configurable!
            .build()?;
        ...
    }
}
```

**7. Parse JSON without error handling**
```rust
// ❌ WRONG - Panics on invalid JSON
let tasks: Vec<SuggestedTask> = serde_json::from_str(json_str).unwrap();

// ✅ CORRECT - Handle error silently
let tasks = serde_json::from_str::<Vec<SuggestedTask>>(json_str).ok();
```

**8. Assume tags exist**
```rust
// ❌ WRONG - Panics if tags don't exist
let start = content.find("<TASKS>").unwrap();
let end = content.find("</TASKS>").unwrap();

// ✅ CORRECT - Return None if tags missing
pub fn parse_suggested_tasks(content: &str) -> Option<Vec<SuggestedTask>> {
    let start_idx = content.find("<TASKS>")?;
    let end_idx = content.find("</TASKS>")?;
    ...
}
```

## Dependencies

### Dependencies (Cargo.toml)

```toml
[dependencies]
tokio = { workspace = true }           # Async runtime
serde = { workspace = true }           # Serialization
serde_json = { workspace = true }       # JSON parsing
async-trait = { workspace = true }      # Async traits
anyhow = { workspace = true }          # Error handling
thiserror = { workspace = true }        # Error types
uuid = { workspace = true }            # UUID generation
chrono = { workspace = true }           # DateTime
tracing = { workspace = true }          # Logging
futures = "0.3"                       # Stream utilities

# Docker
bollard = "0.18"                       # Docker API client

# LLM Providers
anthropic_rust = "0.1"                # Anthropic Claude
async-openai = "0.28"                   # OpenAI
```

### Internal Dependencies

- `ralph-models` → None (independent crate)
- `ralph-repositories` → None
- `ralph-agent` → `ralph-models` (shared types, if needed)

### Downstreams (who depends on this crate)

- `ralph-services` → Uses CodeAgent to orchestrate Ralph loops
- `ralph-server` → Uses via ralph-services (not direct)

## Main Components

### LLMProviderTrait

Trait that all providers implement:

```rust
#[async_trait]
pub trait LLMProviderTrait: Send + Sync {
    async fn complete(&self, request: &LLMRequest) -> anyhow::Result<LLMResponse>;
}
```

**Implemented providers:**
- `ClaudeProvider` → Anthropic Claude (via `anthropic_rust`)
- `OpenAIProvider` → OpenAI GPT (via `async-openai`)
- **TODO**: `AmpProvider` → Sourcegraph Amp (not implemented)

### CodeAgent

Main orchestrator:

```rust
pub struct CodeAgent {
    provider: Box<dyn LLMProviderTrait>,
    config: AgentConfig,
}

impl CodeAgent {
    pub fn new(provider: Box<dyn LLMProviderTrait>, config: AgentConfig) -> Self;
    pub async fn execute_task(&self, prd: String, task: String, context: Vec<String>)
        -> Result<AgentResult, AgentError>;
    fn build_truncated_context(&self, context: Vec<String>) -> Vec<String>;
}
```

**Responsibilities:**
- Manage LLM provider
- Automatically truncate context
- Apply timeout
- Build prompts
- Return structured results

### AgentConfig

Agent configuration:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub max_iterations: u32,                 // Default: 10
    pub max_tokens_per_request: Option<usize>, // Default: Some(4000)
    pub timeout_seconds: u64,                // Default: 60
}
```

### LLMRequest / LLMResponse

LLM communication structures:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    pub prd: String,              // Product Requirements Document
    pub task: String,             // Current task
    pub context: Vec<String>,      // Iteration history
    pub max_tokens: Option<usize>, // Token limit
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub content: String,               // Generated content
    pub tokens_used: u32,              // Tokens used
    pub suggested_tasks: Vec<SuggestedTask>, // Tasks from LLM
    pub commands: Vec<String>,         // Commands to execute
}
```

### ExecutionContext

Docker container command execution:

```rust
pub struct ExecutionContext {
    container_id: String,
    working_dir: String,
    env_vars: HashMap<String, String>,
    docker: Docker,
}

impl ExecutionContext {
    pub async fn execute_command(&self, command: &str) -> Result<String>;
    pub async fn read_file(&self, path: &str) -> Result<String>;
    pub async fn write_file(&self, path: &str, content: &str) -> Result<()>;
    pub async fn list_files(&self, path: Option<&str>) -> Result<Vec<String>>;
}
```

**Features:**
- 300s timeout per command
- Checks exit code
- Decodes output UTF-8
- Supports custom env vars
- Configurable working directory

### Tool Trait

Interface for tools used by agent:

```rust
pub trait Tool {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&self, args: &[&str]) -> ToolResult;
}
```

**Implemented tools:**
- `FileTool` → File operations (read, write, list)
- `CommandTool` → Shell command execution

**Note**: Tools currently return placeholders. Full integration with `ExecutionContext` pending.

## Tests

### Unit Tests

```bash
# Run all tests
cargo test --package ralph-agent

# Run tests for specific module
cargo test --package ralph-agent agent
cargo test --package ralph-agent provider
cargo test --package ralph-agent executor
cargo test --package ralph-agent tools
```

### Integration Tests

Tests requiring Docker are marked with `#[ignore]`:

```bash
# Run integration tests (requires Docker running)
cargo test --package ralph-agent -- --ignored
```

### Mocks

`MockLLMProvider` for testing without calling real APIs:

```rust
let mock = MockLLMProvider::new();
// Optional: add delay to test timeout
let mock = MockLLMProvider::new().with_delay(2000);
```

## Pitfalls

### Common Confusions

**1. Context Truncation is not an error**

```rust
// When context exceeds 100 entries, truncation is NORMAL
// A warning is logged, but it's not an error
warn!(
    "Context truncated from {} to {} entries (dropped {} oldest entries)",
    original_len, MAX_CONTEXT_ENTRIES, original_len - MAX_CONTEXT_ENTRIES
);

// This is a FEATURE, not a bug!
```

**2. Parsing tags are optional**

```rust
// LLM doesn't NEED to return <TASKS> or <CMD> tags
// parse_suggested_tasks() returns None if tags don't exist
// parse_commands() returns None if tags don't exist

// Both are Option<T> and can be None
// Don't assume they will always have values
```

**3. Providers can fail unexpectedly**

```rust
// LLM APIs can:
// - Rate limit (429)
// - Timeout
// - Return invalid JSON
// - Go offline

// Always use ? to propagate errors
let response = provider.complete(&request).await?;
// ^^^ Any error is propagated
```

**4. Docker containers must be running**

```rust
// ExecutionContext assumes:
// 1. Docker daemon is running
// 2. Container exists and is running
// 3. Container has /bin/sh

// If not, execute_in_container() will fail
```

**5. Tools are not async (yet)**

```rust
// Tool trait is not async
fn execute(&self, args: &[&str]) -> ToolResult;

// Integration with ExecutionContext (which is async)
// is planned but not implemented

// For now, tools return placeholders
```

### Unexpected Behaviors

**1. Timeout is applied by agent, not provider**

```rust
// Agent applies timeout via tokio::time::timeout
tokio::time::timeout(Duration::from_secs(config.timeout_seconds), ...)

// Provider doesn't know about timeout
// May continue processing after timeout
```

**2. Tokens used may vary between providers**

```rust
// Each provider counts tokens differently:
// - Claude: input_tokens + output_tokens
// - OpenAI: total_tokens

// Values are not directly comparable between providers
```

**3. SuggestedTasks have priority field**

```rust
pub struct SuggestedTask {
    pub title: String,
    pub description: String,
    pub priority: i32,  // Higher = more priority
}

// Priority is not used internally by ralph-agent
// It's ralph-services' responsibility to prioritize tasks
```

**4. Extracted commands are not executed automatically**

```rust
// LLMResponse includes commands:
pub struct LLMResponse {
    pub commands: Vec<String>,
    ...
}

// But ralph-agent does NOT execute them
// It's ralph-services' responsibility to execute via ExecutionContext
```

**5. Context entries are simple strings**

```rust
// Context is Vec<String>, not rich struct
pub context: Vec<String>

// Each entry is a string from previous iteration
// No metadata or timestamps
```

## Configuration

### Environment Variables

**Anthropic Claude:**
```bash
ANTHROPIC_API_KEY=sk-ant-api03-...
```

**OpenAI:**
```bash
OPENAI_API_KEY=sk-...
```

### Default Values

```rust
AgentConfig {
    max_iterations: 10,
    max_tokens_per_request: Some(4000),
    timeout_seconds: 60,
}

MAX_CONTEXT_ENTRIES: 100

ExecutionContext command timeout: 300 seconds (5 minutes)
```

## Downlinks (Additional Context)

**For better understanding:**
- `/AGENTS.md` - General project architecture
- `/ralph-models/AGENTS.md` - Shared data models
- `/ralph-services/AGENTS.md` - How CodeAgent is used in loop orchestration
- `/ralph-repositories/AGENTS.md` - Iteration and task persistence

## Roadmap

### Pending Items

1. **AmpProvider** → Sourcegraph Amp not implemented
2. **Tools async integration** → Integrate Tool trait with ExecutionContext
3. **Streaming responses** → Support LLM response streaming
4. **Tool calling** → Implement native function calling (not tags)
5. **Context window management** → Tokens-aware truncation (based on tokens, not entries)
6. **Circuit breaker** → Protection against API rate limits

---

**Last updated:** 2026-01-18
**Version:** 1.0
