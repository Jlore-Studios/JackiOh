// Heroic Power (SPEC §8 #98, R43, R103, R352, R752–R761): the thirteen powers, the roll that picks
// one, and each power as the Activate ability it is since the Heroic Power patch.
//
// R43 puts everything about the card on its instance: `memory.power` is the power it rolled. Nothing
// here is a module variable, so two Heroic Powers in one game never share a roll or a use, and a
// serialized game resumes knowing both (§10.1). The roll goes through the match rng, so a replay rolls
// the same power (§9.3).
//
// R752: the card costs (0) and playing it uses nothing. Each power is an "Activate: Spend (X): …"
// ability (R384, `activate.ts`): once per turn, its X paid in mana as it is activated, its target —
// Ping's — declared with the activation (R81), so the client can drag the power onto it. The card
// declares all thirteen (`powerAbilities`) and has only the one it rolled (`ActivationDecl.has`), so
// `legalActions`, the refusal, the view and a resumed tail all read the one Activate subsystem, and
// `activatePower` is an alias that names the rolled power's ability (`reduce.ts`).
//
// Three powers pause on a Discover (§6.3): Witness Value, Stitching and Terminus Tricks. Each is one
// builder that reads the answer: with one it does the work, without one it opens the Discover and
// names `POWER_RESUME` as the step to come back to. The answer re-enters `heroPower` below with the
// pick in the context, which runs the same builder down its other branch — so a paused activation is a
// `PendingChoice` in state and never a callback (§9.3, §10.6).

import type { PlayerId, TargetDecl } from "@jackioh/shared";
import { opponentOf } from "@jackioh/shared";
import { defByIndex } from "../catalog";
import {
  addPlayerModifier,
  addRandomFromCatalog,
  addToHand,
  afterStateCheck,
  cardsInScope,
  chosenOptions,
  damage,
  discoverFromCatalog,
  draw,
  drawFromOpponent,
  forEachCard,
  fuseCards,
  gainHeroArmor,
  recruit,
  resolveTarget,
  setCostMod,
  setNumber,
  summon,
  summonRandom,
} from "../effects";
import { cardTypeOf } from "../faces";
import { unitView } from "../layers";
import { costNow } from "../mana";
import { param } from "../params";
import type { EngineSink } from "../resolve";
import type { ActivationDecl, Effect, EffectContext, Hook } from "../script";
import { leftFieldAfter } from "../stays";
import type { CardInstance, GameState } from "../state";
import { slotOf } from "../zones";
import { ACTIVATIONS_MEMORY_KEY, abilitiesOf, usesAllowed, usesThisTurn } from "./activate";

/** R43: where the rolled power lives on the instance. */
export const POWER_KEY = "power";

/**
 * The resume step a power that opened a Discover comes back to (§10.6). `prompts.ts` stores it as
 * the `step` of the `Resume` a prompt carries and looks the answer up in the card's own step table,
 * so #98's script wires the pair together in one line:
 *
 *     resume: { [POWER_RESUME]: heroPower }
 */
export const POWER_RESUME = "heroPower";

/** The data key that names the power a paused activation is finishing. */
export const POWER_DATA_KEY = "power";

/** §7: the tokens #98 summons, by catalog index (§5.3), so the engine names no catalog id. */
export const RUSH_TOKEN_INDEX = "T-rush";
export const FELINOR_TOKEN_INDEX = "T-felinor";
export const GHOUL_TOKEN_INDEX = "T-ghoul";
/** B2.2: the set whose indices those are, since an index is unique only within its set. */
const TOKEN_SET = "Core";

/** R753: Life Tap's "Take 2 damage", dealt to its own hero by the card. */
export const LIFE_TAP_DAMAGE = 2;
/** R754: Steady Shot's declared number ("{shot}"), and how far its Radiant face raises it per use. */
export const STEADY_SHOT_PARAM = "shot";
export const STEADY_SHOT_RAISE = 2;
/** R756: Ping's damage. */
export const PING_DAMAGE = 1;
/** R757: Armor Up's Armor until your next turn, and the Armor Tank Up (its Radiant face) keeps. */
export const ARMOR_UP_ARMOR = 2;
export const TANK_UP_ARMOR = 4;
/** R758: Die Insect's damage, and its Radiant face's Lucky. */
export const DIE_INSECT_DAMAGE = 8;
export const DIE_INSECT_LUCKY = 1;
/** R759: how much KY Brainstorm takes off each Spell in your hand. */
export const BRAINSTORM_DISCOUNT = 1;
/** R760: what Pluck's Fruit costs. */
export const PLUCK_COST = 0;
/** R352: Stitching's "Discover two (2) Cost or less Units". */
export const STITCHING_INGREDIENTS = 2;
export const STITCHING_MAX_COST = 2;
/** The data key a paused Stitching carries its Discover picks in (§10.6). */
const STITCHING_PICKS_KEY = "picks";

