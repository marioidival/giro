# PRD: Ralph Loop Management Web

**Versão:** 1.2
**Data:** 2026-01-17
**Status:** Rascunho

---

## 1. Contexto e Problema

### O que é Ralph?

Ralph é uma técnica de desenvolvimento com IA baseada em um loop infinito:

```bash
while :; do cat PROMPT.md | npx --yes @sourcegraph/amp ; done
```

A ideia é simples: um LLM rodando continuamente, lendo um prompt, executando tarefas, e repetindo indefinidamente. Ralph tem se mostrado capaz de:
- Construir projetos greenfield completos
- Criar novas linguagens de programação
- Trabalhar com tecnologias fora do training set

**O problema:** Ralph é atualmente uma técnica manual, rodando em terminal, sem gerenciamento, monitoramento ou controle adequados.

### Como Ralph Funciona: PRD + Tasks

Ralph opera com dois conceitos fundamentais:

**PRD (Product Requirements Document)**
- Define o contexto do projeto
- Descreve o que está sendo construído
- Permanece constante durante o loop
- Exemplo: "Construir um compilador para linguagem CURSED"

**Tasks (Unidades de Trabalho)**
- São as atividades específicas executadas a cada iteração
- Podem ser criadas manualmente ou pelo próprio LLM
- Têm estados: `pending` → `in_progress` → `completed`/`failed`
- Podem ter dependências entre si

```
┌─────────────────────────────────────────────────────────────┐
│                    Loop Execution Flow                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. Loop Inicia                                              │
│     ↓                                                        │
│  2. Carrega PRD do Loop (contexto do projeto)               │
│     ↓                                                        │
│  3. Busca próxima Task com status `pending`                 │
│     ↓                                                        │
│  4. Marca Task como `in_progress`                           │
│     ↓                                                        │
│  5. Envia para LLM: PRD + Task atual                        │
│     ↓                                                        │
│  6. LLM executa (gera código, arquivos, etc)                │
│     ↓                                                        │
│  7. Task marcada como `completed` ou `failed`               │
│     ↓                                                        │
│  8. LLM pode criar novas Tasks baseado no resultado         │
│     ↓                                                        │
│  9. Repete do passo 3                                        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Exemplo Prático:**

```
PRD: "Construir API REST para gestão de usuários"

Tasks criadas inicialmente:
  1. [pending] Definir modelos de dados (User, etc)
  2. [pending] Implementar rota POST /users
  3. [pending] Implementar rota GET /users/:id
  4. [pending] Implementar rota PUT /users/:id
  5. [pending] Implementar rota DELETE /users/:id

Iteração 1: PRD + Task 1 → LLM cria modelos
Iteração 2: PRD + Task 2 → LLM implementa POST /users
Iteração 3: PRD + Task 3 → LLM implementa GET /users/:id
...

Durante execução, o LLM pode criar novas tasks:
  6. [pending] Adicionar validação de email
  7. [pending] Implementar autenticação JWT
