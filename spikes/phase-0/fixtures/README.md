# Fixture provenance and document probes

All fixtures were authored or synthesized for Pelita. They are cleared for Phase 0 testing. They contain invented titles, simple shapes and generated media. No recordings, lyrics, songbook material, imported images or font files are included. System fonts are referenced by name. See the [separate permissions register](../../../docs/legal/content-permissions.md).

| Fixture | Origin and expected behavior |
| --- | --- |
| Three-slide live deck | `../web/deck.js` uses three original titles and different backgrounds. Change preview while another title is live to expose accidental coupling. |
| `continuous.mp3` | `../generate-media.sh` generates four minutes of uninterrupted 440 Hz audio, 48 kHz mono MP3 at 128 kb/s. Lower output volume before the rehearsal. |
| `candidate.mp4` | The same script generates 12 seconds of moving FFmpeg test pattern at 1920×1080, 30 fps, H.264/yuv420p with 660 Hz AAC-LC audio. The pitch differs from the MP3 for switching checks. |
| `fidelity.pdf` | Three invented title/border pages authored with ReportLab. Page 1 is 576×324 pt, page 2 has a 324×576 pt media box rotated 90° with deliberately sideways text, and page 3 has a 648×396 pt media box cropped to 576×324 pt. The text OUTSIDE CROP must disappear. |
| `fidelity.pptx` | An editable three-slide 16:9 deck authored with Artifact Tool, with Arial text and three accent bars at different heights. The `reference-*.png` files were rendered independently of LibreOffice from the source deck model. |
| App icon | Original colored rectangle on a solid background, supplied in platform-specific formats. |

## PDF raster probe

Create a scratch directory. Run `pdfinfo -f 1 -l 3 -box fidelity.pdf` and `pdftoppm -cropbox -r 72 -png fidelity.pdf "$scratch/pdf"`. Expect three 576×324 PNGs in title order. Check the orientation and border, and verify the crop sentinel is absent. Run Poppler again on a scratch file containing only a PDF header and invalid body. Require a nonzero exit and zero PNGs. This checks the renderer. An app importer still needs staging and atomic commit checks.

## PPTX conversion probe

Disconnect networking. Use LibreOffice with an isolated, fresh temporary profile and output directory. Pass each flag as a separate argument. Replace `$scratch` with an absolute scratch directory without spaces.

```sh
soffice "-env:UserInstallation=file://$scratch/profile" --headless \
  --convert-to pdf:impress_pdf_Export --outdir "$scratch" fidelity.pptx
pdftoppm -png -scale-to-x 480 -scale-to-y 270 "$scratch/fidelity.pdf" "$scratch/pptx"
```

On macOS, the executable can be `/Applications/LibreOffice.app/Contents/MacOS/soffice`. Compare each converted page against its corresponding reference PNG. Check text, line breaks, order, aspect, background and accents, and record font substitutions. This is a trusted synthetic corpus. A temporary profile does not isolate untrusted files. Add an independent Office export and broader corpus before making a support claim.
