// R3405001: Meditative's 32 signatures together (each card's own beats are its batch's tests beside this
// one: signatures.spells, .fields, .units-a and .units-b): every card's entry is its signature,
// as the issue's table sets it; from the caster's opponent's seat no beat of any of them names a card,
// shows a face or flies more than a card back (R202); and every cue starts inside its entry and is gone
// within FX_MAX_TAIL_MS of its end, its entrance's included, while nothing draws with the effects off (R200).

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid } from "../../../game/animations.ts";
import { LANES, testid } from "../../../game/contract.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import { HOLD, castOf, endOf, played, run } from "../../../test/signatures.ts";
import { FX_INTENSITY_SCALE, FX_MAX_TAIL_MS } from "../../constants.ts";
import { planFx } from "../../cues.ts";
import { ENTRANCES } from "../../entrances.ts";
import { createFxMemory } from "../../memory.ts";
import { galleryEntry, RECIPE_FIXTURES } from "../../motifs/gallery.ts";
import type { FxCue } from "../../types.ts";
import type { CardRecipeKey } from "../bespoke.ts";
import type { EntranceKey } from "../recipe.ts";
import { FX_CARD_TABLE, type FxAuraSpec, type OwnTakeoverLook } from "../tables.ts";

/** One card of the issue's table: its bespoke recipe, entrance, own takeover look and aura, beside `signature: true`. */
type Row = { b?: CardRecipeKey; e?: EntranceKey; t?: OwnTakeoverLook; a?: FxAuraSpec };

const aura = (look: FxAuraSpec["look"], scope: FxAuraSpec["scope"], palette?: FxAuraSpec["palette"]): FxAuraSpec => ({ look, scope, ...(palette === undefined ? {} : { palette }) });

/** The issue's table, in catalog order. */
const TABLE: Readonly<Record<string, Row>> = {
  "meditative-007": { b: "introspection", e: "poolBreath", a: aura("ripple", "zone", "Holy") },
  "meditative-008": { b: "summitAscent", t: "ascent" },
  "meditative-011": { b: "doubleHeader", e: "floodlights", a: aura("arcane", "zone") },
  "meditative-017": { b: "trueCraft", t: "workshop" },
  "meditative-020": { b: "brushSeal", t: "brush" },
  "meditative-026": { b: "alternateFate", e: "crackedEarth", a: aura("void", "zone") },
  "meditative-039-4": { b: "redJade", t: "mine" },
  "meditative-039-5": { b: "jadeBeauty", e: "jadeBoulder", t: "quarry", a: aura("steam", "tile", "Hustle/jade") },
  "meditative-040": { b: "fengShui", e: "fiveCompass", a: aura("orbit", "zone") },
  "meditative-042": { b: "fleaMarket" },
  "meditative-045": { b: "knowledgeBreaker", e: "chalkMask", a: aura("chalk", "tile") },
  "meditative-045-1": { b: "breakerPrime", e: "maskSplit", a: aura("storm", "tile", "Nerd/chalk") },
  "meditative-047": { b: "taotie", e: "bronzeCauldron", a: aura("steam", "tile", "Toxic") },
  "meditative-049": { b: "yileGpt", e: "scrollScholar", t: "ai", a: aura("drift", "tile", "Tech/hologram") },
  "meditative-049-1": { b: "yilePrime", e: "pavilionUnfurl", t: "pavilion", a: aura("orbit", "tile", "Tech/hologram") },
  "meditative-049-2": { b: "yileVirus", e: "virusBoot", t: "virus", a: aura("glitch", "tile", "Tech/glitch") },
  "meditative-049-3": { b: "aiGirlfriend", e: "dojoBoot", t: "dojo", a: aura("holy", "tile", "Tech/hologram") },
  "meditative-051": { b: "devinBot", e: "cushionDrop", a: aura("incense", "tile", "Tech/hologram") },
  "meditative-063": { b: "skullJnari", e: "skullThud", a: aura("ember", "zone", "Dark/curse") },
  "meditative-065": { b: "keenusKeys", e: "keyRing", a: aura("orbit", "tile", "Physical/steel") },
  "meditative-071": { b: "pareto", e: "studyCurve", t: "studyHall", a: aura("holy", "lane") },
  "meditative-071-1": { e: "caneRise", t: "caneRack", a: aura("storm", "zone") },
  "meditative-075": { b: "everGrowingTree", e: "treeSprout", a: aura("bloom", "lane") },
  "meditative-079": { b: "touchedByKy", e: "chalkHand", a: aura("chalk", "tile", "Holy") },
  "meditative-080": { b: "aluneth", e: "lanternLight", a: aura("frost", "zone", "Nerd/ink") },
  "meditative-086": { e: "coinFlip", a: aura("orbit", "tile", "Hustle/cash") },
  "meditative-087": { b: "tatches", e: "totemStack", a: aura("holy", "tile") },
  "meditative-095": { b: "jadeDie" },
  "meditative-095-1": { b: "cnGolem", e: "golemLock", a: aura("timber", "tile", "Toxic") },
  "meditative-097": { b: "blueprints", t: "warRoom" },
  "meditative-097-8": { b: "prison", e: "prisonBars", a: aura("drift", "tile", "Dark/shadow") },
  "meditative-097-9": { b: "jlockeedHq", e: "fortressRise", t: "fortress", a: aura("ember", "tile", "Tech/rocket") },
};

