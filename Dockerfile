FROM ghcr.io/famedly/rust-container:nightly as builder
ARG CARGO_NET_GIT_FETCH_WITH_CLI=true
ARG CARGO_BUILD_RUSTFLAGS
ARG CI_SSH_PRIVATE_KEY

# Add CI key for git dependencies in Cargo.toml. This is only done in the builder stage, so the key
# is not available in the final container.
RUN mkdir -p ~/.ssh
RUN echo "${CI_SSH_PRIVATE_KEY}" > ~/.ssh/id_ed25519
RUN chmod 600 ~/.ssh/id_ed25519
RUN echo "Host *\n\tStrictHostKeyChecking no\n\n" > ~/.ssh/config

COPY . /app
WORKDIR /app
RUN cargo auditable build --release

FROM debian:bullseye-slim
RUN apt update && apt install -y ca-certificates
RUN mkdir -p /opt/hookd
WORKDIR /opt/hookd
COPY --from=builder /app/target/release/hookd /usr/local/bin/hookd
CMD ["/usr/local/bin/hookd"]

