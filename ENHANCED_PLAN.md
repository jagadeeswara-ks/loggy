# Loggy - Enhanced Observability Platform

## User Requirements

### Current Issues
- Logs only filter future logs, not historical
- Need better dashboard with analysis tools
- Need container-level log selection
- Need stack-level log collection

### Requirements
1. **Stack Selection** - When selected, collect logs for that stack
2. **Container Selection** - Select/unselect containers to filter logs
3. **Dashboard UI/UX** - Better analysis tools
4. **Log Storage** - Store logs for selected stacks/containers
5. **Export Options** - Export logs in various formats
6. **Long-term Storage** - Research storage options

---

## Research: Log Aggregation Platforms

### Popular Solutions

| Platform | Storage Backend | Export Options | Long-term Storage |
|----------|----------------|----------------|-------------------|
| **Grafana Loki** | S3, GCS, Azure Blob | JSON, CSV, Prometheus | MinIO, S3, GCS |
| **Elasticsearch** | Local disk, S3 | JSON, CSV, SQL | S3, Azure, GCP |
| **ClickHouse** | Local disk | JSON, CSV, SQL | S3 tables, remote disks |
| **Splunk** | Proprietary | CSV, JSON, PDF | Cloud, on-prem |
| **Datadog** | Cloud | JSON, CSV | Cloud storage |

### Log Export Formats
- **JSON** - Machine readable, good for APIs
- **CSV** - Spreadsheet analysis
- **SQL** - Direct database queries
- **Prometheus** - Metrics format

### Long-term Storage Options
1. **S3/MinIO** - Cheap object storage
2. **ClickHouse S3 Tables** - Native S3 integration
3. **Remote Disks** - Network attached storage
4. **Cloud Storage** - GCS, Azure Blob, AWS S3

---

## Implementation Plan

### Phase 1: Stack & Container Selection (Current)

#### Backend
- [x] Stack discovery endpoint
- [x] Container filtering by stack
- [x] Container filtering by search
- [ ] Store selected stack/containers in state
- [ ] Start log collection for selected stack

#### Frontend
- [x] Stack selector dropdown
- [x] Container list with selection
- [ ] Checkbox to select/deselect containers
- [ ] Visual indicator for selected containers

### Phase 2: Log Storage & History

#### Backend
- [ ] Fix ClickHouse query_logs to return actual data
- [ ] Add stack/container filters to log queries
- [ ] Implement log retention policies

#### Frontend
- [ ] Load historical logs on stack selection
- [ ] Show log count and time range
- [ ] Pagination for large log sets

### Phase 3: Dashboard & Analysis Tools

#### UI Features
- [ ] Log statistics panel (error count, warning count)
- [ ] Time range selector (last 1h, 24h, 7d, custom)
- [ ] Log level distribution chart
- [ ] Container resource overview

#### Analysis Tools
- [ ] Pattern highlighting (errors in red, stack traces)
- [ ] Log search with regex support
- [ ] Download/Export button (JSON, CSV)
- [ ] Copy log entries

### Phase 4: Export & Storage Options

#### Export Features
- [ ] Export visible logs to JSON
- [ ] Export visible logs to CSV
- [ ] Export date range selection
- [ ] Export container selection

#### Storage Configuration
- [ ] Retention period setting (7d, 30d, 90d, 1y)
- [ ] S3/MinIO configuration for long-term storage
- [ ] Log rotation settings

---

## API Changes

### Enhanced Endpoints

```
# Get logs with full filtering
GET /api/logs?stack=<name>&containers=<id1,id2>&level=<level>&from=<ts>&to=<ts>&limit=100

# Get log statistics
GET /api/logs/stats?stack=<name>&from=<ts>&to=<ts>

# Export logs
POST /api/logs/export
Body: { format: "json"|"csv", stack, containers, from, to }

# Configuration
GET /api/config/logging
PUT /api/config/logging
```

---

## Frontend Dashboard Design

```
┌────────────────────────────────────────────────────────────────────┐
│  HEADER: Logo | Stack Selector | Time Range | Export | Settings    │
├──────────────┬───────────────────────────────────────────────────┤
│  SIDEBAR    │  MAIN CONTENT                                      │
│              │                                                    │
│  Stacks     │  ┌─────────────────┬───────────────────────────┐  │
│  └ stack1   │  │ Stats Panel    │ Log Level Distribution    │  │
│    ✓ cont1  │  │ Errors: 12     │ [████████░░] Error        │  │
│    ✓ cont2  │  │ Warnings: 45   │ [██████████] Warn         │  │
│    ✗ cont3  │  │ Total: 1,234  │ [████] Info              │  │
│  └ stack2   │  └─────────────────┴───────────────────────────┘  │
│              │                                                    │
│  Containers │  ┌─────────────────────────────────────────────┐   │
│  └ contA    │  │ LOG VIEWER                                │   │
│  └ contB    │  │ [Filter] [Search...] [Export] [Clear]    │   │
│              │  │ ─────────────────────────────────────────  │   │
│              │  │ 2026-03-04 12:00:00 cont1 ERROR Failed..│   │
│              │  │ 2026-03-04 12:00:01 cont2 WARN Retry... │   │
│              │  │ 2026-03-04 12:00:02 cont1 INFO OK       │   │
│              │  └─────────────────────────────────────────────┘   │
└──────────────┴───────────────────────────────────────────────────┘
```

---

## Next Steps

### Immediate Priorities
1. Fix log query to return historical data
2. Add container checkboxes for selection
3. Add export functionality
4. Improve dashboard layout

### Questions for User
1. What time ranges do you need? (1h, 24h, 7d, custom)
2. What export formats? (JSON, CSV, both)
3. Where to store long-term? (local, S3, ClickHouse)
4. What analysis features? (charts, patterns, search)

---

## Commands to Implement

Use this command to continue implementation:

```
Implement Phase 2: Enhanced Log Storage & Dashboard

1. Fix src/db/mod.rs query_logs to return actual ClickHouse data
2. Add container_id and stack filters to log queries  
3. Create /api/logs/stats endpoint for statistics
4. Add container checkboxes to frontend
5. Add export buttons (JSON, CSV)
6. Add time range selector to UI
7. Improve dashboard layout with stats panel

Project: /home/kjr/loggy
Running: http://localhost:8080
```

