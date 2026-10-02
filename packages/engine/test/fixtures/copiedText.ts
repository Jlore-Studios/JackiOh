// Fixture cards for copying the last Spell's text (docs/classic-sets.md B5 E14; Classic #57 Echo; R399,
// R545–R547). The engine never imports `packages/cards` (CLAUDE.md), so the copier and the Spells it
// copies are proved on cards of their shape here. Ids are `ct-*`, indices from 5900, registered on top
// of the shared fixture catalog by `withCopiedText`.

import type { CardDef, CardType, Param } from "@jackioh/shared";
import { registerCatalog, registeredCatalog } from "../../src/catalog";
import { castNew, chooseTarget, counterPlay, damage, heal } from "../../src/effects";
import { param } from "../../src/params";
import type { CardScripts, Script } from "../../src/script";
import { registerScripts, registeredScripts } from "../../src/scripts";
import type { GameState } from "../../src/state";
import { copiedTextOf } from "../../src/subsystems/copiedText";

let nextIndex = 5900;

function def(id: string, type: CardType, extra: Partial<CardDef> = {}): CardDef {
  nextIndex += 1;
  return {
    id: `ct-${id}`,
    index: String(nextIndex),
    name: `${id} (copied text)`,
    set: "Core",
    type,
    tags: [],
    rarity: "Common",
    token: false,
    cost: 1,
    base: { keywords: [], text: id },
    radiant: { keywords: [], text: `${id} radiant` },
    ...extra,
  };
}

function both(script: Script): CardScripts {
  return { base: script, radiant: script };
}

const DAMAGE: Param[] = [{ key: "damage", base: 2, radiant: 4, better: "up" }];
const ASK_DAMAGE: Param[] = [{ key: "damage", base: 3, radiant: 6, better: "up" }];
const ANY_TARGET = [{ kind: "target" as const, min: 1, max: 1, filter: { of: ["unit" as const, "hero" as const] } }];

export const CT = {
  /** Classic #57 Echo's shape: has the last Spell's text; its Radiant face adds Echo 1. */
  echo: def("echo", "Spell"),
  /** A Spell that deals {damage} (2, Radiant 4) to a declared unit or hero. */
  bolt: def("bolt", "Spell", { params: DAMAGE }),
  /** A Spell with no choices: 1 damage to the enemy hero (Radiant 3). */
  ping: def("ping", "Spell"),
  /** A Spell whose script asks for an enemy unit, then deals {damage} (3, Radiant 6) to it. */
  asker: def("asker", "Spell", { params: ASK_DAMAGE }),
  /** An X-cost Spell: X damage to the enemy hero. */
  xBolt: def("x-bolt", "Spell", { cost: "X" }),
  /** "Choose one": 2 damage to the enemy hero, or heal your hero 2. */
  modal: def("modal", "Spell"),
  /** A Spell with Echo 1: 1 damage to the enemy hero, twice. */
  echoSpell: def("echo-spell", "Spell"),
  /** Cast on draw: 1 damage to the enemy hero. */
  drawCast: def("draw-cast", "Spell"),
  /** A Field Spell: never a Spell for the record. */
  field: def("field", "Field Spell"),
  /** A Spell whose script casts a ping, then deals 5 to the enemy hero: a newer Spell mid-resolution. */
  caster: def("caster", "Spell"),
  /** A Spell with a preview and a yellow glow, both reading its declared number. */
  glow: def("glow", "Spell", { params: DAMAGE }),
  /** A Trap that counters any play of the opponent's. */
  counter: def("counter", "Trap"),
  /** An embiggen Spell: 1 damage to the enemy hero at its base price, 5 at its embiggen price. */
  bigger: def("bigger", "Spell", { cost: { base: 1, embiggen: 3 } }),
  /** A plain Unit to target. */
  dummy: def("dummy", "Unit", { base: { attack: 1, health: 9, keywords: [], text: "dummy" }, radiant: { attack: 2, health: 18, keywords: [], text: "dummy radiant" } }),
} as const;

const echoBase: Script = {
  staticFlags: { copiesLastSpell: true },
  recordsPlayAs: ({ state, self }) => copiedTextOf(state, self),
};

const SCRIPTS: Record<string, CardScripts> = {
  [CT.echo.id]: { base: echoBase, radiant: { ...echoBase, staticFlags: { copiesLastSpell: true, echo: 1 } } },
  [CT.bolt.id]: both({ targets: ANY_TARGET, cry: (ctx) => [damage({ to: { of: "chosen" }, amount: param(ctx, "damage") })] }),
  [CT.ping.id]: {
    base: { cry: () => [damage({ to: { of: "enemyHero" }, amount: 1 })] },
    radiant: { cry: () => [damage({ to: { of: "enemyHero" }, amount: 3 })] },
  },
  [CT.asker.id]: both({
    cry: () => [chooseTarget({ step: "hit", scope: { side: "enemy", of: ["unit"] } })],
    resume: { hit: (ctx) => [damage({ to: { of: "chosen" }, amount: param(ctx, "damage") })] },
  }),
  [CT.xBolt.id]: both({ cry: (ctx) => [damage({ to: { of: "enemyHero" }, amount: ctx.x })] }),
  [CT.modal.id]: both({
    modes: [{ kind: "mode", options: ["hit", "heal"] }],
    cry: (ctx) =>
      ctx.modes[0] === "heal"
        ? [heal({ target: { of: "selfHero" }, amount: 2 })]
        : [damage({ to: { of: "enemyHero" }, amount: 2 })],
  }),
  [CT.echoSpell.id]: both({ staticFlags: { echo: 1 }, cry: () => [damage({ to: { of: "enemyHero" }, amount: 1 })] }),
  [CT.drawCast.id]: both({ staticFlags: { castOnDraw: true }, cry: () => [damage({ to: { of: "enemyHero" }, amount: 1 })] }),
  [CT.caster.id]: both({ cry: () => [castNew({ def: CT.ping.id }), damage({ to: { of: "enemyHero" }, amount: 5 })] }),
  [CT.glow.id]: both({
    targets: ANY_TARGET,
    cry: (ctx) => [damage({ to: { of: "chosen" }, amount: param(ctx, "damage") })],
    conditionMet: (ctx) => param(ctx, "damage") >= 4,
    preview: (ctx) => [{ label: "damage", value: param(ctx, "damage") }],
  }),
  [CT.counter.id]: both({
    triggers: [
      {
        id: "counter",
        on: ["cardAnnounced"],
        when: (ctx) => ctx.event.type === "cardAnnounced" && ctx.event.player !== ctx.controller,
        run: (ctx) =>
          ctx.event.type === "cardAnnounced"
            ? [counterPlay({ target: { of: "instance", instanceId: ctx.event.instanceId } })]
            : [],
      },
    ],
  }),
  [CT.bigger.id]: both({ cry: (ctx) => [damage({ to: { of: "enemyHero" }, amount: ctx.embiggened ? 5 : 1 })] }),
  [CT.dummy.id]: both({}),
};

export const CT_DEFS: readonly CardDef[] = Object.values(CT);

/** Register this file's defs and scripts on top of whatever is registered now. */
export function registerCopiedText(): void {
  registerCatalog({ ...registeredCatalog(), ...Object.fromEntries(CT_DEFS.map((card) => [card.id, card])) });
  registerScripts({ ...registeredScripts(), ...SCRIPTS });
}

/** `registerCopiedText` for a game `newGame` has just made (its catalog is registered by then). */
export function withCopiedText(state: GameState): GameState {
  registerCopiedText();
  return state;
}
