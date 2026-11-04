// #2681, R3012001: the artwork of each release (releaseArt.ts): the definitions a patch's record names
// and no others, a sample of a large patch and a major's headline cards, card backs for a patch that
// changed no card, the SVG's bytes (nothing in them moves), the paths it is served at, which versions
// have art and which have none, and the real history, every patch of it.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { GLITCH_DEF_ID } from "@jackioh/engine/config";
import type { CardDef } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import {
  RELEASE_ART_CARD_LIMIT,
  RELEASE_ART_HEIGHT,
  RELEASE_ART_LEGACY_MICRO_LAST,
  RELEASE_ART_MAJOR_CARD_LIMIT,
  RELEASE_ART_WIDTH,
} from "./constants.ts";
import { FIXTURE_PATCHES, FIXTURE_SNAPSHOTS, M1, M2, TIERED_PATCHES, TIERED_SNAPSHOTS, V1, V2, V3, fixtureDef } from "./fixtures.ts";
import {
  releaseArtAsset,
  releaseArtAssets,
  releaseArtCards,
  releaseArtHeadlines,
  releaseArtPath,
  releaseArtSvg,
  type ReleaseArtInput,
} from "./releaseArt.ts";
import { RELEASES, tiers, type Release } from "./releases.ts";
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
  return { patch, release: LINE, kind: "patch", before: null, after: snapshotOf(patch.version), ...over };
}

