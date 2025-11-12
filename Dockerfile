# syntax=docker/dockerfile:1.3-labs

FROM rust:1.91.1-slim-trixie AS builder

RUN update-ca-certificates
ENV USER=worker
ENV UID=1001

RUN useradd \
    --no-create-home \
    --home-dir /nonexistent \
    --shell /sbin/nologin \
    --uid "${UID}" \
    "${USER}"

RUN apt-get update \
    && apt-get -y upgrade \
    && apt-get -y --no-install-recommends install pkg-config make g++ libssl-dev \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Create an empty src/main.rs, so compilation of dependencies works without errors
RUN cargo init --vcs none
COPY ./Cargo.toml ./Cargo.lock /app/
# This step compiles only our dependencies and saves them in a layer. This is the most impactful time savings
# Note the use of --mount=type=cache. On subsequent runs, we'll have the crates already downloaded
RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release

COPY ./src /app/src/
# A bit of magic here!
# * We're mounting that cache again to use during the build, otherwise it's not present and we'll have to download those again - bad!
# * Rust here is a bit fiddly, so we'll touch the files (even though we copied over them) to force a new build
RUN --mount=type=cache,target=/usr/local/cargo/registry touch /app/src/main.rs && cargo build --release

FROM gcr.io/distroless/cc-debian12

COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /app

COPY --from=builder /app/target/release/shellyplug-exporter-rs ./
USER worker:worker

EXPOSE $PORT

CMD ["/app/shellyplug-exporter-rs"]
