// The deck one Assault attempt carries: an account's saved decks as practice offers them, the
// three practice presets, or the deterministic day deck. The validator supplies every refusal.

import { useMemo, type ReactElement } from "react";

import { DECK_SIZE } from "@jackioh/engine/config";
import type { AssaultDeckSource } from "@jackioh/run";

import { readerName } from "../../cards/chinese.ts";
import { printedPrice } from "../../game/deckbuilder/filters.ts";
import { t, useLocale } from "../../i18n/i18n.ts";
import {
  PRACTICE_PRESETS,
  isPlayableSavedDeck,
  savedDeckRefusal,
  type PracticeSavedDeck,
} from "../../practice/decks.ts";
import type { SavedDecks } from "../../practice/PracticeSetup.tsx";
import { aiConstants, assaultDayDeck, registeredCatalog } from "../../wasm/index.ts";
import { assaultTestid } from "../testids.ts";

export type AssaultDeckChoice = {
  key: string;
  name: string;
  cards: readonly string[];
  source: AssaultDeckSource;
};

export function dayDeckChoice(date: string): AssaultDeckChoice {
  return { key: `day:${date}`, name: t("Today's Assault deck"), cards: assaultDayDeck(date), source: "dayDeck" };
}

type DeckPickerProps = {
  date: string;
  saved: SavedDecks;
  choice: AssaultDeckChoice;
  onChange(choice: AssaultDeckChoice): void;
};

function savedReason(deck: PracticeSavedDeck): string | null {
  const refusal = savedDeckRefusal(deck);
  if (refusal !== null) return t(refusal);
  if (isPlayableSavedDeck(deck)) return null;
  return t("{count} of {total} cards, not complete", {
    count: String(deck.cards.length),
    total: String(DECK_SIZE),
  });
}

function SavedChoices({ saved, choice, onChange }: Omit<DeckPickerProps, "date">): ReactElement {
  let body: ReactElement;
  switch (saved.kind) {
    case "ready":
      body = (
        <div className="assault-deck-options">
          {saved.decks.map((deck, index) => {
            const reason = savedReason(deck);
            const option: AssaultDeckChoice = {
              key: `saved:${String(index)}`,
              name: deck.name,
              cards: deck.cards,
              source: "saved",
            };
            return (
              <label key={option.key} className="assault-deck-option" data-disabled={reason === null ? undefined : "true"}>
                <input
                  type="radio"
                  name="assault-deck"
                  checked={choice.key === option.key}
                  disabled={reason !== null}
                  data-testid={assaultTestid.option("saved", index)}
                  onChange={() => {
                    onChange(option);
                  }}
                />
                <span>
                  <span>{deck.name}</span>
                  {reason === null ? null : <span className="assault-deck-reason">{reason}</span>}
                </span>
              </label>
            );
          })}
        </div>
      );
      break;
    case "anonymous":
      body = <p>{t("Sign in and save a deck to bring one of your own decks.")}</p>;
      break;
    case "checking":
      body = <p>{t("Looking for your saved decks…")}</p>;
      break;
    case "inactive":
      body = <p>{t("Your saved decks show up once your account is active.")}</p>;
      break;
    case "none":
      body = <p>{t("You have no saved decks yet.")}</p>;
      break;
    case "unavailable":
      body = <p>{t("Your saved decks could not be loaded just now. The other decks still play.")}</p>;
      break;
  }
  return (
    <fieldset className="assault-block" data-testid={assaultTestid.deckSaved}>
      <legend>{t("Saved decks")}</legend>
      {body}
    </fieldset>
  );
}

type Cost = keyof ReturnType<typeof aiConstants>["AI_DECK"]["curve"];

function curveColumn(cost: number | "X"): Cost {
  if (cost === "X" || cost >= 4) return "4+";
  if (cost <= 1) return "0-1";
  return String(cost) as Cost;
}

function DeckPreview({ choice }: { choice: AssaultDeckChoice }): ReactElement {
  useLocale();
  const preview = useMemo(() => {
    const defs = registeredCatalog();
    const columns = Object.keys(aiConstants().AI_DECK.curve) as Cost[];
    const counts = Object.fromEntries(columns.map((column) => [column, 0])) as Record<Cost, number>;
    const cards = choice.cards.map((id, index) => {
      const def = defs[id];
      const cost = def === undefined ? 0 : printedPrice(def.cost);
      counts[curveColumn(cost)] += 1;
      return { key: `${id}-${String(index)}`, name: def === undefined ? id : readerName(def), cost };
    });
    return { cards, columns, counts };
  }, [choice]);
  return (
    <section className="assault-block assault-deck-preview">
      <h2>{choice.name}</h2>
      <div className="assault-curve" data-testid={assaultTestid.curve}>
        {preview.columns.map((column) => (
          <span key={column} data-cost={column} data-count={preview.counts[column]}>
            {t("{cost}: {count}", { cost: column, count: String(preview.counts[column]) })}
          </span>
        ))}
      </div>
      <ol className="assault-card-list" aria-label={t("{name}: {count} cards", { name: choice.name, count: String(choice.cards.length) })}>
        {preview.cards.map((card) => (
          <li key={card.key}>
            <span>{card.cost}</span> {card.name}
          </li>
        ))}
      </ol>
    </section>
  );
}

export function DeckPicker({ date, saved, choice, onChange }: DeckPickerProps): ReactElement {
  const day = useMemo(() => dayDeckChoice(date), [date]);
  return (
    <section className="assault-picker">
      <h2>{t("Choose your deck")}</h2>
      <SavedChoices saved={saved} choice={choice} onChange={onChange} />
      <fieldset className="assault-block" data-testid={assaultTestid.deckPreset}>
        <legend>{t("Practice decks")}</legend>
        <div className="assault-deck-options">
          {PRACTICE_PRESETS.map((preset, index) => {
            const option: AssaultDeckChoice = {
              key: `preset:${preset.id}`,
              name: t(preset.name),
              cards: preset.cards,
              source: "preset",
            };
            return (
              <label key={preset.id} className="assault-deck-option">
                <input
                  type="radio"
                  name="assault-deck"
                  checked={choice.key === option.key}
                  data-testid={assaultTestid.option("preset", index)}
                  onChange={() => {
                    onChange(option);
                  }}
                />
                <span>{t(preset.name)}</span>
              </label>
            );
          })}
        </div>
      </fieldset>
      <fieldset className="assault-block" data-testid={assaultTestid.deckDay}>
        <legend>{t("Daily deck")}</legend>
        <label className="assault-deck-option">
          <input
            type="radio"
            name="assault-deck"
            checked={choice.key === day.key}
            data-testid={assaultTestid.option("day", 0)}
            onChange={() => {
              onChange(day);
            }}
          />
          <span>
            <span>{day.name}</span>
            <span className="assault-deck-reason">{t("The same deck for everyone today.")}</span>
          </span>
        </label>
      </fieldset>
      <DeckPreview choice={choice} />
    </section>
  );
}
