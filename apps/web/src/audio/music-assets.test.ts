// R631 (SPEC §10.11 "Music"): the rendered music on disk, its manifest, its size cap, its licence
// record, the cards that play it, and `gen-music.mjs --check`. R1352: every Legendary's and Mythic's
// own intro, tokens printed so included, over the sets that ship.

import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, openSync, closeSync, ftruncateSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { SHIPPED_SETS, type SetName } from "@jackioh/shared";
import { afterAll, describe, expect, it } from "vitest";

import { MUSIC_BUDGET_BYTES, MUSIC_INTRO_MAX_S, MUSIC_INTRO_MIN_S, MUSIC_STATIONS } from "./constants.ts";
import { MENU_TRACK, MUSIC_CARDS, MUSIC_MANIFEST, RESULT_TRACKS, dangerTrack, parseMusicCards, parseMusicManifest, startTrack, stationTracks } from "./musicData.ts";

const here = dirname(fileURLToPath(import.meta.url));
const WEB = resolve(here, "../..");
const REPO = resolve(WEB, "../..");
const MUSIC_DIR = join(WEB, "public/audio/music");
const GEN_MUSIC = join(WEB, "scripts/gen-music.mjs");
const LICENSES = join(REPO, "assets/music/LICENSES.md");
const CATALOG = JSON.parse(readFileSync(join(REPO, "crates/cards/catalog.json"), "utf8")) as Record<
  string,
  { set: SetName; rarity?: string; token?: boolean; printedRarity?: string }
>;
const BLOCK = 4096;
const CHECK_TIMEOUT_MS = 60_000;

const entries = Object.entries(MUSIC_MANIFEST.files);
const scratch: string[] = [];

afterAll(() => {
  for (const dir of scratch) rmSync(dir, { recursive: true, force: true });
});

/**
 * R1352: the cards of `sets` that open their play with an intro of their own: every Legendary and
 * Mythic, and every token printed Legendary or Mythic. The release that ships a set (the Meditative
 * set's 22 come with MR) is what adds it to the list this is called with.
 */
function introCards(sets: readonly SetName[]): string[] {
  return Object.entries(CATALOG)
    .filter(([, c]) => sets.includes(c.set))
    .filter(([, c]) => {
      const rarity = c.token === true ? c.printedRarity : c.rarity;
      return rarity === "Legendary" || rarity === "Mythic";
    })
    .map(([id]) => id);
}

/** Every set the catalog holds, shipped or not. */
const CATALOG_SETS = [...new Set(Object.values(CATALOG).map((c) => c.set))];

function check(root?: string): { status: number | null; out: string } {
  const result = spawnSync(process.execPath, [GEN_MUSIC, "--check", ...(root === undefined ? [] : ["--root", root])], { encoding: "utf8" });
  return { status: result.status, out: `${result.stdout}${result.stderr}` };
}

