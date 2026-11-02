import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const BRAND = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "public", "brand");

// #2845: WebKit rasterizes whatever an SVG filter touches at 1x and scales it up, so an
// feDropShadow blurred the whole logo on a phone. The wood variants keep their grain and
// glow filters (no screen shows them); every other brand SVG must be filter-free.
describe("#2845 the brand stays sharp on WebKit", () => {
  it("#2845 no shipped brand SVG but the two -wood ones uses an SVG filter", () => {
    const files = readdirSync(BRAND).filter((f) => f.endsWith(".svg"));
    for (const name of [
      "jackioh-card-back.svg",
      "jackioh-icon.svg",
      "jackioh-logo-stacked.svg",
      "jackioh-logo.svg",
      "jackioh-mark-small.svg",
      "jackioh-mark.svg",
    ]) {
      expect(files).toContain(name);
    }
    expect(files.filter((f) => f.endsWith("-wood.svg")).sort()).toEqual([
      "jackioh-logo-wood.svg",
      "jackioh-mark-wood.svg",
    ]);
    for (const file of files) {
      if (file.endsWith("-wood.svg")) continue;
      const text = readFileSync(join(BRAND, file), "utf8");
      expect(text, file).not.toContain("<filter");
      expect(text, file).not.toContain("filter=");
    }
  });
});
