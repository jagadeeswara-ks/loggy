# Loggy - Implementation Tasks

## Phase 1: Core Engine (Week 1-2)

### M1.1: Rust Project Setup
- [ ] Initialize Rust project with Cargo
- [ ] Add dependencies: axum, bollard, clickhouse, serde, tokio, tracing
- [ ] Set up logging with tracing
- [ ] Create basic project structure
- [ ] Verify build compiles

### M1.2: ClickHouse Integration
- [ ] Set up ClickHouse client
- [ ] Create database schema (logs, metrics tables)
- [ ] Implement log insertion
- [ ] Implement metrics insertion
- [ ] Test with sample data
- [ ] Add connection pooling

### M1.3: Docker Discovery Engine
- [ ] Scan directories for docker-compose.yml
- [ ] Parse compose files
- [ ] Extract container names and project names
- [ ] Watch for file changes (notify crate)
- [ ] Handle nested directories

### M1.4: Basic Log Streaming
- [ ] Connect to Docker daemon via Bollard
- [ ] Stream logs from containers
- [ ] Parse JSON logs
- [ ] Extract timestamps
- [ ] Extract log levels
- [ ] Insert into ClickHouse

### M1.5: Simple WebSocket Endpoint
- [ ] Set up Axum with WebSocket
- [ ] Broadcast log events to clients
- [ ] Handle client connections/disconnections

---

## Phase 2: Query Engine (Week 3-4)

### M2.1: Full-Text Search
- [ ] Implement text search in ClickHouse
- [ ] Add search endpoint
- [ ] Optimize for large result sets
- [ ] Add pagination

### M2.2: Filter System
- [ ] Filter by container name
- [ ] Filter by log level
- [ ] Filter by time range
- [ ] Filter by compose project
- [ ] Combine multiple filters

### M2.3: Pattern Detection
- [ ] Analyze log content for patterns
- [ ] Detect stack traces
- [ ] Detect exceptions/errors
- [ ] Detect common formats
- [ ] Store patterns in database

### M2.4: Auto-Filter Generation
- [ ] Generate filters from patterns
- [ ] Categorize filters (error, warning, etc.)
- [ ] Make filters clickable
- [ ] Cache generated filters

---

## Phase 3: WASM Frontend (Week 5-6)

### M3.1: Yew Setup
- [ ] Initialize Yew project
- [ ] Set up WASM build
- [ ] Configure WebSocket client
- [ ] Set up state management

### M3.2: Dashboard Page
- [ ] Show discovered stacks
- [ ] Show container status (running/stopped)
- [ ] Show quick stats
- [ ] Add enable/disable controls

### M3.3: Log Viewer
- [ ] Virtualized list for performance
- [ ] Auto-scroll (toggleable)
- [ ] Search within logs
- [ ] Apply filters
- [ ] Color coding by level

### M3.4: Metrics Charts
- [ ] CPU usage chart
- [ ] Memory usage chart
- [ ] Time range selector
- [ ] Auto-refresh

---

## Phase 4: Polish (Week 7-8)

### M4.1: Metrics Collection
- [ ] Poll Docker stats API
- [ ] Store metrics in ClickHouse
- [ ] Aggregate metrics (avg, max, min)
- [ ] Historical chart data

### M4.2: Alert System
- [ ] Pattern-based alerts
- [ ] Threshold-based alerts
- [ ] In-app notification UI
- [ ] Alert history

### M4.3: Performance Optimization
- [ ] Profile memory usage
- [ ] Optimize ClickHouse queries
- [ ] Reduce WASM bundle size
- [ ] Add caching where needed

### M4.4: Configuration System
- [ ] Load config from file
- [ ] Environment variable support
- [ ] Web UI for settings
- [ ] Persist user preferences

---

## Task Dependencies

```
M1.1 → M1.2 → M1.3 → M1.4 → M1.5
                         ↓
M2.1 ← M2.2 ← M2.3 ← M2.4
  ↓
M3.1 ← M3.2 ← M3.3 ← M3.4
  ↓
M4.1 ← M4.2 ← M4.3 ← M4.4
```

---

## File Structure

```
loggy/
├── src/
│   ├── main.rs              # Entry point
│   ├── lib.rs               # Library root
│   ├── config.rs            # Configuration
│   ├── db/
│   │   ├── mod.rs
│   │   ├── clickhouse.rs    # ClickHouse client
│   │   └── models.rs       # Data models
│   ├── docker/
│   │   ├── mod.rs
│   │   ├── discovery.rs    # Compose file discovery
│   │   ├── logs.rs         # Log streaming
│   │   └── metrics.rs      # Metrics collection
│   ├── api/
│   │   ├── mod.rs
│   │   ├── routes.rs       # REST routes
│   │   └── websocket.rs    # WebSocket handler
│   ├── patterns/
│   │   ├── mod.rs
│   │   └── detector.rs     # Pattern detection
│   └── alerts/
│       ├── mod.rs
│       └── manager.rs       # Alert management
├── frontend/
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── app.rs          # Main app component
│   │   ├── pages/
│   │   │   ├── dashboard.rs
│   │   │   ├── logs.rs
│   │   │   ├── metrics.rs
│   │   │   └── settings.rs
│   │   ├── components/
│   │   │   ├── log_viewer.rs
│   │   │   ├── metric_chart.rs
│   │   │   └── stack_card.rs
│   │   └── api.rs           # WebSocket & API client
│   ├── index.html
│   ├── Cargo.toml
│   └── webpack.config.js
├── config/
│   └── default.toml
├── docker-compose.yml       # For running ClickHouse
├── Dockerfile              # Build binary
├── Dockerfile.wasm          # Build frontend
├── PRD.md
├── README.md
└── Cargo.toml
```

---

## Key Technical Decisions

| Decision | Rationale |
|----------|-----------|
| ClickHouse over SQLite | 100x faster for log queries |
| Yew over React | Same language, smaller bundle |
| Bollard over http | Best async Rust Docker client |
| Axum over Actix | Simpler, well-maintained |
| Virtualized logs | Handle millions of entries |

---

## Testing Strategy

### Unit Tests
- Pattern detection logic
- Log parsing
- Config loading

### Integration Tests
- ClickHouse read/write
- Docker API integration
- WebSocket communication

### Manual Tests
- Full user flows
- Performance under load
- Edge cases

---

*Tasks updated: 2026-03-03*
