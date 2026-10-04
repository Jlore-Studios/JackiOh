// Fuse and Craft a Card (SPEC §6.3 Fuse, R77): the transient definition two or three cards make,
// and the instance the result lives on. Used by #85 Unlicensed Experimentation and #99 Craft a Card.
//
// Three things make Fuse unlike every other verb.
//
// First, the result is a *definition*, not an instance: two cards' base forms make the fused base
// form and their radiant forms make the fused radiant one, so the fused card still has both faces
// and Make Radiant keeps working on it (§5.2). Definitions are shared by every copy of a card and
// are never edited, so the fusion writes a fresh one into `state.transientDefs`, where `defOf`
// finds it ahead of the catalog. It is match state and survives a JSON round-trip (§10.1).
//
// Second, the scripts are code, which no JSON state can hold. The concatenated pair therefore joins
// the script registry under the new def id, exactly as `packages/cards` and the test fixtures
// register theirs (`scripts.ts`). The registry is the process's — every match on a server, and every
// world the practice AI simulates beside the real game in its worker — so the id is what keeps it
// right: R179's id names the ingredients (`t-<n>:<a>+<b>`), and the fused scripts are a function of
// the ingredients' ids and nothing else (`fusedScript`), so one id means one pair of scripts in every
// state that can mint it, and no fusion replaces another state's. Because the id names them, the
// pair can also be rebuilt from the id alone (`fusedIngredients`): `syncFusedScripts` registers any
// of a state's fused scripts the registry lacks — a state that came through JSON into a process
// that never ran its Fuse, or a registry some package re-registered wholesale — and the engine calls
// it wherever it is entered (`reduce`, `legalActions`, `viewFor`), so a replay of the same action
// list runs the same scripts (§9.3).
//
// Third, R77 keeps an ingredient's *instance* when one of them is a target already on the field:
// the fused card is that card, with its zone, damage, exertion, counters and memory intact, and the
// other ingredients cease to exist without dying. So the only instance-level work here is the def
// id, the summed buffs and the united granted keywords; everything else is deliberately untouched.
//
// Patch v0.2.0 (docs/classic-sets.md B5 E23) adds three things, each a ruling of its own. R468: an
// id that would spell out a list longer than `FUSED_ID_CAP` is a digest of that list instead, and
// the definition keeps the list (`CardDef.ingredients`), which is what rebuilds a digest's scripts.
// R469: an ingredient may go in on its Radiant face ("fuse a random Radiant card"), lending that
// face and its text to both fused forms; the id marks it `*`. R470: the kept instance may be a card
// in a hand or a library (`into`), which stays where it is, and "its cost doesn't change" keeps the
// cost it had (`keepCost`).

import type {
  CardCost,
  CardDef,
  CardFace,
  CardType,
  FusedIngredient,
  Keyword,
  PlayerId,
  Rarity,
  Selection,
  Tag,
} from "@jackioh/shared";
import { keywordKey } from "@jackioh/shared";
import {
  FUSED_DIGEST_MARK,
  RADIANT_INGREDIENT_MARK,
  defOf,
  findDef,
  isDigestId,
  fusedIdParts,
  fusedIdSpecs,
  registerFusedIngredients,
} from "../catalog";
import { FUSED_ID_CAP, FUSE_COST_CAP } from "../config";
import { addToHand } from "../draw";
import { unitedEnchantments } from "../enchantments";
import { unitHas, wornStatsOverride } from "../layers";
import { printedCost } from "../mana";
import type { EngineSink } from "../resolve";
import { activeTargetDecls, selectionsPerDeclaration, storedDeclarationSlices } from "../playChoices";
import { runStartOfGame } from "../prompts";
import { lazyPart } from "../resolve";
import type { AuraHook, CardScripts, Effect, EffectContext, Hook, Script, TargetCheck, TriggerDef } from "../script";
import {
  INGREDIENTS_KEY,
  asIngredient,
  ingredientPaid,
  ingredientRecord,
  registerScripts,
  registeredScripts,
  scriptOf,
  scriptsFor,
} from "../scripts";
import { newInstance, type CardInstance, type GameState } from "../state";
import { sumTunings } from "../tuning";
import { PART_DEPTH_KEY, PART_KEY, partPathOf, rerootRemembered } from "../work";
import { ceaseToExist } from "../zones";

/** R77: Craft a Card fuses "two or three cards", and #85 fuses two. Fewer is not a fusion. */
export const FUSE_MIN_INGREDIENTS = 2;

/** §8 #99: "the result costs 0 and goes to your hand", as a `costOverride` per R65. */
export const CRAFTED_CARD_COST = 0;

/**
 * The three `Script` members that are functions but return no list, so the generic concatenation
 * below cannot combine them and each is handled on its own: `cost` is dropped, because R77 fixes
 * the fused cost at min(sum, 4); `setStat` is summed like every other stat R77 sums; and
 * `conditionMet`, R195's yellow glow, answers a boolean, so the ingredients' hooks are or-ed
 * (R196); `tributeWhen` and `wouldCounter` (R403, R658) are or-ed the same way. A new member of
 * `Script` that returns something other than a list belongs in this set:
 * left to `combineValues`, two such hooks become one that returns an array of their answers.
 * (`preview`, R280, returns a list, but of numbers rather than effects: it is one of `EAGER_KEYS`.)
 */
const COST_KEY = "cost";
/** The `Script` key of the step table a continuation re-enters (`prompts.RESUME_HOOK`). */
const RESUME_KEY = "resume";
/** `Script.targetChecks`: named predicates (§10.6), which are not hooks and combine as `combinedChecks` says. */
const TARGET_CHECKS_KEY = "targetChecks";
const SET_STAT_KEY = "setStat";
const CONDITION_MET_KEY = "conditionMet";
/** Classic #88 (R403): "When …, Tribute this" holds for a fusion when it holds for any ingredient. */
const TRIBUTE_WHEN_KEY = "tributeWhen";
/** R471: a placement multiplier answers a number, so the ingredients' multipliers multiply. */
const PLAGUE_MULTIPLIER_KEY = "plagueMultiplier";
/** Classic #87 (R658): a fusion would counter a play when any ingredient's counter would. */
const WOULD_COUNTER_KEY = "wouldCounter";

/** The script keys whose entries carry an `id` that has to stay unique across the ingredients. */
const TRIGGER_KEYS = ["triggers", "handTriggers", "deckTriggers", "graveyardTriggers"] as const;

/** §8's rarity ladder, lowest first, so a fusion can report the rarest ingredient's rarity. */
const RARITY_ORDER: readonly Rarity[] = ["Token", "Common", "Rare", "Epic", "Legendary", "Mythic"];

