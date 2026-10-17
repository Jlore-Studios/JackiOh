// Practice setup maps difficulty and deck selection to Start (SPEC §9.9).
// AI resource tiers derive from `AI_DIFFICULTY` (CLAUDE.md rule 9).
// R1373, R924000: only the player's Random deck can lean toward a set; retain its pick.
// R915005: a saved deck that names custom cards starts with them (`customCards`), drawn in the preview
// from their frames; one that names a card the account no longer has says so and does not start.
// §9.9: a complete saved deck the worker would refuse is listed disabled, with its reason in the deck hint and a link to Decks (`savedDeckRefusal`).

import { useId, useMemo, useState, type FormEvent, type ReactElement } from "react";

import {
  AI_DIFFICULTY,
  DECK_SIZE,
  DIFFICULTIES,
  DRAWS_PER_TURN,
  HUMAN_HANDICAP,
  type Difficulty,
  type Handicap,
} from "@jackioh/engine/config";

import type { CardDefs, CustomCard, LeanPick } from "@jackioh/shared";

import {
  customCardsOfDeck,
  customDefsOf,
  gameCustomCards,
  namesCustomCard,
  type CustomCardsState,
} from "../creator/useCustomCards.ts";
import { LeanSetPicker } from "../game/LeanNewest.tsx";
import { t } from "../i18n/i18n.ts";
import { paths } from "../net/navigate.ts";
import { DeckPreview } from "./DeckPreview.tsx";
import {
  RANDOM_DECK_IDENTITY,
  deckChoiceFromValue,
  deckOptions,
  isPlayableSavedDeck,
  presetById,
  savedDeckOf,
  savedDeckRefusal,
  type PracticeSavedDeck,
} from "./decks.ts";
import type { PracticeDeckChoice } from "./protocol.ts";
import { practiceTestid } from "./testids.ts";
import { DIFFICULTY_LABEL, DIFFICULTY_TAGLINE, TierCrest } from "./Tier.tsx";
import "./practice.css";

/**
 * Selection passed to Start, including the random deck's "More cards from" pick (R1373, R924000) and
 * whether the game is a sandbox with dev tools (R923000).
 */
export type PracticeSetupChoice = {
  difficulty: Difficulty;
  deck: PracticeDeckChoice;
  leanSet?: LeanPick | null;
  sandbox?: boolean;
  /** R915006: the custom cards the deck names, which the game carries (R915009: it runs on the Luau module). */
  customCards?: CustomCard[];
};

/**
 * What the player's account offers the deck picker, and why when it offers nothing, so the hint
 * under the picker can say the right thing: a signed-in player is never told to sign in.
 */
export type SavedDecks =
  /** No session. */
  | { kind: "anonymous" }
  /** The account, or its saved decks, are still being read. */
  | { kind: "checking" }
  /** Signed in, but the account is not active yet (an invite code is owed) or may not play. */
  | { kind: "inactive" }
  /** An active account that has no saved deck. */
  | { kind: "none" }
  /** The account or its saved decks could not be read. */
  | { kind: "unavailable" }
  /** `GET /api/decks`'s decks, oldest first (R250). */
  | { kind: "ready"; decks: readonly PracticeSavedDeck[] };

type PracticeSetupProps = {
  saved: SavedDecks;
  initial: { difficulty: Difficulty; deck: string; leanSet?: LeanPick | null };
  /** The worker's catalog for the deck preview; null while it loads or when it could not be read. */
  defs: CardDefs | null;
  defsFailed: boolean;
  /** §12.9: the account's custom cards, which a saved deck may name; absent, there are none. */
  custom?: CustomCardsState;
  onStart(choice: PracticeSetupChoice): void;
};

const NO_CUSTOM: CustomCardsState = { kind: "none" };
const NO_CARDS: readonly CustomCard[] = [];

/** Why a saved deck that names custom cards cannot start yet, or null when it can. */
function customBlock(custom: CustomCardsState, missing: number): string | null {
  if (custom.kind === "loading") return t("Loading your custom cards…");
  if (custom.kind !== "ready") return t("Your custom cards could not be loaded, so this deck cannot start.");
  if (missing === 0) return null;
  return missing === 1
    ? t("This deck names a custom card you no longer have. Replace it in Decks to play this deck.")
    : t("This deck names {count} custom cards you no longer have. Replace them in Decks to play this deck.", {
        count: String(missing),
      });
}

