// What the AI may know of patch v0.2.0's instance data (SPEC §9.9, R185; R385, R386, B5 E39): a
// hidden card's Brittle count, its tuning and its enchantments are the card's, so `redact` drops them
// with its face, and two true states that differ only in them redact to the same hash.

import { describe, expect, it } from "vitest";
import { hashState } from "@jackioh/engine";
import { hiddenInstanceIds, redact } from "../src/index";
import { AI, cardById, clone, scenario } from "./_support";

const P1 = { hand: ["core-008"], field: ["core-011"], library: ["core-020", "core-053"] } as const;
const P2 = { hand: ["core-002", "core-011"], field: ["core-019"], library: ["core-005", "core-016"] } as const;

describe("R185 the AI never reads a hidden card's instance data", () => {
  it("R185 a hidden card's Brittle count, tuning and enchantments go with its face", () => {
    const base = scenario({ seed: "observe-instance", p1: P1, p2: P2 }).state;
    const changed = clone(base);
    for (const card of [...changed.players.p2.hand, ...changed.players.p2.library]) {
      card.brittle = { count: 2, since: 1 };
      card.tuning = { attack: 3, numbers: { damage: 1 } };
      card.enchantments = [{ kind: "castOnDraw" }];
    }
    const hidden = hiddenInstanceIds(changed, AI);
    const pub = redact(changed, AI);
    for (const id of hidden) {
      const card = cardById(pub, id);
      expect(card?.tuning, id).toBeUndefined();
      expect(card?.brittle, id).toBeUndefined();
      expect(card?.enchantments, id).toBeUndefined();
    }
    expect(hashState(redact(changed, AI))).toBe(hashState(redact(base, AI)));
  });

  it("R185 a public card's instance data stays, since the seat reads it", () => {
    const state = scenario({ seed: "observe-instance-public", p1: P1, p2: P2 }).state;
    const unit = state.players.p2.units.flatMap((pile) => pile ?? [])[0];
    if (unit === undefined) throw new Error("expected a unit");
    unit.tuning = { attack: 1 };
    unit.brittle = { count: 1, since: 1 };
    const pub = redact(state, AI);
    expect(cardById(pub, unit.id)?.tuning).toEqual({ attack: 1 });
    expect(cardById(pub, unit.id)?.brittle).toEqual({ count: 1, since: 1 });
  });
});
