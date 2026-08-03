# Feen backend

Rust backend for Tap Titans raid simulations. Player stats and boss events are versioned in PostgreSQL; simulations run as persistent background jobs; every deck result is stored; and the backend calculates the highest-total 6- and 9-deck sets without duplicated cards.

## Run locally

Requirements: Rust and Docker.

```powershell
Copy-Item .env.example .env
docker compose up -d postgres
cargo run
```

The application applies SQL migrations at startup. `GET /api/health` checks both the API and database. The default address is `http://localhost:3000`.

If PostgreSQL is stopped or `DATABASE_URL` is omitted, the backend starts in degraded mode. Card definitions, player-data conversion, synchronous simulations, health, and static assets remain available. Database-backed endpoints return `503 DATABASE_UNAVAILABLE`; restart the backend after PostgreSQL is available to reconnect.

Docker publishes PostgreSQL on host port `5433` to avoid conflicting with a locally installed PostgreSQL service.

Requests to `/internal/*` must include `x-internal-api-key` matching `INTERNAL_API_KEY`.

To stop the local database without deleting its data:

```powershell
docker compose down
```

## Workflow

1. Create a player with `POST /api/players`. The optional unique `player_id` holds the Tap Titans player identifier.
2. Store the player's current stats with `PUT /api/players/{player_id}/stats`. Each update creates an immutable version.
3. Enable automatic simulations with `PUT /api/players/{player_id}/auto_sims` and `{ "auto_sims": true }`.
4. Deliver a normalized boss-spawn event to `POST /internal/raid-events`. Duplicate `event_id` values are idempotent.
5. A persistent simulation job is created for every player with `auto_sims` enabled. Poll `GET /internal/simulation-jobs/{job_id}` or list `GET /api/players/{player_id}/simulation-jobs`.
6. Read the best set from `GET /api/players/{player_id}/recommendations/current?deck_count=6` (or `9`).

Updating the stats of an auto-sim player, or enabling `auto_sims` while a boss is active, automatically schedules the current boss simulation. Equivalent player/stat/boss/simulator inputs reuse the existing job.

All individual results are available at `GET /api/simulation-jobs/{job_id}/deck-results?limit=100&offset=0`.

The existing synchronous simulation endpoints remain available for debugging and compatibility. New automated callers should use `POST /internal/simulation-jobs`.

## Raid API boundary

`POST /internal/raid-events` is the stable normalized event boundary. Tap Titans exposes raid data over Socket.IO using an in-game generated token, but the protocol documentation and token are external to this repository. A live subscriber can map its boss-spawn message to this endpoint without coupling the simulation/domain code to Socket.IO. Do not commit a raid API token; supply it through runtime configuration when the subscriber is added.

## Checks

```powershell
cargo fmt --all --check
cargo clippy --all-targets
cargo test
cargo build --release
```

`SIMULATION_CONCURRENCY=1` is intentional by default: each simulation is CPU-heavy. Increase it only after measuring the target machine; extra workers can make every simulation slower through CPU contention.