export type FuseArgs = {
  /** Every card going into the fusion, in the order the fusing card names them. */
  ingredients: readonly CardInstance[];
  /**
   * R77: an ingredient already on the field that the result keeps as its instance. #85 fuses the
   * permanent the opponent just played onto one of yours, and that one is the target.
   */
  target?: CardInstance;
  /** Craft a Card: no target on the field, so the result is a fresh card in this player's hand. */
  toHand?: PlayerId;
  /**
   * R352: what the hand card costs. `"free"` is Craft a Card's "the result costs 0" (§8 #99), a
   * `costOverride` of `CRAFTED_CARD_COST`; `"fused"` leaves R77's fused cost, min(sum, 4), as
   * Heroic Power's Stitching does. Absent is `"free"`.
   */
  handPrice?: HandPrice;
  /** R352: the hand card is Radiant (radiant Stitching). Absent is R77's non-Radiant hand card. */
  radiant?: boolean;
  // ---- v0.2.0, generation (E23, R468–R470) ----
  /**
   * R470: a card in a hand or a library that the result keeps as its instance, as R77's `target`
   * keeps one on the field — Classic+ #31 Fusion Lab's hand card, Classic+ #73's deck cards, the
   * card Classic #78's Radiant face picks from a hand or a deck. It stays where it is; its type is
   * the result's (R77). A card named here that is not in a hand or a library, or is Immutable (R23),
   * refuses the fusion. `target` wins when both are named.
   */
  into?: CardInstance;
  /**
   * R469: the ingredients, by instance id, that go in on their Radiant face — "fuse a random Radiant
   * card into it", "a Radiant copy of it is fused into this". Such an ingredient puts its Radiant face
   * and text into both of the fused forms; every other ingredient puts in the face the form is.
   */
  radiantIngredients?: readonly string[];
  /**
   * R470: "its cost doesn't change" — the kept card (`target` or `into`) keeps the cost it had:
   * a `costOverride` of its own cost as it stood (its override if it had one), or, for an X-cost or
   * embiggen card with no override, that printed cost form on the fused definition.
   */
  keepCost?: boolean;
};

/** R352: the two prices a fused hand card can have. */
export type HandPrice = "free" | "fused";

// ---------------------------------------------------------------------------
// The fused definition (R77).
// ---------------------------------------------------------------------------

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/**
 * R77's unions: one keyword per distinct keyword — except Armor, which every ingredient keeps. Armor
 * is the keyword whose number stacks from every source (§6.1, §10.4 sums it), so it adds up on the
 * fused face the way the stats beside it do: Armor 7 and Armor 3 print 10, and so do Armor 7 and
 * Armor 7 print 14 rather than collapsing into one because the numbers happen to match (R102).
 */
function unionKeywords(keywords: readonly Keyword[]): Keyword[] {
  const seen = new Set<string>();
  const out: Keyword[] = [];
  for (const keyword of keywords) {
    if (keyword.kind === "Armor") {
      out.push(keyword);
      continue;
    }
    const key = keywordKey(keyword);
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(keyword);
  }
  return out;
}

function unionTags(defs: readonly CardDef[]): Tag[] {
  const seen = new Set<Tag>();
  for (const def of defs) for (const tag of def.tags) seen.add(tag);
  return [...seen];
}

/**
 * R279, R102: the cards a fused text names are every ingredient's — the fused face prints both texts
 * (`fusedFace`), so each name in them still links. The union keeps the first appearance of each id,
 * in ingredient order, and is null when no ingredient names any card, so the def omits the key just
 * as a catalog card with no reference does.
 */
function unionRefs(defs: readonly CardDef[]): string[] | null {
  const seen = new Set<string>();
  for (const def of defs) for (const ref of def.refs ?? []) seen.add(ref);
  return seen.size === 0 ? null : [...seen];
}

/**
 * A stat the fused face has only if some ingredient had it: two fused Spells or Traps keep a face
 * with no attack and no health rather than gaining a printed 0/0 (§5, `CardFace`).
 */
function sumDefined(values: readonly (number | undefined)[]): number | null {
  const defined = values.filter((value): value is number => value !== undefined);
  return defined.length === 0 ? null : defined.reduce((sum, value) => sum + value, 0);
}

/**
 * One ingredient's face as its instance wears it. §7 and R175: a token summoned X/X carries its X as
 * `statsOverride`, and the Bread Token its "Armor X" as `armorOverride`, because neither number can
 * be printed — they ARE its printed face, §10.4's layer 1. So a Fuse sums that X/X, not the printed
 * 0/0 it stands in for, and the X replaces only that token's own Armor, never an Armor another
 * ingredient prints (#85 fusing a 7/7 onto #18's Bread Token keeps the 7/7's Armor 7).
 */
function wornFace(def: CardDef, card: CardInstance | undefined, radiant: boolean): CardFace {
  const face = radiant ? def.radiant : def.base;
  // R349: a token with no Radiant form of its own (the Ghoul Token) doubles its X/X on the Radiant
  // face, so the fused Radiant face sums the doubled X.
  const stats = card === undefined ? undefined : wornStatsOverride(def, { ...card, radiant });
  const armor = card?.armorOverride;
  return {
    ...face,
    ...(stats === undefined ? {} : { attack: stats.attack, health: stats.health }),
    keywords:
      armor === undefined
        ? face.keywords
        : face.keywords.map((keyword) => (keyword.kind === "Armor" ? { kind: "Armor" as const, n: armor } : keyword)),
  };
}

/**
 * R77: one face of the fusion — summed stats, united keywords, both texts. R469: an ingredient that
 * went in on its Radiant face (`forced`) puts that face into the base form too.
 */
function fusedFace(
  ingredients: readonly CardInstance[],
  defs: readonly CardDef[],
  radiant: boolean,
  forced: readonly boolean[],
): CardFace {
  const faces = defs.map((def, at) => wornFace(def, ingredients[at], radiant || forced[at] === true));
  const attack = sumDefined(faces.map((face) => face.attack));
  const health = sumDefined(faces.map((face) => face.health));
  return {
    ...(attack === null ? {} : { attack }),
    ...(health === null ? {} : { health }),
    keywords: unionKeywords(faces.flatMap((face) => face.keywords)),
    text: faces.map((face) => face.text).join("\n"),
  };
}

/**
 * R77: "Its type is the target's, or the ingredients' shared type when there is no target on the
 * field (Field Trap if any ingredient is one)". The parenthetical settles a trap fusion, because a
 * Field Trap is not consumed when it fires and a plain Trap is, so the Field Trap half wins; it
 * cannot turn a unit fusion into a trap, and #85 only ever fuses two cards of the same type.
 *
 * Ingredients of different types with no target is a shape no Core card makes (#99 Discovers Units);
 * the first ingredient's type is the fallback rather than a fizzle.
 */
function fusedType(defs: readonly CardDef[], targetDef: CardDef | null): CardType {
  const types = defs.map((def) => def.type);
  const first = types[0] ?? "Unit";
  const shared = types.every((type) => type === first) ? first : undefined;
  const base = targetDef?.type ?? shared ?? first;
  if ((base === "Trap" || base === "Field Trap") && types.includes("Field Trap")) return "Field Trap";
  return base;
}

/**
 * R77: "min(sum of the printed costs per R65, 4)". `printedCost` is R65's reading of one card —
 * Ceaseless Void's computed cost, the X chosen on the instance, the embiggen price it was played
 * for — so the sum needs no second cost rule here.
 */
function fusedCost(state: GameState, ingredients: readonly CardInstance[]): number {
  const sum = ingredients.reduce((total, card) => total + printedCost(state, card), 0);
  return Math.min(sum, FUSE_COST_CAP);
}

