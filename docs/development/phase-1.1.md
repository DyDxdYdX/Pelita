# Phase 1.1 operator usability

## Design

- The Audio playlist lists a service's existing standalone MP3 items. Import uses the existing managed media flow. Audio-only reordering swaps positions between MP3 items and keeps other items in place. The existing service format saves track order. The active queue follows its service until the operator starts a track from another service.
- One operator audio element handles the playlist, private tests, and accompaniment taken with slides. The playlist continues through slide changes, visual-only Takes, Black, Clear, and output disconnects. Taking a slide item with an attached MP3 switches to that accompaniment. Taking a video pauses the playlist to avoid overlapping sound. The operator can resume it after the video leaves the output.
- Thumbnails reuse Pelita's text and image rendering, including PDF pages prepared at import. Selecting a thumbnail of a private deck sets the starting slide for Take. On the unchanged live deck, selecting a thumbnail jumps the current Rust session to that slide. Rust validates the live Take token and index and preserves the mask and accompaniment. The projector continues drawing against its own viewport.
- The operator window keeps projection actions and the audio transport visible. The service and audio lists share a side panel, and projector setup opens in a dialog. Lists and the projection workspace scroll independently.

## Acceptance

- Import multiple MP3s, reorder and remove tracks, reopen the service, and verify the saved order. Play, pause, seek, change volume, and test Previous, Next, automatic progression, and each repeat mode. Verify missing and unreadable tracks offer recovery through the transport and preflight.
- Select a later thumbnail and Take the deck. Verify live jumps, arrow keys, Black and Restore, and private selection of another deck. Verify audio position stays continuous during navigation.
- At 800×600, 1024×768, 1280×720 and 1366×768, verify independent scrolling, readable controls, long names and dialog controls. Confirm resizing the operator window leaves the audience output unchanged.

## Validation and remaining rehearsal

- Node checks cover playlist order and progression. Rust checks cover the initial Take index, jump token validation, masked navigation, and preservation of the current accompaniment.
- A headless Chrome smoke check of the production frontend with a simulated Tauri bridge passed all four target window sizes and the 580×580 minimum. It exercised dialogs, independent scrolling, text and image thumbnails, private Take, live jumps, keyboard navigation, and Black and Restore.
- The smoke check decoded generated MP3s and exercised multi-file selection, order changes, removal, pause and resume, seek, volume, repeat, automatic progression, and recovery from an unavailable track. It also verified pausing and resuming the playlist around a simulated video Take.
- Thumbnails show a 16:9 preview. Projector text and letterboxing still fit the physical audience display. Rehearse the same workflows in the packaged webviews, including audio output and MP4 handoff, extended and mirrored displays, and resizing the operator window during projection.
