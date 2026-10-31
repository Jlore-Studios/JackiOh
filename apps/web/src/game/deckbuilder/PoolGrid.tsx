// Card pool grid (docs/polish/6-cards.md, Surface D).
// Pool entry: `card-pool-<id>` (card details/inspect trigger, R251 trio uniqueness, issue #1017 full text preview,
// R1383 "New" ribbon) and `db-add-<id>` quick-add button.
// Read-only Almanac mode (R630): omits add/drag/legality badges; inspect remains accessible.
// Cosmetics: R1956000 / R1655000 owned cards render user cosmetics via `OwnCopy`.
// Performance: #2266 memoized `PoolCard` + deferred rendering; #2334 virtualized windowing (poolWindow.ts).

import {
  memo,
  useContext,
  useDeferredValue,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type DragEvent,
  type ReactElement,
  type ReactNode,
} from "react";

import type { CardDef, SetName } from "@jackioh/shared";
import type { CatalogSnapshot, Collection } from "@jackioh/validator";

import { CardFace, TermTips, faceModel, faceText, useInspectTrigger } from "../../cards/index.ts";
import { OwnershipContext, type CardOwnership } from "../../cards/cosmetics.ts";
import { FullArtOwned, OwnCopy, useFullArt, useOwnCopy } from "../../cards/fullArt.ts";
import { CARD_TABLES, readerName } from "../../cards/chinese.ts";
import { costText } from "../../patches/diff.ts";
import { NEW_RIBBON_SET } from "../../patches/ribbon.ts";
import { getLocale, t, useLocale } from "../../i18n/i18n.ts";
import { poolRowTracks } from "./poolWindow.ts";
import { CARD_POOL, addPoolId, poolCardId } from "./testids.ts";
import { usePoolWindow } from "./usePoolWindow.ts";
import type { Holder } from "./workshop.ts";

/** #2266: Initial chunk of pool cards drawn on first render. */
export const POOL_FIRST_CARDS = 48;

/** Where a pool card already is: in the open deck, in a compared deck, or nowhere that matters. */
export type PoolPlace = "deck" | Holder | null;

/** R1956000: the cards that are the viewer's own copies, and the cosmetics they own on them. */
export type OwnCopies = { cards: ReadonlySet<string>; owned: CardOwnership };

/** R1956000: renders card with viewer's cosmetics. */
export function AsOwnCopy({
  own,
  cardId,
  children,
}: {
  own: OwnCopies | undefined;
  cardId: string;
  children?: ReactNode;
}): ReactElement {
  const outer = useContext(OwnershipContext);
  const mine = own !== undefined && own.cards.has(cardId);
  return (
    <FullArtOwned owned={mine ? own.owned : outer}>
      <OwnCopy own={mine}>{children}</OwnCopy>
    </FullArtOwned>
  );
}

/** What every grid needs: the cards to show and the detail view a card opens. */
type PoolGridBaseProps = {
  /** The visible pool, already filtered and sorted (`visiblePool`, or the almanac's `almanacPool`). */
  ids: readonly string[];
  catalog: CatalogSnapshot;
  onInspect: (cardId: string) => void;
  /** R1383: the set whose cards wear the "New" ribbon; `NEW_RIBBON_SET` when absent, null for none. */
  ribbon?: SetName | null;
  /** R1956000: the cards drawn as the viewer's own copies (the signed-in Almanac's); none when absent. */
  own?: OwnCopies;
  /** #2334: mount only the rows of cards near the view (the Almanac's), not the whole pool. */
  virtual?: boolean;
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
  /** R1383: the card wears the "New" ribbon. */
  fresh: boolean;
  /** #2334: the grid area a virtual grid places the card in; it then takes no part in the grid's flow. */
  area?: string | undefined;
  onInspect: (cardId: string) => void;
};

/** The grid's handlers, the same functions on every render, each calling the screen's latest. */
type PoolHandlers = Pick<PoolItemDeck, "onAdd" | "onDragStart" | "onDragEnd"> & { onInspect: (cardId: string) => void };

/** The badge on a card that is already somewhere: "In deck", or "In <compared deck>". */
export function placeWords(place: PoolPlace): string | null {
  if (place === null) return null;
  return place === "deck" ? t("In deck") : t("In {name}", { name: place.name });
}

/** Pool card accessible name (R432; printed rarity for tokens R503, R1177001). */
export function poolCardLabel(
  def: CardDef,
  place: PoolPlace = null,
  owned: boolean | null = null,
  fresh = false,
): string {
  // The type and rarity in the card frame's own words (cards/chinese.ts, R1303).
  const terms = CARD_TABLES[getLocale()]?.terms;
  const rarity = terms?.rarities[def.rarity] ?? def.rarity;
  const printed = def.printedRarity;
  const rarityWords =
    printed === undefined ? rarity : t("{rarity}, printed {printed}", { rarity, printed: terms?.rarities[printed] ?? printed });
  const parts = [readerName(def), terms?.types[def.type] ?? def.type, t(costText(def.cost)), rarityWords];
  if (fresh) parts.push(t("new"));
  if (place === "deck") parts.push(t("in this deck"));
  else if (place !== null) parts.push(t("in {deck}, unavailable", { deck: place.name }));
  if (owned === false) parts.push(t("not in your collection"));
  return t("{card}. Show details", { card: parts.join(t(", ")) });
}

