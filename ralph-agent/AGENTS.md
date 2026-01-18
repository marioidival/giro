# Ralph Agent - Intent Layer

## Propósito

Este crate fornece a camada de orquestração de LLM (Large Language Model) para o Ralph Loop Manager. Implementa uma interface abstrata para múltiplos providers de LLM (Claude, OpenAI, Sourcegraph Amp) e gerencia a execução de tarefas de desenvolvimento assistido por IA.

**O que esta área faz:**
- Abstrai chamadas de LLM para múltiplos providers (Claude, OpenAI, Amp)
- Gerencia configuração e execução de tarefas de IA
- Faz parsing de respostas estruturadas (tasks sugeridas, comandos)
- Executa comandos em containers Docker via `ExecutionContext`
- Fornece interface de tools para operações de arquivo e comando
- Gerencia contexto e iterações com truncamento automático
- Mock providers para testes

**O que esta área NÃO faz:**
- Não acessa banco de dados (isso é responsabilidade de `ralph-repositories`)
- Não gerencia containers Docker diretamente (usa `ExecutionContext`)
- Não implementa lógica de negócio de loops (isso é responsabilidade de `ralph-services`)
- Não lida com HTTP requests/responses (isso é responsabilidade de `ralph-server`)

## Arquitetura Geral

### Estrutura de Módulos

```
ralph-agent/
├── lib.rs          # Re-exports públicos
├── agent.rs        # CodeAgent, AgentConfig, AgentResult
├── provider.rs     # LLMProviderTrait, ClaudeProvider, OpenAIProvider
├── executor.rs     # ExecutionContext (execução em containers)
├── tools.rs        # Tool trait, FileTool, CommandTool
└── mocks.rs       # MockLLMProvider para testes
```

### Fluxo de Execução

```
┌─────────────┐
│  ralph-     │
│  services   │ (Loop executor)
└──────┬──────┘
       │
       ▼
┌──────────────────────────────────────────────┐
│           CodeAgent                        │
│  - Gerencia configuração                  │
│  - Trunca contexto (MAX 100 entradas)    │
│  - Envia requests para providers          │
│  - Aplica timeout (default 60s)          │
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
                     │  - Comandos em   │
                     │    containers     │
                     │  - File I/O      │
                     └────────────────────┘
```

## Invariantes Críticos

### Provider Abstraction

**TODO** provider deve implementar `LLMProviderTrait`:

```rust
#[async_trait]
pub trait LLMProviderTrait: Send + Sync {
    async fn complete(&self, request: &LLMRequest) -> anyhow::Result<LLMResponse>;
}
```

**Requisitos:**
- Deve ser `Send + Sync` para execução async
- Recebe `LLMRequest` (prd, task, context, max_tokens)
- Retorna `LLMResponse` (content, tokens_used, suggested_tasks, commands)
- Propaga erros via `anyhow::Result`

### Context Truncation

**Contexto SEMPRE truncado para no máximo 100 entradas:**

```rust
pub const MAX_CONTEXT_ENTRIES: usize = 100;
```

- FIFO (First-In-First-Out) quando excede
- Mantém as 100 entradas mais recentes
- Log warning quando truncamento acontece
- Ordem preservada após truncamento

**Por que é crítico:**
- Previne context window overflow
- Mantém tokens sob controle
- Garante performance consistente

### Timeout Enforcement

**SEMPRE** aplicar timeout a requests LLM:

```rust
let timeout_duration = tokio::time::Duration::from_secs(self.config.timeout_seconds);

let llm_response = tokio::time::timeout(timeout_duration, self.provider.complete(&request))
    .await
    .map_err(|_| AgentError::Timeout(self.config.timeout_seconds))??;
```

- Default: 60 segundos
- Configurável via `AgentConfig::timeout_seconds`
- Retorna `AgentError::Timeout` se exceder

### Response Parsing

**LLM DEVE usar tags específicas para dados estruturados:**

**Tasks sugeridas:**
```
<TASKS>[
  {"title": "Add tests", "description": "Write unit tests", "priority": 5}
]</TASKS>
```

**Comandos:**
```
<CMD>cargo build</CMD>
<CMD>cargo test</CMD>
```

**Parsing:**
- `parse_suggested_tasks()` → Extrai JSON array entre `<TASKS>...</TASKS>`
- `parse_commands()` → Extrai todos `<CMD>...</CMD>` tags
- Retorna `None` se tags não encontradas
- Silencioso (não falha se faltam tags)

### Docker Execution Safety

**TODOS** os comandos em containers DEVEM:

