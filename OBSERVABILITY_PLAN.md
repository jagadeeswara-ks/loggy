# Loggy Observability Platform - Feature Plan

## Goal
Transform Loggy into a **pure observability tool** for Docker Compose stacks - monitoring, metrics, logs, and alerting. NO container management/actions.

---

## What We Want (Observability)
- ✅ View stack/project level overview
- ✅ View container level metrics
- ✅ Aggregate and filter logs
- ✅ Detect patterns and anomalies
- ✅ Alert on conditions
- ❌ NO container start/stop/restart
- ❌ NO container management

---

## Target Features

### 1. Stack/Project Level Observability
- [ ] Auto-discover Docker Compose stacks
- [ ] Stack health summary (total containers, running, errors)
- [ ] Stack-level log aggregation
- [ ] Stack-level metrics aggregation

### 2. Container Level Observability
- [ ] Container list with status indicators
- [ ] Container metrics (CPU, Memory, Network, Disk)
- [ ] Container uptime
- [ ] Container resource limits
- [ ] Per-container log stream

### 3. Log Aggregation & Filtering
- [ ] Unified log view across all containers/stacks
- [ ] Filter by: stack, container, level, time range
- [ ] Full-text search in logs
- [ ] Log pattern detection (errors, warnings, stack traces)
- [ ] Export logs (JSON/CSV)

### 4. Metrics Dashboard
- [ ] Real-time CPU usage per container
- [ ] Real-time Memory usage per container
- [ ] Network I/O per container
- [ ] Historical metrics (time-series charts)
- [ ] Container resource comparison

### 5. Alerts & Pattern Detection
- [ ] Configurable error patterns
- [ ] Anomaly detection
- [ ] Alert history
- [ ] Alert notifications (webhook)
- [ ] Alert status dashboard

---

## Implementation Plan

### Phase 1: Foundation UI
- [ ] **1.1** Sidebar navigation (Stacks → Containers → Logs → Metrics)
- [ ] **1.2** Stack selector dropdown
- [ ] **1.3** Container list with status badges
- [ ] **1.4** Log filter controls (level, search, time)

### Phase 2: Metrics Integration
- [ ] **2.1** Container stats collection (CPU, Memory, Network)
- [ ] **2.2** Metrics storage in ClickHouse
- [ ] **2.3** Real-time metrics display
- [ ] **2.4** Metrics visualization (simple charts)

### Phase 3: Advanced Observability
- [ ] **3.1** Time range selector
- [ ] **3.2** Log export functionality
- [ ] **3.3** Pattern detection improvements
- [ ] **3.4** Alert configuration UI
- [ ] **3.5** Dashboard overview

---

## What's NOT Included (Management)
- ❌ Start container
- ❌ Stop container
- ❌ Restart container
- ❌ Container configuration changes
- ❌ Deploy/update containers

---

## API Design (Observability Only)

```bash
# Stacks
GET  /api/stacks                          # List all stacks
GET  /api/stacks/:id                      # Stack details + summary

# Containers (Read-only)
GET  /api/containers                      # List (filter: ?stack=, ?status=, ?search=)
GET  /api/containers/:id                  # Container details + current metrics
GET  /api/containers/:id/metrics          # Container historical metrics

# Logs
GET  /api/logs                            # Query (filter: ?stack=, ?container=, ?level=, ?search=, ?from=, ?to=)
GET  /api/patterns                        # Detected patterns

# Metrics
GET  /api/metrics                         # Aggregate metrics (filter: ?stack=, ?container=, ?from=, ?to=)
GET  /api/metrics/summary                 # Quick summary for dashboard
```

---

## Database Schema (ClickHouse)

```sql
-- Existing: logs table (already exists)

-- New: metrics table
CREATE TABLE metrics (
    container_id String,
    container_name String,
    compose_project String,
    timestamp DateTime,
    cpu_percent Float32,
    memory_usage UInt64,
    memory_limit UInt64,
    memory_percent Float32,
    network_rx UInt64,
    network_tx UInt64,
    block_read UInt64,
    block_write UInt64,
    pids UInt32
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(timestamp)
ORDER BY (container_id, timestamp);
```

---

## Priority Implementation

| Priority | Feature | Description |
|----------|---------|-------------|
| P0 | Stack Selector | Filter all views by stack |
| P0 | Container List | Show containers with status |
| P0 | Log Filtering | Filter by level, search, container |
| P1 | Metrics Display | Show CPU/Memory per container |
| P1 | Real-time Updates | WebSocket for live data |
| P2 | Historical Charts | Time-series visualization |
| P2 | Alert Dashboard | View detected alerts |

---

## Files to Modify

### Backend
- `src/api/mod.rs` - Add metrics endpoints, improve filtering
- `src/docker/mod.rs` - Enhance container stats collection
- `src/db/mod.rs` - Add metrics storage queries
- `src/lib.rs` - Add metrics collection task

### Frontend
- `frontend/index.html` - Complete observability UI redesign

---

## Success Criteria

1. ✅ User can select a stack and see its containers
2. ✅ User can filter logs by level, text, container
3. ✅ User can view CPU/Memory metrics per container
4. ✅ All data updates in real-time via WebSocket
5. ✅ No container management actions available
