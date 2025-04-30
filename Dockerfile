# Build stage
FROM rust:1.86-slim as builder

WORKDIR /usr/src/app
COPY . .

# Build the application and list the built files
RUN cargo build --release && \
    ls -la /usr/src/app/target/release/

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /usr/src/app/target/release/codecrafters-http-server /app/http-server

# Verify the binary exists and is executable
RUN ls -la /app/http-server && \
    chmod +x /app/http-server

# Create a directory for serving files
RUN mkdir -p /app/files

# Expose the port the server listens on
EXPOSE 4221

# Set the default directory for serving files
ENV DIRECTORY=/app/files

# Run the server with --directory flag
CMD ["/app/http-server", "--directory", "."]