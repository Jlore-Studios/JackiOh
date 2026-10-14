// Challenging a friend (#936, B48): the body `POST /api/rooms` takes — a mode and, for it, a saved
// deck or trio — plus the friend, sent as `POST /api/challenges`. The server makes the room exactly
// as `/play` would and refuses a busy, offline or already challenged friend; its refusal is shown in
// its own words. Once sent, the challenge waits on Play together, and the shell takes the challenger
// to the game when the friend joins (`ChallengeToast.tsx`).
//
// CLAUDE.md rule 7: the picks are the lobby's (`routes/play.tsx`), with no verdict of their own here;
// whether a deck may be played is the server's, at the room.

import { useEffect, useId, useState, type FormEvent, type ReactElement } from "react";

import { DECK_SIZE } from "@jackioh/engine/config";

import { t, useLocale } from "../i18n/i18n.ts";
import { getDecks, type ModeChoice, type QueueMode, type SavedDeck, type SavedTrio } from "../net/api.ts";
import { paths } from "../net/navigate.ts";
import { createChallenge, type PlayerRef } from "../net/social.ts";
import { followInApp } from "../routes/nav.tsx";
import { followChallenge } from "./follow.ts";
import { attempt, messageOf, withToken } from "./session.ts";
import { challengeModeTestid, togetherTestid } from "./testids.ts";
import { CHALLENGE_MODES, MODE_WORDS, Named } from "./words.tsx";

type Saved = { decks: readonly SavedDeck[]; trios: readonly SavedTrio[] };

/** The first complete deck, else the first; as the lobby picks one with nothing remembered. */
function firstDeck(decks: readonly SavedDeck[]): SavedDeck | null {
  return decks.find((deck) => deck.cards.length === DECK_SIZE) ?? decks[0] ?? null;
}

/** The first trio with three decks, else the first. */
function firstTrio(trios: readonly SavedTrio[]): SavedTrio | null {
  return trios.find((trio) => trio.deckIds.every((id) => id !== null)) ?? trios[0] ?? null;
}

/** What a room is made with for `mode`, or null while its pick is missing (`choiceFor` in `routes/play.tsx`). */
function choiceOf(mode: QueueMode, deckId: string | null, trioId: string | null): ModeChoice | null {
  switch (mode) {
    case "random":
      return { mode: "random" };
    case "bo1":
      return deckId === null ? null : { mode: "bo1", deckId };
    case "bo3":
      return trioId === null ? null : { mode: "bo3", trioId };
  }
}

export function ChallengeDialog({
  friend,
  onClose,
  onSent,
}: {
  friend: PlayerRef;
  onClose: () => void;
  onSent: () => void;
}): ReactElement {
  useLocale();
  const titleId = useId();
  const [mode, setMode] = useState<QueueMode>("bo1");
  const [saved, setSaved] = useState<Saved | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [deckId, setDeckId] = useState<string | null>(null);
  const [trioId, setTrioId] = useState<string | null>(null);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    withToken(getDecks).then(
      (answer) => {
        if (cancelled) return;
        setSaved({ decks: answer.decks, trios: answer.trios });
        setDeckId(firstDeck(answer.decks)?.id ?? null);
        setTrioId(firstTrio(answer.trios)?.id ?? null);
      },
      (cause: unknown) => {
        if (!cancelled) setLoadError(messageOf(cause));
      },
    );
    return () => {
      cancelled = true;
    };
  }, []);

  const choice = choiceOf(mode, deckId, trioId);

  const send = (event: FormEvent<HTMLFormElement>): void => {
    event.preventDefault();
    if (choice === null || sending) return;
    setSending(true);
    setError(null);
    attempt(() => createChallenge({ ...choice, friendId: friend.profileId }))
      .then(
        () => {
          // The shell takes the challenger to the game once the friend joins (`follow.ts`).
          followChallenge();
          onSent();
        },
        (cause: unknown) => {
          setError(messageOf(cause));
        },
      )
      .finally(() => {
        setSending(false);
      });
  };

  return (
    <div
      className="together-dialog-scrim"
      onClick={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <form
        className="lobby-card together-dialog"
        data-testid={togetherTestid.challengeForm}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        onSubmit={send}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.stopPropagation();
            onClose();
          }
        }}
      >
        <h2 className="play-panel__heading" id={titleId}>
          <Named text={t("Challenge {name}")} name={friend.username} />
        </h2>

        <fieldset className="lobby-modes">
          <legend className="lobby-modes__legend">{t("Mode")}</legend>
          {CHALLENGE_MODES.map((option) => (
            <label key={option} className="lobby-mode" data-selected={option === mode ? "true" : "false"}>
              <input
                type="radio"
                name="together-challenge-mode"
                value={option}
                checked={option === mode}
                data-testid={challengeModeTestid(option)}
                onChange={() => {
                  setMode(option);
                }}
              />
              <span className="lobby-mode__text">
                <span className="lobby-mode__name">{t(MODE_WORDS[option])}</span>
              </span>
            </label>
          ))}
        </fieldset>

        {mode === "random" ? (
          <p className="lobby-note">{t("No deck needed: the server deals both of you one when the game starts.")}</p>
        ) : null}
        {mode !== "random" && saved === null && loadError === null ? (
          <p className="lobby-note" role="status">
            {t("Loading your decks…")}
          </p>
        ) : null}
        {mode !== "random" && loadError !== null ? (
          <p className="notice" role="alert">
            {t("Your decks could not be loaded: {message}", { message: t(loadError) })}
          </p>
        ) : null}
        {mode === "bo1" && saved !== null ? (
          saved.decks.length === 0 ? (
            <p className="lobby-note">
              <a href={paths.decks} onClick={followInApp(paths.decks)}>
                {t("Build a deck first")}
              </a>
            </p>
          ) : (
            <label className="together-pick">
              {t("Your deck")}
              <select
                className="lobby-select"
                data-testid={togetherTestid.challengeDeck}
                value={deckId ?? ""}
                onChange={(event) => {
                  setDeckId(event.target.value);
                }}
              >
                {saved.decks.map((deck) => (
                  <option key={deck.id} value={deck.id}>
                    {deck.name}
                  </option>
                ))}
              </select>
            </label>
          )
        ) : null}
        {mode === "bo3" && saved !== null ? (
          saved.trios.length === 0 ? (
            <p className="lobby-note">
              <a href={paths.decks} onClick={followInApp(paths.decks)}>
                {t("Build a trio first")}
              </a>
            </p>
          ) : (
            <label className="together-pick">
              {t("Your trio")}
              <select
                className="lobby-select"
                data-testid={togetherTestid.challengeTrio}
                value={trioId ?? ""}
                onChange={(event) => {
                  setTrioId(event.target.value);
                }}
              >
                {saved.trios.map((trio) => (
                  <option key={trio.id} value={trio.id}>
                    {trio.name}
                  </option>
                ))}
              </select>
            </label>
          )
        ) : null}

        {error === null ? null : (
          <p className="notice" data-testid={togetherTestid.challengeError} role="alert">
            {t(error)}
          </p>
        )}

        <div className="together-actions">
          <button type="submit" className="button-primary" data-testid={togetherTestid.challengeSend} disabled={choice === null || sending} aria-busy={sending}>
            {t("Send challenge")}
          </button>
          <button type="button" className="link-button" data-testid={togetherTestid.challengeClose} onClick={onClose}>
            {t("Cancel")}
          </button>
        </div>
      </form>
    </div>
  );
}
