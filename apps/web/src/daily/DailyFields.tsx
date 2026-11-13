// What a daily game is made or accepted with (#3727): the mode (Best of 1 with a saved deck, or All random
// with the live queue's "More cards from" lean, R924000), the deck, and the time per move. The New daily
// game dialog, a friend's Challenge (`together/ChallengeDialog.tsx`) and a challenge's Accept share it.
//
// CLAUDE.md rule 7: whether a deck may be played is the server's, when the game is made. Rule 9: the times
// per move are `crates/shared`'s (`DAILY_TIME_PER_MOVE_DAYS`), read from the WebAssembly module, so a
// screen that may paint before the module is in (the shell's challenge dialog) waits for it here.

import { useEffect, useState, type ReactElement } from "react";

import type { LeanPick } from "@jackioh/shared";

import { LeanSetPickerLazy } from "../game/LeanSetPickerLazy.tsx";
import { readPlayLean, writePlayLean } from "../game/playLean.ts";
import { t } from "../i18n/i18n.ts";
import { getDecks, type SavedDeck } from "../net/api.ts";
import { DAILY_MODES, type DailyChoice, type DailyMode } from "../net/daily.ts";
import { attempt, messageOf } from "../net/errors.ts";
import { withSession } from "../net/gate.ts";
import { paths } from "../net/navigate.ts";
import { followInApp } from "../routes/nav.tsx";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { Segmented } from "../tabs/index.ts";
import { pickDeck, readChallengeChoice } from "../together/challengeChoice.ts";
import { MODE_WORDS } from "../together/words.tsx";
import { dailyConstants, loadWasm, wasmLoaded } from "../wasm/index.ts";
import { perMoveText } from "./order.ts";
import { dailyModeTestid, dailyPerMoveTestid, dailyTestid } from "./testids.ts";

/**
 * The engine's module is in: the times per move can be read. A failed load shows no times, and no send.
 * `active` false asks for nothing yet (the shell's challenge dialog, until Daily is picked).
 */
export function useDailyReady(active = true): boolean {
  const [ready, setReady] = useState(wasmLoaded);
  useEffect(() => {
    if (ready || !active) return undefined;
    let canceled = false;
    loadWasm().then(
      () => {
        if (!canceled) setReady(true);
      },
      () => undefined,
    );
    return () => {
      canceled = true;
    };
  }, [ready, active]);
  return ready;
}

/** The times per move a game may be played at, shortest first; none until the module is in. */
export function perMoveOptions(ready: boolean): readonly number[] {
  return ready ? dailyConstants().DAILY_TIME_PER_MOVE_DAYS : [];
}

export type DailyPick = {
  mode: DailyMode;
  setMode: (mode: DailyMode) => void;
  /** Null until the module is in. */
  perMoveDays: number | null;
  setPerMoveDays: (days: number) => void;
  deckId: string | null;
  setDeckId: (id: string) => void;
  lean: LeanPick | null;
  setLean: (pick: LeanPick | null) => void;
  /** The saved decks, null while they are read. */
  decks: readonly SavedDeck[] | null;
  decksError: string | null;
  retryDecks: () => void;
};

/** The device's last challenge mode, when a daily game can be played in it; else Best of 1. */
function initialMode(fixed: DailyMode | undefined): DailyMode {
  if (fixed !== undefined) return fixed;
  const { mode } = readChallengeChoice();
  return mode === "random" ? "random" : "bo1";
}

/**
 * `fixed`: the mode is not the player's to choose (an accepted challenge plays the challenge's). `active`
 * false reads no decks and loads no module yet (a challenge dialog open on Live). `initialDays`: the time
 * per move picked to begin with (a rematch's, its game's).
 */
export function useDailyPick(fixed?: DailyMode, active = true, initialDays: number | null = null): DailyPick {
  const ready = useDailyReady(active);
  const [initial] = useState(readChallengeChoice);
  const [mode, setMode] = useState<DailyMode>(() => initialMode(fixed));
  const [chosenDays, setPerMoveDays] = useState<number | null>(initialDays);
  const [deckId, setDeckId] = useState<string | null>(null);
  const [lean, setLeanState] = useState(readPlayLean);
  const [decks, setDecks] = useState<readonly SavedDeck[] | null>(null);
  const [decksError, setDecksError] = useState<string | null>(null);
  const [reads, setReads] = useState(0);

  useEffect(() => {
    if (!active) return undefined;
    let canceled = false;
    attempt(() => withSession(getDecks)).then(
      (answer) => {
        if (canceled) return;
        setDecks(answer.decks);
        setDeckId((before) => before ?? pickDeck(answer.decks, initial.deckId)?.id ?? null);
      },
      (cause: unknown) => {
        if (!canceled) setDecksError(messageOf(cause));
      },
    );
    return () => {
      canceled = true;
    };
  }, [reads, initial, active]);

  const options = perMoveOptions(ready);
  const perMoveDays = chosenDays !== null && options.includes(chosenDays) ? chosenDays : (options[0] ?? null);
  return {
    mode,
    setMode,
    perMoveDays,
    setPerMoveDays,
    deckId,
    setDeckId,
    lean,
    setLean: (pick) => {
      setLeanState(pick);
      writePlayLean(pick);
    },
    decks,
    decksError,
    retryDecks: () => {
      setDecksError(null);
      setReads((before) => before + 1);
    },
  };
}

