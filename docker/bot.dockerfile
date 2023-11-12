# Use Ubuntu as the base image
FROM ubuntu:20.04

# Avoid prompts from apt
ENV DEBIAN_FRONTEND=noninteractive

# Update and install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    gcc \
    pkg-config \
    libssl-dev \
    # Add the PostgreSQL client library
    libpq-dev

# Install Rust (use a specific version or latest stable)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Set the working directory in the container to /app
WORKDIR /app

# Copy the rest of your Rust app's source code into the Docker image
COPY . .

# Make the data folder persistent.
VOLUME ["/app/data"]

# Build your application
RUN cargo build --release

# Set the command to run your application when the docker container starts
CMD ["./target/release/trading_bot"]
