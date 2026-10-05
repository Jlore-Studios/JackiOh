// C #88 Siphon Squad (SPEC §8.6 row 88). (2) Field Trap, Rare.
//   Base:    "Start of Turn: Reveal.
//            Aura: Enemy Units have −X Attack, where X is {multiplier}× the number of Units your opponent
//            controls.
//            When your opponent controls no Units, Tribute this." — ×2
//   Radiant: "Start of Turn: Reveal.
//            Aura: Enemy Units have 0 Attack.
//            When your opponent controls no Units, Tribute this."
//   Engine:  "A layer-5 aura (§10.4), attack floored at 0; the Radiant's "0 attack" sets attack last, after
//            every other layer. The self-Tribute is a condition checked at every state check, the one right
//            after it is set included. … live while face-down … (R403). The base face's `preview` (R280)
//            shows X, to its controller only while it is face-down (§10.8). Tunes: multiplier 2 ↑."
//
// R403: the aura and the self-Tribute (`tributeWhen`, read at every state check) work from the moment it
// is set, face-down. The preview is X off the public unit count; `viewFor` shows a face-down card's to
// its controller alone. Its proofs are in `test/preview.test.ts`.

import { activeUnitsOf, param, type CardInstance, type GameState, type Script, type StatMod } from "@jackioh/engine";
import { opponentOf, type PreviewValue } from "@jackioh/shared";
import { reveal } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-088");

type Read = { state: GameState; self: CardInstance; radiant: boolean };

const enemyUnits = ({ state, self }: Read): number => activeUnitsOf(state, opponentOf(self.controller)).length;

/** X: {multiplier} times the Units the opponent controls now. */
const xNow = (read: Read): number => param(read, "multiplier") * enemyUnits(read);

function siphon(mod: (read: Read) => StatMod, preview: (read: Read) => PreviewValue[]): Script {
  return {
    // "Start of Turn: Reveal" (balance patch 1, R666): at its controller's start of turn the card
    // shows its face to both players. The aura keeps working — revealed is not face-up, and a Field
    // Trap fires face-down or up alike.
    startOfTurn: () => [reveal()],
    aura: (read) => [{ applies: (unit) => unit.zone.z === "field" && unit.controller !== read.self.controller, mod: mod(read) }],
    tributeWhen: (read) => enemyUnits(read) === 0,
    preview,
  };
}

export const base: Script = siphon(
  (read) => ({ attack: -xNow(read) }),
  (read) => [{ label: "−X Attack", value: xNow(read) }],
);

// The Radiant face has no X: its preview is empty, which is no preview (R280).
export const radiant: Script = siphon(() => ({ setAttack: 0 }), () => []);
