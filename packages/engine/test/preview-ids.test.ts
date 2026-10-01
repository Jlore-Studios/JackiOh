// R280's preview may count a set of cards (`PreviewValue.ids`): Classic+ #44 Simplicity Audit's and
// #45 Complexity Audit's Radiant "highlight targets" are the permanents the card would exile now
// (SPEC §8.7 rows 44 and 45). `previewOf` copies the ids beside the label and the value, as it copies
// `display` (R372), so the view holds its own array and never a reference into the hook's answer.
//
// The definitions are test-only, on top of the fixture catalog, put back in `afterAll` as
// preview.test.ts does.

import type { CardDef, CardView, PlayerView, PreviewValue } from "@jackioh/shared";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import type { CardScripts, ConditionContext } from "../src/script";
import { registerScripts, registeredScripts } from "../src/scripts";
import type { GameState } from "../src/state";
import { viewFor } from "../src/viewFor";
import { inHand, newGame } from "./fixtures/harness";

const marker: CardDef = {
  id: "pid-marker",
  index: "2901",
  name: "marker",
  set: "Core",
  type: "Spell",
  tags: [],
  rarity: "Common",
  token: false,
  cost: 1,
  base: { keywords: [], text: "marker" },
  radiant: { keywords: [], text: "marker" },
};

/** The ids the hook answers with, held so a test can mutate them after the view is built. */
let answered: string[] = [];
const hook = (_ctx: ConditionContext): PreviewValue[] => [
  { label: "marker", value: answered.length, ids: answered },
  { label: "plain", value: 1 },
];
const SCRIPTS: Record<string, CardScripts> = { [marker.id]: { base: { preview: hook }, radiant: { preview: hook } } };

let savedCatalog: ReturnType<typeof registeredCatalog> = {};
let savedScripts: ReturnType<typeof registeredScripts> = {};

beforeAll(() => {
  savedCatalog = registeredCatalog();
  savedScripts = registeredScripts();
});

afterAll(() => {
  registerCatalog(savedCatalog);
  registerScripts(savedScripts);
});

function game(seed: string): GameState {
  const state = newGame(seed);
  registerCatalog({ ...registeredCatalog(), [marker.id]: marker });
  registerScripts({ ...registeredScripts(), ...SCRIPTS });
  state.turn = 3;
  state.active = "p1";
  state.phase = "main";
  return state;
}

function handCard(view: PlayerView, instanceId: string): CardView {
  const hand = view.you.hand;
  if (!Array.isArray(hand)) throw new Error("the viewer's own hand travels in full");
  const found = hand.find((card) => card.instanceId === instanceId);
  if (found === undefined) throw new Error(`${instanceId} is not in the hand`);
  return found;
}

describe("R280 a preview value may carry the set of cards it counts", () => {
  it("R280 the view carries the ids beside the label and the value, and a value without ids carries none", () => {
    const state = game("preview-ids");
    const [card] = inHand(state, marker.id, "p1");
    if (card === undefined) throw new Error("the marker in hand");
    answered = ["c1", "c2"];

    const preview = handCard(viewFor(state, "p1"), card.id).preview;

    expect(preview).toEqual([
      { label: "marker", value: 2, ids: ["c1", "c2"] },
      { label: "plain", value: 1 },
    ]);
  });

  it("R280 the view holds a copy of the ids, never the hook's own array", () => {
    const state = game("preview-ids-copy");
    const [card] = inHand(state, marker.id, "p1");
    if (card === undefined) throw new Error("the marker in hand");
    answered = ["c7"];

    const preview = handCard(viewFor(state, "p1"), card.id).preview;
    answered.push("c8");

    expect(preview?.[0]?.ids).toEqual(["c7"]);
  });
});
