# Ralph Server - Templates Layer

## Propósito

Este diretório contém todos os templates HTML usados pelo Ralph Loop Manager. Templates são renderizados pelo Askama (template engine Rust) e usam HTMX + Tailwind CSS para criar uma interface web moderna e reativa.

**O que esta área faz:**
- Define estrutura visual da aplicação web
- Implementa formulários com submissão assíncrona via HTMX
- Fornece navegação SPA (Single Page Application) sem JavaScript complexo
- Renderiza páginas HTML dinâmicas a partir de dados Rust
- Gerencia estados visuais (loading, errors, success)
- Implementa design system consistente com Tailwind CSS

**O que esta área NÃO faz:**
- Não contém lógica de negócio (isso é responsabilidade de handlers e services)
- Não acessa banco de dados diretamente (recebe dados via template structs)
- Não processa requisições HTTP (isso é responsabilidade dos handlers)
- Não executa loops ou gerencia containers (backend responsibility)

## Estrutura

```
ralph-server/templates/
├── base.html              # Template base com navbar, footer e CDN includes
├── auth/
│   ├── login.html         # Formulário de login
│   └── register.html      # Formulário de registro
└── loops/
    ├── index.html         # Lista de loops com paginação
    └── new.html           # Formulário de criação de loop
```

### Template Structs (em `src/templates/mod.rs`)

```rust
#[derive(Template)]
#[template(path = "base.html")]
pub struct BaseTemplate {
    pub logged_in: bool,
}

#[derive(Template)]
#[template(path = "auth/login.html")]
pub struct LoginTemplate {
    pub logged_in: bool,
    pub csrf_token: String,
}

#[derive(Template)]
#[template(path = "loops/index.html")]
pub struct LoopListTemplate {
    pub logged_in: bool,
    pub csrf_token: String,
    pub loops: Vec<LoopSummary>,
    pub page: u32,
    pub total_pages: u32,
    pub has_prev: bool,
    pub has_next: bool,
}
```

## Invariantes Críticos

### Sempre Herdar de base.html

**TODO** template deve estender `base.html` (exceto o próprio):

```html
<!-- ✅ CERTO -->
{% extends "base.html" %}

{% block content %}
<div class="...">
  <!-- conteúdo -->
</div>
{% endblock %}

<!-- ❌ ERRADO - Sem base.html -->
<!DOCTYPE html>
<html>
<head>...</head>
<body>...</body>
</html>
```

**Por que é crítico:**
- Garante consistência de HTML structure
- Inclui automaticamente Tailwind e HTMX CDNs
- Mantém navbar/footer consistentes
- Facilita atualizações globais

### CSRF Token Obrigatório

**TODO** template com formulário ou mutations deve incluir CSRF token:

```html
<!-- 1. Meta tag para JavaScript -->
{% block head %}
<meta name="csrf-token" content="{{ csrf_token }}">
{% endblock %}

<!-- 2. Hidden input em forms -->
<form hx-post="/api/loops">
  <input type="hidden" name="csrf_token" value="{{ csrf_token }}">
  <!-- outros campos -->
</form>
```

**Por que é crítico:**
- Proteção contra CSRF attacks
- Middleware CSRF no backend valida este token
- Sem token, mutations (POST/PUT/DELETE) falham com 403

### HTMX Attributes Consistentes

Use HTMX attributes de forma consistente:

```html
<!-- ✅ CERTO - POST com redirect (form submission padrão) -->
<form hx-post="/api/loops" hx-boost="true">
  <!-- ... -->
</form>

<!-- ✅ CERTO - POST sem swap (AJAX com redirect manual) -->
<form hx-post="/api/auth/login" hx-swap="none">
  <!-- ... -->
</form>

<!-- ✅ CERTO - GET para carregar conteúdo dinâmico -->
<button hx-get="/loops/new" hx-target="#main-content" hx-swap="innerHTML">
  New Loop
</button>

<!-- ❌ ERRADO - misturando conceitos -->
<form hx-get="/api/loops">
  <!-- Forms devem usar POST para mutations -->
</form>
```

### Tailwind Classes Padrão

Use classes Tailwind consistentes para design system:

