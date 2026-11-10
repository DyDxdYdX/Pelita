# Development setup

The [Phase 0 spike](../../spikes/phase-0/README.md) is isolated from application code. Follow the [feasibility record](phase-0.md) for the matrix and acceptance gate.

| Host | Preparation |
| --- | --- |
| All | Git, Rust via rustup, an active Node.js LTS and a frontend package manager. Use npm provisionally. Record exact versions when the spike starts. Provision an external display, test audio output and local cleared fixtures. |
| Windows | Install the supported MSVC C++ build tools and Windows SDK and verify WebView2. Test whether the chosen installer needs an offline WebView2 provisioning path. |
| macOS | Install Xcode Command Line Tools and verify WKWebView on the target OS. Record CPU architecture and Spaces settings. Plan separate signing and notarization for release testing. |
| Linux | Install the Tauri build dependencies for the chosen distro, including WebKitGTK development libraries and required native tooling. Record desktop, compositor, display session and media plugin availability. |

Follow the official platform-specific package list for the toolchain being tested. Evaluate optional LibreOffice separately and record its version, installed fonts and availability. [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) · [Tauri Windows installers](https://v2.tauri.app/distribute/windows-installer/)

## Implementation handoff

- [x] Select the first host and record the candidate OS/session matrix.
- [ ] Confirm physical display access on the first host and provision the baseline machine.
- [x] Create a rights-cleared fixture set and a manual rehearsal checklist tied to requirement IDs.
- [x] Set an evidence deadline and a decision owner for each Phase 0 question.
- [x] Record initial toolchain and runtime versions, then scaffold an isolated spike once implementation is authorized.
- [ ] Run the [first milestone](milestones.md) and record pass, fallback or blocker by target.
- [ ] Update the [framework ADR](../decisions/0001-desktop-framework.md), support floor and performance budgets before Phase 1.
- [x] Select the source license before accepting public contributions or distributing builds. Track LPMI outreach separately.

After the framework decision, document the real build and dev commands, pin the toolchain and check in dependency lockfiles. Keep machine-specific paths and signing credentials outside the repository. Development data should live in a separate profile from church production data.