function savedHint(saved: SavedDecks): ReactElement | null {
  switch (saved.kind) {
    case "ready": {
      // §9.9: name each complete deck the worker would refuse (listed disabled), and where to fix it.
      const reasons = saved.decks.map((deck) => savedDeckRefusal(deck)).filter((reason): reason is string => reason !== null);
      if (reasons.length === 0) return null;
      const [before = "", after = ""] = (
        reasons.length === 1 ? t("Fix it in {decks} to play it here.") : t("Fix them in {decks} to play them here.")
      ).split("{decks}");
      return (
        <>
          {reasons.map((reason) => `${t(reason)} `).join("")}
          {before}
          <a href={paths.decks}>{t("Decks")}</a>
          {after}
        </>
      );
    }
    case "anonymous":
      return <>{t("Sign in and save a deck to play one of your own decks here.")}</>;
    case "checking":
      return <>{t("Looking for your saved decks…")}</>;
    case "inactive":
      return <>{t("Your saved decks show up here once your account is active.")}</>;
    case "none": {
      // Split around the link, so English keeps the text nodes it always had.
      const [before = "", after = ""] = t("Save a deck in {decks} to play one of your own decks here.").split("{decks}");
      return (
        <>
          {before}
          <a href={paths.decks}>{t("Decks")}</a>
          {after}
        </>
      );
    }
    case "unavailable":
      return <>{t("Your saved decks could not be loaded just now. The decks above still play.")}</>;
  }
}

function previewFor(
  choice: PracticeDeckChoice,
  saved: readonly PracticeSavedDeck[] | null,
): { title: string; identity: string; cards: readonly string[] | null } {
  switch (choice.kind) {
    case "random":
      return { title: t("Random deck"), identity: t(RANDOM_DECK_IDENTITY), cards: null };
    case "preset": {
      const preset = presetById(choice.id);
      return preset === undefined
        ? { title: choice.id, identity: "", cards: null }
        : { title: t(preset.name), identity: t(preset.identity), cards: preset.cards };
    }
    case "saved":
      return {
        title: saved?.[choice.index - 1]?.name ?? t("Saved deck {n}", { n: String(choice.index) }),
        identity: t("One of your saved decks, exactly as you saved it."),
        cards: choice.cards,
      };
    case "try":
      return { title: t("Random deck"), identity: t("A random deck with your custom card."), cards: null };
  }
}

function cardCount(count: number): string {
  return count === 1 ? t("{count} card", { count: String(count) }) : t("{count} cards", { count: String(count) });
}

/** A deck option as the reader reads it; a saved deck's name is its player's own and stays as written. */
function optionLabel(value: string, label: string, saved: readonly PracticeSavedDeck[] | null): string {
  const deck = savedDeckOf(value, saved);
  if (deck === undefined) return t(label);
  if (savedDeckRefusal(deck) !== null) return t("{name} (needs a fix)", { name: deck.name });
  if (isPlayableSavedDeck(deck)) return deck.name;
  return t("{name} ({count} of {total} cards, not complete)", {
    name: deck.name,
    count: String(deck.cards.length),
    total: String(DECK_SIZE),
  });
}

type TierStat = { key: string; label: string; value: string; ahead: boolean };

function tierStats(handicap: Handicap): TierStat[] {
  const human = HUMAN_HANDICAP;
  return [
    {
      key: "deck",
      label: t("Deck"),
      value: cardCount(handicap.deckSize),
      ahead: handicap.deckSize !== human.deckSize,
    },
    {
      key: "mana",
      label: t("Mana"),
      value:
        handicap.manaBonus > 0
          ? t("+{bonus}, up to {cap}", { bonus: String(handicap.manaBonus), cap: String(handicap.manaCap) })
          : t("up to {cap}", { cap: String(handicap.manaCap) }),
      ahead: handicap.manaBonus !== human.manaBonus || handicap.manaCap !== human.manaCap,
    },
    {
      key: "hand",
      label: t("Opening hand"),
      value:
        handicap.extraOpeningCards > 0
          ? handicap.extraOpeningCards === 1
            ? t("+{count} card", { count: String(handicap.extraOpeningCards) })
            : t("+{count} cards", { count: String(handicap.extraOpeningCards) })
          : t("as yours"),
      ahead: handicap.extraOpeningCards !== human.extraOpeningCards,
    },
    {
      key: "draws",
      label: t("Draws"),
      value: t("{count} per turn", { count: String(DRAWS_PER_TURN + handicap.extraDrawsPerTurn) }),
      ahead: handicap.extraDrawsPerTurn !== human.extraDrawsPerTurn,
    },
  ];
}