/** Whether an SVG holds anything that moves, or a style sheet that could. */
function moves(svg: string): boolean {
  return /<animate|<style|@keyframes|animation|<set\b/u.test(svg);
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
      const svg = releaseArtSvg(input(patch, { after: {} }));
      expect(count(svg, '<use href="#back"/>'), patch.changes.length === 0 ? "none" : "glitch").toBe(5);
      expect(svg).not.toContain("<image");
      expect(svg).toContain("This patch changed no card.");
    }
    const drawn = releaseArtSvg(input(fixturePatch(V3), { before: snapshotOf(V2) }));
    expect(drawn).not.toContain('<use href="#back"/>');
    // Each card's art once in the fan, and the first card's once more as the backdrop.
    expect(count(drawn, "<image")).toBe(5 + 1);
  });

  it("#2681 emits byte-identical escaped full-screen SVG, a major's poster and a patch's art, and nothing in it moves", () => {
    const base = fixtureDef(V1, "core-001");
    const odd: CardDef = { ...base, id: "core-101", name: `Knight & <Co> "Sons" 'n'` };
    const record: Patch = {
      ...fixturePatch(V1),
      player: { title: `A <b>&"bold"</b> 'title'`, new: [], balance: [], changes: [], text: [], fixes: [] },
      changes: [{ id: odd.id, name: odd.name, kind: "added" }],
    };
    const line: Release = { ...LINE, name: `Fixture & <Line>` };
    const major = input(record, { release: line, kind: "major", after: { [odd.id]: odd } });
    const patch = { ...major, kind: "patch" as const };

    const poster = releaseArtSvg(major);
    expect(releaseArtSvg(major)).toBe(poster);
    expect(parses(poster)).toBe(true);
    expect(poster).toContain(`viewBox="0 0 ${String(RELEASE_ART_WIDTH)} ${String(RELEASE_ART_HEIGHT)}"`);
    expect(poster).toContain(`width="${String(RELEASE_ART_WIDTH)}" height="${String(RELEASE_ART_HEIGHT)}"`);

    // What a person wrote reaches the file escaped, in its text and its title, never as markup.
    for (const svg of [poster, releaseArtSvg(patch)]) {
      expect(svg).toContain("FIXTURE &#38; &#60;LINE&#62;");
      expect(svg).toContain("Knight &#38; &#60;Co&#62; &#34;Sons&#34; &#39;n&#39;");
      expect(svg).toContain("A &#60;b&#62;&#38;&#34;bold&#34;&#60;/b&#62; &#39;title&#39;");
      expect(svg).toContain(`>${V1}</text>`);
      expect(svg).not.toContain("<b>");
      expect(svg).not.toContain("<Co>");
      expect(svg).not.toContain("<script");
      expect(svg.replace("http://www.w3.org/2000/svg", "")).not.toMatch(/https?:/u);
      expect(svg).not.toMatch(/\bhref="(?!data:image\/svg\+xml,|#)/u);
      expect(moves(svg)).toBe(false);
    }
    // A poster is richer than a patch's art: a frame and the count of the cards it names.
    expect(poster).toContain("1 card</text>");
    expect(releaseArtSvg(patch)).not.toContain("1 card</text>");
    expect(poster).not.toBe(releaseArtSvg(patch));
  });

  it("R3012001 a major draws a headline poster with more cards than a patch, the rarest in the middle", () => {
    const base = fixtureDef(V1, "core-001");
    const rarities = ["Common", "Rare", "Epic", "Legendary", "Mythic", "Common"] as const;
    const defs: CardDef[] = Array.from({ length: 20 }, (_none, at) => ({
      ...base,
      id: `core-${String(100 + at)}`,
      name: `Card ${String(at)}`,
      rarity: rarities[at % rarities.length] ?? "Common",
    }));
    // One Mythic is the rarest of them, the 5th card (index 4), and the first Mythic of the record.
    const patch: Patch = { ...fixturePatch(V1), tier: "major", changes: defs.map((def) => ({ id: def.id, name: def.name, kind: "added" as const })) };
    const after: Snapshot = Object.fromEntries(defs.map((def) => [def.id, def]));

    const headlines = releaseArtHeadlines(patch, after, null);
    expect(headlines).toHaveLength(RELEASE_ART_MAJOR_CARD_LIMIT);
    expect(RELEASE_ART_MAJOR_CARD_LIMIT).toBeGreaterThan(RELEASE_ART_CARD_LIMIT);
    // The fan's middle card (the one drawn on top) is the first Mythic of the record; Common is picked last.
    expect(headlines[Math.floor(headlines.length / 2)]?.name).toBe("Card 4");
    expect(headlines.filter((def) => def.rarity === "Mythic")).toHaveLength(3);
    expect(headlines.filter((def) => def.rarity === "Common")).toHaveLength(0);
    // The same picture each time, and the cards are the record's.
    expect(releaseArtHeadlines(patch, after, null).map((def) => def.id)).toEqual(headlines.map((def) => def.id));
    for (const def of headlines) expect(patch.changes.map((change) => change.id)).toContain(def.id);

    const rendered = releaseArtSvg({ patch, release: LINE, kind: "major", before: null, after });
    expect(count(rendered, "<image")).toBe(RELEASE_ART_MAJOR_CARD_LIMIT + 1);
    expect(rendered).toContain("20 cards</text>");
    const art = releaseArtSvg({ patch, release: LINE, kind: "patch", before: null, after });
    expect(count(art, "<image")).toBe(RELEASE_ART_CARD_LIMIT + 1);

    // A record of fewer cards than the limit shows each of them, a token last of a tie.
    const few = releaseArtHeadlines(fixturePatch(V3), snapshotOf(V3), snapshotOf(V2));
    expect(few.map((def) => def.id).sort()).toEqual(["classic-001", "classicplus-012", "classicplus-012-1", "core-002", "core-004"]);
    expect(releaseArtHeadlines({ ...fixturePatch(V1), changes: [] }, {}, null)).toEqual([]);
  });

  it("#2681 resolves only canonical normal and major-still asset paths", () => {
    expect(releaseArtPath("v0.4.0")).toBe("/release-art/v0.4.0.svg");
    expect(releaseArtPath("v0.4.0", "main")).toBe("/release-art/v0.4.0.svg");
    expect(releaseArtPath("v0.4.0", "still")).toBe("/release-art/v0.4.0-still.svg");
    expect(releaseArtPath("v0.2.10b")).toBe("/release-art/v0.2.10b.svg");
    expect(releaseArtPath("a b/c", "still")).toBe("/release-art/a%20b%2Fc-still.svg");

    // V1 is a major here, V2 and V3 are patches: three versions, each with art, and the major's companion.
    const patches = FIXTURE_PATCHES.map((patch, at) => ({ ...patch, tier: at === 0 ? ("major" as const) : ("patch" as const) }));
    const reads: string[] = [];
    const snapshot = (version: string): Snapshot | null => {
      reads.push(version);
      return FIXTURE_SNAPSHOTS[version] ?? null;
    };
    const assets = releaseArtAssets(patches, snapshot, [LINE]);
    expect(assets.map((asset) => asset.path)).toEqual([releaseArtPath(V1), releaseArtPath(V1, "still"), releaseArtPath(V2), releaseArtPath(V3)]);
    // Each snapshot is read once, in patches.json's order, and the removed relic is drawn from the one before.
    expect(reads).toEqual([V1, V2, V3]);
    expect(assets[2]?.svg).toContain("Old Relic");
    // R3012001: a major's still companion is its poster, byte for byte, and nothing in any file moves.
    expect(assets[0]?.svg).toBe(assets[1]?.svg);
    expect(assets[1]?.svg).toContain("FIXTURE LINE");
    for (const asset of assets) expect(moves(asset.svg), asset.path).toBe(false);

    // A path is served only as it is named; one asset is drawn from its patch and the one before it.
    const lookups: string[] = [];
    const lookup = (version: string): Snapshot | null => {
      lookups.push(version);
      return FIXTURE_SNAPSHOTS[version] ?? null;
    };
    for (const asset of assets) expect(releaseArtAsset(asset.path, patches, snapshot, [LINE])).toEqual(asset);
    expect(releaseArtAsset(releaseArtPath(V3), patches, lookup, [LINE])?.svg).toBe(assets[3]?.svg);
    expect(lookups).toEqual([V3, V2]);
    for (const path of [
      releaseArtPath(V2, "still"),
      releaseArtPath(V3, "still"),
      `${releaseArtPath(V1)}.bak`,
      releaseArtPath(V1).toUpperCase(),
      releaseArtPath(V1).replace("/release-art/", "/release-art//"),
      releaseArtPath(V1).replace("/release-art/", "/art/"),
      "/release-art/",
      "/release-art/nobody.svg",
      "/patch-notes",
      "",
    ]) {
      expect(releaseArtAsset(path, patches, snapshot, [LINE]), path).toBeNull();
    }
  });

  it("R3012001 every patch has art and every new micro has none, the micros before the cut keeping the file they had", () => {
    // Major V1 → patch V2 → micro M1 → micro M2 → patch V3, the cut at M1: M1 keeps its old file, M2 has none.
    const reads: string[] = [];
    const snapshot = (version: string): Snapshot | null => {
      reads.push(version);
      return TIERED_SNAPSHOTS[version] ?? null;
    };
    const assets = releaseArtAssets(TIERED_PATCHES, snapshot, [LINE], M1);
    expect(assets.map((asset) => asset.path)).toEqual([releaseArtPath(V1), releaseArtPath(V1, "still"), releaseArtPath(V2), releaseArtPath(M1), releaseArtPath(V3)]);
    // Every snapshot is read once, in order, micros' included, so the next patch's removed cards are drawn from the one before.
    expect(reads).toEqual([V1, V2, M1, M2, V3]);
    expect(assets.map((asset) => asset.path)).not.toContain(releaseArtPath(M2));
    for (const asset of assets) expect(moves(asset.svg), asset.path).toBe(false);

    // The dev server answers for exactly those paths: no `-still` of a patch or a micro, and nothing for M2.
    for (const asset of assets) expect(releaseArtAsset(asset.path, TIERED_PATCHES, snapshot, [LINE], M1)?.svg, asset.path).toBe(asset.svg);
    for (const path of [releaseArtPath(M2), releaseArtPath(M2, "still"), releaseArtPath(M1, "still"), releaseArtPath(V2, "still"), releaseArtPath(V3, "still")]) {
      expect(releaseArtAsset(path, TIERED_PATCHES, snapshot, [LINE], M1), path).toBeNull();
    }

    // With the cut at V2 (a version before both micros) neither micro has a file; a cut the history does not hold gives none either.
    for (const cut of [V2, "not-a-version"]) {
      const paths = releaseArtAssets(TIERED_PATCHES, snapshot, [LINE], cut).map((asset) => asset.path);
      expect(paths, cut).toEqual([releaseArtPath(V1), releaseArtPath(V1, "still"), releaseArtPath(V2), releaseArtPath(V3)]);
    }
    // And with the cut at the last version, both keep theirs.
    expect(releaseArtAssets(TIERED_PATCHES, snapshot, [LINE], V3).map((asset) => asset.path)).toContain(releaseArtPath(M2));
    expect(tiers(TIERED_PATCHES)).toEqual(["major", "patch", "micro", "micro", "patch"]);
    expect([M1, M2]).toEqual(TIERED_PATCHES.filter((patch) => patch.tier === "micro").map((patch) => patch.version));
  });
});