function rarestOf(defs: readonly CardDef[]): Rarity {
  let best: Rarity = "Common";
  let bestRank = -1;
  for (const def of defs) {
    const rank = RARITY_ORDER.indexOf(def.rarity);
    if (rank > bestRank) {
      bestRank = rank;
      best = def.rarity;
    }
  }
  return best;
}

/**
 * The id a transient def gets (R102, R179): `t-<n>`, where n depends only on how many transient defs
 * the state already holds, so the same action list always produces the same id (§9.3) — followed by
 * the ids of the ingredients it was fused from, `t-<n>:<a>+<b>`. An ingredient that is itself a
 * fused card is written in parentheses, `t-2:(t-1:<a>+<b>)+<c>`, so the id reads back one way:
 * without them `t-2:t-1:a+b+c+d` could be `t-1:a+b` fused with c and d, or `t-1:a+b+c` with d.
 *
 * The suffix is what keeps the script registry right. A def is match state, but its scripts are code
 * and live in the process-wide registry under the def's id (see this file's header), and one server
 * process runs every match (§9.2) and folds a match's log to rebuild it (§9.3). With a bare `t-<n>`,
 * two matches that fused different pairs into the same slot shared one registry entry, and the
 * later fusion replaced the scripts of the earlier match's card. The fused scripts are a function of
 * the ingredients' ids and nothing else (`fusedScript`), so an id that carries them names the same
 * scripts in every match that can mint it. R469: an ingredient that went in on its Radiant face is
 * written with a trailing `*`, since its scripts are its Radiant ones on both forms.
 *
 * R468: the list is spelled out only while it fits `FUSED_ID_CAP`. A card fused onto again and
 * again (Classic+ #74) nests every earlier id inside the next, so the spelled-out id grows with every
 * fusion; past the cap the id is `t-<n>:#<digest>`, the digest a pure hash of the very list the id
 * would have spelled out (`fusedDigest`). It still names one list and so one pair of scripts in every
 * match (R179), and the definition keeps the list itself (`CardDef.ingredients`), which is where the
 * scripts are rebuilt from (`syncFusedScripts`).
 */
function nextTransientId(state: GameState, specs: readonly FusedIngredient[]): string {
  const taken = (n: number): boolean =>
    Object.keys(state.transientDefs).some((id) => id === `t-${n}` || id.startsWith(`t-${n}:`));
  let n = Object.keys(state.transientDefs).length + 1;
  while (taken(n)) n += 1;
  const body = specs.map(ingredientName).join("+");
  return body.length <= FUSED_ID_CAP ? `t-${n}:${body}` : `t-${n}:${FUSED_DIGEST_MARK}${fusedDigest(body)}`;
}

const FUSED_ID = /^t-\d+:/;

/**
 * An ingredient as a fused id writes it: in parentheses when it is itself a fused card (R179), and
 * followed by `*` when it went in on its Radiant face (R469).
 */
function ingredientName(spec: FusedIngredient): string {
  const name = FUSED_ID.test(spec.defId) ? `(${spec.defId})` : spec.defId;
  return spec.radiant === true ? `${name}${RADIANT_INGREDIENT_MARK}` : name;
}

/**
 * R468: a pure, deterministic 64-bit digest of a string, as 16 hex digits — two 32-bit lanes of
 * multiply-xorshift mixing (the cyrb53 construction, widened to both lanes), so no `crypto` and no
 * I/O (CLAUDE.md rule 4). It names an ingredient list, not a secret: all it must do is give two
 * different lists two different ids in any process that could hold both.
 */
export function fusedDigest(text: string): string {
  let h1 = 0xdeadbeef ^ text.length;
  let h2 = 0x41c6ce57 ^ text.length;
  for (let at = 0; at < text.length; at += 1) {
    const code = text.charCodeAt(at);
    h1 = Math.imul(h1 ^ code, 2654435761);
    h2 = Math.imul(h2 ^ code, 1597334677);
  }
  h1 = Math.imul(h1 ^ (h1 >>> 16), 2246822507) ^ Math.imul(h2 ^ (h2 >>> 13), 3266489909);
  h2 = Math.imul(h2 ^ (h2 >>> 16), 2246822507) ^ Math.imul(h1 ^ (h1 >>> 13), 3266489909);
  return `${(h2 >>> 0).toString(16).padStart(8, "0")}${(h1 >>> 0).toString(16).padStart(8, "0")}`;
}

/**
 * R179: the ingredient ids a fused def's id names, in ingredient order, or null for an id no Fuse
 * minted (a catalog card's, or a bare `t-<n>`). The inverse of `nextTransientId`: the list is split
 * at the `+` signs outside parentheses, and a parenthesised ingredient loses its parentheses
 * (`catalog.fusedIdParts`, which R387's self-exclusion reads too). R468: a digest id's list comes
 * from the definitions this process has minted or entered.
 */
export function fusedIngredients(defId: string): string[] | null {
  const parts = fusedIdParts(defId);
  return parts !== null && parts.length >= FUSE_MIN_INGREDIENTS ? parts : null;
}

/** R179, R468, R469: `fusedIngredients` with each ingredient's Radiant mark. */
export function fusedIngredientSpecs(defId: string): FusedIngredient[] | null {
  const specs = fusedIdSpecs(defId);
  return specs !== null && specs.length >= FUSE_MIN_INGREDIENTS ? specs : null;
}

/** E36: a fused definition's lines of code are its ingredients' sum, absent when none has any. */
function summedLoc(defs: readonly CardDef[]): number | null {
  const counted = defs.flatMap((def) => (def.loc === undefined ? [] : [def.loc]));
  return counted.length === 0 ? null : counted.reduce((sum, loc) => sum + loc, 0);
}

function buildDef(
  state: GameState,
  ingredients: readonly CardInstance[],
  defs: readonly CardDef[],
  targetDef: CardDef | null,
  forced: readonly boolean[],
  keptCost: KeptCost | null,
  fixedId?: string,
): CardDef {
  const specs: FusedIngredient[] = defs.map((def, at) => ({
    defId: def.id,
    ...(forced[at] === true ? { radiant: true as const } : {}),
  }));
  const id = fixedId ?? nextTransientId(state, specs);
  const refs = unionRefs(defs);
  const loc = summedLoc(defs);
  return {
    id,
    // Transient defs are not catalog cards, so no random pool or Discover can reach one (§5.1);
    // the index is the id itself, which keeps `defByIndex` unambiguous.
    index: id,
    name: defs.map((def) => def.name).join(" + "),
    set: targetDef?.set ?? defs[0]?.set ?? "Core",
    type: fusedType(defs, targetDef),
    tags: unionTags(defs),
    rarity: rarestOf(defs),
    // A fusion is a real card unless every ingredient was a token, so fusing a token onto a unit
    // gives a result that no longer ceases to exist off the field (R11).
    token: defs.every((def) => def.token),
    // R470: a kept X-cost or embiggen card that keeps its cost keeps that printed form.
    cost: keptCost?.form ?? fusedCost(state, ingredients),
    ...(refs === null ? {} : { refs }),
    ...(loc === null ? {} : { loc }),
    // R179, R468: the list the id names, kept on the definition so the scripts can be rebuilt from
    // it even when the id is only a digest of it.
    ingredients: specs,
    base: fusedFace(ingredients, defs, false, forced),
    radiant: fusedFace(ingredients, defs, true, forced),
  };
}

