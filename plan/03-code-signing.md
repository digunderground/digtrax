# Plan — Code Signing & Notarization (macOS + Windows)

**Status:** deferred / low priority. Tracked here so it isn't forgotten.

## Why this matters

Without code signing, both macOS and Windows show generic "this might be malware" warnings on first launch. That's an adoption hurdle for non-technical users. The README has a workaround section so users can self-serve in the meantime.

## What it would take

### Apple side (~$99/yr + ~24–48h enrollment wait)

1. Enroll in Apple Developer Program at developer.apple.com/programs.
2. Generate a "Developer ID Application" certificate via Keychain Access → Certificate Assistant → Request Cert From CA, upload the CSR at developer.apple.com → download `.cer` → import to login keychain.
3. Export the certificate as `.p12` with a strong password.
4. Note the 10-character Team ID from the developer portal.
5. Create an app-specific password at appleid.apple.com (this is what `notarytool` authenticates with — NOT the real Apple ID password).

### Windows side (~$120/yr, ~1–3 day verification)

Recommend **Azure Trusted Signing** ($9.99/mo). Cheapest CI-friendly option; works without USB hardware tokens; SmartScreen reputation parity with EV certs. Microsoft verifies business identity (1–3 days for sole-trader, longer for orgs).

Alternatives:
- DigiCert KeyLocker / Software Trust Manager — more expensive but enterprise-grade.
- SSL.com eSigner — similar to Azure.
- Plain OV cert from Sectigo (~$179/yr) — cheapest cert but SmartScreen reputation accrues only after ~1000 downloads, so users see warnings for weeks.

### GitHub repo secrets

Once the credentials exist, add as repo secrets:

| Secret | Source |
|---|---|
| `MACOS_CERT_P12_BASE64` | Base64 of the Developer ID Application .p12 |
| `MACOS_CERT_P12_PASSWORD` | .p12 export password |
| `MACOS_KEYCHAIN_PASSWORD` | Random 32-char string for ephemeral CI keychain |
| `MACOS_DEVELOPER_ID` | Cert common name, e.g. `Developer ID Application: Adam Tout (AB1CD2EF3G)` |
| `MACOS_APPLE_ID` | Apple ID email |
| `MACOS_APPLE_TEAM_ID` | 10-char team ID |
| `MACOS_APPLE_APP_PASSWORD` | App-specific password from appleid.apple.com |
| `WIN_AZURE_TENANT_ID` / `WIN_AZURE_CLIENT_ID` / `WIN_AZURE_CLIENT_SECRET` | Azure AD App Registration |
| `WIN_AZURE_ENDPOINT` | e.g. `https://eus.codesigning.azure.net` |
| `WIN_AZURE_CODE_SIGNING_ACCOUNT_NAME` | Trusted Signing Account name |
| `WIN_AZURE_CERT_PROFILE_NAME` | Cert profile name |

## Code/config changes when ready

### New files

- **`assets/entitlements.plist`** — minimal hardened-runtime entitlements for wry/WKWebView. Start with:
  ```xml
  <?xml version="1.0" encoding="UTF-8"?>
  <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
            "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
  <plist version="1.0">
  <dict>
    <key>com.apple.security.cs.allow-jit</key><true/>
    <key>com.apple.security.cs.allow-unsigned-executable-memory</key><true/>
    <key>com.apple.security.cs.disable-library-validation</key><true/>
  </dict>
  </plist>
  ```
  Trim later based on what Apple's notary log actually flags.

- **`scripts/sign-and-notarize.sh`** — local-runnable wrapper mirroring the CI flow.

### Modified files

