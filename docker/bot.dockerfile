# Use an official Rust runtime as the parent image
FROM rust:1.72.1

# Set the working directory in the container to /app
WORKDIR /app

# Copy the rest of your Rust app's source code into the Docker image
COPY . .

# Make the data folder persistent.
VOLUME "/app/data"

# Set the command to run your application when the docker container starts
CMD ["cargo", "run", "--release", "--bin", "trading_bot"]

