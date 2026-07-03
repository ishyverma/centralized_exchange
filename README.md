# Centralized Exchange Backend

A cryptocurrency exchange backend built in Rust, inspired by the architecture and API conventions of Binance. This project implements a matching engine with price-time priority, a REST API for order management, JWT-based authentication, and an API gateway that routes and secures requests to backend services.

## What We Are Building

A modular, multi-service exchange backend that supports:

- User registration and login with Argon2 password hashing
- JWT-based session tokens and HMAC-signed API keys for programmatic trading
- Limit order placement with real-time price-time priority matching
- Partial fill tracking and order lifecycle management (NEW, PARTIALLY_FILLED, FILLED, CANCELLED)
- Order book state with bids (highest-first) and asks (lowest-first) snapshots
- Trade history recording with match-level granularity
- Kafka event streaming infrastructure for external consumers
- Rate limiting skeleton at the gateway layer

## Architecture

The system follows an API gateway pattern with separate microservices, each in its own crate:

```
Client -> API Gateway (port 8080) -> Auth Service (port 8081)
                                  -> Order Service (port 8082)
                                       -> Matching Engine (in-process)
                                       -> PostgreSQL
```

### API Gateway

The entry point for all client requests. Written with Actix-web, it terminates TLS, authenticates requests via JWT, applies rate limiting, and reverse-proxies requests to the appropriate backend service. Routes mirror the Binance API:

- `/api/v3/ping`, `/api/v3/time` -- health and server time
- `/api/v3/auth/*` -- proxied to Auth Service
- `/api/v3/order` (GET, POST, DELETE) -- proxied to Order Service
- `/api/v3/allOrders` (GET) -- proxied to Order Service

### Auth Service

Handles user identity and access control. Uses Argon2id for password hashing (memory-hard, resistant to GPU/ASIC attacks) and jsonwebtoken for session tokens. API keys are generated with `bp_` prefix and HMAC-SHA256 secrets. Endpoints:

- `POST /api/v3/auth/register` -- create account with email + password
- `POST /api/v3/auth/login` -- authenticate and receive JWT
- `GET /api/v3/auth/me` -- current user profile
- `POST /api/v3/auth/api-keys` -- generate API key/secret pair
- `GET /api/v3/auth/api-keys` -- list active API keys
- `DELETE /api/v3/auth/api-keys` -- revoke an API key

### Order Service

Manages order persistence and lifecycle. Accepts validated orders, sends them to the matching engine, records trades, and maintains order state in PostgreSQL. Endpoints follow the Binance REST API spec:

- `POST /api/v3/order` -- place a limit or market order
- `GET /api/v3/order` -- query order by orderId or origClientOrderId
- `DELETE /api/v3/order` -- cancel an open order
- `GET /api/v3/allOrders` -- list historical orders with pagination

### Matching Engine

An in-process, single-threaded matching engine that implements price-time priority (first-come, first-served within each price level). It runs as part of the Order Service process (behind a `Mutex<MatchingEngine>`) to avoid Kafka round-trip latency for matching. Key design:

- BTreeMap-based order books with bids as descending keys and asks as ascending keys
- For a BUY order, the engine sweeps asks where ask price <= taker price, lowest price first
- For a SELL order, the engine sweeps bids where bid price >= taker price, highest price first
- Within a price level, FIFO order via Vec with first_mut() access
- Partial fills leave the remaining quantity on the order book
- A monotonically increasing match counter assigns unique match IDs to each trade
- Matches produce MatchingEvent variants (OrderPlaced, OrderMatched, OrderCancelled, OrderBookSnapshot) for downstream consumers

### Common Crate

Shared types and error definitions used across all services:

- Domain enums: Side (Buy/Sell), OrderType (Limit/Market), OrderStatus, TimeInForce, UserStatus, ApiKeyStatus
- Domain structs: User, ApiKey, Balance, Order, Trade
- Error types: ApiError with variants for validation, auth, balance, order, and internal errors
- All monetary values use `rust_decimal` for precision

## Why Rust