describe("R631 the rendered music", () => {
  it("R631 has a file for every manifest entry, of the manifest's size, each an M4A", () => {
    for (const [id, track] of entries) {
      const file = join(MUSIC_DIR, `${id}.m4a`);
      expect(statSync(file).size, id).toBe(track.bytes);
      const head = readFileSync(file).subarray(0, 12);
      expect(head.toString("latin1", 4, 8), `${id} ftyp`).toBe("ftyp");
      expect(head.toString("latin1", 8, 12), `${id} brand`).toBe("M4A ");
    }
    const onDisk = readdirSync(MUSIC_DIR).filter((f) => f.endsWith(".m4a")).map((f) => f.slice(0, -4)).sort();
    expect(onDisk).toEqual(entries.map(([id]) => id).sort());
  });

  it("R631 stays under its size cap, counted in whole disk blocks", () => {
    const total = entries.reduce((sum, [, t]) => sum + Math.ceil(t.bytes / BLOCK) * BLOCK, 0);
    expect(total).toBeLessThanOrEqual(MUSIC_BUDGET_BYTES);
  });

  it("R631 gives every station two in-game tracks, a low-health track and a match-start sting, and has the shared tracks", () => {
    for (const station of MUSIC_STATIONS) {
      expect(stationTracks(station).length, station).toBeGreaterThanOrEqual(2);
      expect(MUSIC_MANIFEST.files[dangerTrack(station)]?.loop, `${station} danger`).toBe(true);
      expect(MUSIC_MANIFEST.files[startTrack(station)]?.loop, `${station} start`).toBe(false);
    }
    for (const id of [MENU_TRACK, ...Object.values(RESULT_TRACKS)]) expect(MUSIC_MANIFEST.files[id]?.loop, id).toBe(true);
  });

  it("R631 loops a whole number of bars, and hands a sting off on a bar line", () => {
    for (const [id, t] of entries) {
      const bar = (60 / t.bpm) * t.beatsPerBar;
      const bars = t.loop ? ((t.loopEnd ?? 0) - (t.loopStart ?? 0)) / bar : (t.handoff ?? 0) / bar;
      expect(Math.abs(bars - Math.round(bars)), `${id}: ${bars} bars`).toBeLessThan(1e-3);
      expect(Math.round(bars), id).toBeGreaterThan(0);
      if (t.loop) {
        // The tail past the loop's end is what lets a decoder's priming offset loop cleanly.
        expect(t.duration - (t.loopEnd ?? 0), id).toBeGreaterThan(0.25);
        // #51: two to four minutes for whatever can play for a long stretch, the menu theme and each
        // station's in-game and low-health tracks. A result's loop and a Mythic's theme play for a
        // moment of the match and are shorter, though still longer than a few bars.
        const minimum = /^(tavern|edm|lofi|epic)-(\d+|danger)$/.test(id) || id === MENU_TRACK ? 120 : 15;
        expect((t.loopEnd ?? 0) - (t.loopStart ?? 0), id).toBeGreaterThanOrEqual(minimum - 1e-6);
      }
    }
  });

  it("R631 names its source and licence for every track", () => {
    const text = readFileSync(LICENSES, "utf8");
    for (const [id] of entries) expect(text, id).toContain(`\`${id}\``);
    expect(text).toContain("FluidR3");
    expect(text).toContain("MIT");
  });
});

