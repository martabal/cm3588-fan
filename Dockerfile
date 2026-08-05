FROM rust:1.97.0-slim-trixie@sha256:686a437ead83701e8f871e66e838c3ec55f46b5fc235b025756396ac823bdc51

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
  cargo build \
  --locked \
  --release \
  --target=aarch64-unknown-linux-gnu

ENTRYPOINT ["cp", "./target/aarch64-unknown-linux-gnu/release/cm3588-fan", "/build/cm3588-fan"]
