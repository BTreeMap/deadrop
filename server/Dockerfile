# Use the official Rust image as the builder stage
FROM rust:1-slim as builder

# Set the working directory
WORKDIR /usr/src/deadrop-server

# Copy the entire project
COPY . .

# Build the application
RUN cargo build --release

# Use a minimal image for the final stage
FROM debian:stable-slim

# Create a non-root user
RUN groupadd --gid 1000 deadrop && \
    useradd --uid 1000 --gid 1000 --shell /bin/bash --create-home deadrop

# Environment variables (see .env.example for details)
ENV SERVER_HOST=0.0.0.0 \
    SERVER_PORT=63460 \
    DATABASE_URL=postgres://user:password@localhost/deadrop \
    JWT_SECRET=EXAMPLE_v7BFjiX/aDP5i2fThhbfxKuy00SaFPV6qBQ7DxxqEX0xola2O8oOSxdC \
    JWT_EXPIRATION_SECONDS=300

# Copy the built binary from the builder stage
COPY --from=builder /usr/src/deadrop-server/target/release/deadrop-server /opt/deadrop-server

# Set the user
USER deadrop

# Expose the server port (unprivileged)
EXPOSE 63460

# Set the entrypoint
ENTRYPOINT ["/opt/deadrop-server"]