/** R756: Ping reaches any unit or hero, either side, declared with the activation (R81). */
const PING_TARGETS: TargetDecl[] = [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }];

/** R761: Terminus Tricks Discovers a Trap; a Field Trap is a Trap. */
const TRAP_TYPES: ("Trap" | "Field Trap")[] = ["Trap", "Field Trap"];

/**
 * R103: the stored power names. They are state, so they stay stable across versions and a new power
 * is added at the end: the eight of v0.1.1, then the five the Heroic Power patch added (R752).
 */
export type HeroPowerName =
  | "recruit"
  | "draw"
  | "ping"
  | "burn"
  | "rush"
  | "felinor"
  | "discover"
  | "stitching"
  | "armor"
  | "insect"
  | "brainstorm"
  | "pluck"
  | "tricks";

export type HeroPower = {
  name: HeroPowerName;
  /** The power's name on the card (§8 #98), on each face: Armor Up is Tank Up on the Radiant one. */
  title: string;
  radiantTitle: string;
  /** "Activate: Spend (X)" (R752): the mana each use pays. */
  x: number;
  /** The §8 #98 words this entry implements, base face and Radiant face. */
  label: string;
  radiantLabel: string;
  /** What the activation declares (R81): Ping's target. */
  targets?: TargetDecl[];
  /** The effects one activation runs. `radiant` picks the Radiant face (§5.2). */
  build: (ctx: EffectContext, radiant: boolean) => Effect[];
};

function tokenDefId(index: string): string | null {
  return defByIndex(TOKEN_SET, index)?.id ?? null;
}

/** R103: a token summon resolves the token by catalog index, and fizzles in silence without it. */
function summonToken(index: string, radiant: boolean): Effect[] {
  const defId = tokenDefId(index);
  return defId === null ? [] : [summon({ defId, ...(radiant ? { radiant: true } : {}) })];
}

/** Expedition Map. R43: "Recruit a permanent", which is §6.3's Recruit; the Radiant face makes it Radiant. */
function expeditionMap(_ctx: EffectContext, radiant: boolean): Effect[] {
  return [recruit({ radiant })];
}

/** Life Tap (R753): draw, then 2 damage to your hero; Radiant: the top card of each player's deck. */
function lifeTap(_ctx: EffectContext, radiant: boolean): Effect[] {
  if (radiant) return [draw({ count: 1 }), drawFromOpponent({ end: "top" })];
  return [draw({ count: 1 }), damage({ to: { of: "selfHero" }, amount: LIFE_TAP_DAMAGE })];
}

/** Steady Shot (R754): {shot} to the enemy hero; on the Radiant face, then {shot} goes up by 2 for good. */
function steadyShot(ctx: EffectContext, radiant: boolean): Effect[] {
  const shot = param(ctx, STEADY_SHOT_PARAM);
  const hit = damage({ to: { of: "enemyHero" }, amount: shot });
  if (!radiant) return [hit];
  return [hit, setNumber({ target: { of: "self" }, which: `param:${STEADY_SHOT_PARAM}`, value: shot + STEADY_SHOT_RAISE })];
}

/** Ranching: a Rush Token, Radiant on the Radiant face (§7). */
function ranching(_ctx: EffectContext, radiant: boolean): Effect[] {
  return summonToken(RUSH_TOKEN_INDEX, radiant);
}

/** Cat Cafe (R755): a Felinor Token; the Radiant face summons a random non-token Felinor of every set. */
function catCafe(_ctx: EffectContext, radiant: boolean): Effect[] {
  if (radiant) return [summonRandom({ query: { type: "Unit", tags: ["Felinor"] }, player: "self" })];
  return summonToken(FELINOR_TOKEN_INDEX, false);
}

