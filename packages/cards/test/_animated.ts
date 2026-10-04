// Patch v0.2.10 (issue #113; SPEC §6.1 Animated, R383, R445): the eighteen Field Spells that gained
// Animated. Each card's own test runs this check on both faces: played from hand into backrow lane 3, the
// card animates at once into unit lane 3 as a Unit of its unit face (its own aura counted, as the layers
// compute it), summoning sick, reported `animated` and never `summoned`; with unit lane 3 taken it goes
// to the leftmost open unit zone; with the unit row full it stays a Field Spell in its backrow zone.

import type { GameEvent } from "@jackioh/shared";
import { expect } from "vitest";
import { scenario, type PileSetup, type PlayOptions, type Scenario } from "./_harness";

const FILLER = "core-005"; // (1) Stockpile, a Spell.
const VANILLA = "core-008"; // a plain Unit, to take unit zones.
const DECK = [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER];

export type AnimatedCheck = {
  def: string;
  radiant?: boolean;
  /** Its stats once animated, as the layers read them (its own aura included). */
  stats: { attack: number; health: number };
  /** Extra play choices its text declares (an X, a Cry's pick), read off the game when they name a card. */
  play?: PlayOptions | ((s: Scenario) => PlayOptions);
  /** The rest of p1's hand, when its Cry reads the hand (default one Stockpile). */
  hand?: readonly PileSetup[];
};

function played(check: AnimatedCheck, field: readonly string[]): { s: Scenario; id: string; turn: number } {
  const s = scenario({
    seed: `animated-${check.def}`,
    p1: {
      hand: [{ def: check.def, radiant: check.radiant === true }, ...(check.hand ?? [FILLER])],
      field: field.map((def, at) => ({ def, lane: field.length === 1 ? 3 : at + 1 })),
      library: DECK,
      mana: 10,
    },
    p2: { hand: [FILLER], library: DECK },
  });
  const id = s.card(check.def).id;
  const turn = s.state.turn;
  s.play(id, { zone: 3, ...(typeof check.play === "function" ? check.play(s) : check.play) });
  return { s, id, turn };
}

const animatedEvents = (s: Scenario, id: string): Extract<GameEvent, { type: "animated" }>[] =>
  s.events.filter((event): event is Extract<GameEvent, { type: "animated" }> => event.type === "animated" && event.instanceId === id);

/** R383: the whole check, for one face of one card. */
export function expectAnimated(check: AnimatedCheck): void {
  // Its lane's unit zone open: it animates there at once, a summoning-sick Unit.
  const open = played(check, []);
  expect(open.s.unit("p1", 3)?.id).toBe(open.id);
  expect(open.s.backrow("p1", 3)).toBeNull();
  open.s.expectStats(open.id, check.stats);
  expect(open.s.card(open.id).summonedTurn).toBe(open.turn);
  expect(animatedEvents(open.s, open.id)).toMatchObject([{ backrowLane: 3, unitLane: 3 }]);
  // R445: animating is not a summon — the play put it in the backrow, and nothing summoned it to a unit zone.
  const summons = open.s.events.filter((event) => event.type === "summoned" && event.instanceId === open.id);
  expect(summons).toMatchObject([{ row: "backrow", lane: 3 }]);

  // Its lane's unit zone taken: the leftmost open one (R64).
  const taken = played(check, [VANILLA]);
  expect(taken.s.unit("p1", 1)?.id).toBe(taken.id);

  // No open unit zone: it stays a Field Spell in its backrow zone.
  const full = played(check, [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA]);
  expect(full.s.backrow("p1", 3)?.id).toBe(full.id);
  expect(animatedEvents(full.s, full.id)).toEqual([]);
}