```

### O Problema que Resolvemos

**Desenvolvedores que usam Ralph enfrentam:**

1. **Falta de visibilidade:** Não sabem o que Ralph está fazendo em tempo real
2. **Sem histórico:** Perdem-se iterações e experimentos anteriores
3. **Dificuldade de orquestração:** Rodar múltiplos loops simultâneos é complexo
4. **Ausência de controle:** Não há forma fácil de pausar/retomar loops
5. **Sem isolamento:** Código rodando na máquina do desenvolvedor sem sandbox
6. **Integração manual:** Resultados precisam ser copiados para git manualmente

---

## 2. Solução Proposta

**Ralph Loop Management Web** é uma plataforma web para gerenciar, monitorar e orquestrar loops Ralph com:

- Interface web para criar/editar prompts
- Execução isolada em containers Docker
- Dashboard em tempo real dos loops em execução
- Histórico completo de iterações
- Integração com Git (PRs/MRs automáticas)
- Suporte a múltiplos providers LLM
- Sistema multi-usuário

---

## 3. Personas

### Primária: Engenheiro de Software Autônomo

**Perfil:**
- Desenvolvedor experiente trabalhando em projetos greenfield
- Usa IA para acelerar desenvolvimento
- Precisa gerenciar múltiplos experimentos simultâneos
- Valoriza isolamento e segurança

**Dores:**
- Perde tempo gerenciando loops manuais em terminais
- Não tem visibilidade do progresso dos loops
- Precisa de histórico para comparar experimentos

**Ganhos:**
- Gerencia todos loops em um lugar
- Visualiza progresso em tempo real
- Compara iterações facilmente
- Envia código automaticamente para PRs

### Secundária: Time de P&D / Innovation Lab

**Perfil:**
- Equipe explorando novas tecnologias com IA
- Múltiplos desenvolvedores rodando experimentos
- Precisa compartilhar resultados entre time

**Dores:**
- Dificuldade de compartilhar prompts e resultados
- Colisão de recursos quando múltiplos usam Ralph
- Sem centralização de conhecimento

**Ganhos:**
- Repositório compartilhado de prompts
- Multi-usuário com controle de acesso
- Histórico compartilhado do time

---

## 4. Casos de Uso

### UC1: Criar e Executar Loop

**Ator:** Engenheiro de Software

**Fluxo:**
1. Usuário acessa dashboard e clica "Novo Loop"
2. Define nome, descrição e **PRD** (markdown)
3. Seleciona provider LLM (Claude/OpenAI/Amp) e modelo
4. Configura parâmetros (max iterations, timeout)
5. Configura repositório Git de destino (opcional)
6. **Adiciona Tasks Iniciais** (ou deixa vazio para LLM criar)
   - Pode adicionar tasks manualmente uma a uma
   - Pode importar tasks de um template
   - Pode deixar vazio - LLM vai criar tasks autonomamente
7. (Opcional) Configura **Docker image** customizada
   - Default: imagem oficial do Ralph Loop Manager
   - Inclui: ferramentas de dev (git, curl, editors), runtimes (Node, Python, Rust, Go)
8. Clica "Executar"
9. Sistema cria container Docker com volumes montados:
   - `/workspace/prd.md` → PRD do projeto
   - `/workspace/task.md` → **Apenas a task atual** sendo executada
   - `/workspace/repo/` → Repositório Git (clonado ou inicializado)
10. Loop inicia dentro do container
11. Usuário acompanha execução em tempo real

**Nota:** O loop só executa quando há tasks com status `pending`. Cada iteração pega uma task e envia **PRD + Task** para o LLM.

### UC2: Monitorar Loop em Tempo Real

**Ator:** Engenheiro de Software

**Fluxo:**
1. Usuário acessa dashboard
2. Vê lista de loops com status (running/paused/completed/error)
3. Clica em loop específico
4. Visualiza:
   - **Kanban de Tasks** (colunas: pending, in_progress, completed, failed)
   - Task atual sendo executada
   - Output da iteração em tempo real
   - Arquivos criados/modificados (live preview)
   - Métricas (tempo, tokens usados)
5. Pode pausar/retomar/parar loop
6. Pode adicionar/remover/reordenar tasks manualmente

### UC3: Gerenciar Tasks

**Ator:** Engenheiro de Software

**Fluxo:**
1. Usuário acessa view de detalhes do loop
2. Vê todas tasks organizadas por status
3. Pode criar nova task (título + descrição)
4. Pode editar task existente
5. Pode reordenar tasks (drag & drop)
6. Pode criar subtasks
7. Pode cancelar task
8. Pode re-executar task failed

### UC4: Histórico e Comparação

**Ator:** Engenheiro de Software

**Fluxo:**
1. Usuário acessa seção "Histórico" do loop
2. Filtra por task, data, status
3. Visualiza iterações anteriores (agrupadas por task)
4. Compara outputs entre iterações da mesma task
5. Vê diff de arquivos gerados
6. Pode ver tasks criadas pelo LLM durante execução

### UC5: Integração Git

**Ator:** Engenheiro de Software

**Fluxo:**
1. Usuário configura loop com repositório Git
2. Loop gera/modifica arquivos na microVM
3. Ao completar (ou em intervalos definidos), sistema cria commit
4. Sistema abre Pull Request (GitHub) ou Merge Request (GitLab)
5. Usuário recebe notificação
6. Review e merge no fluxo normal

---

## 5. Features

### 5.1 Core MVP

#### Gestão de Loops
- [ ] Criar novo loop (nome, descrição, **PRD** markdown)
- [ ] Editar loop existente
- [ ] Deletar loop
- [ ] Listar loops (com filtros e busca)
- [ ] Clonar loop (para experimentação)

#### Gestão de Tasks
- [ ] Criar task manualmente (título + descrição)
- [ ] Editar task existente
- [ ] Deletar task
- [ ] Reordenar tasks (drag & drop)
- [ ] Criar subtasks (parent_task_id)
- [ ] Cancelar task (status → cancelled)
- [ ] Re-executar task failed
- [ ] Visualizar tasks em Kanban (pending/in_progress/completed/failed)
- [ ] Filtrar tasks por status, criador (user/llm), prioridade
- [ ] Importar tasks de template
- [ ] LLM pode criar novas tasks durante execução

#### Execução
- [ ] Iniciar loop em container Docker
- [ ] Pausar loop (pausar container)
- [ ] Retomar loop pausado
- [ ] Parar loop (stop + remove container)
- [ ] Configurar Docker image (default ou customizada)
- [ ] Configurar max iterations
- [ ] Configurar timeout por iteração
- [ ] Configurar delay entre iterações
- [ ] Configurar resource limits (CPU, memória)

#### Monitoramento
- [ ] Dashboard com status de todos loops
- [ ] View detalhada de loop em execução
- [ ] Output em tempo real (streaming)
- [ ] Preview de arquivos gerados
- [ ] Métricas: iterations, tempo total, tokens

#### Histórico
- [ ] Histórico de iterações por loop
- [ ] Comparação entre iterações (diff)
- [ ] Download de artefatos (zip)
- [ ] Filtros por data/status

#### Multi-Provider LLM
- [ ] Anthropic Claude
- [ ] OpenAI (GPT-4, etc)
- [ ] Sourcegraph Amp
- [ ] Configuração de API keys por usuário
- [ ] Seleção de modelo

#### Integração Git
- [ ] Configurar repositório (GitHub/GitLab)
- [ ] Autenticação OAuth ou personal access token
- [ ] Commit automático de mudanças
- [ ] Criação de PR/MR
- [ ] Configurar branch pattern

### 5.2 Features Intermediárias (Post-MVP)

#### Usuários
- [ ] Login/senha (multi-usuário básico)
- [ ] Perfil (nome, email)
- [ ] Loops privados vs compartilhados
- [ ] API keys por usuário

#### Orquestração
- [ ] Executar múltiplos loops simultâneos
- [ ] Fila de execução (quando limite atingido)
- [ ] Priorização de loops
- [ ] Rate limiting por usuário/provider

#### Automação
- [ ] Webhooks (eventos de loop)
- [ ] Scheduling (agendar execução)
- [ ] Triggers (git push, webhook)

### 5.3 Features Avançadas (Futuro)

- [ ] Automação com estados (state machine)
- [ ] RBAC completo (roles, permissões)
- [ ] Templates de prompts
- [ ] Marketplace de prompts
- [ ] Export/import de loops
- [ ] CLI client
- [ ] API REST

---

## 6. Arquitetura Técnica

### 6.1 Visão Geral

```
┌─────────────────────────────────────────────────────────────┐
│                         Frontend                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   HTMX       │  │  Tailwind    │  │  WebSocket   │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      Backend (Rust)                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │    Axum      │  │    SQLx      │  │   Tokio      │      │
│  │  (HTTP/WS)   │  │  (SQLite)    │  │ (Async RT)   │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                                                               │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                   Loop Manager                       │   │
│  │  - Orquestração                                      │   │
│  │  - Estado                                            │   │
│  │  - Fila                                              │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                               │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                   Docker Manager                     │   │
│  │  - Criação/destroy de containers                     │   │
│  │  - Volume management                                 │   │
│  │  - Resource limits                                   │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ LLM Adapter  │  │ Git Adapter  │  │  Auth        │      │
│  │ (Multi-prov) │  │ (GH/GL)      │  │  (Basic)     │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Infrastructure                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │    Docker    │  │   SQLite     │  │  Filesystem  │      │
│  │ (containers) │  │  (database)  │  │ (artefatos)  │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

