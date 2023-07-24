FROM ghcr.io/cross-rs/aarch64-unknown-linux-gnu:main@sha256:b4f5bf74812f9bb6516140d4b83d1f173c2d5ce0523f3e1c2253d99d851c734f

ENV PKG_CONFIG_ALLOW_CROSS="true"

RUN set-eux; dpkg --add-architecture arm64 && \
    apt-get update && \
    apt-get install --assume-yes clang-8 libclang-8-dev binutils-aarch64-linux-gnu zlib1g-dev:arm64 unzip wget && \
    wget https://github.com/protocolbuffers/protobuf/releases/download/v21.10/protoc-21.10-linux-x86_64.zip && \
    unzip protoc-21.10-linux-x86_64.zip -d /usr/local \
