// T-AI-1 Helpful Assistant — SPEC §8.7 row T-AI-1, §10.8, R60, R97, R177, BUILD M9 row T-AI-1.

import { drawsThisTurn, effectiveCost, hashState, reduce, type GameState } from "@jackioh/engine";
import type { Action } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { def } from "../../src/scripts/classic-plus/t-ai-01-helpful-assistant";
import { scenario, type Scenario } from "../_harness";

const ASSISTANT = "classicplus-t-ai-01";
const FILLER = "core-005";
const HINDER = "core-021"; // Cast on draw
const MENACE = "core-019";
const TIMMY = "core-011";
const RENO = "core-053";
const DECK = [MENACE, TIMMY, RENO, HINDER, FILLER];

function assistant(opts: { radiant?: boolean; library?: readonly string[]; hand?: readonly string[] } = {}): Scenario {
  return scenario({
    p1: { hand: [{ def: ASSISTANT, radiant: opts.radiant === true }, ...(opts.hand ?? [FILLER])], library: opts.library ?? DECK },
    p2: { hand: [FILLER] },
  });
}

/** The library cards the open Discover offers, by def id. */
function offered(s: Scenario): string[] {
  return (s.state.pending?.options ?? []).map((option) =>
    option.selection.pick === "instance" ? s.card(option.selection.instanceId).defId : "",
  );
}

describe("T-AI-1 Helpful Assistant", () => {
  it("is a 1/3 Taunt, Radiant 2/6 Taunt and Divine Shield", () => {
    expect(def.id).toBe(ASSISTANT);
    const s = scenario({ p1: { field: [ASSISTANT, { def: ASSISTANT, radiant: true }] } });
    s.expectStats(s.unit("p1", 1) ?? ASSISTANT, { attack: 1, health: 3 });
    expect(s.stats(s.unit("p1", 1) ?? ASSISTANT).keywords.map((k) => k.kind)).toEqual(["Taunt"]);
    s.expectStats(s.unit("p1", 2) ?? ASSISTANT, { attack: 2, health: 6 });
    expect(s.stats(s.unit("p1", 2) ?? ASSISTANT).keywords.map((k) => k.kind)).toEqual(["Taunt", "Divine Shield"]);
  });

  describe("base", () => {
    it("R60 Cry: a Discover of 3 different cards of your own deck, shown to you; the chosen one moves to your hand", () => {
      const s = assistant().play(ASSISTANT);
      expect(s.state.pending?.kind).toBe("discover");
      expect(s.state.pending?.playerId).toBe("p1");
      expect(new Set(offered(s)).size).toBe(3);
      for (const id of offered(s)) expect(DECK).toContain(id);
      const pick = s.state.pending?.options[0]?.selection;
      const chosen = pick?.pick === "instance" ? pick.instanceId : "";
      const rest = s.pile("p1", "library").filter((card) => card.id !== chosen).map((card) => card.id);
      s.answer(chosen);
      s.expectInZone(chosen, "hand");
      // The rest stay put, in the order they lay.
      expect(s.pile("p1", "library").map((card) => card.id)).toEqual(rest);
      expect(s.card(chosen).costMod).toBe(0);
    });

    it("all of a deck shorter than three are offered; an empty deck asks nothing", () => {
      const short = assistant({ library: [MENACE, TIMMY] }).play(ASSISTANT);
      expect(offered(short).sort()).toEqual([MENACE, TIMMY].sort());
      const empty = assistant({ library: [] }).play(ASSISTANT);
      expect(empty.state.pending).toBeNull();
      expect(empty.events.some((event) => event.type === "fatigue")).toBe(false);
    });

    it("R177 the options come in a shuffled order, never the deck's", () => {
      const orders = new Set<string>();
      for (let n = 0; n < 12; n += 1) {
        const s = scenario({ seed: `assistant-order-${n}`, p1: { hand: [ASSISTANT], library: [MENACE, TIMMY, RENO] } }).play(ASSISTANT);
        orders.add(offered(s).join(","));
      }
      expect(orders.size).toBeGreaterThan(1);
    });

    it("§2.4 it is not a draw: a Cast on draw card reaches the hand uncast, and no draw is counted", () => {
      const s = assistant({ library: [HINDER] }).play(ASSISTANT);
      const draws = drawsThisTurn(s.state, "p1");
      s.answer(s.card(HINDER).id);
      expect(drawsThisTurn(s.state, "p1")).toBe(draws);
      s.expectInZone(HINDER, "hand");
      expect(s.events.some((event) => event.type === "cardPlayed" && event.defId === HINDER)).toBe(false);
    });

    it("§2.4 a full hand burns the chosen card", () => {
      const s = assistant({ library: [MENACE], hand: Array.from({ length: 10 }, () => FILLER) }).play(ASSISTANT);
      s.answer(s.card(MENACE).id);
      s.expectInZone(MENACE, "graveyard");
      s.expectEvents("burned");
    });

    it("§10.8 R97 R177 the opponent sees only that a prompt is open, then a card reaching your hand under the sentinel", () => {
      const s = assistant().play(ASSISTANT);
      expect(s.view("p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
      const seen = JSON.stringify(s.view("p2"));
      for (const id of [MENACE, TIMMY, RENO, HINDER]) expect(seen).not.toContain(id);
      const pick = s.state.pending?.options[0]?.selection;
      s.answer(pick?.pick === "instance" ? pick.instanceId : "");
      const added = s.view("p2").events.filter((event) => event.type === "addedToHand" && event.player === "p1");
      expect(added).toHaveLength(1);
      expect(added[0]?.type === "addedToHand" ? added[0].defId : "").toBe("hidden");
    });

    it("§9.3 paused on the Discover, the state survives JSON and answers to the same hash", () => {
      const s = assistant().play(ASSISTANT);
      const thawed = JSON.parse(JSON.stringify(s.state)) as GameState;
      const pick = s.state.pending?.options[1]?.selection;
      const action = { type: "answer", playerId: "p1", choiceId: s.state.pending?.id ?? "", selection: [pick], nonce: "assistant-json" } as Action;
      const live = reduce(s.state, action);
      const frozen = reduce(thawed, action);
      expect(live.error).toBeUndefined();
      expect(hashState(frozen.state)).toBe(hashState(live.state));
    });
  });

  describe("radiant", () => {
    it("the chosen card costs (1) less, a costMod that stacks", () => {
      const s = assistant({ radiant: true, library: [MENACE] }).play(ASSISTANT);
      const before = effectiveCost(s.state, s.card(MENACE));
      s.answer(s.card(MENACE).id);
      expect(s.card(MENACE).costMod).toBe(-1);
      expect(effectiveCost(s.state, s.card(MENACE))).toBe(before - 1);
    });

    it("an empty deck asks nothing", () => {
      const s = assistant({ radiant: true, library: [] }).play(ASSISTANT);
      expect(s.state.pending).toBeNull();
    });
  });
});
