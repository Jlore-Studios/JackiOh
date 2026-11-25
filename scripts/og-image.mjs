// Rebuilds apps/web/public/og.png: run `node scripts/og-image.mjs`. Needs google-chrome on PATH.
//
// The share image `og:image` points to. It renders one 1200x630 HTML page in headless
// Chrome and screenshots it at deviceScaleFactor 1. The site and its images carry no
// tagline (#4653): the left column holds only the wordmark and the plain sentence,
// vertically centered so no line looks missing.

import { spawnSync } from "node:child_process";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const OG_WIDTH = 1200;
const OG_HEIGHT = 630;

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const OUT = join(ROOT, "apps", "web", "public", "og.png");
const ALEGREYA = join(ROOT, "apps", "web", "public", "fonts", "alegreya-5.3.0-latin-wght-normal.woff2");
const INTER = join(ROOT, "apps", "web", "public", "fonts", "inter-5.3.0-latin-wght-normal.woff2");

// Five fanned card backs, left to right: arc centers and leans measured off the original fan.
const CARD_W = 132;
const CARD_H = 204;
const CARDS = [
  { cx: 743, cy: 322, rotate: -20 },
  { cx: 816, cy: 312, rotate: -10 },
  { cx: 890, cy: 304, rotate: 0 },
  { cx: 964, cy: 312, rotate: 10 },
  { cx: 1037, cy: 322, rotate: 20 },
];

const EMBLEM = `<svg viewBox="0 0 120 120" width="116" height="116" aria-hidden="true">
<circle cx="60" cy="60" r="52" fill="none" stroke="#c9a24a" stroke-width="3"/>
<circle cx="60" cy="60" r="44" fill="none" stroke="#c9a24a" stroke-width="1" stroke-dasharray="4 5" opacity="0.8"/>
<path d="M60 12 L70 50 L108 60 L70 70 L60 108 L50 70 L12 60 L50 50 Z" fill="#7e75c9" stroke="#d9d4f5" stroke-width="2.5" stroke-linejoin="round"/>
<circle cx="60" cy="60" r="10" fill="#d9a441" stroke="#7a5a1e" stroke-width="2"/>
</svg>`;

const html = `<!doctype html>
<html><head><meta charset="utf-8" />
<style>
@font-face {
  font-family: "Alegreya";
  font-weight: 100 900;
  font-style: normal;
  src: url("file://${ALEGREYA}") format("woff2");
}
@font-face {
  font-family: "Inter";
  font-weight: 100 900;
  font-style: normal;
  src: url("file://${INTER}") format("woff2");
}
* { margin: 0; padding: 0; box-sizing: border-box; }
html, body { width: ${OG_WIDTH}px; height: ${OG_HEIGHT}px; overflow: hidden; }
#og {
  position: relative;
  width: ${OG_WIDTH}px;
  height: ${OG_HEIGHT}px;
  overflow: hidden;
  background:
    radial-gradient(1000px 480px at 46% 112%, rgba(140, 90, 38, 0.60), rgba(140, 90, 38, 0) 65%),
    radial-gradient(1000px 420px at 50% -12%, rgba(60, 36, 18, 0.35), rgba(60, 36, 18, 0) 60%),
    linear-gradient(180deg, #0a0605 0%, #0e0806 70%, #120b07 100%);
}
#floor {
  position: absolute;
  left: 0; right: 0; bottom: 0;
  height: 190px;
  background:
    linear-gradient(180deg, transparent 0%, transparent 40%, rgba(90, 55, 25, 0.30) 70%, rgba(20, 12, 6, 0.55) 100%);
}
#seams {
  position: absolute;
  left: 0; right: 0; bottom: 0;
  height: 90px;
  background: repeating-linear-gradient(90deg, transparent 0 174px, rgba(0, 0, 0, 0.25) 174px 176px);
  background-position: 72px 0;
}
.copy {
  position: absolute;
  left: 42px;
  top: 0;
  bottom: 0;
  width: 560px;
  display: flex;
  flex-direction: column;
  justify-content: center;
  /* No eyebrow line anymore: the column stays centered, biased a little high so the
     wordmark sits near where it did instead of dropping to the exact middle. */
  padding-bottom: 60px;
}
.wordmark {
  font-family: "Alegreya", Georgia, serif;
  font-weight: 800;
  font-size: 110px;
  line-height: 1.1;
  background: linear-gradient(180deg, #fdf3c4 0%, #f3cd72 40%, #d99a35 70%, #8f5c1a 100%);
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
  filter: drop-shadow(0 0 12px rgba(217, 154, 53, 0.35)) drop-shadow(0 2px 2px rgba(0, 0, 0, 0.6));
}
.sentence {
  margin-top: 14px;
  max-width: 440px;
  font-family: "Inter", Verdana, sans-serif;
  font-size: 15px;
  line-height: 1.75;
  color: #bcae92;
}
.card {
  position: absolute;
  width: ${CARD_W}px;
  height: ${CARD_H}px;
  border-radius: 10px;
  border: 2px solid #a8853f;
  background:
    repeating-linear-gradient(45deg, rgba(255, 255, 255, 0.05) 0 3px, transparent 3px 8px),
    linear-gradient(160deg, #2a3359 0%, #242a4d 45%, #1d2340 100%);
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.55);
  display: flex;
  align-items: center;
  justify-content: center;
}
</style></head>
<body>
<div id="og">
  <div id="floor"></div>
  <div id="seams"></div>
  <div class="copy">
    <div class="wordmark">JackiOh</div>
    <p class="sentence">Summon units into lanes, set traps face down in your backrow, and break the other hero before they break yours.</p>
  </div>
  ${CARDS.map(
    (card) =>
      `<div class="card" style="left: ${card.cx - CARD_W / 2}px; top: ${card.cy - CARD_H / 2}px; transform: rotate(${card.rotate}deg);">${EMBLEM}</div>`,
  ).join("\n  ")}
</div>
</body></html>
`;

const dir = mkdtempSync(join(tmpdir(), "og-image-"));
const page = join(dir, "og.html");
writeFileSync(page, html);
const result = spawnSync(
  "google-chrome",
  [
    "--headless",
    "--disable-gpu",
    "--no-sandbox",
    "--hide-scrollbars",
    "--allow-file-access-from-files",
    "--force-device-scale-factor=1",
    `--window-size=${OG_WIDTH},${OG_HEIGHT}`,
    "--virtual-time-budget=8000",
    `--screenshot=${OUT}`,
    `file://${page}`,
  ],
  { stdio: "inherit" },
);
if (result.status !== 0) {
  throw new Error(`google-chrome exited with status ${String(result.status)}`);
}
console.log(`wrote ${OUT} (${OG_WIDTH}x${OG_HEIGHT})`);
