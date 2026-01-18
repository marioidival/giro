# Ralph Server - Templates Layer

## Purpose

This directory contains all HTML templates used by the Ralph Loop Manager. Templates are rendered by Askama (Rust template engine) and use HTMX + Tailwind CSS to create a modern, reactive web interface.

**What this area does:**
- Defines visual structure of web application
- Implements forms with asynchronous submission via HTMX
- Provides SPA (Single Page Application) navigation without complex JavaScript
- Renders dynamic HTML pages from Rust data
- Manages visual states (loading, errors, success)
- Implements consistent design system with Tailwind CSS

**What this area does NOT do:**
- Does not contain business logic (responsibility of handlers and services)
- Does not access database directly (receives data via template structs)
- Does not process HTTP requests (responsibility of handlers)
- Does not execute loops or manage containers (backend responsibility)

## Structure

```
ralph-server/templates/
├── base.html              # Base template with navbar, footer and CDN includes
├── auth/
│   ├── login.html         # Login form
│   └── register.html      # Register form
└── loops/
    ├── index.html         # Loop list with pagination
    └── new.html           # Create loop form
```

### Template Structs (in `src/templates/mod.rs`)

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

## Critical Invariants

### Always Extend base.html

**ALL** templates must extend `base.html` (except itself):

```html
<!-- ✅ CORRECT -->
{% extends "base.html" %}

{% block content %}
<div class="...">
  <!-- content -->
</div>
{% endblock %}

<!-- ❌ WRONG - Without base.html -->
<!DOCTYPE html>
<html>
<head>...</head>
<body>...</body>
</html>
```

**Why this is critical:**
- Ensures HTML structure consistency
- Automatically includes Tailwind and HTMX CDNs
- Maintains consistent navbar/footer
- Facilitates global updates

### CSRF Token Mandatory

**ALL** templates with forms or mutations must include CSRF token:

```html
<!-- 1. Meta tag for JavaScript -->
{% block head %}
<meta name="csrf-token" content="{{ csrf_token }}">
{% endblock %}

<!-- 2. Hidden input in forms -->
<form hx-post="/api/loops">
  <input type="hidden" name="csrf_token" value="{{ csrf_token }}" />
  <!-- other fields -->
</form>
```

**Why this is critical:**
- Protection against CSRF attacks
- Backend CSRF middleware validates this token
- Without token, mutations (POST/PUT/DELETE) fail with 403

### Consistent HTMX Attributes

Use HTMX attributes consistently:

```html
<!-- ✅ CORRECT - POST with redirect (default form submission) -->
<form hx-post="/api/loops" hx-boost="true">
  <!-- ... -->
</form>

<!-- ✅ CORRECT - POST without swap (AJAX with manual redirect) -->
<form hx-post="/api/auth/login" hx-swap="none">
  <!-- ... -->
</form>

<!-- ✅ CORRECT - GET to load dynamic content -->
<button hx-get="/loops/new" hx-target="#main-content" hx-swap="innerHTML">
  New Loop
</button>

<!-- ❌ WRONG - mixing concepts -->
<form hx-get="/api/loops">
  <!-- Forms should use POST for mutations -->
</form>
```

### Standard Tailwind Classes

Use consistent Tailwind classes for design system:

```html
<!-- Primary buttons -->
<button class="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500">
  Save
</button>

<!-- Secondary buttons -->
<button class="inline-flex items-center px-4 py-2 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500">
  Cancel
</button>

<!-- Standard inputs -->
<input type="text" class="mt-1 focus:ring-indigo-500 focus:border-indigo-500 block w-full shadow-sm sm:text-sm border-gray-300 rounded-md" placeholder="...">

<!-- Cards/containers -->
<div class="bg-white shadow px-4 py-5 sm:rounded-lg sm:p-6">
  <!-- content -->
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

### Always Responsive

**ALL** interactive elements must be responsive:

```html
<!-- ✅ CORRECT - Responsive -->
<div class="grid grid-cols-1 gap-y-6 gap-x-4 sm:grid-cols-6">
  <div class="sm:col-span-6">
    <!-- full width mobile, full width desktop -->
  </div>
  <div class="sm:col-span-3">
    <!-- full width mobile, half width desktop -->
  </div>
