import type {
  CardType,
  CraftEffect,
  CraftHat,
  CraftHatKind,
  CraftKeyword,
  CraftRecipe,
  CraftVerb,
  Keyword,
} from "../../wire/index.ts";
import { CRAFT_MAX_N, CRAFT_VERB_PRICES } from "../../wire/engineConfig.ts";

/** ME-CRAFT (Meditative #17, R880–R883): the block editor's pure recipe helpers. Every helper
 * returns a new recipe and leaves its argument alone, so the editor's state stays immutable. */

/** What the palette calls each effect verb, in the block table's words. */
export const VERB_LABELS: Record<CraftVerb, string> = {
  damageTarget: "Deal damage to a target",
  damageEnemyHero: "Deal damage to the enemy hero",
  damageRandomEnemy: "Deal damage to a random enemy",
  damageEachEnemy: "Deal damage to each enemy",
  healTarget: "Heal a target",
  healYourHero: "Heal your hero",
  healYourSide: "Heal your hero and your units",
  draw: "Draw cards",
  gainMana: "Gain mana",
  gainArmor: "Gain Armor",
  summonRush: "Summon a Rush Token",
  summonFelinor: "Summon a Felinor Token",
  summonSheep: "Summon a Sheep Token",
  buffTarget: "Give a unit +N/+N",
  buffYourUnits: "Give your units +N/+N",
  grantKeyword: "Give a unit a keyword",
  destroy: "Destroy a unit",
  bounce: "Bounce a unit",
  addRandom: "Add a random card",
  discover: "Discover a card",
  opponentDiscards: "Opponent discards",
  buffRandomCard: "Buff a random card of yours",
  nerfRandomEnemyCard: "Nerf a random enemy card",
  lockRandomZone: "Lock a random enemy zone",
};

/** What the palette calls each hat. */
export const HAT_LABELS: Record<CraftHatKind, string> = {
  cry: "Cry",
  death: "Death",
  startOfTurn: "Start of turn",
  endOfTurn: "End of turn",
  whenCast: "When cast",
  opponentPlaysUnit: "Opponent plays a Unit",
  opponentPlaysSpell: "Opponent plays a Spell",
  opponentAttacks: "Opponent attacks",
};

/** The hats a card type may take (R883), for what the palette offers and the canvas keeps. */
export const HATS_FOR_TYPE: Record<CardType, readonly CraftHatKind[]> = {
  Unit: ["cry", "death", "startOfTurn", "endOfTurn"],
  Spell: ["whenCast"],
  "Field Spell": ["cry", "startOfTurn", "endOfTurn"],
  Trap: ["opponentPlaysUnit", "opponentPlaysSpell", "opponentAttacks"],
  "Field Trap": [],
};

function verbTakesN(verb: CraftVerb): boolean {
  return CRAFT_VERB_PRICES.find((row) => row.verb === verb)?.takesN === true;
}

/** The card's type. Fields its new type cannot hold are cleared; hats it cannot take are dropped. */
export function setType(recipe: CraftRecipe, type: CardType): CraftRecipe {
  const allowed = HATS_FOR_TYPE[type] ?? [];
  return {
    ...recipe,
    type,
    attack: type === "Unit" ? recipe.attack : 0,
    health: type === "Unit" ? recipe.health : 1,
    keywords:
      type === "Unit" || type === "Spell"
        ? recipe.keywords.filter((block) => type === "Unit" || block.kind === "Lifesteal" || block.kind === "Pierce")
        : [],
    echo: type === "Spell" ? recipe.echo : 0,
    hats: recipe.hats.filter((hat) => allowed.includes(hat.hat)),
  };
}

/** Step a Unit's attack or health. The engine refuses what the type cannot hold (R880). */
export function stepStat(recipe: CraftRecipe, stat: "attack" | "health", delta: number): CraftRecipe {
  return { ...recipe, [stat]: Math.max(0, recipe[stat] + delta) };
}

/** Add a keyword block, or remove it when it is already there. Armor starts at 1. */
export function toggleKeyword(recipe: CraftRecipe, kind: Keyword["kind"], n?: number): CraftRecipe {
  if (recipe.keywords.some((block) => block.kind === kind)) {
    return { ...recipe, keywords: recipe.keywords.filter((block) => block.kind !== kind) };
  }
  const block: CraftKeyword =
    kind === "Armor" ? { kind, n: n ?? 1 } : { kind };
  return { ...recipe, keywords: [...recipe.keywords, block] };
}