```rust
// 1. Timeout de 300 segundos (5 minutos)
timeout(Duration::from_secs(300), self.execute_in_container(&[command])).await

// 2. Checar exit code != 0
if let Some(exit_code) = inspect.exit_code && exit_code != 0 {
    return Err(anyhow::anyhow!("Command failed with exit code {}", exit_code));
}

// 3. Decodificar output UTF-8 com tratamento de erro
String::from_utf8(stdout).map_err(|e| anyhow::anyhow!("Failed to decode output: {}", e))
```

### Serialization/Deserialization

**TODOS** os structs públicas DEVEM implementar `Serialize`/`Deserialize`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig { ... }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest { ... }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse { ... }
```

**Por que é crítico:**
- Persistência de configuração
- Comunicação entre serviços
- Debugging e logging

## Padrões de Uso

### Criar CodeAgent com Provider

```rust
use ralph_agent::{CodeAgent, AgentConfig, provider::ClaudeProvider};

// Criar provider Claude
let claude = ClaudeProvider::new("sk-ant-...".to_string())?;

// Configurar agent
let config = AgentConfig {
    max_iterations: 10,
    max_tokens_per_request: Some(4000),
    timeout_seconds: 60,
};

// Criar agent (provider deve ser Box<dyn LLMProviderTrait>)
let agent = CodeAgent::new(Box::new(claude), config);
```

### Executar Tarefa com Contexto

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

// Processar tasks sugeridas
for suggested_task in result.suggested_tasks {
    println!("Suggested: {} (priority: {})", suggested_task.title, suggested_task.priority);
}

// Executar comandos extraídos
for command in result.commands_executed {
    println!("Command: {}", command);
}
```

### Usar OpenAI Provider

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

### Executar Comandos em Container

```rust
use ralph_agent::ExecutionContext;
use std::collections::HashMap;

let ctx = ExecutionContext::new(
    "container-id-123".to_string(),
    "/workspace".to_string(),
    HashMap::new(), // sem env vars customizadas
);

// Executar comando simples
let output = ctx.execute_command("echo 'Hello, World!'").await?;
assert_eq!(output, "Hello, World!");

// Ler arquivo
let content = ctx.read_file("/workspace/config.toml").await?;

// Escrever arquivo
ctx.write_file("/workspace/output.txt", "Generated content").await?;

// Listar arquivos
let files = ctx.list_files(Some("/workspace/src")).await?;
```

### Usar Tools para Operações

```rust
use ralph_agent::{Tool, FileTool, CommandTool};

// File tool
let file_tool = FileTool::new();

// Ler arquivo (placeholder - integra com ExecutionContext no futuro)
let result = file_tool.execute(&["read", "/workspace/file.txt"]);
println!("{}", result.output);

// Escrever arquivo
let result = file_tool.execute(&["write", "/workspace/file.txt", "Hello"]);
println!("{}", result.output);

// Listar arquivos
let result = file_tool.execute(&["list", "/workspace"]);
println!("{}", result.output);

// Command tool
let cmd_tool = CommandTool::new();

// Executar comando
let result = cmd_tool.execute(&["cargo", "build"]);
println!("{}", result.output);
```

### Testar com Mock Provider

```rust
use ralph_agent::CodeAgent;
use ralph_agent::mocks::MockLLMProvider;

// Criar mock provider
let mock = MockLLMProvider::new();

// Criar agent com mock
let agent = CodeAgent::new(Box::new(mock), AgentConfig::default());

// Executar tarefa (não chama LLM real)
let result = agent.execute_task(
    "Test PRD".to_string(),
    "Test task".to_string(),
    vec![],
).await?;

assert_eq!(result.content, "Mock LLM response - for testing purposes");
assert_eq!(result.tokens_used, 100);
```

## Anti-padrões

### NUNCA FAZER

**1. Não truncar contexto**
```rust
// ❌ ERRADO - Contexto pode crescer infinitamente
pub async fn execute_task(&self, prd: String, task: String, context: Vec<String>) -> Result {
    let request = LLMRequest {
        context,  // usa todo o contexto sem truncar!
        ...
    };
    ...
}

// ✅ CERTO - Sempre truncar
let truncated_context = self.build_truncated_context(context);
let request = LLMRequest {
    context: truncated_context,
    ...
};
```

**2. Não aplicar timeout**
```rust
// ❌ ERRADO - Request pode bloquear indefinidamente
let llm_response = self.provider.complete(&request).await?;

// ✅ CERTO - Aplicar timeout
let llm_response = tokio::time::timeout(
    Duration::from_secs(self.config.timeout_seconds),
    self.provider.complete(&request)
)
.await
.map_err(|_| AgentError::Timeout(self.config.timeout_seconds))??;
```

