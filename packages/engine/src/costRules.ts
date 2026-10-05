// Cost rules (docs/classic-sets.md B5 E15, SPEC §6.3 Cost, R65, R363, R455): the rungs a card's price
// climbs past on its way through `mana.effectiveCost`, where they come from, and the plays a price
// rule forbids outright.
//
// A price rule comes from one of two places, and both carry the same data (`CostRule`):
//
//  - a player modifier (`kind: "costRule"`, `state.ts`) — Classic #2 The Trickster's "your next Trap
//    or Field Spell costs (2) less" (or "costs (0)", its Radiant face), until used; AI Alignment
//    Tax's "your opponent's cards cost (1) more during their next turn", R48's timing turned outward;
//  - an aura (`Script.costAura`) on a card acting on the field — Classic #6's "your Traps cost (0)",
//    #68's "Cost (3)+ cards cost (1) more" and its Radiant "your opponent can't play Cost (3)+ cards",
//    #77's "Spells cost (1) more". The aura is a hook, not a static flag, so a Degrade or Upgrade of
//    the card's declared numbers (B3.4, `CardDef.params`) moves the rule it lays down.
//
// The ladder itself (R455) is `mana.effectiveCost`'s, which reads these through `priceRulesFor`:
// flat changes first (R65's player discounts, then every rule with an `amount` and no threshold),
// then the threshold rules, each testing the one number the flat changes left (R363 reads Professor
// Curvature there, and #68's "(3)+" reads the same number, before its own surcharge), then "costs
// (N)" sets, then the floor a card carries (Classic+ #14 Forever&'s "can't cost less than (2)",
// `enchantments.ts`), then 0. A ban ("can't play") reads the finished price and refuses the play
// (`whyPlayBanned`); a cast pays nothing and is never refused (R70).
//
// The module imports nothing from `mana.ts`, which imports it: liveness of a modifier is the caller's
// question (`mana.modifierIsLive`), so the two never form a cycle.

import type { CardType, Enchantment, PlayerId } from "@jackioh/shared";
import { PLAYER_IDS, opponentOf } from "@jackioh/shared";
import { GLITCH_DEF_ID } from "./config";
import { enchantmentsOfKind } from "./enchantments";
import { cardTypeOf } from "./faces";
import { scriptOf } from "./scripts";
import type { CardInstance, GameState, ModifierExpiry, PlayerModifier } from "./state";
import { cardAt, slotsOf } from "./zones";

/**
 * One rung of a price (E15). `types` names the card types it reaches, all of them when absent — a
 * card's type is its running face's (B2.7, `faces.cardTypeOf`), and a text that says "Trap" names
 * "Field Trap" too, which the card's own list spells out. `minCost` makes it a threshold rule, "Cost
 * (N)+ cards", tested against the price the flat rungs left (R363). `amount` is added to the price
 * (below 0 a discount, above 0 a surcharge); `setTo` is "costs (N)", which wins over every add.
 */
export type CostRule = {
  types?: CardType[];
  minCost?: number;
  amount?: number;
  setTo?: number;
};

/** Whose cards an aura reaches, seen from the aura card's controller. */
export type CostAuraWhose = "yours" | "opponents" | "all";

/**
 * E15: what a card on the field lays on prices while it acts there (`Script.costAura`). A rule with
 * `ban: true` is "can't play" — it prices nothing and refuses a play of a card it reaches whose price
 * is `minCost` or more (Classic #68 Radiant), and never a cast (R70).
 */
export type CostAura = CostRule & { whose: CostAuraWhose; ban?: boolean };

/** The argument of `Script.costAura`, as `Script.aura`'s is: a pure read of the board. */
export type CostAuraArgs = { state: GameState; self: CardInstance; radiant: boolean };

/** A `costRule` modifier (`state.ts`), named so the readers here can take one. */
export type CostRuleModifier = Extract<PlayerModifier, { kind: "costRule" }>;

// ---------------------------------------------------------------------------
// Matching
// ---------------------------------------------------------------------------