/**
 * R470: what "its cost doesn't change" writes, read off the kept card before it becomes the fusion:
 * nothing when it already has a `costOverride` (which the kept instance keeps); for an X-cost or
 * embiggen card with no cost hook, that printed form (`form`), which the fused definition then
 * prints; otherwise its own cost as it stands (`override`, R65's printed cost with a hook's computed
 * one), which becomes its `costOverride`. `costMod` is the instance's and stays, so the card's cost
 * after the fusion is the one it had before it.
 */
type KeptCost = { form?: CardCost; override?: number };

function keptCostOf(state: GameState, kept: CardInstance): KeptCost {
  if (kept.costOverride !== undefined) return {};
  const printed = defOf(state, kept.defId).cost;
  if (typeof printed !== "number" && scriptOf(kept).cost === undefined) return { form: printed };
  return { override: printedCost(state, kept) };
}

// ---------------------------------------------------------------------------
// The concatenated scripts (R77).
// ---------------------------------------------------------------------------

/** Every hook in `script.ts` is "context in, list out", which is what makes concatenation total. */
type ListFn = (...args: unknown[]) => unknown[];

/**
 * The script keys whose functions return something other than effects, so they combine by building
 * every ingredient's list at once: an aura's entries are read off the field on every stat read
 * (§10.4), and there is no "when the list reaches it" for them. `preview` (R280) is the other: the
 * labelled numbers `viewFor` shows, a pure read with nothing to resolve, so a fusion's list is its
 * ingredients' lists in ingredient order, each asked with the fused card's own context (the fused
 * instance as `self`, the face it runs, the zone it is asked about), as R196 asks `conditionMet` —
 * and each label still sits in the fused face's text, which prints every ingredient's text whole.
 * `drawLimit` (B5 E3, R457) is a third: the limits a card sets while it acts, read on every draw, so a
 * fusion sets every ingredient's limit and the lowest holds.
 */
// B5 E6, E35: `heroGuard` and `conditionalKeywords` are pure reads returning lists too, so a fusion
// guards its hero with every ingredient's guard and has every ingredient's conditional keywords.
const EAGER_KEYS: readonly string[] = [
  "aura",
  "preview",
  // B5 E3 (R457): a draw limit is a pure read of the field.
  "drawLimit",
  // B5 E15, E11 (R455, R454): a price rule and a graveyard permission are pure reads of the field too.
  "costAura",
  "graveyardPlay",
  "heroGuard",
  "conditionalKeywords",
];

/**
 * §10.4 layer 5: each ingredient's aura, reading "this" as the fused card at the price that
 * ingredient was played for (R102, `scripts.asIngredient`): #46 Suppressive Aura played for 4 and
 * fused onto a Mana Well is still "paid 4: −5/−5", though the kept instance's own price is the Mana
 * Well's.
 */
function fusedAura(faces: readonly Face[]): AuraHook | undefined {
  const hooks = faces.map((face) => face.script.aura);
  if (hooks.every((hook) => hook === undefined)) return undefined;
  return (ctx) =>
    hooks.flatMap((hook, index) => (hook === undefined ? [] : hook({ ...ctx, self: asIngredient(ctx.self, index) })));
}

/**
 * The static flags that are an amount of what the text does, not a quality the card has: #79's
 * "the next Spell you play gains Echo +1", #38's granted Combo and #84's hero Armor. R102: a card
 * fused from two such texts carries both, so its amount is theirs added — a Twinspell fused onto a
 * Twinspell grants Echo +2, and a Going Long onto a Going Long gives Armor twice, as two standing
 * apart do (R124) — where a quality (`castOnDraw`, `immutable`) is had once and a requirement
 * (`tribute`) takes the stricter. A `true` is one.
 */
const SUMMED_FLAGS: readonly string[] = ["echoGrant", "quickstriker", "heroArmor"];

/**
 * The context ingredient `index`'s text builds and applies with (R102): its place in the fusion
 * appended to the path the combined hooks above it have used (`work.PART_KEY`).
 */
function partData(data: Record<string, unknown>, index: number): Record<string, unknown> {
  const depth = typeof data[PART_DEPTH_KEY] === "number" ? (data[PART_DEPTH_KEY] as number) : 0;
  const path = (partPathOf(data) ?? []).slice(0, depth);
  return { [PART_KEY]: [...path, index], [PART_DEPTH_KEY]: depth + 1 };
}

/**
 * The context an ingredient's text runs in: its place in the fusion (`partData`'s patch), and the
 * price its card was played for as `embiggened` (R102, `scripts.ingredientPaid`), found by that
 * place's path — #59's trigger reads it.
 */
function inPlace(ctx: EffectContext, patch: Record<string, unknown>): EffectContext {
  const path = partPathOf(patch) ?? [];
  const embiggened = ctx.self === null ? ctx.embiggened : ingredientPaid(ctx.self, path);
  return { ...ctx, embiggened, data: { ...ctx.data, ...patch } };
}

/** An effect that applies, and builds any part of its own, with its ingredient's place (R102). */
function inIngredient(effect: Effect, patch: Record<string, unknown>): Effect {
  const expand = effect.expand;
  return {
    ...effect,
    apply: (ctx) => effect.apply(inPlace(ctx, patch)),
    ...(expand === undefined
      ? {}
      : {
          expand: (ctx, memo) => {
            const built = expand(inPlace(ctx, patch), memo);
            return { ...built, effects: built.effects.map((inner) => inIngredient(inner, patch)) };
          },
        }),
  };
}

/**
 * One ingredient's list as a part of the combined list (`resolve.lazyPart`, R102): built when the
 * list reaches it, with the ingredient's place in its context, and every effect of it applied there.
 */
function ingredientPart(index: number, build: (ctx: EffectContext) => readonly Effect[]): Effect {
  return lazyPart(`fused:part${index}`, (at) => {
    const patch = partData(at.data, index);
    const effects = build(inPlace(at, patch));
    // The part needs nothing handed to its rebuild; `null` says so in a form JSON keeps, where an
    // absent memo would sit in a paused list's `memo` array as `undefined` and come back as `null`,
    // so the paused state and its JSON round trip would differ (§9.3).
    return { effects: effects.map((effect) => inIngredient(effect, patch)), memo: null };
  });
}

/**
 * A combined hook: each ingredient's list in turn, as parts (R102, R113). A continuation one
 * ingredient's text left — the step its prompt re-enters, the delayed effect it scheduled — names
 * that ingredient (`work.PART_KEY`, which `prompts.resumeSelf` carries in the card's data), and comes
 * back to its list alone: the answer to one Masochism Mask's "choose one" is that Mask's pick, not a
 * pick for every ingredient that names its step the same.
 */
