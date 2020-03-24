FROM rust:1.42 as builder
WORKDIR /usr/src/seabird-rs

ENV CARGO_TARGET_DIR=/tmp/seabird-target

# Copy over only the files which specify dependencies
COPY Cargo.toml Cargo.lock ./

# We need to create a dummy main in order to get this to properly build.
RUN mkdir src /tmp/seabird-target && echo 'fn main() {}' > src/main.rs && cargo build --release

# Copy over the files to actually build the application.
COPY src src
RUN cargo install --path .

FROM debian:buster-slim
COPY --from=builder /usr/local/cargo/bin/seabird /usr/local/bin/seabird
ENV RUST_LOG=info
RUN apt-get update && apt-get install -y libssl1.1 ca-certificates && rm -rf /var/lib/apt/lists/*
CMD ["seabird"]
