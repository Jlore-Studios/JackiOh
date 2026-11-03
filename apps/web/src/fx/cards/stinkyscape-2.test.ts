// R2784000 #2798: Stinkyscape's pass, part two, over its animation table (stinkyscape-2.json5, `stinkyscape-091`
// to the end, the set's tokens included): no line pending, at least FX_SET_PASS_FLOOR of its non-token cards
// wear an entry other than `none`, every Spell, Field Spell, Trap and Field Trap has a `cast` (or a recipe or
// set piece of its own) but the Starship's own cards, which the Starship part of #2782 draws, every Field
// Spell an aura, every hook a drawn motif, and every play's motif draws on its gallery fixture as that card's
// play, a hidden play keying none (R202). stinkyscape-2.md, the pass's per-card table, holds one row per
// card of the file that matches the table.

import type { GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { fullBoardView } from "../../test/fixtures.ts";
import { CATALOG } from "../../wire/cards.ts";
import { planCardFx, planCardMotif } from "../cardFx.ts";
import { FX_INTENSITY_SCALE, FX_SET_PASS_FLOOR } from "../constants.ts";
import { createFxMemory } from "../memory.ts";
import { motifScale } from "../motifs/build.ts";
import { galleryEntry, motifFixture, RECIPE_FIXTURES } from "../motifs/gallery.ts";
import { FX_CARD_HOOKS, isDrawnMotif } from "../motifs/names.ts";
import type { FxPlanEnv } from "../types.ts";
import { isCardRecipeKey } from "./bespoke.ts";
import rawTable from "./stinkyscape-2.md?raw";
import { FX_CARD_FILES, parseFxCardTable, type FxCardEntry } from "./tables.ts";

const FILE = FX_CARD_FILES.find((candidate) => candidate.file === "stinkyscape-2.json5");
if (FILE === undefined) throw new Error("no stinkyscape-2.json5");
const TABLE = parseFxCardTable(FILE.file, FILE.source);
const RANGE = [...TABLE.keys()];
const CARDS = RANGE.filter((id) => CATALOG[id]?.token !== true);

/** The Starship's own cards (#2782's Starship part): its fuse and launch, Launch Sequencer, the Retrofitting Program and the Launchers. */
const STARSHIP_PARTS = [
  "stinkyscape-106",
  "stinkyscape-130",
  "stinkyscape-133",
  "stinkyscape-136",
  "stinkyscape-137",
  "stinkyscape-140",
  "stinkyscape-141",
  "stinkyscape-142",
  "stinkyscape-143",
  "stinkyscape-176",
  "stinkyscape-t-dormant-starship",
  "stinkyscape-t-ai-05",
];
const STARSHIP_PART = "Starship part";
/** R2801000: the Starship parts the Starship's moment gives an entry: the Retrofitting Program's send, the Launchers' conjure and Launch Sequencer's tithe. */
const STARSHIP_ENTRIES = ["stinkyscape-130", "stinkyscape-140", "stinkyscape-141", "stinkyscape-142", "stinkyscape-143", "stinkyscape-t-ai-05"];

function entryOf(id: string): FxCardEntry {
  const entry = TABLE.get(id);
  if (entry === undefined) throw new Error(`${id} has no entry`);
  return entry;
}

/** An entry as stinkyscape-2.md's Entry column writes it. */
function describeEntry(entry: FxCardEntry): string {
  switch (entry.kind) {
    case "none":
      return `none: ${entry.reason}`;
    case "bespoke":
      return `bespoke ${entry.key}`;
    case "setPiece":
      return `setPiece ${entry.key}`;
    case "hooks": {
      const parts: string[] = [];
      for (const hook of FX_CARD_HOOKS) {
        const spec = entry.hooks[hook];
        if (spec === undefined) continue;
        parts.push([hook, spec.motif, spec.palette, spec.row === "keep" ? "keep" : undefined].filter((part) => part !== undefined).join(" "));
      }
      if (entry.aura !== undefined) parts.push(["aura", entry.aura.look, entry.aura.scope, entry.aura.palette].filter((part) => part !== undefined).join(" "));
      return parts.join("; ");
    }
  }
}

function envOf(): FxPlanEnv {
  return { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
}

const played = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 1 });

/** The play's motif on its gallery fixture, as the board plans it once the play's `cardPlayed` is remembered. */
function playMotif(defId: string, instanceId: string, motif: Parameters<typeof motifFixture>[0]) {
  const view = fullBoardView();
  const env = envOf();
  env.memory.remember([played("p1", instanceId, defId)]);
  const entry = { ...galleryEntry(motifFixture(motif, view, instanceId, "p1"), view), durationMs: 400 };
  return planCardMotif(entry, view, env);
}

/** Each card's play motif: its `cry` (a Unit) or `cast`, when a drawn motif. */
function playMotifs(): { id: string; motif: Parameters<typeof motifFixture>[0] }[] {
  return RANGE.flatMap((id) => {
    const entry = entryOf(id);
    if (entry.kind !== "hooks") return [];
    const hook = entry.hooks[CATALOG[id]?.type === "Unit" ? "cry" : "cast"];
    return hook !== undefined && isDrawnMotif(hook.motif) ? [{ id, motif: hook.motif }] : [];
  });
}

