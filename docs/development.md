# Development

## Setup

Rust (stable, 1.89 or newer), Node 22 with pnpm, `just`, and for the desktop app WebKitGTK 4.1,
libayatana-appindicator and librsvg. On Fedora-based systems (also Bazzite and Silverblue) a toolbox
keeps the host untouched:

```sh
git clone https://github.com/firsttris/pd200x-control && cd pd200x-control
bash packaging/toolbox.sh           # creates the toolbox "pd200x-dev"
toolbox enter pd200x-dev
just setup                          # pnpm install
just                                # lint and every test
```

On Debian/Ubuntu: `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev
libayatana-appindicator3-dev pipewire-bin`.

## `just` recipes

| Recipe | |
|---|---|
| `just dev` | the desktop app with hot reload, against the real microphone |
| `just dev-sim` | the same against a simulated microphone (`PD200X_SIMULATOR=1`) |
| `just dev-mock` | only the frontend in the browser, against the in-memory mock backend |
| `just lint` | `cargo fmt --check`, clippy with `-D warnings`, Biome, TypeScript |
| `just test` | Rust tests, Vitest with coverage, Playwright |
| `just bindings` | regenerate `app/src/lib/bindings.ts` after changing a Tauri command |
| `just screenshots` | the pictures in `docs/` from the mock backend's demo data |
| `just docs` | this documentation on http://127.0.0.1:8000 |
| `just build` / `just install` | CLI and AppImage; `install` puts them into `~/.local/bin` with a desktop entry |

## Layout

| Path | |
|---|---|
| `crates/core` | protocol, device, level meter, presets, PipeWire, calibration, recording, settings, simulator |
| `crates/cli` | the `pd200x` command |
| `app/src-tauri` | Tauri commands (`commands.rs`), the app logic over the core (`hub.rs`), the tray |
| `app/src` | React frontend: `routes/` (pages), `components/`, `lib/` (bindings, queries, EQ math, mock backend) |
| `app/messages` | the German and English texts (Paraglide JS) |
| `packaging` | udev rule, desktop entry, icon, toolbox script |
| `flatpak` | the Flatpak manifest, its desktop entry and metainfo, the offline sources |
| `docs` | this documentation and its pictures |

How the pieces fit is on [How it works](how-it-works.md).

## Tests

- **Rust**: the protocol, value encodings, presets and preset detection, calibration, WAV analysis
  and settings as unit tests; the device against the simulator (`sim.rs`, the same `Transport` trait
  as the real hidraw device); the CLI end to end against the simulator with its own config folder;
  the app's hub with a fake PipeWire and fake `pw-record`/`pw-play`.
- **Vitest**: the EQ curve against known biquad responses, number formatting, the optimistic
  snapshot updates, the mock backend, and every page clicked through against the mock backend.
- **Playwright**: the frontend in Chromium (and WebKit in CI, like Tauri's WebKitGTK) against the
  mock backend: level meter, dragging sliders and EQ points, the level finder, profiles, light,
  the test recording, English and German.
- **The bindings test** fails when `bindings.ts` no longer matches the Rust commands.

The mock backend (`app/src/lib/mock/backend.ts`) must behave like `hub.rs`: what one does, the other
does too. Presets, defaults and limits come from Rust as constants in the bindings, so neither side
repeats them.

## Translations

Every text the UI shows is in `app/messages/de.json` and `app/messages/en.json`; components call
`m.key()`. A test fails when the two files do not have the same keys. Numbers are formatted for the
UI language (`−6,4 dB` / `−6.4 dB`).

## Screenshots

The pictures in `docs/` come from `app/scripts/screenshots.mjs`: the built frontend against the mock
backend, English UI, a fixed clock. After a change to the look run **Actions → Update screenshots →
Run workflow**; it takes them in the official Playwright image and commits the ones that changed.
`just screenshots` does the same locally.

## Flatpak

pd200x-control is also built as a Flatpak, `io.github.firsttris.pd200x-control`, on the GNOME 51
runtime, the same way as [Prefixr](https://github.com/firsttris/prefixr):

```sh
git submodule update --init flatpak/shared-modules
flatpak run org.flatpak.Builder --user --install --force-clean --disable-rofiles-fuse \
  flatpak/build-dir flatpak/io.github.firsttris.pd200x-control.yml
```

(`flatpak-builder` instead of `flatpak run org.flatpak.Builder` where it is installed on the host.)
The manifest builds from the checkout without network access, as Flathub requires:

- `flatpak/cargo-sources.json` (from `Cargo.lock`) and `flatpak/node-sources.json` (from
  `app/pnpm-lock.yaml`) hold every download. After changing either lock file, run
  `flatpak/update-sources.sh` (with [flatpak-builder-tools](https://github.com/flatpak/flatpak-builder-tools)
  in `FBT`) and commit the results; the **Flatpak** workflow checks that they match and builds the
  bundle on pull requests that touch the Flatpak or a lock file.
- pnpm itself comes from the npm registry as a source; Node and Rust are the SDK extensions.
- The `<release>` entry of the metainfo, which Flatpak shows as the app's version, is set from
  `Cargo.toml` when the bundle is built, since **Bump version** does not rewrite the metainfo.
- The runtime has PipeWire's tools (`pw-dump`, `pw-record`, `pw-play`) but not WirePlumber's, so
  the manifest builds WirePlumber for `wpctl`, as pwvucontrol on Flathub does.

In the sandbox the core and the app look at `FLATPAK_ID` (`crates/core/src/flatpak.rs`); the native
builds behave as before:

- The `pipewire-pulse` drop-in of "Protect from apps" goes to the host's config folder
  (`$HOST_XDG_CONFIG_HOME`, else `~/.config`, which the manifest shares); the restart is left to
  the user, since the sandbox has no `systemctl` and no way to the host's systemd.
- The single-instance D-Bus name is below the app ID (a Flatpak may only own those), the GTK
  program name is the app ID (so the desktop matches the window to the desktop file), and the tray
  icon goes to `$XDG_RUNTIME_DIR/app/<app ID>`, the one folder the host's tray can read.

Still open: a Flathub submission (it needs exceptions for `--device=all` and the
`xdg-config/pipewire/pipewire-pulse.conf.d:create` permission), and a test with the real microphone
on GNOME and KDE. The system microphone volume from the sandbox was measured against PipeWire with
WirePlumber 0.5.18 (works) and 0.4.17 (the volume does not change).

## Releases

**Actions → Bump version** raises the version in `Cargo.toml` and `app/package.json`, tags `vX.Y.Z`
and starts `release.yml`, which builds the CLI, the AppImage, the `.deb`, the `.rpm` and the
Flatpak bundle and creates the GitHub release with checksums.