</div>

<!-- ❌ WRONG - Not responsive -->
<div class="grid grid-cols-6 gap-4">
  <div class="col-span-3">
    <!-- breaks on mobile -->
  </div>
</div>
```

## Usage Patterns

### Creating New Template

**1. Define template struct in `src/templates/mod.rs`:**

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

**2. Create template file:**

```html
{% extends "base.html" %}

{% block title %}{{ loop_.name }} - Ralph Loop Manager{% endblock %}

{% block head %}
<meta name="csrf-token" content="{{ csrf_token }}">
{% endblock %}

{% block content %}
<div class="px-4 py-6 sm:px-0">
  <!-- content -->
</div>
{% endblock %}
```

**3. Use in handler:**

```rust
pub async fn loop_detail(
    State(_state): State<AppState>,
    Extension(user_id): Extension<String>,
    Path(id): Path<String>,
) -> (StatusCode, Html<String>) {
    let logged_in = !user_id.is_empty();
    let csrf_token = CsrfToken::generate().to_string();

    // fetch loop and tasks...

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

### HTMX - SPA Navigation

Use `hx-boost="true"` for automatic navigation:

```html
<!-- Regular links -->
<a href="/loops" class="...">
  Loops
</a>

<!-- Boosted: HTMX intercepts click and does async fetch -->
<a href="/loops" class="..." hx-boost="true">
  Loops
</a>
```

### HTMX - Partial Updates

Use `hx-target` and `hx-swap` to update specific parts:

```html
<!-- Target container -->
<div id="loop-details">
  <!-- current content -->
</div>

<!-- Button that updates only the container -->
<button
  hx-get="/loops/{{ loop.id }}/details"
  hx-target="#loop-details"
  hx-swap="innerHTML"
>
  Refresh
</button>
```

Values of `hx-swap`:
- `innerHTML`: Replaces target content (default)
- `outerHTML`: Replaces the target itself
- `beforebegin`: Inserts before target
- `afterbegin`: Inserts as first child
- `beforeend`: Inserts as last child
- `afterend`: Inserts after target
- `none`: Does not modify DOM (useful for POST with redirect)

### HTMX - Form Submission

Form submission patterns:

```html
<!-- Pattern 1: POST with boost (full page reload if success) -->
<form hx-post="/api/auth/register" hx-boost="true">
  <input type="hidden" name="csrf_token" value="{{ csrf_token }}" />
  <!-- fields -->
  <button type="submit">Submit</button>
</form>

<!-- Pattern 2: POST without swap (handler does manual redirect) -->
<form hx-post="/api/auth/login" hx-swap="none">
  <input type="hidden" name="csrf_token" value="{{ csrf_token }}" />
  <!-- fields -->
  <button type="submit">Login</button>
</form>

<!-- Pattern 3: POST with target (partial update) -->
<form hx-post="/api/loops/{{ id }}/update" hx-target="#loop-status" hx-swap="outerHTML">
  <input type="hidden" name="csrf_token" value="{{ csrf_token }}" />
  <!-- fields -->
  <button type="submit">Update</button>
</form>
```

### HTMX - Loading States

Use `hx-indicator` for loading indicators:

```html
<!-- Loading indicator -->
<div id="loading" class="hidden">
  <svg class="animate-spin h-5 w-5 text-gray-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 0 12h4zm2 5.291A7.962 7.962 0 014 8-8V0C5.373 0 0 0 12h4zm2 5.291A7.962 7.962 0 014 8-8V0C5.373 0 0 0 12h4zm2 5.291A7.962 7.962 0 0 14 8-8V0C5.373 0 0 0 12h4zm2 5.291A7.962 7.962 0 0 14 8-8V0C5.373 0 0 0 12h4zm2 5.291A7.962 7.962 0 0 14 8-8V0C5.373 0 0 0 12h4zm2 5.291A7.962 7.962 0 0 14 8-8V0C5.373 0 0 0 12h4zm2 5.291A7.962 7.962 0 0 14 8-8V0C5.373 0 0 0 12h4z"></path>
  </svg>
</div>

<!-- Button with indicator -->
<button hx-get="/api/loops" hx-indicator="#loading">
  Refresh
</button>
```

### Askama - Conditionals and Loops

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

<!-- Loop with index -->
{% for (index, loop) in loops.iter().enumerate() %}
<div>{{ index + 1 }}. {{ loop.name }}</div>
{% endfor %}
```

### Askama - Filters and Modifiers

```html
<!-- String slicing -->
<p>{{ loop.created_at[..10] }}</p>  <!-- First 10 characters -->

<!-- Conditional formatting -->
<span class="inline-flex rounded-full px-2 text-xs font-semibold leading-5
  {% if loop.status == "running" %} bg-green-100 text-green-800
  {% elif loop.status == "paused" %} bg-yellow-100 text-yellow-800
  {% else %} bg-gray-100 text-gray-800 {% endif %}">
  {{ loop.status }}
</span>
```

### Displaying Validation Errors

```html
<!-- In template struct -->
pub struct RegisterTemplate<'a> {
    pub logged_in: bool,
    pub csrf_token: String,
    pub errors: &'a [String],
}

