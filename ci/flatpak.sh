#!/bin/sh
# Build (and lint) the Bigify Flatpak inside Docker. Produces
# dist/bigify.flatpak, an installable single-file bundle.
#   docker build -t bigify-flatpak -f ci/flatpak.Dockerfile .
#   ci/flatpak.sh
set -e
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
if command -v cygpath >/dev/null 2>&1; then ROOT="$(cygpath -w "$ROOT")"; fi
exec env MSYS_NO_PATHCONV=1 docker run --rm --privileged \
  -v "$ROOT:/src" \
  -v bigify-flatpak-repo:/var/lib/flatpak \
  bigify-flatpak sh -euc '
    appstreamcli validate --no-net \
      packaging/linux/io.github.bigify.Bigify.metainfo.xml
    python3 /usr/local/bin/flatpak-cargo-generator.py Cargo.lock \
      -o packaging/flatpak/cargo-sources.json
    # State dir, build dir and repo share the cached volume, so they are on
    # one filesystem (flatpak-builder requires that) and survive reruns.
    W=/var/lib/flatpak/bigify-work
    flatpak-builder --system --install-deps-from=flathub --force-clean \
      --disable-rofiles-fuse --state-dir="$W/state" --repo="$W/repo" \
      "$W/build" packaging/flatpak/io.github.bigify.Bigify.yml
    mkdir -p dist
    flatpak build-bundle "$W/repo" dist/bigify.flatpak io.github.bigify.Bigify
    ls -la dist/bigify.flatpak
  '
