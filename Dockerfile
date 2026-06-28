FROM rust:1-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p lectito-api

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/lectito-api /usr/local/bin/lectito-api
ENV PORT=3000
CMD ["lectito-api"]
