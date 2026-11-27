# Testing strategy

Tie evidence to [requirement IDs and quality budgets](../product/requirements.md). Store fixture provenance and keep copyright-cleared media in test assets. Prefer invented lyrics, generated test tones and purpose-built slides. Record build, hardware, OS/session, webview, display topology and output device for every hardware result.

| Layer | Coverage |
| --- | --- |
| Rust domain | Command transitions, revision ordering, Take/Black/Clear semantics, transport ownership, display loss, stale callbacks, validation, migrations and transactional writes. Use fake display and media adapters for timing and failure cases. |
| Frontend | Preview privacy, focus restoration, accessible names, keyboard handling, projector reload and snapshot resync. Verify focus in text fields and mirrored fullscreen. |
| Persistence and import integration | Migration of prior data, missing assets, abort/retry, inconsistent cache cleanup, backup and restore. Compare raster images for orientation, ordering and layout with tolerance for renderer differences. Review PPTX substitutions manually. |
| Package validation | Corrupt archive, bad checksum, unsupported schema and version, duplicate or unsafe paths, over-budget content, interruption and previous-version recovery. Use synthetic inputs and inspect staging plus committed state. |
| Packaged hardware | Validate playback and output on actual Windows, macOS and Linux systems. Cover mixed DPI, mirrored or single display, unplug/replug, focus switching, sleep/wake and audio-device changes. Separate Wayland compositor results from X11. |
| Pilot | Volunteer runs a two-hour offline service, including a missing-file recovery and display interruption. Record time, operator confusion, interruptions and any unintended audience output. |

CI should run formatting, type checks, unit and integration checks and platform build smoke tests after scaffolding exists. Use automated UI tests where the packaged webview allows. Keep external display, physical audio capture, and accessibility checks in a repeatable manual checklist. Use the acceptance budgets for p95 and resource reporting. Record failures and compare them to a documented baseline before changing targets. Require approval of support matrix changes when a release candidate regresses. A clean CI run alone cannot certify external displays or codecs.

Release evidence should identify the exact fixture, expected result, actual result, build and environment, and whether the case passed or has an accepted limitation. Failed core live-service cases block the affected support claim.

## PPTX conversion

Run normal unit tests with `cargo test --locked --manifest-path app/src-tauri/Cargo.toml`. The tests use the original [Phase 0 slide fixture](../../spikes/phase-0/fixtures/README.md) and generate variations for aspect ratio, embedded art, hidden slides, invalid sources and linked content. They use mock providers for platform policy, fallback, cache reuse, corruption, and regeneration. No Office installation is required for standard CI.

With LibreOffice Impress installed, run:

```sh
cargo test --locked --manifest-path app/src-tauri/Cargo.toml libreoffice_smoke -- --ignored --nocapture
```

On an interactive Windows user account with an activated PowerPoint desktop app, run:

```powershell
cargo test --locked --manifest-path app/src-tauri/Cargo.toml powerpoint_smoke -- --ignored --nocapture
```

Keep a separate, unsaved test document open in PowerPoint and confirm it stays open and unchanged. Test import while PowerPoint is already running, after restarting it, during an Office security or license prompt, and while cancelling an export. Verify the prompt remains in Office, the managed import either completes or reports the cause, and no other presentation is closed. Test a security-marked download, compare static images with the source, then verify Automatic fallback with LibreOffice installed. Run the LibreOffice smoke test separately on Windows and Linux.

Rehearse batch import, retry, regeneration, and native drag-and-drop in the packaged app. Use legal test decks and play a separate MP3 while changing live slides, choosing another deck, and preparing a new PPTX. Verify 800×600, 1024×768, 1280×720, 1366×768, and 1920×1080. Restart Pelita with the source and converter unavailable and Take the cached deck. Record each provider version, OS, webview, and projector topology.

### Current implementation evidence

| Environment | Observed result |
| --- | --- |
| macOS 27.0.1 (Apple silicon) | Rust tests, TypeScript checks, and a native build with bundled frontend assets passed. |
| LibreOffice 26.2.5.2 on macOS | Real headless export and rasterization passed at 1920×1080, 1440×1080, and 1920×800. The fixtures covered ordered slides, backgrounds, an embedded image, and a hidden slide. |
| Headless Chrome on macOS with a simulated Tauri bridge | Passed all five target sizes plus 580×580. Exercised settings, partial import, retry, cancellation, the drop callback, ordered thumbnails, Take, live jumps, keyboard navigation, regeneration, and MP3 continuity. |
| Windows x86-64 | Rust target cross-check and Clippy passed. Native PowerPoint lifecycle, Protected View, export, and real fallback to LibreOffice need an interactive Windows session. |
| Linux and packaged desktop controls | CI has a Linux LibreOffice smoke step. Native Linux and Windows LibreOffice testing, packaged picker and drag-and-drop, and actual audio/display rehearsal remain pending. |