describe("R631 the cards that play music", () => {
  it("R631 gives every Mythic a unique theme, every Legendary a shared entrance theme, and names only real cards", () => {
    const mythics = Object.entries(CATALOG).filter(([, c]) => c.rarity === "Mythic" && c.token !== true).map(([id]) => id);
    expect(mythics.length).toBeGreaterThan(0);
    for (const id of mythics) expect(MUSIC_CARDS[id]?.theme, id).toBeDefined();
    const mythicTracks = mythics.map((id) => MUSIC_CARDS[id]?.theme);
    expect(new Set(mythicTracks).size, "no two Mythics share a track").toBe(mythics.length);
    const legendaries = Object.entries(CATALOG)
      .filter(([, c]) => c.rarity === "Legendary" && c.token !== true)
      .map(([id]) => id);
    expect(legendaries.length).toBeGreaterThan(0);
    for (const id of legendaries) expect(MUSIC_CARDS[id]?.theme, id).toMatch(/^legendary-[12]$/);
    for (const id of ["legendary-1", "legendary-2"]) expect(mythicTracks, id).not.toContain(id);
    for (const id of Object.keys(MUSIC_CARDS)) expect(CATALOG[id], id).toBeDefined();
  });

  it("R631 has at least one card that switches its caster's station, and one for every station", () => {
    const switched = new Set(Object.values(MUSIC_CARDS).map((e) => e.station).filter((s) => s !== undefined));
    expect([...switched].sort()).toEqual([...MUSIC_STATIONS].sort());
  });

  it("R1352 gives every Legendary and Mythic of the shipped sets, tokens printed so included, an intro of its own", () => {
    const cards = introCards(SHIPPED_SETS);
    // Cards and tokens both: a token printed Legendary or Mythic opens its play like any card.
    expect(cards.filter((id) => CATALOG[id]?.token !== true).length).toBeGreaterThan(0);
    expect(cards.filter((id) => CATALOG[id]?.token === true).length).toBeGreaterThan(0);
    for (const id of cards) expect(MUSIC_CARDS[id]?.intro, id).toBeDefined();
    const intros = cards.map((id) => MUSIC_CARDS[id]?.intro);
    expect(new Set(intros).size, "no two cards share an intro").toBe(cards.length);
    const themes = new Set(Object.values(MUSIC_CARDS).map((e) => e.theme));
    for (const intro of intros) expect(themes.has(intro), `${String(intro)} is no theme`).toBe(false);
  });

  it("R1352 gives an intro to no card that is not printed Legendary or Mythic", () => {
    const allowed = new Set(introCards(CATALOG_SETS));
    for (const [id, entry] of Object.entries(MUSIC_CARDS)) if (entry.intro !== undefined) expect(allowed.has(id), id).toBe(true);
  });

  it("R1352 renders each intro as a sting whose music runs MUSIC_INTRO_MIN_S at least, and whose file runs MUSIC_INTRO_MAX_S at most", () => {
    const intros = Object.values(MUSIC_CARDS).flatMap((e) => (e.intro === undefined ? [] : [e.intro]));
    expect(intros.length).toBeGreaterThan(0);
    for (const id of intros) {
      const track = MUSIC_MANIFEST.files[id];
      expect(track?.loop, id).toBe(false);
      expect(track?.handoff, id).toBe(track?.intro);
      expect(track?.handoff ?? 0, id).toBeGreaterThanOrEqual(MUSIC_INTRO_MIN_S - 1e-6);
      expect(track?.duration ?? Infinity, id).toBeLessThanOrEqual(MUSIC_INTRO_MAX_S + 1e-6);
    }
  });

  it("R1352 refuses an intro that names a looping track or none", () => {
    expect(() => parseMusicCards({ version: 1, cards: { "core-097": { intro: "menu" } } }, MUSIC_MANIFEST)).toThrow(/intro/);
    expect(() => parseMusicCards({ version: 1, cards: { "core-097": { intro: "nope" } } }, MUSIC_MANIFEST)).toThrow(/intro/);
    expect(parseMusicCards({ version: 1, cards: { "core-097": { intro: "intro-core-097" } } }, MUSIC_MANIFEST)).toEqual({ "core-097": { intro: "intro-core-097" } });
  });

  it("R631 refuses a table that names a track the manifest lacks, or a station that does not exist", () => {
    expect(() => parseMusicCards({ version: 1, cards: { "core-097": { theme: "nope" } } }, MUSIC_MANIFEST)).toThrow(/theme/);
    expect(() => parseMusicCards({ version: 1, cards: { "core-004": { station: "polka" } } }, MUSIC_MANIFEST)).toThrow(/station/);
    expect(() => parseMusicCards({ version: 1, cards: { "core-004": {} } }, MUSIC_MANIFEST)).toThrow(/a theme, a station or an intro/);
  });

  it("R631 refuses a manifest whose loop does not sit inside its file", () => {
    const loop = { hash: "x", bytes: 1, bpm: 120, beatsPerBar: 4, loop: true, intro: 0, duration: 10, loopStart: 2, loopEnd: 12, handoff: null };
    expect(() => parseMusicManifest({ version: 1, files: { bad: loop } })).toThrow(/loopStart < loopEnd/);
    expect(() => parseMusicManifest({ version: 1, files: { bad: { ...loop, loop: false, handoff: null } } })).toThrow(/handoff/);
  });
});

describe("R631 gen-music.mjs --check", () => {
  it(
    "R631 exits 0 on the committed tree",
    () => {
      const { status, out } = check();
      expect(status, out).toBe(0);
      expect(out).toMatch(/^gen-music: ok, \d+ files, \d+ bytes/m);
    },
    CHECK_TIMEOUT_MS,
  );

  it(
    "R631 exits 1 naming the track whose score changed since its render",
    () => {
      const root = mkdtempSync(join(tmpdir(), "jackioh-gen-music-check-"));
      scratch.push(root);
      mkdirSync(join(root, "src/audio"), { recursive: true });
      mkdirSync(join(root, "public/audio/music"), { recursive: true });
      const manifest = JSON.parse(readFileSync(join(here, "music-manifest.json"), "utf8")) as { files: Record<string, { hash: string; bytes: number }> };
      // Files of the right size stand in for the real ones: `--check` reads sizes, not audio.
      for (const [id, entry] of Object.entries(manifest.files)) {
        const fd = openSync(join(root, "public/audio/music", `${id}.m4a`), "w");
        ftruncateSync(fd, entry.bytes);
        closeSync(fd);
      }
      const victory = manifest.files.victory;
      if (victory === undefined) throw new Error("no victory track");
      victory.hash = "0000000000000000";
      writeFileSync(join(root, "src/audio/music-manifest.json"), JSON.stringify(manifest));
      copyFileSync(join(here, "music-cards.json"), join(root, "src/audio/music-cards.json"));
      const { status, out } = check(root);
      expect(status, out).toBe(1);
      expect(out.trim().split("\n")).toEqual(["victory: score changed since its render"]);
    },
    CHECK_TIMEOUT_MS,
  );
});
