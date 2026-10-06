// Test-only cards for Activate (docs/classic-sets.md B3.2, R384). Each reproduces one shape of the
// Classic and Classic+ cards that use the keyword through the engine's own verbs: a targeted ping
// (Classic+ #76.1 Brother Ping), an "Activate 2" on a Unit, a ♾️ Tribute cost that reads the tributed
// unit's Attack (Classic #21 Turtinator), a random-discard cost (Classic #15 Nose Hunter), "Tribute
// this" on an Indestructible Field Spell (Classic #84 Lockdown), a mana price (Heroic Power's "spend
// (X)", the Heroic Power patch), modes with mode-bound targets and a delayed destroy (Classic #20 The Power to
// Punish), a condition the text sets (Classic #7 InfiniScepter), an ability that asks mid-list,
// abilities a card has only on some instances (the Heroic Power patch's rolled power), and abilities a
// fused card carries, each read in its ingredient's place (R102). The engine never imports
// `packages/cards`; the real cards' tests cover the same cases again.
//
// Ids are prefixed `act-` and indexed from 4100, so they cannot collide with another fixture file's.

import type { CardDef, CardDefs, Keyword } from "@jackioh/shared";
import {
  damage,
  destroyAtNextTurnStart,
  discardRandom,
  draw,
  gainMana,
} from "../../src/effects";
import { param } from "../../src/params";
import { openPrompt, resumeSelf } from "../../src/prompts";
import { recalled } from "../../src/query";
import type { ActivationDecl, CardScripts, Effect, EffectContext, Script } from "../../src/script";
import { activationPaid } from "../../src/subsystems/activate";
import { heroPower, powerAbilities, rollPower, POWER_RESUME, STEADY_SHOT_PARAM } from "../../src/subsystems/heroPower";
import type { GameState } from "../../src/state";

let nextIndex = 4100;

function def(
  name: string,
  type: CardDef["type"],
  extra: Partial<CardDef> & { attack?: number; health?: number; keywords?: Keyword[] } = {},
): CardDef {
  nextIndex += 1;
  const { attack = 2, health = 2, keywords = [], ...rest } = extra;
  const face =
    type === "Unit"
      ? { attack, health, keywords, text: name }
      : { keywords, text: name };
  const radiantFace = type === "Unit" ? { ...face, attack: attack * 2, health: health * 2 } : { ...face };
  return {
    id: `act-${name}`,
    index: String(nextIndex),
    name: `${name} (activate)`,
    set: "Core",
    type,
    tags: [],
    rarity: "Common",
    token: false,
    cost: 0,
    base: face,
    radiant: radiantFace,
    ...rest,
  };
}

// ---------------------------------------------------------------------------
// The note log: a Field Spell whose memory records what ran, in order.
// ---------------------------------------------------------------------------

export const logCard = def("log", "Field Spell");
/** The note log sits in p2's backrow lane 5, out of the way of every card under test. */
export const LOG_LANE = 5;

function logOf(state: GameState): { memory: Record<string, unknown> } | null {
  return state.players.p2.backrow[LOG_LANE - 1] ?? null;
}

export function notes(state: GameState): string[] {
  const log = logOf(state);
  return Array.isArray(log?.memory.steps) ? (log.memory.steps as string[]) : [];
}

export function note(entry: string): Effect {
  return {
    kind: "act:note",
    apply(ctx): void {
      const log = logOf(ctx.state);
      if (log === null) return;
      const steps = Array.isArray(log.memory.steps) ? (log.memory.steps as string[]) : [];
      log.memory.steps = [...steps, entry];
    },
  };
}

/** §10.6: a prompt for the controller with one answer, whose answer re-enters `step`. */
export function askController(step: string): Effect {
  return {
    kind: "act:ask",
    apply(ctx: EffectContext): void {
      openPrompt(ctx, {
        player: ctx.controller,
        kind: "target",
        prompt: "the ability asks its controller",
        options: [{ key: "none", label: "nothing", selection: { pick: "none" } }],
        resume: resumeSelf(ctx, step),
      });
    },
  };
}