### 6.2 Componentes

#### Frontend
- **HTMX 2.0:** Interatividade sem JavaScript complexo
- **Tailwind CSS 4.x:** Estilização rápida e responsiva
- **WebSocket:** Streaming em tempo real de outputs

#### Backend
- **Axum 0.8:** Framework web async (HTTP/WebSocket)
- **Tokio 1.x:** Runtime async para concorrência
- **SQLx 0.8:** Queries type-safe com SQLite
- **Serde 1.x:** Serialização/deserialização
- **Tower 0.5:** Middleware (auth, logging, CORS)
- **Tracing 0.1:** Structured logging

#### Gerenciamento de Loops
- **State Machine:** Estados (created, running, paused, completed, error)
- **Queue:** Sistema de fila para execução
- **Supervisor:** Monitorar e recuperar loops falhos

#### Docker
- **Container Manager:** Wrapper sobre Docker API (bollard)
- **Volume Management:** Montar PRD, task e repo no container
- **Resource Limits:** CPU, memória, disco por container
- **Network:** Isolamento de network (bridge/none)

#### LLM Providers
- **Adapter Pattern:** Interface comum para providers
- **Implementações:** Claude (anthropic-rust), OpenAI (async-openai)
- **Token Tracking:** Monitorar uso por loop

#### Git Integration
- **GitHub API:** octocrab 0.42
- **GitLab API:** gitlab-sdk 0.2
- **Git Client:** git2 0.20

