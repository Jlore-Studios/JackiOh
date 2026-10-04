// Heroic Power (SPEC §8 #98, R43, R103, R352, patch v0.2.1's R651–R658): the thirteen powers, the roll
// that picks one, and each power as an Activate ability.
//
// R43 puts everything about the card on its instance: `memory.power` is the power it rolled, and its
// uses are the Activate count every ability keeps (`memory.activations`, R384). Nothing here is a
// module variable, so two Heroic Powers in one game never share a roll or a use, and a serialized game
// resumes knowing both (§10.1). The roll goes through the match rng, so a replay rolls the same power
// (§9.3).
//
// Patch v0.2.1 moves the card onto Activate (docs/classic-sets.md B3.2 rule 10): it costs (0) to play
// and playing it uses nothing; each power is an "Activate: Spend (X): …" ability, declared by
// `heroPowerActivations` for #98's script, which the `activate` action uses through
// `subsystems/activate.ts` like any other card's — the mana price, the declared target (the ping's,
// so the power can be dragged to it), the once-per-turn count, the `activated` event. The card
// declares one ability per power and has only the one it rolled (`ActivationDecl.has`).
//
// Four powers ask. Witness Value's Discover, Stitching's two and Terminus Tricks' are prompts by
// definition (§6.3). Each is one builder that reads `ctx.targets`: with a selection it does the work,
// without one it opens the prompt and names `POWER_RESUME` as the step to come back to. The answer
// re-enters `heroPower` below with the selection in `ctx.targets`, which runs the same builder down
// its other branch — so a paused activation is a `PendingChoice` in state and never a callback (§9.3,
// §10.6).

import type { CardType, PlayerId, TargetDecl } from "@jackioh/shared";
import { opponentOf } from "@jackioh/shared";
import { defByIndex, type CatalogQueryArgs } from "../catalog";
import {
  ARMOR_UP,
  DIE_INSECT_DAMAGE,
  DIE_INSECT_LUCKY,
  HERO_POWER_COST,
  KY_BRAINSTORM_DISCOUNT,
  LIFE_TAP_DAMAGE,
  LIFE_TAP_DRAW,
  PING_DAMAGE,
  PLUCK_COST,
  STEADY_SHOT_STEP_DAMAGE,
  STEADY_SHOT_UPGRADE_STEPS,
} from "../config";
import {
  addRandomFromCatalog,
  addToHand,
  afterStateCheck,
  chosenOptions,
  damage,
  discoverFromCatalog,
  draw,
  drawFromOpponent,
  forEachCard,
  fuseCards,
  gainHeroArmor,
  gainHeroArmorUntilNextTurn,
  recruit,
  setCostMod,
  summon,
  summonRandom,
  upgradeOwnNumber,
} from "../effects";
import { selfOnItsStay } from "../effects/targets";
import { cardTypeOf } from "../faces";
import { unitView } from "../layers";
import { costNow, isXCost } from "../mana";
import { param } from "../params";
import { lazyPart, type EngineSink } from "../resolve";
import type { ActivationDecl, Effect, EffectContext, Hook } from "../script";
import { findInstance, type CardInstance, type GameState } from "../state";
import { exitMark, leftFieldAfter } from "../stays";
import { activeUnitsOf, slotOf } from "../zones";
import { projectedHeroDamage } from "./lethal";
import { abilitiesOf, usesAllowed, usesThisTurn } from "./activate";

/** R43: where the rolled power lives on the instance. */
export const POWER_KEY = "power";

/**
 * The resume step a power that opened a prompt comes back to (§10.6). `prompts.ts` stores it as the
 * `step` of the `Resume` a prompt carries and looks the answer up in the card's own step table
 * (`RESUME_HOOK`), so #98's script wires the pair together in one line:
 *
 *     resume: { [POWER_RESUME]: heroPower }
 *
 * Nothing else in the engine needs to know about hero powers for a prompted power to work.
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

/** R656: Steady Shot's damage, the number #98 declares (`params`) so its Radiant face can Upgrade it. */
export const STEADY_SHOT_PARAM = "shot";

