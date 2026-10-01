// C #7 InfiniScepter (SPEC §8.6 row 7). Field Spell, cost 1, Legendary.
//   Both faces: "Cry: Exile a ({costLimit}) Cost or less Spell from your hand.
//                Activate: Cast a copy of that Spell." — costLimit 1 on the base face, 2 on the Radiant.
//
// The Cry (a Field Spell's Cry, as #73 Anti-oneshot Armor's) declares a hand pick (R81): a Spell — the
// Spell type — in your hand whose cost now (R65, a hand card at its hand cost; an X Spell 0) is
// ({costLimit}) or less, a number no declaration field can carry once Degrade or Upgrade moves it, so
// it is a `targetChecks` rule (§10.6). It exiles that card and remembers its definition and Radiant
// flag on the instance (`memory.scepter`). With no such Spell the play is still legal (R90) and the
// Cry does nothing, so nothing is remembered and the card can never activate. The exiled card stays
// in exile; the Radiant face keeps "from your hand", since the Cry has nowhere else to look.
//
// Activate (R384), once per turn: cast a fresh copy of the remembered Spell (§6.3 Cast, B5 E12,
// `castNew`): free, counted as a play (R70) — the activation itself is not one — with its radiant flag
// kept, its targets and modes chosen by you in prompts as the cast begins (R70, R81), the `activate`
// action carrying none; the copy goes to your graveyard after it resolves (R87). With nothing
// remembered the ability can't be activated (`canActivate`). Leaving the field clears the memory
// (R78), so a replayed InfiniScepter needs a new Cry.
//
// R520: a remembered X-cost Spell is cast with the X its caster chooses as the cast begins, from 1 to
// their current mana (R348); the cast pays nothing, so the X is not paid.

import type { ConditionContext, EffectContext, Script, TargetCheck } from "@jackioh/engine";
import { defOf, effectiveCost, param, recalled } from "@jackioh/engine";
import { castNew, exile, instanceOf, remember } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-007");

/** Where the instance keeps the Spell it exiled. */
const SCEPTER = "scepter";
/** The name of the Cry's hand-pick rule in `targetChecks`. */
const WITHIN_LIMIT = "spellWithinLimit";

type Held = { defId: string; radiant: boolean };

function heldSpell(ctx: Pick<EffectContext, "self" | "data">): Held | null {
  const held = recalled(ctx, SCEPTER);
  if (held === null || typeof held !== "object") return null;
  const { defId, radiant } = held as Partial<Held>;
  return typeof defId === "string" ? { defId, radiant: radiant === true } : null;
}

/** "A ({costLimit}) Cost or less Spell from your hand", at the cost it would be played for now (R65). */
const withinLimit: TargetCheck = ({ state, self, radiant, candidate }) =>
  candidate !== null &&
  candidate.zone.z === "hand" &&
  defOf(state, candidate.defId).type === "Spell" &&
  effectiveCost(state, candidate) <= param({ state, self, radiant }, "costLimit");

function cry(ctx: EffectContext) {
  const picked = instanceOf(ctx, { of: "chosen" });
  if (picked === null) return [];
  return [
    remember({ key: SCEPTER, value: { defId: picked.defId, radiant: picked.radiant } }),
    exile({ target: { of: "chosen" } }),
  ];
}

export const base: Script = {
  targets: [{ kind: "hand", min: 1, max: 1, filter: { of: ["hand"], type: "Spell", check: WITHIN_LIMIT } }],
  targetChecks: { [WITHIN_LIMIT]: withinLimit },
  cry,
  activations: [
    {
      id: "cast-copy",
      label: "Cast a copy of that Spell",
      uses: 1,
      canActivate: (ctx: ConditionContext) => heldSpell({ self: ctx.self, data: {} }) !== null,
      run: () => [castNew({ def: (ctx) => heldSpell(ctx) })],
    },
  ],
};

// The same script: the Radiant face's (2) is its declared costLimit, which `param` reads off the face.
export const radiant: Script = base;