describe("#2681 the real history's artwork", () => {
  const dir = join(dirname(fileURLToPath(import.meta.url)), "../../../../crates/cards/patches");
  const read = <T>(file: string): T => JSON.parse(readFileSync(join(dir, file), "utf8")) as T;
  const patches = read<Patch[]>("patches.json");

  it("R3012001 gives every real major and patch artwork, a major its companion, and no micro after the cut any", () => {
    const assets = releaseArtAssets(patches, (version) => read<Snapshot>(`${version}.json`));
    const tierList = tiers(patches);
    const cut = patches.findIndex((patch) => patch.version === RELEASE_ART_LEGACY_MICRO_LAST);
    // The cut is a version of the history, and every micro up to it keeps the file its published links name.
    expect(cut, RELEASE_ART_LEGACY_MICRO_LAST).toBeGreaterThanOrEqual(0);
    const expected = patches.flatMap((patch, at) => {
      if (tierList[at] === "major") return [releaseArtPath(patch.version), releaseArtPath(patch.version, "still")];
      return tierList[at] === "patch" || at <= cut ? [releaseArtPath(patch.version)] : [];
    });
    expect(assets.map((asset) => asset.path)).toEqual(expected);
    expect(new Set(expected).size).toBe(expected.length);
    // Every recorded tier says major for the same patches the history's lines open at (R1369000).
    expect(patches.filter((_patch, at) => tierList[at] === "major").map((patch) => patch.version)).toEqual(
      patches.filter((patch) => patch.tier === "major").map((patch) => patch.version),
    );
    expect(tierList.filter((tier) => tier === "major").length).toBeGreaterThanOrEqual(RELEASES.length);
    for (const [at, patch] of patches.entries()) {
      const path = releaseArtPath(patch.version);
      if (patch.tier === "patch" || patch.tier === "major") expect(expected, patch.version).toContain(path);
      if (patch.tier === "micro" && at > cut) expect(expected, patch.version).not.toContain(path);
      if (patch.tier === "micro" && at <= cut) expect(expected, patch.version).toContain(path);
    }
    for (const version of ["v0.3.0", "v0.4.0"]) expect(expected).toContain(releaseArtPath(version, "still"));

    // Every major's poster: no animation at all, and its `-still` file the same bytes.
    for (const [at, patch] of patches.entries()) {
      if (tierList[at] !== "major") continue;
      const main = assets.find((asset) => asset.path === releaseArtPath(patch.version));
      const still = assets.find((asset) => asset.path === releaseArtPath(patch.version, "still"));
      expect(main, patch.version).toBeDefined();
      expect(still?.svg, patch.version).toBe(main?.svg);
    }
    for (const asset of assets) {
      expect(moves(asset.svg), asset.path).toBe(false);
      expect(asset.svg, asset.path).toContain(`viewBox="0 0 ${String(RELEASE_ART_WIDTH)} ${String(RELEASE_ART_HEIGHT)}"`);
      expect(parses(asset.svg), asset.path).toBe(true);
    }

    // Each patch shows cards its own record names, at most the limit of them (a major's headlines, the major limit), and cards when it names any.
    let before: Snapshot | null = null;
    for (const [at, patch] of patches.entries()) {
      const after = read<Snapshot>(`${patch.version}.json`);
      const major = tierList[at] === "major";
      const shown = (major ? releaseArtHeadlines(patch, after, before) : releaseArtCards(patch, after, before)).map((def) => def.id);
      const named = patch.changes.map((change) => change.id).filter((id) => id !== GLITCH_DEF_ID);
      expect(named, patch.version).toEqual(expect.arrayContaining(shown));
      expect(shown.length, patch.version).toBe(Math.min(new Set(named).size, major ? RELEASE_ART_MAJOR_CARD_LIMIT : RELEASE_ART_CARD_LIMIT));
      before = after;
    }
    // The asset of the first patch, and every other, is drawn the same each time.
    const again = releaseArtAssets(patches, (version) => read<Snapshot>(`${version}.json`));
    expect(again).toEqual(assets);
  }, 60_000);
});