/** The entry `row` parses to: a bespoke entry, or (The Cane, the Mayor) an entrance alone, a hooks entry of no hook. */
function entryOf(row: Row): object {
  const frame = { ...(row.e === undefined ? {} : { entrance: row.e }), ...(row.t === undefined ? {} : { takeover: row.t }), signature: true };
  const worn = row.a === undefined ? {} : { aura: row.a };
  return row.b === undefined ? { kind: "hooks", hooks: {}, ...worn, ...frame } : { kind: "bespoke", key: row.b, ...worn, ...frame };
}

/** The ids the caster's hand, deck and face-down cards carry on the other seat (R97). */
const HIDDEN = "hidden";

/** `event` as the caster's opponent reads it: every card in a hand or a deck, and every face-down card, is hidden. */
function asOpponent(event: GameEvent): GameEvent {
  switch (event.type) {
    case "addedToHand":
    case "drawn":
    case "shuffledIn":
      return { ...event, instanceId: HIDDEN, defId: HIDDEN };
    case "costChanged":
      return { ...event, instanceId: HIDDEN };
    case "radiantSet":
      return event.zone.z === "hand" ? { ...event, instanceId: HIDDEN, defId: HIDDEN } : event;
    case "summoned":
      return event.row === "backrow" && event.formerId !== undefined ? { ...event, instanceId: HIDDEN, defId: HIDDEN } : event;
    default:
      return event;
  }
}

/** The cards whose text shows a Discover, a hand, a deck or a face-down card (the issue's acceptance list). */
const HIDING = [
  "meditative-007", "meditative-008", "meditative-011", "meditative-017", "meditative-040", "meditative-042", "meditative-045", "meditative-045-1",
  "meditative-049", "meditative-049-1", "meditative-049-2", "meditative-063", "meditative-075", "meditative-079", "meditative-080", "meditative-087",
  "meditative-095", "meditative-095-1", "meditative-097",
] as const;

const CARD_ID = /(core|classic|meditative|stinkyscape|traininggrounds)[a-z]*-\d/;

function keyOf(defId: string): CardRecipeKey {
  const key = TABLE[defId]?.b;
  if (key === undefined) throw new Error(`${defId} has no recipe`);
  return key;
}

