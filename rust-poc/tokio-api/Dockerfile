# ── Build stage ──────────────────────────────────────────────────────────────
FROM rust:1.86-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y libsqlite3-dev && rm -rf /var/lib/apt/lists/*

# Cache dependencies before copying source
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm src/main.rs

COPY src ./src
RUN touch src/main.rs && cargo build --release

# ── Runtime stage ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/tokio-api .

# Persist the SQLite database file across restarts
VOLUME ["/app/data"]

ENV DATABASE_URL=sqlite:/app/data/items.db

EXPOSE 3000

CMD ["./tokio-api"]