**3. Ignorar exit codes de comandos**
```rust
// ❌ ERRADO - Não verifica se comando falhou
self.execute_in_container(&[command]).await?;

// ✅ CERTO - Checar exit code
let inspect = self.docker.inspect_exec(&exec_id).await?;
if let Some(exit_code) = inspect.exit_code && exit_code != 0 {
    return Err(anyhow::anyhow!("Command failed with exit code {}", exit_code));
}
```

**4. Não serializar structs públicas**
```rust
// ❌ ERRADO - Não pode persistir ou comunicar
pub struct AgentConfig {
    pub max_iterations: u32,
    ...
}

// ✅ CERTO - Implementar Serialize/Deserialize
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub max_iterations: u32,
    ...
}
```

**5. Usar provider específico em vez de trait**
```rust
// ❌ ERRADO - Acoplamento à implementação específica
fn execute_with_claude(&self, claude: ClaudeProvider) -> Result { ... }

// ✅ CERTO - Usar trait abstrato
fn execute_with_provider(&self, provider: &dyn LLMProviderTrait) -> Result { ... }
```

**6. Hardcode IDs de modelos**
```rust
// ❌ ERRADO - Dificil trocar modelo
let client = Client::builder()
    .model(Model::Claude35Sonnet20241022)  // hardcoded!
    .build()?;

// ✅ CERTO - Passar como parâmetro ou config
impl ClaudeProvider {
    pub fn new(api_key: String, model: Model) -> anyhow::Result<Self> {
        let client = Client::builder()
            .model(model)  // configurável!
            .build()?;
        ...
    }
}
```

**7. Parsear JSON sem tratamento de erro**
```rust
// ❌ ERRADO - panic em JSON inválido
let tasks: Vec<SuggestedTask> = serde_json::from_str(json_str).unwrap();

// ✅ CERTO - Tratar erro silenciosamente
let tasks = serde_json::from_str::<Vec<SuggestedTask>>(json_str).ok();
```

**8. Assumir que tags existem**
```rust
// ❌ ERRADO - panic se tags não existem
let start = content.find("<TASKS>").unwrap();
let end = content.find("</TASKS>").unwrap();

// ✅ CERTO - Retornar None se tags faltam
pub fn parse_suggested_tasks(content: &str) -> Option<Vec<SuggestedTask>> {
    let start_idx = content.find("<TASKS>")?;
    let end_idx = content.find("</TASKS>")?;
    ...
}
```

## Dependências

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

- `ralph-models` → Nenhuma (crate independente)
- `ralph-repositories` → Nenhuma
- `ralph-agent` → `ralph-models` (tipos compartilhados, se necessário)

### Downstreams (quem depende deste crate)

- `ralph-services` → Usa CodeAgent para orquestrar loops Ralph
- `ralph-server` → Usa via ralph-services (não direto)

## Componentes Principais

### LLMProviderTrait

Trait que todos os providers implementam:

```rust
#[async_trait]
pub trait LLMProviderTrait: Send + Sync {
    async fn complete(&self, request: &LLMRequest) -> anyhow::Result<LLMResponse>;
}
```

**Providers implementados:**
- `ClaudeProvider` → Anthropic Claude (via `anthropic_rust`)
- `OpenAIProvider` → OpenAI GPT (via `async-openai`)
- **TODO**: `AmpProvider` → Sourcegraph Amp (não implementado)

### CodeAgent

Orchestrador principal:

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

**Responsabilidades:**
- Gerenciar provider LLM
- Truncar contexto automaticamente
- Aplicar timeout
- Construir prompts
- Retornar resultados estruturados

### AgentConfig

Configuração do agent:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub max_iterations: u32,                 // Default: 10
    pub max_tokens_per_request: Option<usize>, // Default: Some(4000)
    pub timeout_seconds: u64,                // Default: 60
}
```

### LLMRequest / LLMResponse

Estruturas de comunicação com LLM:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    pub prd: String,              // Product Requirements Document
    pub task: String,             // Task atual
    pub context: Vec<String>,      // Histórico de iterações
    pub max_tokens: Option<usize>, // Limite de tokens
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub content: String,               // Conteúdo gerado
    pub tokens_used: u32,              // Tokens usados
    pub suggested_tasks: Vec<SuggestedTask>, // Tasks do LLM
    pub commands: Vec<String>,         // Comandos para executar
}
```

### ExecutionContext

Execução de comandos em containers Docker:

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

**Características:**
- Timeout de 300s por comando
- Checa exit code
- Decodifica output UTF-8
- Suporta env vars customizadas
- Working directory configurável

### Tool Trait

Interface para tools usados pelo agent:

```rust
pub trait Tool {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&self, args: &[&str]) -> ToolResult;
}
```

**Tools implementados:**
- `FileTool` → Operações de arquivo (read, write, list)
- `CommandTool` → Execução de comandos shell

**Nota**: Tools atualmente retornam placeholders. Integração completa com `ExecutionContext` pendente.