/** R654: Ping's declared target — any unit or hero, either side — carried in the `activate` action (R81). */
const PING_TARGETS: TargetDecl[] = [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }];

/** R657: "a Trap" — a Field Trap is a Trap that stays (§5.1), so the pool holds both. */
const TRAP_TYPES: CardType[] = ["Trap", "Field Trap"];

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
  | "terminus";

export type HeroPower = {
  /** R103: the name stored in `memory.power`; state, so stable across versions. */
  name: HeroPowerName;
  /** "Activate: Spend (X)" (§8 #98): the mana the power's activation spends. */
  x: number;
  /** The power's name as the card prints it, base face and Radiant face (Armor Up's is Tank Up). */
  title: string;
  radiantTitle: string;
  /** The §8 #98 clause this entry implements, base form and Radiant form (`{shot}` is `params`). */
  label: string;
  radiantLabel: string;
  /** The choices its activation declares (R81): only the ping's target. */
  targets?: TargetDecl[];
  /** The effects one activation runs. `radiant` picks the Radiant clause (§5.2). */
  build: (ctx: EffectContext, radiant: boolean) => Effect[];
};

function tokenDefId(index: string): string | null {
  return defByIndex(TOKEN_SET, index)?.id ?? null;
}

/** §7: a token summon needs the token's def id, which the catalog holds under its index (R103). */
function summonToken(index: string, radiant = false): Effect[] {
  const defId = tokenDefId(index);
  if (defId === null) return [];
  return [summon({ defId, ...(radiant ? { radiant: true } : {}) })];
}

/**
 * Expedition Map. R43: "'Recruit a card' recruits a permanent", which is what §6.3's Recruit already
 * does — it scans the library top down for the first Unit, Field Spell, Trap or Field Trap.
 */
function recruitEffects(_ctx: EffectContext, radiant: boolean): Effect[] {
  return [recruit({ radiant })];
}

/**
 * Life Tap, R655: "Draw 1. Take 2 damage" — damage now, a hit on its own hero from the card through
 * §4.4, where v0.2.0's "lose 2 health" was a loss (R18). The Radiant face, "Draw 1 from each player's
 * deck", draws the top of its own deck and then the top of the opponent's, a draw of its own taken
 * from their deck (§6.3 Draw, C #58), which becomes its card (R12); it takes no damage.
 */
function drawEffects(_ctx: EffectContext, radiant: boolean): Effect[] {
  if (radiant) return [draw({ count: LIFE_TAP_DRAW }), ...Array.from({ length: LIFE_TAP_DRAW }, () => drawFromOpponent({ end: "top" }))];
  return [draw({ count: LIFE_TAP_DRAW }), damage({ to: { of: "selfHero" }, amount: LIFE_TAP_DAMAGE })];
}

/**
 * R654: what Radiant Ping's hit did to its target, read as the hit lands and kept as the rider's part
 * memo, so a Death that asks — whose answer comes in a later action, with an event list of its own —
 * still finds it (§9.3, R113). `from` is the field's departures then (R174).
 */
type PingKill = { doomed: boolean; attack: number; health: number; from: number };

function isPingKill(memo: unknown): memo is PingKill {
  if (memo === null || typeof memo !== "object") return false;
  const kill = memo as Partial<PingKill>;
  return (
    typeof kill.doomed === "boolean" &&
    typeof kill.attack === "number" &&
    typeof kill.health === "number" &&
    typeof kill.from === "number"
  );
}

/**
 * R42, R89: whether the hit doomed the target with this card as its killer — the Unit's lethal hit is
 * this card's (`lastDamagedBy`) — and the attack and max health it has as the check is about to
 * collect it, which are the ones it dies with.
 */
function pingKillOf(ctx: EffectContext, targetId: string | null, sourceId: string | undefined): PingKill {
  const from = exitMark(ctx.state);
  const none: PingKill = { doomed: false, attack: 0, health: 0, from };
  if (targetId === null || sourceId === undefined) return none;
  const unit = findInstance(ctx.state, targetId);
  if (unit === undefined || unit.zone.z !== "field" || unit.lastDamagedBy !== sourceId) return none;
  const view = unitView(ctx.state, unit);
  // A hit that doomed it took its health to 0 or less, or was Poisonous (§4.4 step 7: a Heroic Power
  // fused onto a Poisonous card, R102), and either way named this card its killer above.
  if (view.health > 0 && unit.markedDestroyed !== true) return none;
  return { doomed: true, attack: view.attack, health: view.maxHealth, from };
}

