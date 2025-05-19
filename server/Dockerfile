# Template Dockerfile
# We are using multi-stage builds to reduce the size of the final image

# 1. Phase: The build phase
FROM rust:slim-bullseye AS builder

# 2. Installing the necessary build dependencies
RUN apt-get update -y && \
	apt-get install -y pkg-config make g++ libssl-dev protobuf-compiler && \
	rustup target add x86_64-unknown-linux-gnu

RUN rustup install nightly
RUN rustup default nightly

ARG SERVICE_NAME

# 3. Cargo cache előkészítése
WORKDIR /usr/src/server

COPY Cargo.toml ./
COPY Rocket.toml ./
RUN mkdir src && echo "fn main() {}" > src/main.rs

# 5. Függőségek letöltése (ha a Cargo.lock nem változott, akkor cache-ből épül)
RUN cargo build --release && rm -rf src

# Copying the whole project to the container
COPY / /usr/src/server

# 3. Setting the working directory
WORKDIR /usr/src/server

# RUN cargo fetch

# 6. Building the application
RUN cargo build --release 

# 7. Phase: The runtime environment
FROM debian:bullseye-slim

# 8. Installing the necessary runtime dependencies
RUN apt-get update && apt-get install -y \
	libssl-dev \
	ca-certificates \
	&& rm -rf /var/lib/apt/lists/*

ARG SERVICE_NAME

# 9. Copying the built binary to the new container
COPY --from=builder /usr/src/server/target/release/server /usr/local/bin/my_binary

ARG SERVICE_NAME

# 10. Default command that starts the application
ENTRYPOINT ["my_binary"]