describe("R3405001 Meditative's signatures", () => {
  it("R3405001 the 32 are signatures with their entries", () => {
    expect(Object.keys(TABLE)).toHaveLength(32);
    for (const [defId, row] of Object.entries(TABLE)) expect(FX_CARD_TABLE.get(defId), defId).toEqual(entryOf(row));
  });

  it("R3405001 R202 from the opponent's seat no beat names a card or shows a face", () => {
    const view = fullBoardView();
    for (const defId of HIDING) {
      // The caster is the other seat: its play's fixture as that seat's opponent reads it, a Discover's prompt bare.
      const events: GameEvent[] = [
        { type: "promptOpened", player: "p2" },
        { type: "promptAnswered", player: "p2" },
        ...RECIPE_FIXTURES[keyOf(defId)](view, "ex", "p2").map(asOpponent),
      ];
      const cues = [...run(defId, events, 400, "p2", view).flatMap((entry) => entry.cues), ...castOf(defId, 1400, HOLD, "p2", view).cues];
      const text = JSON.stringify(cues);
      expect(text, defId).not.toMatch(CARD_ID);
      expect(text, defId).not.toContain(`"${HIDDEN}"`);
      // No sheen of its own (a face is its owner's to read). R3399000: a card it summons lands by that card's own
      // entrance, public, its shimmer included (Claude's Datacenter under YileGPT Prime's Cry).
      const own = new Map([...FX_CARD_TABLE].filter(([id]) => id === defId));
      const owned = run(defId, events, 400, "p2", view, undefined, own).flatMap((entry) => entry.cues);
      expect([...owned, ...castOf(defId, 1400, HOLD, "p2", view).cues].some((cue) => cue.kind === "sheen"), defId).toBe(false);
      const piles = new Set([animTestid.hand("opponent"), animTestid.library("opponent"), animTestid.hand("you"), animTestid.library("you"), animTestid.graveyard("you"), animTestid.graveyard("opponent"), animTestid.exile("you"), animTestid.exile("opponent")]);
      // Or, set face down (Knowledge Breaker Prime's Cry), into a backrow zone.
      for (const side of ["you", "opponent"] as const) for (const lane of LANES) piles.add(testid.zone(side, "backrow", lane));
      for (const cue of cues) {
        if (cue.kind !== "ghost") continue;
        // A card flies only as a card back, to a pile.
        expect(cue.to.kind === "testid" && piles.has(cue.to.testid), `${defId} ghost to ${JSON.stringify(cue.to)}`).toBe(true);
      }
    }
  });

  it("R3405001 R200 every cue ends within FX_MAX_TAIL_MS, nothing with the effects off", () => {
    const view = fullBoardView();
    const within = (cues: readonly FxCue[], D: number, label: string): void => {
      for (const cue of cues) {
        const where = `${label} ${cue.kind} at D=${String(D)}`;
        expect(cue.delayMs, where).toBeGreaterThanOrEqual(0);
        expect(cue.delayMs, where).toBeLessThanOrEqual(D);
        if (cue.kind === "ghost") expect(cue.delayMs + cue.durationMs, where).toBeLessThanOrEqual(D);
        expect(endOf(cue), where).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
        if (cue.kind === "shake") expect(cue.trauma, where).toBeLessThanOrEqual(1);
      }
    };
    for (const D of [120, 400, 1400]) {
      for (const [defId, row] of Object.entries(TABLE)) {
        if (row.b !== undefined) {
          const fixture = run(defId, RECIPE_FIXTURES[row.b](view, "ex", "p1"), D, "p1", view).flatMap((entry) => entry.cues);
          within([...fixture, ...castOf(defId, D, Math.min(HOLD, D)).cues], D, defId);
        }
        if (row.e === undefined) continue;
        const field = row.a?.scope === "zone" || row.a?.scope === "lane";
        const landing: Extract<GameEvent, { type: "summoned" }> = { type: "summoned", player: "p1", instanceId: "ex", defId, row: field ? "backrow" : "units", lane: field ? 1 : 3 };
        const tgt = testid.zone("you", landing.row, landing.lane);
        for (const lead of [0, Math.min(HOLD, D)]) within(ENTRANCES[row.e]({ D, tgt, intensity: FX_INTENSITY_SCALE.normal, view, landing, leadMs: lead }), D, `${row.e} after ${String(lead)}`);
      }
    }
    // The whole effects layer off draws nothing.
    for (const [defId, row] of Object.entries(TABLE)) {
      const env = { intensity: FX_INTENSITY_SCALE.off, card: () => undefined, memory: createFxMemory() };
      const events: GameEvent[] = [played("p1", "ex", defId), ...(row.b === undefined ? [] : RECIPE_FIXTURES[row.b](view, "ex", "p1"))];
      const cues = events.flatMap((event) => {
        env.memory.remember([event]);
        return planFx({ ...galleryEntry([event], view), durationMs: 400 }, view, env);
      });
      expect(cues, defId).toEqual([]);
    }
  });
});