function faces(base: Script, radiant: Script = base): CardScripts {
  return { base, radiant };
}

const ANY_TARGET = { kind: "target", min: 1, max: 1, filter: { of: ["unit", "hero"] } } as const;

// ---------------------------------------------------------------------------
// The cards
// ---------------------------------------------------------------------------

/** Classic+ #76.1's shape: "Activate: Deal 1 damage" (Radiant "Activate 2: Deal 2 damage"). */
export const pinger = def("pinger", "Field Spell");
function ping(uses: number, amount: number): ActivationDecl {
  return {
    id: "ping",
    label: `Deal ${amount} damage`,
    uses,
    targets: [{ ...ANY_TARGET, filter: { of: ["unit", "hero"] } }],
    run: () => [damage({ to: { of: "chosen" }, amount })],
  };
}

/** A Unit with "Activate 2: Gain 1 mana", for summoning sickness and "Activate N". */
export const sentry = def("sentry", "Unit", { attack: 2, health: 2 });

/** Classic #21's shape: "Activate ♾️: Tribute a Unit. Deal damage equal to its Attack to any target." */
export const turtle = def("turtle", "Unit", { attack: 5, health: 4 });

/** Classic #15's shape: "Activate: Discard a random card. …" */
export const nose = def("nose", "Unit", { attack: 3, health: 1 });

/** Classic #84's shape: Indestructible, "Activate: Tribute this." */
export const lockdown = def("lockdown", "Field Spell", { keywords: [{ kind: "Indestructible" }] });

/** Classic #89's shape: to target this, a player must also discard 2 cards (B5 E5, R450). */
export const ghost = def("ghost", "Unit", { attack: 5, health: 6 });

/** A mana price, the Heroic Power patch's "Activate: Spend (2): Draw 1". */
export const merchant = def("merchant", "Field Spell");
export const MERCHANT_PRICE = 2;

/**
 * Classic #20's shape: "Activate: Choose one: Deal 2 damage; your opponent discards a card; or choose
 * a Unit, which is destroyed at the start of your next turn" (Radiant: all enemy Units then).
 */
export const punisher = def("punisher", "Field Spell");
export const PUNISH_MODES = ["damage", "discard", "doom"] as const;
function punish(radiant: boolean): Script {
  return {
    activations: [
      {
        id: "punish",
        label: "Choose one",
        uses: 1,
        modes: [{ kind: "mode", options: [...PUNISH_MODES] }],
        targets: radiant
          ? [{ ...ANY_TARGET, filter: { of: ["unit", "hero"] }, forModes: ["damage"] }]
          : [
              { ...ANY_TARGET, filter: { of: ["unit", "hero"] }, forModes: ["damage"] },
              { kind: "target", min: 1, max: 1, filter: { of: ["unit"] }, forModes: ["doom"] },
            ],
        run: (ctx) => {
          switch (ctx.modes[0]) {
            case "damage":
              return [damage({ to: { of: "chosen" }, amount: 2 })];
            case "discard":
              return [discardRandom({ player: "enemy" })];
            case "doom":
              return radiant
                ? [destroyAtNextTurnStart({ scope: { side: "enemy" } })]
                : [destroyAtNextTurnStart({ target: { of: "chosen" } })];
            default:
              return [];
          }
        },
      },
    ],
  };
}

/** An ability that asks mid-list: `note(ask)`, a prompt, `note(ask:tail)`; the answer notes too. */
export const asker = def("asker", "Field Spell", { tags: ["Quickdraw"] });

/** Classic #7's shape: "Activate: Cast a copy of that Spell" — nothing remembered, no activation. */
export const scepter = def("scepter", "Field Spell");
export const SCEPTER_KEY = "stored";

/**
 * The Heroic Power patch's shape: one ability per power, and a copy has only the one it rolled
 * (`has`), beside one it always has. The rolled one is `memory.pick`.
 */
