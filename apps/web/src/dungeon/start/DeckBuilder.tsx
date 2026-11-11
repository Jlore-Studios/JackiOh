// The start screen's build-your-own deck builder: the deck workshop's browse pane
// (`game/deckbuilder/CardBrowser.tsx`: filter bar, search, sort, pool grid and a card's detail
// view) limited to the run's unlocked cards, beside the 10-card deck and a per-rarity slots meter.
// Presentation only (CLAUDE.md rule 7): it offers the unlocked cards and counts the picks; the run
// still judges the setup through `runCheckSetup`, whose refusal the screen shows word for word.

import { useMemo, useState, type ReactElement } from "react";

import { CATALOG } from "@jackioh/cards";
import type { RunPoolCard } from "@jackioh/run";
import type { CatalogSnapshot } from "@jackioh/validator";

import { CardDefsProvider } from "../../cards/index.ts";
import CardBrowser from "../../game/deckbuilder/CardBrowser.tsx";
import {
  DEFAULT_FILTER,
  DEFAULT_SORT,
  matchesFilter,
  sortPool,
  type PoolFilter,
  type PoolSort,
} from "../../game/deckbuilder/filters.ts";
import { t } from "../../i18n/i18n.ts";
import { dungeonTestid } from "../testids.ts";
import { PoolFace } from "./PoolFace.tsx";

export type DeckBuilderProps = {
  /** Every card the run unlocks, in the pool's order. */
  offered: readonly RunPoolCard[];
  /** The deck's card ids now. */
  picked: readonly string[];
  onToggle: (id: string) => void;
  /** The level's slots, from the lowest. */
  slots: readonly number[];
  /** `RUN_STARTER_SIZE`: the deck's size. */
  starterSize: number;
};

/** DR3: the slots' ids, from the lowest, which are `RUN_SLOT_LADDER`'s columns. */
export const SLOT_IDS = ["Common", "Rare", "Epic", "Legendary", "Mythic", "Anything"] as const;

/** DR3: a slot's name as the player reads it. */
export function slotName(column: number): string {
  switch (column) {
    case 0:
      return t("Common");
    case 1:
      return t("Rare");
    case 2:
      return t("Epic");
    case 3:
      return t("Legendary");
    case 4:
      return t("Mythic");
    default:
      return t("Anything");
  }
}

/** A picked card's slot column: its rarity's, or Anything when the pool names none. */
function slotOf(card: RunPoolCard): number {
  const column = SLOT_IDS.indexOf(card.rarity);
  return column < 0 ? SLOT_IDS.length - 1 : column;
}

export function DeckBuilder({ offered, picked, onToggle, slots, starterSize }: DeckBuilderProps): ReactElement {
  const [filter, setFilter] = useState<PoolFilter>(DEFAULT_FILTER);
  const [sort, setSort] = useState<PoolSort>(DEFAULT_SORT);
  const [detailCardId, setDetailCardId] = useState<string | null>(null);

  // The bundled catalog: the browser reads the faces off it, as the Almanac route does.
  const catalog = useMemo<CatalogSnapshot>(() => ({ version: "bundled", cards: CATALOG }), []);
  const pool = useMemo(() => {
    const ids = offered
      .map((card) => card.id)
      .filter((id) => {
        const def = catalog.cards[id];
        return def !== undefined && matchesFilter(def, filter);
      });
    return sortPool(ids, catalog, sort);
  }, [offered, catalog, filter, sort]);
  const byId = useMemo(() => new Map(offered.map((card) => [card.id, card])), [offered]);
  const detailCard = detailCardId === null ? undefined : byId.get(detailCardId);
  const filled = useMemo(() => {
    const counts = slots.map(() => 0);
    for (const id of picked) {
      const card = byId.get(id);
      if (card !== undefined) counts[slotOf(card)] = (counts[slotOf(card)] ?? 0) + 1;
    }
    return counts;
  }, [picked, byId, slots]);

  return (
    <section aria-label={t("Build your own")}>
      <h3>{t("Build your own")}</h3>
      <div className="dungeon-builder">
        {/* The workshop's scope: its button rules are scoped `.deckbuilder.app-shell`, and the
            dungeon's own rules below keep this wrapper off the viewport's height. */}
        <div className="dungeon-browser deckbuilder app-shell">
          <CardDefsProvider defs={catalog.cards}>
            <CardBrowser
              catalog={catalog}
              pool={pool}
              filter={filter}
              onFilter={setFilter}
              sort={sort}
              onSort={setSort}
              ownedControl={false}
              onInspect={setDetailCardId}
              detail={
                detailCard === undefined
                  ? null
                  : {
                      cardId: detailCard.id,
                      actions: (
                        <button
                          type="button"
                          className="button-secondary"
                          onClick={() => {
                            onToggle(detailCard.id);
                            setDetailCardId(null);
                          }}
                        >
                          {picked.includes(detailCard.id) ? t("Remove") : t("Add")}
                        </button>
                      ),
                    }
              }
              onCloseDetail={() => {
                setDetailCardId(null);
              }}
            />
          </CardDefsProvider>
        </div>
        <div className="dungeon-deck">
          <h4>{t("Your deck")}</h4>
          <p data-testid={dungeonTestid.chosenCount}>
            {t("Cards: {chosen} of {size}", { chosen: String(picked.length), size: String(starterSize) })}
          </p>
          <ul className="dungeon-cards" data-testid={dungeonTestid.deck}>
            {picked.map((id) => {
              const card = byId.get(id);
              return card === undefined ? null : (
                <li key={id}>
                  <button
                    type="button"
                    className="dungeon-card"
                    aria-pressed="true"
                    aria-label={card.name}
                    data-testid={dungeonTestid.card(id)}
                    onClick={() => {
                      onToggle(id);
                    }}
                  >
                    <PoolFace card={card} />
                  </button>
                </li>
              );
            })}
          </ul>
          <ul className="dungeon-meter" data-testid={dungeonTestid.slotsMeter}>
            {slots.map((count, column) =>
              count === 0 ? null : (
                <li key={column}>
                  {t("{slot} {filled} of {count}", {
                    slot: slotName(column),
                    filled: String(filled[column] ?? 0),
                    count: String(count),
                  })}
                </li>
              ),
            )}
          </ul>
        </div>
      </div>
    </section>
  );
}