/**
 * Ping, R654: "Pierce. Deal 1 damage" to the target the activation declared (R81), any unit or hero.
 * The Radiant face adds "If this kills a Unit, summon a Ghoul Token with its stats": the hit's kill
 * is read as it lands (`pingKillOf`), the check runs right after it (§4.5, as `afterStateCheck` runs
 * one), and a target that hit doomed and that check took off the field (R174; an Indestructible one
 * stays, R46) summons a Ghoul Token for the activating player with the attack and max health the Unit
 * died with as its X/X (`statsOverride`). The rider runs after that state check, so a Death that asks
 * is answered before the Ghoul is made.
 */
function pingEffects(ctx: EffectContext, radiant: boolean): Effect[] {
  const hit = damage({ to: { of: "chosen" }, amount: PING_DAMAGE, ignoreArmor: true });
  if (!radiant) return [hit];
  const sourceId = ctx.self?.id;
  const [target] = ctx.targets;
  const targetId = target?.pick === "instance" ? target.instanceId : null;
  return [
    hit,
    // R654: the part is built once the hit has landed, and a resume rebuilds it from its memo.
    lazyPart("pingKill", (landed, memo) => {
      const kill = isPingKill(memo) ? memo : pingKillOf(landed, targetId, sourceId);
      return {
        effects: [
          // `afterStateCheck` parks the Ghoul behind a Death prompt rather than resolving it while an
          // answer is open (R59, R113).
          afterStateCheck((after) => {
            const ghoul = tokenDefId(GHOUL_TOKEN_INDEX);
            if (!kill.doomed || targetId === null || ghoul === null || after.state.result !== null) return [];
            if (!leftFieldAfter(after.state, kill.from, targetId)) return [];
            return [summon({ defId: ghoul, statsOverride: { attack: kill.attack, health: kill.health } })];
          }),
        ],
        memo: kill,
      };
    }),
  ];
}

/**
 * Steady Shot, R656: "Deal {shot} damage to the enemy hero" (2, Radiant 4, the card's declared
 * number). The Radiant face then Upgrades that number by one step, +2, kept in the card's `tuning` in
 * every zone (R386), so each Radiant shot hits 2 harder than the last.
 */
function burnEffects(ctx: EffectContext, radiant: boolean): Effect[] {
  const shot = damage({ to: { of: "enemyHero" }, amount: param(ctx, STEADY_SHOT_PARAM) });
  if (!radiant) return [shot];
  return [shot, upgradeOwnNumber({ key: STEADY_SHOT_PARAM, steps: STEADY_SHOT_UPGRADE_STEPS })];
}

/** Ranching: "Summon a Rush Token"; Radiant: "Summon a Radiant Rush Token" (§7's 6/6 Rush, Cleave). */
function rushEffects(_ctx: EffectContext, radiant: boolean): Effect[] {
  return summonToken(RUSH_TOKEN_INDEX, radiant);
}

/**
 * Cat Cafe: "Summon a Felinor Token"; Radiant: "Summon a random Felinor" — a non-token Felinor Unit
 * of any set (R380), its base face, since the text does not say Radiant.
 */
function felinorEffects(_ctx: EffectContext, radiant: boolean): Effect[] {
  if (radiant) return [summonRandom({ query: { type: "Unit", tags: ["Felinor"] } })];
  return summonToken(FELINOR_TOKEN_INDEX);
}

/**
 * Witness Value. §6.3 Discover: 1 of 3 Units, shown only to the chooser. The pick comes back as a
 * mode selection carrying a def id, and the Unit goes to hand — Radiant when the power is (R103).
 */
