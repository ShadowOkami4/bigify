# Reproducible Linux build environment for Bigify.
#
# Debian 13 (trixie) ships Qt 6.8 LTS, which is the minimum Qt version Bigify
# targets. Rust is installed via rustup so the toolchain matches
# rust-toolchain.toml rather than the distro's packaged rustc.
#
#   docker build -t bigify-dev -f ci/Dockerfile .
#   docker run --rm -v "$PWD:/src" -v bigify-cargo:/usr/local/cargo/registry bigify-dev cargo build
FROM debian:trixie

ENV DEBIAN_FRONTEND=noninteractive \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    QMAKE=/usr/bin/qmake6

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl git pkg-config build-essential clang lld \
        qt6-base-dev qt6-declarative-dev qt6-declarative-private-dev qmake6 \
        qml6-module-qtquick qml6-module-qtquick-window qml6-module-qtquick-layouts \
        qml6-module-qtquick-controls qml6-module-qtquick-templates \
        qml6-module-qtquick-effects qml6-module-qtqml-workerscript \
        qml6-module-qtqml-models qml6-module-qtquick-shapes \
        qt6-svg-dev qml6-module-qtquick-virtualkeyboard qt6-virtualkeyboard-plugin \
        libpulse-dev libasound2-dev libudev-dev \
        xvfb xauth libgl1-mesa-dri fonts-noto-core \
    && rm -rf /var/lib/apt/lists/*

COPY rust-toolchain.toml /tmp/rust-toolchain.toml
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal \
        --default-toolchain "$(grep '^channel' /tmp/rust-toolchain.toml | cut -d'"' -f2)" \
        --component clippy,rustfmt \
    && chmod -R a+w "$RUSTUP_HOME" "$CARGO_HOME"

WORKDIR /src
