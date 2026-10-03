// C #81 The Power to Thrive (SPEC §8.6 row 81). (2) Field Spell, Rare.
//   Base:    "Activate: Choose one: Heal your hero {heal}; draw {draw}; or gain {mana} mana." — 3, 1, 1
//   Radiant: the same text — 6, 2, 2
//   Engine:  "Activate (§6.2, R384), once per turn, the mode declared in the action (R81); the mana is
//            temporary (§2.3) and may exceed 4. "Active" is Activate. Tunes: heal 3 ↑; draw 1 ↑; mana 1 ↑."
//
// One ability, once per turn, its mode carried in the `activate` action. Heal has no cap on a hero (R19).
// The three numbers are declared (R386).

import { param, type Script } from "@jackioh/engine";
import { draw, gainMana, heal } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-081");

const MODES = ["heal", "draw", "mana"] as const;

export const base: Script = {
  activations: [
    {
      id: "thrive",
      label: "Choose one",
      uses: 1,
      modes: [{ kind: "mode", options: [...MODES] }],
      run: (ctx) => {
        const mode = ctx.modes[0];
        if (mode === "heal") return [heal({ target: { of: "selfHero" }, amount: param(ctx, "heal") })];
        if (mode === "draw") return [draw({ count: param(ctx, "draw") })];
        return mode === "mana" ? [gainMana({ amount: param(ctx, "mana") })] : [];
      },
    },
  ],
};

// The same script: the Radiant face's 6, 2 and 2 are its declared numbers.
export const radiant: Script = base;
