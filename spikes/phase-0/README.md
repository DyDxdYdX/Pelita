# Pelita Phase 0 spike

This disposable Tauri package has two local webviews and no production data. See the [Phase 0 record](../../docs/development/phase-0.md) for the hardware matrix, rehearsal, decision gates and results.

1. Install [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/), Node.js and FFmpeg with the libmp3lame and libx264 encoders.
2. From this directory, run `sh generate-media.sh`.
3. Run `npx --yes @tauri-apps/cli@2.12.4 build`. Record the exact CLI version and use the platform bundle for the rehearsal. Preserve the Cargo lockfile.
4. Extend the desktop, disable mirroring and start the package with networking disabled. Select the projector, confirm it, and wait for black output. Pick a private slide and press Take. Start the MP3 with the operator audio controls. The audio host stays in the operator window. The webviews load these bounded fixtures into local blobs for playback and seeking. After a disconnect, refresh the displays, confirm the target and Take again.

Take video requires a decoded frame and stops the MP3. Video autoplay policies are part of the probe. Clear stops transport. A slide Take leaves the MP3 transport under operator control. The spike polls topology and snapshots, so use two-screen video to detect any transient content exposure during unplug. The package handles extended displays only. Run each supported topology using the [rehearsal matrix](../../docs/development/phase-0.md).

Run the [document probes](fixtures/README.md) separately. They use synthetic documents and do not participate in live playback.