/** Each recipe entry's cues over its gallery fixture, the play `defId` resolving, each event its own entry. */
function recipeCues(defId: string, key: string, instanceId: string): number {
  if (!isCardRecipeKey(key)) throw new Error(`${key} is no card recipe`);
  const view = fullBoardView();
  const env = envOf();
  env.memory.remember([played("p1", instanceId, defId)]);
  return RECIPE_FIXTURES[key](view, instanceId, "p1").reduce((n, event) => {
    env.memory.remember([event]);
    const entry = { ...galleryEntry([event], view), durationMs: 400 };
    return n + (planCardFx(event, { entry, view, env, D: 400 })?.cues.length ?? 0);
  }, 0);
}

describe("R2784000 #2798 Stinkyscape's card animation pass, part two", () => {
  it("R2784000 #2798 the file runs from stinkyscape-091 to the end, tokens included, and no line is pending", () => {
    expect(RANGE[0]).toBe("stinkyscape-091");
    expect(RANGE).toHaveLength(120);
    expect(CARDS).toHaveLength(88);
    for (const id of RANGE) expect(describeEntry(entryOf(id)), id).not.toBe("none: pending");
  });

  it("R2784000 #2798 at least FX_SET_PASS_FLOOR of the range's non-token cards have an entry other than none", () => {
    const drawn = CARDS.filter((id) => entryOf(id).kind !== "none");
    expect(drawn.length).toBeGreaterThanOrEqual(Math.ceil(FX_SET_PASS_FLOOR * CARDS.length));
  });

  it("R2784000 #2798 every non-token non-Unit has a cast, bespoke or set piece but the Starship parts, and every Field Spell, tokens included, an aura", () => {
    // 11 of the set's 17 Field Spells, two of them tokens (Wardrum Blob, Drone Uplink).
    const fields = RANGE.filter((id) => CATALOG[id]?.type === "Field Spell");
    expect(fields).toHaveLength(11);
    for (const id of RANGE) {
      const type = CATALOG[id]?.type;
      const entry = entryOf(id);
      if (type === "Field Spell") expect(entry.kind === "hooks" ? entry.aura : undefined, id).toBeDefined();
      if (type === "Unit" || CATALOG[id]?.token === true || STARSHIP_PARTS.includes(id)) continue;
      const plays = entry.kind === "bespoke" || entry.kind === "setPiece" || (entry.kind === "hooks" && entry.hooks.cast !== undefined);
      expect(plays, `${id} (${String(type)})`).toBe(true);
    }
  });

  it("R2784000 #2798 exactly the Starship parts without an entry of their own are none: Starship part", () => {
    const held = RANGE.filter((id) => {
      const entry = entryOf(id);
      return entry.kind === "none" && entry.reason === STARSHIP_PART;
    });
    expect(held).toEqual(STARSHIP_PARTS.filter((id) => !STARSHIP_ENTRIES.includes(id)));
    for (const id of STARSHIP_ENTRIES) expect(entryOf(id).kind, id).toBe("hooks");
  });

  it("R2784000 #2798 every hook names a drawn motif, and every recipe is a card recipe", () => {
    for (const id of RANGE) {
      const entry = entryOf(id);
      if (entry.kind === "bespoke") expect(isCardRecipeKey(entry.key), id).toBe(true);
      if (entry.kind !== "hooks") continue;
      for (const [hook, spec] of Object.entries(entry.hooks)) expect(isDrawnMotif(spec.motif), `${id} ${hook} ${spec.motif}`).toBe(true);
    }
  });

  it("R2784000 #2798 every cast and cry motif draws on its gallery fixture as its card's play, and every recipe on its own", () => {
    const motifs = playMotifs();
    expect(motifs.length).toBeGreaterThan(0);
    for (const { id, motif } of motifs) {
      const result = playMotif(id, "ex", motif);
      expect(result, `${id} ${motif}`).not.toBeNull();
      expect(result?.cues.length ?? 0, `${id} ${motif}`).toBeGreaterThan(0);
      // R202: no cue names a definition.
      expect(JSON.stringify(result?.cues), id).not.toMatch(/(core|stinkyscape)-\d{3}/);
    }
    for (const id of RANGE) {
      const entry = entryOf(id);
      if (entry.kind === "bespoke") expect(recipeCues(id, entry.key, "ex"), id).toBeGreaterThan(0);
    }
  });

  it("R202 #2798 a hidden play keys none of the file's entries", () => {
    for (const { id, motif } of playMotifs()) expect(playMotif("hidden", "hidden", motif), `${id} ${motif}`).toBeNull();
    for (const id of RANGE) {
      const entry = entryOf(id);
      if (entry.kind === "bespoke") expect(recipeCues("hidden", entry.key, "hidden"), id).toBe(0);
    }
  });

  it("R2784000 #2798 stinkyscape-2.md has one row per card of the file, in order, with its name, type, entry, scale, census and why", () => {
    const rows = rawTable
      .split("\n")
      .filter((line) => line.startsWith("| `stinkyscape-"))
      .map((line) => line.slice(1, -1).split(" | ").map((cell) => cell.trim()));
    expect(rows.map((row) => row[0])).toEqual(RANGE.map((id) => `\`${id}\``));
    const view = fullBoardView();
    for (const [cell, name, type, entry, scale, census, why] of rows) {
      const id = (cell ?? "").replace(/`/g, "");
      const def = CATALOG[id];
      const own = entryOf(id);
      expect(name, id).toBe(def?.name.replace(/\s+/g, " ").trim());
      expect(type, id).toBe(def?.type);
      expect(entry, id).toBe(describeEntry(own));
      expect(scale, id).toBe(own.kind === "none" ? "—" : motifScale(id, view, 1).toFixed(2));
      expect(census, id).toMatch(/\S/);
      expect(why, id).toMatch(/\S/);
    }
  });
});
