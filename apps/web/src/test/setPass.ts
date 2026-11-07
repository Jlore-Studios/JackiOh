// #2793 #2799: what each set's card animation pass test (fx/cards/<set>.test.ts) shares
// (docs/polish/9-card-animations.md, "A set's pass"): the set's cards, an entry as the pass's per-card table
// (fx/cards/<set>.md) writes it, that table held to the catalog and the effects table, and each play's motif
// drawn on its gallery fixture as the board plans it.

import type { GameEvent, PlayerId } from "@jackioh/shared";
import { expect } from "vitest";

import { planCardMotif, type MotifResult } from "../fx/cardFx.ts";
import { FX_CARD_TABLE, type FxCardEntry } from "../fx/cards/tables.ts";
import { FX_INTENSITY_SCALE } from "../fx/constants.ts";
import { createFxMemory } from "../fx/memory.ts";
import { motifScale } from "../fx/motifs/build.ts";
import { galleryEntry, motifFixture } from "../fx/motifs/gallery.ts";
import { FX_CARD_HOOKS, isDrawnMotif, type DrawnMotif } from "../fx/motifs/names.ts";
import type { FxPlanEnv } from "../fx/types.ts";
import { CATALOG } from "../wire/cards.ts";
import { fullBoardView } from "./fixtures.ts";

/** A set's cards, tokens included, in catalog order. */
export function setCards(set: string): string[] {
  return Object.keys(CATALOG).filter((id) => CATALOG[id]?.set === set);
}

export function entryOf(id: string): FxCardEntry {
  const entry = FX_CARD_TABLE.get(id);
  if (entry === undefined) throw new Error(`${id} has no entry`);
  return entry;
}

/** An entry as a pass's Entry column writes it. */
export function describeEntry(entry: FxCardEntry): string {
  switch (entry.kind) {
    case "none":
      return `none: ${entry.reason}`;
    case "bespoke": {
      // R3402000: a signature first, then its entrance, its recipe and its aura.
      const parts = [entry.signature === true ? "signature" : undefined, entry.entrance === undefined ? undefined : `entrance ${entry.entrance}`, `bespoke ${entry.key}`];
      if (entry.aura !== undefined) parts.push(["aura", entry.aura.look, entry.aura.scope, entry.aura.palette].filter((part) => part !== undefined).join(" "));
      return parts.filter((part) => part !== undefined).join("; ");
    }
    case "setPiece":
      return `setPiece ${entry.key}`;
    case "hooks": {
      const parts: string[] = [];
      // R3332000: a signature first, then its entrance, then the hooks.
      if (entry.signature === true) parts.push("signature");
      if (entry.entrance !== undefined) parts.push(`entrance ${entry.entrance}`);
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
export function playMotif(defId: string, instanceId: string, motif: DrawnMotif): MotifResult | null {
  const view = fullBoardView();
  const env = envOf();
  env.memory.remember([played("p1", instanceId, defId)]);
  const entry = { ...galleryEntry(motifFixture(motif, view, instanceId, "p1"), view), durationMs: 400 };
  return planCardMotif(entry, view, env);
}

/** Each card's play motif: its `cry` (a Unit) or `cast`, when a drawn motif. */
export function playMotifs(ids: readonly string[]): { id: string; motif: DrawnMotif }[] {
  return ids.flatMap((id) => {
    const entry = entryOf(id);
    if (entry.kind !== "hooks") return [];
    const hook = entry.hooks[CATALOG[id]?.type === "Unit" ? "cry" : "cast"];
    return hook !== undefined && isDrawnMotif(hook.motif) ? [{ id, motif: hook.motif }] : [];
  });
}

/**
 * Holds a pass's per-card table (`raw`, its rows those whose first cell is a backticked id of `prefix`) to
 * `ids`: one row per card, in order, with the card's name, type, entry and motif scale, a census and a why.
 */
export function expectPassTable(raw: string, prefix: string, ids: readonly string[]): void {
  const rows = raw
    .split("\n")
    .filter((line) => line.startsWith(`| \`${prefix}-`))
    .map((line) => line.slice(1, -1).split(" | ").map((cell) => cell.trim()));
  expect(rows.map((row) => row[0])).toEqual(ids.map((id) => `\`${id}\``));
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
}
