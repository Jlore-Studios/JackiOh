// Mana refresh, temporary mana and the cost calculation (SPEC §2.3, §6.3 Cost, R65, R396, R455).

import type { GameEvent } from "@jackioh/shared";
import { defOf } from "./catalog";
import { GLITCH_DEF_ID } from "./config";
import { climbPriceRules, costFloorOf, priceRulesFor } from "./costRules";
import { cardTypeOf } from "./faces";
import { playableFromGraveyard } from "./graveyardPlay";
import { scriptOf } from "./scripts";
import { handicapOf, type CardInstance, type GameState, type PlayerModifier, type PlayerState } from "./state";

/**
 * §2.3, R181: max mana is min(turns started + the seat's mana bonus, its mana cap), plus persistent
 * modifiers, floored at 0. With no handicap the bonus is 0 and the cap is MAX_MANA, which is §2.3's
 * "min(number of turns you have started, 4), plus persistent modifiers". `nextTurnMod` is not one of
 * those: it is a one-shot rider on a single refresh, which the refresh spends and clears, so it never
 * reaches max mana. Hinder and every other modifier apply on top of the capped value exactly as for
 * a human.
 */
export function maxManaFor(side: PlayerState): number {
  const handicap = handicapOf(side);
  const base = Math.min(side.turnsStarted + handicap.manaBonus, handicap.manaCap);
  return Math.max(0, base + side.mana.permMod);
}

/**
 * R169: the id the next refresh's rider (`mana.nextTurnMod`, §6.3 Mana: "'next turn' mana is stored
 * as a modifier for the next refresh") travels under — one badge per player, which `modifierChanged`
 * names as it appears, changes and is spent, and which the view lists while the rider is not 0.
 */
export const NEXT_REFRESH_MODIFIER_ID = "nextTurnMana";

/**
 * Start of turn: refresh to max, moved by the one-shot rider (§6.3 Mana: "'next turn' mana is stored
 * as a modifier for the next refresh"), which is then cleared. The rider changes what the refresh
 * gives, not max mana: #24 Efficiency Dividend's next-turn mana is temporary mana on §2.3's list and
 * "adds to current mana and can exceed 4", exactly as #6 Mana Well's gain does, and #21 Hinder
 * "subtracts from the opponent's next refresh". Current mana never goes below 0 (§2.3).
 */
export function refreshMana(side: PlayerState): void {
  const max = maxManaFor(side);
  side.mana.max = max;
  side.mana.current = Math.max(0, max + side.mana.nextTurnMod);
  side.mana.nextTurnMod = 0;
}

/**
 * §6.3 Refresh, R364: give back up to `amount` spent mana, never past max — Hearthstone's "Refresh
 * Mana Crystals" (#78 /fullsend). Unlike temporary mana (`gainMana`) it cannot take current above
 * max, and a player already at or above max gains nothing.
 */
export function refreshSomeMana(side: PlayerState, amount: number): void {
  if (amount <= 0 || side.mana.current >= side.mana.max) return;
  side.mana.current = Math.min(side.mana.max, side.mana.current + amount);
}

/** Temporary mana may take current above max (§2.3). */
export function gainMana(side: PlayerState, amount: number): void {
  side.mana.current = Math.max(0, side.mana.current + amount);
}

export function spendMana(side: PlayerState, amount: number): void {
  side.mana.current = Math.max(0, side.mana.current - amount);
}

export function manaEvent(player: "p1" | "p2", side: PlayerState): GameEvent {
  return { type: "manaChanged", player, current: side.mana.current, max: side.mana.max };
}

/** The printed cost as it stands: X uses the chosen X, an embiggen card the chosen price (R65). */
export function printedCost(state: GameState, instance: CardInstance): number {
  const script = scriptOf(instance);
  if (script.cost !== undefined) return Math.max(0, script.cost({ state, instance }));

  const cost = defOf(state, instance.defId).cost;
  if (cost === "X") return Math.max(0, instance.x ?? 0);
  if (typeof cost === "number") return cost;
  return instance.embiggened === true ? cost.embiggen : cost.base;
}