---

## 7. Stack Tecnológica

### Backend
| Componente | Tecnologia | Versão | Justificativa |
|-----------|-----------|--------|---------------|
| Linguagem | Rust | 1.85+ | Performance, segurança, async nativo |
| Edition | 2024 | - | Última edition com features modernas |
| Framework Web | Axum | 0.8 | Moderno, async, excelente suporte WebSocket |
| Database | SQLx | 0.8 | Type-safe queries, support para SQLite |
| Runtime | Tokio | 1.42 | Padrão Rust para async |
| Middleware | Tower | 0.5 | Auth, logging, CORS |
| Async Traits | async-trait | 0.1 | Traits em contextos async |
| Serialização | Serde | 1.0 | Padrão Rust |
| Logging | Tracing | 0.1 | Structured logging moderno |
| HTTP Client | Reqwest | 0.12 | Client HTTP async |
| WebSocket | Tokio-tungstenite | 0.26 | WebSocket async |
| Templates | Askama | 0.13 | Templates HTML type-safe |

### LLM Clients
| Componente | Tecnologia | Versão |
|-----------|-----------|--------|
| Claude | anthropic-rust | 0.2 |
| OpenAI | async-openai | 0.28 |
| Amp | Custom | - |

### Git Clients
| Componente | Tecnologia | Versão |
|-----------|-----------|--------|
| GitHub | octocrab | 0.42 |
| GitLab | gitlab-sdk | 0.2 |
| Git Local | git2 | 0.20 |

### Auth & Security
| Componente | Tecnologia | Versão |
|-----------|-----------|--------|
| Password Hashing | bcrypt | 0.16 |
| JWT | jsonwebtoken | 9.3 |
| Session | tower-sessions | 0.13 |
| CSRF | tower-csrf | 0.4 |

### Frontend
| Componente | Tecnologia | Versão | Justificativa |
|-----------|-----------|--------|---------------|
| HTML/UX | HTMX | 2.0 | Interatividade sem JS complexo |
| Estilização | Tailwind CSS | 4.x | Desenvolvimento rápido |
| Icons | Heroicons | SVG inline | Compatível com Tailwind |
| Streaming | SSE/WebSocket | HTMX native | Real-time updates |

