# Media playback and presentation import

## Playback design

Start the Phase 0 comparison with a persistent audio host in the operator webview and a single video element in the projector webview. Keep the audio host mounted across slide changes and projector recreation. Use a shared transport coordinator to ensure one audible source. The native core holds intended state and checks adapter reports before declaring playback active. Load and probe the next item's metadata separately from the live transport. When a Take succeeds, stop the previous transport and commit the new frame. If decoder start then fails, show an explicit error and allow retry, fallback or skip. Never restart an earlier item automatically.

| Candidate | Trade-off and probe |
| --- | --- |
| HTML audio/video | Smallest integration. Probe local scoped URLs, seek and range behavior, actual decoding, hardware acceleration, playback gesture rules and audible gaps on all target webviews. |
| OS-native playback | Compare Windows Media Foundation, macOS AVFoundation, and Linux GStreamer only if needed. More control, with separate adapters and codec or packaging differences. |
| Shared native engine | Evaluate libVLC or libmpv if webview playback fails the floor. Measure embedding, audio routing, seeking, installer size, dependencies and redistribution obligations per target. |

Baseline test profiles are local MP3 and MP4 with H.264 video and AAC-LC audio at up to 1080p. Containers, suffixes and capability hints do not establish actual decode support. Use a corpus with variable-bit-rate MP3, multiple frame rates, silent video, corrupt files and incompatible codecs. Probe startup, seek, end-of-file, screen loss and output-device changes. Publish exact supported profiles and a preparation-time re-encode fallback if practical. Use real decoding results to choose the backend. [Tauri webview versions](https://v2.tauri.app/reference/webview-versions/)

## Static import pipeline

1. Select a local source, check limits and identify format. Import during preparation with progress and cancellation. Images become ordered slides with orientation normalized. Phase 0 should start with PNG and JPEG fixtures.
2. For PDF, evaluate PDF.js rasterization in an isolated preparation context versus a native PDF renderer such as PDFium. Compare fidelity, memory, packaging, notices and security update burden. Process pages incrementally with bounded resolution and preserve page order, crop and rotation.
3. For PPTX, compare offline export by an operator with optional headless LibreOffice Impress conversion to PDF. Use a separate temporary profile and directory, argument-based process invocation, no external resource access where the containment permits, macros disabled, and bounded runtime/output. Profile isolation alone provides no sandbox. Require an explicit supported converter configuration before exposing in-app conversion.
4. Stage page assets, show a preview, and commit only after all expected pages validate. Cache by source digest, renderer version and options. Keep a prepared copy for live rendering so conversion and source changes cannot interrupt a Take.

Direct PPTX rendering libraries are a later option only if they cover the fidelity corpus and have acceptable licensing and maintenance. Compare exports with reference slides containing local fonts, substitutions, mixed layouts, cropping, notes, animations and linked or embedded media. Static imports preserve visible frames according to renderer fidelity. Animations, transitions, interactive objects and embedded audio/video need a separate workflow. On failure or unacceptable substitution, offer pre-exported PDF or images. [LibreOffice command-line parameters](https://help.libreoffice.org/latest/en-US/text/shared/guide/start_parameters.html)
