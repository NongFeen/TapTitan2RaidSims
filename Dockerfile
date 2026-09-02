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
RUN cargo build --release --bin backend

####################
# 2. Runtime image #
####################
FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --shell /usr/sbin/nologin appuser

WORKDIR /app
COPY --from=builder /app/target/release/backend /usr/local/bin/backend

USER appuser
ENV PORT=3000
EXPOSE 3000

HEALTHCHECK --interval=10s --timeout=3s --start-period=10s --retries=5 \
    CMD curl -f "http://localhost:${PORT}/api/health" || exit 1

ENTRYPOINT ["/usr/local/bin/backend"]