/** Whether the browser reports a `content-visibility: auto` holder nearing the screen. */
const SEES_NEAR =
  typeof HTMLElement !== "undefined" && "oncontentvisibilityautostatechange" in HTMLElement.prototype;

/** Pool card face, mounted on content-visibility unskip to avoid layout costs. */
function PoolFace({ children }: { children: ReactNode }): ReactElement {
  const holder = useRef<HTMLSpanElement>(null);
  const [near, setNear] = useState(!SEES_NEAR);
  // Before the first frame, which is when the browser decides whether the holder is skipped.
  useLayoutEffect(() => {
    const element = holder.current;
    if (near || element === null) return undefined;
    const onState = (event: Event): void => {
      if ((event as Event & { skipped?: boolean }).skipped === false) setNear(true);
    };
    element.addEventListener("contentvisibilityautostatechange", onState);
    return () => {
      element.removeEventListener("contentvisibilityautostatechange", onState);
    };
  }, [near]);
  return (
    <span ref={holder} className="db-card-face" aria-hidden="true" data-skippable="">
      {near ? children : null}
    </span>
  );
}

function PoolItem({ cardId, def, deck, fresh, area, onInspect }: PoolItemProps): ReactElement {
  const locale = useLocale();
  const place = deck?.place ?? null;
  const badge = placeWords(place);

  const face = useMemo(() => faceModel({ defId: cardId, def, radiant: false }), [cardId, def, locale]);
  // R1313000: a full art the viewer does not own prints its words, and opens a preview as they do.
  const fullArtOn = useFullArt(cardId);
  // R1956000: the viewer's own copy wears their picks; a card the collection says is unowned is not one.
  const worn = useOwnCopy() && deck?.owned !== false;
  const inspect = useInspectTrigger(
    { key: poolCardId(cardId), face },
    {
      hover: faceText(face, undefined, fullArtOn).less,
      onLongPress: () => {
        onInspect(cardId);
      },
      onContextMenu: () => {
        onInspect(cardId);
      },
    },
  );

  return (
    <div
      className={area === undefined ? "db-item" : "db-item db-item--placed"}
      data-card={cardId}
      style={area === undefined ? undefined : { gridArea: area }}
    >
      <button
        type="button"
        className="db-card"
        data-testid={poolCardId(cardId)}
        data-card={cardId}
        // data-legal false when already in deck or held by compared deck.
        data-legal={deck === null ? undefined : place === null ? "true" : "false"}
        data-in-deck={place === "deck" ? "true" : undefined}
        data-unavailable={place !== null && place !== "deck" ? "true" : undefined}
        data-held-by={place !== null && place !== "deck" ? place.name : undefined}
        data-refused={deck?.refused === true ? "true" : undefined}
        data-owned={deck === null || deck.owned === null ? undefined : deck.owned ? "true" : "false"}
        data-rarity={def.rarity}
        aria-disabled={deck === null ? undefined : place !== null}
        aria-label={poolCardLabel(def, place, deck?.owned ?? null, fresh)}
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
        {/* `data-skippable` (fit.ts SKIPPABLE_ATTRIBUTE): deckbuilder.css lets the browser skip the
            face's layout off screen, and the face is drawn and fitted when it is near (#263). */}
        {/* R2098000: its terms show their rule on hover and long press; they take no focus inside this button, whose face is aria-hidden. */}
        <PoolFace>
          <TermTips>
            <CardFace face={face} layout="full" lazyArt worn={worn} />
          </TermTips>
        </PoolFace>
        {fresh ? (
          <span className="db-new" data-new-set={def.set} aria-hidden="true">
            {t("New")}
          </span>
        ) : null}
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
          aria-label={t("Add {card} to {deck}", { card: readerName(def), deck: deck.deckName })}
          title={t("Add to {deck}", { deck: deck.deckName })}
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

/**
 * Whether a pool card would draw the same: `itemDeck` builds its deck share anew on each render, so
 * that is compared field by field (#2266).
 */
function samePoolItem(before: PoolItemProps, after: PoolItemProps): boolean {
  if (
    before.cardId !== after.cardId ||
    before.def !== after.def ||
    before.fresh !== after.fresh ||
    before.area !== after.area ||
    before.onInspect !== after.onInspect
  ) {
    return false;
  }
  const was = before.deck;
  const is = after.deck;
  if (was === null || is === null) return was === is;
  return (
    was.deckName === is.deckName &&
    was.place === is.place &&
    was.owned === is.owned &&
    was.refused === is.refused &&
    was.onAdd === is.onAdd &&
    was.onDragStart === is.onDragStart &&
    was.onDragEnd === is.onDragEnd
  );
}

/** A filter, sort, ownership or deck change re-renders only the cards it changes (#2266). */
const PoolCard = memo(PoolItem, samePoolItem);

/** One card's share of the deck editor's props, or null for a read-only grid. */
function itemDeck(props: PoolGridProps, cardId: string, handlers: PoolHandlers): PoolItemDeck | null {
  if (props.onAdd === undefined) return null;
  const { deckName, collection, inDeck, holders, refusedCardId } = props;
  return {
    deckName,
    place: inDeck.has(cardId) ? "deck" : (holders.get(cardId) ?? null),
    owned: collection === null ? null : (collection[cardId] ?? 0) > 0,
    refused: refusedCardId === cardId,
    onAdd: handlers.onAdd,
    onDragStart: handlers.onDragStart,
    onDragEnd: handlers.onDragEnd,
  };
}

/** Stable handler references delegating to latest props. */
function usePoolHandlers(props: PoolGridProps): PoolHandlers {
  const latest = useRef(props);
  useLayoutEffect(() => {
    latest.current = props;
  });
  return useMemo<PoolHandlers>(
    () => ({
      onInspect: (cardId) => {
        latest.current.onInspect(cardId);
      },
      onAdd: (cardId) => {
        latest.current.onAdd?.(cardId);
      },
      onDragStart: (cardId, event) => {
        latest.current.onDragStart?.(cardId, event);
      },
      onDragEnd: () => {
        latest.current.onDragEnd?.();
      },
    }),
    [],
  );
}

/** One card of the pool: null for an id the catalog does not hold. */
function poolCard(props: PoolGridProps, handlers: PoolHandlers, cardId: string, area?: string): ReactElement | null {
  const { catalog, ribbon = NEW_RIBBON_SET, own } = props;
  const def = catalog.cards[cardId];
  if (def === undefined) return null;
  return (
    <AsOwnCopy key={cardId} own={own} cardId={cardId}>
      <PoolCard
        cardId={cardId}
        def={def}
        deck={itemDeck(props, cardId, handlers)}
        fresh={ribbon !== null && def.set === ribbon}
        area={area}
        onInspect={handlers.onInspect}
      />
    </AsOwnCopy>
  );
}

export default function PoolGrid(props: PoolGridProps): ReactElement {
  return props.virtual === true ? <WindowedPool {...props} /> : <StagedPool {...props} />;
}

/** Every card, the first POOL_FIRST_CARDS of them first (#2266). */
function StagedPool(props: PoolGridProps): ReactElement {
  const { ids } = props;
  const handlers = usePoolHandlers(props);
  // Deferred rendering prevents displaying cards no longer in pool.
  const deferred = useDeferredValue(ids, ids.slice(0, POOL_FIRST_CARDS));
  const shown = ids.length > deferred.length ? ids.slice(0, deferred.length) : ids;

  return (
    <section className="db-pool" aria-label={t("Card pool")} data-testid={CARD_POOL}>
      {shown.map((cardId) => poolCard(props, handlers, cardId))}
    </section>
  );
}

/** The grid area of the card at `at`: its row and column, and the lines they end on. */
function cellOf(at: number, columns: number): string {
  const row = Math.floor(at / columns) + 1;
  const column = (at % columns) + 1;
  return `${String(row)} / ${String(column)} / ${String(row + 1)} / ${String(column + 1)}`;
}

/** Only the rows near the view (#2334); the grid's row tracks hold the place of the rest. */
function WindowedPool(props: PoolGridProps): ReactElement {
  const { ids } = props;
  const handlers = usePoolHandlers(props);
  const section = useRef<HTMLElement>(null);
  const view = usePoolWindow(section, ids.length);
  // Keeps focused card mounted when scrolled out of view.
  const [held, setHeld] = useState<string | null>(null);

  const rows = view === null || view === "all" ? null : view;
  const from = rows === null ? 0 : rows.firstRow * rows.columns;
  const to =
    view === "all"
      ? ids.length
      : Math.min(ids.length, rows === null ? POOL_FIRST_CARDS : (rows.lastRow + 1) * rows.columns);
  const heldAt = held === null ? -1 : ids.indexOf(held);
  const mounted: number[] = [];
  if (heldAt >= 0 && heldAt < from) mounted.push(heldAt);
  for (let at = from; at < to; at += 1) mounted.push(at);
  if (heldAt >= to) mounted.push(heldAt);

  return (
    <section
      ref={section}
      className={rows === null ? "db-pool" : "db-pool db-pool--virtual"}
      aria-label={t("Card pool")}
      data-testid={CARD_POOL}
      style={rows === null ? undefined : { gridTemplateRows: poolRowTracks(ids.length, rows) }}
      onFocus={(event) => {
        const item = event.target instanceof Element ? event.target.closest<HTMLElement>(".db-item") : null;
        setHeld(item?.dataset.card ?? null);
      }}
      onBlur={(event) => {
        // Focus moving to another card of the pool is the next card's focus to say.
        if (!(event.relatedTarget instanceof Node && event.currentTarget.contains(event.relatedTarget))) setHeld(null);
      }}
    >
      {mounted.map((at) => {
        const cardId = ids[at];
        if (cardId === undefined) return null;
        return poolCard(
          props,
          handlers,
          cardId,
          rows === null ? undefined : cellOf(at, rows.columns),
        );
      })}
    </section>
  );
}
