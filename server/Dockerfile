# Use the official Rust image as the builder stage
FROM rust:1-slim as builder

# Set the working directory
WORKDIR /usr/src/deadrop-server

# Copy the Cargo manifest and lock file
COPY Cargo.toml Cargo.lock ./

# Build dependencies first to leverage Docker cache
# Create a dummy src/main.rs to build only dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release

# Copy the actual source code
COPY src ./src

# Build the application
# Remove dummy main.rs and build the actual project
RUN rm -f src/main.rs
RUN cargo build --release

# Use a minimal image for the final stage
FROM debian:stable-slim

# Create a non-root user
RUN groupadd --gid 1000 appuser && \
    useradd --uid 1000 --gid 1000 --shell /bin/bash --create-home appuser

# Copy the built binary from the builder stage
COPY --from=builder /usr/src/deadrop-server/target/release/deadrop-server /usr/local/bin/deadrop-server

# Set the user
USER appuser

# Expose the server port (unprivileged)
EXPOSE 8080

# Set the entrypoint
ENTRYPOINT ["deadrop-server"]
