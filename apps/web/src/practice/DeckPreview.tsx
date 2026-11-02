// What the chosen deck is, before Start. The card
// data is the catalog the worker sends (§5.1: the catalog is public), so this page imports none of its own;
// until it arrives, or when it cannot be read, the preview says so and Start still works.

import { useMemo, type ReactElement } from "react";

import type { CardDef, CardDefs } from "@jackioh/shared";

import { TryAgainButton } from "../auth/AccountError.tsx";
import { faceModel, useInspectTrigger } from "../cards/index.ts";
import { readerName } from "../cards/chinese.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { practiceTestid } from "./testids.ts";
import { printedPrice } from "../game/deckbuilder/filters.ts";

/** The curve's columns, Hearthstone's way: one per cost up to the last, which holds the rest. */
const CURVE_COLUMNS = ["0", "1", "2", "3", "4", "5+"] as const;
type CurveColumn = (typeof CURVE_COLUMNS)[number] | "X";

function columnOf(cost: number | "X"): CurveColumn {
  if (cost === "X") return "X";
  const last = CURVE_COLUMNS.length - 1;
  return CURVE_COLUMNS[Math.max(0, Math.min(cost, last))] ?? "5+";
}

type PreviewCard = { id: string; name: string; type: string; cost: number | "X"; def?: CardDef };

function previewCards(ids: readonly string[], defs: CardDefs): PreviewCard[] {
  const cards = ids.map((id): PreviewCard => {
    const def: CardDef | undefined = defs[id];
    if (def === undefined) return { id, name: id, type: "", cost: 0 };
    return { id, name: readerName(def), type: def.type, cost: printedPrice(def.cost), def };
  });
  // Cheapest first, X last, then by name: the order a deck list reads in.
  const rank = (cost: number | "X"): number => (cost === "X" ? Number.POSITIVE_INFINITY : cost);
  return cards.sort((a, b) => rank(a.cost) - rank(b.cost) || a.name.localeCompare(b.name));
}

export type DeckPreviewProps = {
  title: string;
  identity: string;
  /** The deck's card ids; null for the random deck, which is dealt only when the game starts. */
  cards: readonly string[] | null;
  /** The worker's catalog; null while it loads or when it could not be read. */
  defs: CardDefs | null;
  /** The catalog could not be read, so there is nothing to wait for. */
  defsFailed: boolean;
  /** #1691: asks for the catalog again; offered beside the failure when given. */
  onRetry?: () => void;
};

export function DeckPreview({ title, identity, cards, defs, defsFailed, onRetry }: DeckPreviewProps): ReactElement {
  let body: ReactElement | null = null;
  if (cards !== null && defs === null) {
    body = defsFailed ? (
      <p className="deck-preview__status">
        {t("The card list could not be loaded; the deck still plays.")}
        {onRetry !== undefined ? (
          <>
            {" "}
            <TryAgainButton onRetry={onRetry} testId={practiceTestid.defsRetry} />
          </>
        ) : null}
      </p>
    ) : (
      <div className="deck-preview__status">
        <Skeleton variant="rows" label={t("Loading the card list…")} />
      </div>
    );
  } else if (cards !== null && defs !== null) {
    const list = previewCards(cards, defs);
    const counts = new Map<CurveColumn, number>();
    for (const card of list) counts.set(columnOf(card.cost), (counts.get(columnOf(card.cost)) ?? 0) + 1);
    const columns: CurveColumn[] = [...CURVE_COLUMNS, ...(counts.has("X") ? (["X"] as const) : [])];
    const tallest = Math.max(1, ...counts.values());
    body = (
      <>
        <div className="deck-preview__curve" data-testid={practiceTestid.deckCurve} role="img" aria-label={curveLabel(columns, counts)}>
          {columns.map((column) => {
            const count = counts.get(column) ?? 0;
            return (
              <span key={column} className="deck-preview__bar" data-cost={column} data-count={count}>
                <span className="deck-preview__bar-count">{count}</span>
                <span className="deck-preview__bar-fill" style={{ height: `${String(Math.round((count / tallest) * 100))}%` }} />
                <span className="deck-preview__bar-cost">{column}</span>
              </span>
            );
          })}
        </div>
        <ol className="deck-preview__cards" aria-label={t("{title}: {count} cards", { title, count: String(list.length) })}>
          {list.map((card) => (
            <PreviewRow key={card.id} card={card} />
          ))}
        </ol>
      </>
    );
  }

  return (
    <section className="deck-preview" data-testid={practiceTestid.deckPreview} aria-live="polite">
      <h3 className="deck-preview__title">{title}</h3>
      <p className="deck-preview__identity">{identity}</p>
      {body}
    </section>
  );
}

/** One card of the list: hover and long-press inspect it as the deck builder's list does (docs/polish/6-cards.md). */
function PreviewRow({ card }: { card: PreviewCard }): ReactElement {
  const def = card.def;
  const locale = useLocale();
  const face = useMemo(
    () => (def === undefined ? null : faceModel({ defId: card.id, def, radiant: false })),
    [card.id, def, locale],
  );
  const inspect = useInspectTrigger(face === null ? null : { key: `deck-preview-${card.id}`, face });
  return (
    <li
      className="deck-preview__card"
      data-testid={practiceTestid.deckCard(card.id)}
      data-type={card.type}
      data-rarity={def?.rarity}
      {...inspect.handlers}
    >
      <span className="deck-preview__cost" aria-label={t("({cost}) Cost", { cost: String(card.cost) })}>
        {card.cost}
      </span>
      <span className="deck-preview__name">{card.name}</span>
      {inspect.overlay}
    </li>
  );
}

function curveLabel(columns: readonly CurveColumn[], counts: ReadonlyMap<CurveColumn, number>): string {
  const parts = columns.map((column) => t("{count} at {cost}", { count: String(counts.get(column) ?? 0), cost: column }));
  return t("Mana curve: {columns}", { columns: parts.join(t(", ")) });
}