```html
<!-- Primary buttons -->
<button class="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500">
  Save
</button>

<!-- Secondary buttons -->
<button class="inline-flex items-center px-4 py-2 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500">
  Cancel
</button>

<!-- Inputs padrão -->
<input type="text" class="mt-1 focus:ring-indigo-500 focus:border-indigo-500 block w-full shadow-sm sm:text-sm border-gray-300 rounded-md" placeholder="...">

<!-- Cards/containers -->
<div class="bg-white shadow px-4 py-5 sm:rounded-lg sm:p-6">
  <!-- conteúdo -->
</div>

<!-- Success alerts -->
<div class="rounded-md bg-green-50 p-4">
  <div class="flex">
    <svg class="h-5 w-5 text-green-400">...</svg>
    <div class="ml-3">
      <h3 class="text-sm font-medium text-green-800">Success</h3>
      <div class="mt-2 text-sm text-green-700">...</div>
    </div>
  </div>
</div>

<!-- Error alerts -->
<div class="rounded-md bg-red-50 p-4">
  <div class="flex">
    <svg class="h-5 w-5 text-red-400">...</svg>
    <div class="ml-3">
      <h3 class="text-sm font-medium text-red-800">Error</h3>
      <div class="mt-2 text-sm text-red-700">...</div>
    </div>
  </div>
</div>
```

### Responsividade Sempre

**TODO** elemento interativo deve ser responsivo:

```html
<!-- ✅ CERTO - Responsivo -->
<div class="grid grid-cols-1 gap-y-6 gap-x-4 sm:grid-cols-6">
  <div class="sm:col-span-6">
    <!-- full width mobile, full width desktop -->
  </div>
  <div class="sm:col-span-3">
    <!-- full width mobile, half width desktop -->
  </div>
</div>

<!-- ❌ ERRADO - Não responsivo -->
<div class="grid grid-cols-6 gap-4">
  <div class="col-span-3">
    <!-- quebra em mobile -->
  </div>
</div>
```

## Padrões de Uso

### Criar Novo Template

**1. Definir template struct em `src/templates/mod.rs`:**

```rust
#[derive(Template)]
#[template(path = "loops/detail.html")]
pub struct LoopDetailTemplate {
    pub logged_in: bool,
    pub csrf_token: String,
    pub loop_: LoopDetail,
    pub tasks: Vec<TaskSummary>,
}
```

**2. Criar arquivo de template:**

```html
{% extends "base.html" %}

{% block title %}{{ loop_.name }} - Ralph Loop Manager{% endblock %}

{% block head %}
<meta name="csrf-token" content="{{ csrf_token }}">
{% endblock %}

{% block content %}
<div class="px-4 py-6 sm:px-0">
  <!-- conteúdo -->
</div>
{% endblock %}
```

**3. Usar no handler:**

```rust
pub async fn loop_detail(
    State(_state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(id): Path<String>,
) -> (StatusCode, Html<String>) {
    let logged_in = !user_id.is_empty();
    let csrf_token = CsrfToken::generate().to_string();

    // buscar loop e tasks...

    let template = LoopDetailTemplate {
        logged_in,
        csrf_token,
        loop_,
        tasks,
    };

    match template.render() {
        Ok(html) => (StatusCode::OK, Html(html)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("Failed to render template: {}", e)),
        ),
    }
}
```

### HTMX - Navegação SPA

Use `hx-boost="true"` para navegação automática:

```html
<!-- Links normais -->
<a href="/loops" class="...">
  Loops
</a>

<!-- Boosted: HTMX intercepta o clique e faz fetch assíncrono -->
<a href="/loops" class="..." hx-boost="true">
  Loops
</a>
```

### HTMX - Atualização Parcial

Use `hx-target` e `hx-swap` para atualizar partes específicas:

```html
<!-- Container alvo -->
<div id="loop-details">
  <!-- conteúdo atual -->
</div>

<!-- Botão que atualiza apenas o container -->
<button
  hx-get="/loops/{{ loop.id }}/details"
  hx-target="#loop-details"
  hx-swap="innerHTML"
>
  Refresh
</button>
```

Valores de `hx-swap`:
- `innerHTML`: Substitui conteúdo do target (padrão)
- `outerHTML`: Substitui o próprio target
- `beforebegin`: Insere antes do target
- `afterbegin`: Insere como primeiro filho
- `beforeend`: Insere como último filho
- `afterend`: Insere depois do target
- `none`: Não modifica DOM (útil para POST com redirect)

