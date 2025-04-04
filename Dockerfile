FROM rust:1.86.0@sha256:9c1ef35ab804dc78361948794f60748e79a7a2e297580604b288590bc52ebdaa

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

RUN cargo build --release --target=aarch64-unknown-linux-gnu

ENTRYPOINT ["cp", "./target/aarch64-unknown-linux-gnu/release/cm3588-fan", "/build/cm3588-fan"]