/**
 * Ping (R756): Pierce, 1 damage to the unit or hero declared with the activation. The Radiant face
 * then asks, after the state check that follows the hit (R59), whether the unit left the field in it
 * (`leftFieldAfter` against the activation's own mark, which a pause keeps, R174) — killed, a Reborn
 * body put back included — and if so summons a Ghoul Token for you with that unit's attack and health
 * as it stood before the hit (its health undamaged: Ping kills only a unit left on 1).
 */
function ping(ctx: EffectContext, radiant: boolean): Effect[] {
  const hit = damage({ to: { of: "chosen" }, amount: PING_DAMAGE, ignoreArmor: true });
  if (!radiant) return [hit];
  const target = resolveTarget(ctx, { of: "chosen" });
  const ghoul = tokenDefId(GHOUL_TOKEN_INDEX);
  const mark = ctx.exitsFrom;
  if (target === null || target.kind !== "unit" || ghoul === null || mark === undefined) return [hit];
  const unit = target.instance;
  const view = unitView(ctx.state, unit);
  const stats = { attack: Math.max(0, view.attack), health: Math.max(1, view.maxHealth) };
  return [
    hit,
    afterStateCheck((after) =>
      leftFieldAfter(after.state, mark, unit.id) ? [summon({ defId: ghoul, player: "self", statsOverride: stats })] : [],
    ),
  ];
}

/**
 * Witness Value. §6.3 Discover: 1 of 3 Units, shown only to the chooser. The pick comes back as a mode
 * selection carrying a def id, and the Unit goes to hand — Radiant when the power is (R103).
 */
function witnessValue(ctx: EffectContext, radiant: boolean): Effect[] {
  const picked = chosenOptions(ctx)[0];
  if (picked !== undefined) return [addToHand({ defId: picked, player: "self", radiant })];
  return [
    discoverFromCatalog({
      step: POWER_RESUME,
      query: { type: "Unit" },
      prompt: radiant ? "Discover a Radiant Unit" : "Discover a Unit",
      data: { [POWER_DATA_KEY]: "discover" },
    }),
  ];
}

/** The picks a paused Stitching has made so far, read back out of the captured data. */
function stitchedSoFar(ctx: EffectContext): string[] {
  const stored: unknown = ctx.data[STITCHING_PICKS_KEY];
  if (!Array.isArray(stored)) return [];
  return stored.filter((entry): entry is string => typeof entry === "string");
}

/**
 * R352: "Discover two (2) Cost or less Units. Fuse them." Two chained Discovers, each answer
 * re-entering `heroPower` with the picks so far in the prompt's data; the second answer fuses the two
 * per R77 into the activating player's hand at R77's fused cost, min(sum, 4). On the Radiant face the
 * Units are Radiant and so is the result. A Discover with no pool left fizzles, and fewer than two
 * picks fuse nothing.
 */
function stitching(ctx: EffectContext, radiant: boolean): Effect[] {
  const answered = chosenOptions(ctx)[0];
  const picks = answered === undefined ? stitchedSoFar(ctx) : [...stitchedSoFar(ctx), answered];
  if (picks.length >= STITCHING_INGREDIENTS) {
    return [fuseCards({ defIds: picks, toHand: "self", handPrice: "fused", radiant })];
  }
  return [
    discoverFromCatalog({
      step: POWER_RESUME,
      query: { type: "Unit", costRange: { max: STITCHING_MAX_COST } },
      prompt: radiant ? "Discover a Radiant Unit that costs (2) or less" : "Discover a Unit that costs (2) or less",
      data: { [POWER_DATA_KEY]: "stitching", [STITCHING_PICKS_KEY]: picks },
    }),
  ];
}

/**
 * R757: the power refreshes into a different one — it rolls again among the other twelve (the match
 * rng, as the first roll did) and gives back the use this activation spent, so the new power may be
 * activated this turn too, paying its own X.
 */
export function refreshPower(): Effect {
  return {
    kind: "refreshPower",
    apply(ctx): void {
      const card = ctx.self;
      if (card === null) return;
      const current = powerOf(card);
      const next = ctx.rng.pick(HERO_POWERS.filter((power) => power.name !== current?.name));
      if (next === undefined) return;
      card.memory[POWER_KEY] = next.name;
      const used = usesThisTurn(ctx.state, card);
      if (used > 0) card.memory[ACTIVATIONS_MEMORY_KEY] = { turn: ctx.state.turn, count: used - 1 };
    },
  };
}

