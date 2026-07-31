# Loggy - Product Requirements Document

> Docker Observability Platform
> Version: 1.0.0
> Date: 2026-03-03

---

## 1. Executive Summary

### 1.1 Product Vision

**Loggy** is a high-performance Docker observability platform designed for developers managing multiple Docker Compose stacks on a single VPS. It provides real-time log aggregation, intelligent filtering, metrics visualization, and debugging capabilities with minimal resource overhead.

### 1.2 Problem Statement

- Developers running 5+ Docker Compose stacks lack unified observability
- Manual log tailing is tedious and inefficient
- No historical log search or pattern detection
- Existing tools (ELK, Loki) are too resource-heavy for VPS environments

### 1.3 Target Users

- Full-stack developers managing personal VPS
- Small teams running containerized applications
- DevOps engineers needing lightweight log aggregation

---

## 2. Technical Architecture

### 2.1 Technology Stack

| Component | Technology | Rationale |
|-----------|------------|------------|
| Backend | Rust + Axum | Zero GC, fastest performance |
| Database | ClickHouse | Built for log analytics |
| Frontend | Yew (Rust WASM) | Same language, minimal bundle |
| Real-time | WebSockets | <10ms latency |
| Docker API | Bollard | Best async Rust client |

### 2.2 System Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                           Loggy System                                │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌───────────────┐    ┌───────────────┐    ┌───────────────┐    │
│  │  Discovery    │    │   Log         │    │   Metrics     │    │
│  │   Engine      │    │  Aggregator   │    │   Collector   │    │
│  │               │    │               │    │               │    │
│  │ - Scan dirs   │    │ - Stream from │    │ - CPU stats   │    │
│  │ - Find compose│    │   Docker      │    │ - Memory      │    │
│  │ - Watch files │    │ - Parse JSON  │    │ - Network     │    │
│  │ - Detect new  │    │ - Timestamp   │    │ - Disk I/O    │    │
│  └───────┬───────┘    └───────┬───────┘    └───────┬───────┘    │
│          │                    │                    │             │
│          └────────────────────┴────────────────────┘             │
│                               │                                    │
│                    ┌──────────▼──────────┐                      │
│                    │    ClickHouse       │                      │
│                    │   (Time-series)     │                      │
│                    │                     │                      │
│                    │ - logs table        │                      │
│                    │ - metrics table     │                      │
│                    │ - patterns table    │                      │
│                    └──────────┬──────────┘                      │
│                               │                                    │
│          ┌────────────────────┼────────────────────┐           │
│          │                    │                    │           │
│  ┌──────▼──────┐    ┌────────▼────────┐    ┌──────▼──────┐  │
│  │   Query     │    │    WebSocket    │    │   Alert     │  │
│  │   Engine    │    │    Publisher    │    │   Manager   │  │
│  │             │    │                 │    │             │  │
│  │ - Full-text │    │ - Real-time     │    │ - Pattern   │  │
│  │ - Filters   │    │   push         │    │ - Threshold │  │
│  │ - SQL       │    │ - Broadcast    │    │ - Notify    │  │
│  └──────┬───────┘    └────────┬────────┘    └──────┬───────┘  │
│         │                      │                    │           │
└─────────┼──────────────────────┼────────────────────┼──────────┘
          │                      │                    │
          │              ┌──────▼──────┐            │
          │              │   WASM      │            │
          │              │   Frontend  │            │
          │              │             │            │
          │              │ - Dashboard │            │
          │              │ - Logs view │            │
          │              │ - Metrics   │            │
          │              │ - Filters   │            │
          │              └─────────────┘            │
          │                                           │
          └───────────────────────────────────────────┘
                           User Browser
