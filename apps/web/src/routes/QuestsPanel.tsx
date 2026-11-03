// `/collection`'s quests (SPEC §14.9, #1304, #1307): each held quest, its progress and pay, and the
// time until the next; the landing's bookmark (R2035000) shares `useQuests` and `QuestsView`.
// The server decides all of it (CLAUDE.md rule 7): `GET /api/quests` tops the account up first
// (R1304000) and answers when the next falls due, `null` at `QUEST_MAX_HELD` or with the economy off
// (R1304002). This only counts down on the clock (`auth/cooldown.ts`), so a background tab's frozen
// timers never put it wrong. A title is the pool's English, looked up as the server's sentences are.
// R2210000: the server counts the day's replaces and draws the new quest; this only asks.

import { useCallback, useEffect, useState, type ReactElement } from "react";

import { QUEST_MAX_HELD } from "@jackioh/server-config";
import { useSecondsUntil } from "../auth/cooldown.ts";
import { EmptyState } from "../empty/EmptyState.tsx";
import { formatNumber, t } from "../i18n/i18n.ts";
import { getQuests, keptPaths, replaceQuest, type QuestsResponse } from "../net/api.ts";
import { cachedRead } from "../net/readCache.ts";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { messageOf } from "../net/errors.ts";

/** Unit conversions, not configuration. */
const SECONDS_PER_MINUTE = 60;
const SECONDS_PER_HOUR = 60 * SECONDS_PER_MINUTE;
/** A clock's minutes and seconds take two digits each. */
const CLOCK_DIGITS = 2;

export const questsTestid = {
  panel: "quests-panel",
  quest: "quests-quest",
  progress: "quests-progress",
  next: "quests-next",
  error: "quests-error",
  /** #1568: the loading line shown while the quests are on their way. */
  loading: "quests-loading",
  /** #1568: a failed read offers Try again beside the alert. */
  retry: "quests-retry",
  /** R2210000: a held quest's Replace, and its confirm's two buttons. */
  replace: "quests-replace",
  replaceConfirm: "quests-replace-confirm",
  replaceCancel: "quests-replace-cancel",
  /** R2210000: why the server refused a replace. */
  replaceError: "quests-replace-error",
} as const;