/** What the server is sent, or null while a pick is missing (a deck for Best of 1, the time). */
export function dailyChoiceOf(pick: Pick<DailyPick, "mode" | "deckId" | "lean" | "perMoveDays">): DailyChoice | null {
  if (pick.perMoveDays === null) return null;
  if (pick.mode === "bo1") return pick.deckId === null ? null : { mode: "bo1", deckId: pick.deckId, perMoveDays: pick.perMoveDays };
  return pick.lean === null
    ? { mode: "random", perMoveDays: pick.perMoveDays }
    : { mode: "random", leanSet: pick.lean, perMoveDays: pick.perMoveDays };
}

/** An accept's body: the challenge's mode, with this player's own deck or lean. */
export function acceptBodyOf(pick: DailyPick): { mode: DailyMode; deckId?: string; leanSet?: LeanPick } | null {
  if (pick.mode === "bo1") return pick.deckId === null ? null : { mode: "bo1", deckId: pick.deckId };
  return pick.lean === null ? { mode: "random" } : { mode: "random", leanSet: pick.lean };
}

/**
 * The fields: the mode (unless `fixedMode`), its deck or lean, and the time per move (unless `withTime`
 * is false: an accepted challenge plays at the challenge's time).
 */
export function DailyFields({
  pick,
  fixedMode = false,
  withTime = true,
  disabled = false,
}: {
  pick: DailyPick;
  fixedMode?: boolean;
  withTime?: boolean;
  disabled?: boolean;
}): ReactElement {
  const ready = useDailyReady();
  const options = perMoveOptions(ready);
  return (
    <div className="daily-fields">
      {fixedMode ? null : (
        <div className="daily-field">
          <span className="daily-field__label">{t("Mode")}</span>
          <Segmented
            label={t("Mode")}
            testId={dailyTestid.mode}
            value={pick.mode}
            disabled={disabled}
            onChange={pick.setMode}
            options={DAILY_MODES.map((mode) => ({ value: mode, label: t(MODE_WORDS[mode]), testId: dailyModeTestid(mode) }))}
          />
        </div>
      )}
      {pick.mode === "random" ? (
        <>
          <p className="lobby-note">{t("No deck needed: the server deals both of you one when the game starts.")}</p>
          <LeanSetPickerLazy value={pick.lean} disabled={disabled} testid={dailyTestid.lean} onChange={pick.setLean} />
        </>
      ) : (
        <DeckField pick={pick} disabled={disabled} />
      )}
      {withTime ? (
        <div className="daily-field">
          <span className="daily-field__label">{t("Time per move")}</span>
          {options.length === 0 ? (
            <Skeleton variant="panel" label={t("Loading…")} />
          ) : (
            <Segmented
              label={t("Time per move")}
              testId={dailyTestid.perMove}
              value={String(pick.perMoveDays ?? options[0])}
              disabled={disabled}
              onChange={(value) => {
                pick.setPerMoveDays(Number(value));
              }}
              options={options.map((days) => ({ value: String(days), label: perMoveText(days), testId: dailyPerMoveTestid(days) }))}
            />
          )}
        </div>
      ) : null}
    </div>
  );
}

function DeckField({ pick, disabled }: { pick: DailyPick; disabled: boolean }): ReactElement {
  if (pick.decksError !== null) {
    return (
      <>
        <p className="notice" role="alert">
          {t("Your decks could not be loaded: {message}", { message: t(pick.decksError) })}
        </p>
        <div className="row">
          <button type="button" data-testid={dailyTestid.decksRetry} onClick={pick.retryDecks}>
            {t("Try again")}
          </button>
        </div>
      </>
    );
  }
  if (pick.decks === null) return <Skeleton variant="panel" label={t("Loading your decks…")} />;
  if (pick.decks.length === 0) {
    return (
      <p className="lobby-note">
        <a href={paths.decks} onClick={followInApp(paths.decks)}>
          {t("Build a deck first")}
        </a>
      </p>
    );
  }
  return (
    <label className="daily-field">
      <span className="daily-field__label">{t("Your deck")}</span>
      <select
        className="lobby-select"
        data-testid={dailyTestid.deck}
        value={pick.deckId ?? ""}
        disabled={disabled}
        onChange={(event) => {
          pick.setDeckId(event.target.value);
        }}
      >
        {pick.decks.map((deck) => (
          <option key={deck.id} value={deck.id}>
            {deck.name}
          </option>
        ))}
      </select>
    </label>
  );
}