```

### 2.3 Data Models

#### Logs Table
```sql
CREATE TABLE logs (
    id UInt64,
    timestamp DateTime64(3),
    container_id String,
    container_name String,
    compose_project String,
    compose_file String,
    message String,
    level Enum8('DEBUG'=1, 'INFO'=2, 'WARN'=3, 'ERROR'=4, 'FATAL'=5),
    source Enum8('STDOUT'=1, 'STDERR'=2),
    metadata JSON
) ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(timestamp)
ORDER BY (container_id, timestamp)
TTL timestamp + INTERVAL 7 DAY;
```

#### Metrics Table
```sql
CREATE TABLE metrics (
    id UInt64,
    timestamp DateTime64(3),
    container_id String,
    container_name String,
    cpu_percent Float32,
    memory_percent Float32,
    memory_usage UInt64,
    network_rx UInt64,
    network_tx UInt64,
    block_read UInt64,
    block_write UInt64
) ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(timestamp)
ORDER BY (container_id, timestamp)
TTL timestamp + INTERVAL 7 DAY;
```

---

## 3. Functional Requirements

### 3.1 Auto-Discovery

| ID | Requirement | Priority |
|----|-------------|----------|
| F1.1 | Scan specified directories for docker-compose.yml files | Must |
| F1.2 | Auto-detect container names from compose files | Must |
| F1.3 | Watch for new/deleted compose files | Must |
| F1.4 | Support nested directory structures | Should |
| F1.5 | Remember selected stacks between sessions | Should |

### 3.2 Log Aggregation

| ID | Requirement | Priority |
|----|-------------|----------|
| F2.1 | Stream logs from all containers in selected stacks | Must |
| F2.2 | Parse JSON-formatted logs | Must |
| F2.3 | Extract log level from log content | Must |
| F2.4 | Handle multi-line logs (stack traces) | Must |
| F2.5 | Timestamp normalization | Must |
| F2.6 | Real-time streaming via WebSocket | Must |

### 3.3 Metrics Collection

| ID | Requirement | Priority |
|----|-------------|----------|
| F3.1 | Collect CPU usage per container | Must |
| F3.2 | Collect memory usage per container | Must |
| F3.3 | Collect network I/O per container | Must |
| F3.4 | Collect disk I/O per container | Should |
| F3.5 | Configurable collection interval | Should |

### 3.4 Query & Search

| ID | Requirement | Priority |
|----|-------------|----------|
| F4.1 | Full-text search across all logs | Must |
| F4.2 | Filter by container name | Must |
| F4.3 | Filter by log level | Must |
| F4.4 | Filter by time range | Must |
| F4.5 | Filter by compose project | Must |
| F4.6 | Regex pattern matching | Should |
| F4.7 | Saved queries/filters | Should |

### 3.5 Smart Filtering

| ID | Requirement | Priority |
|----|-------------|----------|
| F5.1 | Auto-detect log patterns | Must |
| F5.2 | Generate filters from detected patterns | Must |
| F5.3 | Common patterns: stack traces, exceptions, errors | Must |
| F5.4 | User-defined pattern rules | Should |

### 3.6 Dashboard

| ID | Requirement | Priority |
|----|-------------|----------|
| F6.1 | Overview of all monitored stacks | Must |
| F6.2 | Container health status | Must |
| F6.3 | Real-time log viewer | Must |
| F6.4 | Metrics charts (CPU, Memory) | Must |
| F6.5 | Auto-refreshing dashboard | Must |

### 3.7 Alerts

| ID | Requirement | Priority |
|----|-------------|----------|
| F7.1 | Pattern-based alerts (error keywords) | Should |
| F7.2 | Threshold alerts (high CPU/memory) | Should |
| F7.3 | Container restart detection | Should |
| F7.4 | In-app notifications | Should |

---

## 4. User Interface

### 4.1 Pages

| Page | Description |
|------|-------------|
| Dashboard | Overview of all stacks, health status |
| Logs | Real-time log viewer with filters |
| Metrics | Historical metrics charts |
| Settings | Configuration, stack selection |

### 4.2 User Flows

#### Initial Setup
```
1. User visits Loggy
2. System scans for docker-compose.yml files
3. User sees list of discovered stacks
4. User selects stacks to monitor
5. User clicks "Start Monitoring"
6. Redirected to Dashboard
```

#### Log Viewing
```
1. User clicks on container or stack
2. Real-time logs appear in viewer
3. User applies filters (level, search)
4. User can pause/resume stream
5. User can clear or export logs
```

### 4.3 UI Components

| Component | Description |
|-----------|-------------|
| StackCard | Summary card for each compose stack |
| LogViewer | Virtualized log list with streaming |
| MetricChart | Time-series charts |
| FilterBar | Search and filter controls |
| StatusBadge | Container health indicator |

---

## 5. Non-Functional Requirements

### 5.1 Performance

| Metric | Target |
|--------|--------|
| Log ingestion | 100,000+ logs/second |
| Query latency | <50ms for full-text search |
| WebSocket latency | <10ms |
| Memory usage | <100MB baseline |
| Binary size | <20MB |

### 5.2 Scalability

| Scenario | Support |
|----------|---------|
| 5 stacks (current) | ✅ Full support |
| 20 stacks | ✅ Should work |
| 50 containers | ✅ With optimization |
| 1M logs/day | ✅ Compressed storage |

### 5.3 Reliability

| Requirement | Target |
|-------------|--------|
| Uptime | 99.9% |
| Data retention | 7 days default (configurable) |
| Crash recovery | Auto-restart, no data loss |

### 5.4 Security

| Feature | Implementation |
|---------|----------------|
| Local only | No remote access by default |
| No auth | Single-user local tool |
| HTTPS | Optional for remote access |

---

## 6. API Design

### 6.1 REST Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | /api/stacks | List discovered compose stacks |
| GET | /api/stacks/:id | Get stack details |
| POST | /api/stacks/:id/enable | Enable monitoring |
| POST | /api/stacks/:id/disable | Disable monitoring |
| GET | /api/containers | List running containers |
| GET | /api/logs | Query logs with filters |
| GET | /api/metrics | Query metrics |
| GET | /api/patterns | Get detected patterns |

### 6.2 WebSocket Events

| Event | Direction | Payload |
|-------|-----------|---------|
| log:new | Server→Client | Log entry |
| log:batch | Server→Client | Batch of logs |
| metrics:update | Server→Client | Metrics snapshot |
| alert:triggered | Server→Client | Alert notification |

---

## 7. Configuration

### 7.1 Config File (config.toml)

```toml
[server]
host = "0.0.0.0"
port = 8080

