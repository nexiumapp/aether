FROM rust:1.53 as base

# Add additional rust components.
RUN rustup component add rustfmt

# Create filler projects.
RUN cargo new aether

# Switch workdir.
WORKDIR /aether

# Copy over manifest files.
COPY ./Cargo.toml ./Cargo.toml

# Build dependency for release.
RUN cargo build --release

# Copy over the rest of the files.
COPY ./src ./src

# Remove dependencies.
RUN rm ./target/release/deps/aether*

# Build it all for release.
RUN cargo build --release

# Build from the image.
FROM debian:buster

# Copy the binary from the base image.
COPY --from=base /aether/target/release/aether .

# Port to serve on.
EXPOSE 8000

# Start the binary.
CMD ["./aether"]