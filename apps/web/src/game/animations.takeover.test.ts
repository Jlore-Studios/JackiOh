// R2722001, R2722002: a Legendary's or a Mythic's entrance in the animation queue, and the one capped
// hold a readable Mythic's takeover takes per action (`entranceEntries`, `TAKEOVER_HOLD_MS`).

import type { CardType, GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import {
  ANIMATIONS,
  TAKEOVER_HOLD_MS,
  createAnimationQueue,
  entranceEntries,
  leadMs,
  planEntries,
  scaleForSpeed,
  type AnimationEntry,
  type EntranceResolver,
} from "./animations.ts";
import { baseView, emptySide } from "../test/fixtures.ts";

/** Each kind of card at each rarity the entrance reads. */
const CARDS: Record<string, { type: CardType; rarity: string }> = {
  "unit-mythic": { type: "Unit", rarity: "Mythic" },
  "unit-legend": { type: "Unit", rarity: "Legendary" },
  "unit-epic": { type: "Unit", rarity: "Epic" },
  "spell-mythic": { type: "Spell", rarity: "Mythic" },
  "field-mythic": { type: "Field Spell", rarity: "Mythic" },
  "trap-mythic": { type: "Trap", rarity: "Mythic" },
  "trap-legend": { type: "Field Trap", rarity: "Legendary" },
  "token-mythic": { type: "Unit", rarity: "Mythic" },
};
const resolve: EntranceResolver = (defId) => CARDS[defId] ?? null;

const p1 = baseView();
const p2 = baseView({ viewer: "p2", you: emptySide("p2"), opponent: emptySide("p1") });

const play = (defId: string, instanceId = `i-${defId}`, player: PlayerId = "p1"): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 3 });
const summon = (defId: string, instanceId = `i-${defId}`, player: PlayerId = "p1"): GameEvent => ({ type: "summoned", player, instanceId, defId, row: "units", lane: 2 });
const fired = (defId: string, instanceId: string, again = false): GameEvent => ({ type: "trapFired", instanceId, defId, controller: "p1", row: "backrow", lane: 3, ...(again ? { again: true } : {}) });
const revealed = (defId: string): GameEvent => ({ type: "activated", player: "p1", instanceId: "c61", defId, ability: "reveal" });

function plan(events: GameEvent[], view: PlayerView = p1, opts: { reduced?: boolean; speed?: number; seen?: Set<string> } = {}): AnimationEntry[] {
  return entranceEntries(planEntries(events, view, false), resolve, { reduced: opts.reduced ?? false, speed: opts.speed ?? 1, seen: opts.seen ?? new Set() });
}
const holds = (entries: readonly AnimationEntry[]): number[] => entries.map((entry) => entry.entrance?.holdMs ?? -1);