### HTMX - Submissão de Formulários

Padrões de submissão de formulários:

```html
<!-- Padrão 1: POST com boost (full page reload se sucesso) -->
<form hx-post="/api/auth/register" hx-boost="true">
  <input type="hidden" name="csrf_token" value="{{ csrf_token }}">
  <!-- campos -->
  <button type="submit">Submit</button>
</form>

<!-- Padrão 2: POST sem swap (handler faz redirect manual) -->
<form hx-post="/api/auth/login" hx-swap="none">
  <input type="hidden" name="csrf_token" value="{{ csrf_token }}">
  <!-- campos -->
  <button type="submit">Login</button>
</form>

<!-- Padrão 3: POST com target (atualização parcial) -->
<form hx-post="/api/loops/{{ id }}/update" hx-target="#loop-status" hx-swap="outerHTML">
  <input type="hidden" name="csrf_token" value="{{ csrf_token }}">
  <!-- campos -->
  <button type="submit">Update</button>
</form>
```

### HTMX - Loading States

Use `hx-indicator` para indicadores de loading:

```html
<!-- Indicador de loading -->
<div id="loading" class="hidden">
  <svg class="animate-spin h-5 w-5 text-gray-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
  </svg>
</div>

<!-- Botão com indicador -->
<button hx-get="/api/loops" hx-indicator="#loading">
  Refresh
</button>
```

### Askama - Condicionais e Loops

```html
<!-- If/Else -->
{% if logged_in %}
<nav>
  <!-- logged in content -->
</nav>
{% endif %}

{% if loops.is_empty() %}
<div>No loops found</div>
{% else %}
<div>{{ loops.len() }} loops</div>
{% endif %}

<!-- If/Else/Elif -->
{% if loop.status == "running" %}
<span class="text-green-600">Running</span>
{% elif loop.status == "paused" %}
<span class="text-yellow-600">Paused</span>
{% elif loop.status == "completed" %}
<span class="text-blue-600">Completed</span>
{% else %}
<span class="text-gray-600">{{ loop.status }}</span>
{% endif %}

<!-- Loop -->
{% for loop in loops %}
<tr>
  <td>{{ loop.name }}</td>
  <td>{{ loop.status }}</td>
</tr>
{% endfor %}

<!-- Loop com índice -->
{% for (index, loop) in loops.iter().enumerate() %}
<div>{{ index + 1 }}. {{ loop.name }}</div>
{% endfor %}
```

### Askama - Filtros e Modificadores

```html
<!-- String slicing -->
<p>{{ loop.created_at[..10] }}</p>  <!-- Primeiros 10 caracteres -->

<!-- Formatação condicional -->
<span class="inline-flex rounded-full px-2 text-xs font-semibold leading-5
  {% if loop.status == "running" %} bg-green-100 text-green-800
  {% elif loop.status == "paused" %} bg-yellow-100 text-yellow-800
  {% else %} bg-gray-100 text-gray-800 {% endif %}">
  {{ loop.status }}
</span>
```

### Exibir Erros de Validação

```html
<!-- No template struct -->
pub struct RegisterTemplate<'a> {
    pub logged_in: bool,
    pub csrf_token: String,
    pub errors: &'a [String],
}

<!-- No template -->
{% if !errors.is_empty() %}
<div class="rounded-md bg-red-50 p-4">
  <div class="flex">
    <div class="flex-shrink-0">
      <svg class="h-5 w-5 text-red-400" viewBox="0 0 20 20" fill="currentColor">
        <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd" />
      </svg>
    </div>
    <div class="ml-3">
      <h3 class="text-sm font-medium text-red-800">Registration failed</h3>
      <div class="mt-2 text-sm text-red-700">
        <ul class="list-disc pl-5 space-y-1">
          {% for error in errors %}
          <li>{{ error }}</li>
          {% endfor %}
        </ul>
      </div>
    </div>
  </div>
</div>
{% endif %}
```

### Paginação

