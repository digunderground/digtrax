# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

DigTrax is a cross-platform music metadata tagger aimed at DJs. Originally a fork of OneTagger by Marekkon5, it's been rebranded with a full V4 UI redesign. It fetches metadata from Beatport, Traxsource, Juno Download, Discogs, MusicBrainz, and Spotify; supports MP3/AIFF/FLAC/M4A; ships a manual tag editor and a keyboard-driven Quick Tag editor. Frontend is Vue 3 + Quasar, backend is Rust.

The repository directory is still named `onetagger` for now (matches GitHub remote); the *binaries*, *crate names*, *bundle*, and *config dir* have all been renamed to DigTrax. See `plan/00.5-digtrax-rename.md` for the full rename history.

## Critical build invariant: build the frontend before cargo

`digtrax-ui` embeds the compiled frontend at compile time:

```rust
// crates/digtrax-ui/src/lib.rs
static CLIENT_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../client/dist");
```

If `client/dist/` is empty or stale, the resulting binary will serve a stale UI (or fail to build). **Always run the frontend build first**, in this order:

```sh
cd client && pnpm i && pnpm run build && cd ..
cargo build --release
```

When you change anything under [client/src/](client/src/), rebuild `client/dist/` then `cargo build` again — there is no auto-trigger.

## Build commands (verified on macOS, May 2026)

```sh
# 1. Frontend
cd client && pnpm i && pnpm run build && cd ..

# 2. Rust workspace (release builds digtrax + digtrax-cli + all libs)
cargo build --release

# 3. macOS-only: produce DigTrax.app via cargo-bundle
#    First time: cargo install cargo-bundle --version 0.6.1 --locked
cargo bundle --release
# -> target/release/bundle/osx/DigTrax.app

# Linux extras (Debian/Ubuntu):
sudo apt install -y lld autogen libasound2-dev pkg-config make libssl-dev \
                    gcc g++ curl wget git libwebkit2gtk-4.1-dev
```

CI ([.github/workflows/build.yml](.github/workflows/build.yml)) runs `cargo update` before each build.

### Universal macOS build (Apple Silicon + Intel)

The CI Mac job builds both targets and uses `lipo` to produce a universal binary. Local recipe:

```sh
rustup target add x86_64-apple-darwin aarch64-apple-darwin
cd client && pnpm run build && cd ..
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin
lipo -create -output target/release/digtrax \
  target/aarch64-apple-darwin/release/digtrax \
  target/x86_64-apple-darwin/release/digtrax
cargo bundle --release
lipo -create -output target/release/bundle/osx/DigTrax.app/Contents/MacOS/digtrax \
  target/aarch64-apple-darwin/release/digtrax \
  target/x86_64-apple-darwin/release/digtrax
codesign --force --deep -s - target/release/bundle/osx/DigTrax.app
```

## Run modes

```sh
target/release/digtrax              # GUI (wry webview + embedded server)
target/release/digtrax -S           # server-only (browser to http://127.0.0.1:36913)
target/release/digtrax --browser    # auto-open in default browser
target/release/digtrax-cli --help   # headless CLI
```

Default port is `PORT = 36913` in [crates/digtrax-shared/src/lib.rs](crates/digtrax-shared/src/lib.rs).

## Architecture: how the GUI app actually runs

The GUI binary is a webview shell over an embedded HTTP/WebSocket server. From [crates/digtrax/src/main.rs](crates/digtrax/src/main.rs):

1. `digtrax_shared::setup()` — initialize logger and load `Settings` from the user's config dir (resolved via `directories`).
2. Spawn `digtrax_ui::start_all(ctx)` on a background OS thread — Tokio multi-thread runtime + Axum router on `127.0.0.1:36913` (or `0.0.0.0:36913` with `--expose`).
3. On the main thread, build a `tao` event loop + `wry` `WebViewBuilder` pointed at the local server. The webview must run on the main thread (macOS requirement).
4. Server mode (`-S`) skips the webview entirely.

Axum router (see [crates/digtrax-ui/src/lib.rs](crates/digtrax-ui/src/lib.rs)):

| Route | Handler |
|---|---|
| `/ws` | primary WebSocket transport for all UI ↔ backend RPC |
| `/audio` | streams audio for the player |
| `/thumb` | cover-art thumbnails (320×320 JPEG) |
| `/spotify` | OAuth callback page |
| `/` and `/{*path}` | serve `client/dist/` from the embedded `CLIENT_DIR` |

**The WebSocket is the actual API surface.** When tracing a UI feature, start at the matching `socket.rs` handler.

## Workspace layout

11 Rust crates under [crates/](crates/), defined in the workspace root [Cargo.toml](Cargo.toml). Dependency graph (top → bottom = depends on):

```
digtrax (GUI bin)         digtrax-cli (CLI bin)
    │                          │
    └──────► digtrax-ui ◄──────┘   (axum server, embeds client/dist)
                  │
   ┌──────────────┼──────────────┬──────────────┬───────────────┐
   ▼              ▼              ▼              ▼               ▼
digtrax-     digtrax-      digtrax-      digtrax-          digtrax-
 autotag      platforms      player        playlist          renamer
   │              │              │              │               │
   └──────────────┴──► digtrax-tagger ◄─────────┘               │
                              │                                  │
                              └──► digtrax-tag ◄─────────────────┘
                                          │
                                          └──► digtrax-shared
```

## Frontend

Vue 3 + Quasar 2 + Vite 6 + TypeScript, built with `vue-tsc --noEmit && vite build`. Sources in [client/src/](client/src/), output to `client/dist/`. Package.json deps are exact-pinned — bumping majors has been actively painful in the past, so keep upgrades within existing semver ranges.

V4 design system in [client/src/style/tokens.scss](client/src/style/tokens.scss); spec in [DESIGN.md](DESIGN.md).

The TypeScript central store class is still named `OneTagger` in [client/src/scripts/onetagger.ts](client/src/scripts/onetagger.ts) — internal-only, deferred from the rename.

## Platform notes

- **macOS**: workspace pins `lzma-sys = { version = "*", features = ["static"] }` in the root [Cargo.toml](Cargo.toml). macOS-specific deps in [crates/digtrax/Cargo.toml](crates/digtrax/Cargo.toml): `muda` (menu bar), `native-dialog` (file pickers). Bundle metadata under `[package.metadata.bundle]` — name "DigTrax", identifier `com.digtrax.app`.
- **Windows**: NSIS installer at [assets/installer.nsi](assets/installer.nsi); icon resources baked in via `winres` build-deps.
- **Linux**: WebKit GTK 4.1 dev headers required.

## Config dir migration

`Settings::get_folder()` in [crates/digtrax-shared/src/lib.rs](crates/digtrax-shared/src/lib.rs) checks for an existing OneTagger config dir on first launch and copies it to the new DigTrax location. Idempotent — runs only when the DigTrax dir doesn't exist yet.

- Old: `~/Library/Application Support/com.OneTagger.OneTagger/` (macOS) etc.
- New: `~/Library/Application Support/com.DigTrax.DigTrax/` (macOS) etc.

Existing users carry over: settings.json, custom platforms, auth tokens, log file location.

## Tests

There is **no test suite** (`cargo test` produces zero tests across all 11 crates as of this writing). When adding features, manual verification through the GUI or `digtrax-cli` is the standard.