/** Whole seconds as a clock reads them, `H:MM:SS`, the hours running past a day: 3725 is "1:02:05". */
export function countdownWords(seconds: number): string {
  const whole = Math.max(0, Math.floor(seconds));
  const hours = Math.floor(whole / SECONDS_PER_HOUR);
  const minutes = Math.floor((whole % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE);
  const rest = whole % SECONDS_PER_MINUTE;
  const two = (value: number): string => String(value).padStart(CLOCK_DIGITS, "0");
  return `${String(hours)}:${two(minutes)}:${two(rest)}`;
}

/** The sentence under the quests: the time until the next, or why none is coming; null for none. */
function nextWords(quests: QuestsResponse, left: number, rereads: boolean): string | null {
  if (quests.nextDueAt !== null) {
    if (left > 0) return t("Next quest in {time}", { time: countdownWords(left) });
    return rereads ? null : t("A new quest has arrived: reopen this page to see it.");
  }
  return quests.quests.length >= QUEST_MAX_HELD
    ? t("You hold {max} quests: finish one to make room for the next.", { max: formatNumber(QUEST_MAX_HELD) })
    : null;
}

/** The quests the account holds and the time until the next; nothing while none is held or due. */
export function QuestsList({
  quests,
  onArrived,
  onReplace,
}: {
  quests: QuestsResponse;
  onArrived?: () => void;
  /** R2210000: replaces a held quest; answers why it was refused, or null once it is done. */
  onReplace?: (questId: string) => Promise<string | null>;
}): ReactElement | null {
  const left = useSecondsUntil(quests.nextDueAt);
  const arrived = quests.nextDueAt !== null && left === 0;
  const [confirming, setConfirming] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [replaceError, setReplaceError] = useState<string | null>(null);
  // R2035000: a caller that can read again does, once per due time, instead of asking for a reopen.
  useEffect(() => {
    if (arrived && onArrived !== undefined) onArrived();
  }, [arrived, quests.nextDueAt, onArrived]);
  if (quests.quests.length === 0 && quests.nextDueAt === null) return null;
  const next = nextWords(quests, left, onArrived !== undefined);
  const canReplace = onReplace !== undefined && (quests.replacesLeft ?? 0) > 0;
  const confirmReplace = async (questId: string): Promise<void> => {
    if (onReplace === undefined) return;
    setBusy(true);
    const failure = await onReplace(questId);
    setBusy(false);
    setConfirming(null);
    setReplaceError(failure);
  };
  return (
    <div className="account-quests" data-testid={questsTestid.panel}>
      <h3 className="account-heading">{t("Quests")}</h3>
      {quests.quests.length === 0 ? (
        <EmptyState text={t("No quests yet.")} />
      ) : (
        <ul className="account-quests__list">
          {quests.quests.map((quest) => {
            const title = t(quest.title);
            return (
              <li
                key={quest.questId}
                className="account-quests__quest"
                data-testid={questsTestid.quest}
                data-quest-id={quest.questId}
              >
                <span className="account-quests__title">{title}</span>
                <progress max={quest.target} value={quest.progress} aria-label={title} />
                <span data-testid={questsTestid.progress}>
                  {t("{progress} of {target}", {
                    progress: formatNumber(quest.progress),
                    target: formatNumber(quest.target),
                  })}
                </span>
                <span className="account-quests__reward">
                  {t("{jlore} Jlore · {xp} XP", { jlore: formatNumber(quest.jlore), xp: formatNumber(quest.xp) })}
                </span>
                {canReplace ? (
                  confirming === quest.questId ? (
                    <div className="row" role="alertdialog" aria-label={t("Replace this quest?")}>
                      <p>{t("You lose its progress and get a different quest.")}</p>
                      <button
                        type="button"
                        data-testid={questsTestid.replaceConfirm}
                        disabled={busy}
                        onClick={() => {
                          void confirmReplace(quest.questId);
                        }}
                      >
                        {t("Replace it")}
                      </button>
                      <button
                        type="button"
                        className="link-button"
                        data-testid={questsTestid.replaceCancel}
                        onClick={() => {
                          setConfirming(null);
                        }}
                      >
                        {t("Keep it")}
                      </button>
                    </div>
                  ) : (
                    <button
                      type="button"
                      className="link-button"
                      data-testid={questsTestid.replace}
                      aria-label={t("Replace {quest}", { quest: title })}
                      disabled={busy}
                      onClick={() => {
                        setReplaceError(null);
                        setConfirming(quest.questId);
                      }}
                    >
                      {t("Replace")}
                    </button>
                  )
                ) : null}
              </li>
            );
          })}
        </ul>
      )}
      {replaceError !== null ? (
        <p className="notice" role="alert" data-testid={questsTestid.replaceError}>
          {t(replaceError)}
        </p>
      ) : null}
      {next !== null ? <p data-testid={questsTestid.next}>{next}</p> : null}
    </div>
  );
}

/**
 * The caller's quests, read on mount and on Try again. R2125000: it starts on the quests the last
 * visit kept, and a read that fails behind them keeps them and says nothing.
 */
export function useQuests(token: string): {
  quests: QuestsResponse | null;
  error: string | null;
  reread: () => void;
  replace: (questId: string) => Promise<string | null>;
} {
  const [quests, setQuests] = useState<QuestsResponse | null>(
    () => cachedRead<QuestsResponse>(token, keptPaths.quests) ?? null,
  );
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    let canceled = false;
    getQuests(token)
      .then((next) => {
        if (!canceled) setQuests(next);
      })
      .catch((cause: unknown) => {
        if (canceled) return;
        const kept = cachedRead<QuestsResponse>(token, keptPaths.quests);
        if (kept === undefined) setError(messageOf(cause));
        else setQuests(kept);
      });
    return () => {
      canceled = true;
    };
  }, [token, attempt]);

  const reread = useCallback((): void => {
    setError(null);
    setAttempt((before) => before + 1);
  }, []);

  // R2210000: the quests the server answers replace the ones shown; a refusal says why and reads the
  // quests again, since the day's replaces or the held quests are not what the panel showed.
  const replace = useCallback(
    async (questId: string): Promise<string | null> => {
      try {
        setQuests(await replaceQuest(token, questId));
        return null;
      } catch (cause: unknown) {
        setAttempt((before) => before + 1);
        return messageOf(cause);
      }
    },
    [token],
  );
  return { quests, error, reread, replace };
}

/**
 * The quests, the failure (in the panel's place, so the screen around it still shows) or the wait,
 * so loading never looks like having no quests.
 */
export function QuestsView({
  quests,
  error,
  onRetry,
  onArrived,
  onReplace,
}: {
  quests: QuestsResponse | null;
  error: string | null;
  onRetry: () => void;
  onArrived?: () => void;
  onReplace?: (questId: string) => Promise<string | null>;
}): ReactElement | null {
  if (error !== null) {
    return (
      <div className="account-quests" data-testid={questsTestid.panel}>
        <h3 className="account-heading">{t("Quests")}</h3>
        <p className="notice" role="alert" data-testid={questsTestid.error}>
          {t(error)}
        </p>
        <div className="row">
          <button type="button" data-testid={questsTestid.retry} onClick={onRetry}>
            {t("Try again")}
          </button>
        </div>
      </div>
    );
  }
  if (quests === null) {
    // Not the panel's testid: that stands for quests read, or a read that failed.
    return (
      <div className="account-quests">
        <h3 className="account-heading">{t("Quests")}</h3>
        <Skeleton variant="rows" label={t("Loading your quests…")} testId={questsTestid.loading} />
      </div>
    );
  }
  return <QuestsList quests={quests} onArrived={onArrived} onReplace={onReplace} />;
}

export default function QuestsPanel({ token }: { token: string }): ReactElement | null {
  const { quests, error, reread, replace } = useQuests(token);
  return <QuestsView quests={quests} error={error} onRetry={reread} onReplace={replace} />;
}