export function PracticeSetup({ saved, initial, defs, defsFailed, custom = NO_CUSTOM, onStart }: PracticeSetupProps): ReactElement {
  const [difficulty, setDifficulty] = useState<Difficulty>(initial.difficulty);
  const [deck, setDeck] = useState<string>(initial.deck);
  const [lean, setLean] = useState<LeanPick | null>(initial.leanSet ?? null);
  const [sandbox, setSandbox] = useState(false);
  const baseId = useId();

  const savedDecks = saved.kind === "ready" ? saved.decks : null;
  const options = deckOptions(savedDecks);
  // A remembered saved deck that is gone, not complete or not loaded yet falls back to random.
  const selected = options.some((option) => option.value === deck && !option.disabled) ? deck : "random";
  const picked: PracticeDeckChoice = deckChoiceFromValue(selected, savedDecks) ?? { kind: "random" };
  // R1373, R924000: only the Random deck leans, and only on a pick.
  const choice: PracticeDeckChoice = picked.kind === "random" && lean !== null ? { kind: "random", leanSet: lean } : picked;
  const preview = previewFor(choice, savedDecks);
  const hint = savedHint(saved);

  // R915005: the custom cards a saved deck names, drawn from their frames beside the catalog.
  const customCards = custom.kind === "ready" ? custom.cards : NO_CARDS;
  const customDefs = useMemo(() => customDefsOf(customCards), [customCards]);
  const previewDefs = useMemo(() => (defs === null ? null : { ...defs, ...customDefs }), [defs, customDefs]);
  const named = choice.kind === "saved" && namesCustomCard(choice.cards) ? customCardsOfDeck(choice.cards, customCards) : null;
  const blocked = named === null ? null : customBlock(custom, named.missing.length);

  function submit(event: FormEvent<HTMLFormElement>): void {
    event.preventDefault();
    if (blocked !== null) return;
    onStart({
      difficulty,
      deck: choice,
      leanSet: lean,
      ...(sandbox ? { sandbox: true } : {}),
      ...(named === null || named.cards.length === 0 ? {} : { customCards: gameCustomCards(named.cards) }),
    });
  }

  return (
    <form className="practice-setup" data-testid={practiceTestid.setup} onSubmit={submit}>
      <fieldset className="practice-setup__difficulties">
        <legend className="practice-setup__legend">
          <h2 className="practice-setup__heading">{t("Choose your opponent")}</h2>
        </legend>
        <div className="practice-tiers">
          {DIFFICULTIES.map((d) => {
            const nameId = `${baseId}-${d}-name`;
            const statsId = `${baseId}-${d}-stats`;
            return (
              <label key={d} className="practice-tier" data-tier={d} data-selected={difficulty === d ? "true" : "false"}>
                <input
                  className="practice-tier__radio"
                  type="radio"
                  name="practice-difficulty"
                  value={d}
                  checked={difficulty === d}
                  onChange={() => {
                    setDifficulty(d);
                  }}
                  aria-labelledby={nameId}
                  aria-describedby={statsId}
                  data-testid={practiceTestid.difficulty(d)}
                />
                <span className="practice-tier__crest">
                  <TierCrest tier={d} />
                </span>
                <span className="practice-tier__heading">
                  <span className="practice-tier__name" id={nameId}>
                    {t(DIFFICULTY_LABEL[d])}
                  </span>
                  <span className="practice-tier__tagline">{t(DIFFICULTY_TAGLINE[d])}</span>
                </span>
                <span className="practice-tier__stats" id={statsId}>
                  {tierStats(AI_DIFFICULTY[d]).map((stat) => (
                    <span key={stat.key} className="practice-tier__stat" data-ahead={stat.ahead ? "true" : "false"}>
                      <span className="practice-tier__stat-label">{stat.label}</span>{" "}
                      <span className="practice-tier__stat-value">{stat.value}</span>
                    </span>
                  ))}
                </span>
              </label>
            );
          })}
        </div>
      </fieldset>

      <div className="practice-setup__footer">
        <div className="practice-setup__deck">
          <h2 className="practice-setup__heading">
            <label className="practice-setup__deck-label" htmlFor={`${baseId}-deck`}>
              {t("Your deck")}
            </label>
          </h2>
          <span className="practice-select">
            <select
              id={`${baseId}-deck`}
              data-testid={practiceTestid.deck}
              value={selected}
              onChange={(event) => {
                setDeck(event.target.value);
              }}
            >
              {options.map((option) => (
                <option key={option.value} value={option.value} disabled={option.disabled}>
                  {optionLabel(option.value, option.label, savedDecks)}
                </option>
              ))}
            </select>
          </span>
          {hint === null ? null : (
            <p className="practice-setup__hint" data-testid={practiceTestid.deckHint}>
              {hint}
            </p>
          )}
          {choice.kind === "random" ? (
            <LeanSetPicker value={lean} testid={practiceTestid.leanNewest} onChange={setLean} />
          ) : null}
          <DeckPreview
            title={preview.title}
            identity={preview.identity}
            cards={preview.cards}
            defs={previewDefs}
            defsFailed={defsFailed}
          />
          {blocked === null ? null : (
            <p className="practice-setup__hint" data-testid={practiceTestid.customBlocked} role="status">
              {blocked}
            </p>
          )}
        </div>

        <label className="practice-setup__sandbox">
          <input
            type="checkbox"
            data-testid={practiceTestid.sandbox}
            checked={sandbox}
            aria-describedby={`${baseId}-sandbox-note`}
            onChange={(event) => setSandbox(event.currentTarget.checked)}
          />
          <span>
            <span className="practice-setup__sandbox-label">{t("Sandbox")}</span>
            <span className="practice-setup__sandbox-note" id={`${baseId}-sandbox-note`}>
              {t("Dev tools to spawn any card and set up the board. The game counts for nothing.")}
            </span>
          </span>
        </label>

        <button type="submit" className="practice-play button-primary" data-testid={practiceTestid.start} disabled={blocked !== null}>
          {t("Start game")}
        </button>
      </div>
    </form>
  );
}
