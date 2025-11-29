FROM docker.io/library/rust:1.91.1-trixie AS builder

RUN apt-get update && apt-get install -y libdbus-1-3 libdbus-1-dev

COPY --parents Cargo.lock Cargo.toml src /root/app/
WORKDIR /root/app
RUN cargo build --release

# ----

FROM docker.io/library/debian:13.2-slim
LABEL org.opencontainers.image.source=https://github.com/graipher/ruuvi-prometheus-rs
LABEL org.opencontainers.image.licenses=MIT

ARG USERNAME=ruuvi
ARG USER_UID=1000
ARG USER_GID=$USER_UID

RUN groupadd --gid $USER_GID $USERNAME \
    && useradd --uid $USER_UID --gid $USER_GID -m $USERNAME

RUN apt-get update && apt-get install -y --no-install-recommends libdbus-1-3
COPY --from=builder /root/app/target/release/ruuvi-prometheus-rs /usr/bin/ruuvi-prometheus-rs

ENV PORT="9185"
ENV ENABLE_PROCESS_COLLECTION="false"
ENV IDLE_TIMEOUT="60s"
ENV BLUETOOTH_DEVICE="hci0"

EXPOSE 9185

USER ruuvi

ENTRYPOINT ["ruuvi-prometheus-rs"]
