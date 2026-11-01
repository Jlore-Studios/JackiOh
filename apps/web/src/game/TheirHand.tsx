// "Their hand" on the game-over screen (R434): the opponent's hand as the finished game shows it
// (`reveal.ts`), each card a face that opens large. Nothing here reads anything but the view's own
// cards and the public catalog (CLAUDE.md rule 7).
//
// - `TheirHand`: the result panel's section. Each face opens its preview on a resting mouse and its
//   inspect sheet on a click, a tap, a long-press or Enter, as a card in a pile's list does.
// - `TheirHandButton`: the result chip's "Their hand", which opens the whole hand in the list dialog
//   (`CardListSheet`). Practice keeps the chip from the start, so this is its way in.
//
// R1834004: the cards are the opponent's, so they wear that seat's loadout where the board carries one
// (game/wearer.ts), faces and backs alike, as they did in their hand.

import { useContext, useState, type CSSProperties, type ReactElement } from "react";

import type { CardView, PlayerView } from "@jackioh/shared";

import { CardBack, CardFace, CardListSheet, useInspectTrigger, type CardListEntry, type FaceModel } from "../cards/index.ts";
import { DEFAULT_CARD_COSMETICS } from "../cards/cosmetics.ts";
import { TheirCopy } from "../cards/loadouts.ts";
import { CatalogContext, type CardLookup } from "./catalog.ts";
import { listedFace } from "./faces.ts";
import { revealTestid } from "./reveal.ts";
import { useSeatDeckCosmetics, useTheirLoadout } from "./wearer.ts";
import { t } from "../i18n/i18n.ts";
import "./reveal.css";

/** R434: what the section and the dialog are called. */
export const THEIR_HAND_TITLE = "Their hand";

/** What the list says of its order: the hand's own, left to right. */
const THEIR_HAND_ORDER = "As held";

/** R434: the revealed hand and the view its cards are read in. */
export type TheirHandProps = { view: PlayerView; cards: readonly CardView[] };

/**
 * Each card's face in play (faces.ts), or a back for a card the view names by the sentinel alone,
 * wearing `back` (R1834004: the opponent's seat's card back, absent for the default).
 */
export function theirHandEntries(lookup: CardLookup | null, view: PlayerView, cards: readonly CardView[], back?: string): CardListEntry[] {
  return cards.map((card) => ({ key: card.instanceId, face: listedFace(lookup, view, card), ...(back === undefined ? {} : { back }) }));
}

/** R1834004: the card back the opponent's seat wears, absent for the default (a back names no card, B16). */
function useTheirBack(view: PlayerView): string | undefined {
  const { cardBack } = useSeatDeckCosmetics(view.opponent.player);
  return cardBack === DEFAULT_CARD_COSMETICS.cardBack ? undefined : cardBack;
}

function cardsWord(count: number): string {
  return count === 1 ? t("· 1 card") : t("· {count} cards", { count: String(count) });
}

function TheirCard({
  card,
  face,
  index,
  back,
}: {
  card: CardView;
  face: FaceModel | null;
  index: number;
  back?: string;
}): ReactElement {
  // Lines of code is a hidden stat in matches.
  const inspect = useInspectTrigger(face === null ? null : { key: `their-hand-${card.instanceId}`, face }, { prefer: "above", showLoc: false });
  const style = { "--i": index } as CSSProperties;
  if (face === null) {
    return (
      <li className="their-hand__item" style={style}>
        <span className="their-hand__card" data-testid={revealTestid.theirHandCard(card.instanceId)} role="img" aria-label={t("Unknown card")}>
          <CardBack back={back} />
        </span>
      </li>
    );
  }
  return (
    <li className="their-hand__item" style={style}>
      <button
        type="button"
        className="their-hand__card"
        data-testid={revealTestid.theirHandCard(card.instanceId)}
        data-def-name={face.name}
        aria-label={t("{name}: show it large", { name: face.name })}
        {...inspect.handlers}
        onClick={(event) => {
          inspect.openSheet(event.currentTarget);
        }}
      >
        <CardFace face={face} layout="full" />
      </button>
      {inspect.overlay}
    </li>
  );
}

/** R434: the result panel's "Their hand", every card a face that opens large. */
export function TheirHand({ view, cards }: TheirHandProps): ReactElement {
  const lookup = useContext(CatalogContext);
  const entries = theirHandEntries(lookup, view, cards);
  const back = useTheirBack(view);
  const theirs = useTheirLoadout(view.opponent.player);
  return (
    <TheirCopy loadout={theirs}>
      <section className="their-hand" data-testid={revealTestid.theirHand} data-count={cards.length} aria-label={t(THEIR_HAND_TITLE)}>
        <h3 className="their-hand__title">
          {t(THEIR_HAND_TITLE)} <span className="their-hand__count">{cardsWord(cards.length)}</span>
        </h3>
        {cards.length === 0 ? (
          <p className="their-hand__empty">{t("Their hand was empty.")}</p>
        ) : (
          <ul className="their-hand__cards">
            {cards.map((card, index) => (
              <TheirCard key={card.instanceId} card={card} face={entries[index]?.face ?? null} index={index} back={back} />
            ))}
          </ul>
        )}
      </section>
    </TheirCopy>
  );
}

/** R434: the result chip's way to the revealed hand, in the list dialog; nothing for an empty hand. */
export function TheirHandButton({ view, cards }: TheirHandProps): ReactElement | null {
  const lookup = useContext(CatalogContext);
  const [open, setOpen] = useState(false);
  const back = useTheirBack(view);
  const theirs = useTheirLoadout(view.opponent.player);
  if (cards.length === 0) return null;
  return (
    <>
      <button
        type="button"
        className="result-overlay__their-hand"
        data-testid={revealTestid.theirHandOpen}
        data-count={cards.length}
        aria-haspopup="dialog"
        onClick={() => {
          setOpen(true);
        }}
      >
        {t(THEIR_HAND_TITLE)}
      </button>
      {open ? (
        <TheirCopy loadout={theirs}>
          <CardListSheet
            title={t(THEIR_HAND_TITLE)}
            entries={theirHandEntries(lookup, view, cards, back)}
            order={t(THEIR_HAND_ORDER)}
            onClose={() => {
              setOpen(false);
            }}
          />
        </TheirCopy>
      ) : null}
    </>
  );
}