```html
<!-- No template struct -->
pub struct LoopListTemplate {
    pub page: u32,
    pub total_pages: u32,
    pub has_prev: bool,
    pub has_next: bool,
    // ...
}

<!-- No template -->
{% if total_pages > 1 %}
<nav class="mt-6 flex items-center justify-between border-t border-gray-200 px-4 sm:px-0">
  <div class="-mt-2 flex w-0 flex-1">
    {% if has_prev %}
    <a href="/loops?page={{ page - 1 }}"
       class="inline-flex items-center border-t-2 border-transparent pr-1 pt-4 text-sm font-medium text-gray-500 hover:border-gray-300 hover:text-gray-700">
      Previous
    </a>
    {% endif %}
  </div>

  <div class="hidden md:-mt-2 md:flex md:space-x-8">
    <span class="inline-flex items-center border-t-2 border-indigo-500 px-4 pt-4 text-sm font-medium text-gray-900">
      Page {{ page }} of {{ total_pages }}
    </span>
  </div>

  <div class="-mt-2 flex w-0 flex-1 justify-end">
    {% if has_next %}
    <a href="/loops?page={{ page + 1 }}"
       class="inline-flex items-center border-t-2 border-transparent pl-1 pt-4 text-sm font-medium text-gray-500 hover:border-gray-300 hover:text-gray-700">
      Next
    </a>
    {% endif %}
  </div>
</nav>
{% endif %}
```

## Anti-padrões

### NUNCA FAZER

**1. JavaScript complexo para coisas simples**

```html
<!-- ❌ ERRADO - JavaScript manual -->
<button onclick="fetch('/api/loops').then(...)">Refresh</button>

<!-- ✅ CERTO - HTMX faz o fetch -->
<button hx-get="/api/loops">Refresh</button>
```

**2. Esquecer CSRF token em forms**

```html
<!-- ❌ ERRADO - Sem CSRF -->
<form hx-post="/api/loops">
  <input type="text" name="name">
  <button>Submit</button>
</form>

<!-- ✅ CERTO - Com CSRF -->
<form hx-post="/api/loops">
  <input type="hidden" name="csrf_token" value="{{ csrf_token }}">
  <input type="text" name="name">
  <button>Submit</button>
</form>
```

**3. Misturar hx-get/hx-post incorretamente**

```html
<!-- ❌ ERRADO - GET para mutation -->
<button hx-get="/api/loops" hx-post="true">
  Create Loop
</button>

<!-- ✅ CERTO - POST para mutation -->
<button hx-post="/api/loops">
  Create Loop
</button>
```

**4. Não usar base.html**

```html
<!-- ❌ ERRADO - HTML duplicado -->
<!DOCTYPE html>
<html>
<head>
  <script src="https://cdn.tailwindcss.com"></script>
  <script src="https://unpkg.com/htmx.org@1.9.10"></script>
</head>
<body>
  <nav>...</nav>
  <main>...</main>
</body>
</html>

<!-- ✅ CERTO - Herdar de base.html -->
{% extends "base.html" %}
{% block content %}
<main>...</main>
{% endblock %}
```

**5. Classes Tailwind inconsistentes**

```html
<!-- ❌ ERRADO - Misturando estilos -->
<button class="btn btn-primary bg-blue-500 rounded">Save</button>

<!-- ✅ CERTO - Classes Tailwind padrão -->
<button class="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md text-white bg-indigo-600 hover:bg-indigo-700">Save</button>
```

**6. Não tratar estados de loading**

```html
<!-- ❌ ERRADO - Sem feedback -->
<button hx-get="/api/loops">Refresh</button>

<!-- ✅ CERTO - Com indicador -->
<div id="loading" class="hidden">Loading...</div>
<button hx-get="/api/loops" hx-indicator="#loading">Refresh</button>
```

**7. Elementos não responsivos**

```html
<!-- ❌ ERRADO - Quebra em mobile -->
<div class="grid grid-cols-4 gap-4">
  <div>Item 1</div>
  <div>Item 2</div>
  <div>Item 3</div>
  <div>Item 4</div>
</div>

<!-- ✅ CERTO - Responsivo -->
<div class="grid grid-cols-1 gap-y-6 gap-x-4 sm:grid-cols-4">
  <div>Item 1</div>
  <div>Item 2</div>
  <div>Item 3</div>
  <div>Item 4</div>
</div>
```

## Dependências

### Libraries Usadas

