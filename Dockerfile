# Build stage
# Rust >= 1.78 required: Cargo.lock is format version 4. (Was pinned at 1.75, which fails.)
FROM rust:1.83-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy Cargo files first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /build/target/release/loggy /app/loggy

# The UI is static and served by ServeDir::new("frontend"), resolved from the working dir.
COPY frontend ./frontend

# Create non-root user for security
RUN useradd -m -u 1000 loggy && chown -R loggy:loggy /app
USER loggy

# Expose port
EXPOSE 8080

# Run the application
CMD ["/app/loggy"]
