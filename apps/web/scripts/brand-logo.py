#!/usr/bin/env python3
"""The JackiOh logo: its SVGs in apps/web/public/brand, which the game serves, and its PNGs in
photos/logo, which it does not (a production build never reads photos/), drawn here.

Run by hand when the logo changes, and commit what it writes. Needs fontTools and brotli
(`pip install fonttools brotli`) to outline the wordmark from the self-hosted Alegreya (OFL), so no
SVG depends on a font, and Node with Playwright's Chromium (`require("playwright")`) to rasterize:

    python3 apps/web/scripts/brand-logo.py

The mark is two fanned cards: a face-down trap behind, its back the card back's emblem
(src/cards/CardBack.tsx), and a face in front with a gold J and the emblem's four-pointed star. The
wordmark is the landing page's (routes/landing.css): Alegreya 700, 0.01em tracking, its metallic
gold over a #3a1f0c edge. The colors are the palette's (src/palette.css) and the tavern's gold.

    jackioh-logo.*          the mark and the wordmark on one line (transparent)
    jackioh-logo-stacked.*  the mark above the wordmark (transparent)
    jackioh-logo-wood.*     the mark above the wordmark on the tavern's wood, opaque, as the loading screens show it
    jackioh-mark.*          the mark alone, 1024 and 256 px (transparent)
    jackioh-mark-small.*    the front card alone, for 16 to 48 px, where the back card is noise
    jackioh-icon.*          the mark on the tavern's dark wood, opaque, for app and avatar icons
"""

import json
import random
import subprocess
from pathlib import Path

from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

ROOT = Path(__file__).resolve().parent.parent
FONT = ROOT / "public" / "fonts" / "alegreya-5.3.0-latin-wght-normal.woff2"
OUT = ROOT / "public" / "brand"
PNG_OUT = ROOT.parent.parent / "photos" / "logo"

_FACES = {}
def _face(wght):
    if wght not in _FACES:
        f = instantiateVariableFont(TTFont(FONT), {"wght": wght})
        _FACES[wght] = (f.getBestCmap(), f.getGlyphSet(), f["hmtx"])
    return _FACES[wght]

def text_path(text, size, x0=0.0, y0=0.0, tracking=0.0, wght=900):
    """Path data for `text` at `size` px with its baseline at y0, and its ink bounds."""
    _cmap, _gs, _hmtx = _face(wght)
    s = size / 1000.0
    x = 0.0; d = []; bp = BoundsPen(_gs)
    for ch in text:
        g = _cmap[ord(ch)]
        t = (s, 0, 0, -s, x0 + x * s, y0)
        pen = SVGPathPen(_gs); _gs[g].draw(TransformPen(pen, t)); d.append(pen.getCommands())
        _gs[g].draw(TransformPen(bp, t))
        x += _hmtx[g][0] + tracking
    return " ".join(d), bp.bounds  # (xmin, ymin, xmax, ymax) in svg coords

def centered_text(text, size, cx, cy, tracking=0.0):
    _, (x0, y0, x1, y1) = text_path(text, size, 0, 0, tracking)
    return text_path(text, size, cx - (x0 + x1) / 2, cy - (y0 + y1) / 2, tracking)

def star(cx, cy, r, inner):
    """The card back's four-pointed star: points at r, waist at `inner`."""
    pts = [(0, -r), (inner, -inner), (r, 0), (inner, inner), (0, r), (-inner, inner), (-r, 0), (-inner, -inner)]
    return "M" + " L".join(f"{cx + x:.1f} {cy + y:.1f}" for x, y in pts) + " Z"

