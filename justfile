# pd200x-control – build, test, install. Run `just` for lint and the full test suite.

set shell := ["bash", "-euo", "pipefail", "-c"]

bin_dir := env_var_or_default("XDG_BIN_HOME", env_var("HOME") / ".local/bin")
apps_dir := env_var("HOME") / ".local/share/applications"
icons_dir := env_var("HOME") / ".local/share/icons/hicolor/512x512/apps"

default: lint test

# Install frontend dependencies.
setup:
    cd app && pnpm install

# Start the desktop app in development mode (against the real microphone).
dev:
    cd app && pnpm tauri dev

# Start the desktop app against a simulated microphone.
dev-sim:
    cd app && PD200X_SIMULATOR=1 pnpm tauri dev

# Start only the frontend in the browser, against the in-memory mock backend.
dev-mock:
    cd app && pnpm dev:mock

# All checks that CI runs before the tests.
lint: _dist
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cd app && pnpm lint && pnpm typecheck

# Format Rust and TypeScript, apply Biome's safe fixes.
format:
    cargo fmt --all
    cd app && pnpm format

# Every test layer: Rust, Vitest, Playwright.
test: test-rust test-frontend e2e

# Rust unit and integration tests (simulated microphone, fake PipeWire).
test-rust: _dist
    cargo test --workspace

# Vitest unit and component tests with coverage thresholds.
test-frontend:
    cd app && pnpm coverage

# Playwright end-to-end tests against the mock backend.
e2e:
    cd app && pnpm e2e

# Regenerate app/src/lib/bindings.ts from the Tauri commands.
bindings: _dist
    UPDATE_BINDINGS=1 cargo test -p pd200x-control --test bindings

# Pictures in docs/ (README, documentation, banner, social preview) from the mock backend's demo data.
screenshots:
    bash scripts/screenshots.sh

# Serve the documentation (MkDocs Material) on http://127.0.0.1:8000 with live reload.
docs:
    python3 -m venv .venv-docs && .venv-docs/bin/pip install -q -r requirements-docs.txt
    .venv-docs/bin/mkdocs serve

# Release builds of the CLI and the AppImage.
build:
    cargo build --release -p pd200x-cli
    cd app && NO_STRIP=1 pnpm tauri build --bundles appimage

# Copy the CLI and the AppImage to ~/.local/bin and register the desktop entry.
install: build
    mkdir -p {{bin_dir}} {{apps_dir}} {{icons_dir}}
    install -m 755 target/release/pd200x {{bin_dir}}/pd200x
    install -m 755 "$(ls -t target/release/bundle/appimage/*.AppImage | head -1)" {{bin_dir}}/pd200x-control.AppImage
    install -m 644 app/src-tauri/icons/icon.png {{icons_dir}}/pd200x-control.png
    sed "s|@BIN_DIR@|{{bin_dir}}|" packaging/pd200x-control.desktop > {{apps_dir}}/pd200x-control.desktop
    @echo "Installed pd200x and pd200x-control.AppImage to {{bin_dir}}"
    @echo "The udev rule needs root once: sudo cp packaging/99-pd200x.rules /etc/udev/rules.d/"

# Remove what `just install` put in place (settings stay).
uninstall:
    rm -f {{bin_dir}}/pd200x {{bin_dir}}/pd200x-control.AppImage {{apps_dir}}/pd200x-control.desktop {{icons_dir}}/pd200x-control.png

# The Tauri context needs a dist folder at compile time.
_dist:
    @test -d app/dist || (mkdir -p app/dist && echo '<!doctype html>' > app/dist/index.html)
