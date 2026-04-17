FROM rust:1.85-slim AS builder
WORKDIR /app
COPY crates/openusage-shared crates/openusage-shared
COPY crates/openusage-relay crates/openusage-relay
COPY Cargo.toml Cargo.toml
# Create dummy src-tauri so workspace resolves (not built)
RUN mkdir -p src-tauri/src && echo 'fn main() {}' > src-tauri/src/main.rs
COPY src-tauri/Cargo.toml src-tauri/Cargo.toml
RUN cargo build --release -p openusage-relay

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/openusage-relay /usr/local/bin/openusage-relay
ENV RELAY_PORT=8090
EXPOSE 8090
CMD ["openusage-relay"]
