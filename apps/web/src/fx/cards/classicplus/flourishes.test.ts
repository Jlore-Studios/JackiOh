// R2795000: Classic+'s three new Flourishes (#35 Rollback, #41 KY's Constant, #56 Book of Pain), planned
// the way FxLayer plans them (remember, then plan) from the gallery's fixtures of their plays: what each
// draws, R200's bounds on every cue at short and long entries, nothing with the effects off, and R202:
// a hidden play keys none of them.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid, planEntries, type AnimationEntry } from "../../../game/animations.ts";
import { LANES, testid } from "../../../game/contract.ts";
import { fullBoardView, withEvents } from "../../../test/fixtures.ts";
import {
  FX_INTENSITY_SCALE,
  FX_MAX_PARTICLE_LIFE_MS,
  FX_MAX_TAIL_MS,
  FX_MOTIF_PHASES,
  FX_ROLLBACK_STEP,
} from "../../constants.ts";
import { planFx } from "../../cues.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { createFxMemory } from "../../memory.ts";
import { timesOf } from "../../motifs/build.ts";
import { RECIPE_FIXTURES } from "../../motifs/gallery.ts";
import type { FxCue, FxPlanEnv } from "../../types.ts";
import { FX_CARD_TABLE } from "../tables.ts";
import type { CardRecipeKey } from "../bespoke.ts";

const ROLLBACK = "classicplus-035";
const CONSTANT = "classicplus-041";
const PAIN = "classicplus-056";

/** Each Flourish's card and its recipe, as the table names them. */
const FLOURISHES: readonly (readonly [string, CardRecipeKey])[] = [
  [ROLLBACK, "rollbackSweep"],
  [CONSTANT, "constantStamp"],
  [PAIN, "inkTear"],
];

function envOf(over: Partial<FxPlanEnv> = {}): FxPlanEnv {
  return { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory(), ...over };
}

/** Plans each entry of `events` in order, as FxLayer does on every `start`: remember, then plan. */
function planAll(events: readonly GameEvent[], view: PlayerView, env: FxPlanEnv, D?: number): { entry: AnimationEntry; cues: FxCue[] }[] {
  const planned = planEntries(events, withEvents(view, [...events]), false);
  return planned.map((original) => {
    const entry = D === undefined ? original : { ...original, durationMs: D };
    env.memory.remember(entry.events);
    return { entry, cues: planFx(entry, entry.view, env) };
  });
}

/** Where a cue ends, in ms after its entry starts (R200's bound is D + FX_MAX_TAIL_MS). */
function endOf(cue: FxCue): number {
  switch (cue.kind) {
    case "burst":
      return cue.delayMs + FX_MAX_PARTICLE_LIFE_MS;
    case "projectile":
    case "shot":
      return cue.delayMs + cue.flightMs + FX_MAX_PARTICLE_LIFE_MS;
    case "shake":
      return cue.delayMs;
    default:
      return cue.delayMs + ("durationMs" in cue ? cue.durationMs : 0);
  }
}

const played = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 1 });
const resolved = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({
  type: "cardResolved",
  player,
  instanceId,
  defId,
  permanent: false,
  costPaid: 1,
});

/** `player` plays `defId` under the instance `instanceId` and its recipe's fixture resolves under it. */
function stream(defId: string, key: CardRecipeKey, view: PlayerView, player: PlayerId = "p1", instanceId = "ex"): GameEvent[] {
  const shownId = instanceId === "hidden" ? "hidden" : defId;
  return [played(player, instanceId, shownId), ...RECIPE_FIXTURES[key](view, instanceId, player), resolved(player, instanceId, shownId)];
}

/** The cues of every entry of the play but its own `cardPlayed` and `cardResolved`. */
function effectCues(events: readonly GameEvent[], D = 400, env: FxPlanEnv = envOf()): FxCue[] {
  return planAll(events, fullBoardView(), env, D)
    .filter(({ entry }) => !entry.events.some((event) => event.type === "cardPlayed" || event.type === "cardResolved"))
    .flatMap(({ cues }) => cues);
}

const at = (cue: FxCue): string => ("at" in cue && cue.at.kind === "testid" ? cue.at.testid : "");

