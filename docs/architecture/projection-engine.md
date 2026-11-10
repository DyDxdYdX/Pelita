# Projection engine

Supports PRJ-01 through PRJ-04. Keep display lifecycle, visual masking and transport as independent state. A prepared item contains immutable ordered frames or a video reference and optional accompaniment.

| Intent | Expected behavior |
| --- | --- |
| Preview | Changes the operator selection only. |
| Take | Requires ready content and confirmed display. Replaces the live item at its first frame and starts its configured transport once. A successful Take reveals the new frame. |
| Next / Previous | Clamp at first and last frame. Update the current frame even while masked. Audio identity and playback position remain stable. |
| Black / Restore | Toggle a black overlay while retaining the current frame. Audio continues and video transport retains its timeline. |
| Clear | Remove live frame, stop transport, and show a neutral black canvas. Restore after Clear has no effect. |
| Display lost / window closed | Suspend visual output, retire the old projector window generation and alert the operator. Accompaniment remains under explicit operator control. Video hosted by the lost window stops. |
| Recover | Re-enumerate, confirm target, then open in black. Require a new Take. For the same suspended item with surviving accompaniment, preserve that transport unless the operator explicitly restarts it. |

Use a projector-ready handshake before revealing content. Number snapshots monotonically per session and scope acknowledgments to a window generation. On topology change verify the current target again and hide or close a displaced window where the platform permits. Use native signals where reliable and a low-frequency recheck as a fallback. An OS may relocate the window before the app observes a disconnect. Test that exposure window on physical hardware. Store best-effort target descriptors and treat collisions and stale matches as requiring operator confirmation. Keep clear/black state visibly distinct in the operator UI.

| Platform | Phase 0 probe and fallback |
| --- | --- |
| Windows | Compare fullscreen with borderless bounds on mixed-DPI displays and negative coordinates. Validate hot-plug, target changes and WebView2 composition. |
| macOS | Compare native fullscreen Spaces with Tauri simple fullscreen on a supported version and a borderless window. Check separate Spaces settings, focus return, Dock/menu-bar behavior and reconnection. |
| Linux X11 | Validate placement and fullscreen under the selected window manager. Record desktop and scale. |
| Linux Wayland | Treat placement and fullscreen as compositor-mediated. Compare supported fullscreen requests and manual placement before fullscreen. If targeting is unreliable, certify only the modes that pass and document session-specific limitations. |
| Mirror / single display | The OS shares the desktop view. Warn the operator and use content-only fullscreen with local keyboard controls and a deliberate exit. Avoid assuming mirror detection is portable. |

Render against the actual projector viewport with aspect-preserving fit and letterboxing. Keep black/clear controls keyboard reachable in both operator and mirrored modes and suppress normal slide shortcuts inside text editing controls. [Tauri window API](https://docs.rs/tauri/latest/tauri/window/struct.Window.html) · [Tauri window backend platform notes](https://docs.rs/tauri-winit/latest/winit/window/struct.Window.html)
