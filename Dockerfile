FROM rust:1.90.0@sha256:976303ceda00c5f21d6fe97500927285c7e0f6a2e8df71ae18a6c8e9b37550a1

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
