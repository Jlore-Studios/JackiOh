// Challenging a friend (#936, B48): the body `POST /api/rooms` takes — a mode and, for it, a saved
// deck or trio — plus the friend, sent as `POST /api/challenges`. The server makes the room exactly
// as `/play` would and refuses a busy, offline or already challenged friend; its refusal is shown in
// its own words. Once sent, the challenge waits on Play together, and the shell takes the challenger
// to the game when the friend joins (`ChallengeToast.tsx`).
//
// CLAUDE.md rule 7: the picks are the lobby's (`routes/play.tsx`), with no verdict of their own here;
// whether a deck may be played is the server's, at the room.
//
// A modal like the client's others (`game/ConfirmConcede.tsx`, #1569): the focus moves to the checked
// mode when it opens, Tab and Shift+Tab stay inside, Escape closes it wherever the focus is, and the
// caller gives the focus back to the Challenge button that opened it (`TogetherRoute.tsx`).

import { useEffect, useId, useRef, useState, type FormEvent, type ReactElement } from "react";

import { DECK_SIZE } from "@jackioh/engine/config";

import { t, useLocale } from "../i18n/i18n.ts";
import { getDecks, type ModeChoice, type QueueMode, type SavedDeck, type SavedTrio } from "../net/api.ts";
import { withSession } from "../net/gate.ts";
import { paths } from "../net/navigate.ts";
import { createChallenge, type PlayerRef } from "../net/social.ts";
import { followInApp } from "../routes/nav.tsx";
import { followChallenge } from "./follow.ts";
import { attempt, messageOf } from "./session.ts";
import { challengeModeTestid, togetherTestid } from "./testids.ts";
import { CHALLENGE_MODES, MODE_WORDS, Named } from "./words.tsx";

type Saved = { decks: readonly SavedDeck[]; trios: readonly SavedTrio[] };

/** What Tab may land on in the dialog. */
const TAB_STOPS = "a[href], button:not([disabled]), input:not([disabled]), select:not([disabled])";

/** The dialog's tab stops in order; the mode group is one stop, its checked radio, as the browser's own Tab takes it. */
function tabStops(dialog: HTMLElement): HTMLElement[] {
  return [...dialog.querySelectorAll<HTMLElement>(TAB_STOPS)].filter(
    (element) => !(element instanceof HTMLInputElement && element.type === "radio" && !element.checked),
  );
}

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
  const [deckReads, setDeckReads] = useState(0);
  const [deckId, setDeckId] = useState<string | null>(null);
  const [trioId, setTrioId] = useState<string | null>(null);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const dialog = useRef<HTMLFormElement>(null);

  // Keyboard and screen-reader users start inside the dialog, on the mode it opens with.
  useEffect(() => {
    const form = dialog.current;
    if (form !== null) tabStops(form)[0]?.focus({ preventScroll: true });
  }, []);

  // Both keys are read at the window, in the capture phase, so they hold wherever the focus is (a
  // click on the dialog's text or the dimmed page takes it off the controls): Escape closes, and Tab
  // and Shift+Tab step through the dialog's stops, wrapping, never onto the page behind the scrim.
  useEffect(() => {
    function onKey(event: globalThis.KeyboardEvent): void {
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        onClose();
        return;
      }
      if (event.key !== "Tab") return;
      const form = dialog.current;
      if (form === null) return;
      const order = tabStops(form);
      if (order.length === 0) return;
      const focused = document.activeElement;
      const at = focused instanceof HTMLElement ? order.indexOf(focused) : -1;
      const next = event.shiftKey ? (at <= 0 ? order.length - 1 : at - 1) : at === order.length - 1 ? 0 : at + 1;
      event.preventDefault();
      event.stopPropagation();
      order[next]?.focus();
    }
    window.addEventListener("keydown", onKey, true);
    return () => {
      window.removeEventListener("keydown", onKey, true);
    };
  }, [onClose]);

  useEffect(() => {
    let cancelled = false;
    withSession(getDecks).then(
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
  }, [deckReads]);

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
          <p className="lobby-note">{t("No deck needed: the server deals both of you one when the game starts.")}</p>
        ) : null}
        {mode !== "random" && saved === null && loadError === null ? (
          <p className="lobby-note" role="status">
            {t("Loading your decks…")}
          </p>
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
