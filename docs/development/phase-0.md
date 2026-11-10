# Phase 0 feasibility record

**Open.** The Pelita maintainer owns each decision. Review evidence by **27 November 2026, 17:00 Asia/Kuching**. Mark missing evidence blocked at review and reschedule with named hardware. Keep ADR 0001 proposed until the real hardware gate is met.

## Target matrix

| Run order | Target | Runtime | Sessions and topology | Availability |
| --- | --- | --- | --- | --- |
| 1 | This Apple M5 arm64 Mac, macOS 27.0.1 (26A434) | WKWebView, system WebKit bundle 22625.1.29.11.28 | Extended 1080p first, then mixed scale, single and mirror. Record separate Spaces setting. | Selected first host. CoreGraphics reported 0 active displays in this session on 10 November. Needs a console session with a physical projector. |
| 2 | Windows 11 x64 | WebView2 Evergreen, record installed version | Extended 1080p, negative coordinates, mixed DPI, single and mirror | Host and exact build pending |
| 3 | Linux x64, Ubuntu LTS candidate | WebKitGTK 4.1 ABI, record library and media plugin versions | Test GNOME Wayland and GNOME on Xorg separately on a distro release that exposes both sessions. Cover mixed scale, single and mirror. | Host, distro release and compositor versions pending |
| 4 | Four logical cores, 8 GB RAM, integrated GPU | Runtime from applicable row | 1080p extended and two-hour offline rehearsal | Reference machine pending |

Test Windows and both Linux session rows only after the first dual-display rehearsal. For each run record package revision, OS build, webview, display make/resolution/scale/position, compositor or Spaces, audio route, fixture hashes, trial counts and raw observations.

## Rehearsal and decisions

| Question and requirement | Action and pass criterion | Decision by 27 Nov, 17:00 MYT | Current result |
| --- | --- | --- | --- |
| Private fullscreen and keyboard (PRJ-01–03, NFR-09) | Launch offline. Confirm an extended target and check that it opens black. Change preview before and after Take and watch both screens. Exercise endpoint clamps, Black, Restore and Clear using keyboard. Audience sees only the last Take or black, with no controls. In mirror/single mode require a warning, content-only output and keyboard exit. | Approve tested topologies or try one placement mode per failed platform. | Pending physical run. Mirror/single presentation mode is outside this scaffold. |
| Continuous audio (MED-01–02, NFR-05) | Check pause, resume, restart and volume. Capture loopback during 100 Next/Previous/Black/Restore actions. Require zero unintended resets, pauses or gaps ≥50 ms. | Keep webview audio or compare one native adapter. | Pending capture |
| Candidate video (MED-03) | Prepare and Take the H.264/AAC clip during MP3 playback. Require visible motion, audio, full playback, no double audio and a clean switch back to slides. With a deliberately invalid copy in a scratch build, require failed preparation to leave the live item intact. | Confirm this profile or compare one native adapter. | Pending packaged playback |
| Safe display loss (PRJ-04, NFR-06) | While live, physically unplug and film both screens. Record any unintended exposure, visual suspension and alert time. Reconnect, confirm again, verify black, then Take. Check the MP3 clock survives and repeat with video. Run 20 cycles per supported topology. Require zero unintended exposure, p95 loss alert/suspension ≤3 s and black recovery after confirmation ≤10 s. Restart offline and verify black startup. | Approve tested path or record a blocker. | Pending physical run |
| Package and budget (NFR-01–03, NFR-07, NFR-10) | Install locally on each target and disable network before first launch. Record 20 cold process starts and p95 ≤5 s. On reference host require two-hour rehearsal without crash, unintended Take or stuck controls and peak app plus child working set <1 GB. Record CPU and stalls. | Set support floor or hold the ADR. | macOS debug app bundled. Runtime and resource budgets blocked by hardware access. |
| PDF raster fidelity (IMP-01) | Separately rasterize the invented PDF with crop enabled. Verify three pages in order, correct rotated orientation and crop, correct aspect and no clipped content. Reject a corrupt input. | Assess candidate rasterizer against independent reference. | Pass on this synthetic corpus with Poppler. |
| PPTX conversion fidelity (IMP-02) | Separately export the invented editable deck offline through LibreOffice in a temporary profile. Compare with independent slide renders for count, order, aspect, text, line breaks, colors and clipping. | Use operator-exported PDF if conversion fidelity or availability blocks support. | Pass on this synthetic corpus with LibreOffice. Broader compatibility is pending. |

