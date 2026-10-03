// The card pool as a grid of full cards (docs/polish/6-cards.md, Surface D).
//
// Every pool entry is a `div.db-item` holding two buttons:
//   - `card-pool-<id>`, the card itself: `data-card`, `data-legal` ("false" when the open deck will
//     not take it), `data-in-deck="true"` when the open deck holds it, `data-unavailable="true"` and
//     `data-held-by="<deck name>"` when a deck it is being compared with holds it (R251: a card is
//     its catalog id, so one copy per trio), `data-refused`, `data-owned`, `aria-disabled` and
//     `draggable`. It draws the base face as a `CardFace` and badges where the card already is
//     (`.db-held`: "In deck", or "In <deck name>").
//   - `db-add-<id>`, the "+" that puts the card in the open deck in one tap.
//
// A click on the card opens its detail view, as the brief asks ("in the deck builder, a click opens
// a detail view"; since issue #37 it pages the faces one at a time), and the detail's "Add to <deck>"
// adds it. So does a right-click, or a touch long-press, which is why the inspect trigger here
// runs with `hover: false` and hands both gestures to `onInspect`. Adding takes one gesture still:
// the "+", or a drag onto the deck.
//
// The card's accessible name is the card, not just its name: cost, type, rarity, the deck that
// holds it, and what a click does. The face inside is decoration to a screen reader (it repeats
// the name), and the detail view reads the rules out in full.
//
// READ-ONLY, for the Card Almanac (R630): given none of the deck editor's props, the grid has no
// "+", no card is draggable, and no card carries `data-legal`, `data-in-deck`, `data-unavailable`,
// `data-held-by`, `data-refused`, `data-owned` or `aria-disabled`, since there is no deck to put
// it in and no collection to own it. A click, a right-click and a long-press still open the detail.

import { useMemo, type DragEvent, type ReactElement } from "react";

import type { CardDef } from "@jackioh/shared";
import type { CatalogSnapshot, Collection } from "@jackioh/validator";

import { CardFace, faceModel, useInspectTrigger } from "../../cards/index.ts";
import { costText } from "../../patches/diff.ts";
import { CARD_POOL, addPoolId, poolCardId } from "./testids.ts";
import type { Holder } from "./workshop.ts";

/** Where a pool card already is: in the open deck, in a compared deck, or nowhere that matters. */
export type PoolPlace = "deck" | Holder | null;

/** What every grid needs: the cards to show and the detail view a card opens. */
type PoolGridBaseProps = {
  /** The visible pool, already filtered and sorted (`visiblePool`, or the almanac's `almanacPool`). */
  ids: readonly string[];
  catalog: CatalogSnapshot;
  onInspect: (cardId: string) => void;
};

/** The deck editor's half: the open deck, ownership, the "+" and the drag. */
export type PoolGridDeckProps = {
  /** The open deck's name, for the "+" label. */
  deckName: string;
  /** Null when the collection could not be read: then nothing claims to be owned or unowned. */
  collection: Collection | null;
  /** The cards the open deck holds. */
  inDeck: ReadonlySet<string>;
  /** Card id → the compared deck that holds it (R251). */
  holders: ReadonlyMap<string, Holder>;
  refusedCardId: string | null;
  onAdd: (cardId: string) => void;
  onDragStart: (cardId: string, event: DragEvent<HTMLElement>) => void;
  onDragEnd: () => void;
};

/** A read-only grid passes none of the deck editor's props. */
type NoDeckProps = { [Key in keyof PoolGridDeckProps]?: undefined };

export type PoolGridProps = PoolGridBaseProps & (PoolGridDeckProps | NoDeckProps);

/** One card's share of the deck editor's props; null in a read-only grid. */
type PoolItemDeck = {
  deckName: string;
  place: PoolPlace;
  /** True or false when the collection is known, null when it is not. */
  owned: boolean | null;
  refused: boolean;
  onAdd: (cardId: string) => void;
  onDragStart: (cardId: string, event: DragEvent<HTMLElement>) => void;
  onDragEnd: () => void;
};

type PoolItemProps = {
  cardId: string;
  def: CardDef;
  deck: PoolItemDeck | null;
  onInspect: (cardId: string) => void;
};

/** The badge on a card that is already somewhere: "In deck", or "In <compared deck>". */
export function placeWords(place: PoolPlace): string | null {
  if (place === null) return null;
  return place === "deck" ? "In deck" : `In ${place.name}`;
}

