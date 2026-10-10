// Open deck sidebar: count, curve, tiles, comparison, verdict and actions (docs/polish/6-cards.md,
// Surface D; SPEC §9.4, R250–R251).
//
// Tile order is display-only; the deck's order is saved.
//
// R251 conflicts are marked, never removed for the player.
//
// On phones the list folds but stays mounted (deckbuilder.css).
//
// Tiles support preview, inspect, and keyboard detail view (I, the context-menu key or Shift+F10).

import { useId, useLayoutEffect, useMemo, useRef, useState, type DragEvent, type ReactElement, type ReactNode, type RefObject } from "react";

import type { CardCost, CardDef } from "@jackioh/shared";
import type { CatalogSnapshot } from "@jackioh/validator";

import { CardArt, faceModel, isInspectKey, useInspectTrigger } from "../../cards/index.ts";
import { DECK_SIZE } from "./deckSize.ts";
import { deckListOrder } from "./filters.ts";
import ManaCurve from "./ManaCurve.tsx";
import { DB_SIDEBAR, DECK_CARDS, DECK_COUNT, DECK_DROP, DECK_FOLD, deckCardId } from "./testids.ts";
import type { Holder } from "./workshop.ts";

type DeckSidebarProps = {
  deckName: string;
  cards: readonly string[];
  catalog: CatalogSnapshot;
  conflicts: ReadonlyMap<string, Holder>;
  head: ReactNode;
  onDropCard: (event: DragEvent<HTMLElement>) => void;
  onRemove: (cardId: string) => void;
  onInspect: (cardId: string) => void;
  children?: ReactNode;
};

const FULL_PERCENT = 100;

const EDGE_SLACK_PX = 1;

/**
 * Marks hidden deck-list ends for CSS fades. Hidden panels and jsdom report `none`.
 */
function useScrollEdges(ref: RefObject<HTMLElement | null>, count: number): void {
  useLayoutEffect(() => {
    const element = ref.current;
    if (element === null) return undefined;
    const update = (): void => {
      const hidden = element.scrollHeight - element.clientHeight;
      const top = element.scrollTop > EDGE_SLACK_PX;
      const bottom = element.scrollTop < hidden - EDGE_SLACK_PX;
      const more = top && bottom ? "both" : top ? "top" : bottom ? "bottom" : "none";
      if (element.dataset.more !== more) element.dataset.more = more;
    };
    update();
    element.addEventListener("scroll", update, { passive: true });
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(update);
    observer?.observe(element);
    return () => {
      element.removeEventListener("scroll", update);
      observer?.disconnect();
    };
  }, [ref, count]);
}

/**
 * Duplicate-safe keys keep existing tiles mounted so only inserted tiles replay their entrance.
 */
function tileKeys(ordered: readonly string[]): [string, string][] {
  const seen = new Map<string, number>();
  return ordered.map((cardId) => {
    const copy = seen.get(cardId) ?? 0;
    seen.set(cardId, copy + 1);
    return [cardId, `${cardId}:${String(copy)}`];
  });
}

/** `preventDefault` marks a drop target, enabling the browser's `drop` event. */
function allowDrop(event: DragEvent<HTMLElement>): void {
  event.preventDefault();
  try {
    event.dataTransfer.dropEffect = "move";
  } catch {
    // Synthesised events lack DataTransfer; `preventDefault` enables drops.
  }
}

function tileCost(cost: CardCost | undefined): string {
  if (cost === undefined) return "";
  if (typeof cost === "number") return String(cost);
  if (cost === "X") return cost;
  return String(cost.base);
}

type DeckTileProps = {
  deckName: string;
  cardId: string;
  def: CardDef | undefined;
  conflict: Holder | undefined;
  onRemove: (cardId: string) => void;
  onInspect: (cardId: string) => void;
};

export { isInspectKey };

