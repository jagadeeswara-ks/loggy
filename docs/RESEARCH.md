# Loggy - Observability Platform Research & Goals

## Industry Comparison

| Platform | Logs/sec (typical) | Storage | Query Speed | Cost |
|----------|-------------------|---------|-------------|------|
| **Grafana Loki** | 1M+/sec | S3, MinIO | Fast | Low |
| **Elasticsearch** | 100-500K/sec | Local/S3 | Medium | High |
| **ClickHouse** | 1M+/sec | Local/S3 | Very Fast | Low |
| **Splunk** | 100K/sec | Cloud | Fast | Very High |
| **Datadog** | Varies | Cloud | Fast | High |

## Loggy Goals

### Performance Targets
| Metric | Target |
|--------|--------|
| **Ingestion** | 50,000+ logs/second |
| **Query Latency** | <100ms for 1M records |
| **Storage** | S3-compatible (MinIO, AWS S3, GCS) |
| **Retention** | Configurable (7d - 1 year) |
| **Real-time** | <1 second delay |

## Feature Comparison

### Essential Features (Must Have)
| Feature | Loki | Elasticsearch | Loggy (Target) |
|---------|-------|--------------|----------------|
| Stack/Project filtering | ✅ Labels | ✅ Indices | ✅ |
| Container filtering | ✅ Labels | ✅ Query | ✅ |
| Log level filtering | ✅ Levels | ✅ Query | ✅ |
| Time range | ✅ Instant | ✅ Range | ✅ |
| Full-text search | ✅ LogQL | ✅ Lucene | ✅ |
| JSON log parsing | ✅ Labels | ✅ Mapping | ✅ |
| Export | ✅ API | ✅ API | ✅ |

### Advanced Features (Should Have)
| Feature | Loki | Elasticsearch | Loggy (Target) |
|---------|-------|--------------|----------------|
| Pattern detection | ✅ | ✅ | ✅ |
| Metrics from logs | ✅ | ✅ | ✅ |
| Alerting | ✅ | ✅ | ✅ |
| Dashboards | ✅ (Grafana) | ✅ (Kibana) | ✅ |
| SQL queries | ❌ | ✅ | ✅ |

## Filter Options to Implement

### Current Implementation
- ✅ Stack selector
- ✅ Container checkboxes
- ✅ Log level (All/Error/Warn/Info/Debug)
- ✅ Text search
- ✅ Real-time streaming

### Enhanced Filters (To Add)
1. **Time Range Presets**
   - Last 5 minutes, 15 minutes, 1 hour, 6 hours
   - Last 24 hours, 7 days, 30 days
   - Custom range (from/to datetime)

2. **Advanced Level Filters**
   - All levels
   - Error + Fatal
   - Warning + Critical
   - Info + Debug
   - Custom level combination

3. **Container Filters**
   - Select all / Deselect all
   - By status (running/stopped)
   - By image name

4. **Message Filters**
   - Contains text
   - Regex pattern match
   - Exclude pattern

5. **Source Filters**
   - Stdout / Stderr
   - Specific container IDs

## UI/UX Goals

### Ease of Use Principles
1. **Zero learning curve** - Intuitive, similar to GitHub, Jira
2. **Keyboard shortcuts** - Power user efficiency
3. **Dark mode default** - Developer-friendly
4. **Responsive** - Works on laptop and desktop
5. **Offline capable** - Cache recent queries

### Dashboard Layout
```
┌─────────────────────────────────────────────────────────────┐
│ Logo │ Stack ▼ │ Time Range ▼ │ Search...    │ Export │ ⚙️  │
├────────────┬────────────────────────────────────────────────┤
│ STACKS    │ STATS PANEL                                   │
│ ├─ app    │ Errors: 12  Warnings: 45  Total: 1,234        │
│ └─ infra  ├─────────────────────────────────────────────── │
│ CONTAINERS│ FILTERS                                       │
│ ☑ api    │ [All▼] [Container▼] [Level▼] [Source▼]       │
│ ☑ worker  │ [Search logs...] [Regex] [Clear]              │
│ ☑ db      ├─────────────────────────────────────────────── │
│           │ LOG VIEWER                                      │
│           │ 12:00:01 api ERROR Connection failed...       │
│           │ 12:00:02 worker WARN Retry attempt 3...      │
│           │ 12:00:03 db INFO Query executed...           │
└────────────┴───────────────────────────────────────────────┘
```

## Implementation Plan

### Phase 1: Enhanced Filtering (Current)
- [x] Stack selection
- [x] Container selection  
- [x] Log level filtering
- [x] Text search
- [ ] Time range selector
- [ ] Load historical logs from database

### Phase 2: Storage & Query
- [ ] Fix ClickHouse log storage
- [ ] Implement time-range queries
- [ ] Add pagination
- [ ] Optimize query performance

### Phase 3: Advanced Features
- [ ] Regex pattern matching
- [ ] Exclude patterns
- [ ] Source filtering (stdout/stderr)
- [ ] SQL query interface

### Phase 4: Export & Integration
- [ ] Export to multiple formats
- [ ] API for external tools
- [ ] Webhook alerts
