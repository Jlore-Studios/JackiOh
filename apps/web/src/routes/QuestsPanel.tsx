// `/collection`'s quests (SPEC §14.9, #1304; moved from `/account` by #1307): each quest the account
// holds, its progress toward its target and what it pays, and how long until the next one arrives.
//
// The server decides all of it (CLAUDE.md rule 7): `GET /api/quests` tops the account up first
// (R1304000) and answers the held quests and when the next falls due, `null` while the account holds
// `QUEST_MAX_HELD` or the economy is off (R1304002). This only counts that moment down on the clock
// (`auth/cooldown.ts`), so a background tab's frozen timers never put it wrong. A quest's title is the
// pool's English, looked up as the server's sentences are.

import { useEffect, useState, type ReactElement } from "react";

import { QUEST_MAX_HELD } from "@jackioh/server-config";
import { useSecondsUntil } from "../auth/cooldown.ts";
import { formatNumber, t } from "../i18n/i18n.ts";
import { getQuests, type QuestsResponse } from "../net/api.ts";

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
function nextWords(quests: QuestsResponse, left: number): string | null {
  if (quests.nextDueAt !== null) {
    return left > 0
      ? t("Next quest in {time}", { time: countdownWords(left) })
      : t("A new quest has arrived: reopen this page to see it.");
  }
  return quests.quests.length >= QUEST_MAX_HELD
    ? t("You hold {max} quests: finish one to make room for the next.", { max: formatNumber(QUEST_MAX_HELD) })
    : null;
}

/** The quests the account holds and the time until the next; nothing while none is held or due. */
export function QuestsList({ quests }: { quests: QuestsResponse }): ReactElement | null {
  const left = useSecondsUntil(quests.nextDueAt);
  if (quests.quests.length === 0 && quests.nextDueAt === null) return null;
  const next = nextWords(quests, left);
  return (
    <div className="account-quests" data-testid={questsTestid.panel}>
      <h3 className="account-heading">{t("Quests")}</h3>
      {quests.quests.length === 0 ? (
        <p>{t("No quests yet.")}</p>
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
              </li>
            );
          })}
        </ul>
      )}
      {next !== null ? <p data-testid={questsTestid.next}>{next}</p> : null}
    </div>
  );
}

/**
 * The caller's quests, read on mount and again on Try again; a failed read says why in the
 * panel's place, so the screen around it still shows. The panel is loading until the quests
 * arrive, so loading never looks like having no quests.
 */
export default function QuestsPanel({ token }: { token: string }): ReactElement | null {
  const [quests, setQuests] = useState<QuestsResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    let cancelled = false;
    getQuests(token)
      .then((next) => {
        if (!cancelled) setQuests(next);
      })
      .catch((cause: unknown) => {
        if (!cancelled) setError(cause instanceof Error ? cause.message : String(cause));
      });
    return () => {
      cancelled = true;
    };
  }, [token, attempt]);

  if (error !== null) {
    return (
      <div className="account-quests" data-testid={questsTestid.panel}>
        <h3 className="account-heading">{t("Quests")}</h3>
        <p className="notice" role="alert" data-testid={questsTestid.error}>
          {t(error)}
        </p>
        <div className="row">
          <button
            type="button"
            data-testid={questsTestid.retry}
            onClick={() => {
              setError(null);
              setAttempt((before) => before + 1);
            }}
          >
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
        <p className="notice" role="status" data-testid={questsTestid.loading}>
          {t("Loading your quests…")}
        </p>
      </div>
    );
  }
  return <QuestsList quests={quests} />;
}
