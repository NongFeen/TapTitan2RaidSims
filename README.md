# Feen backend

Rust backend for Tap Titans raid simulations. Player stats are versioned in PostgreSQL, one sims boss is stored, simulations run as persistent background jobs, every current result is stored, and the backend calculates the highest-total deck sets without duplicated cards.

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

Player fetching and live raid synchronization require all `TT2_*` values from `.env`. The Socket.IO client uses WebSocket transport and reconnects with bounded exponential backoff. If it cannot connect, the rest of the backend continues in degraded mode and fetch requests return `503`.

Generate the AES-256-GCM token-encryption key once and keep it stable:

```powershell
$bytes = New-Object byte[] 32
[Security.Cryptography.RandomNumberGenerator]::Fill($bytes)
[Convert]::ToBase64String($bytes)
```

Store that output as `TT2_PLAYER_TOKEN_ENCRYPTION_KEY`. Losing or changing it makes existing encrypted player tokens unreadable. The application token and encryption key are backend secrets and must never use `VITE_*` frontend variables.

Set `TT2_RAID_SUBSCRIPTION_PLAYER_TOKEN` directly to one Master/Grand Master player token. The backend does not query PostgreSQL for raid subscription. Once per backend process, after the `/raid` namespace connects, it calls `/raid/unsubscribe` and then `/raid/subscribe`. Both requests send `TT2_APPLICATION_TOKEN` in `API-Authenticate` and `{ "player_tokens": ["..."] }` in the JSON body.

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

1. Fetch clan player data with `POST /internal/tt2/fetch-clan-stats`; the configured raid subscription player token identifies the clan, and clan sync creates or updates players by their Tap Titans player code.
2. Store a player's current stats with `PUT /api/players/{player_id}/stats` when a manual update is needed.
3. Enable automatic simulations with `PUT /api/players/{player_id}/auto_sims` and `{ "auto_sims": true }`.
4. Replace the persisted simulation input with `PUT /internal/sims-boss`. Set `run_sims` to `true` to queue jobs or `false` to only store the sims boss.
5. A persistent simulation job is created for every player with `auto_sims` enabled. Poll `GET /internal/simulation-jobs/{job_id}` or list `GET /api/players/{player_id}/simulation-jobs`.
6. Read the best set from `GET /api/players/{player_id}/recommendations/current?deck_count=6` (or `9`).

Updating the stats of an auto-sim player, or enabling `auto_sims` while a sims boss is active, automatically schedules the simulation. Equivalent player/stat/boss/simulator inputs reuse the existing job.

`PUT /api/players/{player_id}/stats` accepts either the raw Tap Titans player export or the cleaned `PlayerRaidData` format. Raw input is cleaned automatically before it is validated and stored. Each update overwrites that player's current stats; `GET /api/players/{player_id}/stats/current` reads them.

All individual results are available at `GET /api/simulation-jobs/{job_id}/deck-results?limit=100&offset=0`.

The existing synchronous simulation endpoints remain available for debugging and compatibility. New automated callers should use `POST /internal/simulation-jobs`.

## Raid API boundary

`PUT /internal/sims-boss` is the manual normalized simulation boundary, and `GET /api/sims-boss` reads it. The older `/internal/current-boss` and `/api/current-boss` paths remain compatibility aliases.

Live `attack` events update `GET /api/live-current-boss`, a read-only in-memory snapshot for dashboard display. This snapshot is cleared on restart, is never used as simulation input, and its boss HP payload is removed before the attack log is persisted. Authoritative `sub_cycle` events synchronize the persisted sims boss and can queue automatic Current+Void jobs when the enemy changes. `cycle_reset` updates modifiers and queues eligible jobs when the Mirror Force clan boost changes. Read the live cycle modifiers through `GET /api/raid-cycle/current`.

## Checks

```powershell
cargo fmt --all --check
cargo clippy --all-targets
cargo test
cargo build --release
```

`SIMULATION_CONCURRENCY=1` is intentional by default: each simulation is CPU-heavy. Increase it only after measuring the target machine; extra workers can make every simulation slower through CPU contention.

## Deploy (production images)

The backend runs as two separate services sharing one Postgres database, so
a running simulation's CPU usage can never freeze API request handling
(page loads, card images):

- **`Dockerfile`** -- the API/routing service. Serves all HTTP routes and
  enqueues simulation jobs into the `simulation_jobs` table, but never runs
  them itself.
- **`Dockerfile.worker`** -- the sim worker service. No HTTP server, no TT2
  socket; it only polls `simulation_jobs` for `pending` rows and runs them.
  Scale its `SIMULATION_CONCURRENCY`/`SIM_WORKER_COUNT` independently of the
  API service's resources.

Both build from this same `backend/` directory and read `ROLE` (`api` /
`worker`; unset or `all` runs both in one process, useful for local dev) to
decide which half of `main.rs` to run -- see `src/config.rs`'s
`ServiceRole`.

The frontend is **not** part of either image -- deploy it separately as a
static site (e.g. a Render Static Site) built from the `frontend/` repo,
with `VITE_API_BASE_URL` pointing at wherever the API service ends up
publicly reachable. It's static files with no compute needs of its own, so
keeping it off the same host/CPU budget as the API and worker avoids it
going unreachable if either of those get busy.

### Via docker compose

`docker-compose.prod.yml` runs Postgres, the API service, and the worker
service together (see that file's header comment for details):

```powershell
cp backend/.env.prod.example backend/.env.prod   # then fill in real values
docker compose -f backend/docker-compose.prod.yml --env-file backend/.env.prod up -d --build
```

### Via separate deploys (e.g. two Render Web Services)

Build each Dockerfile as its own service, both pointed at an external
`DATABASE_URL` (see `.env.prod.example`'s `DATABASE_URL` override):

```powershell
docker build -f backend/Dockerfile -t feen-api:latest backend
docker build -f backend/Dockerfile.worker -t feen-worker:latest backend
```

The API service needs `CORS_ALLOWED_ORIGINS`, `INTERNAL_API_KEY`, and (if
TT2 integration is enabled) the `TT2_*` secrets; the worker service only
needs `DATABASE_URL`, `SIMULATION_CONCURRENCY`, and `SIM_WORKER_COUNT` --
see `docker-compose.prod.yml`'s `worker` service for the exact variable
list. Verify the API service with:

```powershell
curl http://localhost:3000/api/health
```
