FROM rustlang/rust:nightly-buster-slim

RUN USER=root apt-get update && apt-get install -y libudev-dev pkg-config

RUN USER=root cargo new --bin projectorctl
WORKDIR /projectorctl

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

SHELL ["/bin/bash", "-c"]

RUN cargo build --release
RUN rm src/*.rs & rm ./target/release/deps/projectorctl*

COPY ./src ./src

RUN cargo install --path .

CMD ["projectorctl_api"]
