# syntax=docker/dockerfile:1

########################
# 1. Dependency caching #
########################
FROM rust:1-slim-bookworm AS chef
WORKDIR /app

# The repo's .cargo/config.toml pins clang+lld as the linker (faster local
# incremental builds), so the build image needs to provide them too.
RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential pkg-config clang lld curl \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Builds and caches all dependencies as their own layer, so editing
# application code doesn't force every crate to recompile.
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
# Cargo.toml sets profile.release.debug = true (kept for local profiling),
# so strip the debug symbols here instead -- dead weight in a deploy image.
RUN cargo build --release --bin backend \
    && strip target/release/backend

####################
# 2. Runtime image #
####################
# distroless: just glibc + libssl + CA certs, no shell/package manager, so
# no perl/gcc/curl/tar/etc dragged along -- scans at 0 CVEs vs 50+ baked
# into debian:bookworm-slim before installing anything. Trade-off: no shell
# means no curl-based HEALTHCHECK; nothing depends on this container's own
# health status unless you wire that up at the compose/orchestrator level.
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

WORKDIR /app
COPY --from=builder /app/target/release/backend /usr/local/bin/backend

ENV PORT=3000
EXPOSE 3000

ENTRYPOINT ["/usr/local/bin/backend"]