[discovery]
paths = ["/home/user/projects", "/opt/stacks"]
exclude = ["**/node_modules/**", "**/dist/**"]

[storage]
retention_days = 7
compression = true

[clickhouse]
host = "localhost"
port = 9000
database = "loggy"

[docker]
socket_path = "/var/run/docker.sock"
```

### 7.2 Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| LOGGY_PORT | 8080 | Server port |
| LOGGY_HOST | 0.0.0.0 | Server host |
| LOGGY_DATA_DIR | ./data | Data directory |

---

## 8. Milestones

### Phase 1: Core Engine (Week 1-2)

| Task | Description |
|------|-------------|
| M1.1 | Rust project setup with Axum |
| M1.2 | ClickHouse integration |
| M1.3 | Docker discovery engine |
| M1.4 | Basic log streaming |
| M1.5 | Simple WebSocket endpoint |

### Phase 2: Query Engine (Week 3-4)

| Task | Description |
|------|-------------|
| M2.1 | Full-text search implementation |
| M2.2 | Filter system |
| M2.3 | Pattern detection |
| M2.4 | Auto-filter generation |

### Phase 3: WASM Frontend (Week 5-6)

| Task | Description |
|------|-------------|
| M3.1 | Yew setup |
| M3.2 | Dashboard page |
| M3.3 | Log viewer with virtualization |
| M3.4 | Metrics charts |

### Phase 4: Polish (Week 7-8)

| Task | Description |
|------|-------------|
| M4.1 | Metrics collection |
| M4.2 | Alert system |
| M4.3 | Performance optimization |
| M4.4 | Configuration system |

---

## 9. Future Considerations

### 9.1 Post-v1.0

- [ ] Remote access with authentication
- [ ] Multi-node support
- [ ] Cloud export (S3, GCS)
- [ ] Plugin system
- [ ] Prometheus export
- [ ] Grafana integration

### 9.2 Potential Features

- [ ] Log anomaly detection (ML)
- [ ] Distributed tracing
- [ ] Custom dashboards
- [ ] Team sharing

---

## 10. Glossary

| Term | Definition |
|------|------------|
| Compose Stack | A directory with docker-compose.yml |
| Container | Single Docker container |
| Log Level | DEBUG, INFO, WARN, ERROR, FATAL |
| Pattern | Repeated log structure |
| Retention | How long logs are stored |

---

*Document Version: 1.0*
*Created: 2026-03-03*
*Last Updated: 2026-03-03*