/**
 * Armor Up (R757): 2 hero Armor until your next turn — a player modifier that lasts through the
 * opponent's next turn, so it is gone when yours begins. Tank Up, its Radiant face: 4 Armor your hero
 * keeps, then the power refreshes (`refreshPower`).
 */
function armorUp(ctx: EffectContext, radiant: boolean): Effect[] {
  if (radiant) return [gainHeroArmor({ amount: TANK_UP_ARMOR }), refreshPower()];
  const opponent: PlayerId = opponentOf(ctx.controller);
  return [
    addPlayerModifier({
      mod: {
        kind: "heroArmor",
        amount: ARMOR_UP_ARMOR,
        expiry: { until: "nextTurnOf", player: opponent, fromTurn: ctx.state.turn },
      },
    }),
  ];
}

/** R758: what Die Insect may hit — an enemy Unit acting on the field, or the enemy hero (null). */
type InsectPick = CardInstance | null;

/** R758, R414: a Unit beats the hero; between Units, higher attack plus current health, then cost, then the lower lane. */
function betterInsectPick(state: GameState): (a: InsectPick, b: InsectPick) => InsectPick {
  const worth = (card: CardInstance): number => {
    const view = unitView(state, card);
    return view.attack + view.health;
  };
  const lane = (card: CardInstance): number => slotOf(state, card)?.lane ?? Number.MAX_SAFE_INTEGER;
  return (a, b) => {
    if (a === null) return b;
    if (b === null) return a;
    if (worth(a) !== worth(b)) return worth(a) > worth(b) ? a : b;
    if (costNow(state, a) !== costNow(state, b)) return costNow(state, a) > costNow(state, b) ? a : b;
    return lane(b) < lane(a) ? b : a;
  };
}

/**
 * Die Insect (R758): 8 damage to a random enemy — one pick over the enemy Units acting on the field
 * and the enemy hero, each as likely. The Radiant face is Lucky 1: two picks, the better kept.
 */
function dieInsect(radiant: boolean): Effect {
  return {
    kind: "dieInsect",
    apply(ctx): void {
      const pool: InsectPick[] = [...cardsInScope(ctx, { side: "enemy" }), null];
      const roll = (): InsectPick => ctx.rng.pick(pool) ?? null;
      const pick = radiant ? ctx.rng.lucky(DIE_INSECT_LUCKY, roll, betterInsectPick(ctx.state)) : roll();
      damage({
        to: pick === null ? { of: "enemyHero" } : { of: "instance", instanceId: pick.id },
        amount: DIE_INSECT_DAMAGE,
      }).apply(ctx);
    },
  };
}

/** R759: the Spells (the Spell type, not Field Spells) in the activating player's hand, read once. */
function spellsInHand(ctx: EffectContext): CardInstance[] {
  return ctx.state.players[ctx.controller].hand.filter((card) => cardTypeOf(ctx.state, card) === "Spell");
}

/**
 * KY Brainstorm (R759): a random KY card of every set (R380) to your hand — Radiant on the Radiant
 * face — then every Spell in your hand, that card included, costs (1) less there.
 */
function kyBrainstorm(_ctx: EffectContext, radiant: boolean): Effect[] {
  return [
    addRandomFromCatalog({ query: { tags: ["KY"] }, ...(radiant ? { radiant: true } : {}) }),
    forEachCard({
      cards: spellsInHand,
      each: (instanceId) =>
        setCostMod({ target: { of: "instance", instanceId }, amount: -BRAINSTORM_DISCOUNT, inHandOnly: true }),
    }),
  ];
}

/** Pluck (R760): a random Fruit (R382's pool, a Grape rolled again) to your hand, costing (0); Radiant on the Radiant face. */
function pluck(_ctx: EffectContext, radiant: boolean): Effect[] {
  return [addRandomFromCatalog({ query: { tags: ["Fruit"] }, costOverride: PLUCK_COST, ...(radiant ? { radiant: true } : {}) })];
}

/**
 * Terminus Tricks (R761): Discover a Trap (a Trap or Field Trap of every set) and summon it — face-down
 * into your leftmost free backrow zone (§3.2, R64), fizzling with none (R47). The Radiant face offers
 * Radiant Traps and summons the pick Radiant.
 */
