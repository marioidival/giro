FROM rust:1.85-alpine

RUN apk add --no-cache \
    git \
    bash \
    coreutils \
    curl

WORKDIR /workspace

# Binary is built from ralph-server crate but renamed for consistency with image name
COPY target/release/ralph-server /ralph-loop-manager

RUN chmod +x /ralph-loop-manager

ENTRYPOINT ["/ralph-loop-manager"]
