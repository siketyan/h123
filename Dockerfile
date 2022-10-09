# syntax = docker/dockerfile:1.4

FROM rust:1.64 AS base
SHELL ["/bin/bash", "-c"]
WORKDIR /src

RUN cargo install cargo-chef

# ---

FROM base as planner
COPY . .
RUN cargo chef prepare --recipe-path /recipe.json

# ---

FROM base as builder
SHELL ["/bin/bash", "-c"]

COPY --from=planner /recipe.json .
RUN cargo chef cook --release --recipe-path recipe.json --bin h123

COPY . .
RUN cargo build --release --bin h123

# ---

FROM gcr.io/distroless/cc
COPY --from=builder /src/target/release/h123 /bin/h123
COPY --from=builder /src/*.pem /
ENTRYPOINT ["/bin/h123"]
CMD ["--cert-chain-pem", "/cert.pem", "--private-key-pem", "/privkey.pem", "-d", "/htdocs", "-b", "0.0.0.0:443"]