/**
 * The pool card's accessible name: "Bigot, Unit, (2) Cost, Common, in Control, unavailable. Show details" (R432).
 * A read-only card (the almanac) has no place and no ownership: "Bigot, Unit, (2) Cost, Common. Show details".
 */
export function poolCardLabel(def: CardDef, place: PoolPlace = null, owned: boolean | null = null): string {
  const parts = [def.name, def.type, costText(def.cost), def.rarity];
  if (place === "deck") parts.push("in this deck");
  else if (place !== null) parts.push(`in ${place.name}, unavailable`);
  if (owned === false) parts.push("not in your collection");
  return `${parts.join(", ")}. Show details`;
}

function PoolItem({ cardId, def, deck, onInspect }: PoolItemProps): ReactElement {
  const place = deck?.place ?? null;
  const badge = placeWords(place);

  const face = useMemo(() => faceModel({ defId: cardId, def, radiant: false }), [cardId, def]);
  const inspect = useInspectTrigger(
    { key: poolCardId(cardId), face },
    {
      hover: false,
      onLongPress: () => {
        onInspect(cardId);
      },
      onContextMenu: () => {
        onInspect(cardId);
      },
    },
  );

  return (
    <div className="db-item" data-card={cardId}>
      <button
        type="button"
        className="db-card"
        data-testid={poolCardId(cardId)}
        data-card={cardId}
        // `e2e/support/testids.ts` already exports this as ILLEGAL (M5-T2's vocabulary): a card the
        // builder will not put in the open deck, because it is there already or a compared deck
        // holds it.
        data-legal={deck === null ? undefined : place === null ? "true" : "false"}
        data-in-deck={place === "deck" ? "true" : undefined}
        data-unavailable={place !== null && place !== "deck" ? "true" : undefined}
        data-held-by={place !== null && place !== "deck" ? place.name : undefined}
        data-refused={deck?.refused === true ? "true" : undefined}
        data-owned={deck === null || deck.owned === null ? undefined : deck.owned ? "true" : "false"}
        data-rarity={def.rarity}
        aria-disabled={deck === null ? undefined : place !== null}
        aria-label={poolCardLabel(def, place, deck?.owned ?? null)}
        draggable={deck === null ? undefined : true}
        onDragStart={
          deck === null
            ? undefined
            : (event) => {
                deck.onDragStart(cardId, event);
              }
        }
        onDragEnd={deck?.onDragEnd}
        onClick={() => {
          onInspect(cardId);
        }}
        {...inspect.handlers}
      >
        <span className="db-card-face" aria-hidden="true">
          <CardFace face={face} layout="full" lazyArt />
        </span>
        {badge === null ? null : (
          <span className="db-held" data-place={place === "deck" ? "deck" : "other"} aria-hidden="true">
            {badge}
          </span>
        )}
      </button>
      {deck === null ? null : (
        <button
          type="button"
          className="db-add"
          data-testid={addPoolId(cardId)}
          // Already placed: the same refusal a drag gets, so it still reports why, but says it is off.
          aria-disabled={place !== null}
          aria-label={`Add ${def.name} to ${deck.deckName}`}
          title={`Add to ${deck.deckName}`}
          onClick={() => {
            deck.onAdd(cardId);
          }}
        >
          <span aria-hidden="true">+</span>
        </button>
      )}
      {inspect.overlay}
    </div>
  );
}

/** One card's share of the deck editor's props, or null for a read-only grid. */
function itemDeck(props: PoolGridProps, cardId: string): PoolItemDeck | null {
  if (props.onAdd === undefined) return null;
  const { deckName, collection, inDeck, holders, refusedCardId, onAdd, onDragStart, onDragEnd } = props;
  return {
    deckName,
    place: inDeck.has(cardId) ? "deck" : (holders.get(cardId) ?? null),
    owned: collection === null ? null : (collection[cardId] ?? 0) > 0,
    refused: refusedCardId === cardId,
    onAdd,
    onDragStart,
    onDragEnd,
  };
}

export default function PoolGrid(props: PoolGridProps): ReactElement {
  const { ids, catalog, onInspect } = props;

  return (
    <section className="db-pool" aria-label="Card pool" data-testid={CARD_POOL}>
      {ids.map((cardId) => {
        const def = catalog.cards[cardId];
        if (def === undefined) return null;
        return <PoolItem key={cardId} cardId={cardId} def={def} deck={itemDeck(props, cardId)} onInspect={onInspect} />;
      })}
    </section>
  );
}
