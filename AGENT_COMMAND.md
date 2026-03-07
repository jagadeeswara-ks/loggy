# 🤖 AI Agent Command for Loggy Phase 1

## Copy and Use This Command

---

```
You are a senior Rust/JavaScript developer. Implement Phase 1 of the Loggy Observability Platform.

## Project Context
- Location: /home/kjr/loggy
- Running at: http://localhost:8080
- Tech Stack: Rust (Axum) + JavaScript + ClickHouse + Docker
- Currently working: Container list, real-time logs, log storage

## Read This Plan First
Read /home/kjr/loggy/OBSERVABILITY_PLAN.md to understand the full scope.

## Phase 1 Implementation (Priority Features)

### 1. Stack Selector (P0)
- Add API endpoint: GET /api/stacks
  - Returns list of Docker Compose projects found
  - Each stack: { name, containerCount, runningCount, errorCount }
- Add dropdown in UI to select stack
- "All Stacks" option as default

### 2. Container List (P0)
- Improve container display with status badges
- Add filters: stack, status (running/stopped), search by name
- API: GET /api/containers?stack=<name>&status=<status>&search=<text>

### 3. Log Filtering (P0)
- Add level filter buttons: ALL | ERROR | WARN | INFO | DEBUG
- Add search input for text filtering  
- API: GET /api/logs?level=<level>&search=<text>&container=<id>

## Implementation Steps

1. Check existing API in src/api/mod.rs
2. Add /api/stacks endpoint (use docker labels to discover compose projects)
3. Update /api/containers with new query parameters
4. Update /api/logs with level and container filters
5. Update frontend/index.html:
   - Add sidebar with stack selector
   - Add container list with filters
   - Add log filter controls
   - Use vanilla JavaScript (no frameworks)
   - Keep dark theme styling

6. Build and test:
   cd /home/kjr/loggy && cargo build --release
   curl http://localhost:8080/api/stacks
   curl "http://localhost:8080/api/containers?stack=loggy"
   curl "http://localhost:8080/api/logs?level=error"

## Important Constraints
- NO container management (no start/stop/restart)
- Pure observability only
- Use existing dark theme CSS
- Keep real-time WebSocket logs working

## Success Criteria
- [ ] /api/stacks returns discovered stacks
- [ ] /api/containers accepts stack, status, search filters
- [ ] /api/logs accepts level and search filters  
- [ ] UI shows stack dropdown selector
- [ ] UI shows container list with filters
- [ ] UI shows log level filter buttons
- [ ] All existing functionality still works
```

