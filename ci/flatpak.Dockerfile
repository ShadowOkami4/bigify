# Tooling to build the Bigify Flatpak inside Docker (run with --privileged so
# flatpak-builder's bubblewrap sandbox works).
#   docker build -t bigify-flatpak -f ci/flatpak.Dockerfile .
#   ci/flatpak.sh
FROM debian:trixie
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl git flatpak flatpak-builder appstream elfutils \
        python3 python3-aiohttp python3-tomlkit librsvg2-bin \
        librsvg2-common desktop-file-utils \
    && rm -rf /var/lib/apt/lists/*
RUN curl -sSfL -o /usr/local/bin/flatpak-cargo-generator.py \
        https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py \
    && chmod +x /usr/local/bin/flatpak-cargo-generator.py
RUN flatpak remote-add --system --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
WORKDIR /src
