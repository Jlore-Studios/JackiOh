// The face of a card in play: the one the board draws for a card the view lists, and the one the
// showcase, a log line, a pile and a prompt draw for a card the view names (SPEC §10.10).
//
// A face in play is the card as the view says it stands (R243). Everything is read off the view —
// never worked out — so none of it is a rule (CLAUDE.md rule 7). The collection's faces, the card as
// printed, are the deck builder's own (`faceModel` with no `inPlay`).
//
// The definition comes from the public catalog (§5.1), else from the match-made definitions the
// view carries (R243), else from nothing: the card is drawn by its id. The R97 sentinel names no
// card, so it has no face, and the callers draw a back.

import { useMemo } from "react";

import type { CardDef, CardType, CardView, PlayerView } from "@jackioh/shared";

import { faceModel, type FaceModel, type RolledPower } from "../cards/index.ts";
import type { NamedTally } from "../cards/tallies.ts";
import { getLocale, useLocale } from "../i18n/i18n.ts";
import { face } from "../wasm/index.ts";
import { lookupFromDefs, matchCardsOf, unknownCard, type CardInfo, type CardLookup } from "./catalog.ts";

/** R97: the id and definition an event or a prompt carries in place of a card the viewer may not read. */
export const HIDDEN_CARD = "hidden";

function infoFor(lookup: CardLookup | null, view: PlayerView, defId: string, radiant: boolean): CardInfo {
  const fromCatalog = lookup?.(defId, radiant);
  if (fromCatalog !== undefined) return fromCatalog;
  const defs = view.defs;
  const fromView = defs !== undefined && defs[defId] !== undefined ? lookupFromDefs(defs)(defId, radiant) : undefined;
  return fromView ?? unknownCard(defId);
}

/** What the view lists about a card beyond its printed face, and the face it makes (R243). */
export type LiveFacts = {
  /** A face-up backrow card's type, straight off `BackrowView`. */
  type?: CardType;
  /** The power a Heroic Power on the field rolled (`HeroView.powers`); a hand card carries its own. */
  fieldPower?: RolledPower;
  /** B5 E14: the definition of the Spell a copier's `copies` names. */
  copied?: CardDef;
  /** R2321000: the counts cards on the field keep on a unit, by their keepers' names (`useTallies`). */
  tallies?: readonly NamedTally[];
  /** #2437: the card lies in a hand, so its face carries its Temporary count. */
  inHand?: boolean;
};

/**
 * The face in play of a card the view lists: its live cost, and whatever else the view says about it. The
 * face model reads the card's account off the view (jackioh-shared `face/live.rs`); this passes the card, its
 * definition and the facts the view puts elsewhere.
 */
export function liveFace(info: CardInfo, card: CardView, facts: LiveFacts = {}): FaceModel {
  const answer = face({
    call: "live",
    card,
    ...(info.def === undefined ? {} : { def: info.def }),
    name: info.name,
    facts: { ...facts, tallies: facts.tallies === undefined ? undefined : [...facts.tallies] },
    locale: getLocale(),
  });
  if (answer.answer !== "face") throw new Error(`the face model answered ${answer.answer} for a card`);
  return answer.face;
}

/**
 * #3478: `liveFace` memoized on exactly the inputs it takes: the card's view object (stable while
 * the view is), its definition (`info` itself is a fresh object per render, so the memo keys on
 * `info.def`/`info.name`, never `info`), the live facts (`tallies` is a fresh array per render, so
 * the memo keys on its contents, never the array) and the reader's language (R711000). A hover
 * re-render performs zero new crossings; a new view recomputes only the changed card's face; a
 * language switch recomputes every face. `card: null` draws the back's placeholder, as `Card.tsx`
 * does for a back (no identity to cross for).
 */
