# loggy

**Docker log aggregation into a columnar store, with classification at ingest.**

Reads container logs off the Docker socket, classifies each line as it arrives, writes to
ClickHouse, and pushes matches to a WASM frontend over WebSocket. Built to answer what
`docker logs` can't: *what is happening across every container at once, and has it
happened before?*

## Design decisions

| Choice | Reasoning |
|---|---|
| **ClickHouse, not Postgres** | Logs are append-only and queried by time range and level. `MergeTree` partitioned `toYYYYMMDD(timestamp)`, ordered `(container_id, timestamp)` — the sort key matches the query pattern, so range scans stay on one partition |
| **Classify at ingest, not query** | Regex classification (error / warn / stack trace / HTTP 4xx-5xx / SQL / JSON) runs as lines arrive, so queries filter on an indexed `level` column instead of scanning message text |
| **Interval tail, not `follow`** | `bollard` with `follow: false` on a timer. Bounded work per tick and no per-container task that can wedge — at the cost of sub-interval latency |
| **Buffered RowBinary writes** | Rows go through `Inserter`, flushing at 10,000 rows or 5s. Single-row inserts cap throughput far below what MergeTree absorbs, and each is a separate HTTP round trip. Buffers are flushed on shutdown |
| **No SQL string building anywhere** | Log content is attacker-controlled — any container can emit any bytes — and query filters arrive straight off HTTP params. Writes use RowBinary; reads use bound parameters |
| **WebSocket push** | The UI subscribes; the server is never polled |
| **Rust + axum** | Ingest is the hot path and runs at container-log volume |

## Limitations

- **`follow: false`** means log latency is bounded by the poll interval, not sub-second.
- **Metrics are point-in-time samples** on the same interval — not continuous.
- **7-day TTL** is hardcoded in the table DDL rather than configurable.
- **No auth on the WebSocket** — the API key check covers REST routes only.

## Quick start

```bash
docker-compose up -d
```

## Access

| Service | URL |
|---------|-----|
| **Loggy UI** | http://localhost:8080 |
| **ClickHouse** | http://localhost:8123 |

## Environment Variables

### Server
| Variable | Default | Description |
|----------|---------|-------------|
| `LOGGY_HOST` | `0.0.0.0` | Server host |
| `LOGGY_PORT` | `8080` | Server port |
| `LOGGY_SERVER__WORKERS` | `1` | Number of workers |

### Database
| Variable | Default | Description |
|----------|---------|-------------|
| `LOGGY_DATABASE__HOST` | `localhost` | ClickHouse host |
| `LOGGY_DATABASE__PORT` | `9000` | ClickHouse port |
| `LOGGY_DATABASE__DATABASE` | `loggy` | Database name |
| `LOGGY_DATABASE__USERNAME` | `default` | Username |
| `LOGGY_DATABASE__PASSWORD` | `` | Password |
| `LOGGY_DATABASE__MAX_CONNECTIONS` | `10` | Max connections |

### Docker
| Variable | Default | Description |
|----------|---------|-------------|
| `LOGGY_DOCKER__SOCKET_PATH` | `/var/run/docker.sock` | Docker socket path |
| `LOGGY_DOCKER__POLL_INTERVAL_MS` | `1000` | Poll interval |

### Authentication
| Variable | Default | Description |
|----------|---------|-------------|
| `LOGGY_AUTH__ENABLED` | `false` | Enable authentication |
| `LOGGY_AUTH__API_KEYS` | - | Comma-separated API keys |

Example:
```bash
LOGGY_AUTH__ENABLED=true
LOGGY_AUTH__API_KEYS=key1,key2,key3
```

### CORS
| Variable | Default | Description |
|----------|---------|-------------|
| `LOGGY_CORS__ALLOWED_ORIGINS` | `*` | Allowed origins (comma-separated) |
| `LOGGY_CORS__ALLOWED_METHODS` | `GET,POST,PUT,DELETE` | Allowed methods |
| `LOGGY_CORS__ALLOWED_HEADERS` | `*` | Allowed headers |
| `LOGGY_CORS__ALLOW_CREDENTIALS` | `false` | Allow credentials |

Production example:
```bash
LOGGY_CORS__ALLOWED_ORIGINS=https://yourdomain.com
LOGGY_CORS__ALLOW_CREDENTIALS=true
```

### Background Tasks
| Variable | Default | Description |
|----------|---------|-------------|
| `LOGGY_BACKGROUND__ENABLE_LOG_STREAMING` | `true` | Enable log streaming |
| `LOGGY_BACKGROUND__ENABLE_METRICS_COLLECTION` | `true` | Enable metrics collection |
| `LOGGY_BACKGROUND__LOG_INTERVAL_MS` | `5000` | Log streaming interval |
| `LOGGY_BACKGROUND__METRICS_INTERVAL_MS` | `10000` | Metrics collection interval |
| `LOGGY_BACKGROUND__LOG_BUFFER_SIZE` | `10000` | Log buffer size |

## API Endpoints

```bash
# Health check
curl http://localhost:8080/api/health

# List containers
curl http://localhost:8080/api/containers

# Query logs
curl "http://localhost:8080/api/logs?level=error&limit=10"

# Get metrics
curl "http://localhost:8080/api/metrics"

# Get detected patterns
curl http://localhost:8080/api/patterns

# Auth status
curl http://localhost:8080/api/auth/status
```

## Authentication

When authentication is enabled, include the API key in requests:

```bash
curl -H "Authorization: Bearer <your-api-key>" http://localhost:8080/api/containers
```

## Development

```bash
# Run ClickHouse only
docker-compose up -d clickhouse

# Run Loggy locally (needs ClickHouse)
cargo run --release
```

## Docker Commands

```bash
# Stop everything
docker-compose down

# View logs
docker-compose logs -f

# Rebuild
docker-compose build
```

## Tech Stack

| Component | Technology |
|-----------|------------|
| Backend | Rust + Axum |
| Database | ClickHouse |
| Frontend | WASM (Yew) |
| Real-time | WebSockets |
| Docker API | Bollard |

## Features

- ✅ Real-time log streaming via WebSocket
- ✅ Container metrics collection
- ✅ Pattern detection (errors, warnings, stack traces)
- ✅ Configurable alerting
- ✅ Docker Compose stack discovery
- ✅ Graceful shutdown
- ✅ CORS configuration
- ✅ API authentication
- ✅ Rate limiting
