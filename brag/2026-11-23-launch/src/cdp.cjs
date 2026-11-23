// Connects to the headless Chrome on :9333 and runs one page function: `node cdp.cjs <file.js|-e code> [shot.png]`.
// The page function is an async arrow `async () => …` evaluated in /dev/hotseat's page.
const path = require("path");
const fs = require("fs");
const { chromium } = require(path.resolve(__dirname, "../../.cache/playwright-cli/0.1.22/node_modules/playwright-core"));

async function page() {
  const browser = await chromium.connectOverCDP("http://127.0.0.1:9333");
  const ctx = browser.contexts()[0];
  const p = ctx.pages()[0] ?? (await ctx.newPage());
  return { browser, p };
}

async function main() {
  const [, , a, b, c] = process.argv;
  const { browser, p } = await page();
  let code;
  if (a === "-e") code = b;
  else if (a === "goto") {
    await p.goto(b, { waitUntil: "load" });
    console.log("at", p.url());
    await browser.close();
    return;
  } else if (a === "shot") {
    await p.screenshot({ path: b });
    console.log("shot", b);
    await browser.close();
    return;
  } else code = fs.readFileSync(a, "utf8");
  const out = await p.evaluate(`(${code})()`);
  console.log(typeof out === "string" ? out : JSON.stringify(out, null, 1));
  const shot = a === "-e" ? c : b;
  if (shot) await p.screenshot({ path: shot });
  await browser.close();
}

module.exports = { page };
if (require.main === module) main().catch((e) => { console.error(e.message); process.exit(1); });