export function isXCost(state: GameState, instance: CardInstance): boolean {
  return defOf(state, instance.defId).cost === "X";
}

/**
 * R48: a modifier that covers a player's *next* turn does nothing on the turn it was created on,
 * and nothing on the opponent's turn in between either — #77's text is "during **your** next turn",
 * so it is live only once that player is the active one on a later turn. `expireModifiers` ends it
 * at the cleanup of that player's next turn, which is why this is a separate question from expiry.
 */
export function modifierIsLive(state: GameState, mod: PlayerModifier): boolean {
  // §2.2: "this turn" is the turn it names, and no later one — even when it was made after that
  // turn's cleanup had run and so outlives it until the next cleanup (`expireModifiers`).
  if (mod.expiry.until === "thisTurn") return mod.expiry.turn >= state.turn;
  if (mod.expiry.until !== "nextTurnOf") return true;
  return state.turn > mod.expiry.fromTurn && state.active === mod.expiry.player;
}

/**
 * How a price is read. `asPlay` prices the card as a play of it now wherever it lies: a card played
 * from a graveyard (B5 E11, R454) pays the player's prices exactly as a card played from a hand does.
 */
export type CostOptions = { asPlay?: boolean };

/** What `priceOf` works out: the price, and the `costRule` modifiers that changed it (spent by a play). */
type Price = { cost: number; usedRules: string[] };

/**
 * R65: start from costOverride or the printed cost, add the instance's costMod, add the player's
 * discounts, then Professor Curvature if the result is 4 or more (R363), and floor at 0. An X-cost
 * card costs exactly X and ignores modifiers, unless an override makes it free.
 *
 * The player's discounts and Curvature are prices for a play — §6.3's Cost is "what a card costs to
 * play now", #35's is "the next Spell you play", #78's "this turn your cards cost 1 less", and R48
 * reads Curvature "at play time" — so they reach a card in its controller's hand, where a play takes
 * it from (§10.5 step 1), and no other. A card in a library or a graveyard is read at its own cost,
 * its `costOverride` or printed cost with its `costMod`: #30 Archivist's "highest" (R24), #94's
 * 2-cost draw and odd-cost exile (R66), a Recruit's filter — as Hearthstone's hand discounts never
 * reach the deck or the graveyard (R65) — except a graveyard a permission lets its player play from
 * (E11, R454), where a play takes the card from. `options.asPlay` prices any card as a play of it now.
 *
 * R455 (E15) adds its rungs through `costRules.ts`: after R65's discounts, the flat price rules (the
 * `costRule` modifiers and the field's cost auras), then the threshold rules, which read the one
 * number the flat ones left, as R363 reads Curvature there; then "costs (N)" sets; then the floor the
 * card carries (Forever&'s "can't cost less than (N)"), which holds in every zone, as the card's own
 * `costOverride` does; then 0. An X-cost card still ignores every modifier (R65), and only its floor
 * reaches it.
 */
export function effectiveCost(state: GameState, instance: CardInstance, options: CostOptions = {}): number {
  return priceOf(state, instance, options).cost;
}

/** R454, R455: what a play of this card now costs, wherever the play takes it from. */
export function playCost(state: GameState, instance: CardInstance): number {
  return effectiveCost(state, instance, { asPlay: true });
}

/**
 * R455: the `costRule` modifiers a play of this card at this price spends — the ones "until used" that
 * changed its price (Classic #2's next Trap or Field Spell). A cast spends none, since it pays nothing
 * (R70), and an X-cost card none, since no modifier reaches it (R65).
 */
export function costRulesSpentBy(state: GameState, instance: CardInstance): string[] {
  const { usedRules } = priceOf(state, instance, { asPlay: true });
  const mods = state.players[instance.controller].mods;
  return usedRules.filter((id) => mods.some((mod) => mod.id === id && mod.expiry.until === "used"));
}

