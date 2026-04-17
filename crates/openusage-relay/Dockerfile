FROM rust:1.88-slim AS builder
WORKDIR /app

# Copy only the crates needed for the relay
COPY crates/openusage-shared crates/openusage-shared
COPY crates/openusage-relay crates/openusage-relay

# Create a minimal workspace that only includes relay + shared
RUN echo '[workspace]\nmembers = ["crates/openusage-shared", "crates/openusage-relay"]\nresolver = "2"' > Cargo.toml

RUN cargo build --release -p openusage-relay

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/openusage-relay /usr/local/bin/openusage-relay
ENV RELAY_PORT=8090
EXPOSE 8090
CMD ["openusage-relay"]