/** Whether a rule's type list reaches this card (all types when it names none). */
export function ruleReaches(state: GameState, rule: Pick<CostRule, "types">, card: CardInstance): boolean {
  if (rule.types === undefined || rule.types.length === 0) return true;
  return rule.types.includes(cardTypeOf(state, card));
}

/** Whether an aura laid by a card `source` controls reaches a card `player` would play. */
function whoseReaches(whose: CostAuraWhose, source: PlayerId, player: PlayerId): boolean {
  if (whose === "all") return true;
  return whose === "yours" ? player === source : player === opponentOf(source);
}

/**
 * Every card acting on the field, both sides, in R68's walk (the active side first, units by lane,
 * then the backrow by lane): the tops of the unit piles and the backrow cards. A card dormant under a
 * Stack pile is not on the field for effects (§3.2, R13), so its aura is off, as §10.4 layer 5 reads.
 */
function actingPermanents(state: GameState): CardInstance[] {
  const order: PlayerId[] = state.active === "p1" ? [...PLAYER_IDS] : [...PLAYER_IDS].reverse();
  return order.flatMap((player) =>
    (["units", "backrow"] as const).flatMap((row) =>
      slotsOf(player, row).flatMap((ref) => {
        const card = cardAt(state, ref);
        return card === null ? [] : [card];
      }),
    ),
  );
}

/**
 * E15: the aura rules on the field that reach a card `player` would play, in R68's walk. A Vanilla
 * card lays none (`scriptOf` runs no script for it, §6.3, R115).
 */
export function costAurasFor(state: GameState, player: PlayerId, card: CardInstance): CostAura[] {
  const out: CostAura[] = [];
  for (const source of actingPermanents(state)) {
    const hook = scriptOf(source).costAura;
    if (hook === undefined) continue;
    for (const aura of hook({ state, self: source, radiant: source.radiant })) {
      if (!whoseReaches(aura.whose, source.controller, player)) continue;
      if (!ruleReaches(state, aura, card)) continue;
      out.push(aura);
    }
  }
  return out;
}

/**
 * R455: the price rules on a card `player` would play — the live `costRule` modifiers of that player
 * that reach it (`live` is `mana.modifierIsLive`) and the aura rules of the field — the bans left out,
 * since a ban prices nothing (`whyPlayBanned` reads them).
 */
export function priceRulesFor(
  state: GameState,
  player: PlayerId,
  card: CardInstance,
  live: (mod: PlayerModifier) => boolean,
): { mods: CostRuleModifier[]; auras: CostAura[] } {
  const mods = state.players[player].mods.filter(
    (mod): mod is CostRuleModifier => mod.kind === "costRule" && live(mod) && ruleReaches(state, mod.rule, card),
  );
  const auras = costAurasFor(state, player, card).filter((aura) => aura.ban !== true);
  return { mods, auras };
}

/**
 * R455: walk the rungs over a price that has had R65's own player discounts, and return the price and
 * the modifiers whose rule changed it (a threshold rule counts only when it met its threshold). The
 * caller applies R363's Curvature at `beforeThresholds` itself, since that discount is R65's.
 */
export function climbPriceRules(
  price: number,
  rules: { mods: readonly CostRuleModifier[]; auras: readonly CostAura[] },
  curvature: (beforeThresholds: number) => number,
): { price: number; used: string[] } {
  const used = new Set<string>();
  const all: { rule: CostRule; id?: string }[] = [
    ...rules.mods.map((mod) => ({ rule: mod.rule, id: mod.id })),
    ...rules.auras.map((rule) => ({ rule })),
  ];
  let cost = price;
  // Flat rungs: every add with no threshold.
  for (const { rule, id } of all) {
    if (rule.minCost !== undefined || rule.amount === undefined) continue;
    cost += rule.amount;
    if (id !== undefined) used.add(id);
  }
  // Threshold rungs, each reading the one number the flat rungs left (R363).
  const before = cost;
  cost -= curvature(before);
  for (const { rule, id } of all) {
    if (rule.minCost === undefined || rule.amount === undefined || before < rule.minCost) continue;
    cost += rule.amount;
    if (id !== undefined) used.add(id);
  }
  // "Costs (N)": the lowest set wins over every add.
  let set: number | undefined;
  for (const { rule, id } of all) {
    if (rule.setTo === undefined) continue;
    if (rule.minCost !== undefined && before < rule.minCost) continue;
    set = set === undefined ? rule.setTo : Math.min(set, rule.setTo);
    if (id !== undefined) used.add(id);
  }
  if (set !== undefined) cost = set;
  return { price: cost, used: [...used] };
}