### Infrastructure
| Componente | Tecnologia | Justificativa |
|-----------|-----------|---------------|
| Containerização | Docker | Containers leves e isoladas |
| Docker Client | bollard | Rust Docker API client |
| Database | SQLite | Portabilidade, backup simples |
| Filesystem | Local + S3 (futuro) | Armazenamento de artefatos |

### DevOps
| Componente | Tecnologia | Versão |
|-----------|-----------|--------|
| Container | Docker | 27.x |
| Build | Cargo | embutido |
| Tests | Cargo test | embutido |
| Linting | Clippy | embutido |
| Formatting | rustfmt | embutido |
| Coverage | tarpaulin | 0.31 |

---

## 8. Data Model

### Loop
```sql
CREATE TABLE loops (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    prd TEXT NOT NULL,  -- PRD do projeto (markdown)
    owner_id TEXT NOT NULL,  -- user_id
    provider TEXT NOT NULL,  -- claude, openai, amp
    model TEXT NOT NULL,
    docker_image TEXT NOT NULL DEFAULT 'ralph-loop-manager:latest',  -- imagem a usar
    cpu_limit INTEGER DEFAULT 1,  -- vCPUs (ex: 1 = 100%, 2 = 200%)
    memory_limit INTEGER DEFAULT 1024,  -- MB
    max_iterations INTEGER DEFAULT 100,
    iteration_timeout INTEGER DEFAULT 300,  -- seconds
    iteration_delay INTEGER DEFAULT 0,
    git_repo_url TEXT,
    git_branch_pattern TEXT DEFAULT "ralph/{loop_id}/{timestamp}",
    status TEXT NOT NULL,  -- created, running, paused, completed, error
    current_iteration INTEGER DEFAULT 0,
    container_id TEXT,  -- Docker container ID (quando rodando)
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (owner_id) REFERENCES users(id)
);
```

### Task
```sql
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    loop_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,  -- descrição detalhada da task
    status TEXT NOT NULL,  -- pending, in_progress, completed, failed, cancelled
    priority INTEGER DEFAULT 0,  -- maior = mais prioritário
    parent_task_id TEXT,  -- para subtasks
    created_by TEXT NOT NULL,  -- 'user' ou 'llm'
    iteration_id TEXT,  -- iteração que executou esta task
    started_at TEXT,
    completed_at TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (loop_id) REFERENCES loops(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_task_id) REFERENCES tasks(id) ON DELETE SET NULL,
    FOREIGN KEY (iteration_id) REFERENCES iterations(id) ON DELETE SET NULL
);

-- Índices para performance
CREATE INDEX idx_tasks_loop_status ON tasks(loop_id, status);
CREATE INDEX idx_tasks_status_priority ON tasks(status, priority DESC);
CREATE INDEX idx_tasks_parent ON tasks(parent_task_id);
```

### Iteration
```sql
CREATE TABLE iterations (
    id TEXT PRIMARY KEY,
    loop_id TEXT NOT NULL,
    task_id TEXT NOT NULL,  -- task que foi executada nesta iteração
    iteration_number INTEGER NOT NULL,
    output TEXT,  -- output do LLM
    error TEXT,
    status TEXT NOT NULL,  -- running, completed, error
    started_at TEXT NOT NULL,
    completed_at TEXT,
    tokens_used INTEGER,
    FOREIGN KEY (loop_id) REFERENCES loops(id),
    FOREIGN KEY (task_id) REFERENCES tasks(id)
);
```

### File (artefatos)
```sql
CREATE TABLE files (
    id TEXT PRIMARY KEY,
    iteration_id TEXT NOT NULL,
    path TEXT NOT NULL,  -- caminho na VM
    content_hash TEXT,  -- hash do conteúdo
    size INTEGER,
    file_type TEXT,  -- code, config, markdown, etc
    created_at TEXT NOT NULL,
    FOREIGN KEY (iteration_id) REFERENCES iterations(id)
);
```