DEFS = """
<defs>
  <linearGradient id="wordGold" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="#fbe7b0"/><stop offset="0.22" stop-color="#ffe39a"/><stop offset="0.44" stop-color="#f3c969"/>
    <stop offset="0.62" stop-color="#c9953f"/><stop offset="0.82" stop-color="#9b6423"/><stop offset="1" stop-color="#d9a44c"/>
  </linearGradient>
  <linearGradient id="gold" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="#fff6d6"/><stop offset="0.32" stop-color="#fde68a"/>
    <stop offset="0.62" stop-color="#f0c04a"/><stop offset="1" stop-color="#a86a1f"/>
  </linearGradient>
  <linearGradient id="goldEdge" x1="0" y1="0" x2="1" y2="1">
    <stop offset="0" stop-color="#fde68a"/><stop offset="0.5" stop-color="#c9953f"/><stop offset="1" stop-color="#f0c04a"/>
  </linearGradient>
  <radialGradient id="face" cx="0.5" cy="0.4" r="0.75">
    <stop offset="0" stop-color="#3a2f6e"/><stop offset="0.55" stop-color="#241c44"/><stop offset="1" stop-color="#141a2e"/>
  </radialGradient>
  <linearGradient id="back" x1="0" y1="0" x2="1" y2="1">
    <stop offset="0" stop-color="#2b3349"/><stop offset="1" stop-color="#141a2e"/>
  </linearGradient>
  <pattern id="stripes" width="22" height="22" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
    <rect width="22" height="22" fill="none"/><rect width="9" height="22" fill="#ffffff" fill-opacity="0.045"/>
  </pattern>
  <radialGradient id="glow" cx="0.5" cy="0.5" r="0.5">
    <stop offset="0" stop-color="#8b5cf6" stop-opacity="0.45"/><stop offset="1" stop-color="#8b5cf6" stop-opacity="0"/>
  </radialGradient>
  <filter id="shadow" x="-20%" y="-20%" width="140%" height="140%">
    <feDropShadow dx="0" dy="14" stdDeviation="16" flood-color="#0e0905" flood-opacity="0.55"/>
  </filter>
  <filter id="lift" x="-20%" y="-20%" width="140%" height="140%">
    <feDropShadow dx="0" dy="6" stdDeviation="5" flood-color="#0e0905" flood-opacity="0.65"/>
  </filter>
</defs>"""