// ---------------------------------------------------------------------------
// The floor a card carries (E39 via E15)
// ---------------------------------------------------------------------------

/**
 * Classic+ #14 Forever&: "This can't cost less than (N)" — the `returnAfterResolve` enchantment's
 * floor, applied after every discount (E15, R455). Several floors: the highest holds. 0 without one.
 */
export function costFloorOf(card: Pick<CardInstance, "enchantments">): number {
  return enchantmentsOfKind(card, "returnAfterResolve").reduce((floor, entry) => Math.max(floor, entry.floor), 0);
}

// ---------------------------------------------------------------------------
// Bans (E15, R455)
// ---------------------------------------------------------------------------

/**
 * E15, R455: why a play of this card at this price is forbidden, or null. Classic #68 Radiant's "your
 * opponent can't play Cost (3)+ cards" reads the finished price of this very play (an X card's X
 * included), so `legalActions` never offers it and §10.5 step 1 refuses it; a cast is not asked (R70).
 */
export function whyPlayBanned(state: GameState, player: PlayerId, card: CardInstance, price: number): string | null {
  // R674: no rule forbids a play of Glitch.
  if (card.defId === GLITCH_DEF_ID) return null;
  for (const aura of costAurasFor(state, player, card)) {
    if (aura.ban !== true) continue;
    if (price >= (aura.minCost ?? 0)) return `you can't play (${aura.minCost ?? 0})+ Cost cards now`;
  }
  return null;
}

// ---------------------------------------------------------------------------
// What the badge says (R169, R432's "(N) Cost" noun, "costs (N)" verb)
// ---------------------------------------------------------------------------

function typesText(types: readonly CardType[] | undefined, plural: boolean): string {
  if (types === undefined || types.length === 0) return plural ? "cards" : "card";
  // "Trap" says "Field Trap" too, so a list holding both is read as the one word.
  const shown = types.filter((type) => !(type === "Field Trap" && types.includes("Trap")));
  const words = shown.map((type) => (plural ? `${type}s` : type));
  return words.length === 1 ? (words[0] ?? "") : `${words.slice(0, -1).join(", ")} or ${words[words.length - 1]}`;
}

/** The sentence a rule says, for one card ("next") or for every card it reaches. */
export function costRuleText(rule: CostRule, next: boolean): string {
  const plural = !next;
  const threshold = rule.minCost === undefined ? "" : `(${rule.minCost})+ Cost `;
  const subject = `${next ? "Your next " : "Your "}${threshold}${typesText(rule.types, plural)}`;
  const verb = plural ? "cost" : "costs";
  if (rule.setTo !== undefined) return `${subject} ${verb} (${rule.setTo})`;
  const amount = rule.amount ?? 0;
  if (amount < 0) return `${subject} ${verb} (${-amount}) less`;
  return `${subject} ${verb} (${amount}) more`;
}

/** A `costRule` modifier's badge caption: "next" while it waits to be used, else every card. */
export function costRuleModifierLabel(mod: { rule: CostRule; expiry: ModifierExpiry }): string {
  return costRuleText(mod.rule, mod.expiry.until === "used");
}

/** An `enchantNextSpell` modifier's badge caption. */
export function enchantNextSpellLabel(enchantment: Enchantment): string {
  switch (enchantment.kind) {
    case "returnAfterResolve":
      return `Your next Spell returns to your hand after it resolves (it can't cost less than (${enchantment.floor}))`;
    case "castOnDraw":
      return "Your next Spell gains Cast on draw";
    case "targetEnemies":
      return "Your next Spell aims at enemies when it harms and at your side when it helps";
  }
}
