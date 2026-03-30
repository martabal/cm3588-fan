FROM rust:1.94.1-slim-trixie@sha256:1d0000a49fb62f4fde24455f49d59c6c088af46202d65d8f455b722f7263e8f8

WORKDIR /app

# hadolint ignore=DL3008
RUN \
  apt-get update && \
  apt-get install --no-install-recommends -y \
    gcc-aarch64-linux-gnu \
    libc6-dev-arm64-cross && \
  rustup target add aarch64-unknown-linux-gnu && \
  rm -rf /var/lib/apt/lists/*

COPY .cargo .cargo
COPY Cargo* .
COPY src src

RUN \
  cargo \
  build \
  --locked \
  --release \
  --target=aarch64-unknown-linux-gnu

ENTRYPOINT ["cp", "./target/aarch64-unknown-linux-gnu/release/cm3588-fan", "/build/cm3588-fan"]
