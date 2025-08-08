# syntax=docker/dockerfile:1
#
# Optimized Dockerfile for a Rust Workspace
#
# This Dockerfile is structured to maximize layer caching for faster builds,
# especially in CI/CD environments like Google Cloud Build with Kaniko.

# This ARG must be declared before the first FROM so it can be used there.
# It defines the target architecture for the build.
ARG BUILD_PLATFORM=linux/amd64

##########################################
## 1️⃣ Builder Stage (multi‑arch)        ##
##########################################

# Use the platform argument and pin the exact Rust version for reproducibility.
FROM --platform=${BUILD_PLATFORM} rust:1.88.0-bullseye AS build

# Re-declare the ARG within this stage to make it available to subsequent commands.
ARG BUILD_PLATFORM

# Set the working directory for the entire build stage.
WORKDIR /app

# Install musl-tools for static C compilation, which helps create a minimal final image.
RUN apt-get update && \
    apt-get install -y --no-install-recommends musl-tools && \
    rm -rf /var/lib/apt/lists/*

# Install the correct Rust musl target based on the build platform.
# This enables creating a statically linked binary.
RUN case ${BUILD_PLATFORM} in \
    "linux/amd64")   RUST_TARGET="x86_64-unknown-linux-musl" ;; \
    "linux/arm64")   RUST_TARGET="aarch64-unknown-linux-musl" ;; \
    *) echo "Unsupported platform: ${BUILD_PLATFORM}" && exit 1 ;; \
    esac && \
    rustup target add ${RUST_TARGET} && \
    echo -n "${RUST_TARGET}" > /app/rust_target

# ------------------------------------------------------------------
# Cache Dependencies (the slow part)
# ------------------------------------------------------------------
# Only copy files needed to resolve and compile dependencies.
# This layer will be cached as long as Cargo.toml/Cargo.lock don't change.

# 1. Create the project structure.
RUN mkdir -p crates/lib/src && mkdir -p crates/server/src

# 2. Copy manifests.
COPY Cargo.toml Cargo.lock ./
COPY crates/lib/Cargo.toml ./crates/lib/
COPY crates/server/Cargo.toml ./crates/server/

# 3. Create dummy source files.
# This allows `cargo build` to create a build plan and compile dependencies
# without needing the full source code yet.
RUN echo "pub fn lib() {}" > crates/lib/src/lib.rs
RUN echo "fn main() {println!(\"dependency build\")}" > crates/server/src/main.rs

# 4. Build only the dependencies.
# This is the most important caching step.
RUN RUST_TARGET=$(cat /app/rust_target) && \
    cargo build \
    --package anyrag-server \
    --release \
    --target "${RUST_TARGET}" \
    --locked

# ------------------------------------------------------------------
# Build the Application (the fast part)
# ------------------------------------------------------------------

# 5. Now, copy all the actual source code into the container.
COPY . .

# 6. Build the actual application binary.
# Cargo will reuse the already-compiled dependencies from the cached layer.
# We 'touch' the main source files to explicitly tell Cargo that these crates
# have been updated and need to be recompiled, invalidating the dummy versions.
RUN RUST_TARGET=$(cat /app/rust_target) && \
    touch crates/lib/src/lib.rs && \
    touch crates/server/src/main.rs && \
    cargo build \
    --package anyrag-server \
    --release \
    --target "${RUST_TARGET}" \
    --locked && \
    cp "/app/target/${RUST_TARGET}/release/server" /app/server


##########################################
## 2️⃣ Runtime Stage (minimal, secure)   ##
##########################################

FROM alpine:3.18

# Install SSL certificates required for making HTTPS requests and wget for healthchecks.
RUN apk add --no-cache ca-certificates wget

# Create a dedicated non-root user for security.
RUN addgroup -S app && \
    adduser -S -u 1000 -G app app

# Copy the compiled binary from the builder stage.
COPY --from=build /app/server /usr/local/bin/app

# Set correct ownership and permissions.
RUN chown app:app /usr/local/bin/app

# Expose the service port.
EXPOSE 8080

# Health check to ensure the service is responsive. `wget` is used as `curl` is not in alpine.
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s \
    CMD wget -q --spider http://localhost:8080/health || exit 1

# Run as the non-root user.
USER app

# Set the entrypoint for the container.
ENTRYPOINT ["/usr/local/bin/app"]
