// #2681: the artwork of each release (releaseArt.ts): the definitions a patch's record names and no
// others, a sample of a large patch, card backs for a patch that changed no card, the SVG's bytes and
// the one form that moves, the paths it is served at, and the real history, every patch of it.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { GLITCH_DEF_ID } from "@jackioh/engine/config";
import type { CardDef } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { RELEASE_ART_CARD_LIMIT, RELEASE_ART_HEIGHT, RELEASE_ART_MAJOR_CYCLE_SECONDS, RELEASE_ART_WIDTH } from "./constants.ts";
import { FIXTURE_PATCHES, FIXTURE_SNAPSHOTS, V1, V2, V3, fixtureDef } from "./fixtures.ts";
import { releaseArtAsset, releaseArtAssets, releaseArtCards, releaseArtPath, releaseArtSvg, type ReleaseArtInput } from "./releaseArt.ts";
import { RELEASES, majors, type Release } from "./releases.ts";
import type { Patch, Snapshot } from "./source.ts";

const LINE: Release = { id: "fixture", name: "Fixture line", summary: "", technical: [] };

function fixturePatch(version: string): Patch {
  const found = FIXTURE_PATCHES.find((patch) => patch.version === version);
  if (found === undefined) throw new Error(`no fixture patch ${version}`);
  return found;
}

function snapshotOf(version: string): Snapshot {
  const found = FIXTURE_SNAPSHOTS[version];
  if (found === undefined) throw new Error(`no fixture snapshot ${version}`);
  return found;
}

/** A patch of the fixtures' V3 with `ids` added, over a snapshot that holds a numbered card for each. */
function numberedPatch(count: number): { patch: Patch; after: Snapshot } {
  const base = fixtureDef(V1, "core-001");
  const defs: CardDef[] = Array.from({ length: count }, (_none, at) => ({ ...base, id: `core-${String(100 + at)}`, name: `Card ${String(at)}` }));
  const patch: Patch = { ...fixturePatch(V1), changes: defs.map((def) => ({ id: def.id, name: def.name, kind: "added" as const })) };
  return { patch, after: Object.fromEntries(defs.map((def) => [def.id, def])) };
}

function input(patch: Patch, over: Partial<ReleaseArtInput> = {}): ReleaseArtInput {
  return { patch, release: LINE, major: false, before: null, after: snapshotOf(patch.version), ...over };
}

const count = (text: string, part: string): number => text.split(part).length - 1;

function parses(svg: string): boolean {
  const document = new DOMParser().parseFromString(svg, "image/svg+xml");
  return document.querySelector("parsererror") === null && document.documentElement.tagName === "svg";
}

