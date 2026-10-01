// C+ #12.2 Death Boil (SPEC §8.7 row 12.2): (1) Spell, Pancake, Token (printed Legendary).
//   Both faces: "Choose a Unit or hero. If it's an enemy, deal {amount} damage to it. If it's yours,
//   heal it {amount}." — 6, Radiant 12.
// Which clause applies is read at resolution from who controls the target; the damage is one §4.4
// instance (Spell Damage raises it), the heal a Heal (a hero past 30, a unit up to its max, R19).

import type { TargetDecl } from "@jackioh/shared";
import type { EffectContext, Script } from "@jackioh/engine";
import { param } from "@jackioh/engine";
import { damage, heal, resolveTarget } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-012-2");

const targets: TargetDecl[] = [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }];

function isYours(ctx: EffectContext): boolean | null {
  const target = resolveTarget(ctx, { of: "chosen" });
  if (target === null) return null;
  return (target.kind === "hero" ? target.player : target.instance.controller) === ctx.controller;
}

export const base: Script = {
  targets,
  cry: (ctx) => {
    const amount = param(ctx, "amount");
    const yours = isYours(ctx);
    if (yours === null) return [];
    return [yours ? heal({ target: { of: "chosen" }, amount }) : damage({ to: { of: "chosen" }, amount })];
  },
};

/** The Radiant face is the same text at 12, a catalog value. */
export const radiant: Script = base;
