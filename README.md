<h1 align='center'>DigTrax</h1>
<h3 align='center'>The cross-platform music tagger for DJs</h3>

<p align='center'>
    <img alt='Supported OS' src='https://img.shields.io/badge/OS-Windows%2C%20Mac%20OS%2C%20Linux-orange'>
</p>

DigTrax is a cross-platform music metadata tagger built for DJs. It fetches metadata from Beatport, Traxsource, Juno Download, Discogs, MusicBrainz, and Spotify; supports a manual editor and a keyboard-driven Quick Tag editor with energy / mood / genre / custom-tag bindings; and writes back to MP3, AIFF, FLAC, and M4A (AAC, ALAC) files.

Originally a fork of [OneTagger](https://github.com/Marekkon5/onetagger) by Marekkon5 — UI design originally by Bas Curtiz. DigTrax adds a redesigned UI, universal macOS builds, and a streamlined Quick Tag workflow.

## Installing

Download the latest binaries for your platform from the [Releases](../../releases) page:
- **macOS**: `DigTrax-mac.zip` — universal binary (Apple Silicon + Intel)
- **Windows**: `DigTrax-windows-setup.exe` — installer
- **Linux**: `DigTrax-linux.tar.gz` — extracted binary

## Compiling

### Linux & macOS

Install dependencies: [rustup](https://rustup.rs), [Node.js](https://nodejs.org/), [pnpm](https://pnpm.io/installation).

On Linux also install:
```sh
sudo apt install -y lld autogen libasound2-dev pkg-config make libssl-dev gcc g++ curl wget git libwebkit2gtk-4.1-dev
```

Compile the UI:
```sh
cd client && pnpm i && pnpm run build && cd ..
```

Compile the app:
```sh
cargo build --release
```

Output: `target/release/digtrax` (and `target/release/digtrax-cli`).

### Universal macOS build (Apple Silicon + Intel)

```sh
rustup target add x86_64-apple-darwin aarch64-apple-darwin
cd client && pnpm i && pnpm run build && cd ..
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

Output: `target/release/bundle/osx/DigTrax.app` — runs natively on both architectures.

### Windows

Install dependencies: [rustup](https://rustup.rs), [Node.js](https://nodejs.org/), [Visual Studio 2019 Build Tools](https://aka.ms/vs/16/release/vs_buildtools.exe), [pnpm](https://pnpm.io/installation).

```sh
cd client && pnpm i && pnpm run build && cd ..
cargo build --release
```

Output: `target\release\digtrax.exe`.

## Migration from OneTagger

DigTrax automatically migrates user settings from any existing OneTagger install on first launch — your folder paths, mood/energy bindings, custom tag definitions, and platform auth tokens carry over. The original OneTagger config dir is left untouched as a backup; the new DigTrax config dir is created alongside it.

## Credits

- **Marekkon5** — Original OneTagger
- **Bas Curtiz** — Original UI design
- **SongRec** (Shazam support) — https://github.com/marin-m/SongRec
