// Challenging a friend (#936, B48): the body `POST /api/rooms` takes plus the friend, sent as `POST
// /api/challenges`. The server makes the room exactly as `/play` would and refuses a busy, offline or
// already challenged friend, in its own words. The shell takes the challenger to the game when the
// friend joins (`ChallengeToast.tsx`).
//
// CLAUDE.md rule 7: the picks are the lobby's (`routes/play.tsx`); whether a deck may be played is the
// server's, at the room. R915005: as there, a deck naming a custom card (or a trio holding one) is
// listed off, "(practice only)", and never picked.
//
// It opens on the last mode and deck (or trio) a challenge was sent with from this browser
// (`challengeChoice.ts`, #1831), so a repeat challenge is Challenge and Enter. R2031000: All Random
// offers "More cards from" (R924000) on the device's online pick, once the engine is in (#778).
//
// A modal like the client's others (`game/ConfirmConcede.tsx`, #1569): the focus stays inside,
// Escape closes it, and the caller gives the focus back to the button that opened it.

import { useEffect, useId, useRef, useState, type FormEvent, type ReactElement } from "react";

import { LeanSetPickerLazy } from "../game/LeanSetPickerLazy.tsx";
import { readPlayLean, writePlayLean } from "../game/playLean.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { getDecks, type QueueMode, type SavedDeck, type SavedTrio } from "../net/api.ts";
import { withSession } from "../net/gate.ts";
import { paths } from "../net/navigate.ts";
import { createChallenge, type PlayerRef } from "../net/social.ts";
import { followInApp } from "../routes/nav.tsx";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import {
  choiceOf,
  deckPracticeOnly,
  pickDeck,
  pickTrio,
  practiceOnlyLabel,
  readChallengeChoice,
  rememberChallengeChoice,
  trioPracticeOnly,
} from "./challengeChoice.ts";
import { followChallenge } from "./follow.ts";
import { tabStops, useModalKeys } from "./modalKeys.ts";
import { attempt, messageOf } from "./session.ts";
import { challengeModeTestid, togetherTestid } from "./testids.ts";
import { CHALLENGE_MODES, MODE_WORDS, Named } from "./words.tsx";

