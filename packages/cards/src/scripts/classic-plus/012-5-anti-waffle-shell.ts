// C+ #12.5 Anti-Waffle Shell (SPEC §8.7 row 12.5): (1) Field Spell, Pancake, Token (printed Legendary).
//   Both faces: "Cry: Give your Units Divine Shield. Aura: Your Units have +{aura}/+{aura}." — 2, Radiant 4.
// The Cry grants Divine Shield (a granted keyword, §10.4) to each Unit you control then; the aura covers
// every Unit you control while this is on the field, later arrivals and a carried Unit included.

import type { Script } from "@jackioh/engine";
import { activeUnitsOf, param } from "@jackioh/engine";
import { forEachCard, grantKeyword } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-012-5");

export const base: Script = {
  cry: () => [
    forEachCard({
      cards: (ctx) => activeUnitsOf(ctx.state, ctx.controller),
      each: (instanceId) => grantKeyword({ target: { of: "instance", instanceId }, keyword: { kind: "Divine Shield" } }),
    }),
  ],
  aura: ({ state, self, radiant }) => {
    const n = param({ state, self, radiant }, "aura");
    const yours = new Set(activeUnitsOf(state, self.controller).map((unit) => unit.id));
    return [{ applies: (unit) => yours.has(unit.id), mod: { attack: n, maxHealth: n } }];
  },
};

/** The Radiant face is the same text at +4/+4, a catalog value. */
export const radiant: Script = base;
