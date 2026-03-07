# Loggy - Docker Observability Platform

## 📋 Project Planning Document

---

## 1. Problem Understanding

### Current Pain Points
- Multiple Docker Compose stacks running on VPS
- No unified logging solution
- Difficult to debug issues across stacks
- Need observability for present and future projects
- Manual log tailing is tedious
- No historical log data
- No easy way to filter/search logs
- No metrics visibility

### User Profile
- **You**: Full-stack developer
- **Works with**: Multiple tech stacks (Python, Node, Go, Rust, etc.)
- **Environment**: VPS with 5+ Docker Compose stacks
- **Needs**: Logs, metrics, debugging, observability

### Success Criteria
1. Auto-detect all Docker Compose stacks on a VPS
2. Allow selection of stacks to monitor
3. Beautiful dashboard with real-time logs
4. Smart filters auto-generated from log patterns
5. Historical log storage and search
6. Works out-of-the-box with zero config

---

## 2. User Stories

### Epic 1: Auto-Discovery
```
As a developer
I want Loggy to automatically find all Docker Compose stacks
So that I don't have to configure anything manually
```

### Epic 2: Log Aggregation
```
As a developer  
I want to see logs from all containers in one place
So that I can debug issues quickly
```

### Epic 3: Smart Filtering
```
As a developer
I want intelligent filters based on actual log patterns
So that I can find issues faster
```

### Epic 4: Historical Access
```
As a developer
I want to search through historical logs
So that I can investigate past issues
```

### Epic 5: Real-time Monitoring
```
As a developer
I want real-time log streaming
So that I can see issues as they happen
```

### Epic 6: Metrics & Health
```
As a developer
I want container metrics (CPU, Memory, Network)
So that I can correlate logs with resource usage
```

---

## 3. Technology Research

### Option A: Full-Stack Custom Solution

**Frontend**
| Technology | Pros | Cons |
|------------|------|------|
| React + Vite | Fast, modern | Setup time |
| Vue 3 | Simple, fast | Less popular |
| Svelte | Very fast, small | Smaller ecosystem |
| HTMX | Simple, server-rendered | Limited interactivity |

**Backend**
| Technology | Pros | Cons |
|------------|------|------|
| Go | Fast, single binary, Docker-friendly | Learning curve |
| Rust | Fastest, memory safe | Steeper learning curve |
| Node.js | JavaScript everywhere | More resource usage |
| Python | Quick to build | Slower |

**Database**
| Technology | Pros | Cons |
|------------|------|------|
| SQLite | Simple, file-based | Not for high volume |
| PostgreSQL | Robust, JSON support | Requires setup |
| TimescaleDB | Time-series optimized | Complex |
| ClickHouse | Analytics powerhouse | Heavy |

**Log Storage**
| Technology | Pros | Cons |
|------------|------|------|
| Loki (Grafana) | Designed for logs | Heavy |
| Elasticsearch | Powerful search | Memory hungry |
| SQLite + compression | Simple | Limited scale |

---

### Option B: Lightweight Custom Solution

**Stack: Go + HTMX + SQLite**

- **Go**: Single binary, perfect for Docker, fast
- **HTMX**: Server-side rendering, simple, fast
- **SQLite**: Embedded, perfect for single-server use

**Why this combo?**
- Single binary deployment
- Low memory usage
- Fast enough for 5 stacks
- Easy to maintain

---

### Option C: Integration-Based (Use Existing Tools)

**Tools to integrate:**
- **Loki**: Log aggregation
- **Prometheus**: Metrics
- **Grafana**: Visualization
- **Docker Compose VA**: Service discovery

**Problem**: Heavy setup, resource intensive

---

## 4. Recommended Architecture

### Loggy Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Loggy Server                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │
│  │  Discovery  │  │   Log       │  │    Metrics      │  │
│  │   Engine    │  │   Aggregator│  │   Collector     │  │
│  └──────┬──────┘  └──────┬──────┘  └────────┬────────┘  │
│         │                │                   │            │
│         └────────────────┴───────────────────┘            │
│                          │                                │
│                   ┌──────▼──────┐                       │
│                   │   Storage    │                       │
│                   │   (SQLite)  │                       │
│                   └──────┬──────┘                       │
│                          │                                │
└──────────────────────────┼────────────────────────────────┘
                           │
              ┌────────────▼────────────┐
              │      Web UI             │
              │   (HTMX + Go)          │
              │   Dashboard             │
              └─────────────────────────┘
```

### Component Breakdown

| Component | Responsibility |
|-----------|---------------|
| Discovery Engine | Scan for docker-compose.yml files |
| Log Aggregator | Stream logs from Docker daemon |
| Metrics Collector | Gather CPU/Memory/Network stats |
| Storage Engine | SQLite with time-series optimization |
| Web Server | Serve UI, handle API requests |
| Filter Engine | Auto-generate filters from log patterns |

---

## 5. Phase Plan

### Phase 1: MVP (Week 1-2)

**Goal**: Basic log streaming from auto-detected stacks

- [ ] Auto-discover docker-compose stacks
- [ ] Docker daemon integration
- [ ] Real-time log streaming
- [ ] Basic dashboard UI
- [ ] Container selection

**Tech**: Go + HTMX + SQLite

### Phase 2: Smart Features (Week 3-4)

**Goal**: Intelligent filtering and search

- [ ] Auto-generate filters from log patterns
- [ ] Full-text search in logs
- [ ] Log level detection (ERROR, WARN, INFO, DEBUG)
- [ ] Pattern recognition (stack traces, exceptions)

### Phase 3: Observability (Week 5-6)

**Goal**: Metrics and health monitoring

- [ ] Container metrics (CPU, Memory, Network)
- [ ] Health status dashboard
- [ ] Alert on errors
- [ ] Log correlation with metrics

### Phase 4: Polish (Week 7-8)

**Goal**: Production-ready features

- [ ] Historical log retention
- [ ] Export logs
- [ ] User preferences
- [ ] Dark/Light theme
- [ ] Performance optimization

---

## 6. Risk Assessment

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Docker API rate limits | Medium | Low | Implement caching |
| High log volume | High | Medium | Log rotation, sampling |
| Memory usage | High | Medium | Boundaries, limits |
| Complex parsing | Medium | Low | Incremental approach |
| UI performance | Medium | Low | HTMX is fast |

---

## 7. Technical Decisions (Draft)

### Pending Decisions:

1. **Database**: SQLite or PostgreSQL?
2. **Frontend**: HTMX or React?
3. **Log Retention**: How long to keep logs?
4. **Storage Location**: Local or cloud?

### Questions for You:

1. Do you want historical logs (days/weeks)?
2. Prefer simple HTMX or richer React UI?
3. Need remote access (outside VPS)?
4. Alerting needed?

---

## 8. Next Steps

1. ⬜ Review this plan
2. ⬜ Answer pending questions
3. ⬜ Confirm tech stack
4. ⬜ Start Phase 1 implementation

---

*Created with Pi Planning Skill*
*Date: 2026-03-03*
