#!/usr/bin/env bash
# Creates a Fedora toolbox with everything needed to build pd200x-control, so
# the host system (e.g. Bazzite, Silverblue) stays untouched.
set -euo pipefail
name="${1:-pd200x-dev}"
toolbox create -y "$name" 2>/dev/null || true
toolbox run -c "$name" sudo dnf install -y \
  webkit2gtk4.1-devel openssl-devel curl wget file libayatana-appindicator-gtk3-devel librsvg2-devel \
  gcc gcc-c++ make patchelf nodejs pnpm git just pipewire-utils rpm-build
toolbox run -c "$name" bash -c 'command -v cargo || curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y'
echo "Done. Enter it with 'toolbox enter $name', then 'just setup && just'."