export function useLiveFace(info: CardInfo, card: CardView | null, facts: LiveFacts = {}): FaceModel {
  const { type, fieldPower, copied, tallies, inHand } = facts;
  // A language change recomputes the face, as CardFace's labels redraw.
  const locale = useLocale();
  const { def, name } = info;
  const talliesKey = (tallies ?? []).map((entry) => `${entry.name}\0${entry.value}\0${entry.goal}`).join("\n");
  return useMemo(
    () =>
      card === null
        ? faceModel({ defId: "", radiant: false })
        : liveFace(info, card, {
            ...(type === undefined ? {} : { type }),
            ...(fieldPower === undefined ? {} : { fieldPower }),
            ...(copied === undefined ? {} : { copied }),
            ...(talliesKey === "" ? {} : { tallies }),
            ...(inHand === undefined ? {} : { inHand }),
          }),
    // `info` and the `tallies` array are fresh objects per render; the deps key on their stable
    // contents (`def`/`name`, `talliesKey`) instead, so an unchanged card never crosses again. The
    // callback still reads them through `info`/`tallies`, which is sound because a dep change always
    // recomputes with the latest render's values.
    [def, name, card, type, fieldPower, copied, talliesKey, inHand, locale],
  );
}

function backrowType(card: CardView): CardType | undefined {
  return "faceDown" in card && card.faceDown === false && "type" in card ? (card.type as CardType) : undefined;
}

export function listedFace(lookup: CardLookup | null, view: PlayerView, card: CardView): FaceModel | null {
  if (card.defId === HIDDEN_CARD || card.defId === "") return null;
  const info = infoFor(lookup, view, card.defId, card.radiant);
  const type = backrowType(card);
  const fieldPower = matchCardsOf(view).powers.get(card.instanceId);
  const copied = copiedDefOf(lookup, view, card);
  return liveFace(info, card, {
    ...(type === undefined ? {} : { type }),
    ...(fieldPower === undefined ? {} : { fieldPower }),
    ...(copied === undefined ? {} : { copied }),
  });
}

function copiedDefOf(lookup: CardLookup | null, view: PlayerView, card: CardView): CardDef | undefined {
  const copies = card.copies;
  return copies === undefined ? undefined : infoFor(lookup, view, copies.defId, copies.radiant).def;
}

export type NamedCard = {
  defId: string;
  radiant: boolean;
  /** The instance, so a card the view still lists is drawn as it stands. */
  instanceId?: string;
  /** A cost to show when the view lists the card nowhere (a play's `costPaid`). */
  cost?: number;
};

/**
 * The face in play of a card an event names, or null for the sentinel. A card the view still lists is
 * drawn as it stands there (`listedFace`); one it no longer lists is its definition as the game shows it,
 * at `cost` when one is given.
 */
export function namedFace(lookup: CardLookup | null, view: PlayerView, named: NamedCard): FaceModel | null {
  if (named.defId === HIDDEN_CARD || named.defId === "") return null;
  const listed = named.instanceId === undefined ? null : cardInView(view, named.instanceId);
  if (listed !== null && listed.defId === named.defId) return listedFace(lookup, view, listed);
  const info = infoFor(lookup, view, named.defId, named.radiant);
  return faceModel({
    defId: named.defId,
    def: info.def,
    name: info.name,
    type: info.type,
    radiant: named.radiant,
    ...(named.cost === undefined ? {} : { liveCost: named.cost }),
    inPlay: {},
  });
}

/**
 * The `CardView` for an instance anywhere the view shows cards face up. Null when the view shows it
 * nowhere, which is also what a hidden card gives.
 */
export function cardInView(view: PlayerView, instanceId: string): CardView | null {
  if (instanceId === HIDDEN_CARD) return null;
  for (const seat of [view.you, view.opponent]) {
    for (const unit of seat.units) {
      if (unit !== null && unit.instanceId === instanceId) return unit;
    }
    for (const entry of seat.backrow) {
      if (entry !== null && !entry.faceDown && entry.instanceId === instanceId) return entry;
    }
    for (const card of [...seat.graveyard, ...seat.exile, ...(seat.resolving ?? [])]) {
      if (card.instanceId === instanceId) return card;
    }
    if (Array.isArray(seat.hand)) {
      for (const card of seat.hand) {
        if (card.instanceId === instanceId) return card;
      }
    }
  }
  return null;
}
