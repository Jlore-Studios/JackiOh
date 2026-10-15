// The practice sandbox's card search and the cards its tools can name (issue #923, R923000–R923003).
//
// Nothing here is a rule (CLAUDE.md rule 7): the panel builds a `sandbox` action from the player's
// picks and the engine decides; search only finds cards to offer, and a card the engine would refuse
// (one of a set that has not shipped, R1420) is simply never offered.

import { setShips, type ActionBody, type CardDef, type CardDefs, type PlayerView, type SandboxOp } from "@jackioh/shared";

import { SANDBOX_RESULTS_MAX } from "./config.ts";

/** R923001: every card and token a spawn may make, in catalog order: those of a set that ships. */
export function spawnableDefs(defs: CardDefs): CardDef[] {
  return Object.values(defs).filter((def) => setShips(def.set));
}

/** A number on the card a search can name: its cost, or an embiggened card's base price. */
function costOf(def: CardDef): number | null {
  if (typeof def.cost === "number") return def.cost;
  if (typeof def.cost === "object") return def.cost.base;
  return null;
}

/** What a word of a search is matched against: name, both texts, id, set, type and rarity. */
function haystack(def: CardDef): string {
  return [
    def.name,
    def.base.text,
    def.radiant.text,
    def.id,
    def.set,
    def.type,
    def.rarity,
    def.printedRarity ?? "",
    def.token ? "token" : "",
    def.cost === "X" ? "x" : "",
  ]
    .join(" ")
    .toLowerCase();
}

/**
 * R923001: the cards a search finds, at most `limit`, in catalog order. Every word of the query must
 * match: a number matches the cost, any other word a part of the name, the text (either face), the
 * id, the set, the type or the rarity. An empty query finds the first `limit` cards.
 */
export function searchCards(defs: readonly CardDef[], query: string, limit: number = SANDBOX_RESULTS_MAX): CardDef[] {
  const words = query.toLowerCase().split(/\s+/u).filter((word) => word.length > 0);
  const found: CardDef[] = [];
  for (const def of defs) {
    if (found.length >= limit) break;
    const text = haystack(def);
    const matches = words.every((word) => (/^\d+$/u.test(word) ? costOf(def) === Number(word) : text.includes(word)));
    if (matches) found.push(def);
  }
  return found;
}

/** The controller's hold reason for "Hold the AI's turn" (`PracticeController.setHold`). */
export const SANDBOX_HOLD = "sandbox";

/** The action a tool sends: the engine's `sandbox`, which the practice worker logs like any other. */
export function sandboxAction(op: SandboxOp): ActionBody {
  return { type: "sandbox", op };
}

/** A card the destroy and remove tools can name: one the human can see. */
export type VisibleCard = {
  instanceId: string;
  defId: string;
  /** Whose side it is on, from the human's view. */
  side: "you" | "opponent";
  where: "field" | "hand" | "graveyard";
};

/**
 * R923003: the cards the human can see and so name: both fields' face-up cards, their own hand and
 * both graveyards. A face-down enemy card and the AI's hand are hidden (R97), so no tool can name them.
 */
export function visibleCards(view: PlayerView): VisibleCard[] {
  const cards: VisibleCard[] = [];
  for (const [side, seat] of [
    ["you", view.you],
    ["opponent", view.opponent],
  ] as const) {
    for (const unit of seat.units) {
      if (unit !== null) cards.push({ instanceId: unit.instanceId, defId: unit.defId, side, where: "field" });
    }
    for (const slot of seat.backrow) {
      if (slot !== null && "defId" in slot) cards.push({ instanceId: slot.instanceId, defId: slot.defId, side, where: "field" });
    }
    if (Array.isArray(seat.hand)) {
      for (const card of seat.hand) cards.push({ instanceId: card.instanceId, defId: card.defId, side, where: "hand" });
    }
    for (const card of seat.graveyard) cards.push({ instanceId: card.instanceId, defId: card.defId, side, where: "graveyard" });
  }
  return cards;
}
