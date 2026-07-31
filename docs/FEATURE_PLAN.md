# Loggy Feature Enhancement Plan

## Goal
Transform Loggy from a basic log viewer into a full-featured Docker observability platform with stack-level and container-level controls.

---

## Current State
- ✅ Basic container listing
- ✅ Real-time log streaming
- ✅ Log storage in ClickHouse
- ✅ WebSocket support
- ❌ No stack filtering
- ❌ No container actions
- ❌ No metrics visualization
- ❌ Limited filtering

---

## Target Features

### 1. Stack/Project Management
- [ ] Auto-discover Docker Compose stacks from docker-compose.yml files
- [ ] Group containers by compose project
- [ ] Enable/disable stack monitoring
- [ ] Stack-level health summary

### 2. Container Management
- [ ] List containers with detailed info
- [ ] Filter by: status, name, image, stack
- [ ] Container actions:
  - [ ] Start container
  - [ ] Stop container
  - [ ] Restart container
  - [ ] View container details
  - [ ] View container logs (raw)

### 3. Advanced Filtering
- [ ] Filter by log level (error, warn, info, debug)
- [ ] Search by text/regex
- [ ] Time range selection
- [ ] Container multi-select
- [ ] Export logs (JSON/CSV)

### 4. Metrics Dashboard
- [ ] CPU usage per container
- [ ] Memory usage per container
- [ ] Network I/O
- [ ] Container uptime
- [ ] Historical metrics charts

### 5. Alerts & Patterns
- [ ] Configure error patterns
- [ ] Alert rules (webhook, email)
- [ ] Alert history

---

## Implementation Plan

### Phase 1: UI Enhancement (Foundation)
**Effort**: M (16-24 hours)

- [ ] **1.1** Redesign frontend with sidebar navigation
  - Stacks list (collapsible)
  - Containers list (filterable)
  - Logs panel
  - Metrics panel
- [ ] **1.2** Add stack selector dropdown
- [ ] **1.3** Add container multi-select
- [ ] **1.4** Add filter controls (level, search, time)

### Phase 2: Container Actions
**Effort**: M (16-24 hours)

- [ ] **2.1** Add API endpoints for container control
  - POST /api/containers/:id/start
  - POST /api/containers/:id/stop
  - POST /api/containers/:id/restart
- [ ] **2.2** Add action buttons in UI
- [ ] **2.3** Add confirmation dialogs
- [ ] **2.4** Show container details modal

### Phase 3: Metrics Integration
**Effort**: L (24-32 hours)

- [ ] **3.1** Implement container stats collection
- [ ] **3.2** Store metrics in ClickHouse
- [ ] **3.3** Create metrics API endpoints
- [ ] **3.4** Add metrics visualization (charts)
- [ ] **3.5** Real-time metrics WebSocket

### Phase 4: Advanced Features
**Effort**: L (24-40 hours)

- [ ] **4.1** Alert configuration UI
- [ ] **4.2** Pattern management
- [ ] **4.3** Log export functionality
- [ ] **4.4** Time range selection
- [ ] **4.5** Saved filters/views

---

## Technical Decisions

### Frontend Architecture
- Keep JavaScript (no WASM complexity for now)
- Use vanilla JS with modern patterns
- Add a simple router for SPA navigation

### API Changes
```
GET  /api/stacks              - List all compose stacks
GET  /api/stacks/:id          - Get stack details
POST /api/stacks/:id/enable   - Enable monitoring
POST /api/stacks/:id/disable  - Disable monitoring

GET  /api/containers          - List (supports ?stack=, ?status=, ?search=)
GET  /api/containers/:id      - Get container details
POST /api/containers/:id/start
POST /api/containers/:id/stop
POST /api/containers/:id/restart

GET  /api/metrics             - Get metrics (supports ?container=, ?from=, ?to=)
GET  /api/metrics/:id         - Get specific container metrics

GET  /api/logs                - Query logs (existing + new filters)
```

### Database Schema (ClickHouse)
```sql
-- Metrics table (new)
CREATE TABLE metrics (
    container_id String,
    container_name String,
    timestamp DateTime,
    cpu_percent Float32,
    memory_usage UInt64,
    memory_limit UInt64,
    network_rx UInt64,
    network_tx UInt64,
    block_read UInt64,
    block_write UInt64
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (container_id, timestamp);
```

---

## Priority Order

### Must Have (Phase 1-2)
1. Stack listing and selection
2. Container filtering
3. Container start/stop/restart
4. Log level filtering
5. Text search

### Should Have (Phase 3)
1. Basic metrics display
2. Container details view

### Nice to Have (Phase 4)
1. Advanced charts
2. Alert configuration
3. Log export

---

## Files to Modify

### Backend
- `src/api/mod.rs` - Add new endpoints
- `src/docker/mod.rs` - Add container control methods
- `src/db/mod.rs` - Add metrics storage
- `src/lib.rs` - Add metrics collection task

### Frontend
- `frontend/index.html` - Complete redesign
- Add new API calls
- Add filtering logic

---

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|-------------|
| WASM rendering issues | High | Use pure JS fallback |
| ClickHouse metrics performance | Medium | Use aggregation, limit retention |
| Docker API rate limiting | Medium | Add caching, throttle requests |
| Container action failures | Medium | Show clear error messages |

---

## Success Criteria

1. User can select a stack and see only its containers
2. User can filter logs by level and search text
3. User can start/stop/restart containers from UI
4. User can view CPU/memory metrics for containers
5. All operations complete within 2 seconds
6. UI is responsive and works on desktop browsers
