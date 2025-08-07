# syntax=docker/dockerfile:1

##########################################
## 1️⃣ Builder Stage (multi‑arch)        ##
##########################################

# Use a manual build argument for platform selection for robustness.
# Default to amd64, but can be overridden with --build-arg.
ARG BUILD_PLATFORM=linux/amd64
# Pin the exact Rust version for reproducibility
FROM --platform=${BUILD_PLATFORM} rust:1.88.0-bullseye AS build

# Make the build-arg available in the stage
ARG BUILD_PLATFORM

# Set the working directory. The build context should be the workspace root.
WORKDIR /app

# Install musl-tools for C compilation with musl target
RUN apt-get update && \
    apt-get install -y --no-install-recommends musl-tools && \
    rm -rf /var/lib/apt/lists/*

# ------------------------------------------------------------------
# Install the correct musl target for the requested platform
# ------------------------------------------------------------------
# We determine the RUST_TARGET and save it to a file, because
# environment variables are not persisted between RUN commands.
RUN case ${BUILD_PLATFORM} in \
    "linux/amd64")   RUST_TARGET="x86_64-unknown-linux-musl" ;; \
    "linux/arm64")   RUST_TARGET="aarch64-unknown-linux-musl" ;; \
    *) echo "Unsupported platform: ${BUILD_PLATFORM}" && exit 1 ;; \
    esac && \
    rustup target add ${RUST_TARGET} && \
    echo -n "${RUST_TARGET}" > /app/rust_target

# ------------------------------------------------------------------
# Cache Cargo dependencies for the workspace
# ------------------------------------------------------------------
# This section is structured to maximize Docker layer caching.
# We first copy only the files needed to fetch dependencies,
# so the expensive `cargo fetch` is only re-run when dependencies change.

# 1. Create the directories for our crates first.
RUN mkdir -p crates/lib && mkdir -p crates/server

# 2. Copy all manifest files.
COPY Cargo.toml Cargo.lock ./
COPY crates/lib/Cargo.toml ./crates/lib/
COPY crates/server/Cargo.toml ./crates/server/

# 3. Create dummy source files to allow `cargo fetch` to work.
# Cargo needs a `main.rs` or `lib.rs` to understand the project structure.
RUN mkdir crates/lib/src && echo "pub fn lib() {}" > crates/lib/src/lib.rs
RUN mkdir crates/server/src && echo "fn main() {}" > crates/server/src/main.rs

# 4. Fetch dependencies. This is the most time-consuming step to cache.
RUN cargo fetch --locked

# ------------------------------------------------------------------
# Build the application
# ------------------------------------------------------------------
# 5. Now, copy all the actual source code into the container.
# This will invalidate the cache from this point on whenever source code changes.
COPY . .

# 6. Perform the actual build for the server binary.
# We read the RUST_TARGET from the file and then copy the final binary
# to a predictable location for the next stage.
RUN RUST_TARGET=$(cat /app/rust_target) && \
    cargo build \
    --package anyquery-server \
    --release \
    --target "${RUST_TARGET}" \
    --locked && \
    cp "/app/target/${RUST_TARGET}/release/server" /app/server

##########################################
## 2️⃣ Runtime Stage (minimal, secure)   ##
##########################################

FROM alpine:3.18

# Create a dedicated non‑root user with deterministic UID/GID
RUN addgroup -S app && \
    adduser -S -u 1000 -G app app

# Copy the compiled binary from the predictable location in the build stage.
COPY --from=build /app/server /usr/local/bin/app

# Set executable flag and ownership
RUN chmod 755 /usr/local/bin/app && \
    chown app:app /usr/local/bin/app

# Expose the service port
EXPOSE 8080

# Health check for the service
HEALTHCHECK --interval=30s --timeout=5s \
    CMD curl -f http://localhost:8080/health || exit 1

# Run as the non‑root user
USER app

# Run the binary
ENTRYPOINT ["/usr/local/bin/app"]