<!-- In template -->
{% if !errors.is_empty() %}
<div class="rounded-md bg-red-50 p-4">
  <div class="flex">
    <div class="flex-shrink-0">
      <svg class="h-5 w-5 text-red-400" viewBox="0 0 20 20" fill="currentColor">
        <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414L10 11.414l1.293 1.293a1 1 0-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd" />
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

### Pagination

```html
<!-- In template struct -->
pub struct LoopListTemplate {
    pub page: u32,
    pub total_pages: u32,
    pub has_prev: bool,
    pub has_next: bool,
    // ...
}

<!-- In template -->
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

## Anti-patterns

### NEVER DO

**1. Complex JavaScript for simple things**

```html
<!-- ❌ WRONG - Manual JavaScript -->
<button onclick="fetch('/api/loops').then(...)">Refresh</button>

<!-- ✅ CORRECT - HTMX does the fetch -->
<button hx-get="/api/loops">Refresh</button>
```

**2. Forgetting CSRF token in forms**

```html
<!-- ❌ WRONG - No CSRF -->
<form hx-post="/api/loops">
  <input type="text" name="name">
  <button>Submit</button>
</form>

<!-- ✅ CORRECT - With CSRF -->
<form hx-post="/api/loops">
  <input type="hidden" name="csrf_token" value="{{ csrf_token }}">
  <input type="text" name="name">
  <button>Submit</button>
</form>
```

**3. Mixing hx-get/hx-post incorrectly**

```html
<!-- ❌ WRONG - GET for mutation -->
<button hx-get="/api/loops" hx-post="true">
  Create Loop
</button>

<!-- ✅ CORRECT - POST for mutation -->
<button hx-post="/api/loops">
  Create Loop
</button>
```

**4. Not using base.html**

```html
<!-- ❌ WRONG - Duplicated HTML -->
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

<!-- ✅ CORRECT - Extend from base.html -->
{% extends "base.html" %}
{% block content %}
<main>...</main>
{% endblock %}
```

**5. Inconsistent Tailwind classes**

```html
<!-- ❌ WRONG - Mixing styles -->
<button class="btn btn-primary bg-blue-500 rounded">Save</button>

<!-- ✅ CORRECT - Standard Tailwind classes -->
<button class="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md text-white bg-indigo-600 hover:bg-indigo-700">Save</button>
```

**6. Not handling loading states**

```html
<!-- ❌ WRONG - No feedback -->
<button hx-get="/api/loops">Refresh</button>

<!-- ✅ CORRECT - With indicator -->
<div id="loading" class="hidden">Loading...</div>
<button hx-get="/api/loops" hx-indicator="#loading">Refresh</button>
```

**7. Non-responsive elements**

```html
<!-- ❌ WRONG - Breaks on mobile -->
<div class="grid grid-cols-4 gap-4">
  <div>Item 1</div>
  <div>Item 2</div>
  <div>Item 3</div>
  <div>Item 4</div>