function priceOf(state: GameState, instance: CardInstance, options: CostOptions): Price {
  const side = state.players[instance.controller];
  const override = instance.costOverride;
  const floor = costFloorOf(instance);
  // R674: Glitch costs (0) wherever it is and whatever would change that.
  if (instance.defId === GLITCH_DEF_ID) return { cost: 0, usedRules: [] };

  // R65: X-cost cards cost exactly X and ignore modifiers, but an override makes one free. R455: a
  // floor the card carries is its own, and holds.
  if (isXCost(state, instance)) {
    return { cost: Math.max(0, override !== undefined ? 0 : printedCost(state, instance), floor), usedRules: [] };
  }

  let cost = (override ?? printedCost(state, instance)) + instance.costMod;
  // R65: a player's discounts price a play, and a play takes a card from its hand — or from its
  // graveyard, while a permission lets its player play it from there (E11, R454).
  const forPlay =
    options.asPlay === true ||
    instance.zone.z === "hand" ||
    (instance.zone.z === "graveyard" && playableFromGraveyard(state, instance));
  if (!forPlay) return { cost: Math.max(0, cost, floor), usedRules: [] };
  const type = cardTypeOf(state, instance);

  for (const mod of side.mods) {
    if (mod.kind !== "costDiscount") continue;
    if (!modifierIsLive(state, mod)) continue;
    if (mod.onlyType !== undefined && mod.onlyType !== type) continue;
    if (mod.minCurrentCost !== undefined) continue; // Curvature is applied below.
    cost -= mod.amount;
  }

  // R65, R363: "apply Professor Curvature if the result is then 4 or more" — the result of the steps
  // above, which every live Curvature reads. Two of them (#39's copy, #33's) each test that one
  // number, so both apply to a card the discounts leave at 4 or more, and the order they were played
  // in changes nothing: a Curvature never reads the cost another Curvature has already lowered (R48).
  // R455: the flat price rules come first, and the threshold rules read the same number Curvature does.
  const curvature = (beforeCurvature: number): number => {
    let off = 0;
    for (const mod of side.mods) {
      if (mod.kind !== "costDiscount" || mod.minCurrentCost === undefined) continue;
      if (!modifierIsLive(state, mod)) continue;
      if (beforeCurvature >= mod.minCurrentCost) off += mod.amount;
    }
    return off;
  };
  const rules = priceRulesFor(state, instance.controller, instance, (mod) => modifierIsLive(state, mod));
  const climbed = climbPriceRules(cost, rules, curvature);

  return { cost: Math.max(0, climbed.price, floor), usedRules: climbed.used };
}

export function canAfford(state: GameState, instance: CardInstance): boolean {
  return effectiveCost(state, instance) <= state.players[instance.controller].mana.current;
}

/**
 * R396 (Classic #10, #18, #25, #32, #39): what a card costs wherever a rule compares or counts costs —
 * the one reader card scripts compare costs with. An X-cost card on the field costs the X it was
 * played for (the instance's `x`); anywhere else, and on the field when it arrived without a chosen X
 * (a Recruit, a summon), it costs 0 (R65). An embiggen card outside a play costs its base price (R65),
 * on the field too. Any other card is R65's cost as it stands: a hand card at its hand cost, a card in
 * a library, a graveyard or on the field at its own (R66). A floor the card carries holds (R455).
 */
export function costNow(state: GameState, card: CardInstance): number {
  const floor = costFloorOf(card);
  if (isXCost(state, card)) {
    return Math.max(0, card.zone.z === "field" ? (card.x ?? 0) : 0, floor);
  }
  if (card.embiggened === true && card.zone.z !== "resolving") {
    return effectiveCost(state, { ...card, embiggened: false });
  }
  return effectiveCost(state, card);
}