- Memory safety without a garbage collector -- critical for financial systems where latency and predictability matter
- Zero-cost abstractions -- the matching engine performs tight loops over order book levels without allocation overhead
- Strong type system with enums and pattern matching -- domain types like Side, OrderStatus, and TradeData are modeled as enums, making illegal states unrepresentable
- Actix-web provides one of the fastest HTTP frameworks among all languages
- sqlx provides compile-time checked SQL queries against the actual database schema
- cargo workspaces allow clean separation of concerns across the gateway, auth, order service, and matching engine crates

## Why This Service Layout

The gateway pattern was chosen because:

- Authentication logic is centralized -- the gateway validates JWT tokens before any request reaches backend services, preventing auth bypass
- Backend services are isolated -- the order service never needs to know about API keys or user management; it receives authenticated user IDs as request data
- The matching engine lives in-process with the order service because matching latency is critical -- network round-trips to a separate matching service would add unacceptable delay for high-frequency trading
- Each service scales independently -- the auth service and order service can be deployed on separate machines and scaled based on their own load profiles

## Technologies Used

| Category | Technology |
|----------|-----------|
| Language | Rust (edition 2021) |
| HTTP Framework | Actix-web 4 |
| Database | PostgreSQL 16 |
| Caching | Redis 7 |
| Message Queue | Apache Kafka 7.7 (confluent) |
| Coordination | ZooKeeper 3.8 |
| ORM / Query | sqlx 0.8 with compile-time checking |
| Auth | Argon2id (password hashing), jsonwebtoken (JWT), HMAC-SHA256 (API keys) |
| Decimal Math | rust_decimal with serde support |
| Serialization | serde / serde_json |
| Async Runtime | tokio (full features) |
| Containerization | Docker Compose |

## Getting Started

### Prerequisites

- Rust toolchain (install via rustup)
- Docker and Docker Compose
- make (optional, for convenience targets)

### Running Locally

```bash
# Start infrastructure (PostgreSQL, Redis, Zookeeper, Kafka)
docker compose up -d

# Run SQL migrations
for f in sql/*.sql; do
  psql -h localhost -U backpack -d backpack -f "$f"
done

# Build all services
cargo build --workspace

# Run tests
cargo test --workspace
```

### Environment Variables

Copy `.env.example` to `.env` and configure:

- `DATABASE_URL` -- PostgreSQL connection string
- `REDIS_URL` -- Redis connection string
- `JWT_SECRET` -- secret key for signing JWT tokens
- `KAFKA_HOSTS` -- optional, enables Kafka event publishing

### Starting Services

Each service runs on its own port:

```bash
# Auth Service
cargo run -p auth-service

# Order Service
cargo run -p order-service

# API Gateway
cargo run -p api-gateway
```

## Inspiration

This project draws heavily from the design and API conventions of Binance, the world's largest cryptocurrency exchange. The REST API endpoints (`/api/v3/order`, `/api/v3/allOrders`), error codes (`-2013` for order not found, `-1013` for validation errors), and order state machine (NEW -> PARTIALLY_FILLED -> FILLED / CANCELLED) all follow Binance's public API specification.

The matching engine algorithm (price-time priority using BTreeMap with FIFO within levels) is the standard approach used by most major exchanges, including Binance, Coinbase, and Kraken.

The in-process matching engine pattern is inspired by the architecture of exchanges where matching latency is the primary concern -- rather than routing every order through a message queue, the matching logic runs in the same process as the API handler.

## CI/CD

GitHub Actions workflow runs on push and pull requests to main:

- Runs all SQL migrations against a PostgreSQL service container
- Checks formatting with `cargo fmt`
- Lints with `cargo clippy -D warnings`
- Builds the entire workspace
- Runs all tests with DATABASE_URL and REDIS_URL configured

## Project Status

The following functionality is implemented and tested:

- User authentication (register, login, JWT tokens)
- API key management (create, list, delete with HMAC secrets)
- API gateway routing, JWT auth middleware, rate limiter skeleton
- Limit order matching with price-time priority
- Partial fill support and order book maintenance
- Order CRUD (place, query, cancel, list history)
- Trade recording with match IDs
- Kafka event infrastructure (events are serialized and published via the EventProducer trait; a Noop producer is the default, Kafka producer requires configuration)
- 44 tests across all services (auth, gateway, matching engine)
- CI pipeline with clippy enforcement

## License

MIT
