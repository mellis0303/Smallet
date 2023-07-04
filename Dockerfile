FROM rust:latest

WORKDIR /app

# Copy the project files into the image
COPY . .

# Build and install the Rust project
RUN cargo build --release

# Specify the entry point or CMD as needed