function terminusTricks(ctx: EffectContext, radiant: boolean): Effect[] {
  const picked = chosenOptions(ctx)[0];
  if (picked !== undefined) return [summon({ defId: picked, player: "self", ...(radiant ? { radiant: true } : {}) })];
  return [
    discoverFromCatalog({
      step: POWER_RESUME,
      query: { type: TRAP_TYPES },
      prompt: radiant ? "Discover a Radiant Trap to summon" : "Discover a Trap to summon",
      data: { [POWER_DATA_KEY]: "tricks" },
    }),
  ];
}

/** The thirteen powers of §8 #98 (R752), in R103's stored order: the eight of v0.1.1, then the five added. */
export const HERO_POWERS: readonly HeroPower[] = [
  {
    name: "recruit",
    title: "Expedition Map",
    radiantTitle: "Expedition Map",
    x: 3,
    label: "Recruit a permanent.",
    radiantLabel: "Recruit a permanent. Make it Radiant.",
    build: expeditionMap,
  },
  {
    name: "draw",
    title: "Life Tap",
    radiantTitle: "Life Tap",
    x: 1,
    label: "Draw 1. Take 2 damage.",
    radiantLabel: "Draw 1 from each player's deck.",
    build: lifeTap,
  },
  {
    name: "ping",
    title: "Ping",
    radiantTitle: "Ping",
    x: 1,
    label: "Pierce. Deal 1 damage.",
    radiantLabel: "Pierce. Deal 1 damage. If this kills a Unit, summon a Ghoul Token with its stats.",
    targets: PING_TARGETS,
    build: ping,
  },
  {
    name: "burn",
    title: "Steady Shot",
    radiantTitle: "Steady Shot",
    x: 1,
    label: "Deal {shot} damage to the enemy hero.",
    radiantLabel: "Deal {shot} damage to the enemy hero. Upgrade this permanently by +2 damage.",
    build: steadyShot,
  },
  {
    name: "rush",
    title: "Ranching",
    radiantTitle: "Ranching",
    x: 2,
    label: "Summon a Rush Token.",
    radiantLabel: "Summon a Radiant Rush Token.",
    build: ranching,
  },
  {
    name: "felinor",
    title: "Cat Cafe",
    radiantTitle: "Cat Cafe",
    x: 1,
    label: "Summon a Felinor Token.",
    radiantLabel: "Summon a random Felinor.",
    build: catCafe,
  },
  {
    name: "discover",
    title: "Witness Value",
    radiantTitle: "Witness Value",
    x: 2,
    label: "Discover a Unit.",
    radiantLabel: "Discover a Radiant Unit.",
    build: witnessValue,
  },
  {
    name: "stitching",
    title: "Stitching",
    radiantTitle: "Stitching",
    x: 2,
    label: "Discover two (2) Cost or less Units. Fuse them.",
    radiantLabel: "Discover two Radiant (2) Cost or less Units. Fuse them.",
    build: stitching,
  },
  {
    name: "armor",
    title: "Armor Up",
    radiantTitle: "Tank Up",
    x: 1,
    label: "Your hero gains 2 Armor until your next turn.",
    radiantLabel: "Your hero gains 4 Armor, then this power refreshes.",
    build: armorUp,
  },
  {
    name: "insect",
    title: "Die Insect",
    radiantTitle: "Die Insect",
    x: 2,
    label: "Deal 8 damage to a random enemy.",
    radiantLabel: "Lucky 1. Deal 8 damage to a random enemy.",
    build: (_ctx, radiant) => [dieInsect(radiant)],
  },
  {
    name: "brainstorm",
    title: "KY Brainstorm",
    radiantTitle: "KY Brainstorm",
    x: 2,
    label: "Add a random KY card to your hand. Reduce the cost of all Spells in your hand by (1).",
    radiantLabel: "Add a Radiant KY card to your hand. Reduce the cost of all Spells in your hand by (1).",
    build: kyBrainstorm,
  },
  {
    name: "pluck",
    title: "Pluck",
    radiantTitle: "Pluck",
    x: 2,
    label: "Add a random Fruit to your hand. It costs (0).",
    radiantLabel: "Add a random Radiant Fruit to your hand. It costs (0).",
    build: pluck,
  },
  {
    name: "tricks",
    title: "Terminus Tricks",
    radiantTitle: "Terminus Tricks",
    x: 3,
    label: "Discover a Trap to summon.",
    radiantLabel: "Discover a Radiant Trap to summon.",
    build: terminusTricks,
  },
];

