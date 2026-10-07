# Dockerfile for building and running snx-rs (Check Point VPN client)
# Supports multiple base images: alpine (default), archlinux
# Build: docker build -t snx-rs .
# Build for Arch: docker build -t snx-rs --build-arg BASE=archlinux .

# Build stage - static binary
# Use Rust 1.92 to satisfy dependency requirements (aes 0.9.3, cached 4.0.1, uuid 1.27.0)
FROM rust:1.92-slim AS builder

# Install musl tools and musl target for static linking
RUN apt-get update && apt-get install -y --no-install-recommends \
    musl-tools \
    && rustup target add x86_64-unknown-linux-musl \
    && rm -rf /var/lib/apt/lists/*

# Copy the entire repository
WORKDIR /usr/src/snx-rs
COPY . .

# Build static binary with vendored dependencies
RUN cargo build --target x86_64-unknown-linux-musl \
    --features snxcore/vendored-openssl,snxcore/vendored-sqlite \
    -p snx-rs \
    --profile lto

# Runtime stage - configurable base
ARG BASE=alpine
FROM ${BASE}:latest

# Install runtime dependencies based on base image
# Alpine uses apk, Arch uses pacman
RUN if [ -f /etc/alpine-release ]; then \
        apk add --no-cache iproute2 openssl ca-certificates bash sudo; \
    elif [ -f /etc/arch-release ]; then \
        pacman -Syu --noconfirm --needed iproute2 openssl ca-certificates bash sudo; \
    fi

# Create a non-root user to run snx-rs
RUN if [ -f /etc/alpine-release ]; then \
        adduser -D -u 1000 snxuser; \
    else \
        useradd -m -u 1000 snxuser; \
    fi

# Copy the static binary from the builder stage
COPY --from=builder /usr/src/snx-rs/target/x86_64-unknown-linux-musl/lto/snx-rs /usr/local/bin/snx-rs

# Set up entry point
WORKDIR /home/snxuser
USER snxuser

# Create a directory for configuration
RUN mkdir -p /home/snxuser/.config/snx-rs

# The container can be run with a command like:
# docker run -it --rm --net=host --cap-add=NET_ADMIN --cap-add=NET_RAW snx-rs snx-rs connect -s vpn.example.com -u username
# Or with a volume for config:
# docker run -it --rm --net=host --cap-add=NET_ADMIN --cap-add=NET_RAW -v $PWD/config:/home/snxuser/.config/snx-rs snx-rs snx-rs <command>

ENTRYPOINT ["snx-rs"]
CMD ["--help"]
