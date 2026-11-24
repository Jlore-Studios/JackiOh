// Shared: playwright-core from the repo's Playwright CLI, and a page on the served client.
const path = require("path");
const { chromium } = require(path.resolve(__dirname, "../../.cache/playwright-cli/0.1.22/node_modules/playwright-core"));
async function open({ width = 1920, height = 1080, dsf = 1 } = {}) {
  const browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--mute-audio", "--hide-scrollbars"] });
  const ctx = await browser.newContext({ viewport: { width, height }, deviceScaleFactor: dsf });
  await ctx.addInitScript(() => {
    const add = () => { const s = document.createElement("style"); s.textContent = ".hotseat-bar{display:none !important}"; document.head.appendChild(s); };
    if (document.head) add(); else document.addEventListener("DOMContentLoaded", add);
  });
  const p = await ctx.newPage();
  return { browser, ctx, p };
}
// Mounts a real component of the client in a fixed layer over a page, with the React the app itself loaded.
const MOUNT = async ({ src, name, propsList, css }) => {
  const urls = performance.getEntriesByType("resource").map((e) => e.name);
  const reactUrl = urls.find((u) => /\.vite\/deps\/react\.js/.test(u));
  const domUrl = urls.find((u) => /\.vite\/deps\/react-dom_client\.js/.test(u));
  if (!reactUrl || !domUrl) return { error: "no react urls", urls: urls.filter((u) => u.includes(".vite/deps")).slice(0, 20) };
  const React = (await import(reactUrl)).default ?? (await import(reactUrl));
  const dm = await import(domUrl); const createRoot = dm.createRoot ?? dm.default.createRoot;
  const mod = await import(src);
  const C = name === "default" ? mod.default : mod[name];
  let layer = document.getElementById("brag-layer");
  if (!layer) {
    layer = document.createElement("div");
    layer.id = "brag-layer";
    layer.style.cssText = "position:fixed;inset:0;z-index:2147483647;background:#120c08;display:flex;flex-wrap:wrap;gap:24px;align-items:center;justify-content:center;padding:24px;color:#efe0bf;" + (css || "");
    document.body.appendChild(layer);
  }
  layer.innerHTML = "";
  const holder = document.createElement("div");
  layer.appendChild(holder);
  const root = createRoot(holder);
  root.render(React.createElement(React.Fragment, null, ...propsList.map((pr, i) => React.createElement("div", { key: i, className: "brag-item", "data-i": i, style: pr.__style || {} }, React.createElement(C, pr)))));
  await new Promise((r) => setTimeout(r, 400));
  return { ok: true, n: propsList.length };
};
module.exports = { open, MOUNT };
