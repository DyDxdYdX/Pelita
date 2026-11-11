# Phase 1 · Core projection pilot

Start implementation on the Tauri 2 candidate using the reported built-in plus REALTEK extended-display run in the [Phase 0 record](phase-0.md). This is a development baseline. ADR 0001 and platform certification remain open until their evidence gates are met.

## Implementation design

- Use a separate application and profile, with React, TypeScript, Vite and a Rust core. The operator prepares and previews a saved service. The projector receives revisioned, generation-scoped snapshots from Rust and starts black. Extended output requires verified placement before Take. Single and mirrored operation use an explicitly warned fullscreen mode with the operator hidden and keyboard controls on the content window.
- Use a versioned JSON service store with a serialized, atomic writer for the pilot. Store ordered playlists, item snapshots and preferences in the app data directory, with immutable managed files beside the store. Keep the schema independent of the UI and migrate the store when the searchable SQLite library arrives. Prevent simultaneous writers by locking the profile.
- Rasterize static PDFs offline with the local PDF.js worker before committing the import. Validate all selected images and PDF pages first, then publish the managed copies as one group. Use operator-exported PDF for PowerPoint. Bound import size, page count and output resolution. Keep conversion off the projector.
- Use a persistent operator MP3 element and projector-hosted MP4 elements. Probe candidate media before replacing current output. Prepare a separate video element in the projector before Take and check its generation on commit. Rust authorizes playback assets by ID. Display loss retires video and preserves the MP3 until the operator changes transport or clears.
- Treat PNG, JPEG, static PDFs, MP3 and H.264/AAC MP4 as pilot inputs. Webview decode and packaged runtime coverage are conditional on hardware evidence.

## Acceptance

| Requirements | Rehearsal |
| --- | --- |
| PRJ-01–04 | Confirm each mode, verify private preview in extended mode, verify keyboard exit in shared mode, clamp navigation, exercise Black, Restore and Clear, and unplug and reconnect during both slides and video. Confirm black startup and require a fresh Take after reconnect. |
| MED-01–03 | Verify MP3 controls and continuity through slide and mask actions. Reject a corrupt video while accompaniment runs, then Take a valid video and confirm one audible source. Switch back to slides. |
| IMP-01 | Import the Phase 0 PDF fixture and ordered images, verify crop, rotation and fit, and confirm a corrupt or cancelled import leaves the playlist unchanged. |
| SVC-01–02 | Save, duplicate, reorder and restart offline. Verify the original order and the live snapshot survive queue edits. Remove a managed file in a scratch profile, run preflight, and relink or skip the affected item. |

Close the milestone after the packaged two-hour offline rehearsal and volunteer pilot in [milestones](milestones.md). Record physical display, codec, keyboard, resource and recovery evidence with the [testing strategy](testing-strategy.md).

## Running the pilot

1. Open the [local debug app](development-setup.md#phase-1-desktop-app) on the extended desktop with the audience screen blank. Create a service with text slides, import the fidelity PDF and sample video, and attach the fixture MP3 to a slide item. Use the [cleared Phase 0 fixtures](../../spikes/phase-0/fixtures/README.md).
2. Run preflight and confirm the intended audience display. Verify that it opens black. Select a playlist item and press **Take**. Preview another item while advancing the live slides. Black masks the live output, Restore reveals it and Clear stops playback. Verify the accompaniment clock and record actual audio continuity through slide and mask changes.
3. With accompaniment playing, Take the valid MP4 and verify the audible switch. Exercise unplug and reconnect, verify black recovery and Take again. For shared presentation, select the starting item and use **Single / mirrored**. Press Enter to Take, arrow keys to advance slides, Page Up or Page Down to Take adjacent items, and Esc to exit. Rehearse with the audience screen visible to an observer.
4. Duplicate the service, reorder the copy and restart offline. In a scratch profile, remove one managed file. Verify that preflight lists the file and that Relink and Skip recover the service. Reopen with the network disabled and run the full two-hour rehearsal.

## Current evidence and limits

- A local arm64 macOS debug bundle was built using the locked dependency graph and verified with an ad hoc signature. On macOS 27.0.1 (26A434), the packaged app confirmed the 1920×1200 extended display beside the built-in Retina display. The audience window opened black and showed only the taken slide. The MP3 clock continued through Next, Black and Restore. The MP4 became live and the MP3 transport cleared. This was a functional check without loopback audio measurement.
- The packaged app imported the synthetic PDF through the native picker, saved three managed PNGs, projected the first page, and reopened the saved deck after restart. Shared presentation on the built-in screen hid the operator window, accepted keyboard Take, and returned to the operator on Esc. The host remained in extended mode for this shared-presentation check.
- Five Rust checks cover service reopen, stale writes and profile locking, missing assets, slide bounds and mask behavior, accompaniment preservation, video candidate authorization, and bounded media reads.
- The built operator UI was exercised in Chromium with simulated native commands. The rehearsal covered creating text slides, preview and Take, Black and Restore, service duplication and reordering across reload, and corrupt PDF rejection. The actual bundled PDF.js worker rendered the three-page crop and rotation fixture offline. All three pages retained their order and matched the independent Poppler renders within 2.4 mean RGB levels per channel at 576×324.
- PDF input is limited to 64 MiB and 100 pages, rasterized into a 1920×1080 bounding box. Images are limited to 24 MiB and 32 megapixels each. Each managed file is limited to 256 MiB and each import to 512 MiB and 100 prepared files. Images and PDF pages selected together form one ordered deck. Preflight checks managed file availability, size and type header. Playback still depends on the target webview decoder.
- Managed copies survive changes to the original files. Relink replaces a reference in the selected service item. Unreferenced managed copies can remain after an edit or interrupted save. Back up the service document and media together while Pelita is closed.
- Physical mirror and single-display topology tests, unplug and reconnect, measured audio continuity, resource budgets, Windows and Linux packages, and the two-hour offline volunteer pilot remain open. These checks determine the supported platform and codec matrix.
