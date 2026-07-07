# syntax=docker/dockerfile:1

# Recommended software versions matching README.md
ARG RUST_VERSION=1.96.1
ARG DEBIAN_VERSION=bullseye-20260623-slim

ARG BUILDER_IMAGE="rust:${RUST_VERSION}-slim-bullseye"
ARG RUNNER_IMAGE="debian:${DEBIAN_VERSION}"

FROM --platform=$BUILDPLATFORM ${BUILDER_IMAGE} AS builder

ARG TARGETPLATFORM
ARG BUILDPLATFORM

LABEL org.opencontainers.image.authors=conradwt@gmail.com
LABEL org.opencontainers.image.title="Zero to GraphQL Using Rust"
LABEL org.opencontainers.image.url=https://hub.docker.com/u/conradwt/zero-to-graphql-using-rust
LABEL org.opencontainers.image.source=https://github.com/conradwt/zero-to-graphql-using-rust
LABEL org.opencontainers.image.licenses=MIT
LABEL com.conradtaylor.rust_version=$RUST_VERSION

# Install build dependencies
RUN apt-get update -y && apt-get install -y pkg-config libssl-dev build-essential git \
  && apt-get clean && rm -f /var/lib/apt/lists/*_*

WORKDIR /app

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "pub mod db; pub mod graphql;" > src/lib.rs \
    && echo "fn main() {}" > src/main.rs \
    && mkdir -p src/db src/graphql \
    && touch src/db.rs src/graphql.rs

RUN cargo build --release
RUN rm -rf src

# Copy migrations (needed at compile time for sqlx::migrate! embedding)
COPY db ./db

# Copy actual source code
COPY src ./src

# Touch lib.rs and main.rs to force cargo to rebuild them (since we replaced dummy files)
RUN touch src/lib.rs src/main.rs

# Build actual release binary
RUN cargo build --release

# Final runtime image
FROM ${RUNNER_IMAGE}

# Install SSL and other runtime necessities
RUN apt-get update -y && apt-get install -y libssl1.1 ca-certificates locales \
  && apt-get clean && rm -f /var/lib/apt/lists/*_*

# Set the locale
RUN sed -i '/en_US.UTF-8/s/^# //g' /etc/locale.gen && locale-gen

ENV LANG=en_US.UTF-8
ENV LANGUAGE=en_US:en
ENV LC_ALL=en_US.UTF-8

WORKDIR /app
RUN chown nobody /app

# Copy the compiled binary from builder
COPY --from=builder --chown=nobody:root /app/target/release/zero-to-graphql-using-rust ./

# Copy the database configuration directory
COPY --chown=nobody:root config ./config

USER nobody

# Start the web server by default
CMD ["/app/zero-to-graphql-using-rust"]
