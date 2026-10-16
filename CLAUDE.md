# Notes for Claude

## Documentation

The documentation is in `docs/` (<https://firsttris.github.io/pd200x-control/>). Read the pages that
touch the change before starting: `how-it-works.md` for the protocol, PipeWire and what is stored
where, `development.md` for recipes, the mock backend, tests and releases, `app.md` for what every
control does. Links in the README and the docs point to the GitHub Pages site, not to `docs/*.md`.

## Reference implementations

Base details on the sources, not on memory: the USB protocol follows
[pd200x](https://github.com/advnpzn/pd200x) (advnpzn, MIT; `src/protocol.rs`, `src/packet.rs`,
`src/api.rs`, `src/presets.rs`), MAONO Link's behaviour the
[PD200X user guide](https://faq.maono.com/docs/PD200X-User-Guide). Clone them shallowly
(`git clone --depth 1`) into the scratchpad and name the source file in comments and commit messages
where it settles a detail. What the real microphone answers beats both: ask it read-only
(`pd200x status`, a query) before relying on a register.

## With every change

Update what the change affects, in the same commit, and only that:

- the tests for what changed: Rust unit/integration (`crates/*`, `app/src-tauri`), the mock backend
  and its tests (`app/src/lib/mock/`), Vitest, Playwright (`app/e2e/`);
- `app/src/lib/bindings.ts` after changing a Tauri command (`just bindings`);
- both message catalogs `app/messages/{de,en}.json` for every text;
- the docs in `docs/` that describe it;
- the README when a feature a user sees is added, changed or removed;
- the screenshots after a change to the look: **Actions → Update screenshots → Run workflow**, then
  look at the new pictures.

## Checks

Green before every push, then check CI:

```sh
just lint            # cargo fmt --check, clippy -D warnings, Biome, tsc
just test            # cargo test --workspace, Vitest with coverage, Playwright
mkdocs build --strict    # when docs/ or mkdocs.yml changed (just docs installs it)
```

The Tauri crate needs WebKitGTK headers; on the user's Bazzite host build it in the toolbox
(`packaging/toolbox.sh`). Playwright locally may need `PW_CHROMIUM_PATH` pointing to an installed
Chromium headless shell.

## Commits

- Work on `main` directly and push there; no pull requests unless the user asks for one.
- One topic per commit, with a message that says why.

## Working rules

- Commands as root (`sudo`, `pkexec`) are run by the user; don't work around that.
- The user's running processes are not stopped; use other ports for tests (Vite uses 1420, the
  screenshots 4317).
- The real microphone: reading is fine; say before writing to it (gain, light, EQ change what the
  user hears and sees).
- Language: code, comments, docs, commits in English; the app in German and English. Answer the
  user in German.

## Code style

Technology choices follow `tech-stack.md` in firsttris/workflows. Deviations of this project:

- Desktop app with Tauri 2 and a Rust core instead of a web server – no TanStack Start, axum,
  database or auth.
- Code-based TanStack Router routes (`app/src/router.tsx`): three pages.
- No Zod: data comes from typed Tauri bindings (tauri-specta, `bindings.ts`) and is validated in Rust.
- The core's `Error` implements `Display` by hand; the frontend translates `Error::kind`.
- Plain hidraw instead of hidapi: no libudev needed, Linux only like the app.
- No MkDocs page per module: the project is small, `how-it-works.md` covers the architecture.

Functional, not object-oriented: pure functions that take data and return data, side effects at the
edges (`protocol.rs`, `changes.rs`, `calibrate.rs` and the WAV analysis are pure; `device.rs`,
`transport.rs`, `pipewire.rs` and the recording processes are the edges); function components and
hooks, no classes; errors as values. A file does one thing: split it past ~400 lines, a function past
~80. A refactor changes no behaviour. Comments explain why, not what. Every bug fix comes with a test
that fails without it.

## pd200x-control specifics

- Hardware: Maono PD200X / PD200XS, USB `352f:0104`, one hidraw node. The microphone sends its level
  (`0x2034`) on its own; queries for every register are answered, but the linkage register
  (`0x2039`) reads back values that do not match what was written, so the app keeps it in the
  settings. The monitor change writes `0x2031` and `0x2070` with pauses, as MAONO Link does.
- "Microphone volume (system)" is the PipeWire source volume (cubic: 175 % = +14.6 dB), the same
  control as the desktop's sound settings; never call it something extra.
- Outside Tauri the frontend runs against the in-memory mock backend (`just dev-mock`); Vitest,
  Playwright and the screenshots use it. It must behave like `hub.rs`.
- Colours are tokens in `app/src/styles.css`, light and dark (follows the system).
- Releases start with **Actions → Bump version**; the version lives in `Cargo.toml` and
  `app/package.json`.
