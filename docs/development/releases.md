# CI and releases

Pelita's desktop application is in `app/`. CI checks the locked npm and Cargo builds on Ubuntu 22.04. A version-tag release first runs those checks, then builds native Tauri installers on Windows x64, macOS Apple Silicon and Intel, and Linux x64. Publishing happens only after all five expected installers are present. A manual release workflow run validates and builds installers for inspection, but does not publish.

| Platform | Release asset |
| --- | --- |
| Windows x64 | unsigned NSIS `.exe` with the offline WebView2 installer |
| macOS arm64 | ad-hoc signed `.dmg` |
| macOS x64 | ad-hoc signed `.dmg` |
| Linux x64 | `.AppImage` and `.deb` |

CI runs TypeScript validation, the Vite build, `cargo fmt --check`, Clippy with warnings denied, and Rust tests. No frontend lint, formatter, or test script is configured yet. The release builds use Tauri 2's `tauri-action` and the checked-in npm and Cargo locks. They do not modify installed user data. The app stores services and managed media in Tauri's application data directory, outside the installed bundle; keep the application identifier stable across upgrades. The built frontend and PDF resources are local. Windows' offline WebView2 mode allows installation without fetching its runtime; normal operation on all platforms is offline.

## Local checks and bundles

Install the platform prerequisites in [development setup](development-setup.md), Rust 1.94.0, Node 24, npm, and Python 3.11 or newer for the release checker. The workflow uses Python 3.12. From the repository root:

```sh
cd app
npm ci
npm run check
npm run build
cargo fmt --check --manifest-path src-tauri/Cargo.toml
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path src-tauri/Cargo.toml
npx --yes @tauri-apps/cli@2.12.4 build --bundles dmg # macOS example
```

On Windows use `--bundles nsis`; on Linux use `--bundles appimage,deb`. The CI build installs the same CLI version. Native installer and hardware validation still require each target OS.

## Publish a version

1. Update `app/package.json`, `app/package-lock.json`, `app/src-tauri/Cargo.toml`, `app/src-tauri/Cargo.lock`, and `app/src-tauri/tauri.conf.json` to the same SemVer version. Check in the change and let main-branch CI pass. Do not change the Tauri identifier.
2. Create an annotated tag for that commit, for example `git tag -a v0.1.0-alpha.1 -m "Pelita v0.1.0-alpha.1"`, then push it with `git push origin v0.1.0-alpha.1`.
3. Watch the Release workflow. It rejects invalid tags and mismatched versions. On success, download the assets from the repository's GitHub **Releases** page. A version containing a prerelease suffix such as `-alpha.1` or `-beta.1` is marked prerelease; GitHub generates the release notes.

The workflow requires Actions enabled and the repository's `GITHUB_TOKEN` allowed to create releases (`contents: write` in the publish job). It uses no custom secrets. Branch protection should require the CI job before merging to `main`. Keep the tag restricted to reviewed commits; the tag workflow itself runs the same CI checks. Manual `workflow_dispatch` is build-only and cannot publish. Concurrent runs for one tag are serialized without cancelling an active run.

## Limits and troubleshooting

Windows installers are unsigned and may show SmartScreen warnings. macOS DMGs have an ad-hoc signature but no Developer ID signature or notarization; Gatekeeper may require user approval in Privacy & Security. Do not disable system security protections. For a production signed release, add signing identities and notarization credentials through protected GitHub secrets and follow Tauri's platform signing guides. Automatic updates need a separate design, updater signing keys, and compatibility testing; none are configured here.

For failed CI, read the failing job's first error: npm lock drift fails `npm ci`; Rust lock drift fails `--locked`; TypeScript, Clippy, formatting, and tests each have separate steps. On Linux, check the WebKitGTK 4.1 and AppImage system packages. A missing bundle or failed matrix job prevents publication. GitHub reruns use the original tag commit, so a workflow fix requires a new version commit and tag. A green workflow verifies builds, not live projection, media codecs, upgrades, or offline rehearsal on physical target machines.
