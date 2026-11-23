// Challenging a friend (#936, B48): the body `POST /api/rooms` takes plus the friend, sent as `POST
// /api/challenges`. The server makes the room exactly as `/play` would and refuses a busy, offline or
// already challenged friend, in its own words. The shell takes the challenger to the game when the
// friend joins (`ChallengeToast.tsx`).
//
// CLAUDE.md rule 7: the picks are the lobby's (`routes/play.tsx`); whether a deck may be played is the
// server's, at the room.
//
// It opens on the last mode and deck (or trio) a challenge was sent with from this browser
// (`challengeChoice.ts`, #1831), so a repeat challenge is Challenge and Send. R2031000: All random
// offers "More cards from" (R924000) on the device's online pick, once the engine is in (#778).
//
// R3221000: an alertdialog on the one Dialog, as the board's confirms are (R3220000): Cancel first,
// holding the focus, and Send last; the focus stays inside, Escape and Back close it, and the caller
// gives the focus back to the button that opened it.
//
// #3727: Live or Daily. Live is the room above, for a friend online and free while the player is free
// too (`liveRefusal` says why not, and the dialog opens on Daily then); Daily challenges any friend to a
// daily game (`daily/DailyFields.tsx`, `POST /api/daily/challenges`), which waits on the Daily screen.

import { useEffect, useState, type ReactElement } from "react";

import { Dialog } from "../game/Dialog.tsx";
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
  pickDeck,
  pickTrio,
  readChallengeChoice,
  rememberChallengeChoice,
} from "./challengeChoice.ts";
import { followChallenge } from "./follow.ts";
import { challengeKindTestid, challengeModeTestid, togetherTestid } from "./testids.ts";
import { CHALLENGE_MODES, MODE_WORDS, Named } from "./words.tsx";
import { attempt, messageOf } from "../net/errors.ts";
import { DailyFields, dailyChoiceOf, useDailyPick } from "../daily/DailyFields.tsx";
import { createDailyChallenge } from "../net/daily.ts";
import { Segmented } from "../tabs/index.ts";

type Saved = { decks: readonly SavedDeck[]; trios: readonly SavedTrio[] };

export type ChallengeKind = "live" | "daily";

export function ChallengeDialog({
  friend,
  onClose,
  onSent,
  liveRefusal = null,
  initialKind,
}: {
  friend: PlayerRef;
  onClose: () => void;
  onSent: () => void;
  /** #3727: why a live game can't be played now, in a player's words (untranslated); null, it can. */
  liveRefusal?: string | null;
  /** R3732002: the kind a row's Daily opens on; Live when it can be played and none is asked for. */
  initialKind?: ChallengeKind;
}): ReactElement {
  useLocale();
  const [kind, setKind] = useState<ChallengeKind>(liveRefusal === null ? (initialKind ?? "live") : "daily");
  const daily = useDailyPick(undefined, kind === "daily");
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

  useEffect(() => {
    let canceled = false;
    withSession(getDecks).then(
      (answer) => {
        if (canceled) return;
        setSaved({ decks: answer.decks, trios: answer.trios });
        setDeckId(pickDeck(answer.decks, initial.deckId)?.id ?? null);
        setTrioId(pickTrio(answer.trios, initial.trioId)?.id ?? null);
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
  const dailyChoice = dailyChoiceOf(daily);
  const ready = kind === "live" ? choice !== null && liveRefusal === null : dailyChoice !== null;

  const send = (): void => {
    if (!ready || sending) return;
    setSending(true);
    setError(null);
    const sent =
      kind === "daily" && dailyChoice !== null
        ? // A daily challenge waits on the Daily screen: no room to follow.
          attempt(() => createDailyChallenge(friend.profileId, dailyChoice)).then(() => undefined)
        : choice === null
          ? Promise.resolve()
          : attempt(() => createChallenge({ ...choice, friendId: friend.profileId })).then(() => {
              rememberChallengeChoice(choice, true);
              // The shell takes the challenger to the game once the friend joins (`follow.ts`).
              followChallenge();
            });
    sent
      .then(
        () => {
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
    <Dialog
      role="alertdialog"
      title={<Named text={t("Challenge {name}")} name={friend.username} />}
      onClose={onClose}
      cancel={{ label: t("Cancel"), kind: "secondary", testid: togetherTestid.challengeClose }}
      actions={[
        {
          label: t("Send challenge"),
          kind: "primary",
          onClick: send,
          testid: togetherTestid.challengeSend,
          disabled: !ready || sending,
          busy: sending,
        },
      ]}
      testid={togetherTestid.challengeForm}
      className="together-dialog"
      data={{ "data-kind": kind }}
    >
      <Segmented
        label={t("How to play")}
        testId={togetherTestid.challengeKind}
        value={kind}
        disabled={sending}
        onChange={setKind}
        options={[
          {
            value: "live",
            label: t("Live"),
            testId: challengeKindTestid("live"),
            disabled: liveRefusal !== null,
            ...(liveRefusal === null ? {} : { title: t(liveRefusal) }),
          },
          { value: "daily", label: t("Daily"), testId: challengeKindTestid("daily") },
        ]}
      />
      {liveRefusal === null ? null : (
        <p className="lobby-note" data-testid={togetherTestid.challengeLiveRefusal}>
          {t(liveRefusal)}
        </p>
      )}
      {kind === "daily" ? (
        <>
          <p className="lobby-note">{t("A move every few days, on your own time. We'll tell you when it's your move.")}</p>
          <DailyFields pick={daily} disabled={sending} />
        </>
      ) : null}
      {kind === "daily" ? null : (
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
      )}

      {kind === "live" && mode === "random" ? (
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
      {kind === "live" && mode !== "random" && saved === null && loadError === null ? (
        <Skeleton variant="panel" label={t("Loading your decks…")} />
      ) : null}
      {kind === "live" && mode !== "random" && loadError !== null ? (
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
      {kind === "live" && mode === "bo1" && saved !== null ? (
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
                {saved.decks.map((deck) => (
                  <option key={deck.id} value={deck.id}>
                    {deck.name}
                  </option>
                ))}
              </select>
            </label>
          </>
        )
      ) : null}
      {kind === "live" && mode === "bo3" && saved !== null ? (
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
                {saved.trios.map((trio) => (
                  <option key={trio.id} value={trio.id}>
                    {trio.name}
                  </option>
                ))}
              </select>
            </label>
          </>
        )
      ) : null}

      {error === null ? null : (
        <p className="notice" data-testid={togetherTestid.challengeError} role="alert">
          {t(error)}
        </p>
      )}
    </Dialog>
  );
}