export const chooser = def("chooser", "Field Spell");
export const PICK_KEY = "pick";

/** A unit whose Death asks, so a Tribute cost that takes it pauses before the ability's effect. */
export const mourner = def("mourner", "Unit", { attack: 1, health: 1 });

/** A Trap with an ability: face-down in the backrow it has no text anyone can use. */
export const trapper = def("trapper", "Trap");

/** "Activate ♾️" with no cost, to reach the cap. */
export const endless = def("endless", "Field Spell");

/** Heroic Power wired as Core #98 is (R752), so `activate` and its alias reach the real power. */
export const heroic = def("heroic", "Field Spell", {
  cost: 0,
  tags: ["Quickdraw"],
  params: [{ key: STEADY_SHOT_PARAM, base: 2, radiant: 4, better: "up", step: 2, min: 1 }],
});

/**
 * Classic #7's shape read the way the real card reads it (R102): the ability is usable once its own
 * text has remembered something (`recalled`), and notes what it remembered, so a fusion shows which
 * ingredient's memory each of its abilities reads.
 */
export const keeper = def("keeper", "Field Spell");
export const KEEPER_KEY = "kept";

/** Classic #20's numbered shape (B3.4): "Activate: Deal {damage} damage", read through `param`. */
export const ZAPPER_DAMAGE = 3;
export const zapper = def("zapper", "Field Spell", {
  params: [{ key: "damage", base: ZAPPER_DAMAGE, radiant: ZAPPER_DAMAGE * 2, better: "up", step: 1, min: 1 }],
});
/** A card that declares the same number, smaller, and has no ability: fused ahead of a zapper (R102). */
export const spark = def("spark", "Field Spell", {
  params: [{ key: "damage", base: 1, radiant: 2, better: "up", step: 1, min: 1 }],
});

/**
 * Two cards whose abilities ask the same step and answer with the number their own text declares,
 * so a fusion holding both shows which ingredient a stored question comes back to (R102, R113).
 */
export const LOW_TELL = 1;
export const HIGH_TELL = 5;
export const lowTeller = def("low-teller", "Field Spell", {
  params: [{ key: "tell", base: LOW_TELL, radiant: LOW_TELL * 2, better: "up", step: 1, min: 1 }],
});
export const highTeller = def("high-teller", "Field Spell", {
  params: [{ key: "tell", base: HIGH_TELL, radiant: HIGH_TELL * 2, better: "up", step: 1, min: 1 }],
});

/** The tellers' script: "Activate: ask; the answer notes {tell}", read by the step the answer re-enters. */
function teller(): CardScripts {
  return faces({
    activations: [{ id: "tell", label: "Ask, then tell {tell}", uses: 1, run: () => [askController("told")] }],
    resume: { told: (ctx) => [note(`told:${param(ctx, "tell")}`)] },
  });
}

export const ACTIVATE_DEFS: CardDef[] = [
  logCard,
  pinger,
  sentry,
  turtle,
  nose,
  lockdown,
  ghost,
  merchant,
  punisher,
  asker,
  scepter,
  chooser,
  mourner,
  trapper,
  endless,
  heroic,
  keeper,
  zapper,
  spark,
  lowTeller,
  highTeller,
];

