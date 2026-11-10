# Technology evaluation

| Layer | Candidate and trade-off | Decision gate |
| --- | --- | --- |
| Desktop | Tauri 2 with Rust and OS webviews reduces the runtime Pelita ships but brings OS-specific display and playback behavior. Electron bundles a common Chromium runtime at higher installer and memory cost. Native toolkits add UI and media integration work. | [ADR 0001](../decisions/0001-desktop-framework.md) |
| Operator and projector UI | React, TypeScript, Vite and Tailwind CSS are the proposed stack. Reuse layout tokens and static presentation components while keeping live controls separate. | Validate fullscreen rendering, keyboard focus and packaged asset loading. |
| UI state | Start with component state and reducer/context for operator state. Evaluate Zustand if cross-screen subscriptions become awkward. | Keep one authoritative native live session either way. Measure projector updates separately from preview edits. |
| Persistence | SQLite with a Rust-owned repository and migrations. Prefer a build that consistently provides FTS5. | Verify packaging and backup/restore before adding search. |
| Playback and import | Webview media and a preparation-time raster pipeline first. Native playback or conversion tools add distribution and maintenance costs. | Run the [media spike](media-engine.md) on packaged builds. |
| Distribution | GitHub Actions builds per target and GitHub Releases hosts installers. Optional static registry can use GitHub-hosted artifacts. | Validate offline prerequisites, clean installs, macOS signing and notarization, Windows signing, Linux packaging and release credentials. |

Tauri uses WebView2 on Windows, WKWebView on macOS, and WebKitGTK on Linux. Bundler output and webview dependencies vary by platform. Pin and record the actual evaluated toolchain, including webview versions, and publish tested configurations before calling them supported. [Tauri webview versions](https://v2.tauri.app/reference/webview-versions/) · [Tauri distribution](https://v2.tauri.app/distribute/) · [GitHub pipeline](https://v2.tauri.app/distribute/pipelines/github/)