describe("R2795000 Classic+'s Flourishes", () => {
  it("R2795000 R3399000 each Flourish is its card's bespoke entry, Rollback's now its signature", () => {
    for (const [defId, key] of FLOURISHES) {
      expect(FX_CARD_TABLE.get(defId), defId).toEqual(defId === ROLLBACK ? { kind: "bespoke", key, signature: true } : { kind: "bespoke", key });
    }
  });

  it("R2795000 Rollback rewinds lane 5 to lane 1 on both sides, then glints both heroes; its row still plays", () => {
    const view = fullBoardView();
    const cues = effectCues(stream(ROLLBACK, "rollbackSweep", view));
    const rings = cues.flatMap((cue) => (cue.kind === "ring" && cue.preset === "arcane" && at(cue).startsWith("zone-") ? [`${at(cue)}@${String(cue.delayMs)}`] : []));
    // At D 400: from the entry's start, FX_ROLLBACK_STEP (32 ms) between lanes, lane 5 first (#3399 moved it up from Travel).
    expect(rings).toEqual([5, 4, 3, 2, 1].flatMap((lane, k) => ["you", "opponent"].map((side) => `${testid.zone(side as "you" | "opponent", "units", lane)}@${String(32 * k)}`)));
    // The heroes glint a step after lane 1.
    const glint = Math.round(LANES.length * FX_ROLLBACK_STEP * 400);
    const glints = cues.filter((cue) => cue.kind === "burst" && cue.preset === "sparkle" && cue.delayMs === glint).map(at);
    expect(glints).toEqual([testid.hero("you"), testid.hero("opponent")]);
    // In the card's own look (Physical's sepia), at its scale.
    for (const cue of cues) if (cue.kind === "burst" && cue.preset === "sparkle") expect(cue.colors).toEqual(FLAVOR_LOOKS.Physical.colors);
    // The row's own rewind (the board's sheen) still plays.
    expect(cues.some((cue) => cue.kind === "sheen" && at(cue) === testid.board)).toBe(true);
    // #3399: its cast frames the screen in a stone clock (twelve ticks and a ring at twelve), its hand stutters
    // three times, and a tint washes every unit zone, lane 5 first, each side in turn.
    const onScreen = (cue: FxCue): boolean => "at" in cue && cue.at.kind === "viewport";
    const cast = planAll(stream(ROLLBACK, "rollbackSweep", view), view, envOf(), 400).find(({ entry }) => entry.events.some((event) => event.type === "cardPlayed"))?.cues ?? [];
    expect(cast.filter((cue) => cue.kind === "burst" && cue.preset === "dust" && onScreen(cue))).toHaveLength(12);
    expect(cast.filter((cue) => cue.kind === "ring" && cue.preset === "dust" && onScreen(cue))).toHaveLength(1);
    expect(cast.filter((cue) => cue.kind === "burst" && cue.preset === "arcane" && cue.spread === "point" && onScreen(cue))).toHaveLength(3);
    const tint = cast.flatMap((cue) => (cue.kind === "burst" && cue.preset === "smoke" ? [at(cue)] : []));
    expect(tint).toEqual([5, 4, 3, 2, 1].flatMap((lane) => ["you", "opponent"].map((side) => testid.zone(side as "you" | "opponent", "units", lane))));
    // At the rewind its hand swings back a tick for each turn rolled back (beside the row's own arcane ring at
    // the middle): one here, three for three turns.
    const swings = (planned: readonly FxCue[]): number => planned.filter((cue) => cue.kind === "burst" && cue.preset === "arcane" && cue.spread === "point" && onScreen(cue)).length;
    expect(swings(cues)).toBe(1);
    const three: GameEvent[] = [played("p1", "ex", ROLLBACK), { type: "rolledBack", player: "p1", turnsAgo: 3, sides: ["p1", "p2"] }, resolved("p1", "ex", ROLLBACK)];
    expect(swings(effectCues(three))).toBe(3);
  });

  it("R2795000 Rollback rings only the sides its rollback names", () => {
    const events: GameEvent[] = [played("p1", "ex", ROLLBACK), { type: "rolledBack", player: "p1", turnsAgo: 2, sides: ["p2"] }, resolved("p1", "ex", ROLLBACK)];
    const rings = effectCues(events).filter((cue) => cue.kind === "ring" && at(cue).startsWith("zone-"));
    expect(rings).toHaveLength(5);
    for (const ring of rings) expect(at(ring)).toMatch(/^zone-opponent-units-/);
  });

  it("R2795000 Book of Pain tears each discarded card out in ink, the first with a ring and a shake", () => {
    const view = fullBoardView();
    const cues = effectCues(stream(PAIN, "inkTear", view));
    const ink = paletteLook("Nerd/ink")?.colors;
    const t = timesOf(400);
    const tears = cues.filter((cue) => cue.kind === "burst" && JSON.stringify(cue.colors) === JSON.stringify(ink) && cue.delayMs === t.G);
    expect(tears).toHaveLength(2);
    const drops = cues.filter((cue) => cue.kind === "burst" && JSON.stringify(cue.colors) === JSON.stringify(ink) && cue.delayMs === t.L);
    expect(drops.map(at)).toEqual([animTestid.graveyard("opponent"), animTestid.graveyard("opponent")]);
    expect(cues.filter((cue) => cue.kind === "ring" && cue.preset === "arcane")).toHaveLength(1);
    expect(cues.filter((cue) => cue.kind === "shake")).toHaveLength(1);
    // Each card's own row (its back flying to the graveyard) still plays.
    expect(cues.filter((cue) => cue.kind === "ghost")).toHaveLength(2);
  });

  it("R2795000 KY's Constant stamps its card in chalk", () => {
    const view = fullBoardView();
    const card = Array.isArray(view.you.hand) ? view.you.hand[0] : undefined;
    expect(card).toBeDefined();
    const target = `hand-card-${card?.instanceId ?? ""}`;
    const cues = effectCues(stream(CONSTANT, "constantStamp", view));
    const t = timesOf(400);
    const chalk = paletteLook("Nerd/chalk");
    expect(cues).toContainEqual(expect.objectContaining({ kind: "ring", preset: "dust", at: { kind: "testid", testid: target }, delayMs: t.G }));
    expect(cues).toContainEqual({ kind: "sheen", at: { kind: "testid", testid: target }, delayMs: t.T, durationMs: t.L - t.T });
    expect(cues).toContainEqual(expect.objectContaining({ kind: "burst", preset: chalk?.particle, colors: chalk?.colors, delayMs: t.L }));
    // A change this seat may not read stamps nothing beyond the row's own.
    const veiled = stream(CONSTANT, "constantStamp", view).map((event) => (event.type === "numberChanged" ? { ...event, hiddenFrom: ["p1" as const] } : event));
    expect(effectCues(veiled).some((cue) => cue.kind === "ring" && cue.preset === "dust")).toBe(false);
  });

  it("R2795000 R200 every Flourish starts inside its entry and ends within FX_MAX_TAIL_MS", () => {
    expect(FX_MOTIF_PHASES.travel + 4 * FX_ROLLBACK_STEP).toBeLessThanOrEqual(FX_MOTIF_PHASES.land);
    for (const D of [120, 200, 400, 732, 1400]) {
      for (const [defId, key] of FLOURISHES) {
        const cues = effectCues(stream(defId, key, fullBoardView()), D);
        expect(cues.length, `${defId} at D=${String(D)}`).toBeGreaterThan(0);
        for (const cue of cues) {
          const label = `${defId} ${cue.kind} at D=${String(D)}`;
          expect(cue.delayMs, label).toBeGreaterThanOrEqual(0);
          expect(cue.delayMs, label).toBeLessThanOrEqual(D);
          expect(endOf(cue), label).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
          if (cue.kind === "shake") expect(cue.trauma, label).toBeLessThanOrEqual(1);
        }
      }
    }
  });

  it("R2795000 R200 the effects off draws none", () => {
    for (const [defId, key] of FLOURISHES) expect(effectCues(stream(defId, key, fullBoardView()), 400, envOf({ intensity: 0 })), defId).toEqual([]);
  });

  it("R2795000 R202 a hidden play keys none", () => {
    for (const [defId, key] of FLOURISHES) {
      const view = fullBoardView();
      const fixture = RECIPE_FIXTURES[key](view, "hidden", "p2");
      const hiddenPlay = effectCues(stream(defId, key, view, "p2", "hidden"));
      const bare = planAll(fixture, view, envOf(), 400).flatMap(({ cues }) => cues);
      expect(hiddenPlay, defId).toEqual(bare);
      const readable = effectCues(stream(defId, key, view, "p2"));
      expect(readable, defId).not.toEqual(bare);
      expect(JSON.stringify(readable), defId).not.toMatch(/classicplus-|core-\d{3}/);
    }
  });
});