describe("R2722001 a readable Mythic's entrance holds its entry for the takeover", () => {
  it("R2722001 a readable Mythic Unit or Spell play takes the hold; a Legendary holds nothing; an Epic, a hidden play and a face-down set take none", () => {
    const [unit] = plan([play("unit-mythic"), summon("unit-mythic")]);
    expect(unit?.entrance).toMatchObject({ tier: "mythic", defId: "unit-mythic", reveal: false, holdMs: TAKEOVER_HOLD_MS, anchor: "zone-you-units-2" });
    expect(unit?.durationMs).toBe(ANIMATIONS.cardPlayed.durationMs + TAKEOVER_HOLD_MS);
    expect(leadMs(unit ?? { })).toBe(TAKEOVER_HOLD_MS);
    const [spell] = plan([play("spell-mythic")]);
    expect(spell?.entrance).toMatchObject({ tier: "mythic", holdMs: TAKEOVER_HOLD_MS });
    // A token printed Mythic summoned without a play makes its entrance on its summoned.
    expect(plan([summon("token-mythic", "t1")])[0]?.entrance?.holdMs).toBe(TAKEOVER_HOLD_MS);

    const [legend] = plan([play("unit-legend"), summon("unit-legend")]);
    expect(legend?.entrance).toMatchObject({ tier: "legendary", holdMs: 0 });
    expect(legend?.durationMs).toBe(ANIMATIONS.cardPlayed.durationMs);
    expect(plan([play("unit-epic"), summon("unit-epic")])[0]?.entrance).toBeUndefined();
    // The other seat's hidden play, and the controller's own face-down set, which took a fresh id (R227).
    expect(plan([play("hidden", "hidden")], p2)[0]?.entrance).toBeUndefined();
    const set: GameEvent = { type: "cardPlayed", player: "p1", instanceId: "c60", defId: "unit-mythic", costPaid: 1, formerId: "c56" };
    expect(plan([set])[0]?.entrance).toBeUndefined();
    // A Trap's play is its set, and makes no entrance either.
    expect(plan([play("trap-mythic")])[0]?.entrance).toBeUndefined();
  });

  it("R2722001 one hold per action, pinned, speed-scaled, none reduced or with the effects off", () => {
    const two = plan([play("spell-mythic", "a"), play("spell-mythic", "b")]);
    expect(holds(two)).toEqual([TAKEOVER_HOLD_MS, 0]);
    const seen = new Set<string>();
    plan([play("spell-mythic", "a")], p1, { seen });
    // The same card again (its summoned after a prompt) makes no second entrance; the next action's Mythic holds again.
    expect(plan([summon("spell-mythic", "a")], p1, { seen })[0]?.entrance).toBeUndefined();
    expect(holds(plan([play("spell-mythic", "c")], p1, { seen }))).toEqual([TAKEOVER_HOLD_MS]);
    // A card bounced to its hand keeps its id (only a set takes a fresh one, R227): played again, it enters again.
    expect(holds(plan([play("unit-mythic", "u"), summon("unit-mythic", "u")], p1, { seen }))).toEqual([TAKEOVER_HOLD_MS]);
    expect(holds(plan([play("unit-mythic", "u"), summon("unit-mythic", "u")], p1, { seen }))).toEqual([TAKEOVER_HOLD_MS]);
    expect(holds(plan([play("spell-mythic")], p1, { speed: 2 }))).toEqual([scaleForSpeed(TAKEOVER_HOLD_MS, 2)]);
    expect(holds(plan([play("spell-mythic")], p1, { reduced: true }))).toEqual([0]);

    // The queue: the held entry keeps its whole time against the burst budget, and the effects off take none.
    const waits = (intensity: "normal" | "off"): number[] => {
      const out: number[] = [];
      const queue = createAnimationQueue({
        reducedMotion: false,
        burstBudgetMs: 900,
        schedule: (fn, ms) => {
          out.push(ms);
          fn();
        },
        settings: () => ({ speed: 1, motion: "system", intensity }),
        entranceOf: resolve,
      });
      queue.enqueue([play("spell-mythic"), { type: "turnEnded", player: "p1", turn: 3, unspentMana: 0 }], p1);
      return out;
    };
    expect(waits("normal")[0]).toBe(ANIMATIONS.cardPlayed.durationMs + TAKEOVER_HOLD_MS);
    expect(waits("off")[0]).toBeLessThanOrEqual(ANIMATIONS.cardPlayed.durationMs);
  });

  it("R2722001 the queue holds the entry TAKEOVER_HOLD_MS longer, and a reset forgets the cards it entered", () => {
    const waits: number[] = [];
    const queue = createAnimationQueue({
      reducedMotion: false,
      schedule: (_fn, ms) => {
        waits.push(ms);
      },
      settings: () => ({ speed: 1, motion: "system", intensity: "normal" }),
      entranceOf: resolve,
    });
    queue.enqueue([play("unit-mythic"), summon("unit-mythic")], p1);
    expect(queue.inFlight()?.entrance?.holdMs).toBe(TAKEOVER_HOLD_MS);
    expect(waits).toEqual([ANIMATIONS.cardPlayed.durationMs + TAKEOVER_HOLD_MS]);
    queue.reset();
    queue.enqueue([play("unit-mythic"), summon("unit-mythic")], p1);
    expect(queue.inFlight()?.entrance?.holdMs).toBe(TAKEOVER_HOLD_MS);
  });
});

describe("R2722002 a hidden card's entrance at its reveal", () => {
  it("R2722002 a Mythic Trap's first firing and a Cloaked Mythic's reveal take it on each seat that reads them, anchored on the flipped zone", () => {
    // A Trap: its controller reads its first firing, the other seat the sentinel; a firing again is no reveal.
    const [trap] = plan([fired("trap-mythic", "b1")]);
    expect(trap?.entrance).toMatchObject({ tier: "mythic", reveal: true, holdMs: TAKEOVER_HOLD_MS, anchor: "zone-you-backrow-3" });
    expect(plan([fired("hidden", "hidden")], p2)[0]?.entrance).toBeUndefined();
    expect(plan([fired("trap-mythic", "b1", true)])[0]?.entrance).toBeUndefined();
    expect(plan([fired("trap-legend", "b2")])[0]?.entrance).toMatchObject({ tier: "legendary", holdMs: 0 });

    // A Cloaked Unit, Spell and Field Spell: `activated` names it on both seats, then its firing.
    for (const defId of ["unit-mythic", "spell-mythic", "field-mythic"]) {
      const own = plan([revealed(defId), fired(defId, "c61")]);
      expect(own[0]?.entrance, `${defId} on its controller's seat, at the activated`).toBeUndefined();
      expect(own[1]?.entrance, `${defId} on its controller's seat`).toMatchObject({ tier: "mythic", reveal: true, holdMs: TAKEOVER_HOLD_MS, anchor: "zone-you-backrow-3" });
      const theirs = plan([revealed(defId), fired("hidden", "hidden")], p2);
      expect(theirs[0]?.entrance, `${defId} on the other seat`).toMatchObject({ tier: "mythic", reveal: true, holdMs: TAKEOVER_HOLD_MS, anchor: "zone-opponent-backrow-3" });
      expect(theirs[1]?.entrance).toBeUndefined();
    }
  });
});
