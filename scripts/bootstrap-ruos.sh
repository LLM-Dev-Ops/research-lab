#!/usr/bin/env bash
set -euo pipefail

# Idempotent prerequisites for Rust/OpenSSL builds and the offline Ruflo adapter.
# Run on the existing Ubuntu/Debian RuOS desktop; this provisions no new service.
if [[ $(id -u) == 0 ]]; then
  elevate=()
else
  elevate=(sudo -n)
fi
if ! pkg-config --exists openssl 2>/dev/null || ! ldconfig -p | grep libseccomp >/dev/null; then
  "${elevate[@]}" apt-get update -qq
  "${elevate[@]}" env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    build-essential pkg-config libssl-dev libseccomp2 python3
fi
pkg-config --modversion openssl
pkg-config --libs openssl
cargo --version
rustc --version