</div>

<!-- ✅ CORRECT - Responsive -->
<div class="grid grid-cols-1 gap-y-6 gap-x-4 sm:grid-cols-4">
  <div>Item 1</div>
  <div>Item 2</div>
  <div>Item 3</div>
  <div>Item 4</div>
</div>
```

## Dependencies

### Libraries Used

```toml
# Template engine
askama = "0.15"  # Compile-time template rendering

# Styling (via CDN in base.html)
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

- `src/templates/mod.rs` - Template structs mapping to HTML files
- Handlers in `src/handlers/` - Create and populate template structs
- Models in `ralph-models` - Data displayed in templates

## Pitfalls

### Common Confusions

**1. hx-target vs hx-swap**

```html
<!-- hx-target: WHERE to insert result -->
<button hx-get="/api/data" hx-target="#result">Load</button>
<div id="result">
  <!-- HTML will be inserted HERE -->
</div>

<!-- hx-swap: HOW to insert result -->
<button hx-get="/api/data" hx-target="#result" hx-swap="outerHTML">Load</button>
<div id="result">
  <!-- This element WILL BE REPLACED by the result -->
</div>
```

**2. hx-boost vs hx-get**

```html
<!-- hx-boost: makes links/forms HTMX-aware automatically -->
<a href="/loops" hx-boost="true">
  Loops
</a>
<!-- The click still navigates to /loops, but via HTMX (if supported) -->

<!-- hx-get: forces async request -->
<button hx-get="/loops">
  Load Loops
</button>
<!-- The click does async GET, injects response -->
```

**3. logged_in vs user_id**

```rust
// Template struct has logged_in (bool)
pub struct LoopListTemplate {
    pub logged_in: bool,
    pub csrf_token: String,
}

// Handler populates logged_in
let logged_in = !user_id.is_empty();
let template = LoopListTemplate {
    logged_in,  // true if user_id is not empty
    csrf_token,
    loops,
};
```

**4. Template struct vs Template path**

```rust
// Template path must correspond to file
#[derive(Template)]
#[template(path = "loops/index.html")]  // ← must be "loops/index.html"
pub struct LoopListTemplate { ... }
//                  ↑ corresponds to templates/loops/index.html
```

### Unexpected Behaviors

**1. HTMX boost breaks if response is not HTML**

```rust
// ❌ WRONG - Handler returns JSON in boosted route
pub async fn list_loops() -> Json<ListLoopsResponse> {
    Json(response)
}

// ✅ CORRECT - Handler returns HTML
pub async fn list_loops_page() -> (StatusCode, Html<String>) {
    (StatusCode::OK, Html(html))
}
```

**2. Askama validates templates at compile-time**

```rust
// If template has syntax error, code DOES NOT compile
// Error: template syntax error in templates/loops/new.html at line 42

// If variable doesn't exist, DOES NOT compile
// Error: no field `missing_field` on struct `LoopFormTemplate`
```

**3. hx-swap="none" still makes request**

```html
<!-- Request is made, but DOM is not modified -->
<form hx-post="/api/login" hx-swap="none">
  <!-- Handler must do redirect manually -->
</form>

// Rust handler
pub async fn login(...) -> Redirect {
    // Needs manual redirect
    Redirect::to("/loops")
}
```

**4. Tailwind CDN not for production**

```html
<!-- ❌ DO NOT use in production -->
<script src="https://cdn.tailwindcss.com"></script>

<!-- ✅ In production, use build process -->
<!-- npm install -D tailwindcss -->
<!-- npx tailwindcss init -->
```

## Downlinks (Additional Context)

**To understand better:**
- `/ralph-server/AGENTS.md` - How handlers use templates
- `/ralph-server/src/templates/mod.rs` - Template structs implementation
- `/ralph-server/src/handlers/` - Handlers that create and render templates
- `https://htmx.org/docs` - HTMX official documentation
- `https://tailwindcss.com/docs` - Tailwind CSS official documentation

---

**Last updated:** 2026-01-18
**Version:** 1.0
