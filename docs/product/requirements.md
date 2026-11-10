# Product requirements

Phase numbers indicate planned delivery. See [milestones](../development/milestones.md) for release gates. Platform coverage and supported formats depend on Phase 0 evidence.

| ID | Phase | Requirement | Acceptance |
| --- | --- | --- | --- |
| PRJ-01 | 1 | Separate operator preview and projector output. Detect displays, choose a target, and go fullscreen on validated extended configurations. | Preview changes stay private until Take. Audience output shows content without controls. |
| PRJ-02 | 1 | Support mirrored and single-display operation with an explicit presentation mode. | Operator sees a mirror warning before launch, presents content-only, and exits by keyboard. |
| PRJ-03 | 1 | Provide next, previous, Black, Clear, and Restore. | Navigation clamps at endpoints. Black masks the frame, Clear discards it, and Restore reveals only an existing frame. |
| PRJ-04 | 1 | Recover from display loss and remember a preferred target when safely identifiable. | Unplug suspends projection. Reconnection requires confirmation and a fresh Take. Ambiguous matches require selection. |
| MED-01 | 1 | Play validated local MP3 files with pause, resume, restart and volume. | Each transport action works offline and failures are shown to the operator. |
| MED-02 | 1 | Run local audio independently of manual lyric or slide navigation. | Next, previous, Black and Restore leave accompaniment running at its current position and volume. |
| MED-03 | 1 | Play validated local MP4 videos and switch among content types. | A Take to video stops previous audio. An unsupported video leaves the existing live item intact when detected during preparation. |
| IMP-01 | 1 | Import images and static PDF pages offline. | Page order, orientation, and aspect ratio match the fixture. Failed import creates no playable item. |
| IMP-02 | 0 | Evaluate an offline PPTX conversion workflow. | Compare reference slides against imported frames. Publish validated conversion guidance or require pre-exported PDF. In-app conversion is optional. |
| SVC-01 | 1 | Create, order, edit, save, and reuse service playlists containing songs or manual lyrics, Scripture, announcements, special music, sermons, and video. | Duplicate a playlist, reorder it, restart offline, and run the duplicate while the original order remains intact. |
| SVC-02 | 1 | Validate source assets before a service and provide recovery. | Missing files are flagged with relink or skip options. Reordering or editing the queue leaves the current live item stable. |
| LIB-01 | 2 | Search songbooks by title, number, and available lyrics. | Exact number results rank first within a songbook. Identical numbers in other books remain distinguishable. |
| LIB-02 | 2 | Create songs manually and store metadata separately from associated media. | A song remains searchable after a linked media file moves and can be relinked. |
| LIB-03 | 2 | Associate multiple media variants and set a preferred variant per song, with an item-level override. | A song offers text, slides with MP3, and MP4 when present. The operator can override the default before Take. |
| EXT-01 | 3 | Import, validate, enable, disable, and uninstall local declarative content packs. | Installed packs work offline. Removal or disablement identifies affected playlist items and preserves their references. |
| EXT-02 | 4 | Browse and search the registry, download packs, and update them. | Compatibility and integrity checks run before activation. Failed updates preserve the previous installed version. |
| LPM-01 | 3+ | Support optional LPMI numbers, titles, lyrics, and authorized media through the generic songbook format. | An invented catalog exercises the same schema. Any real pack passes the separate rights review before distribution. |

## Quality targets

Numbers below are provisional engineering budgets. Use packaged builds and record hardware, OS, webview, codecs, display topology, and measurement methods. Phase 0 proposes revisions based on measured constraints and pilot needs. See [testing strategy](../development/testing-strategy.md).

| ID | Target | Measurement |
| --- | --- | --- |
| NFR-01 | Complete a two-hour simulated service with zero crashes, unintended Takes, or stuck controls. | Run the scripted rehearsal on each supported configuration. |
| NFR-02 | All supported local operations work without a network. | Disable networking before first launch of a pre-provisioned installation. Repeat with an installed pack when Phase 3 ships. |
| NFR-03 | Cold launch to usable operator screen within 5 s at p95. | Measure 20 cold process launches per reference machine and record cache conditions. |
| NFR-04 | Search results visible within 150 ms at p95 from input, including debounce. | Run at least 100 representative searches against 10,000 songs. |
| NFR-05 | Zero unintended pauses, restarts, or detectable gaps of 50 ms or more while manually advancing slides. | Capture loopback playback of a continuous fixture across 100 slide and mask changes. |
| NFR-06 | Alert and suspend within 3 s at p95 of display loss. Resume a black preview within 10 s at p95 after the operator confirms a valid target. | Run 20 unplug and reconnect trials per supported topology. Check both physical screens. |
| NFR-07 | Publish a tested support matrix across Windows, macOS and Linux, with limitations listed by session and hardware class. | Package, install, and exercise each supported row on real hardware. |
| NFR-08 | Committed playlists and edits survive restart or process termination. | Kill the process across save boundaries and restore from a verified backup. |
| NFR-09 | Complete every live action without a mouse, with visible focus, accessible control names, and no color-only state. | Keyboard-only rehearsal, screen reader check per platform, and 200% operator UI zoom check. |
| NFR-10 | Run at 1080p on a provisional four-logical-core, 8 GB RAM, integrated-GPU reference class with peak app and child-process working set below 1 GB during rehearsal. | Record memory, CPU, frame drops and playback stalls on an actual baseline machine. Exclude preparation-only converter processes from the live budget. |
| NFR-11 | Failed package validation or interrupted installs leave the previous active version usable and write nothing outside allowed storage. | Run invalid package and interruption tests and compare live pack state before and after. |
| NFR-12 | Domain transitions, schema migrations, and format validators are testable without a webview. | Run their automated checks on every supported CI build platform. |
