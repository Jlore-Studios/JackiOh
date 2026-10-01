// T-AI-10 Fine-Tuning — SPEC §8.7 row T-AI-10, BUILD M9 row T-AI-10: a Field Spell that, at each end of
// its controller's turn, Upgrades one random card in their hand (R386: one draw; an Immutable card or one
// nothing fits unchanged); an empty hand, nothing and no random draw (R129); the `upgraded` event names
// nothing to the opponent (R97); nothing at the opponent's end; radiant 2 different random cards.

import { describe, expect, it } from "vitest";
import { HIDDEN_ID, applyEffects, createRng, makeContext, type EngineSink } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { base, def, radiant } from "../../src/scripts/classic-plus/t-ai-10-fine-tuning";
import { scenario, type Scenario, type SideSetup } from "../_harness";

const TUNING = "classicplus-t-ai-10";
const UNIT = "core-008";
const MENACE = "core-019"; // Radiant: Immutable.
const FILLER = "core-005";

type Upgraded = Extract<GameEvent, { type: "upgraded" }>;

function upgrades(events: readonly GameEvent[]): Upgraded[] {
  return events.filter((event): event is Upgraded => event.type === "upgraded");
}

function fineTuning(hand: NonNullable<SideSetup["hand"]>, opts: { radiant?: boolean; seed?: string } = {}): Scenario {
  return scenario({
    ...(opts.seed === undefined ? {} : { seed: opts.seed }),
    p1: { hand, backrow: [{ def: TUNING, radiant: opts.radiant === true }], field: [UNIT] },
    p2: { hand: [FILLER, FILLER], field: [UNIT] },
  });
}

describe("T-AI-10 Fine-Tuning", () => {
  it("is a (2) Field Spell, AI, Token, with an end-of-turn hook on each face", () => {
    expect(def).toMatchObject({ cost: 2, type: "Field Spell", tags: ["AI", "Token"], token: true });
    expect(base.endOfTurn).toBeDefined();
    expect(radiant.endOfTurn).toBeDefined();
    expect(base.startOfTurn).toBeUndefined();
  });

  describe("base", () => {
    it("R62 R386 at the end of your turn it Upgrades one card in your hand", () => {
      const s = fineTuning([UNIT, FILLER, FILLER]);
      const hand = s.hand("p1").map((card) => card.id);
      s.endTurn();
      const events = upgrades(s.lastEvents);
      expect(events).toHaveLength(1);
      expect(hand).toContain(events[0]!.instanceId);
      expect(events[0]!.change.kind).not.toBe("none");
    });

    it("R60 the card is a random one of the hand, the seed's", () => {
      const seen = new Set<string>();
      for (let n = 0; n < 16; n += 1) {
        const s = fineTuning([UNIT, FILLER, "core-047"], { seed: `fine-tuning-${n}` });
        s.endTurn();
        const [event] = upgrades(s.lastEvents);
        seen.add(s.card(event!.instanceId).defId);
      }
      expect(seen.size).toBe(3);
    });

    it("nothing at the opponent's end of turn, nor at the start of yours", () => {
      const s = fineTuning([UNIT, FILLER]);
      s.endTurn();
      expect(s.state.active).toBe("p2");
      s.endTurn();
      expect(s.state.active).toBe("p1");
      // p2's end and p1's start: no Upgrade.
      expect(upgrades(s.lastEvents)).toEqual([]);
    });

    it("R386 R440 an Immutable card is unchanged, cued with the change none", () => {
      const s = fineTuning([{ def: MENACE, radiant: true }]);
      const menace = s.hand("p1")[0]!;
      s.endTurn();
      const events = upgrades(s.lastEvents);
      expect(events).toHaveLength(1);
      expect(events[0]!.change).toEqual({ kind: "none" });
      expect(s.card(menace).tuning).toBeUndefined();
      expect(s.card(menace).costMod).toBe(0);
    });

    it("R97 the upgraded event names nothing to the opponent", () => {
      const s = fineTuning([UNIT, FILLER]);
      s.endTurn();
      const theirs = upgrades(s.view("p2").events);
      expect(theirs).toHaveLength(1);
      expect(theirs[0]).toMatchObject({ instanceId: HIDDEN_ID, defId: HIDDEN_ID });
      expect(upgrades(s.view("p1").events)[0]?.instanceId).not.toBe(HIDDEN_ID);
    });

    it("R129 an empty hand: nothing, and no random draw", () => {
      const s = fineTuning([]);
      const state = s.state;
      const sink: EngineSink = { state, events: [], rng: createRng(state.seed, state.rngCursor) };
      const ctx = makeContext(sink, s.backrow("p1", 1), { controller: "p1" });
      applyEffects(base.endOfTurn!(ctx), ctx);
      expect(sink.events).toEqual([]);
      expect(sink.rng.cursor).toBe(state.rngCursor);
    });
  });

  describe("radiant", () => {
    it("R60 Upgrades 2 different random cards in your hand", () => {
      for (let n = 0; n < 6; n += 1) {
        const s = fineTuning([UNIT, FILLER, "core-047"], { radiant: true, seed: `fine-tuning-r-${n}` });
        s.endTurn();
        const ids = upgrades(s.lastEvents).map((event) => event.instanceId);
        expect(ids).toHaveLength(2);
        expect(new Set(ids).size).toBe(2);
      }
    });

    it("R129 with one card in hand, that card only, with no draw for the pick", () => {
      const s = fineTuning([UNIT], { radiant: true });
      s.endTurn();
      expect(upgrades(s.lastEvents).map((event) => event.instanceId)).toEqual([s.pile("p1", "hand")[0]!.id]);
    });
  });
});