def card(cx, cy, w, h, angle, face):
    x, y = cx - w / 2, cy - h / 2
    rx = w * 0.11
    inset = w * 0.055
    body = [f'<g transform="rotate({angle} {cx} {cy})" filter="url(#shadow)">',
            f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="url(#{face})"/>']
    if face == "back":
        body.append(f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="url(#stripes)"/>')
        r = w * 0.34
        body += [f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="none" stroke="#d4af37" stroke-opacity="0.75" stroke-width="{w*0.03}"/>',
                 f'<circle cx="{cx}" cy="{cy}" r="{r*0.8}" fill="none" stroke="#d4af37" stroke-opacity="0.4" stroke-width="{w*0.014}" stroke-dasharray="{w*0.04} {w*0.03}"/>',
                 f'<path d="{star(cx, cy, r*0.86, r*0.24)}" fill="#7f6bd0" fill-opacity="0.75" stroke="#e8d28a" stroke-width="{w*0.018}" stroke-linejoin="round"/>',
                 f'<circle cx="{cx}" cy="{cy}" r="{w*0.055}" fill="#ffd24a"/>']
    body += [f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="none" stroke="url(#goldEdge)" stroke-width="{w*0.045}"/>',
             f'<rect x="{x+inset}" y="{y+inset}" width="{w-2*inset}" height="{h-2*inset}" rx="{rx*0.6}" fill="none" stroke="#d4af37" stroke-opacity="0.45" stroke-width="{w*0.012}"/>',
             '</g>']
    return "\n".join(body)

def svg(w, h, inner):
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}">{DEFS}\n{inner}\n</svg>\n'

def front_card(fcx, fcy, ang, W, H, j_size, star_dx, star_dy, star_r):
    jd, _ = centered_text("J", j_size, fcx - W * 0.027, fcy + H * 0.045)
    return (card(fcx, fcy, W, H, ang, "face") +
            f'<g transform="rotate({ang} {fcx} {fcy})">'
            f'<circle cx="{fcx}" cy="{fcy}" r="{W*0.48}" fill="url(#glow)"/>'
            f'<path d="{jd}" fill="url(#gold)" stroke="#2b1800" stroke-width="{j_size*0.019}" stroke-linejoin="round" paint-order="stroke" filter="url(#lift)"/>'
            f'<path d="{star(fcx + star_dx, fcy + star_dy, star_r, star_r*0.215)}" fill="#fff6d6" stroke="#f0c04a" stroke-width="{star_r*0.06}" stroke-linejoin="round" filter="url(#lift)"/>'
            '</g>')

def mark_group():
    """Two fanned cards: a face-down trap behind, and in front a face with the gold J and its star."""
    W, H = 440, 616
    back = card(372, 548, W, H, -15, "back")
    front = front_card(636, 490, 8, W, H, 520, 118, -178, 84)
    # The fan's ink spans about x 82..900, y 150..905: center it and fill the square.
    return f'<g transform="translate(512 512) scale(1.14) translate(-491 -528)">{back}\n{front}</g>'

def mark():
    return svg(1024, 1024, mark_group())

def mark_small():
    """For 16 to 48 px: the front card alone, larger, so the J survives."""
    return svg(1024, 1024, front_card(512, 512, 6, 640, 896, 760, 172, -258, 124))

def app_icon():
    """An opaque square tile (iOS, Android and PWA icons round its corners themselves): the tavern's
    dark wood with a warm light behind the mark."""
    bg = ('<radialGradient id="tile" cx="0.5" cy="0.42" r="0.75"><stop offset="0" stop-color="#4a2a12"/>'
          '<stop offset="0.6" stop-color="#2b170c"/><stop offset="1" stop-color="#120b07"/></radialGradient>')
    inner = (f'<defs>{bg}</defs><rect width="1024" height="1024" fill="url(#tile)"/>'
             f'<g transform="translate(512 520) scale(0.80) translate(-512 -512)">{mark_group()}</g>')
    return svg(1024, 1024, inner)

def wordmark(x0, baseline, size):
    d, b = text_path("JackiOh", size, x0, baseline, tracking=22, wght=700)
    return (f'<path d="{d}" fill="url(#wordGold)" stroke="#3a1f0c" stroke-width="{size*0.02}" stroke-linejoin="round" '
            f'paint-order="stroke" filter="url(#lift)"/>'), b

def logo_horizontal():
    """The mark, then the wordmark, on one line."""
    size = 520
    m = 1024 * 0.86
    word, (x0, y0, x1, y1) = wordmark(0, 0, size)
    gap = 40
    wx = m + gap - x0
    total_w = m + gap + (x1 - x0) + 40
    h = 1024 * 0.86
    baseline = h / 2 - (y0 + y1) / 2  # center the ink vertically
    word, _ = wordmark(wx, baseline, size)
    inner = (f'<g transform="scale(0.86)">{mark_group()}</g>' + word)
    return svg(round(total_w), round(h), inner), round(total_w), round(h)

def stacked_inner():
    """The mark above the wordmark: its contents and size."""
    size = 420
    _, (x0, y0, x1, y1) = wordmark(0, 0, size)
    ww = x1 - x0
    W = max(ww + 120, 1024)
    mark_h = 860
    baseline = mark_h + 10 - y0
    word, _ = wordmark((W - ww) / 2 - x0, baseline, size)
    H = baseline + y1 + 60
    inner = (f'<g transform="translate({(W - 860) / 2} 0) scale({860 / 1024})">{mark_group()}</g>' + word)
    return inner, W, H

def logo_stacked():
    """The mark above the wordmark."""
    inner, W, H = stacked_inner()
    return svg(round(W), round(H), inner), round(W), round(H)

# The tavern's wood (src/auth/tavern.css): its tones, the night at the edges and the flame's light.
WOOD = ["#5a341b", "#63391d", "#4f2d17", "#5e371c", "#553019"]
WOOD_NIGHT, WOOD_FLAME = "#120b07", "#ffb347"

def logo_wood():
    """The stacked logo on the tavern's wood, as the loading screens show it: planks with staggered
    joints and a grain, a warm light behind the logo and the dark at the edges."""
    W, H, plank = 1600, 1200, 120
    rng = random.Random(2747)  # fixed, so a re-run draws the same planks
    planks = []
    for row, y in enumerate(range(0, H, plank)):
        x = -rng.randint(40, 360)
        while x < W:
            length = rng.randint(380, 640)
            tone = WOOD[(row * 3 + len(planks)) % len(WOOD)]
            planks.append(f'<rect x="{x}" y="{y}" width="{length}" height="{plank}" fill="{tone}"/>'
                          f'<rect x="{x}" y="{y}" width="3" height="{plank}" fill="{WOOD_NIGHT}" fill-opacity="0.7"/>')
            x += length
        planks.append(f'<rect x="0" y="{y + plank - 3}" width="{W}" height="3" fill="{WOOD_NIGHT}" fill-opacity="0.75"/>'
                      f'<rect x="0" y="{y}" width="{W}" height="2" fill="#8a5a33" fill-opacity="0.35"/>')
    defs = (f'<defs>'
            f'<filter id="grainDark" x="0" y="0" width="100%" height="100%">'
            f'<feTurbulence type="fractalNoise" baseFrequency="0.0022 0.085" numOctaves="3" seed="11"/>'
            f'<feColorMatrix type="matrix" values="0 0 0 0 0.07  0 0 0 0 0.04  0 0 0 0 0.02  0 0 0 1.6 -0.62"/></filter>'
            f'<filter id="grainLight" x="0" y="0" width="100%" height="100%">'
            f'<feTurbulence type="fractalNoise" baseFrequency="0.003 0.16" numOctaves="2" seed="29"/>'
            f'<feColorMatrix type="matrix" values="0 0 0 0 0.54  0 0 0 0 0.35  0 0 0 0 0.2  0 0 0 1.2 -0.6"/></filter>'
            f'<radialGradient id="woodLight" cx="0.5" cy="0.45" r="0.5">'
            f'<stop offset="0" stop-color="{WOOD_FLAME}" stop-opacity="0.32"/><stop offset="1" stop-color="{WOOD_FLAME}" stop-opacity="0"/></radialGradient>'
            f'<radialGradient id="woodEdge" cx="0.5" cy="0.47" r="0.72">'
            f'<stop offset="0.45" stop-color="{WOOD_NIGHT}" stop-opacity="0"/><stop offset="1" stop-color="{WOOD_NIGHT}" stop-opacity="0.9"/></radialGradient>'
            f'<filter id="logoGlow" x="-20%" y="-20%" width="140%" height="140%">'
            f'<feDropShadow dx="0" dy="0" stdDeviation="40" flood-color="{WOOD_FLAME}" flood-opacity="0.35"/></filter>'
            f'</defs>')
    inner, lw, lh = stacked_inner()
    scale = H * 0.78 / lh
    logo = (f'<g filter="url(#logoGlow)" transform="translate({(W - lw * scale) / 2:.1f} {(H - lh * scale) / 2:.1f}) '
            f'scale({scale:.4f})">{inner}</g>')
    body = (defs + "".join(planks) +
            f'<rect width="{W}" height="{H}" filter="url(#grainDark)"/>'
            f'<rect width="{W}" height="{H}" filter="url(#grainLight)"/>'
            f'<ellipse cx="{W / 2}" cy="{H * 0.45}" rx="{W * 0.42}" ry="{H * 0.42}" fill="url(#woodLight)"/>'
            f'<rect width="{W}" height="{H}" fill="url(#woodEdge)"/>' + logo)
    return svg(W, H, body), W, H

RENDER = r"""
const { chromium } = require("playwright");
const { readFileSync } = require("node:fs");
(async () => {
  const browser = await chromium.launch();
  for (const [input, output, width, height] of JSON.parse(process.argv[1])) {
    const page = await browser.newPage({ viewport: { width, height } });
    const svg = readFileSync(input, "utf8").replace("<svg ", `<svg width="${width}" height="${height}" style="display:block" `);
    await page.setContent(`<!doctype html><body style="margin:0;background:transparent">${svg}</body>`);
    await page.screenshot({ path: output, omitBackground: true, clip: { x: 0, y: 0, width, height } });
    await page.close();
  }
  await browser.close();
})();
"""

def main():
    OUT.mkdir(exist_ok=True)
    PNG_OUT.mkdir(parents=True, exist_ok=True)
    logo, lw, lh = logo_horizontal()
    stacked, sw, sh = logo_stacked()
    wood, ww, wh = logo_wood()
    svgs = {"jackioh-logo": (logo, [(lw, lh, "")]),
            "jackioh-logo-stacked": (stacked, [(sw, sh, "")]),
            "jackioh-logo-wood": (wood, [(ww, wh, "")]),
            "jackioh-mark": (mark(), [(1024, 1024, ""), (256, 256, "-256")]),
            "jackioh-mark-small": (mark_small(), [(32, 32, "-32")]),
            "jackioh-icon": (app_icon(), [(512, 512, "-512"), (180, 180, "-180")])}
    jobs = []
    for name, (text, sizes) in svgs.items():
        (OUT / f"{name}.svg").write_text(text)
        for w, h, suffix in sizes:
            jobs.append([str(OUT / f"{name}.svg"), str(PNG_OUT / f"{name}{suffix}.png"), w, h])
    subprocess.run(["node", "-e", RENDER, json.dumps(jobs)], check=True)
    for _, png, w, h in jobs:
        print(f"{Path(png).relative_to(ROOT.parent.parent)}  {w}x{h}")

if __name__ == "__main__":
    main()
