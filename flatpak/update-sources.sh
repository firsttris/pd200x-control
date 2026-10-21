#!/bin/sh
# Regenerates the offline sources for flatpak/io.github.firsttris.pd200x-control.yml from Cargo.lock
# and app/pnpm-lock.yaml. Run it whenever either lock file changes; the Flatpak workflow checks
# that the committed files match.
#
# Needs a checkout of flatpak-builder-tools (https://github.com/flatpak/flatpak-builder-tools) in
# FBT and a python3 with the generators' dependencies (pip install "$FBT/node" aiohttp tomlkit
# PyYAML).
set -eu
cd "$(dirname "$0")/.."
: "${FBT:?set FBT to a flatpak-builder-tools checkout}"

python3 "$FBT/cargo/flatpak-cargo-generator.py" -o flatpak/cargo-sources.json Cargo.lock

# The node generator adds Playwright's browsers for @playwright/test (special.py,
# _handle_playwright), hundreds of MB that only the e2e tests use, and its --no-devel does not work
# for pnpm's lockfile v9 yet. So that one handler is switched off.
PYTHONPATH="$FBT/node" python3 - <<'PY'
import sys

from flatpak_node_generator.main import main
from flatpak_node_generator.providers.special import SpecialSourceProvider


async def no_browsers(self, package):
    pass


SpecialSourceProvider._handle_playwright = no_browsers
sys.argv = ["flatpak-node-generator", "--no-requests-cache", "-o", "flatpak/node-sources.json",
            "pnpm", "app/pnpm-lock.yaml"]
main()
PY