### User
```sql
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TEXT NOT NULL
);
```

### API Keys
```sql
CREATE TABLE api_keys (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    provider TEXT NOT NULL,  -- claude, openai, amp
    key_encrypted TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
```

### Git Config
```sql
CREATE TABLE git_configs (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    provider TEXT NOT NULL,  -- github, gitlab
    access_token_encrypted TEXT NOT NULL,
    default_username TEXT,
    default_email TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
```

---

## 9. API Endpoints (Preliminar)

### Loops
- `GET /api/loops` - Listar loops
- `POST /api/loops` - Criar loop
- `GET /api/loops/:id` - Detalhes do loop
- `PUT /api/loops/:id` - Atualizar loop
- `DELETE /api/loops/:id` - Deletar loop
- `POST /api/loops/:id/start` - Iniciar execução
- `POST /api/loops/:id/pause` - Pausar execução
- `POST /api/loops/:id/resume` - Retomar execução
- `POST /api/loops/:id/stop` - Parar execução

### Tasks
- `GET /api/loops/:id/tasks` - Listar tasks do loop
- `POST /api/loops/:id/tasks` - Criar nova task
- `GET /api/tasks/:id` - Detalhes da task
- `PUT /api/tasks/:id` - Atualizar task
- `DELETE /api/tasks/:id` - Deletar task
- `POST /api/tasks/:id/cancel` - Cancelar task
- `POST /api/tasks/:id/retry` - Re-executar task failed
- `POST /api/tasks/:id/reorder` - Reordenar task (mudar priority)
- `GET /api/tasks/:id/subtasks` - Listar subtasks
- `POST /api/tasks/:id/subtasks` - Criar subtask

### Iterations
- `GET /api/loops/:id/iterations` - Histórico de iterações
- `GET /api/loops/:id/iterations/:n` - Detalhes da iteração
- `GET /api/iterations/:id/files` - Arquivos da iteração

### WebSocket
- `WS /api/loops/:id/stream` - Stream em tempo real

### Auth
- `POST /api/auth/register` - Registro
- `POST /api/auth/login` - Login
- `POST /api/auth/logout` - Logout

### Users
- `GET /api/users/me` - Perfil próprio
- `PUT /api/users/me` - Atualizar perfil
- `POST /api/users/me/api-keys` - Adicionar API key
- `DELETE /api/users/me/api-keys/:id` - Remover API key
- `POST /api/users/me/git-configs` - Adicionar config Git

---

## 10. Segurança

### Autenticação
- Senhas hasheadas com bcrypt (cost 12)
- Session-based authentication (cookies)
- CSRF protection via tower-csrf
- HTTP-only, Secure, SameSite cookies

### Autorização
- Cada usuário só acessa seus loops
- Loops podem ser marcados como "shared"

### Isolamento
- Cada loop roda em container Docker isolado
- Network restrictions nos containers
- Resource limits (CPU, memória, disco)
- Non-root user dentro do container

### Secrets
- API keys encriptadas no banco (AES-256-GCM)
- Variáveis de ambiente para secrets do servidor
- Rotation de keys

### Sandbox
- Containers sem acesso ao host
- Filesystem isolado (read-only exceto /workspace)
- Network controlada (bridge/none)

---

## 11. Roadmap

### Fase 1: MVP Core (4-6 semanas)
**Sprint 1-2: Fundação**
- Setup projeto (Cargo, estrutura)
- Database schema + migrations (SQLx)
- Models básicos (Loop, User)
- Auth básico (registro, login, sessions)

**Sprint 3-4: Gestão de Loops**
- CRUD de loops
- UI básica (Askama + HTMX + Tailwind 4.x)
- Listagem e detalhes

**Sprint 5-6: Execução Básica**
- Docker integration (bollard)
- Criação/gerenciamento de containers
- Montagem de volumes (PRD, task, repo)
- Monitoramento básico de containers