function combinedHook(fns: readonly (ListFn | undefined)[], step = false): (ctx: EffectContext) => Effect[] {
  return (ctx) => {
    const depth = typeof ctx.data[PART_DEPTH_KEY] === "number" ? (ctx.data[PART_DEPTH_KEY] as number) : 0;
    // A step of the `resume` table is one continuation of one text, so one that names no part — the
    // engine left it for the card as a whole, not one of its texts: the prompt of the power R43
    // activates once (`heroPower.activatePower`) — comes back to the first ingredient that has the
    // step, once, rather than to every ingredient that names its step the same (R43, R102).
    const routed = partPathOf(ctx.data)?.[depth] ?? (step ? fns.findIndex((fn) => fn !== undefined) : undefined);
    const indices = fns.flatMap((fn, index) =>
      fn === undefined || (routed !== undefined && routed !== index) ? [] : [index],
    );
    return indices.map((index) =>
      ingredientPart(index, (built) => (fns[index] as ListFn)(built) as Effect[]),
    );
  };
}

/**
 * One named target predicate across the ingredients that define it (§10.6, R102). It is not a hook:
 * a declaration's filter asks it with the candidate and gets a boolean back, in a context that
 * carries no instance data to route a part by, so wrapping it as a fused Cry threw on the first ask
 * and would have answered with a list of effects had it not. One ingredient's predicate stays as it
 * is; ingredients that name the same predicate must each admit the candidate, the one stricter
 * requirement that flags and numbers also take when they combine.
 */
function combinedChecks(checks: readonly TargetCheck[]): TargetCheck {
  const [only] = checks;
  if (checks.length === 1 && only !== undefined) return only;
  return (args) => checks.every((check) => check(args));
}

/**
 * Combine one key of several scripts, `values` aligned with the ingredients (undefined where one has
 * none). The rule is the same for every kind of value a script holds, which is what keeps this
 * working as `Script` grows new hooks:
 *   - a hook (Cry, Death, a start/end-of-turn hook, a resume step, a delayed hook) becomes one hook
 *     that runs each of them in turn, so "both Cry and Death lists run" (R77) — each ingredient's
 *     list built only when the one before it has resolved (`lazyPart`, R102), so a later ingredient
 *     reads the board the earlier ones left: #68's "8 if your hero is below 10" after Reno has set
 *     the hero to 30, #22's meal after #100 has exiled it — and a continuation one of them left comes
 *     back to that one alone (`combinedHook`). An aura, which returns no effects, runs each at once;
 *   - a list (triggers, declared targets and modes) becomes the lists in ingredient order, so a
 *     fused trap carries every ingredient's trigger condition;
 *   - a nested object (static flags, the resume table) is combined key by key by the same rules;
 *   - a flag is true when any ingredient set it, and a number takes the larger, which is the one
 *     stricter requirement rather than a doubled one (`staticFlags.tribute`) — except an amount of
 *     what the text does, which adds up (`SUMMED_FLAGS`).
 */
function combineValues(values: readonly unknown[], key = "", parent = ""): unknown {
  const defined = values.filter((value) => value !== undefined);
  if (defined.length === 0) return undefined;
  if (parent === TARGET_CHECKS_KEY && defined.every((value) => typeof value === "function")) {
    return combinedChecks(defined as TargetCheck[]);
  }
  if (defined.every((value) => typeof value === "function")) {
    const fns = values.map((value) => (typeof value === "function" ? (value as ListFn) : undefined));
    if (EAGER_KEYS.includes(key)) {
      if (defined.length === 1) return defined[0];
      return (...args: unknown[]): unknown[] => fns.flatMap((fn) => (fn === undefined ? [] : fn(...args)));
    }
    return combinedHook(fns, parent === RESUME_KEY);
  }
  if (SUMMED_FLAGS.includes(key) && defined.every((value) => typeof value === "number" || typeof value === "boolean")) {
    return defined.reduce<number>((sum, value) => sum + (value === true ? 1 : typeof value === "number" ? value : 0), 0);
  }
  // R102: the table of steps is combined even when one ingredient holds it, so every step of it is
  // wrapped at this fusion's level and a part path (`work.PART_KEY`) counts each level of a nesting,
  // as `param` walks it.
  if (key === RESUME_KEY && defined.every((value) => isPlainObject(value))) {
    return combineObjects(
      values.map((value) => (isPlainObject(value) ? value : undefined)),
      key,
    );
  }
  if (defined.length === 1) return defined[0];
  if (defined.every((value) => Array.isArray(value))) return (defined as unknown[][]).flat();
  if (defined.every((value) => typeof value === "boolean")) return defined.some((value) => value === true);
  if (defined.every((value) => typeof value === "number")) return Math.max(...(defined as number[]));
  if (defined.every((value) => isPlainObject(value))) {
    return combineObjects(
      values.map((value) => (isPlainObject(value) ? value : undefined)),
      key,
    );
  }
  // Nothing in `Script` mixes kinds under one key; the last ingredient wins if one ever does.
  return defined[defined.length - 1];
}

function combineObjects(objects: readonly (Record<string, unknown> | undefined)[], parent = ""): Record<string, unknown> {
  const keys = [...new Set(objects.flatMap((object) => (object === undefined ? [] : Object.keys(object))))];
  const out: Record<string, unknown> = {};
  for (const key of keys) {
    const value = combineValues(
      objects.map((object) => object?.[key]),
      key,
      parent,
    );
    if (value !== undefined) out[key] = value;
  }
  return out;
}

/**
 * One ingredient's script, ready to be combined: without its `cost` hook, because R77 fixes the
 * fused cost at min(sum, 4) and a surviving Ceaseless Void hook would overrule it (R65); and with
 * its trigger ids namespaced, so two ingredients that both call a trigger "turn-end" stay two
 * distinct conditions on the fused card — each running in its ingredient's place (R102), so a
 * question it asks comes back to its own step.
 */
function scriptRecord(script: Script, defId: string, index: number): Record<string, unknown> {
  const out: Record<string, unknown> = { ...script };
  delete out[COST_KEY];
  delete out[SET_STAT_KEY];
  delete out[CONDITION_MET_KEY];
  delete out[TRIBUTE_WHEN_KEY];
  delete out[PLAGUE_MULTIPLIER_KEY];
  delete out[WOULD_COUNTER_KEY];
  for (const key of TRIGGER_KEYS) {
    const list = out[key];
    if (!Array.isArray(list)) continue;
    out[key] = list.map((trigger) =>
      isPlainObject(trigger) && typeof trigger.id === "string"
        ? { ...trigger, id: `${defId}:${trigger.id}`, run: inTriggerIngredient(trigger as unknown as TriggerDef, index) }
        : trigger,
    );
  }
  return out;
}

/** A trigger's list, built and applied in its ingredient's place (R102). */
function inTriggerIngredient(trigger: TriggerDef, index: number): TriggerDef["run"] {
  return (ctx) => {
    const patch = partData(ctx.data, index);
    return trigger.run({ ...inPlace(ctx, patch), event: ctx.event }).map((effect) => inIngredient(effect, patch));
  };
}

/**
 * §10.4 layer 2 (#92 Felinor Fiender): a card that sets its own stats from the board. R77 sums
 * printed stats, so a fusion of two set-stat cards sums what they set, and a stat only one of them
 * sets is that one's.
 */
