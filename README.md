# Loggy - Docker Observability Platform

A high-performance Docker log aggregation and analysis platform built with Rust, ClickHouse, WebSockets, and WASM.

## Quick Start (Single Command)

```bash
cd ~/loggy && docker-compose up -d
```

That's it! 🚀

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