- **`.github/workflows/build.yml`** — three changes:

  **Mac job — replace the "Ad-hoc codesign" step (lines 214–217) with:**
  1. Decode `MACOS_CERT_P12_BASE64`, create ephemeral keychain, import .p12, set default + unlocked.
  2. Sign every binary inside the bundle (`Contents/MacOS/digtrax`, frameworks, dylibs) inside-out:
     `codesign --force --options runtime --timestamp --entitlements assets/entitlements.plist --sign "$MACOS_DEVELOPER_ID"`. Sign the `.app` itself last.
  3. Sign `digtrax-cli` the same way.
  4. Verify: `codesign --verify --strict --deep --verbose=2 DigTrax.app`.
  5. Zip the bundle (notarytool requires zip/dmg/pkg, never raw .app).
  6. `xcrun notarytool submit DigTrax-mac.zip --apple-id ... --team-id ... --password ... --wait`.
  7. On rejection: `xcrun notarytool log <submission-id> ...` to see what Apple wants.
  8. **Staple the `.app` (not the zip — zips can't be stapled):** `xcrun stapler staple DigTrax.app`.
  9. Re-zip the now-stapled `.app` for distribution.
  10. Delete ephemeral keychain.

  **Windows job — after the `Bundle` step (line 124), add:**
  1. Install dotnet sdk 8.
  2. `dotnet tool install --global sign --version 0.9.1-beta.24129.1`.
  3. Set `AZURE_TENANT_ID` / `AZURE_CLIENT_ID` / `AZURE_CLIENT_SECRET` env vars from secrets.
  4. `sign code trusted-signing --trusted-signing-account ... --certificate-profile ... --endpoint ... --description "DigTrax" target\release\digtrax.exe target\release\digtrax-cli.exe dist\DigTrax-windows-setup.exe`.
  5. Verify: `signtool verify /pa /v <file>`.

  **Add a guard at the top of each signing step:**
  ```yaml
  if: github.event_name == 'push' && (startsWith(github.ref, 'refs/tags/v') || github.ref == 'refs/heads/master')
  ```
  PR/feature-branch builds skip signing (PRs from forks can't read secrets anyway).

- **`README.md`** — once signed releases ship, update the "Why your OS warns on first launch" callout to scope it to local builds only (signed releases from GitHub will open without warnings).

## Rollout (when activated)

1. **Phase 0** — submit Apple + Azure enrollments in parallel. Wait. (~24–48h Apple, 1–3d Azure.)
2. **Phase 1** — land entitlements + script on a feature branch (no-op without secrets).
3. **Phase 2** — once Apple approves: add secrets, wire mac signing, push a `v1.x.y-signtest` tag, verify with `spctl --assess --type execute --verbose` ("accepted, source=Notarized Developer ID").
4. **Phase 3** — once Azure approves: same loop for Windows. Verify with `signtool verify /pa /v` and a real browser-download → run test (no SmartScreen prompt).
5. **Phase 4** — cut a real signed release. Update README. Drop the workaround callout to a short note.

## Verification checklist (per platform)

**macOS:**
- `codesign --verify --strict --deep --verbose=2 DigTrax.app` → "valid on disk; satisfies its Designated Requirement"
- `codesign -dv --verbose=4 DigTrax.app` → shows `Authority=Developer ID Application: …`, `flags=0x10000(runtime)`
- `spctl --assess --type execute --verbose DigTrax.app` → "accepted, source=Notarized Developer ID"
- `stapler validate DigTrax.app` → "The validate action worked!"
- Set quarantine bit and double-click — opens with no prompt

**Windows:**
- `signtool verify /pa /v DigTrax-windows-setup.exe` → "Successfully verified"
- Properties → Digital Signatures lists the cert with valid timestamp
- Browser download → run → no SmartScreen prompt

## Risks

- **Apple notarization rejects on first try.** Common (entitlements too restrictive, missing usage descriptions, library validation issues). Mitigation: notary log iteratively. Budget +1d.
- **Azure verification delays for sole-trader accounts.** Reported 3–7 days for some users. Kick off in parallel with Apple.
- **Hardened runtime breaks wry's WebKit.** Unlikely since WKWebView is system-managed. Pre-emptive entitlements above cover most edge cases.
- **Cost creep.** $99 + $120/yr ongoing. Macos work is independent — Windows can ship later if budget is tight.

## Out of scope

- Auto-update / Sparkle / WinSparkle (separate project).
- Hardware-token EV certificates (Azure Trusted Signing supersedes this use case).
- Mac App Store distribution (different cert chain, sandbox required).