type Saved = { decks: readonly SavedDeck[]; trios: readonly SavedTrio[] };

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
  const [initial] = useState(readChallengeChoice);
  const [mode, setMode] = useState<QueueMode>(initial.mode);
  const [saved, setSaved] = useState<Saved | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [deckReads, setDeckReads] = useState(0);
  const [deckId, setDeckId] = useState<string | null>(null);
  const [trioId, setTrioId] = useState<string | null>(null);
  const [lean, setLean] = useState(readPlayLean);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const dialog = useRef<HTMLFormElement>(null);
  const sendButton = useRef<HTMLButtonElement>(null);

  // Keyboard and screen-reader users start inside the dialog, on the mode it opens with.
  useEffect(() => {
    const form = dialog.current;
    if (form !== null) tabStops(form)[0]?.focus({ preventScroll: true });
  }, []);

  useModalKeys(dialog, onClose);

  useEffect(() => {
    let canceled = false;
    withSession(getDecks).then(
      (answer) => {
        if (canceled) return;
        setSaved({ decks: answer.decks, trios: answer.trios });
        setDeckId(pickDeck(answer.decks, initial.deckId)?.id ?? null);
        setTrioId(pickTrio(answer.trios, initial.trioId, answer.decks)?.id ?? null);
      },
      (cause: unknown) => {
        if (!canceled) setLoadError(messageOf(cause));
      },
    );
    return () => {
      canceled = true;
    };
  }, [deckReads, initial]);

  const choice = choiceOf(mode, deckId, trioId, lean);

  // The first time the picks are complete the player can send, so the focus goes to Send (#1831); a
  // later change of mode leaves it where the player put it, and so does a focus already off the dialog
  // (it was closed, or the page behind clicked, while the decks were read).
  const focusedSend = useRef(false);
  useEffect(() => {
    if (choice === null || focusedSend.current) return;
    focusedSend.current = true;
    if (dialog.current?.contains(document.activeElement) === true) sendButton.current?.focus({ preventScroll: true });
  }, [choice]);

  const send = (event: FormEvent<HTMLFormElement>): void => {
    event.preventDefault();
    if (choice === null || sending) return;
    setSending(true);
    setError(null);
    attempt(() => createChallenge({ ...choice, friendId: friend.profileId }))
      .then(
        () => {
          rememberChallengeChoice(choice, true);
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
        ref={dialog}
        className="lobby-card together-dialog"
        data-testid={togetherTestid.challengeForm}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        onSubmit={send}
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
          <>
            <p className="lobby-note">{t("No deck needed: the server deals both of you one when the game starts.")}</p>
            <LeanSetPickerLazy
              value={lean}
              disabled={sending}
              testid={togetherTestid.challengeLean}
              onChange={(pick) => {
                setLean(pick);
                writePlayLean(pick);
              }}
            />
          </>
        ) : null}
        {mode !== "random" && saved === null && loadError === null ? (
          <Skeleton variant="panel" label={t("Loading your decks…")} />
        ) : null}
        {mode !== "random" && loadError !== null ? (
          <>
            <p className="notice" role="alert">
              {t("Your decks could not be loaded: {message}", { message: t(loadError) })}
            </p>
            <div className="row">
              <button
                type="button"
                data-testid={togetherTestid.challengeDecksRetry}
                onClick={() => {
                  setLoadError(null);
                  setDeckReads((before) => before + 1);
                }}
              >
                {t("Try again")}
              </button>
            </div>
          </>
        ) : null}
        {mode === "bo1" && saved !== null ? (
          saved.decks.length === 0 ? (
            <p className="lobby-note">
              <a href={paths.decks} onClick={followInApp(paths.decks)}>
                {t("Build a deck first")}
              </a>
            </p>
          ) : (
            <>
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
                  {deckId === null ? <option value="">{t("No deck plays online yet")}</option> : null}
                  {saved.decks.map((deck) =>
                    deckPracticeOnly(deck) ? (
                      <option key={deck.id} value={deck.id} disabled>
                        {practiceOnlyLabel(deck.name)}
                      </option>
                    ) : (
                      <option key={deck.id} value={deck.id}>
                        {deck.name}
                      </option>
                    ),
                  )}
                </select>
              </label>
              {saved.decks.some(deckPracticeOnly) ? (
                <p className="lobby-note" data-testid={togetherTestid.challengePracticeOnly}>
                  {t("A deck with a custom card plays only in practice, against the AI.")}
                </p>
              ) : null}
            </>
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
            <>
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
                  {trioId === null ? <option value="">{t("No trio plays online yet")}</option> : null}
                  {saved.trios.map((trio) =>
                    trioPracticeOnly(trio, saved.decks) ? (
                      <option key={trio.id} value={trio.id} disabled>
                        {practiceOnlyLabel(trio.name)}
                      </option>
                    ) : (
                      <option key={trio.id} value={trio.id}>
                        {trio.name}
                      </option>
                    ),
                  )}
                </select>
              </label>
              {saved.trios.some((trio) => trioPracticeOnly(trio, saved.decks)) ? (
                <p className="lobby-note" data-testid={togetherTestid.challengePracticeOnly}>
                  {t("A trio holding a deck with a custom card cannot play Conquest: custom cards play only in practice.")}
                </p>
              ) : null}
            </>
          )
        ) : null}

        {error === null ? null : (
          <p className="notice" data-testid={togetherTestid.challengeError} role="alert">
            {t(error)}
          </p>
        )}

        <div className="together-actions">
          <button ref={sendButton} type="submit" className="button-primary" data-testid={togetherTestid.challengeSend} disabled={choice === null || sending} aria-busy={sending}>
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
