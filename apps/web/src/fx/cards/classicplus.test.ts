// R2795000: Classic+'s pass over its card animation table (classicplus.json5): the measure (55% of its
// cards with an entry other than `none`, every Spell, Field Spell, Trap and Field Trap naming its cast, a
// set piece or a bespoke recipe, every Field Spell given an entry wearing an aura), its 3 to 6 Flourishes,
// the tokens it gave entries, and R202: a hidden play keys none of it, as every readable play keys its own.
// classicplus.md, the pass's per-card table, holds one row per Classic+ card that matches the table.

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { HIDDEN_ID, planEntries } from "../../game/animations.ts";
import { fullBoardView, withEvents } from "../../test/fixtures.ts";
import { CATALOG } from "../../wire/cards.ts";
import { planCardFx, planCardMotif } from "../cardFx.ts";
import { FX_INTENSITY_SCALE, FX_SET_PASS_FLOOR } from "../constants.ts";
import { createFxMemory } from "../memory.ts";
import { motifScale } from "../motifs/build.ts";
import { galleryCardEvents, galleryPlayOf } from "../motifs/gallery.ts";
import { FX_CARD_HOOKS, isDrawnMotif } from "../motifs/names.ts";
import type { FxPlanEnv } from "../types.ts";
import { CARD_RECIPES, isCardRecipeKey } from "./bespoke.ts";
import rawTable from "./classicplus.md?raw";
import { FX_CARD_FILES, parseFxCardTable, type FxCardEntry } from "./tables.ts";

/** How many bespoke Flourishes the set may hold (#2795). */
const BESPOKE_MIN = 3;
const BESPOKE_MAX = 6;

/** The tokens the pass gave an entry, each where it earns one (the PR's table says why). */
const TOKENS_WITH_ENTRIES = [
  "classicplus-012-1",
  "classicplus-012-2",
  "classicplus-012-3",
  "classicplus-012-4",
  "classicplus-012-5",
  "classicplus-012-6",
  "classicplus-019-3",
  "classicplus-019-4",
  "classicplus-032-1",
  "classicplus-032-2",
  "classicplus-036-1",
  "classicplus-046-1",
  "classicplus-065-1",
  "classicplus-065-2",
  "classicplus-065-3",
  "classicplus-065-4",
  "classicplus-075-1",
  "classicplus-076-1",
  "classicplus-t-ai-06",
  "classicplus-t-ai-07",
  "classicplus-t-ai-09",
  "classicplus-t-ai-10",
];

const FILE = FX_CARD_FILES.find((file) => file.file === "classicplus.json5");
if (FILE === undefined) throw new Error("fx/cards/classicplus.json5 is not a table file");
const TABLE: ReadonlyMap<string, FxCardEntry> = parseFxCardTable(FILE.file, FILE.source);
const ENTRIES = [...TABLE];
const isToken = (defId: string): boolean => CATALOG[defId]?.token === true;
const typeOf = (defId: string): string | undefined => CATALOG[defId]?.type;
const NON_UNIT = new Set(["Spell", "Field Spell", "Trap", "Field Trap"]);

/** Whether a non-Unit's entry names what its play draws: its cast's drawn motif, a set piece or a card recipe. */
function namesItsCast(entry: FxCardEntry | undefined): boolean {
  if (entry === undefined) return false;
  if (entry.kind === "setPiece") return true;
  if (entry.kind === "bespoke") return isCardRecipeKey(entry.key);
  return entry.kind === "hooks" && entry.hooks.cast !== undefined && isDrawnMotif(entry.hooks.cast.motif);
}

/** An entry as classicplus.md's Entry column writes it (core.test.ts's form). */
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

/** Whether any entry of `events` keys the card: its motif (planCardMotif) or its recipe (planCardFx), planned as FxLayer plans them. */
function keysAny(events: readonly GameEvent[]): boolean {
  const view = fullBoardView();
  const env = envOf();
  let keyed = false;
  for (const entry of planEntries(events, withEvents(view, [...events]), false)) {
    env.memory.remember(entry.events);
    if (planCardMotif(entry, entry.view, env) !== null) keyed = true;
    for (const event of entry.events) if (planCardFx(event, { entry, view: entry.view, env, D: entry.durationMs }) !== null) keyed = true;
  }
  return keyed;
}

/**
 * The same play as another seat sees it when R97 hides it: its `cardPlayed` and `cardResolved` name the
 * sentinel, and so does a Trap's `trapFired` (R2734000), from which its cast plays (R2797000).
 */
function hidden(events: readonly GameEvent[]): GameEvent[] {
  return events.map((event) =>
    event.type === "cardPlayed" || event.type === "cardResolved" || event.type === "trapFired" ? { ...event, instanceId: HIDDEN_ID, defId: HIDDEN_ID } : event,
  );
}

