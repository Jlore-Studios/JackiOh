// C+ #12.1 Devour (SPEC §8.7 row 12.1): (0) Spell, Pancake, Token (printed Legendary).
//   Base:    "Destroy a Unit. Your hero takes damage equal to its health."
//   Radiant: "Destroy a Unit. Heal your hero by its health."
// A target Unit on either side; its current health is read before the destroy. The self-hit is one
// instance from Devour through §4.4 (your Armor, caps and Spell Damage apply); the gain is a Heal. An
// Indestructible target survives (R46) and the damage or heal still happens (Hearthstone's Obliterate).

import type { TargetDecl } from "@jackioh/shared";
import type { EffectContext, Script } from "@jackioh/engine";
import { unitView } from "@jackioh/engine";
import { damage, destroy, heal, instanceOf } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-012-1");

const targets: TargetDecl[] = [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit"] } }];

/** The chosen Unit's health now, before the destroy marks it; 0 when it is gone (nothing then). */
function healthOfChosen(ctx: EffectContext): number {
  const unit = instanceOf(ctx, { of: "chosen" });
  return unit === null ? 0 : Math.max(0, unitView(ctx.state, unit).health);
}

export const base: Script = {
  targets,
  cry: (ctx) => [destroy({ target: { of: "chosen" } }), damage({ to: { of: "selfHero" }, amount: healthOfChosen(ctx) })],
};

export const radiant: Script = {
  targets,
  cry: (ctx) => {
    const health = healthOfChosen(ctx);
    return [destroy({ target: { of: "chosen" } }), ...(health > 0 ? [heal({ target: { of: "selfHero" }, amount: health })] : [])];
  },
};
