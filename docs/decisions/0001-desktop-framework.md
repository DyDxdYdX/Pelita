# ADR 0001 · Desktop framework

**Status: Proposed.** Accept or supersede after Phase 0.

## Context

Pelita needs separate operator and projector surfaces, local media, offline operation, support for modest church computers and maintainability by a part-time developer. Windows, macOS and Linux are product targets. Display placement and decoder behavior need packaged tests on each target.

## Proposed decision

Use Tauri 2 with a Rust application core and React/TypeScript webviews for the feasibility slice. Limit renderer privileges and keep display, media, and persistence behind adapters so domain behavior can be retained if a platform integration changes.

| Option | Trade-off |
| --- | --- |
| Tauri 2 | Uses OS webviews and keeps native domain logic in Rust. Display, runtime provisioning and codec behavior require target-specific validation. |
| Electron | Bundled Chromium narrows web runtime differences, with a larger distribution and likely higher memory use. Still requires OS display and media validation. |
| Native UI toolkit | May give closer control over windows and native media at the cost of a larger UI and integration commitment for a solo maintainer. |

## Acceptance gate

Record packaged results for independent audio and slides, local video, monitor enumeration and targeting, privacy, hot-plug recovery, keyboard operation, startup and memory. Validate PDF import separately. If a candidate fails, try a bounded adapter or operating-mode fallback. Compare Electron if Tauri-specific constraints still block the required workflow. Accept the framework only when tested targets have a usable support floor and the remaining gaps have explicit limitations or follow-up decisions. Keep measured results and the maintainer decision with the ADR. See [milestones](../development/milestones.md) and [stack evaluation](../architecture/tech-stack.md).
