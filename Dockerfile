# Dockerfile for ViaBTC Trading Engine

# Builder stage
FROM rust:1.76-slim-bullseye as builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev libpq-dev && \
    rm -rf /var/lib/apt/lists/*

# Create a new empty project
WORKDIR /app
RUN USER=root cargo new --bin viabtc-rs
WORKDIR /app/viabtc-rs

# Copy Cargo configuration
COPY Cargo.toml ./
COPY */Cargo.toml */Cargo.toml

# Create dummy source files for each crate
RUN mkdir -p src
RUN for d in $(ls -d */); do \
      mkdir -p "${d}src"; \
      echo 'fn main() { println!("Dummy!"); }' > "${d}src/lib.rs"; \
    done
RUN echo 'fn main() { println!("Dummy!"); }' > src/main.rs

# Build dependencies
RUN cargo build --release

# Remove the dummy source files
RUN rm -rf */src
RUN rm -rf src

# Copy the actual source code
COPY . .

# Build the real application
RUN cargo build --release

# Runtime stage
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y libssl-dev libpq-dev ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built binary
COPY --from=builder /app/viabtc-rs/target/release/trading-engine /app/trading-engine

# Expose API port
EXPOSE 8080

# Set environment variables
ENV RUST_LOG=info

# Run the application
CMD ["/app/trading-engine", "--demo"]

