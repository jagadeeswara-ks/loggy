# 🤖 Loggy Phase 1 - Evolved Implementation

## Context & Analysis

### What We've Learned
- The app runs but UI needs improvement
- WASM had rendering issues, switched to vanilla JS
- Logs are streaming but filtering needs work
- Stack discovery needs to be implemented

### Different Approach
Instead of just adding endpoints, let's think about the **data architecture first**:

```
┌─────────────────────────────────────────────────────────────┐
│                     FRONTEND (HTML/JS)                      │
│  ┌──────────┐  ┌──────────────┐  ┌─────────────────────┐  │
│  │ Stack    │  │ Container    │  │ Log Viewer          │  │
│  │ Selector │  │ List         │  │ + Filters           │  │
│  └──────────┘  └──────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    API LAYER (Rust/Axum)                    │
│  ┌──────────┐  ┌──────────────┐  ┌─────────────────────┐  │
│  │ /stacks  │  │ /containers  │  │ /logs              │  │
│  │ (filter) │  │ (filter)     │  │ (filter+search)    │  │
│  └──────────┘  └──────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│               BUSINESS LOGIC (Rust)                         │
│  ┌──────────┐  ┌──────────────┐  ┌─────────────────────┐  │
│  │ Stack    │  │ Container    │  │ Log Query          │  │
│  │ Discovery│  │ Manager      │  │ Builder            │  │
│  └──────────┘  └──────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                   DATA LAYER                               │
│  ┌─────────────────────┐  ┌────────────────────────────┐  │
│  │ ClickHouse          │  │ Docker API (bollard)      │  │
│  │ (logs + metrics)   │  │ (container info)          │  │
│  └─────────────────────┘  └────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## Evolved Implementation Command

```
You are building a Docker observability platform. Implement Phase 1 with a clean, maintainable architecture.

## Project Location
/home/kjr/loggy - Rust + Axum backend, vanilla JS frontend

## Current Status
- Running at http://localhost:8080
- Basic container listing works
- Real-time logs via WebSocket work
- Logs stored in ClickHouse

## Phase 1 Goal: Stack Selector, Container List, Log Filtering

### Step 1: Define Clean Data Structures
Create clear response types for each resource:

```rust
// Stack response
#[derive(Serialize)]
struct StackResponse {
    name: String,           // compose project name
    container_count: usize,
    running_count: usize,
    stopped_count: usize,
}

// Container response (enhanced)
#[derive(Serialize)]  
struct ContainerResponse {
    id: String,
    name: String,
    image: String,
    status: String,         // "running", "stopped", etc.
    created: i64,
    stack: Option<String>,  // compose project
}
```

### Step 2: Implement API Endpoints

#### GET /api/stacks
- Query Docker for containers with "com.docker.compose.project" label
- Group by project name
- Return: [{name, container_count, running_count, stopped_count}]

#### GET /api/containers (enhanced)
Add query parameters:
- stack: filter by compose project
- status: "running" | "stopped" | "all"  
- search: text search in container name
- Example: /api/containers?stack=myapp&status=running&search=web

#### GET /api/logs (enhanced)
Add query parameters:
- stack: filter by stack
- container: specific container ID
- level: "error" | "warn" | "info" | "debug" | "all"
- search: text search in message
- limit: max results (default 100)
- Example: /api/logs?stack=myapp&level=error&search=database&limit=50

### Step 3: Frontend Implementation

Create a clean single-page layout:

```html
<!-- Layout -->
<div class="app">
  <nav class="sidebar">
    <h2>Stacks</h2>
    <select id="stack-selector">
      <option value="">All Stacks</option>
      <!-- Populated via JS -->
    </select>
    
    <h2>Containers</h2>
    <input type="text" id="container-search" placeholder="Search containers...">
    <div id="container-list">
      <!-- Populated via JS -->
    </div>
  </nav>
  
  <main class="content">
    <div class="log-filters">
      <button class="filter-btn active" data-level="all">All</button>
      <button class="filter-btn" data-level="error">Error</button>
      <button class="filter-btn" data-level="warn">Warn</button>
      <button class="filter-btn" data-level="info">Info</button>
      <input type="text" id="log-search" placeholder="Search logs...">
    </div>
    <div id="log-container"></div>
  </main>
</div>
```

### Step 4: JavaScript Logic

```javascript
// State
let currentStack = '';
let currentLevel = 'all';
let currentLogSearch = '';

// Fetch stacks on load
fetch('/api/stacks').then(r => r.json()).then(stacks => {
  // populate dropdown
});

// Fetch containers when stack changes
function fetchContainers() {
  const search = document.getElementById('container-search').value;
  let url = '/api/containers?';
  if (currentStack) url += `stack=${currentStack}&`;
  if (search) url += `search=${search}&`;
  fetch(url).then(r => r.json()).then(renderContainers);
}

// Fetch logs when filters change
function fetchLogs() {
  let url = '/api/logs?';
  if (currentStack) url += `stack=${currentStack}&`;
  if (currentLevel !== 'all') url += `level=${currentLevel}&`;
  if (currentLogSearch) url += `search=${currentLogSearch}&`;
  fetch(url).then(r => r.json()).then(renderLogs);
}
```

## Implementation Steps

1. **Backend - Add Stack Discovery**
   - File: src/api/mod.rs
   - Add route: .route("/api/stacks", get(get_stacks))
   - Implement: query containers, group by label

2. **Backend - Enhance Container Endpoint**
   - Add Query params: stack, status, search
   - Filter containers accordingly

3. **Backend - Enhance Logs Endpoint**
   - Add Query params: stack, level, search
   - Filter in ClickHouse query

4. **Frontend - Complete Rewrite**
   - File: frontend/index.html
   - Implement sidebar + content layout
   - Add all JavaScript for filtering
   - Style with existing dark theme

5. **Test & Verify**
   ```bash
   curl http://localhost:8080/api/stacks
   curl "http://localhost:8080/api/containers?stack=loggy"
   curl "http://localhost:8080/api/logs?level=error"
   ```

## Important Notes
- Keep it SIMPLE - vanilla JS, no frameworks
- Reuse existing dark theme CSS
- Maintain real-time WebSocket logs
- NO container management - observability only

## Success = Working Code
After implementation:
- /api/stacks returns stack list
- UI shows stack dropdown → container list updates
- Log filters work (level buttons + search)
- All existing features still work
```

