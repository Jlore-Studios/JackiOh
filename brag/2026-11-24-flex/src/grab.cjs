// Captures the real Collection's looks as image sequences in slowed page time.
// node grab.cjs <landscape|vertical> [probe]
const fs = require("fs");
const path = require("path");
const { chromium } = require(path.resolve(__dirname, "../../.cache/playwright-cli/0.1.22/node_modules/playwright-core"));
const MODE = process.argv[2] || "landscape";
const PROBE = process.argv[3] === "probe";
const ONLY = !PROBE && process.argv[3] ? process.argv[3].split(",") : null; // shot names to (re)capture
const VIEW = MODE === "vertical" ? { width: 540, height: 960, dsf: 4 } : { width: 1920, height: 1080, dsf: 2 };
const F = 0.1; // page time runs at F of real time while capturing
const FPS = 30;
const CARD = "Nature Titan";
const OUT = `grab-${MODE}`;
// [name, kind, id, frames]: kind "rung" or "strip" (on the card view's face) or "tile" (the Cosmetics tab's preview).
const SHOTS = [
  ["default", "rung", "no-full-art", 1],
  ["alt-art", "rung", "version-alt-art", 1],
  ["alt-full-art", "rung", "version-alt-full-art", 1],
  ["alt-animated", "rung", "version-alt-full-art-animated", 36],
  ["alt-signed", "rung", "version-alt-full-art-signed", 60],
  ["full-art", "rung", "version-full-art", 1],
  ["comic", "rung", "version-comic", 1],
  ["night", "rung", "version-night", 1],
  ["watercolor", "rung", "version-watercolor", 1],
  ["tarot", "rung", "version-tarot", 1],
  ["out-of-frame", "rung", "version-out-of-frame", 1],
  ["foil-cracked-ice", "strip", "foil-cracked-ice", 18],
  ["foil-galaxy", "strip", "foil-galaxy", 18],
  ["foil-molten-gold", "strip", "foil-molten-gold", 18],
  ["foil-void-shimmer", "strip", "foil-void-shimmer", 18],
  ["finish-gilded-edge", "strip", "finish-gilded-edge", 18],
  ["finish-starlit", "strip", "finish-starlit", 18],
  ["border-filigree", "strip", "border-filigree", 18],
  ["border-celestial", "strip", "border-celestial", 18],
  ["back-jlorious", "tile", "card-back-jlorious", 135],
  ["back-nebula", "tile", "card-back-nebula", 30],
  ["table-astral", "tile", "table-astral", 30],
  ["field-cosmic-drift", "tile", "field-cosmic-drift", 45],
];

