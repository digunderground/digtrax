# Phase 0 — Universal macOS builds

## Context

CI currently produces an Intel-only `OneTagger-mac.zip` because [.github/workflows/build.yml:146](../.github/workflows/build.yml#L146) uses `runs-on: macos-13` — GitHub's last x86_64 macOS runner. Apple Silicon users (M1–M4) download that artifact and run it under Rosetta 2: ~20–30% slower, larger binary footprint in memory, no SIMD perf wins. The source builds arm64 perfectly fine — anyone compiling locally on Apple Silicon already gets a native binary (verified: `Mach-O 64-bit executable arm64`). This is a CI/release-pipeline issue only, not a source-code refactor.

A leftover [assets/mac-cross.toml](../assets/mac-cross.toml) exists from a previous cross-compile attempt but is not wired into the current build.

## Decision

Ship a **universal `OneTagger.app`** — one bundle that runs natively on both Intel and Apple Silicon. Trade-off accepted: ~2× binary size (≈100 MB vs ≈53 MB), one CI job runs both compiles (≈3 min instead of ≈1.5 min). Benefit: zero user confusion ("which download do I pick"), no Rosetta perf hit on M-series, no Intel users stranded.

Alternatives considered and rejected:
- **arm64-only.** Cleanest CI but cuts off the long Intel tail (2019 MBPs, Mac Pros, older Mac minis are common in DJ rigs that don't get refreshed). Not acceptable.
- **Two separate downloads.** "Pick the right one" is a known UX wart and adds release-page noise. Not worth it for a desktop app.
- **Status quo + Rosetta.** Ships today. Leaves perf and battery on the table for what's now the dominant Mac hardware.

## Scope (small — 1 PR, ~1–2 hours)

This is not a phase. It's a CI workflow change plus a small bundle post-processing step. No source code changes.

### Files touched

| File | Change |
|---|---|
| [.github/workflows/build.yml](../.github/workflows/build.yml) | Mac job: switch runner to `macos-14`, add x86_64 target install, build both targets, `lipo` into a universal binary, post-process the `.app` |
| [assets/mac-cross.toml](../assets/mac-cross.toml) | Delete — unused leftover, not referenced by anything in the build |
| [README.md](../README.md) | Update macOS build steps to mention `rustup target add x86_64-apple-darwin` for local universal builds (optional — local devs typically only build their host arch) |

### CI workflow change

Replace the Mac job (currently lines 144–207 of [build.yml](../.github/workflows/build.yml)) with:

```yaml
build-mac:
  name: Mac (Universal)
  runs-on: macos-14   # arm64 — last Intel runner (macos-13) is being retired by GitHub anyway

  steps:
    - name: Checkout
      uses: actions/checkout@v3

    - name: Rust Cache
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/bin/
          ~/.cargo/registry/index/
          ~/.cargo/registry/cache/
          ~/.cargo/git/db/
          target/
        key: ${{ runner.os }}-cargo-universal-${{ hashFiles('**/Cargo.lock') }}

    - name: Install dependencies
      run: |
        brew install pnpm
        rustup install stable
        rustup target add x86_64-apple-darwin aarch64-apple-darwin
        cargo install cargo-bundle --version 0.6.1 --locked || true

    - name: Build frontend
      run: |
        cd client
        pnpm i
        pnpm run build

    - name: Build both Rust targets
      run: |
        cargo update
        cargo build --release --target aarch64-apple-darwin
        cargo build --release --target x86_64-apple-darwin

    - name: Create universal binaries
      run: |
        mkdir -p target/release
        lipo -create -output target/release/onetagger \
          target/aarch64-apple-darwin/release/onetagger \
          target/x86_64-apple-darwin/release/onetagger
        lipo -create -output target/release/onetagger-cli \
          target/aarch64-apple-darwin/release/onetagger-cli \
          target/x86_64-apple-darwin/release/onetagger-cli

    - name: Bundle .app
      # cargo-bundle reads from target/release; we just put a universal binary there
      run: |
        cargo bundle --release
        # Replace the bundle's inner binary with the universal one (cargo-bundle
        # rebuilds against the host arch, so we overwrite after the fact)
        lipo -create -output target/release/bundle/osx/OneTagger.app/Contents/MacOS/onetagger \
          target/aarch64-apple-darwin/release/onetagger \
          target/x86_64-apple-darwin/release/onetagger
        chmod +x target/release/bundle/osx/OneTagger.app/Contents/MacOS/onetagger
        # Sanity check
        lipo -info target/release/bundle/osx/OneTagger.app/Contents/MacOS/onetagger

    - name: Ad-hoc codesign
      # Without a Developer ID we can't notarize, but ad-hoc signing is required for
      # universal binaries on macOS 11+ to load correctly without "killed: 9" on first
      # launch after un-zipping (Gatekeeper quarantine still applies — users still
      # right-click → Open the first time).
      run: |
        codesign --force --deep -s - target/release/bundle/osx/OneTagger.app
        codesign --force -s - target/release/onetagger-cli

    - name: Package
      run: |
        mkdir dist
        cd target/release/bundle/osx
        zip -r OneTagger-mac.zip .
        cd -
        cd target/release
        zip OneTagger-mac-cli.zip onetagger-cli
        cd -
        mv target/release/bundle/osx/OneTagger-mac.zip dist/
        mv target/release/OneTagger-mac-cli.zip dist/

    - name: Upload Mac
      uses: actions/upload-artifact@v4
      with:
        name: onetagger-mac
        path: dist/OneTagger-mac.zip

    - name: Upload Mac CLI
      uses: actions/upload-artifact@v4
      with:
        name: onetagger-mac-cli
        path: dist/OneTagger-mac-cli.zip
```

Key differences from the existing job:
- `runs-on: macos-13` → `macos-14` (arm64 host, native build is fast; cross to x86_64 happens via `rustup target add`).
- New step installs both Rust targets.
- Builds both `aarch64-apple-darwin` and `x86_64-apple-darwin` explicitly.
- `lipo -create` merges the per-target binaries into universal binaries.
- Bundle the `.app`, then overwrite its inner binary with the lipo'd universal version.
- Add ad-hoc codesigning (`codesign -s -`) — required on macOS 11+ for the binary to launch from a downloaded zip without "killed: 9" errors. This does **not** make the app trusted — users still see "unidentified developer" the first time. That's a separate (paid) Developer ID + notarization job, out of scope here.

### Local build instructions

Most contributors only need their host architecture and shouldn't pay the cross-compile cost on every build. Add to [README.md](../README.md) under "Linux & Mac":

> To produce a universal binary locally (matches the release):
>
> ```sh
> rustup target add x86_64-apple-darwin aarch64-apple-darwin
> cd client && pnpm i && pnpm run build && cd ..
> cargo build --release --target aarch64-apple-darwin
> cargo build --release --target x86_64-apple-darwin
> lipo -create -output target/release/onetagger \
>   target/aarch64-apple-darwin/release/onetagger \
>   target/x86_64-apple-darwin/release/onetagger
> cargo bundle --release
> lipo -create -output target/release/bundle/osx/OneTagger.app/Contents/MacOS/onetagger \
>   target/aarch64-apple-darwin/release/onetagger \
>   target/x86_64-apple-darwin/release/onetagger
> codesign --force --deep -s - target/release/bundle/osx/OneTagger.app
> ```

## Risks

- **CI build time roughly doubles.** ~3 min cold instead of ~1.5 min. Cache hit rate on `target/` will be lower because we now have two arches. Acceptable for a release pipeline that runs on push/PR; if it becomes painful we can split the two compiles into parallel matrix jobs.
- **`cargo-bundle` rebuilds in `Bundle .app` step.** It will compile a third time against the runner's host arch (arm64) before we overwrite. ~30s wasted. Worth fixing later by passing the binary path explicitly or using `tauri-bundler` which has better universal support — but tauri-bundler is a heavier dep. Keep cargo-bundle for now.
- **lzma-sys static workaround.** The workspace pin `lzma-sys = { version = "*", features = ["static"] }` ([Cargo.toml:18](../Cargo.toml#L18)) was a Mac fix; verify it still applies cleanly across both targets. Should — it's a build-script-level static link, target-agnostic. If it doesn't compile against `x86_64-apple-darwin` on the arm64 runner, we add `[target.x86_64-apple-darwin]` config in `.cargo/config.toml`. Don't pre-emptively change it; fix on signal.
- **Codesigning quirks on the runner.** `codesign -s -` (ad-hoc) requires no entitlements or keychain. Should work out of the box on `macos-14`. If it fails, the binary still ships; users just get a different Gatekeeper warning.
- **`assets/mac-cross.toml` deletion.** Verified not referenced by any other file in the repo — `grep -r mac-cross /Users/dig/code/_tools/onetagger/{Cargo.toml,.github,scripts,assets/installer.nsi}` returns nothing. Safe to remove. (Verify before deleting in the actual PR.)

## Verification

1. Push the PR. CI builds. Download the resulting `OneTagger-mac.zip`.
2. Unzip locally. Run `lipo -info OneTagger.app/Contents/MacOS/onetagger`. Expect:
   ```
   Architectures in the fat file: ... are: x86_64 arm64
   ```
3. Launch the `.app` on the Apple Silicon dev machine. Verify it runs natively (Activity Monitor shows "Apple" in the Kind column, not "Intel").
4. (If accessible) launch on an Intel Mac. Verify it runs natively (Kind: "Intel").
5. Smoke-test the CLI binary the same way: `lipo -info OneTagger-mac-cli` and a quick `--help`.

## Out of scope

- Developer ID signing + notarization. Requires a paid Apple Developer account and `APPLE_ID` / `APPLE_TEAM_ID` / `APPLE_APP_SPECIFIC_PASSWORD` secrets in GitHub Actions. Significantly improves the "unidentified developer" UX for end users. Tracked separately if/when an Apple Developer account is in scope.
- Dropping the existing Linux/Windows jobs. They stay as-is.
- ARM Linux builds (`linux/arm64`). Separate concern — different runner type, different audience.

## Acceptance

- CI produces a `OneTagger-mac.zip` whose `.app` binary reports both `x86_64` and `arm64` under `lipo -info`.
- The `.app` launches natively on at least one Apple Silicon Mac (verified by author).
- `assets/mac-cross.toml` is deleted; no build steps reference it.
- README's macOS build section is updated.
- No source-code changes outside `[.github/workflows/build.yml](../.github/workflows/build.yml)`, [README.md](../README.md), and the deletion of [assets/mac-cross.toml](../assets/mac-cross.toml).
