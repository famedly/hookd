FROM registry.gitlab.com/famedly/infra/containers/rust:main as builder
ARG CARGO_NET_GIT_FETCH_WITH_CLI=true
ARG FAMEDLY_CRATES_REGISTRY
ARG CARGO_HOME
ARG CARGO_REGISTRIES_FAMEDLY_INDEX
ARG KTRA_CARGO_TOKEN
ARG KTRA_INDEX_SSH_KEY
ARG GIT_CRATE_INDEX_USER
ARG GIT_CRATE_INDEX_PASS
ARG AWS_ACCESS_KEY_ID
ARG AWS_SECRET_ACCESS_KEY
ARG AWS_REGION
ARG CACHEPOT_BUCKET
ARG RUSTC_WRAPPER
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
RUN echo "https://${GIT_CRATE_INDEX_USER}:${GIT_CRATE_INDEX_PASS}@gitlab.com" >> ~/.git-credentials
RUN git config --global credential.helper store
RUN echo $KTRA_CARGO_TOKEN | cargo login --registry=famedly && cargo build --release

FROM debian:bullseye-slim
RUN apt update && apt install -y ca-certificates
RUN mkdir -p /opt/hookd
WORKDIR /opt/hookd
COPY --from=builder /app/target/release/hookd /usr/local/bin/hookd
CMD ["/usr/local/bin/hookd"]

