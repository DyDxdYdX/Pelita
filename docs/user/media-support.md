# Media support

Pelita uses managed local copies, so prepared services work offline. Keep the originals and a backup of your service materials.

| File | How Pelita uses it |
| --- | --- |
| PNG, JPG, JPEG | Static image slides. Images selected together form one slide item. |
| PDF | Static pages converted into slides through **Import files**. |
| PPTX | One slide item per PowerPoint file, with ordered thumbnails and a retained original. |
| MP3 | Standalone playback through **Audio playlist**, or accompaniment through **Attach MP3**. |
| MP4 | A video item. Playback depends on your computer's decoder. |

## Import a PowerPoint presentation

1. In **Playlist**, choose **Import PowerPoint** and select one or more `.pptx` files. You can also drop PPTX files onto the operator workspace.
2. Watch the status bar. **Cancel import** stops the remaining work and keeps presentations already added. If a file fails, open **PowerPoint import results** for its message and **Retry failed files** after making the correction.
3. Select the new **PPTX** item and inspect its **Slides** thumbnails and conversion warnings. Select a thumbnail to start there, then press **Take**. On the live item, click a thumbnail or use the live arrows to move immediately.
4. For a separate instrumental track, start an MP3 from **Audio playlist**. It continues during slide navigation and visual-only Takes. Use **Attach MP3** if you want that file to start when the presentation is taken.

## Choose a converter

Open **Presentation conversion settings** using the sliders icon in the top bar. **Automatic** prefers an operational Microsoft PowerPoint desktop installation on Windows and can use LibreOffice if PowerPoint is unavailable or its export fails. On macOS and Linux, install LibreOffice with Impress. The settings show detected versions and errors. Use **Rescan** after installing a converter or **Choose executable** to point Pelita to `soffice` or `soffice.com` for a custom installation.

Select a specific provider to use it for imports. If it fails, Pelita shows the error and an alternative when available. Choose **Automatic** again to allow fallback. PowerPoint needs an activated Windows desktop installation. [Download LibreOffice](https://www.libreoffice.org/download/download-libreoffice/)

## Review, reuse, and regenerate

Pelita saves the original PPTX and prepared slide images in its data directory. A prepared item works after a restart, with the USB drive disconnected, or with the converter removed. Importing the same file reuses a verified prepared copy when it matches the provider preference. Pelita keeps that managed copy if you remove an item from one service.

If an image is missing or the rendering needs improvement, select **Regenerate slides** on the PPTX item. Use the application setting or pick a provider for this run. Pelita keeps the current slides until the new conversion succeeds. Duplicated service items and anything already live keep their own prepared slides.

PowerPoint import supports static text, images, backgrounds, shapes, tables, charts, and custom slide sizes. Font substitution and some effects can change the result. Check the slides on your worship computer. Animations, transitions, macros, notes, embedded audio/video playback, password protection, and legacy `.ppt` files are outside the import workflow. If a PPTX links to external content, embed it or export a static PDF and use **Import files**.

## Limits

- PPTX files can be up to 128 MiB and 100 slides. Slide images fit within 1920×1080 and keep their proportions.
- PDFs can be up to 64 MiB and 100 pages. Individual images can be up to 24 MiB. A media import can contain up to 100 prepared files and 512 MiB.
- Test MP3 and MP4 decoding on the worship computer. The current pilot expects common H.264/AAC MP4 files.
