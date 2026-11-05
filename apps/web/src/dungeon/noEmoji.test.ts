// #3229: nothing under dungeon/ draws a system emoji. They look different on every platform, ignore
// the palette and have no accessible name of their own; the map's node icons are SVGs
// (map/NodeIcon.tsx) and a new one adds a glyph there, never a character. The pattern is written
// in escapes so this file holds none either.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const DUNGEON = dirname(fileURLToPath(import.meta.url));

/** A pictograph, an emoji presentation, a flag's regional indicator, or a variation selector, joiner or keycap mark. */
const EMOJI = /\p{Extended_Pictographic}|\p{Emoji_Presentation}|\p{Regional_Indicator}|\u{FE0F}|\u{200D}|\u{20E3}/u;

function sources(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return sources(path);
    return /\.(?:tsx?|css)$/.test(name) ? [path] : [];
  });
}

describe("#3229 the dungeon holds no emoji", () => {
  it("#3229 nothing under dungeon/ holds an emoji code point", () => {
    const files = sources(DUNGEON).map((path) => relative(DUNGEON, path).replaceAll("\\", "/")).sort();
    expect(files.length).toBeGreaterThan(10);
    expect(files).toContain("map/MapScreen.tsx");
    const found: string[] = [];
    for (const file of files) {
      readFileSync(join(DUNGEON, file), "utf8")
        .split("\n")
        .forEach((line, index) => {
          if (EMOJI.test(line)) found.push(`${file}:${String(index + 1)}`);
        });
    }
    expect(found, "draw it as an SVG (map/NodeIcon.tsx), not a system emoji").toEqual([]);
  });

  it("#3229 the scan sees emoji, VS16 and flags but not plain text", () => {
    const emoji = [
      String.fromCodePoint(0x2694, 0xfe0f),
      String.fromCodePoint(0x1f6e1),
      String.fromCodePoint(0x26e9, 0xfe0f),
      String.fromCodePoint(0x1f1fa, 0x1f1f8),
    ];
    for (const sample of emoji) expect(EMOJI.test(sample), sample.codePointAt(0)?.toString(16)).toBe(true);
    for (const plain of ["Fight 1", "Relic: {name}", String.fromCodePoint(0x2192)]) {
      expect(EMOJI.test(plain), plain).toBe(false);
    }
  });
});