function DeckTile({ deckName, cardId, def, conflict, onRemove, onInspect }: DeckTileProps): ReactElement {
  const name = def?.name ?? cardId;
  const face = useMemo(
    () => (def === undefined ? null : faceModel({ defId: cardId, def, radiant: false })),
    [cardId, def],
  );
  const inspect = useInspectTrigger(face === null ? null : { key: deckCardId(cardId), face }, {
    onContextMenu: () => {
      onInspect(cardId);
    },
  });

  return (
    <li>
      <button
        type="button"
        className="db-tile"
        data-testid={deckCardId(cardId)}
        data-card={cardId}
        data-rarity={def?.rarity}
        data-conflict={conflict === undefined ? undefined : "true"}
        data-conflict-with={conflict?.name}
        aria-label={
          conflict === undefined
            ? `Remove ${name} from ${deckName}`
            : `Remove ${name} from ${deckName} (also in ${conflict.name})`
        }
        title={conflict === undefined ? undefined : `Also in ${conflict.name}`}
        aria-keyshortcuts="I"
        onClick={() => {
          onRemove(cardId);
        }}
        onKeyDown={(event) => {
          if (def === undefined || !isInspectKey(event)) return;
          event.preventDefault();
          onInspect(cardId);
        }}
        {...inspect.handlers}
      >
        <span className="db-tile-cost" data-digits={tileCost(def?.cost).length >= 3 ? "3" : undefined}>
          {tileCost(def?.cost)}
        </span>
        <span className="db-tile-name">{name}</span>
        <span className="db-tile-art">
          {def === undefined ? null : (
            <CardArt defId={cardId} radiant={false} tags={def.tags} type={def.type} shape="strip" />
          )}
        </span>
        {conflict === undefined ? null : <span className="db-tile-flag" aria-hidden="true" />}
        <span className="db-tile-pip" data-rarity={def?.rarity} aria-hidden="true" />
      </button>
      {inspect.overlay}
    </li>
  );
}

export default function DeckSidebar(props: DeckSidebarProps): ReactElement {
  const { deckName, cards, catalog, conflicts, head, onDropCard, onRemove, onInspect, children } = props;
  const [listOpen, setListOpen] = useState(false);
  const ordered = useMemo(() => deckListOrder(cards, catalog), [cards, catalog]);
  const full = cards.length >= DECK_SIZE;
  const list = useRef<HTMLUListElement>(null);
  useScrollEdges(list, cards.length);
  const listId = useId();

  return (
    <aside className="db-sidebar" data-testid={DB_SIDEBAR} aria-label={deckName}>
      {head}
      <section
        className="db-deck"
        aria-label={`Cards in ${deckName}`}
        data-testid={DECK_DROP}
        data-list-open={listOpen ? "true" : "false"}
        onDragOver={allowDrop}
        onDrop={onDropCard}
      >
        <header className="db-deck-head">
          <span className="db-deck-title">Cards</span>
          <span
            className="db-deck-size"
            data-testid={DECK_COUNT}
            data-count={String(cards.length)}
            data-deck-size={String(DECK_SIZE)}
            data-full={full ? "true" : "false"}
          >
            {`${String(cards.length)}/${String(DECK_SIZE)}`}
          </span>
          {cards.length === 0 ? null : (
            <button
              type="button"
              className="db-deck-fold"
              data-testid={DECK_FOLD}
              aria-expanded={listOpen}
              aria-controls={listId}
              onClick={() => {
                setListOpen((open) => !open);
              }}
            >
              {listOpen ? "Hide list" : "Show list"}
            </button>
          )}
          {/* How full the deck is, at a glance. Drawn only: the count beside it is the number. */}
          <span className="db-deck-meter" data-full={full ? "true" : "false"} aria-hidden="true">
            <span
              className="db-deck-meter-fill"
              style={{ width: `${String(Math.min(1, cards.length / DECK_SIZE) * FULL_PERCENT)}%` }}
            />
          </span>
        </header>
        <ManaCurve cardIds={cards} catalog={catalog} />
        <ul ref={list} id={listId} className="db-deck-list" data-testid={DECK_CARDS}>
          {tileKeys(ordered).map(([cardId, key]) => (
            <DeckTile
              key={key}
              deckName={deckName}
              cardId={cardId}
              def={catalog.cards[cardId]}
              conflict={conflicts.get(cardId)}
              onRemove={onRemove}
              onInspect={onInspect}
            />
          ))}
        </ul>
        {cards.length === 0 ? (
          <p className="db-deck-hint">Tap + on a card in the pool, or drag it here, to add it.</p>
        ) : null}
      </section>

      {children}
    </aside>
  );
}