## Implementation scope

Disposable Tauri 2 package in `spikes/phase-0/`. Use local static web assets and vanilla JS for this small probe. React and TypeScript remain part of the proposed application stack. Rust owns the confirmed target, generation and live snapshot. A dedicated projector page starts black, polls its own generation for snapshots, and has no operator controls. The operator page owns the persistent MP3. Confirm only on an extended display separate from the operator display. macOS uses CoreGraphics mirror detection because Tauri reports monitor geometry at each display's backing scale. Other targets use disjoint bounds. A recurring target check retires output on topology change. Reconfirm and Take explicitly after loss. Polling can observe a topology change after the OS has already moved a window, so physical exposure checks are mandatory.

Fixtures use invented text and shapes, generated audio and generated video. No imported songbook content. Source-license selection and the LPMI register have independent gates.

## Evidence recorded on 10 November 2026

- **macOS build:** Tauri 2.12.5, CLI 2.12.4, Rust/Cargo 1.94.0, Node 24.22.0, npm 12.2.0, and Xcode 27.0 (27A266a). The locked Rust graph compiled and the CLI produced a local 32.68 MiB arm64 debug app bundle with Cargo networking disabled after dependencies were cached. JavaScript syntax checks passed. No launch, install, decoder or physical display result is claimed.
- **Media fixtures:** FFmpeg 8.1.1 decoded both files end to end without errors. The MP3 was 240.000 s at 48 kHz. The MP4 was 12.000 s with H.264 High at 1920×1080 and 30 fps, plus 48 kHz AAC-LC. This establishes the fixture profiles, with webview decoding, loopback and switching still pending.
- **PDF (IMP-01):** Poppler 26.05.0 produced three 576×324 PNGs. Visual comparison confirmed ordered titles, deliberate 90° text rotation on page 2, and intact borders and removal of the crop sentinel on page 3. The corrupt PDF exited nonzero and produced zero PNGs. Import staging and atomic commit remain untested.
- **PPTX (IMP-02):** LibreOffice 26.2.5.2 exported three pages using a fresh temporary profile. The 480×270 raster comparisons retained the order, text, line breaks, accent bars and exact background RGB values. The PDF embedded ArialMT. Visual inspection found no clipping. The fixture had no external resources. The host network state was not verified during conversion. PowerPoint, complex slides and other fonts remain untested.
- **Fallback:** Use operator-exported static PDF as the preparation path when PPTX conversion is unavailable or fails visual review. Treat this as Phase 0 guidance. No live-display or playback fallback is accepted without hardware evidence. Mirror and single-display presentation mode remain a separate blocker.
- **Blockers:** The first host has no active displays in the accessible session. Privacy, fullscreen placement, continuity, hot-plug recovery, and packaged offline operation need a physical extended-display run. Windows, Linux X11/Wayland and reference hardware are pending. No platform is certified. [ADR 0001](../decisions/0001-desktop-framework.md) remains proposed and Phase 0 remains open.

### Fixture digests (SHA-256)

| File | Digest |
| --- | --- |
| `continuous.mp3` | `54211f2a218e41cc4d7f6230300fc1c44bd609603fc80953159b9b2f95712ab0` |
| `candidate.mp4` | `27f8bf165fffa1db0576f8bdef75fcc9563ad2fae1eee047094ccc2997b23a7b` |
| `fidelity.pdf` | `b0879adc7bb3e3dcd58b6988eedcea0513dee7503504f081d877fe0ca387a32b` |
| `fidelity.pptx` | `10749d5b19ffde8b561fbd69816bc34ea1aaa255bed1e19018530cdd06babfe7` |

### macOS extended-display retest

A hardware screenshot showed REALTEK configured as an extended display while Pelita rejected confirmation. The spike now checks macOS mirroring through CoreGraphics and opens the projector window using logical desktop coordinates before requesting fullscreen. The operator confirmed the rebuilt app works with the built-in and REALTEK extended display pair after retrying display confirmation. Mirror mode and operator-display rejection still need a retest. The full physical rehearsal remains open.

The operator subsequently reported that the manual rehearsal checks worked on the built-in and REALTEK extended setup. Treat this as a reported functional pass for private preview and fullscreen Take, accompaniment during slide changes, MP4 switching, and unplug/reconnect recovery. Loopback measurements, recovery timing and trial counts were not supplied. Mirror/single-display behavior, resource budgets and the other target platforms still require evidence.
