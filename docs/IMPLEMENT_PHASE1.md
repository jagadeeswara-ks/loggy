# Phase 1 Implementation Command

Use this command to activate an AI coding assistant to implement Phase 1 of the Loggy Observability Platform:

---

## Full Command

```
Read the observability plan at /home/kjr/loggy/OBSERVABILITY_PLAN.md and implement Phase 1.

Context:
- This is a Rust + Axum backend with a JavaScript frontend
- The project is at /home/kjr/loggy
- Currently running at http://localhost:8080
- Uses ClickHouse for storage
- Docker Compose stack is running

Phase 1 Requirements:

1. STACK SELECTOR:
   - Add a dropdown in the UI to select Docker Compose stacks/projects
   - API: GET /api/stacks - returns list of discovered stacks
   - Each stack should show: name, container count, status summary
   - Default: "All Stacks" option

2. CONTAINER LIST:
   - Show containers in a list with: name, image, status (running/stopped), uptime
   - Filter by: stack (from selector), status (running/stopped/all), search text
   - API: GET /api/containers?stack=<name>&status=<status>&search=<text>
   - Show status badges (green for running, red for stopped)

3. LOG FILTERING:
   - Add filter buttons for log levels: ALL, ERROR, WARN, INFO, DEBUG
   - Add search input for text filtering
   - Add container multi-select to filter which containers' logs to show
   - API: GET /api/logs?level=<level>&search=<text>&containers=<id1,id2>&limit=100

Implementation Steps:
1. First, check current API endpoints in src/api/mod.rs
2. Add /api/stacks endpoint to list compose projects
3. Update /api/containers to support stack and search filters
4. Update /api/logs to support level and container filters
5. Redesign frontend/index.html with sidebar + main content layout
6. Add JavaScript for stack selection, filtering, and API calls
7. Rebuild: cargo build --release
8. Test: curl localhost:8080/api/stacks

Technical Notes:
- Use simple vanilla JavaScript (no frameworks)
- Keep existing CSS styling (dark theme)
- Container status is in container.status field (contains "Up" when running)
- Use broadcast::Sender for real-time log streaming
- All API responses should be JSON

Verify after implementation:
- GET /api/stacks returns stack list
- GET /api/containers?stack=loggy filters correctly
- GET /api/logs?level=error filters correctly
- Frontend shows stack dropdown, container list, and log filters
```

