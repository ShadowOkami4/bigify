#!/bin/sh
# Run a command inside the Bigify Linux build container.
# Usage: ci/run.sh cargo test --workspace
set -e
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
if command -v cygpath >/dev/null 2>&1; then ROOT="$(cygpath -w "$ROOT")"; fi
exec env MSYS_NO_PATHCONV=1 docker run --rm \
  -v "$ROOT:/src" \
  -v bigify-cargo:/usr/local/cargo/registry \
  -v bigify-target:/src/target \
  -e CARGO_TERM_COLOR=never \
  ${BIGIFY_DOCKER_ARGS:-} \
  bigify-dev "$@"