export const ACTIVATE_SCRIPTS: Record<string, CardScripts> = {
  [logCard.id]: faces({}),
  [pinger.id]: faces({ activations: [ping(1, 1)] }, { activations: [ping(2, 2)] }),
  [ghost.id]: faces({ targetingDiscards: () => 2 }),
  [sentry.id]: faces({
    activations: [{ id: "surge", label: "Gain 1 mana", uses: 2, run: () => [gainMana({ amount: 1 })] }],
  }),
  [turtle.id]: faces({
    activations: [
      {
        id: "eat",
        label: "Tribute a Unit. Deal damage equal to its Attack to any target",
        uses: "unlimited",
        cost: { tribute: 1 },
        targets: [{ ...ANY_TARGET, filter: { of: ["unit", "hero"] } }],
        run: (ctx) => [damage({ to: { of: "chosen" }, amount: activationPaid(ctx).tributed[0]?.attack ?? 0 })],
      },
    ],
  }),
  [nose.id]: faces({
    activations: [{ id: "sniff", label: "Discard a random card", uses: 1, cost: { discardRandom: 1 }, run: () => [note("sniff")] }],
  }),
  [lockdown.id]: faces({
    activations: [
      {
        id: "leave",
        label: "Tribute this",
        uses: 1,
        cost: { tributeSelf: true },
        run: (ctx) => [note(`left:${ctx.self?.zone.z ?? "none"}`)],
      },
    ],
  }),
  [merchant.id]: faces({
    activations: [{ id: "buy", label: "Spend (2): Draw 1", uses: 1, cost: { mana: MERCHANT_PRICE }, run: () => [draw({ count: 1 })] }],
  }),
  [punisher.id]: faces(punish(false), punish(true)),
  [asker.id]: faces({
    staticFlags: { quickdraw: true },
    activations: [
      { id: "ask", label: "Ask", uses: 1, run: () => [note("ask"), askController("asked"), note("ask:tail")] },
    ],
    resume: { asked: () => [note("answered")] },
  }),
  [scepter.id]: faces({
    activations: [
      {
        id: "cast",
        label: "Cast a copy of that Spell",
        uses: 1,
        canActivate: ({ self }) => self.memory[SCEPTER_KEY] !== undefined,
        run: () => [note("cast")],
      },
    ],
  }),
  [chooser.id]: faces({
    activations: [
      { id: "alpha", label: "Alpha", uses: 1, has: ({ self }) => self.memory[PICK_KEY] === "alpha", run: () => [note("alpha")] },
      { id: "beta", label: "Beta", uses: 1, has: ({ self }) => self.memory[PICK_KEY] === "beta", run: () => [note("beta")] },
      { id: "gamma", label: "Gamma", uses: 1, run: () => [note("gamma")] },
    ],
  }),
  [mourner.id]: faces({
    death: () => [note("death"), askController("mourned"), note("death:tail")],
    resume: { mourned: () => [note("mourned")] },
  }),
  [trapper.id]: faces({
    activations: [{ id: "spring", label: "Spring", uses: 1, run: () => [note("spring")] }],
  }),
  [endless.id]: faces({
    activations: [{ id: "again", label: "Again", uses: "unlimited", run: () => [note("again")] }],
  }),
  [heroic.id]: faces(
    {
      staticFlags: { quickdraw: true },
      startOfGame: () => [rollPower()],
      activations: powerAbilities(false),
      resume: { [POWER_RESUME]: heroPower },
    },
    {
      staticFlags: { quickdraw: true },
      startOfGame: () => [rollPower()],
      activations: powerAbilities(true),
      resume: { [POWER_RESUME]: heroPower },
    },
  ),
  [keeper.id]: faces({
    activations: [
      {
        id: "recall",
        label: "Note what this remembered",
        uses: 1,
        canActivate: ({ self }) => recalled({ self, data: {} }, KEEPER_KEY) !== undefined,
        run: (ctx) => [note(`recall:${String(recalled(ctx, KEEPER_KEY))}`)],
      },
    ],
  }),
  [zapper.id]: faces({
    activations: [
      {
        id: "zap",
        label: "Deal {damage} damage",
        uses: 1,
        targets: [{ ...ANY_TARGET, filter: { of: ["unit", "hero"] } }],
        run: (ctx) => [damage({ to: { of: "chosen" }, amount: param(ctx, "damage") })],
      },
    ],
  }),
  [spark.id]: faces({}),
  [lowTeller.id]: teller(),
  [highTeller.id]: teller(),
};

export function activateCatalog(base: CardDefs = {}): CardDefs {
  return { ...base, ...Object.fromEntries(ACTIVATE_DEFS.map((entry) => [entry.id, entry])) };
}