function discoverEffects(ctx: EffectContext, radiant: boolean): Effect[] {
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

/** R352: Stitching's "Discover two Units that cost (2) or less". */
export const STITCHING_INGREDIENTS = 2;
export const STITCHING_MAX_COST = 2;
/** The data key a paused Stitching carries its Discover picks in (§10.6). */
const STITCHING_PICKS_KEY = "picks";

/** The picks a paused Stitching has made so far, read back out of the captured data. */
function stitchedSoFar(ctx: EffectContext): string[] {
  const stored: unknown = ctx.data[STITCHING_PICKS_KEY];
  if (!Array.isArray(stored)) return [];
  return stored.filter((entry): entry is string => typeof entry === "string");
}

/**
 * R352, §8 #98: "Stitching: Discover two Units that cost (2) or less. Fuse them and add the result to
 * your hand." Two chained Discovers, as #99 Craft a Card chains its own, each answer re-entering
 * `heroPower` with the picks so far in the prompt's data; the second answer fuses the two per R77
 * with no target on the field. Unlike #99's, the result keeps R77's fused cost, min(sum, 4) — the
 * power says nothing of a price — and on the Radiant face ("Discover two Radiant Units") the result
 * is Radiant. A Discover with no pool left fizzles (§8 Conventions), and fewer than two picks fuse
 * nothing: the activation is still spent.
 */
function stitchingEffects(ctx: EffectContext, radiant: boolean): Effect[] {
  const answered = chosenOptions(ctx)[0];
  const picks = answered === undefined ? stitchedSoFar(ctx) : [...stitchedSoFar(ctx), answered];
  if (picks.length >= STITCHING_INGREDIENTS) {
    return [fuseCards({ defIds: picks, toHand: "self", handPrice: "fused", radiant })];
  }
  return [
    discoverFromCatalog({
      step: POWER_RESUME,
      query: { type: "Unit", costRange: { max: STITCHING_MAX_COST } },
      prompt: `Discover a ${radiant ? "Radiant " : ""}Unit that costs (${STITCHING_MAX_COST}) or less`,
      data: { [POWER_DATA_KEY]: "stitching", [STITCHING_PICKS_KEY]: picks },
    }),
  ];
}

/**
 * Armor Up, R651: "Your hero gains 2 Armor until your next turn" — §4.4 step 2's per-hit reduction,
 * held until the start of the activating player's next turn. The Radiant face, Tank Up, "Your hero
 * gains 4 Armor", names no end: the designer writes each power's Radiant face out in full (Life Tap's
 * drops its damage the same way), so its 4 is the hero's for the rest of the game, as C+ #46's Armor
 * is. It then refreshes the power into a different one (R652), whose Radiant face the card runs from
 * then on.
 */
function armorEffects(_ctx: EffectContext, radiant: boolean): Effect[] {
  if (!radiant) return [gainHeroArmorUntilNextTurn({ amount: ARMOR_UP.base })];
  return [gainHeroArmor({ amount: ARMOR_UP.radiant }), refreshPower()];
}

/** R653: one pick for Die Insect, an enemy unit acting on the field or the enemy hero. */
type InsectPick = { kind: "unit"; unit: CardInstance } | { kind: "hero" };

/** R653's documented Lucky ordering: a lethal hero, a destroyed Unit, then a damaged hero, then a surviving Unit. */
const INSECT_PRIORITY = {
  survives: 0,
  heroDamage: 1,
  unitDestroyed: 2,
  heroLethal: 3,
} as const;

/**
 * R653: Lucky's "best" for Die Insect (§6.1: a comparator per effect). A pick the hit would finish
 * beats one it would not — the enemy hero when the hit is lethal, then a Unit it destroys — and
 * otherwise the hero, which takes the whole hit, beats a Unit that survives it. Two Units of one kind
 * are ordered as R414 orders Soul Shot's: the higher attack plus current health, then the higher
 * cost, then the lower lane.
 */
function insectRank(state: GameState, enemy: PlayerId, pick: InsectPick): number {
  if (pick.kind === "hero") {
    return projectedHeroDamage(state, enemy, DIE_INSECT_DAMAGE) >= state.players[enemy].hero.health
      ? INSECT_PRIORITY.heroLethal
      : INSECT_PRIORITY.heroDamage;
  }
  const view = unitView(state, pick.unit);
  const shielded = view.keywords.some((keyword) => keyword.kind === "Divine Shield" || keyword.kind === "Indestructible");
  return !shielded && Math.max(0, DIE_INSECT_DAMAGE - view.armor) >= view.health
    ? INSECT_PRIORITY.unitDestroyed
    : INSECT_PRIORITY.survives;
}

function betterInsectPick(state: GameState, enemy: PlayerId): (a: InsectPick, b: InsectPick) => InsectPick {
  const worth = (unit: CardInstance): number => {
    const view = unitView(state, unit);
    return view.attack + view.health;
  };
  const lane = (unit: CardInstance): number => slotOf(state, unit)?.lane ?? Number.MAX_SAFE_INTEGER;
  return (a, b) => {
    const rankA = insectRank(state, enemy, a);
    const rankB = insectRank(state, enemy, b);
    if (rankA !== rankB) return rankA > rankB ? a : b;
    if (a.kind !== "unit" || b.kind !== "unit") return a;
    if (worth(a.unit) !== worth(b.unit)) return worth(a.unit) > worth(b.unit) ? a : b;
    if (costNow(state, a.unit) !== costNow(state, b.unit)) return costNow(state, a.unit) > costNow(state, b.unit) ? a : b;
    return lane(b.unit) < lane(a.unit) ? b : a;
  };
}

/**
 * Die Insect, R653: "Deal 8 damage to a random enemy" — one pick, uniform over the enemy hero and the
 * enemy Units acting on the field (§3.2, R13), drawn from the match rng as the effect applies; the
 * Radiant face's "Lucky 1" draws a second pick and keeps the better (`betterInsectPick`).
 */
function damageRandomEnemy(lucky: number): Effect {
  return {
    kind: "dieInsect",
    apply(ctx): void {
      const enemy = opponentOf(ctx.controller);
      const pool: InsectPick[] = [
        ...activeUnitsOf(ctx.state, enemy).map((unit): InsectPick => ({ kind: "unit", unit })),
        { kind: "hero" },
      ];
      const roll = (): InsectPick | undefined => ctx.rng.pick(pool);
      const better = betterInsectPick(ctx.state, enemy);
      const pick =
        lucky > 0
          ? ctx.rng.lucky(lucky, roll, (a, b) => (a === undefined ? b : b === undefined ? a : better(a, b)))
          : roll();
      if (pick === undefined) return;
      const to = pick.kind === "hero" ? { of: "enemyHero" as const } : { of: "instance" as const, instanceId: pick.unit.id };
      damage({ to, amount: DIE_INSECT_DAMAGE }).apply(ctx);
    },
  };
}

function insectEffects(_ctx: EffectContext, radiant: boolean): Effect[] {
  return [damageRandomEnemy(radiant ? DIE_INSECT_LUCKY : 0)];
}

/**
 * KY Brainstorm, R658: "Add a random KY card to your hand. Reduce the cost of all Spells in your hand
 * by (1)" — the KY pool of every set (R380), Radiant on the Radiant face; then every Spell in the hand
 * as it stands, the card just added included, takes `costMod` −1 (an X-cost Spell, which no modifier
 * reaches, R65, is left out).
 */
function brainstormEffects(_ctx: EffectContext, radiant: boolean): Effect[] {
  return [
    addRandomFromCatalog({ query: { tags: ["KY"] }, ...(radiant ? { radiant: true } : {}) }),
    forEachCard({
      cards: (ctx) =>
        ctx.state.players[ctx.controller].hand.filter(
          (card) => cardTypeOf(ctx.state, card) === "Spell" && !isXCost(ctx.state, card),
        ),
      each: (instanceId) => setCostMod({ target: { of: "instance", instanceId }, amount: -KY_BRAINSTORM_DISCOUNT }),
    }),
  ];
}

/**
 * Pluck, R658: "Add a random Fruit to your hand. It costs (0)" — R382's Fruit pool, the five Grapes
 * included; Radiant on the Radiant face.
 */
function pluckEffects(_ctx: EffectContext, radiant: boolean): Effect[] {
  return [addRandomFromCatalog({ query: { tags: ["Fruit"] }, costOverride: PLUCK_COST, ...(radiant ? { radiant: true } : {}) })];
}

/**
 * Terminus Tricks, R657: "Discover a Trap to summon" — 1 of 3 Traps or Field Traps of any set, shown
 * only to the chooser; the pick is summoned into its controller's backrow face-down (R64's leftmost
 * open zone), Radiant on the Radiant face. With no open backrow zone the summon fails silently (§3.2).
 */
const TRAP_POOL: CatalogQueryArgs = { type: TRAP_TYPES };

function terminusEffects(ctx: EffectContext, radiant: boolean): Effect[] {
  const picked = chosenOptions(ctx)[0];
  if (picked !== undefined) return [summon({ defId: picked, ...(radiant ? { radiant: true } : {}) })];
  return [
    discoverFromCatalog({
      step: POWER_RESUME,
      query: TRAP_POOL,
      prompt: radiant ? "Discover a Radiant Trap to summon" : "Discover a Trap to summon",
      data: { [POWER_DATA_KEY]: "terminus" },
    }),
  ];
}

/**
 * The thirteen powers of §8 #98 (patch v0.2.1), in the order the roll draws from. R103: the stored
 * name is state, and a new power is added at the end, so the eight before patch v0.2.1 keep their
 * places and its five follow (the card prints them in its own order).
 */
export const HERO_POWERS: readonly HeroPower[] = [
  {
    name: "recruit",
    x: HERO_POWER_COST.recruit,
    title: "Expedition Map",
    radiantTitle: "Expedition Map",
    label: "Recruit a permanent",
    radiantLabel: "Recruit a permanent. Make it Radiant",
    build: recruitEffects,
  },
  {
    name: "draw",
    x: HERO_POWER_COST.draw,
    title: "Life Tap",
    radiantTitle: "Life Tap",
    label: `Draw ${LIFE_TAP_DRAW}. Take ${LIFE_TAP_DAMAGE} damage`,
    radiantLabel: `Draw ${LIFE_TAP_DRAW} from each player's deck`,
    build: drawEffects,
  },
  {
    name: "ping",
    x: HERO_POWER_COST.ping,
    title: "Ping",
    radiantTitle: "Ping",
    label: `Pierce. Deal ${PING_DAMAGE} damage`,
    radiantLabel: `Pierce. Deal ${PING_DAMAGE} damage. If this kills a Unit, summon a Ghoul Token with its stats`,
    targets: PING_TARGETS,
    build: pingEffects,
  },
  {
    name: "burn",
    x: HERO_POWER_COST.burn,
    title: "Steady Shot",
    radiantTitle: "Steady Shot",
    label: `Deal {${STEADY_SHOT_PARAM}} damage to the enemy hero`,
    radiantLabel: `Deal {${STEADY_SHOT_PARAM}} damage to the enemy hero. Upgrade this permanently by +${STEADY_SHOT_STEP_DAMAGE} damage`,
    build: burnEffects,
  },
  {
    name: "rush",
    x: HERO_POWER_COST.rush,
    title: "Ranching",
    radiantTitle: "Ranching",
    label: "Summon a Rush Token",
    radiantLabel: "Summon a Radiant Rush Token",
    build: rushEffects,
  },
  {
    name: "felinor",
    x: HERO_POWER_COST.felinor,
    title: "Cat Cafe",
    radiantTitle: "Cat Cafe",
    label: "Summon a Felinor Token",
    radiantLabel: "Summon a random Felinor",
    build: felinorEffects,
  },
  {
    name: "discover",
    x: HERO_POWER_COST.discover,
    title: "Witness Value",
    radiantTitle: "Witness Value",
    label: "Discover a Unit",
    radiantLabel: "Discover a Radiant Unit",
    build: discoverEffects,
  },
  {
    name: "stitching",
    x: HERO_POWER_COST.stitching,
    title: "Stitching",
    radiantTitle: "Stitching",
    label: `Discover two Units that cost (${STITCHING_MAX_COST}) or less. Fuse them and add the result to your hand`,
    radiantLabel: `Discover two Radiant Units that cost (${STITCHING_MAX_COST}) or less. Fuse them and add the result to your hand`,
    build: stitchingEffects,
  },
  {
    name: "armor",
    x: HERO_POWER_COST.armor,
    title: "Armor Up",
    radiantTitle: "Tank Up",
    label: `Your hero gains ${ARMOR_UP.base} Armor until your next turn`,
    radiantLabel: `Your hero gains ${ARMOR_UP.radiant} Armor. Refresh this power`,
    build: armorEffects,
  },
  {
    name: "insect",
    x: HERO_POWER_COST.insect,
    title: "Die Insect",
    radiantTitle: "Die Insect",
    label: `Deal ${DIE_INSECT_DAMAGE} damage to a random enemy`,
    radiantLabel: `Lucky ${DIE_INSECT_LUCKY}. Deal ${DIE_INSECT_DAMAGE} damage to a random enemy`,
    build: insectEffects,
  },
  {
    name: "brainstorm",
    x: HERO_POWER_COST.brainstorm,
    title: "KY Brainstorm",
    radiantTitle: "KY Brainstorm",
    label: `Add a random KY card to your hand. Reduce the cost of all Spells in your hand by (${KY_BRAINSTORM_DISCOUNT})`,
    radiantLabel: `Add a random Radiant KY card to your hand. Reduce the cost of all Spells in your hand by (${KY_BRAINSTORM_DISCOUNT})`,
    build: brainstormEffects,
  },
  {
    name: "pluck",
    x: HERO_POWER_COST.pluck,
    title: "Pluck",
    radiantTitle: "Pluck",
    label: `Add a random Fruit to your hand. It costs (${PLUCK_COST})`,
    radiantLabel: `Add a random Radiant Fruit to your hand. It costs (${PLUCK_COST})`,
    build: pluckEffects,
  },
  {
    name: "terminus",
    x: HERO_POWER_COST.terminus,
    title: "Terminus Tricks",
    radiantTitle: "Terminus Tricks",
    label: "Discover a Trap to summon",
    radiantLabel: "Discover a Radiant Trap to summon",
    build: terminusEffects,
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

/**
 * R43: the (X) the card's power spends to activate, which the hero panel shows. Since patch v0.2.1
 * the card itself costs (0) to play (§8 #98), so this is no longer its cost; a card that has not
 * rolled has no power to price and reads 0.
 */
export function powerCostOf(instance: CardInstance): number {
  return powerOf(instance)?.x ?? 0;
}

/**
 * R43's roll: a Heroic Power that has no power picks one from the match rng and remembers it, and
 * one that already has a power keeps it. #98 calls this from `startOfGame` for every copy in a hand
 * or library (§6.2), and again whenever a copy arrives somewhere without one — a bounced or reset
 * instance (R78) — which is why it is idempotent rather than a plain roll.
 */
export function ensurePower(sink: EngineSink, instance: CardInstance): HeroPower | null {
  const existing = powerOf(instance);
  if (existing !== null) return existing;
  const rolled = sink.rng.pick(HERO_POWERS);
  if (rolled === undefined) return null;
  instance.memory[POWER_KEY] = rolled.name;
  return rolled;
}

function sinkOf(ctx: EffectContext): EngineSink {
  return { state: ctx.state, events: ctx.events, rng: ctx.rng };
}

function cardOf(ctx: EffectContext, instanceId?: string): CardInstance | null {
  if (instanceId === undefined) return ctx.self;
  return findInstance(ctx.state, instanceId) ?? null;
}

/**
 * R43's roll as an effect, so #98's `startOfGame` hook is a list of effects like every other card's
 * (CLAUDE.md rule 5).
 */
export function rollPower(args: { instanceId?: string } = {}): Effect {
  return {
    kind: "rollPower",
    apply(ctx): void {
      const card = cardOf(ctx, args.instanceId);
      if (card === null) return;
      ensurePower(sinkOf(ctx), card);
    },
  };
}

/**
 * R652, Tank Up: "then this power refreshes into a different Radiant one" — the card rolls again,
 * uniformly from the match rng over the twelve powers it does not have, and runs the new one's face
 * from then on (the card is Radiant, so its Radiant face). Its use this turn is spent already: the
 * Activate count is the card's, whichever ability spent it (R384), so the new power waits for the next
 * turn. A card that has left the field since the run began is not this power (R174).
 */
export function refreshPower(): Effect {
  return {
    kind: "refreshPower",
    apply(ctx): void {
      const card = selfOnItsStay(ctx);
      if (card === null || card.zone.z !== "field") return;
      const current = powerOf(card);
      const next = ctx.rng.pick(HERO_POWERS.filter((power) => power.name !== current?.name));
      if (next === undefined) return;
      card.memory[POWER_KEY] = next.name;
    },
  };
}

/** R102: an ability's id on a fused card carries its part (`<id>#<n>`); its own id is the part before. */
function abilityBase(id: string): string {
  return id.split("#")[0] ?? id;
}

/** The card's Activate ability for the power it has now (R384), when it has one. */
function powerDecl(state: GameState, instance: CardInstance): ActivationDecl | undefined {
  const power = powerOf(instance);
  if (power === null) return undefined;
  return abilitiesOf(state, instance).find((ability) => abilityBase(ability.id) === power.name);
}

/**
 * The id an `activate` names for the card's power (R384) — the power's name, or a fused card's own
 * id for it (R102) — or null for a card that has no power to use.
 */
export function powerAbilityOf(state: GameState, instance: CardInstance): string | null {
  return powerDecl(state, instance)?.id ?? null;
}

/**
 * Whether the card's power is spent for this turn: its Activate uses are all used (R384). A card
 * Upgrade has turned into "Activate 2" has two (R386). The count is the card's, so this reads the
 * ability of the power it has now.
 */
export function usedThisTurn(state: GameState, instance: CardInstance): boolean {
  const decl = powerDecl(state, instance);
  if (decl === undefined) return usesThisTurn(state, instance) > 0;
  return usesThisTurn(state, instance) >= usesAllowed(instance, decl);
}

// ---------------------------------------------------------------------------
// The powers as Activate abilities (R43, R384, patch v0.2.1).
// ---------------------------------------------------------------------------

/** The power's printed name on a face (§8 #98: the Radiant face of Armor Up is Tank Up). */
export function powerTitle(power: HeroPower, radiant: boolean): string {
  return radiant ? power.radiantTitle : power.title;
}

/**
 * #98's `activations` (B3.2 rule 10, R43): one "Activate: Spend (X): …" ability per power, under the
 * power's stored name, once per turn, paying the power's X in mana, with the ping's target declared so
 * it travels in the action (R81) — and the card has only the one it rolled (`has`), so only that one
 * is listed, shown or accepted. `radiant` is the face that declares them (§5.2).
 */
export function heroPowerActivations(radiant: boolean): ActivationDecl[] {
  return HERO_POWERS.map((power) => ({
    id: power.name,
    label: `${powerTitle(power, radiant)}: ${radiant ? power.radiantLabel : power.label}`,
    uses: 1,
    cost: { mana: power.x },
    ...(power.targets === undefined ? {} : { targets: power.targets }),
    has: ({ self }) => powerOf(self)?.name === power.name,
    run: (ctx) => power.build(ctx, radiant),
  }));
}

/**
 * The continuation an answered prompt comes back to: `prompts.ts` re-enters the step the prompt's
 * `Resume` names — `POWER_RESUME`, which #98's `resume` table points here — with the selection
 * already in `ctx.targets` (§10.6). The power's own builder finishes the activation, so the
 * prompted and the unprompted path are one piece of code.
 */
export const heroPower: Hook = (ctx) => {
  const named = ctx.data[POWER_DATA_KEY];
  const power = (typeof named === "string" ? powerByName(named) : null) ?? (ctx.self === null ? null : powerOf(ctx.self));
  if (power === null) return [];
  return power.build(ctx, ctx.self?.radiant ?? ctx.radiant);
};
