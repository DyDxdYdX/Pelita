# Brand and operator theme

Pelita uses the supplied wordmark for repository documentation and the symbol without text for the operator header and desktop icons. Source artwork is stored in `app/public/brand/`; generated platform icons live in `app/src-tauri/icons/`. The artwork has a white background, so the symbol sits on a white rounded tile in the operator header and desktop icon.

## Ember & Amethyst palette

| Role | Light | Dark |
| --- | --- | --- |
| Background | `#F9F7FB` | `#180C20` |
| Surface | `#FFFFFF` | `#291832` |
| Primary text | `#291832` | `#F7F0FA` |
| Secondary text | `#6B5872` | `#BBAABD` |
| Border | `#E8DFEC` | `#493450` |
| Primary action | `#431050` with white text | `#FFB01A` with `#291832` text |
| Selection | `#F0E8F3` | `#431050` |

Deep Plum `#431050` is the main brand color. Amethyst `#814670` is a secondary accent. Flame Orange `#FF9302`, Warm Amber `#FFB01A`, and Golden Light `#FFD65A` are reserved for prominent presentation actions and status. The available gradients are flame (`#FF9302` → `#FFB01A` → `#FFD65A`) and purple (`#431050` → `#814670`).

The operator UI follows the operating system's light or dark preference. Preview and live-output frames remain neutral and dark so the slide content stays legible and unaffected by the operator theme. Live state must continue to use text or icons as well as color. Keyboard focus must remain visible in both modes.

## Acceptance criteria

- The README displays the wordmark, and the app header, browser tab, and packaged desktop app display the symbol.
- Light and dark operator colors match the roles above, with readable text and visible focus.
- **Take** and live indicators receive the warm accent; projection content itself is unchanged.
