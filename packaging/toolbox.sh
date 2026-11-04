#!/usr/bin/env bash
# Creates a Fedora toolbox with everything needed to build pd200x-control, so
# the host system (e.g. Bazzite, Silverblue) stays untouched.
set -euo pipefail
name="${1:-pd200x-dev}"
toolbox create -y "$name" 2>/dev/null || true
toolbox run -c "$name" sudo dnf install -y \
  webkit2gtk4.1-devel openssl-devel curl wget file libayatana-appindicator-gtk3-devel librsvg2-devel \
  gcc gcc-c++ make patchelf unzip git just pipewire-utils rpm-build
toolbox run -c "$name" bash -c 'command -v cargo || curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y'
# Bun, the version in app/package.json's packageManager (what CI runs), read without jq or node;
# installed unless that version is there already (the official installer takes the release tag
# and needs unzip)
bun_version=$(sed -n 's/.*"packageManager": *"bun@\([0-9][^"]*\)".*/\1/p' "$(dirname "$0")/../app/package.json")
[[ -n "$bun_version" ]] || { echo "No Bun version in app/package.json (packageManager)." >&2; exit 1; }
printf -v install_bun '[ "$(PATH="$HOME/.bun/bin:$PATH" bun --version 2>/dev/null)" = %q ] || curl -fsSL https://bun.sh/install | bash -s %q' \
  "$bun_version" "bun-v$bun_version"
toolbox run -c "$name" bash -c "$install_bun"
echo "Done. Enter it with 'toolbox enter $name', then 'just setup && just'."