function fusedSetStat(scripts: readonly Script[]): Script["setStat"] | undefined {
  const fns = scripts.flatMap((script) => (script.setStat === undefined ? [] : [script.setStat]));
  if (fns.length === 0) return undefined;
  if (fns.length === 1) return fns[0];
  return (args) => {
    let attack: number | undefined;
    let maxHealth: number | undefined;
    for (const fn of fns) {
      const set = fn(args);
      if (set.attack !== undefined) attack = (attack ?? 0) + set.attack;
      if (set.maxHealth !== undefined) maxHealth = (maxHealth ?? 0) + set.maxHealth;
    }
    return {
      ...(attack === undefined ? {} : { attack }),
      ...(maxHealth === undefined ? {} : { maxHealth }),
    };
  };
}

/**
 * R471: "Plague Tokens placed on this are doubled" (Classic #27). Each ingredient's text multiplies
 * what is placed on the fused card, so two such texts multiply: a Pestilent Slime fused onto a
 * Pestilent Slime quadruples, as two doublings in a row do. Each hook is asked about the fused card
 * at its own ingredient's price (`scripts.asIngredient`, R102).
 */
function fusedPlagueMultiplier(scripts: readonly Script[]): Script["plagueMultiplier"] | undefined {
  const hooks = scripts.map((script) => script.plagueMultiplier);
  if (hooks.every((hook) => hook === undefined)) return undefined;
  return (args) =>
    hooks.reduce(
      (product, hook, index) =>
        hook === undefined ? product : product * hook({ ...args, self: asIngredient(args.self, index) }),
      1,
    );
}

type Face = { defId: string; script: Script };

/** An effect that resolves with one ingredient's own play choices, whatever context applies it. */
function withChoices(effect: Effect, targets: readonly Selection[], modes: readonly string[]): Effect {
  return { ...effect, apply: (ctx) => effect.apply({ ...ctx, targets: [...targets], modes: [...modes] }) };
}

/**
 * R102 concatenates the ingredients' declared targets and modes in ingredient order, and R90 reads
 * that flat list declaration by declaration — so each ingredient's Cry must resolve with its OWN
 * slice of the play's choices, not the whole list. Handed the whole list, every ingredient read its
 * first slot: a crafted Bigot + Twisted Sorcerer aimed the Sorcerer's 4 damage at the unit Bigot
 * destroyed, and an Archivist + Silly Silas rotated by Archivist's "highest". Modes split by each
 * ingredient's count of mode declarations; targets split by R90's rule over the declarations that
 * the ingredient's own modes make active (`forModes`) — with the lengths §10.5 step 1 read the play
 * with, which the pipeline hands over in `data` (`DECLARATION_SLICES_KEY`), because the board at
 * resolution is not the one the play was checked against: by step 5 a crafted Postdoc + Sorcerer
 * stands on the field and is itself a Human the Postdoc's declaration could take. Only a Cry run
 * with no play behind it measures against the board as it stands.
 *
 * Every effect an ingredient's Cry returns is bound to that slice, because an effect reads
 * `ctx.targets` when it applies, and the context applying it is the fused card's. And each
 * ingredient's Cry is its own part of the list (`resolve.lazyPart`), built when the list reaches it,
 * so it reads the board the ingredients before it left (R102), and a pause inside it resumes into
 * the rest of that part and then every part after it (`prompts.applyResumable`, R113).
 */
function fusedCry(faces: readonly Face[]): Hook | undefined {
  if (!faces.some((face) => face.script.cry !== undefined)) return undefined;
  return (ctx) => {
    const modesOf: string[][] = [];
    let modeAt = 0;
    for (const face of faces) {
      const count = face.script.modes?.length ?? 0;
      modesOf.push(ctx.modes.slice(modeAt, modeAt + count));
      modeAt += count;
    }
    const declsOf = faces.map((face, index) => activeTargetDecls(face.script.targets ?? [], modesOf[index] ?? []));
    const decls = declsOf.flat();
    const stored = storedDeclarationSlices(ctx.data);
    const slices =
      stored !== null && stored.length === decls.length
        ? cutSlices(ctx.targets, stored)
        : ctx.self === null
          ? decls.map(() => [])
          : selectionsPerDeclaration(ctx.state, ctx.controller, ctx.self, decls, ctx.targets);

    // Each ingredient's Cry is built as the list reaches it (`lazyPart`), so it reads the board the
    // ingredients before it left (R102); its slice of the play's choices is fixed now, as step 1
    // checked them.
    let declAt = 0;
    return faces.map((face, index) => {
      const count = declsOf[index]?.length ?? 0;
      const targets = slices.slice(declAt, declAt + count).flat();
      declAt += count;
      const cry = face.script.cry;
      const modes = modesOf[index] ?? [];
      return ingredientPart(index, (at) =>
        cry === undefined ? [] : cry({ ...at, targets, modes }).map((effect) => withChoices(effect, targets, modes)),
      );
    });
  };
}

/** The flat list cut into consecutive slices of these lengths; the last takes the remainder (R90). */
function cutSlices(selections: readonly Selection[], lengths: readonly number[]): Selection[][] {
  let at = 0;
  return lengths.map((length, index) => {
    const slice = index === lengths.length - 1 ? selections.slice(at) : selections.slice(at, at + length);
    at += slice.length;
    return slice;
  });
}

/**
 * R196: a fusion's yellow glow. Its Cry, Death and triggers run every ingredient's list, so each
 * ingredient's printed condition still picks its own branch when the fused card resolves, and the
 * fused card glows when any of them holds. Each hook is asked with the fused card's own context
 * (the fused instance as `self`, the face it runs), and only an answer of exactly `true` counts,
 * as `conditionActive` counts it. One hooked ingredient's hook is the fusion's unchanged.
 */
function fusedConditionMet(scripts: readonly Script[]): Script["conditionMet"] | undefined {
  const hooks = scripts.flatMap((script) => (script.conditionMet === undefined ? [] : [script.conditionMet]));
  if (hooks.length === 0) return undefined;
  if (hooks.length === 1) return hooks[0];
  return (ctx) => hooks.some((hook) => hook(ctx) === true);
}

/** R403, R102: a fusion carries every ingredient's "When …, Tribute this", so any one that holds takes it. */
function fusedTributeWhen(scripts: readonly Script[]): Script["tributeWhen"] | undefined {
  const hooks = scripts.flatMap((script) => (script.tributeWhen === undefined ? [] : [script.tributeWhen]));
  if (hooks.length <= 1) return hooks[0];
  return (args) => hooks.some((hook) => hook(args));
}

/** R658, R102: a fusion carries every ingredient's counter trigger, so it would counter what any of them would. */
function fusedWouldCounter(scripts: readonly Script[]): Script["wouldCounter"] | undefined {
  const hooks = scripts.flatMap((script) => (script.wouldCounter === undefined ? [] : [script.wouldCounter]));
  if (hooks.length <= 1) return hooks[0];
  return (args) => hooks.some((hook) => hook(args));
}

/**
 * One form's script of a fusion: each ingredient's script on that form — or on its Radiant form
 * whichever form this is, for an ingredient that went in on it (R469) — combined member by member.
 */