```toml
# Template engine
askama = "0.15"  # Compile-time template rendering

# Styling (via CDN no base.html)
tailwindcss = "3.4"  # CSS framework (CDN)
htmx = "1.9.10"      # Hypermedia-driven interactions (CDN)
```

### External Resources (CDNs)

```html
<!-- Tailwind CSS -->
<script src="https://cdn.tailwindcss.com"></script>

<!-- HTMX -->
<script src="https://unpkg.com/htmx.org@1.9.10"></script>
```

### Internal Dependencies

- `src/templates/mod.rs` - Template structs que mapeiam para arquivos HTML
- Handlers em `src/handlers/` - Criam e preenchem template structs
- Models em `ralph-models` - Dados exibidos nos templates

## Armadilhas

### Confusões Comuns

**1. hx-target vs hx-swap**

```html
<!-- hx-target: ONDE inserir o resultado -->
<button hx-get="/api/data" hx-target="#result">Load</button>
<div id="result">
  <!-- HTML será inserido AQUI -->
</div>

<!-- hx-swap: COMO inserir o resultado -->
<button hx-get="/api/data" hx-target="#result" hx-swap="outerHTML">Load</button>
<div id="result">
  <!-- Este elemento SERÁ SUBSTITUÍDO pelo resultado -->
</div>
```

**2. hx-boost vs hx-get**

```html
<!-- hx-boost: torna links/forms HTMX-aware automaticamente -->
<a href="/loops" hx-boost="true">
  Loops
</a>
<!-- O clique ainda navega para /loops, mas via HTMX (se suportado) -->

<!-- hx-get: força requisição assíncrona -->
<button hx-get="/loops">
  Load Loops
</button>
<!-- O clique faz GET assíncrono, injeta response -->
```

**3. logged_in vs user_id**

```rust
// Template struct tem logged_in (bool)
pub struct LoopListTemplate {
    pub logged_in: bool,
    pub csrf_token: String,
}

// Handler preenche logged_in
let logged_in = !user_id.is_empty();
let template = LoopListTemplate {
    logged_in,  // true se user_id não vazio
    csrf_token,
    loops,
};
```

**4. Template struct vs Template path**

```rust
// O caminho do template deve corresponder ao arquivo
#[derive(Template)]
#[template(path = "loops/index.html")]  // ← deve ser "loops/index.html"
pub struct LoopListTemplate { ... }
//                  ↑ corresponde a templates/loops/index.html
```

### Comportamentos Inesperados

**1. HTMX boost pode quebrar se response não for HTML**

```rust
// ❌ ERRADO - Handler retorna JSON em rota boosted
pub async fn list_loops() -> Json<ListLoopsResponse> {
    Json(response)
}

// ✅ CERTO - Handler retorna HTML
pub async fn list_loops() -> (StatusCode, Html<String>) {
    (StatusCode::OK, Html(html))
}
```

**2. Askama valida templates em compile-time**

```rust
// Se template tem erro sintático, código NÃO compila
// Error: template syntax error in templates/loops/new.html at line 42

// Se variável não existe, NÃO compila
// Error: no field `missing_field` on struct `LoopFormTemplate`
```

**3. hx-swap="none" ainda faz request**

```html
<!-- O request é feito, mas DOM não é modificado -->
<form hx-post="/api/login" hx-swap="none">
  <!-- Handler deve fazer redirect manual -->
</form>

// Handler Rust
pub async fn login(...) -> Redirect {
    // Precisa redirect manualmente
    Redirect::to("/loops")
}
```

**4. Tailwind CDN não para produção**

```html
<!-- ❌ NÃO usar em produção -->
<script src="https://cdn.tailwindcss.com"></script>

<!-- ✅ Em produção, usar build process -->
<!-- npm install -D tailwindcss -->
<!-- npx tailwindcss init -->
```

## Downlinks (Contexto Adicional)

**Para entender melhor:**
- `/ralph-server/AGENTS.md` - Como handlers usam templates
- `/ralph-server/src/templates/mod.rs` - Template structs implementation
- `/ralph-server/src/handlers/` - Handlers que criam e renderizam templates
- `https://htmx.org/docs` - HTMX documentation oficial
- `https://tailwindcss.com/docs` - Tailwind CSS documentation oficial

---

**Última atualização:** 2026-01-18
**Versão:** 1.0
