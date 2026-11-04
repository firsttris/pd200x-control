#!/bin/sh
# Regenerates the offline sources for flatpak/io.github.firsttris.pd200x-control.yml from Cargo.lock
# and app/bun.lock. Run it whenever either lock file changes; the Flatpak workflow checks that the
# committed files match.
#
# Needs Bun and, for Cargo.lock, a checkout of flatpak-builder-tools
# (https://github.com/flatpak/flatpak-builder-tools) in FBT and a python3 with its cargo
# generator's dependencies (pip install aiohttp tomlkit). flatpak-builder-tools has no generator
# for bun.lock, so node-sources.json comes from flatpak/node-sources.ts.
set -eu
cd "$(dirname "$0")/.."
: "${FBT:?set FBT to a flatpak-builder-tools checkout}"

python3 "$FBT/cargo/flatpak-cargo-generator.py" -o flatpak/cargo-sources.json Cargo.lock
bun flatpak/node-sources.ts
