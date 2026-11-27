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
2. Standalone PDFs use the bundled PDF.js worker. Process pages incrementally with bounded resolution and preserve page order, crop and rotation.
3. [Presentation import](presentation-import.md) validates PPTX packages and converts them offline through Windows PowerPoint or LibreOffice.
4. Stage page assets and publish only after all expected pages validate. Keep a prepared copy for live rendering so conversion and source changes cannot interrupt a Take.

Static imports preserve visible frames according to renderer fidelity. Animations, transitions, interactive objects and embedded audio/video playback require a separate workflow. Operators can import a pre-exported PDF or images if they need a different render.