/** Step Armor's N, 1 to `CRAFT_MAX_N`. */
export function stepKeywordN(recipe: CraftRecipe, kind: Keyword["kind"], delta: number): CraftRecipe {
  return {
    ...recipe,
    keywords: recipe.keywords.map((block) =>
      block.kind === kind
        ? { ...block, n: Math.min(CRAFT_MAX_N, Math.max(1, (block.n ?? 1) + delta)) }
        : block,
    ),
  };
}

/** Snap a hat onto the canvas, unless it is already there. */
export function addHat(recipe: CraftRecipe, hat: CraftHatKind): CraftRecipe {
  if (recipe.hats.some((block) => block.hat === hat)) return recipe;
  const block: CraftHat = { hat, effects: [] };
  return { ...recipe, hats: [...recipe.hats, block] };
}

/** Snap an effect block under a hat. A verb that takes N starts at 1. */
export function addEffect(recipe: CraftRecipe, hatIndex: number, verb: CraftVerb): CraftRecipe {
  const hats = recipe.hats.map((hat, index) => {
    if (index !== hatIndex) return hat;
    const effect: CraftEffect = verbTakesN(verb) ? { verb, n: 1 } : { verb };
    return { ...hat, effects: [...hat.effects, effect] };
  });
  return { ...recipe, hats };
}

/** Step an effect's N, 1 to `CRAFT_MAX_N`. */
export function stepN(recipe: CraftRecipe, hatIndex: number, effectIndex: number, delta: number): CraftRecipe {
  const hats = recipe.hats.map((hat, hi) => {
    if (hi !== hatIndex) return hat;
    return {
      ...hat,
      effects: hat.effects.map((effect, ei) =>
        ei !== effectIndex || effect.n === undefined
          ? effect
          : { ...effect, n: Math.min(CRAFT_MAX_N, Math.max(1, effect.n + delta)) },
      ),
    };
  });
  return { ...recipe, hats };
}

/** Set an effect's keyword (`grantKeyword`) or card type (`addRandom`, `discover`). */
export function setEffectSlot(
  recipe: CraftRecipe,
  hatIndex: number,
  effectIndex: number,
  slot: { keyword?: Keyword["kind"] } | { cardType?: CardType },
): CraftRecipe {
  const hats = recipe.hats.map((hat, hi) => {
    if (hi !== hatIndex) return hat;
    return {
      ...hat,
      effects: hat.effects.map((effect, ei) =>
        ei !== effectIndex ? effect : { ...effect, ...slot },
      ),
    };
  });
  return { ...recipe, hats };
}

/** Lift a hat (and the effects under it) off the canvas. */
export function removeHat(recipe: CraftRecipe, hatIndex: number): CraftRecipe {
  return { ...recipe, hats: recipe.hats.filter((_, index) => index !== hatIndex) };
}

/** Lift an effect block off its hat. */
export function removeEffect(recipe: CraftRecipe, hatIndex: number, effectIndex: number): CraftRecipe {
  const hats = recipe.hats.map((hat, hi) => {
    if (hi !== hatIndex) return hat;
    return { ...hat, effects: hat.effects.filter((_, index) => index !== effectIndex) };
  });
  return { ...recipe, hats };
}

/** Move an effect block within its hat, or across hats. */
export function moveEffect(
  recipe: CraftRecipe,
  from: { hat: number; effect: number },
  to: { hat: number; effect?: number },
): CraftRecipe {
  const moving = recipe.hats[from.hat]?.effects[from.effect];
  if (moving === undefined || recipe.hats[to.hat] === undefined) return recipe;
  const hats = recipe.hats.map((hat, hi) => {
    if (hi === from.hat && hi === to.hat) {
      const effects = hat.effects.filter((_, ei) => ei !== from.effect);
      const at = to.effect ?? effects.length;
      return { ...hat, effects: [...effects.slice(0, at), moving, ...effects.slice(at)] };
    }
    if (hi === from.hat) return { ...hat, effects: hat.effects.filter((_, ei) => ei !== from.effect) };
    if (hi === to.hat) {
      const at = to.effect ?? hat.effects.length;
      return { ...hat, effects: [...hat.effects.slice(0, at), moving, ...hat.effects.slice(at)] };
    }
    return hat;
  });
  return { ...recipe, hats };
}
