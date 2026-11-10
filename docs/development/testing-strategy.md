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
