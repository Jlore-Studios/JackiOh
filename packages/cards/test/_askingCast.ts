// A cast-on-draw Spell that asks, for the tests that prove a prompt opened in the middle of a draw
// pauses it (§2.4, R58, R113, R158).
//
// Those tests used #21 Hinder, whose base face's "Discard 1" was a hand pick its caster answered as
// the cast began. Balance patch 1 made every discard random unless the card says "of your choice"
// (R682), so no catalog card casts on draw and asks any more. This fixture is that old Hinder base
// face without its mana clause: "Cast on draw: Discard a card of your choice." — a declared hand
// pick (R81) asked as the cast begins (R70), never the fixture itself (R90), fizzling on an empty hand.
//
// It is a fixture (a transient def on the state and a test-only script), never a catalog card, so it
// draws no rng and changes no random pool (R380).

import type { CardDef, PlayerId } from "@jackioh/shared";
import { newInstance, registerScripts, registeredScripts, type CardInstance, type Script } from "@jackioh/engine";
import { discard } from "@jackioh/engine/effects";
import type { Scenario } from "./_harness";

export const ASKING_CAST = "test-asking-cast-on-draw";

const SCRIPT: Script = {
  staticFlags: { castOnDraw: true },
  targets: [{ kind: "hand", min: 1, max: 1, filter: { of: ["hand"] } }],
  cry: () => [discard({ target: { of: "chosen" } })],
};

function register(s: Scenario): void {
  const face = { keywords: [], text: "Cast on draw: Discard a card of your choice." };
  const def: CardDef = {
    id: ASKING_CAST,
    index: ASKING_CAST,
    name: "Asking cast",
    set: "Core",
    type: "Spell",
    tags: [],
    rarity: "Common",
    token: false,
    cost: 0,
    base: { ...face },
    radiant: { ...face },
  };
  s.state.transientDefs[ASKING_CAST] = def;
  registerScripts({ ...registeredScripts(), [ASKING_CAST]: { base: SCRIPT, radiant: SCRIPT } });
}

/** Put an asking cast-on-draw Spell into `player`'s deck at `at` (0 is the top, drawn first). */
export function askingCastOnDraw(s: Scenario, player: PlayerId = "p1", at = 0): CardInstance {
  register(s);
  const card = newInstance(s.state, ASKING_CAST, player, { z: "library", player });
  s.state.players[player].library.splice(at, 0, card);
  return card;
}