### Fase 2: MVP Features (4-6 semanas)
**Sprint 7-8: LLM Integration**
- Adapter pattern para providers
- Implementação Claude (anthropic-rust 0.2)
- Implementação OpenAI (async-openai 0.28)
- Implementação Amp

**Sprint 9-10: Monitoramento**
- WebSocket streaming (tokio-tungstenite)
- Dashboard real-time
- Histórico de iterações

**Sprint 11-12: Git Integration**
- GitHub API (octocrab 0.42)
- GitLab API (gitlab-sdk 0.2)
- Criação de PRs/MRs

### Fase 3: Polimento (2-4 semanas)
**Sprint 13-14: UX e Melhorias**
- Melhorias na UI (HTMX 2.0 features)
- Performance profiling
- Testes E2E

---

## 12. Métricas de Sucesso

### Técnicas
- Latência de stream < 100ms
- Uptime > 99%
- Tempo de boot de VM < 5s
- Suportar 10+ loops simultâneos

### Produto
- Tempo para criar e executar loop < 2min
- Tempo para review de iteração < 30s
- Taxa de sucesso de iterações > 95%

---

## 13. Riscos e Mitigações

| Risco | Probabilidade | Impacto | Mitigação |
|-------|--------------|---------|-----------|
| Docker host não disponível | Baixa | Alto | Suportar modo sem container (dev local) |
| Performance com múltiplos containers | Média | Médio | Pool de containers, fila de execução, resource limits |
| Custo de LLMs | Média | Alto | Tracking de tokens, limits |
| Segurança de containers | Baixa | Médio | Non-root user, read-only fs, network restrictions |

---

## 14. Próximos Passos

1. **Validação:** Revisar PRD v1.2 com stakeholders
2. **Docker Image:** Construir imagem base `ralph-loop-manager:latest`
3. **Design System:** Definir UI/UX patterns
4. **Implementação:** Seguir roadmap fase 1

---

## Apêndice A: Como o LLM Cria Tasks

### Mecanismo de Criação de Tasks pelo LLM

O sistema permite que o próprio LLM crie novas tasks durante a execução do loop. Isso permite que o projeto evolua organicamente.

#### Quando o LLM cria tasks:

1. **Ao completar uma task**: O LLM pode identificar subtasks necessárias
2. **Ao encontrar um problema**: Pode criar tasks para corrigir
3. **Ao identificar melhorias**: Pode criar tasks para refatoração
4. **Ao descobrir dependências**: Pode criar tasks que precisam ser feitas primeiro

#### Formato de Criação:

O sistema instrui o LLM a criar tasks usando um formato estruturado no output:

```markdown
<TASKS>
[
  {
    "title": "Adicionar validação de email",
    "description": "Implementar validação regex para formato de email válido",
    "priority": 5
  },
  {
    "title": "Criar testes para User model",
    "description": "Escrever testes unitários para o modelo de User",
    "priority": 3,
    "parent_task_id": "current_task_id"
  }
]
</TASKS>
```

#### Processamento:

1. **Parser**: Sistema detecta tags `<TASKS>` no output
2. **Validação**: Verifica formato JSON válido
3. **Criação**: Insere tasks no banco com `created_by = 'llm'`
4. **Notificação**: Usuário vê novas tasks no Kanban em tempo real
5. **Aprovação**: Usuário pode editar/cancelar tasks criadas pelo LLM

#### Configuração:

Loop pode ter configurações para limitar criação automática:
- `max_llm_tasks`: Número máximo de tasks que LLM pode criar (default: 50)
- `require_approval`: Se true, tasks criadas pelo LLM ficam `pending` até aprovação
- `llm_task_priority`: Prioridade padrão para tasks criadas pelo LLM

---

## Apêndice B: Exemplo de Prompt Ralph

```markdown
# CURSED Language Compiler

You are building a new esoteric programming language called CURSED.

## Requirements:
- Syntax inspired by Rust but intentionally awkward
- Compile to WebAssembly
- Create compiler, tests, and documentation

## Process:
1. Start by creating a basic playground
2. Implement lexer
3. Implement parser
4. Implement codegen to WASM
5. Write comprehensive tests
6. Document everything

## Constraints:
- All code must be in /src
- Use cargo for build
- Run `cargo test` after each change
- If tests fail, fix before proceeding
```

