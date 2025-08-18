FROM rust:1.89.0@sha256:e090f7b4adf86191313dba91260351d7f5e15cac0fe34f26706a805c0cb9641f

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