(async () => {
  fs.mkdirSync(OUT, { recursive: true });
  const browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--mute-audio", "--hide-scrollbars"] });
  const ctx = await browser.newContext({ viewport: { width: VIEW.width, height: VIEW.height }, deviceScaleFactor: VIEW.dsf });
  await ctx.addInitScript(() => {
    localStorage.setItem("jackioh.e2e.session", JSON.stringify({ accessToken: "e2e-token-p1" }));
    // The preview's close button overlaps its picture's corner: keep it (clickable) but unseen in the shots.
    const add = () => { const st = document.createElement("style"); st.textContent = '[data-testid="collection-preview-close"]{opacity:0 !important}'; document.head.appendChild(st); };
    if (document.head) add(); else document.addEventListener("DOMContentLoaded", add);
  });
  const p = await ctx.newPage();
  await p.goto("http://localhost:5173/collection", { waitUntil: "load" });
  await p.waitForSelector('[data-testid="collection-grid"] button', { timeout: 30000 });
  await p.waitForTimeout(1500);
  const openCard = async () => {
    await p.fill('input[placeholder^="Search name"]', CARD);
    await p.waitForTimeout(1200);
    await p.locator('[data-testid="collection-grid"] button').first().click();
    await p.waitForSelector('[data-testid="collection-card-view-face"]');
    await p.waitForTimeout(1500);
  };
  await openCard();
  if (PROBE) {
    await p.screenshot({ path: `${OUT}/probe-view.png` });
    const r = await p.locator('[data-testid="collection-card-view-face"]').boundingBox();
    console.log("face", JSON.stringify(r));
    await browser.close();
    return;
  }
  // Slow the page.
  const cdp = await ctx.newCDPSession(p);
  await cdp.send("Animation.enable");
  await p.evaluate((F) => {
    const rp = performance.now.bind(performance), rd = Date.now.bind(Date);
    const rST = window.setTimeout.bind(window), rSI = window.setInterval.bind(window), rRAF = window.requestAnimationFrame.bind(window);
    const p0 = rp(), d0 = rd();
    performance.now = () => p0 + (rp() - p0) * F;
    Date.now = () => d0 + (rd() - d0) * F;
    window.setTimeout = (fn, ms = 0, ...a) => rST(fn, Math.max(0, Number(ms) || 0) / F, ...a);
    window.setInterval = (fn, ms = 0, ...a) => rSI(fn, Math.max(1, Number(ms) || 0) / F, ...a);
    window.requestAnimationFrame = (cb) => rRAF((ts) => cb(p0 + (ts - p0) * F));
    window.__pageNow = () => (rp() - p0) * F / 1000;
  }, F);
  await cdp.send("Animation.setPlaybackRate", { playbackRate: F });
  const pageNow = () => p.evaluate(() => window.__pageNow());
  const waitPage = (sec) => p.waitForTimeout((sec / F) * 1000);
  const box = async (sel) => {
    const r = await p.locator(sel).boundingBox();
    return { x: Math.floor(r.x), y: Math.floor(r.y), width: Math.ceil(r.width), height: Math.ceil(r.height) };
  };
  const seq = async (name, sel, frames) => {
    fs.mkdirSync(`${OUT}/${name}`, { recursive: true });
    const clip = await box(sel);
    const t0 = await pageNow();
    for (let k = 0; k < frames; k++) {
      while ((await pageNow()) - t0 < k / FPS) await p.waitForTimeout(5);
      await p.screenshot({ path: `${OUT}/${name}/${String(k).padStart(3, "0")}.png`, clip });
    }
    console.log(name, frames, `${clip.width}x${clip.height}`);
  };
  let onTab = "cards";
  for (const [name, kind, id, frames] of SHOTS) {
    if (ONLY && !ONLY.includes(name)) continue;
    if (kind === "tile") {
      if (onTab !== "cosmetics") {
        await p.locator('[data-testid="collection-card-view-close"]').click().catch(() => {});
        await waitPage(0.4);
        await p.locator('[data-testid="collection-tab-cosmetics"]').click();
        await waitPage(1.0);
        onTab = "cosmetics";
      }
      await p.locator('[data-testid="collection-preview-close"]').click({ timeout: 2000 }).catch(() => {});
      await waitPage(0.4);
      const b = p.locator(`[data-testid="collection-tile-${id}"] button`);
      await b.scrollIntoViewIfNeeded();
      await b.focus();
      await p.keyboard.press("i");
      await p.waitForSelector('[data-testid="collection-preview"]');
      await waitPage(0.8);
      await seq(name, '[data-testid="collection-preview"] .collection-preview__picture', frames);
    } else {
      const sel = kind === "rung" ? `[data-testid="collection-rung-${id}"]` : `[data-testid="collection-strip-item-${id}"]`;
      // Focus previews it as hover does ("Hover or focus a cosmetic to see it on the card"), and scrolls nothing.
      await p.mouse.move(2, 2);
      const ok = await p.evaluate((sel) => {
        const el = document.querySelector(sel);
        if (!el) return false;
        const f = el.matches("button,[tabindex]") ? el : el.querySelector("button,[tabindex]");
        (f || el).focus({ preventScroll: true });
        return document.activeElement === (f || el);
      }, sel);
      if (!ok) console.log("no focus", name);
      await waitPage(0.5);
      await seq(name, '[data-testid="collection-card-view-face"]', frames);
    }
  }
  await browser.close();
})().catch((e) => { console.error(e.stack.slice(0, 1500)); process.exit(1); });