## Testes

### Unit Tests

```bash
# Rodar todos os testes
cargo test --package ralph-agent

# Rodar testes de um módulo específico
cargo test --package ralph-agent agent
cargo test --package ralph-agent provider
cargo test --package ralph-agent executor
cargo test --package ralph-agent tools
```

### Integration Tests

Tests que requerem Docker são marcados com `#[ignore]`:

```bash
# Rodar integration tests (requer Docker rodando)
cargo test --package ralph-agent -- --ignored
```

### Mocks

`MockLLMProvider` para testes sem chamar APIs reais:

```rust
let mock = MockLLMProvider::new();
// Opcional: adicionar delay para testar timeout
let mock = MockLLMProvider::new().with_delay(2000);
```

## Armadilhas

### Confusões Comuns

**1. Context Truncation não é um erro**

```rust
// Quando contexto excede 100 entradas, truncamento é NORMAL
// Um warning é logado, mas não é um erro
warn!(
    "Context truncated from {} to {} entries (dropped {} oldest entries)",
    original_len, MAX_CONTEXT_ENTRIES, original_len - MAX_CONTEXT_ENTRIES
);

// Isso é uma FEATURE, não um bug!
```

**2. Tags de parsing são opcionais**

```rust
// LLM não PRECISA retornar <TASKS> ou <CMD> tags
// parse_suggested_tasks() retorna None se tags não existem
// parse_commands() retorna None se tags não existem

// Ambos são Option<T> e podem ser None
// Não assuma que sempre terão valores
```

**3. Providers podem falhar inesperadamente**

```rust
// APIs de LLM podem:
// - Rate limit (429)
// - Timeout
// - Retornar JSON inválido
// - Ficar offline

// Sempre use ? para propagar erros
let response = provider.complete(&request).await?;
// ^^^ Qualquer erro é propagado
```

**4. Docker containers precisam estar rodando**

```rust
// ExecutionContext assume que:
// 1. Docker daemon está rodando
// 2. Container existe e está rodando
// 3. Container tem /bin/sh

// Se não, execute_in_container() vai falhar
```

**5. Tools não são async (ainda)**

```rust
// Tool trait não é async
fn execute(&self, args: &[&str]) -> ToolResult;

// Integração com ExecutionContext (que é async)
// está planejada mas não implementada

// Por enquanto, tools retornam placeholders
```

### Comportamentos Inesperados

**1. Timeout é aplicado pelo agent, não pelo provider**

```rust
// O agent aplica timeout via tokio::time::timeout
tokio::time::timeout(Duration::from_secs(config.timeout_seconds), ...)

// O provider não sabe sobre timeout
// Pode continuar processando após timeout
```

**2. Tokens usados podem variar entre providers**

```rust
// Cada provider conta tokens de forma diferente:
// - Claude: input_tokens + output_tokens
// - OpenAI: total_tokens

// Valores não são diretamente comparáveis entre providers
```

**3. SuggestedTasks têm priority field**

```rust
pub struct SuggestedTask {
    pub title: String,
    pub description: String,
    pub priority: i32,  // Maior = mais prioritária
}

// Priority não é usado internamente por ralph-agent
// É responsabilidade de ralph-services priorizar tasks
```

**4. Commands extraídos não são executados automaticamente**

```rust
// LLMResponse inclui commands:
pub struct LLMResponse {
    pub commands: Vec<String>,
    ...
}

// Mas ralph-agent NÃO os executa
// É responsabilidade de ralph-services executar via ExecutionContext
```

**5. Context entries são strings simples**

```rust
// Context é Vec<String>, não struct rica
pub context: Vec<String>

// Cada entrada é uma string de iteração anterior
// Não há metadata ou timestamps
```

## Configurações

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

ExecutionContext command timeout: 300 seconds (5 minutos)
```

## Downlinks (Contexto Adicional)

**Para entender melhor:**
- `/AGENTS.md` - Arquitetura geral do projeto
- `/ralph-models/AGENTS.md` - Modelos de dados compartilhados
- `/ralph-services/AGENTS.md` - Como CodeAgent é usado na orquestração de loops
- `/ralph-repositories/AGENTS.md` - Persistência de iterações e tasks

## Roadmap

### Pendências

1. **AmpProvider** → Sourcegraph Amp não implementado
2. **Tools async integration** → Integrar Tool trait com ExecutionContext
3. **Streaming responses** → Suporte a streaming de respostas LLM
4. **Tool calling** → Implementar function calling nativo (não tags)
5. **Context window management** → Tokens-aware truncation (baseado em tokens, não entradas)
6. **Circuit breaker** → Proteção contra rate limits de API

---

**Última atualização:** 2026-01-18
**Versão:** 1.0
