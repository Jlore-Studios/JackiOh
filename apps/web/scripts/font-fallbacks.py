#!/usr/bin/env python3
"""The metric-matched fallbacks for the web fonts (apps/web/src/fonts.css), computed.

Run by hand when a font file in apps/web/public/fonts changes, and copy the printed descriptors into
fonts.css's two fallback @font-face rules. Needs fontTools and brotli (`pip install fonttools brotli`)
and the Liberation fonts, which are metric-compatible with Arial and Times New Roman, the local
faces the fallbacks are built on:

    python3 apps/web/scripts/font-fallbacks.py [/usr/share/fonts/truetype/liberation]

A fallback is the local face scaled so that a line of English text is as wide as in the web font
(`size-adjust`: the ratio of their average advance over English letter frequencies, space included,
at the default weight), with the web font's own ascent, descent and line gap (each divided by that
scale, since the overrides apply to the scaled face). So a card's text measures the same before and
after the web font arrives, and `useFitText` does not refit it when it swaps in.
"""

import sys
from pathlib import Path

from fontTools.ttLib import TTFont

ROOT = Path(__file__).resolve().parent.parent
FONTS = ROOT / "public" / "fonts"

# English letter frequencies (per cent), with the space between words; digits and punctuation are
# rare enough in card text to leave out.
FREQUENCIES = {
    " ": 18.0, "e": 10.2, "t": 7.5, "a": 6.5, "o": 6.2, "i": 5.7, "n": 5.7, "s": 5.3, "r": 5.0,
    "h": 4.9, "l": 3.3, "d": 3.4, "c": 2.3, "u": 2.3, "m": 2.0, "f": 1.8, "p": 1.6, "g": 1.6,
    "w": 1.7, "y": 1.6, "b": 1.2, "v": 0.8, "k": 0.6, "x": 0.1, "j": 0.1, "q": 0.1, "z": 0.1,
}

PAIRS = [
    ("Inter", "inter-5.3.0-latin-wght-normal.woff2", "LiberationSans-Regular.ttf", "Arial"),
    ("Alegreya", "alegreya-5.3.0-latin-wght-normal.woff2", "LiberationSerif-Regular.ttf", "Times New Roman"),
]


def average_advance(font: TTFont) -> float:
    """The frequency-weighted advance of one character, in em."""
    cmap = font.getBestCmap()
    metrics = font["hmtx"].metrics
    upm = font["head"].unitsPerEm
    total = sum(FREQUENCIES.values())
    width = sum(metrics[cmap[ord(ch)]][0] * share for ch, share in FREQUENCIES.items())
    return width / total / upm


def vertical(font: TTFont) -> tuple[float, float, float]:
    """Ascent, descent and line gap in em, as browsers read them (typo metrics when the font says so)."""
    upm = font["head"].unitsPerEm
    os2 = font["OS/2"]
    if os2.fsSelection & (1 << 7):
        return os2.sTypoAscender / upm, -os2.sTypoDescender / upm, os2.sTypoLineGap / upm
    hhea = font["hhea"]
    return hhea.ascent / upm, -hhea.descent / upm, hhea.lineGap / upm


def main() -> None:
    local = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("/usr/share/fonts/truetype/liberation")
    for family, web_file, local_file, local_name in PAIRS:
        web = TTFont(FONTS / web_file)
        fallback = TTFont(local / local_file)
        scale = average_advance(web) / average_advance(fallback)
        ascent, descent, gap = vertical(web)
        print(f'/* "{family} Fallback": {local_name}, from {web_file} */')
        print(f"  size-adjust: {scale * 100:.2f}%;")
        print(f"  ascent-override: {ascent / scale * 100:.2f}%;")
        print(f"  descent-override: {descent / scale * 100:.2f}%;")
        print(f"  line-gap-override: {gap / scale * 100:.2f}%;")


if __name__ == "__main__":
    main()