---

## Apêndice C: Docker Considerações

### Imagem Base: Ralph Loop Manager

A imagem oficial `ralph-loop-manager:latest` inclui:

**Ferramentas de Desenvolvimento:**
- `git` - Controle de versão
- `curl`/`wget` - Downloads
- `vim`/`nano` - Editores de texto
- `jq` - Processamento de JSON
- `ripgrep` (rg) - Busca rápida

**Runtimes e Linguagens:**
- **Node.js 20.x** - npm, npx, pnpm
- **Python 3.12** - pip, poetry, uv
- **Rust 1.85** - cargo, rustup
- **Go 1.23** - go mod

**Utilitários:**
- Docker CLI (para Docker-in-Docker se necessário)
- Build tools (gcc, make, cmake)

### Estrutura do Container

```
/workspace/
├── prd.md              # PRD do projeto (montado do host)
├── task.md             # Task atual (montado do host)
└── repo/               # Repositório Git (montado do host)
    ├── .git/
    ├── src/
    └── ...
```

### Volume Mounts

| Host Path | Container Path | Descrição |
|-----------|---------------|-----------|
| `/var/ralph/loops/{id}/prd.md` | `/workspace/prd.md` | PRD do projeto |
| `/var/ralph/tasks/{id}.md` | `/workspace/task.md` | Task atual (altera a cada iteração) |
| `/var/ralph/repos/{id}/` | `/workspace/repo/` | Código fonte |

### Resource Limits

**Default:**
- CPU: 1 vCPU (100% de 1 core)
- Memory: 1024 MB
- Disk: 10 GB (growable)
- Network: bridge (acesso externo permitido)

**Configurável por Loop:**
- `cpu_limit`: 0.5 a 4 vCPUs
- `memory_limit`: 256MB a 8GB

### Lifecycle do Container

```
1. Loop criado (status: created)
   ↓
2. Container criado (docker create)
   - Volumes montados
   - Configuração aplicada
   ↓
3. Container iniciado (docker start)
   - Script entrypoint roda
   - Loop começa a executar
   (status: running)
   ↓
4. Pausa (docker pause)
   (status: paused)
   ↓
5. Retomada (docker unpause)
   (status: running)
   ↓
6. Parada (docker stop + rm)
   - Cleanup de volumes
   (status: completed/error)
```

### Security

**Isolamento:**
- Cada loop roda em container isolado
- Sem privilégios escalated (non-root user)
- Network opcionalmente desabilitada
- Read-only filesystem (exceto /workspace)

**Cleanup:**
- Containers são removidos após completion
- Volumes temporários são limpos
- Logs são persistidos fora do container

---

## Apêndice D: Cargo.toml Template

```toml
[package]
name = "ralph-loop-manager"
version = "0.1.0"
edition = "2024"

[dependencies]
# Web
axum = { version = "0.8", features = ["ws", "macros"] }
tower = { version = "0.5", features = ["full"] }
tower-http = { version = "0.6", features = ["fs", "trace", "cors"] }
tower-sessions = "0.13"
tower-csrf = "0.4"

# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "chrono", "uuid"] }

# Async
tokio = { version = "1.42", features = ["full"] }
async-trait = "0.1"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Templates
askama = { version = "0.13", features = ["with-axum"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# HTTP
reqwest = { version = "0.12", features = ["json"] }
tokio-tungstenite = "0.26"

# Auth
bcrypt = "0.16"
jsonwebtoken = "9.3"

# LLM
anthropic-rust = "0.2"
async-openai = "0.28"

# Git
octocrab = "0.42"
gitlab-sdk = "0.2"
git2 = "0.20"

# Docker
bollard = "0.18"

# Utils
uuid = { version = "1.11", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1.0"
thiserror = "2.0"

[dev-dependencies]
tarpaulin = "0.31"
```

---

**Fim do PRD v1.0**