describe("#2681 release artwork", () => {
  it("#2681 selects only definitions affected by the patch in recorded order", () => {
    // V2 changes the knight and removes the relic: the knight as V2 holds it, the relic as V1 held it.
    const second = releaseArtCards(fixturePatch(V2), snapshotOf(V2), snapshotOf(V1));
    expect(second.map((def) => def.id)).toEqual(["core-001", "core-003"]);
    expect(second[0]).toBe(fixtureDef(V2, "core-001"));
    expect(second[1]).toBe(fixtureDef(V1, "core-003"));
    // V3 names five cards; the golem it changed is one, the cards it left alone are none.
    const third = releaseArtCards(fixturePatch(V3), snapshotOf(V3), snapshotOf(V2));
    expect(third.map((def) => def.id)).toEqual(["core-002", "core-004", "classic-001", "classicplus-012", "classicplus-012-1"]);
    expect(third.map((def) => def.id)).not.toContain("core-001");
    expect(third[0]).toBe(fixtureDef(V3, "core-002"));

    // A card named twice is drawn once, in the place of its first mention; Glitch is never drawn (R674).
    const record: Patch = {
      ...fixturePatch(V3),
      changes: [
        { id: "core-004", name: "Quiet Golem", kind: "changed" },
        { id: GLITCH_DEF_ID, name: "Glitch", kind: "added" },
        { id: "classic-001", name: "Fresh Face", kind: "added" },
        { id: "core-004", name: "Quiet Golem", kind: "changed" },
      ],
    };
    expect(releaseArtCards(record, snapshotOf(V3), snapshotOf(V2)).map((def) => def.id)).toEqual(["core-004", "classic-001"]);

    // A card the record claims and its snapshot lacks fails with the version and the id, never another card.
    const missing: Patch = { ...fixturePatch(V3), changes: [{ id: "core-999", name: "Nobody", kind: "added" }] };
    expect(() => releaseArtCards(missing, snapshotOf(V3), snapshotOf(V2))).toThrow(`${V3} records core-999 as added`);
    const gone: Patch = { ...fixturePatch(V2), changes: [{ id: "core-003", name: "Old Relic", kind: "removed" }] };
    expect(() => releaseArtCards(gone, snapshotOf(V2), snapshotOf(V2))).toThrow(`${V2} records core-003 as removed`);
    expect(() => releaseArtCards(gone, snapshotOf(V2), null)).toThrow("core-003");
  });

  it("#2681 samples a large patch deterministically and draws backs for a zero-card patch", () => {
    const limit = RELEASE_ART_CARD_LIMIT;
    // Up to the limit every card is drawn; above it the limit's worth, evenly, the first and the last among them.
    const exact = numberedPatch(limit);
    expect(releaseArtCards(exact.patch, exact.after, null)).toHaveLength(limit);
    const large = numberedPatch(20);
    const picked = releaseArtCards(large.patch, large.after, null).map((def) => def.name);
    expect(picked).toEqual(["Card 0", "Card 3", "Card 6", "Card 10", "Card 13", "Card 16", "Card 19"]);
    expect(releaseArtCards(large.patch, large.after, null).map((def) => def.name)).toEqual(picked);
    const next = numberedPatch(limit + 1);
    const nextPicked = releaseArtCards(next.patch, next.after, null);
    expect(nextPicked).toHaveLength(limit);
    expect(new Set(nextPicked.map((def) => def.id)).size).toBe(limit);
    expect(nextPicked[0]?.name).toBe("Card 0");
    expect(nextPicked.at(-1)?.name).toBe(`Card ${String(limit)}`);

    // A patch that changed no card, or only Glitch, draws five card backs and no card's art.
    const quiet: Patch = { ...fixturePatch(V1), changes: [] };
    const glitch: Patch = { ...fixturePatch(V1), changes: [{ id: GLITCH_DEF_ID, name: "Glitch", kind: "added" }] };
    for (const patch of [quiet, glitch]) {
      const svg = releaseArtSvg(input(patch, { after: {} }), "animated");
      expect(count(svg, '<use href="#back"/>'), patch.changes.length === 0 ? "none" : "glitch").toBe(5);
      expect(svg).not.toContain("<image");
      expect(svg).toContain("This patch changed no card.");
    }
    const drawn = releaseArtSvg(input(fixturePatch(V3), { before: snapshotOf(V2) }), "animated");
    expect(drawn).not.toContain('<use href="#back"/>');
    // Each card's art once in the fan, and the first card's once more as the backdrop.
    expect(count(drawn, "<image")).toBe(5 + 1);
  });

  it("#2681 emits byte-identical escaped full-screen SVG and animates only the normal form of a major", () => {
    const base = fixtureDef(V1, "core-001");
    const odd: CardDef = { ...base, id: "core-101", name: `Knight & <Co> "Sons" 'n'` };
    const record: Patch = {
      ...fixturePatch(V1),
      player: { title: `A <b>&"bold"</b> 'title'`, new: [], balance: [], changes: [], text: [], fixes: [] },
      changes: [{ id: odd.id, name: odd.name, kind: "added" }],
    };
    const line: Release = { ...LINE, name: `Fixture & <Line>` };
    const major = input(record, { release: line, major: true, after: { [odd.id]: odd } });

    const animated = releaseArtSvg(major, "animated");
    expect(releaseArtSvg(major, "animated")).toBe(animated);
    expect(releaseArtSvg(major, "still")).toBe(releaseArtSvg(major, "still"));
    expect(parses(animated)).toBe(true);
    expect(animated).toContain(`viewBox="0 0 ${String(RELEASE_ART_WIDTH)} ${String(RELEASE_ART_HEIGHT)}"`);
    expect(animated).toContain(`width="${String(RELEASE_ART_WIDTH)}" height="${String(RELEASE_ART_HEIGHT)}"`);

    // What a person wrote reaches the file escaped, in its text and its title, never as markup.
    for (const svg of [animated, releaseArtSvg(major, "still")]) {
      expect(svg).toContain("FIXTURE &#38; &#60;LINE&#62;");
      expect(svg).toContain("Knight &#38; &#60;Co&#62; &#34;Sons&#34; &#39;n&#39;");
      expect(svg).toContain("A &#60;b&#62;&#38;&#34;bold&#34;&#60;/b&#62; &#39;title&#39;");
      expect(svg).toContain(`>${V1}</text>`);
      expect(svg).not.toContain("<b>");
      expect(svg).not.toContain("<Co>");
      expect(svg).not.toContain("<script");
      expect(svg.replace("http://www.w3.org/2000/svg", "")).not.toMatch(/https?:/u);
      expect(svg).not.toMatch(/\bhref="(?!data:image\/svg\+xml,|#)/u);
    }

    // The animated form of a major moves, one cycle of the constant's seconds, and stops under reduced motion.
    expect(animated).toContain("<style>");
    expect(animated).toContain(`animation:pan ${String(RELEASE_ART_MAJOR_CYCLE_SECONDS)}s`);
    expect(animated).toContain("@keyframes pan");
    expect(animated).toMatch(/@media \(prefers-reduced-motion:reduce\)\{[^}]*animation:none/u);
    // Its still companion, a patch's file and a micro patch's file hold the picture and nothing that moves.
    const still = releaseArtSvg(major, "still");
    const patch = releaseArtSvg({ ...major, major: false }, "animated");
    for (const svg of [still, patch, releaseArtSvg({ ...major, major: false }, "still")]) {
      expect(svg).not.toContain("<style");
      expect(svg).not.toContain("@keyframes");
      expect(svg).not.toContain("animation");
      expect(svg).not.toContain("<animate");
    }
    expect(still).toBe(patch);
    expect(animated.replace(/<style>.*?<\/style>/u, "")).toBe(still);
  });

  it("#2681 resolves only canonical normal and major-still asset paths", () => {
    expect(releaseArtPath("v0.4.0", "animated")).toBe("/release-art/v0.4.0.svg");
    expect(releaseArtPath("v0.4.0", "still")).toBe("/release-art/v0.4.0-still.svg");
    expect(releaseArtPath("v0.2.10b", "animated")).toBe("/release-art/v0.2.10b.svg");
    expect(releaseArtPath("a b/c", "still")).toBe("/release-art/a%20b%2Fc-still.svg");

    // V1 is a major here, V2 and V3 are not.
    const patches = FIXTURE_PATCHES.map((patch) => (patch.version === V1 ? { ...patch, tier: "major" as const } : patch));
    const reads: string[] = [];
    const snapshot = (version: string): Snapshot | null => {
      reads.push(version);
      return FIXTURE_SNAPSHOTS[version] ?? null;
    };
    const assets = releaseArtAssets(patches, snapshot, [LINE]);
    expect(assets.map((asset) => asset.path)).toEqual([
      releaseArtPath(V1, "animated"),
      releaseArtPath(V1, "still"),
      releaseArtPath(V2, "animated"),
      releaseArtPath(V3, "animated"),
    ]);
    // Each snapshot is read once, in patches.json's order, and the removed relic is drawn from the one before.
    expect(reads).toEqual([V1, V2, V3]);
    expect(assets[2]?.svg).toContain("Old Relic");
    expect(assets[0]?.svg).toContain("<style>");
    expect(assets[1]?.svg).not.toContain("<style");
    expect(assets[1]?.svg).toContain("FIXTURE LINE");
    expect(assets[2]?.svg).not.toContain("<style");

    // A path is served only as it is named; one asset is drawn from its patch and the one before it.
    const lookups: string[] = [];
    const lookup = (version: string): Snapshot | null => {
      lookups.push(version);
      return FIXTURE_SNAPSHOTS[version] ?? null;
    };
    for (const asset of assets) expect(releaseArtAsset(asset.path, patches, snapshot, [LINE])).toEqual(asset);
    expect(releaseArtAsset(releaseArtPath(V3, "animated"), patches, lookup, [LINE])?.svg).toBe(assets[3]?.svg);
    expect(lookups).toEqual([V3, V2]);
    for (const path of [
      releaseArtPath(V2, "still"),
      releaseArtPath(V3, "still"),
      `${releaseArtPath(V1, "animated")}.bak`,
      releaseArtPath(V1, "animated").toUpperCase(),
      releaseArtPath(V1, "animated").replace("/release-art/", "/release-art//"),
      releaseArtPath(V1, "animated").replace("/release-art/", "/art/"),
      "/release-art/",
      "/release-art/nobody.svg",
      "/patch-notes",
      "",
    ]) {
      expect(releaseArtAsset(path, patches, snapshot, [LINE]), path).toBeNull();
    }
  });
});

describe("#2681 the real history's artwork", () => {
  const dir = join(dirname(fileURLToPath(import.meta.url)), "../../../../crates/cards/patches");
  const read = <T>(file: string): T => JSON.parse(readFileSync(join(dir, file), "utf8")) as T;
  const patches = read<Patch[]>("patches.json");

  it("#2681 gives every real shipped patch an artwork and every real major a still companion", () => {
    const assets = releaseArtAssets(patches, (version) => read<Snapshot>(`${version}.json`));
    const flags = majors(patches);
    const expected = patches.flatMap((patch, at) => [
      releaseArtPath(patch.version, "animated"),
      ...(flags[at] === true ? [releaseArtPath(patch.version, "still")] : []),
    ]);
    expect(assets.map((asset) => asset.path)).toEqual(expected);
    expect(new Set(expected).size).toBe(expected.length);
    // Every recorded tier says major for the same patches the history's lines open at (R1369000).
    expect(patches.filter((_patch, at) => flags[at] === true).map((patch) => patch.version)).toEqual(patches.filter((patch) => patch.tier === "major").map((patch) => patch.version));
    expect(flags.filter(Boolean).length).toBeGreaterThanOrEqual(RELEASES.length);
    for (const version of ["v0.3.0", "v0.4.0"]) expect(expected).toContain(releaseArtPath(version, "still"));

    const moving = assets.filter((asset) => asset.svg.includes("<style>")).map((asset) => asset.path);
    expect(moving).toEqual(patches.filter((_patch, at) => flags[at] === true).map((patch) => releaseArtPath(patch.version, "animated")));

    for (const asset of assets) {
      expect(asset.svg, asset.path).toContain(`viewBox="0 0 ${String(RELEASE_ART_WIDTH)} ${String(RELEASE_ART_HEIGHT)}"`);
      expect(parses(asset.svg), asset.path).toBe(true);
    }

    // Each patch shows cards its own record names, at most the limit of them, and cards when it names any.
    let before: Snapshot | null = null;
    for (const patch of patches) {
      const after = read<Snapshot>(`${patch.version}.json`);
      const shown = releaseArtCards(patch, after, before).map((def) => def.id);
      const named = patch.changes.map((change) => change.id).filter((id) => id !== GLITCH_DEF_ID);
      expect(named, patch.version).toEqual(expect.arrayContaining(shown));
      expect(shown.length, patch.version).toBe(Math.min(new Set(named).size, RELEASE_ART_CARD_LIMIT));
      before = after;
    }
    // The asset of the first patch, and every other, is drawn the same each time.
    const again = releaseArtAssets(patches, (version) => read<Snapshot>(`${version}.json`));
    expect(again).toEqual(assets);
  }, 60_000);
});