function fusedScript(specs: readonly FusedIngredient[], radiant: boolean): Script {
  const faces: Face[] = specs.map((spec) => {
    const pair = scriptsFor(spec.defId);
    return { defId: spec.defId, script: radiant || spec.radiant === true ? pair.radiant : pair.base };
  });
  const scripts = faces.map((face) => face.script);
  const combined = combineObjects(
    faces.map((face, index) => scriptRecord(face.script, face.defId, index)),
  ) as Script;
  const setStat = fusedSetStat(scripts);
  const conditionMet = fusedConditionMet(scripts);
  const cry = fusedCry(faces);
  const aura = fusedAura(faces);
  const plagueMultiplier = fusedPlagueMultiplier(scripts);
  const tributeWhen = fusedTributeWhen(scripts);
  const wouldCounter = fusedWouldCounter(scripts);
  return {
    ...combined,
    ...(tributeWhen === undefined ? {} : { tributeWhen }),
    ...(wouldCounter === undefined ? {} : { wouldCounter }),
    ...(setStat === undefined ? {} : { setStat }),
    ...(conditionMet === undefined ? {} : { conditionMet }),
    ...(aura === undefined ? {} : { aura }),
    ...(cry === undefined ? {} : { cry }),
    ...(plagueMultiplier === undefined ? {} : { plagueMultiplier }),
  };
}

// ---------------------------------------------------------------------------
// Keeping the process's registry in step with the state being run.
// ---------------------------------------------------------------------------

/**
 * R179: register the scripts a fused id names, and those of every fused ingredient it names, unless
 * the registry already holds them. An entry that is there is right whoever wrote it, since the id
 * names its scripts; one that is missing is rebuilt from the id. `seen` stops a malformed id that
 * names itself.
 */
function ensureFused(defId: string, seen: ReadonlySet<string> = new Set()): void {
  if (registeredScripts()[defId] !== undefined || seen.has(defId)) return;
  const from = fusedIngredientSpecs(defId);
  if (from === null) return;
  const inside = new Set([...seen, defId]);
  for (const ingredient of from) ensureFused(ingredient.defId, inside);
  const scripts: CardScripts = { base: fusedScript(from, false), radiant: fusedScript(from, true) };
  registerScripts({ ...registeredScripts(), [defId]: scripts });
}

/**
 * Register any of this state's fused scripts the process's registry lacks (§9.3, R77, R179): a state
 * that came through JSON into a process that never ran its Fuse, or a registry that was replaced
 * wholesale. Cheap when nothing is missing: a state with no transient defs does nothing, and a
 * registered id is left alone. R468: a digest id's list is read off its definition first
 * (`CardDef.ingredients`), every definition's before any script is rebuilt, so a digest that names
 * another digest finds both.
 */
export function syncFusedScripts(state: GameState): void {
  const defs = Object.values(state.transientDefs);
  for (const def of defs) {
    if (def.ingredients !== undefined) registerFusedIngredients(def.id, def.ingredients);
  }
  for (const def of defs) ensureFused(def.id);
}

/**
 * R179, R417, R564: the definition `defId` names in this state — a catalog card's, one the state
 * already holds, or a fused one rebuilt from the id alone into `transientDefs` (its fused
 * ingredients first), as `syncFusedScripts` rebuilds the scripts. No instance stands behind it, so
 * each ingredient is worn as printed and priced as R65 reads it out of play, with no target on the
 * field (R77's shared type). Null for an id that cannot be rebuilt: a digest
 * (R468), a bare `t-<n>`, an ingredient the catalog lacks. C+ #29 brings fused cards back this way.
 */
export function rebuildFusedDef(state: GameState, defId: string, owner: PlayerId): CardDef | null {
  const known = findDef(state, defId);
  if (known !== undefined) return known;
  const specs = isDigestId(defId) ? null : fusedIngredientSpecs(defId);
  if (specs === null) return null;
  const defs: CardDef[] = [];
  for (const spec of specs) {
    const def = rebuildFusedDef(state, spec.defId, owner);
    if (def === null) return null;
    defs.push(def);
  }
  // Instances in no pile, numbered off a scratch counter so the state's ids are untouched.
  const scratch = { nextId: 0 };
  const ingredients = defs.map((def) => newInstance(scratch, def.id, owner, { z: "gone", player: owner }));
  const def = buildDef(state, ingredients, defs, null, specs.map((spec) => spec.radiant === true), null, defId);
  state.transientDefs[defId] = def;
  ensureFused(defId);
  return def;
}

// ---------------------------------------------------------------------------
// The result.
// ---------------------------------------------------------------------------

/**
 * R77's keep-the-instance path. The fused card *is* the target: only its def id, its buffs (the sum
 * of every ingredient's) and its granted keywords (their union) change, and the rest of the
 * instance — zone, position, damage, exertion, summonedTurn, counters, memory, the radiant flag and
 * the Vanilla flag — is left exactly as it was. A token's `statsOverride` and `armorOverride` are one
 * exception: they were its printed face (§7, R175), which `wornFace` has already summed into the
 * fused definition, so they leave the instance with it — kept, §10.4's layer 1 would read them in
 * place of the fused face and a 3/3 Bread Token fused with a 7/7 would still be a 3/3. The other is
 * R77's own: the memory gains the ingredients' prices (`scripts.INGREDIENTS_KEY`) when they were not
 * all the kept card's, so each ingredient's text reads its own (R102).
 */
function keepInstance(
  state: GameState,
  def: CardDef,
  ingredients: readonly CardInstance[],
  kept: CardInstance,
): CardInstance {
  const attack = ingredients.reduce((sum, card) => sum + card.buffs.attack, 0);
  const health = ingredients.reduce((sum, card) => sum + card.buffs.health, 0);
  const granted = unionKeywords(ingredients.flatMap((card) => card.grantedKeywords));
  const before = defOf(state, kept.defId);

  // R102: the price each ingredient's text reads as its own, recorded before any of them ceases to
  // exist and only when they are not all the kept card's (`scripts.INGREDIENTS_KEY`).
  const record = ingredientRecord(kept, ingredients);
  // R77, R102: the kept card's texts become ingredient `index` of the fusion, and what they
  // remembered moves with them to the path they now run at (`work.rerootRemembered`).
  rerootRemembered(kept.memory, ingredients.findIndex((card) => card.id === kept.id));
  gainPrintedKeywords(kept, before, def);
  kept.defId = def.id;
  kept.buffs = { attack, health };
  kept.grantedKeywords = granted;
  // R102, B3.4 rule 4, R443: the fused card sums its ingredients' tuning and carries all their
  // enchantments.
  carryInstanceData(kept, ingredients);
  if (record === null) delete kept.memory[INGREDIENTS_KEY];
  else kept.memory[INGREDIENTS_KEY] = record;
  delete kept.statsOverride;
  delete kept.armorOverride;

  // R77 and R86: an ingredient that is not kept ceases to exist — no graveyard, no exile pile, no
  // Death trigger and nothing destroyed — and one that stood on the field has left it (R174).
  for (const card of ingredients) {
    if (card.id === kept.id) continue;
    ceaseToExist(state, card);
  }
  return kept;
}

