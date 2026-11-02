# Photos

Images to look at, share and reuse. No build reads this folder, so nothing here ships with the game:
the site is `apps/web/dist`.

## `logo/`

The JackiOh logo as PNGs, drawn by `apps/web/scripts/brand-logo.py`, which also writes the SVGs the
game shows into `apps/web/public/brand` (the web README says how).

| File | What |
|---|---|
| `jackioh-logo.png` | The mark and the wordmark on one line, transparent (2714×881) |
| `jackioh-logo-stacked.png` | The mark above the wordmark, transparent (1536×1306) |
| `jackioh-logo-wood.png` | The mark above the wordmark on the tavern's wood, as the loading screens show it (1600×1200) |
| `jackioh-mark.png`, `jackioh-mark-256.png` | The mark alone, transparent (1024 and 256 px) |
| `jackioh-mark-wood.png` | The mark alone, the two cards, on the tavern's wood (1024×1024) |
| `jackioh-mark-small-32.png` | The front card alone, for 16 to 48 px |
| `jackioh-icon-512.png`, `jackioh-icon-180.png` | The mark on dark wood, opaque, for app and avatar icons |
| `preview-*.png` | The lockup on dark and light, and the icons at their real small sizes |

A PNG the game needs (a favicon, say) is copied into `apps/web/public` by the change that uses it.

## `screens/`

Screenshots of the client at 1280×720 and 390×844, taken when the logo went in: a loading screen
(the account check), the 404, and the Shop's, Replays' and Statistics' skeletons mid-load.
