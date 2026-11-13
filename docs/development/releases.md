# CI and releases

Pelita's desktop application is in `app/`. CI checks the locked npm and Cargo builds on Ubuntu 22.04. A version-tag release first runs those checks, then builds native Tauri installers on Windows x64, macOS Apple Silicon and Intel, and Linux x64. Publishing happens only after all five expected installers are present. A manual release workflow run defaults to build-only inspection; selecting **Publish release** on `main` creates the tag and publishes after validation.

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

1. Run `python3 scripts/release.py prepare --version 0.1.0-alpha.2` (substitute the intended version). This updates npm, Cargo, both lockfile version entries, and Tauri metadata together, then validates their consistency. Review and commit the five changed files, merge the change into `main`, and let main-branch CI pass. Do not change the Tauri identifier.
2. In GitHub **Actions → Release → Run workflow**, select `main`, enter `0.1.0-alpha.2`, and select **Publish release**. Without that selection, the run only builds downloadable workflow artifacts. Alternatively, push an existing version tag such as `v0.1.0-alpha.2` to trigger the same pipeline.
3. Watch the Release workflow. It rejects invalid tags, mismatched versions, and manual publishing from a branch other than `main`. On success, download the assets from the repository's GitHub **Releases** page. A version containing a prerelease suffix such as `-alpha.1` or `-beta.1` is marked prerelease; GitHub generates the release notes.

The workflow requires Actions enabled and the repository's `GITHUB_TOKEN` allowed to create releases (`contents: write` in the publish job). It uses no custom secrets. Branch protection should require the CI job before merging to `main`. Keep version changes on reviewed commits; the release workflow runs the same CI checks again. Manual publishing is an explicit opt-in and only works on `main`; it refuses to publish if `main` moves after the installers were built. Concurrent runs for one version are serialized without cancelling an active run.

## Limits and troubleshooting

Windows installers are unsigned and may show SmartScreen warnings. macOS DMGs have an ad-hoc signature but no Developer ID signature or notarization; Gatekeeper may require user approval in Privacy & Security. Do not disable system security protections. For a production signed release, add signing identities and notarization credentials through protected GitHub secrets and follow Tauri's platform signing guides. Automatic updates need a separate design, updater signing keys, and compatibility testing; none are configured here.

For failed CI, read the failing job's first error: npm lock drift fails `npm ci`; Rust lock drift fails `--locked`; TypeScript, Clippy, formatting, and tests each have separate steps. On Linux, check the WebKitGTK 4.1 and AppImage system packages. On macOS, Tauri builds the app once and retries DMG bundling when `hdiutil` is temporarily busy. A missing bundle or failed matrix job prevents publication. GitHub reruns use the original commit; after a workflow fix, start a new manual run on `main`, or use a new version tag for tag-triggered releases. A green workflow verifies builds, not live projection, media codecs, upgrades, or offline rehearsal on physical target machines.
