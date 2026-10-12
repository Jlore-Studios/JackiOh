// Concurrent Conquest picks mirror the mulligan: confirm seals them, only pick status is revealed,
// and a shared server clock runs (SPEC §9.5, R263, R265, R266, R331–R333, R338; CLAUDE.md rule 7).

import { useEffect, useId, useState, type ReactElement } from "react";

import { t } from "../i18n/i18n.ts";
import type { SeriesView } from "../net/api.ts";
import "../auth/tavern.css";
import "./lobby.css";

/** Test IDs mirrored by `e2e/support/testids.ts`. */
export const seriesPickerTestid = {
  /** Panel state and auto-pick flags. */
  picker: "series-picker",
  /** A deck selector, locked after it wins (R330). */
  choice: (slot: number): string => `series-pick-${String(slot)}`,
  /** Seals the pick (R331). */
  lockIn: "series-lock-in",
  /** Pick-clock seconds remaining (R333). */
  clock: "series-pick-clock",
  /** Opponent pick status (R331). */
  opponentStatus: "series-opponent-status",
} as const;

export type SeriesPickerProps = {
  view: SeriesView;
  secondsLeft: number;
  busy: boolean;
  onLockIn: (slot: number) => void;
};

export function deckStanding(deck: SeriesView["you"]["decks"][number]): string {
  if (deck.won) return t("Won · locked");
  if (deck.games === 0) return t("Not played yet");
  return deck.games === 1 ? t("Played once, not won") : t("Played {count} times, not won", { count: String(deck.games) });
}

function OpponentStatus({ picked }: { picked: boolean }): ReactElement {
  return (
    <p
      className="series-picker__opponent"
      data-testid={seriesPickerTestid.opponentStatus}
      data-picked={picked ? "true" : "false"}
      role="status"
    >
      {picked ? t("Opponent has picked") : t("Opponent is choosing…")}
    </p>
  );
}

function Clock({ secondsLeft }: { secondsLeft: number }): ReactElement {
  return (
    <p className="series-picker__clock">
      {t("Time left to pick:")}{" "}
      <span className="series-clock" data-testid={seriesPickerTestid.clock} data-seconds={secondsLeft}>
        {t("{seconds} s", { seconds: String(secondsLeft) })}
      </span>
    </p>
  );
}

function DeckIcon(): ReactElement {
  return (
    <svg viewBox="0 0 32 32" aria-hidden="true" className="play-mode-tile__icon">
      <path d="M8 6l8-3 8 3v9c0 6-3.6 10-8 12-4.4-2-8-6-8-12z" />
    </svg>
  );
}

export default function SeriesPicker({ view, secondsLeft, busy, onLockIn }: SeriesPickerProps): ReactElement {
  const titleId = useId();
  const decks = [...view.you.decks].sort((a, b) => a.slot - b.slot);
  const open = decks.filter((deck) => !deck.won);
  // The server auto-picks the final deck (R332).
  const [selected, setSelected] = useState<number | null>(null);
  const pick = view.you.pick;

  // Clear a selection invalidated by a server update.
  const openSlots = open.map((deck) => deck.slot).join(",");
  useEffect(() => {
    if (selected !== null && !openSlots.split(",").includes(String(selected))) setSelected(null);
  }, [selected, openSlots]);

  if (pick !== null) {
    const name = decks.find((deck) => deck.slot === pick)?.name ?? t("Deck {number}", { number: String(pick + 1) });
    return (
      <section
        className="lobby-card play-panel series-picker"
        data-testid={seriesPickerTestid.picker}
        data-state="waiting"
        data-auto={view.you.autoPick ? "true" : "false"}
        aria-labelledby={titleId}
      >
        <h2 id={titleId} className="play-panel__heading">
          {t("Game {game}: your deck is in", { game: String(view.gameNo) })}
        </h2>
        <div className="play-search">
          <span className="play-search__beacon" aria-hidden="true">
            <span className="play-search__ring" />
            <span className="play-search__ring play-search__ring--late" />
          </span>
          <span className="play-search__text">{t("Waiting for your opponent…")}</span>
        </div>
        <p className="series-picker__sealed" role="status">
          {view.you.autoPick
            ? t("{name} is your last deck that hasn’t won, so it was picked for you for game {game}.", {
                name,
                game: String(view.gameNo),
              })
            : t("You’re playing {name} in game {game}. Your pick is sealed: your opponent sees only that you’ve picked.", {
                name,
                game: String(view.gameNo),
              })}
        </p>
        <OpponentStatus picked={view.opponent.picked} />
        <Clock secondsLeft={secondsLeft} />
      </section>
    );
  }

  const chosen = decks.find((deck) => deck.slot === selected) ?? null;
  return (
    <section
      className="lobby-card play-panel series-picker"
      data-testid={seriesPickerTestid.picker}
      data-state="choosing"
      data-auto="false"
      aria-labelledby={titleId}
    >
      <h2 id={titleId} className="play-panel__heading">
        <span className="play-step" aria-hidden="true">
          1
        </span>
        {t("Game {game}: choose your deck", { game: String(view.gameNo) })}
      </h2>
      <p className="lobby-note">
        {t(
          "Pick a deck that hasn’t won yet. Once you lock it in it’s final, and your opponent sees only that you’ve picked — neither of you sees the other’s deck until the game starts. If the clock runs out, your first deck that hasn’t won is picked for you.",
        )}
      </p>
      <fieldset className="lobby-modes series-picker__decks">
        <legend className="lobby-modes__legend">{t("Your decks")}</legend>
        {decks.map((deck) => {
          const isSelected = selected === deck.slot;
          return (
            <label
              key={deck.slot}
              className="lobby-mode play-mode-tile series-picker__deck"
              data-selected={isSelected ? "true" : "false"}
              data-won={deck.won ? "true" : "false"}
            >
              <input
                type="radio"
                name="series-pick"
                value={String(deck.slot)}
                checked={isSelected}
                disabled={deck.won || busy}
                data-testid={seriesPickerTestid.choice(deck.slot)}
                onChange={() => {
                  setSelected(deck.slot);
                }}
              />
              <DeckIcon />
              <span className="lobby-mode__text">
                <span className="lobby-mode__name">{deck.name}</span>
                <span className="lobby-mode__hint">{t("{count} cards", { count: String(deck.cards.length) })}</span>
              </span>
              <span className="play-mode-tile__waiting series-picker__standing" data-won={deck.won ? "true" : "false"}>
                {deckStanding(deck)}
              </span>
            </label>
          );
        })}
      </fieldset>
      <h2 className="play-panel__heading play-panel__heading--sub">
        <span className="play-step" aria-hidden="true">
          2
        </span>
        {t("Lock it in")}
      </h2>
      <button
        type="button"
        className="button-primary play-cta"
        data-testid={seriesPickerTestid.lockIn}
        disabled={chosen === null || busy}
        onClick={() => {
          if (chosen !== null) onLockIn(chosen.slot);
        }}
      >
        {chosen === null ? t("Choose a deck") : t("Lock in {name}", { name: chosen.name })}
      </button>
      <OpponentStatus picked={view.opponent.picked} />
      <Clock secondsLeft={secondsLeft} />
    </section>
  );
}
