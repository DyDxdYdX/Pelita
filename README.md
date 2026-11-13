# Pelita

**A simpler way to prepare and present a worship service.**

Pelita is a free, open-source desktop presentation app built around church worship. It is shaped by Seventh-day Adventist Sabbath services, and other churches are welcome to use it. Pelita is an independent community project, not an official church application.

## Why Pelita?

Church volunteers often juggle slides, music, videos, and a projector during one service. Pelita brings those local files into a reusable service playlist, with a private preview and clear controls for what the congregation sees. You can prepare and present without an internet connection. The current release is an **alpha** for trying and rehearsing; check your own equipment and media before relying on it in a live service.

## What you can do now

- Create, reorder, duplicate, and reuse service playlists for songs, Scripture, announcements, and sermon material.
- Make text slides for lyrics or readings; add an MP3 accompaniment and advance the slides yourself while it plays.
- Import PNG or JPEG images and static PDF pages as slides. Play local MP4 videos or MP3 audio.
- Select an item in a private **Preview**, then press **Take** to put it on the projector. Move through live slides with buttons or keyboard shortcuts.
- Use **Black**, **Restore**, and **Clear** to manage the audience display. Choose an extended projector display, or use the **Single / mirrored** presentation mode.
- Run **preflight** to find unavailable media and relink a file or skip an affected item.

Supported playback can vary with the computer's media decoder and display setup. See [media support](docs/user/media-support.md) and [presenting](docs/user/presenting.md) before a service.

## Download and installation

[Download Pelita from GitHub Releases](https://github.com/DyDxdYdX/Pelita/releases).

The published **v0.1.0-alpha.2** release includes a Windows x64 installer (`.exe`), macOS installers for Apple Silicon and Intel (`.dmg`), and Linux x64 packages (`.AppImage` and `.deb`). Choose the file for your computer from that release's **Assets** list. These are experimental builds; platform and hardware validation is still in progress. The Windows installer is unsigned, and the macOS builds are not notarized, so your operating system may ask you to review the app before opening it. Keep normal system security protections enabled.

For installation and first-use steps, see the [Getting Started guide](docs/user/getting-started.md).

## Get started

1. Install and open Pelita.
2. Select **New service** and name your worship service, or use the existing service.
3. Add **Text slides** and **Import files**, then put the playlist in service order.
4. Connect your projector, choose it under **Projector**, and select **Confirm display**. For a shared screen, use **Single / mirrored**.
5. Select an item, check the private **Preview**, then press **Take** when you are ready to show it.

Before Sabbath worship, run **Run preflight** and rehearse the service. The [user guide](docs/user/README.md) walks through preparation, presenting, and recovery.

## Screenshots

Verified application screenshots have not yet been added. If you can provide screenshots from a current build using rights-cleared sample content, see [Contributing](CONTRIBUTING.md). We will add them here after checking that they match the released interface.

## Roadmap

Planned work includes a searchable song library, more presentation customization, and installable content extensions. A future extension marketplace is a longer-term idea. Optional LPMI songbook integration depends on the necessary permissions; LPMI content is **not** bundled with Pelita. See the [product plans](docs/developer/README.md) for detail.

## Contributing

Developers, testers, translators, documentation writers, and church volunteers can all help. Try a rehearsal, report what confused you, or suggest a clearer workflow. Start with [CONTRIBUTING.md](CONTRIBUTING.md).

## License and content rights

Pelita's own source code is released under the [MIT license](LICENSE). That license does not grant rights to third-party lyrics, hymnbooks, recordings, videos, or images. Use material you have permission to use; no LPMI distribution permission is claimed. See [content permissions](docs/legal/content-permissions.md).

## Documentation

- [User Guide](docs/user/README.md)
- [FAQ](docs/user/faq.md)
- [Troubleshooting](docs/user/troubleshooting.md)
- [Contributing](CONTRIBUTING.md)
- [Developer Documentation](docs/developer/README.md)
