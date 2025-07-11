FROM rust:1.88-bookworm AS builder
WORKDIR /usr/src/app

# Workaround to allow arm64 builds to work properly
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

# We currently use protoc rather than relying on the protobuf-sys package
# because it greatly cuts down on build times. This may change in the future.
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

RUN cargo install --version=0.8.2 sqlx-cli

# Copy over only the files which specify dependencies
COPY ./Cargo.toml ./Cargo.lock ./

# We need to create a dummy main in order to get this to properly build.
RUN mkdir src && echo 'fn main() {}' > src/main.rs && cargo build --release

# Copy over the files to actually build the application.
COPY . .

# We need to make sure the update time on main.rs is newer than the temporary
# file or there are weird cargo caching issues we run into.
RUN touch src/main.rs && cargo build --release && cp -v target/release/seabird-* /usr/local/bin

# Create a new base and copy in only what we need.
FROM debian:bookworm
ENV RUST_LOG=info
WORKDIR /usr/src/app
RUN mkdir migrations

# Extras we need for the plugin-bundle
COPY --from=builder /usr/local/cargo/bin/sqlx /usr/local/bin/sqlx
COPY --from=builder /usr/local/cargo/bin/cargo-sqlx /usr/local/bin/cargo-sqlx

COPY --from=builder /usr/local/bin/seabird-* /usr/local/bin/
COPY entrypoint.sh /usr/local/bin/seabird-entrypoint.sh
COPY --from=builder /usr/src/app/migrations/* /usr/src/app/migrations/
CMD ["/usr/local/bin/seabird-entrypoint.sh"]
