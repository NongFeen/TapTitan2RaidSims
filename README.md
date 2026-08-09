# Feen backend

Rust backend for Tap Titans raid simulations. Player stats are versioned in PostgreSQL, one current boss is stored, simulations run as persistent background jobs, every current result is stored, and the backend calculates the highest-total 6- and 9-deck sets without duplicated cards.

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

### TT2 player API

Player fetching requires all five `TT2_*` values from `.env`. The Socket.IO client registers its handlers before explicitly connecting, uses only WebSocket transport, and does not automatically reconnect in development. If it cannot connect, the rest of the backend continues in degraded mode and fetch requests return `503`.

Generate the AES-256-GCM token-encryption key once and keep it stable:

```powershell
$bytes = New-Object byte[] 32
[Security.Cryptography.RandomNumberGenerator]::Fill($bytes)
[Convert]::ToBase64String($bytes)
```

Store that output as `TT2_PLAYER_TOKEN_ENCRYPTION_KEY`. Losing or changing it makes existing encrypted player tokens unreadable. The application token and encryption key are backend secrets and must never use `VITE_*` frontend variables.

Protected admin workflow:

1. `PUT /internal/players/{player_id}/token` stores `{ "player_token": "..." }` encrypted.
2. `POST /internal/players/{player_id}/fetch-stats` fetches, converts, validates, and saves the latest GameHive player data.
3. `DELETE /internal/players/{player_id}/token` removes the token.
4. `GET /internal/tt2/player-status` reports whether the integration is configured and connected.

To stop the local database without deleting its data:

```powershell
docker compose down
```

## Workflow

1. Create a player with `POST /api/players`. The optional unique `player_id` holds the Tap Titans player identifier.
2. Store the player's current stats with `PUT /api/players/{player_id}/stats`. Each update creates an immutable version.
3. Enable automatic simulations with `PUT /api/players/{player_id}/auto_sims` and `{ "auto_sims": true }`.
4. Replace the current boss with `PUT /internal/current-boss`. Set `run_sims` to `true` to queue jobs or `false` to only store the boss.
5. A persistent simulation job is created for every player with `auto_sims` enabled. Poll `GET /internal/simulation-jobs/{job_id}` or list `GET /api/players/{player_id}/simulation-jobs`.
6. Read the best set from `GET /api/players/{player_id}/recommendations/current?deck_count=6` (or `9`).

Updating the stats of an auto-sim player, or enabling `auto_sims` while a boss is active, automatically schedules the current boss simulation. Equivalent player/stat/boss/simulator inputs reuse the existing job.

`PUT /api/players/{player_id}/stats` accepts either the raw Tap Titans player export or the cleaned `PlayerRaidData` format. Raw input is cleaned automatically before it is validated and stored. Each update overwrites that player's current stats; `GET /api/players/{player_id}/stats/current` reads them.

All individual results are available at `GET /api/simulation-jobs/{job_id}/deck-results?limit=100&offset=0`.

The existing synchronous simulation endpoints remain available for debugging and compatibility. New automated callers should use `POST /internal/simulation-jobs`.

## Raid API boundary

`PUT /internal/current-boss` is the single normalized boss boundary. Its body contains `boss_data`, `attackable_parts`, and `run_sims`. Replacing the singleton current boss deletes all previous simulation jobs, deck results, and recommendations. Read it through `GET /api/current-boss`. A future Tap Titans subscriber should call the update route with `run_sims: true`; manual setup can use `false`.

## Checks

```powershell
cargo fmt --all --check
cargo clippy --all-targets
cargo test
cargo build --release
```

`SIMULATION_CONCURRENCY=1` is intentional by default: each simulation is CPU-heavy. Increase it only after measuring the target machine; extra workers can make every simulation slower through CPU contention.