/**
 * §5.2: "newly gained keywords apply at once". The fused face can print a keyword the kept card's
 * own face did not — Jilliax's Divine Shield fused onto a K-Pop Fanatic, a Radiant Saintess's Reborn
 * onto a unit that came back through a granted one — and the card gains it with the new text, so a
 * shield or a Reborn the card had spent is up again, as radiant #50's printed shield is after a
 * granted one was spent (`effects/radiant.ts`). A keyword the kept face already printed is not newly
 * gained, and stays spent. A Vanilla unit prints nothing on either face (§6.3).
 */
function gainPrintedKeywords(kept: CardInstance, before: CardDef, after: CardDef): void {
  if (kept.vanilla) return;
  const prints = (def: CardDef, kind: string): boolean =>
    (kept.radiant ? def.radiant : def.base).keywords.some((keyword) => keyword.kind === kind);
  if (prints(after, "Divine Shield") && !prints(before, "Divine Shield")) delete kept.divineShieldSpent;
  if (prints(after, "Reborn") && !prints(before, "Reborn")) delete kept.rebornSpent;
}

/**
 * R102, B3.4 rule 4, R443: what a fusion's card carries of its ingredients' instance data — their
 * tuning summed (`tuning.sumTunings`) and their enchantments united. A Brittle count is a counter,
 * which a Fuse keeps only on the kept card as it keeps its other counters (R77).
 */
function carryInstanceData(card: CardInstance, ingredients: readonly CardInstance[]): void {
  const tuning = sumTunings(ingredients.map((ingredient) => ingredient.tuning));
  if (tuning === undefined) delete card.tuning;
  else card.tuning = tuning;
  const enchantments = unitedEnchantments(ingredients);
  if (enchantments === undefined) delete card.enchantments;
  else card.enchantments = enchantments;
}

/**
 * R77's Craft a Card path: "a fresh, non-Radiant hand card with `costOverride` 0". The ingredients
 * went into it, so they cease to exist here too — #99's are Discovered definitions that were never
 * cards on a board, and for anything else a consumed ingredient is what a fusion means.
 * A full hand burns the result like any other card reaching it (§2.4, R4), and the 0 goes with the
 * hand: §8 #99's "the result costs 0 and goes to your hand" is a price for the card in that hand,
 * the reading `addToHand`'s cost riders have (R4), so a burned result is an ordinary graveyard card
 * that R78 would otherwise carry the price for into every later zone (a Reminisce, a Gravedigger).
 */
function craftInHand(
  sink: EngineSink,
  def: CardDef,
  player: PlayerId,
  ingredients: readonly CardInstance[],
  terms: { handPrice: HandPrice; radiant: boolean },
): CardInstance {
  const card = newInstance(sink.state, def.id, player, { z: "hand", player });
  // R352: radiant Stitching's result is Radiant as it is made, so it reaches the hand on that face.
  if (terms.radiant) card.radiant = true;
  // R102, B3.4 rule 4, R443: as `keepInstance`'s.
  carryInstanceData(card, ingredients);
  for (const ingredient of ingredients) ceaseToExist(sink.state, ingredient);
  if (addToHand(sink, card) === "hand" && terms.handPrice === "free") card.costOverride = CRAFTED_CARD_COST;
  return card;
}

/**
 * §6.3 Fuse per R77. Returns the fused card — the kept target instance, the kept hand or library
 * card (R470), or the crafted hand card — or null when the fusion cannot happen, in which case
 * nothing has changed.
 *
 * It does not happen when there are fewer than two ingredients, when a named target is not on the
 * field or a card named `into` is not in a hand or a library, when the kept card is Immutable (R23:
 * an Immutable permanent is never chosen as a Fuse target, and R61 has the trap fire and do
 * nothing), or when the call names neither a kept card nor a hand to craft into. A target the caller did not also list as an ingredient is one anyway, so #85
 * may name the played card and its victim separately.
 *
 * An ingredient only ever contributes its definition, so an ingredient that already ceased to exist
 * in an earlier fusion still fuses: that is what lets radiant #85 fuse the played permanent "onto
 * each matching permanent separately, one fusion at a time" (R77), each fusion its own transient
 * definition, without the card having to rebuild the permanent it consumed.
 */
export function fuse(sink: EngineSink, args: FuseArgs): CardInstance | null {
  const state = sink.state;
  // R470: the kept instance — R77's target on the field, or a hand or library card (`into`).
  const target = args.target ?? args.into ?? null;
  const toHand = args.toHand;

  const ingredients: CardInstance[] = [];
  for (const card of [...args.ingredients, ...(target === null ? [] : [target])]) {
    if (!ingredients.some((seen) => seen.id === card.id)) ingredients.push(card);
  }
  if (ingredients.length < FUSE_MIN_INGREDIENTS) return null;

  if (target !== null) {
    const zone = target.zone.z;
    const allowed = args.target !== undefined ? zone === "field" : zone === "hand" || zone === "library";
    if (!allowed) return null;
    if (unitHas(state, target, "Immutable")) return null;
  } else if (toHand === undefined) {
    return null;
  }

  const defs = ingredients.map((card) => defOf(state, card.defId));
  const targetDef = target === null ? null : defOf(state, target.defId);
  const radiantIds = new Set(args.radiantIngredients ?? []);
  const forced = ingredients.map((card) => radiantIds.has(card.id));
  // R470: read before the kept card becomes the fusion, whose own cost is R77's.
  const keptCost = args.keepCost === true && target !== null ? keptCostOf(state, target) : null;
  const def = buildDef(state, ingredients, defs, targetDef, forced, keptCost);

  // The def is match state; its id names the scripts, which join the process's registry (R179), and
  // a digest id's list joins the process's table of them (R468).
  state.transientDefs[def.id] = def;
  registerFusedIngredients(def.id, def.ingredients ?? []);
  ensureFused(def.id);

  let result: CardInstance;
  if (target !== null) {
    result = keepInstance(state, def, ingredients, target);
    if (keptCost?.override !== undefined) result.costOverride = keptCost.override;
    // R43, R151: the kept card now carries every ingredient's text, a #98 Heroic Power's included —
    // and "one created later rolls when it is created". The ingredient's rolled power ceased to exist
    // with it, and the kept instance's memory is the target's (R77), so without the roll the card
    // would carry "Once per turn, spend X" and no power for as long as it stood. A card that already
    // has its power keeps it (`heroPower.ensurePower`). A crafted card rolls as it reaches the hand.
    runStartOfGame(sink, result, result.controller);
  } else if (toHand !== undefined) {
    result = craftInHand(sink, def, toHand, ingredients, {
      handPrice: args.handPrice ?? "free",
      // R469: with no kept card and every ingredient Radiant, the result is Radiant (Classic+ #43).
      radiant: args.radiant === true || forced.every((isRadiant) => isRadiant),
    });
  } else {
    return null;
  }

  sink.events.push({
    type: "fused",
    instanceIds: ingredients.map((card) => card.id),
    resultInstanceId: result.id,
    defId: def.id,
  });
  return result;
}
