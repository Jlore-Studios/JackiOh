// Practice setup maps difficulty and deck selection to Start (SPEC §9.9).
// AI resource tiers derive from `AI_DIFFICULTY` (CLAUDE.md rule 9).
// R1373: only the player's Random deck can lean toward the newest set; retain its switch selection.

import { useId, useState, type FormEvent, type ReactElement } from "react";

import {
  AI_DIFFICULTY,
  DIFFICULTIES,
  DRAWS_PER_TURN,
  HUMAN_HANDICAP,
  type Difficulty,
  type Handicap,
} from "@jackioh/engine/config";

import type { CardDefs } from "@jackioh/shared";

import { LeanNewestToggle } from "../game/LeanNewest.tsx";
import { paths } from "../net/navigate.ts";
import { DeckPreview } from "./DeckPreview.tsx";
import {
  RANDOM_DECK_IDENTITY,
  deckChoiceFromValue,
  deckOptions,
  presetById,
  type PracticeSavedDeck,
} from "./decks.ts";
import type { PracticeDeckChoice } from "./protocol.ts";
import { practiceTestid } from "./testids.ts";
import { DIFFICULTY_LABEL, DIFFICULTY_TAGLINE, TierCrest } from "./Tier.tsx";
import "./practice.css";

/** Selection passed to Start, including R1373's random-deck preference. */
export type PracticeSetupChoice = { difficulty: Difficulty; deck: PracticeDeckChoice; leanNewest?: boolean };

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
  initial: { difficulty: Difficulty; deck: string; leanNewest?: boolean };
  /** The worker's catalog for the deck preview; null while it loads or when it could not be read. */
  defs: CardDefs | null;
  defsFailed: boolean;
  onStart(choice: PracticeSetupChoice): void;
};

function savedHint(saved: SavedDecks): ReactElement | null {
  switch (saved.kind) {
    case "ready":
      return null;
    case "anonymous":
      return <>Sign in and save a deck to play one of your own decks here.</>;
    case "checking":
      return <>Looking for your saved decks…</>;
    case "inactive":
      return <>Your saved decks show up here once your account is active.</>;
    case "none":
      return (
        <>
          Save a deck in <a href={paths.decks}>Decks</a> to play one of your own decks here.
        </>
      );
    case "unavailable":
      return <>Your saved decks could not be loaded just now. The decks above still play.</>;
  }
}

function previewFor(
  choice: PracticeDeckChoice,
  saved: readonly PracticeSavedDeck[] | null,
): { title: string; identity: string; cards: readonly string[] | null } {
  switch (choice.kind) {
    case "random":
      return { title: "Random deck", identity: RANDOM_DECK_IDENTITY, cards: null };
    case "preset": {
      const preset = presetById(choice.id);
      return preset === undefined
        ? { title: choice.id, identity: "", cards: null }
        : { title: preset.name, identity: preset.identity, cards: preset.cards };
    }
    case "saved":
      return {
        title: saved?.[choice.index - 1]?.name ?? `Saved deck ${String(choice.index)}`,
        identity: "One of your saved decks, exactly as you saved it.",
        cards: choice.cards,
      };
  }
}

function plural(count: number, one: string, many: string): string {
  return `${String(count)} ${count === 1 ? one : many}`;
}

type TierStat = { key: string; label: string; value: string; ahead: boolean };

function tierStats(handicap: Handicap): TierStat[] {
  const human = HUMAN_HANDICAP;
  return [
    {
      key: "deck",
      label: "Deck",
      value: plural(handicap.deckSize, "card", "cards"),
      ahead: handicap.deckSize !== human.deckSize,
    },
    {
      key: "mana",
      label: "Mana",
      value:
        handicap.manaBonus > 0
          ? `+${String(handicap.manaBonus)}, up to ${String(handicap.manaCap)}`
          : `up to ${String(handicap.manaCap)}`,
      ahead: handicap.manaBonus !== human.manaBonus || handicap.manaCap !== human.manaCap,
    },
    {
      key: "hand",
      label: "Opening hand",
      value: handicap.extraOpeningCards > 0 ? `+${plural(handicap.extraOpeningCards, "card", "cards")}` : "as yours",
      ahead: handicap.extraOpeningCards !== human.extraOpeningCards,
    },
    {
      key: "draws",
      label: "Draws",
      value: `${String(DRAWS_PER_TURN + handicap.extraDrawsPerTurn)} per turn`,
      ahead: handicap.extraDrawsPerTurn !== human.extraDrawsPerTurn,
    },
  ];
}

export function PracticeSetup({ saved, initial, defs, defsFailed, onStart }: PracticeSetupProps): ReactElement {
  const [difficulty, setDifficulty] = useState<Difficulty>(initial.difficulty);
  const [deck, setDeck] = useState<string>(initial.deck);
  const [leanNewest, setLeanNewest] = useState<boolean>(initial.leanNewest ?? false);
  const baseId = useId();

  const savedDecks = saved.kind === "ready" ? saved.decks : null;
  const options = deckOptions(savedDecks);
  // A remembered saved deck that is gone, not complete or not loaded yet falls back to random.
  const selected = options.some((option) => option.value === deck && !option.disabled) ? deck : "random";
  const picked: PracticeDeckChoice = deckChoiceFromValue(selected, savedDecks) ?? { kind: "random" };
  // R1373: only the Random deck leans, and only when the switch is on.
  const choice: PracticeDeckChoice = picked.kind === "random" && leanNewest ? { kind: "random", leanNewest: true } : picked;
  const preview = previewFor(choice, savedDecks);
  const hint = savedHint(saved);

  function submit(event: FormEvent<HTMLFormElement>): void {
    event.preventDefault();
    onStart({ difficulty, deck: choice, leanNewest });
  }

  return (
    <form className="practice-setup" data-testid={practiceTestid.setup} onSubmit={submit}>
      <fieldset className="practice-setup__difficulties">
        <legend className="practice-setup__legend">
          <h2 className="practice-setup__heading">Choose your opponent</h2>
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
                    {DIFFICULTY_LABEL[d]}
                  </span>
                  <span className="practice-tier__tagline">{DIFFICULTY_TAGLINE[d]}</span>
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
              Your deck
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
                  {option.label}
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
            <LeanNewestToggle checked={leanNewest} testid={practiceTestid.leanNewest} onChange={setLeanNewest} />
          ) : null}
          <DeckPreview
            title={preview.title}
            identity={preview.identity}
            cards={preview.cards}
            defs={defs}
            defsFailed={defsFailed}
          />
        </div>

        <button type="submit" className="practice-play button-primary" data-testid={practiceTestid.start}>
          Start game
        </button>
      </div>
    </form>
  );
}