export const HERO_POWER_NAMES: readonly HeroPowerName[] = HERO_POWERS.map((power) => power.name);

export function powerByName(name: string): HeroPower | null {
  return HERO_POWERS.find((power) => power.name === name) ?? null;
}

// ---------------------------------------------------------------------------
// The power on the instance (R43).
// ---------------------------------------------------------------------------

/** The power this card rolled, or null for a card that has not rolled one yet. */
export function powerOf(instance: CardInstance): HeroPower | null {
  const name = instance.memory[POWER_KEY];
  return typeof name === "string" ? powerByName(name) : null;
}

/** The power's name on the card's face (R752): Armor Up reads Tank Up on a Radiant card. */
export function powerTitleOf(power: HeroPower, radiant: boolean): string {
  return radiant ? power.radiantTitle : power.title;
}

/**
 * R43's roll: a Heroic Power that has no power picks one from the match rng and remembers it, and
 * one that already has a power keeps it. #98 calls this from `startOfGame` for every copy in a hand
 * or library (§6.2), and again whenever a copy arrives somewhere without one — a bounced or reset
 * instance (R78, R151) — which is why it is idempotent rather than a plain roll.
 */
export function ensurePower(sink: EngineSink, instance: CardInstance): HeroPower | null {
  const existing = powerOf(instance);
  if (existing !== null) return existing;
  const rolled = sink.rng.pick(HERO_POWERS);
  if (rolled === undefined) return null;
  instance.memory[POWER_KEY] = rolled.name;
  return rolled;
}

/**
 * R43's roll as an effect, so #98's `startOfGame` hook is a list of effects like every other card's
 * (CLAUDE.md rule 5).
 */
export function rollPower(): Effect {
  return {
    kind: "rollPower",
    apply(ctx): void {
      if (ctx.self === null) return;
      ensurePower({ state: ctx.state, events: ctx.events, rng: ctx.rng }, ctx.self);
    },
  };
}

/**
 * R752: the card's thirteen Activate abilities, one per power, on the face `radiant` names: each pays
 * its power's X (`cost.mana`), declares what it targets (Ping's, R81), and is the card's only while
 * the card rolled that power (`has`). The ability's id is the stored name (R103), so a fused card's
 * ids (R102), a resumed tail (`activation:<id>`) and the alias all name the same power.
 */
export function powerAbilities(radiant: boolean): ActivationDecl[] {
  return HERO_POWERS.map((power) => ({
    id: power.name,
    label: `${powerTitleOf(power, radiant)}: ${radiant ? power.radiantLabel : power.label}`,
    uses: 1,
    cost: { mana: power.x },
    ...(power.targets === undefined ? {} : { targets: power.targets }),
    has: ({ self }) => self.memory[POWER_KEY] === power.name,
    run: (ctx) => power.build(ctx, radiant),
  }));
}

/**
 * The continuation a Discover comes back to: `prompts.ts` re-enters the step the prompt's `Resume`
 * names — `POWER_RESUME`, which #98's `resume` table points here — with the pick in the context
 * (§10.6). The power's own builder finishes the activation, so the prompted and the unprompted path
 * are one piece of code.
 */
export const heroPower: Hook = (ctx) => {
  const named = ctx.data[POWER_DATA_KEY];
  const power = (typeof named === "string" ? powerByName(named) : null) ?? (ctx.self === null ? null : powerOf(ctx.self));
  if (power === null) return [];
  return power.build(ctx, ctx.radiant);
};

/** The ability of the power this card rolled: its id is the power's name, `<name>#n` on a fusion (R102). */
export function powerAbilityOf(state: GameState, card: CardInstance): ActivationDecl | null {
  const power = powerOf(card);
  if (power === null) return null;
  return abilitiesOf(state, card).find((decl) => decl.id.split("#")[0] === power.name) ?? null;
}

/** R752: whether the card's power has spent this turn's uses (Activate's count, R384). */
export function usedThisTurn(state: GameState, card: CardInstance): boolean {
  const decl = powerAbilityOf(state, card);
  return decl !== null && usesThisTurn(state, card) >= usesAllowed(card, decl);
}
