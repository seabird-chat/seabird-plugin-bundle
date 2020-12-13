FROM rust:1.48 as builder
WORKDIR /usr/src/seabird-rs

# NOTE: tonic_build uses rustfmt to properly format the output files and give
# better errors.
RUN rustup component add rustfmt

RUN cargo install --version=0.2.0 sqlx-cli

# Copy over only the files which specify dependencies
COPY Cargo.toml Cargo.lock ./

# We need to create a dummy main in order to get this to properly build.
RUN mkdir src && echo 'fn main() {}' > src/main.rs && cargo build --release

# Copy over the files to actually build the application.
COPY . .

# We need to make sure the update time on main.rs is newer than the temporary
# file or there are weird cargo caching issues we run into.
RUN touch src/main.rs && cargo build --release && cp -v target/release/seabird-plugin-bundle /usr/local/bin

# Create a new base and copy in only what we need.
FROM debian:buster-slim
ENV RUST_LOG=info
WORKDIR /usr/src/seabird-rs
RUN apt-get update && apt-get install -y libssl1.1 ca-certificates && rm -rf /var/lib/apt/lists/* && mkdir migrations
COPY --from=builder /usr/local/cargo/bin/sqlx /usr/local/bin/sqlx
COPY --from=builder /usr/local/cargo/bin/cargo-sqlx /usr/local/bin/cargo-sqlx
COPY --from=builder /usr/local/bin/seabird-plugin-bundle /usr/local/bin/seabird-plugin-bundle
COPY --from=builder /usr/src/seabird-rs/migrations/* /usr/src/seabird-rs/migrations/
CMD ["seabird-plugin-bundle"]
