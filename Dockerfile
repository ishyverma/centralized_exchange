ARG SERVICE

FROM rust:slim-bookworm AS builder
ARG SERVICE
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .

# Build only the requested service (plus its workspace dependencies)
RUN cargo build --release -p ${SERVICE} \
  && cp target/release/${SERVICE} /usr/local/bin/server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/bin/server /usr/local/bin/server
EXPOSE 8080
CMD ["server"]
