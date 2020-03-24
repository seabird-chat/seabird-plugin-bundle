FROM rust:1.42 as builder
WORKDIR /usr/src/seabird-rs
COPY . .
RUN cargo install --path .

FROM debian:buster-slim
COPY --from=builder /usr/local/cargo/bin/seabird /usr/local/bin/seabird
ENV RUST_LOG=info
RUN apt-get update && apt-get install -y libssl1.1 && rm -rf /var/lib/apt/lists/*
CMD ["seabird"]