describe("R2795000 Classic+'s card animation table", () => {
  it("R2795000 at least 55% of Classic+'s cards have an entry other than none, and every Spell, Field Spell, Trap and Field Trap names its cast, a set piece or a bespoke recipe", () => {
    const cards = ENTRIES.filter(([defId]) => !isToken(defId));
    expect(cards).toHaveLength(78);
    const given = cards.filter(([, entry]) => entry.kind !== "none");
    expect(given.length).toBeGreaterThanOrEqual(Math.ceil(FX_SET_PASS_FLOOR * cards.length));
    const nonUnits = cards.filter(([defId]) => NON_UNIT.has(typeOf(defId) ?? ""));
    expect(nonUnits).toHaveLength(55);
    for (const [defId, entry] of nonUnits) expect(namesItsCast(entry), defId).toBe(true);
    // A token given an entry is held to the same, and every hook names a motif that draws.
    for (const [defId, entry] of ENTRIES) {
      if (isToken(defId) && entry.kind !== "none" && NON_UNIT.has(typeOf(defId) ?? "")) expect(namesItsCast(entry), defId).toBe(true);
      if (entry.kind === "hooks") for (const [hook, spec] of Object.entries(entry.hooks)) expect(isDrawnMotif(spec.motif), `${defId} ${hook}`).toBe(true);
    }
  });

  it("R2795000 every Classic+ Field Spell given an entry wears an aura, the set's ten among them", () => {
    const fields = ENTRIES.filter(([defId]) => typeOf(defId) === "Field Spell");
    expect(fields.filter(([defId]) => !isToken(defId))).toHaveLength(10);
    for (const [defId, entry] of fields) {
      if (isToken(defId) && entry.kind === "none") continue;
      expect(entry.kind === "hooks" ? entry.aura : undefined, defId).toBeDefined();
    }
  });

  it("R2795000 Classic+ has 3 to 6 bespoke Flourishes, Crushing Walls kept, each a card recipe", () => {
    const keys = ENTRIES.flatMap(([, entry]) => (entry.kind === "bespoke" ? [entry.key] : []));
    expect(keys.length).toBeGreaterThanOrEqual(BESPOKE_MIN);
    expect(keys.length).toBeLessThanOrEqual(BESPOKE_MAX);
    expect(TABLE.get("classicplus-024")).toEqual({ kind: "bespoke", key: "crushingWalls" });
    for (const key of keys) {
      expect(isCardRecipeKey(key), key).toBe(true);
      if (isCardRecipeKey(key)) expect(typeof CARD_RECIPES[key], key).toBe("function");
    }
  });

  it("R2795000 R202 a hidden play keys no Classic+ entry, while each readable play keys its own", () => {
    const played = ENTRIES.flatMap(([defId, entry]) => {
      const play = galleryPlayOf(defId, entry);
      return play !== null && (play.kind === "board" || play.kind === "recipe") ? [{ defId, play }] : [];
    });
    expect(played.length).toBeGreaterThan(40);
    const view = fullBoardView();
    for (const card of played) {
      const events = galleryCardEvents(card.defId, card.play, view);
      expect(keysAny(events), card.defId).toBe(true);
      expect(keysAny(hidden(events)), `${card.defId} hidden`).toBe(false);
    }
  });

  it("R2795000 classicplus.md has one row per Classic+ card, in order, with its name, type, entry, scale, census and why", () => {
    const rows = rawTable
      .split("\n")
      .filter((line) => line.startsWith("| `classicplus-"))
      .map((line) => line.slice(1, -1).split(" | ").map((cell) => cell.trim()));
    expect(rows.map((row) => row[0])).toEqual(ENTRIES.map(([defId]) => `\`${defId}\``));
    const view = fullBoardView();
    for (const [cell, name, type, entry, scale, census, why] of rows) {
      const defId = (cell ?? "").replace(/`/g, "");
      const def = CATALOG[defId];
      const own = TABLE.get(defId);
      if (own === undefined) throw new Error(`${defId} has no entry`);
      expect(name, defId).toBe(def?.name.replace(/\s+/g, " ").trim());
      expect(type, defId).toBe(def?.type);
      expect(entry, defId).toBe(describeEntry(own));
      expect(scale, defId).toBe(own.kind === "none" ? "—" : motifScale(defId, view, 1).toFixed(2));
      expect(census, defId).toMatch(/\S/);
      expect(why, defId).toMatch(/\S/);
    }
  });

  it("R2795000 the tokens given entries are the twenty-two the pass lists", () => {
    const tokens = ENTRIES.filter(([defId, entry]) => isToken(defId) && entry.kind !== "none").map(([defId]) => defId);
    expect(tokens).toEqual(TOKENS_WITH_ENTRIES);
    expect(ENTRIES.filter(([defId]) => isToken(defId))).toHaveLength(38);
  });
});
