// `/daily` (#3727): daily games, played a move every few days. The games waiting on the player first, least
// time left first, then those waiting on the opponent, the challenges to and from friends, the seeks in the
// daily queue and the games that ended (`order.ts`); "New daily game"; and the notifications: email on or
// off, and push on this device (`push.ts`). The tab says "Your move (n)" while n games wait.
//
// Behind the gate like Play together. The list is the server's answer (`GET /api/daily`), read again after
// each of this screen's own changes and whenever the social read's `dailyWaiting` changes (a move passed
// to the player, a game ended). CLAUDE.md rule 7: who holds a move, its deadline and every refusal are the
// server's; rule 9: the numbers are the WebAssembly module's (`dailyConstants`).

import { useEffect, useRef, useState, type ReactElement } from "react";

import type { PortraitId } from "@jackioh/shared";

import { AccountError } from "../auth/AccountError.tsx";
import Username from "../auth/Username.tsx";
import { PortraitArt } from "../emotes/PortraitArt.tsx";
import { PORTRAIT_DEFS } from "../emotes/portraits.ts";
import { EmptyState } from "../empty/EmptyState.tsx";
import { Dialog } from "../game/Dialog.tsx";
import { RESULT_WORD } from "../game/Result.tsx";
import { t, useLocale } from "../i18n/i18n.ts";
import {
  acceptDailyChallenge,
  cancelDailyChallenge,
  cancelDailySeek,
  declineDailyChallenge,
  getDaily,
  type DailyGameSummary,
  type DailyInvite,
} from "../net/daily.ts";
import { attempt, messageOf } from "../net/errors.ts";
import { SITE_NAME, documentTitleFor } from "../net/head.ts";
import { navigate, paths } from "../net/navigate.ts";
import { BackLink, followInApp } from "../routes/nav.tsx";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { usePolled } from "../together/session.ts";
import { useSocialPoll } from "../together/socialPoll.ts";
import { MODE_WORDS, Named } from "../together/words.tsx";
import { DailyFields, acceptBodyOf, useDailyPick } from "./DailyFields.tsx";
import { moveLine, useNow } from "./DailyClockPill.tsx";
import { NewDailyDialog } from "./NewDailyDialog.tsx";
import { dailySections, isUrgent, perMoveRowText, yourMoveTitle, type DailySections } from "./order.ts";
import { NotificationSettings } from "./NotificationSettings.tsx";
import { dailyTestid } from "./testids.ts";

import "../auth/tavern.css";
import "../routes/lobby.css";
import "./daily.css";

type SectionName = keyof DailySections;

/** Each section's heading, in the order the screen shows them; wrapped in `t()` where shown. */
const SECTION_HEADINGS: readonly (readonly [SectionName, string])[] = [
  ["yourMove", "Your move"],
  ["theirMove", "Their move"],
  ["incoming", "Challenges for you"],
  ["outgoing", "Challenges you sent"],
  ["seeks", "Looking for an opponent"],
  ["finished", "Finished"],
];

function isPortrait(value: string): value is PortraitId {
  return Object.hasOwn(PORTRAIT_DEFS, value);
}

/** A row's mode and time: "All random · 3 days per move". */
function modeLine(entry: { mode: DailyGameSummary["mode"]; perMoveDays: number }): string {
  return `${t(MODE_WORDS[entry.mode])} · ${perMoveRowText(entry.perMoveDays)}`;
}

/** A finished game's line: the outcome, on time when the move's time ran out. */
function resultLine(game: DailyGameSummary): string {
  const result = game.result;
  if (result === null) return t("Finished");
  if (result.reason === "disconnect" && result.outcome !== "draw") return result.outcome === "win" ? t("Won on time") : t("Lost on time");
  return t(RESULT_WORD[result.outcome]);
}

function GameRow({ game, now }: { game: DailyGameSummary; now: number }): ReactElement {
  const href = paths.dailyGame(game.matchId);
  const left = game.deadline === null ? null : game.deadline - now;
  const urgent = game.hasMove && left !== null && isUrgent(left);
  return (
    <li
      className="daily-row"
      data-testid={dailyTestid.gameRow}
      data-match-id={game.matchId}
      data-has-move={game.hasMove ? "true" : "false"}
      data-over={game.over ? "true" : "false"}
    >
      <a className="daily-row__open" href={href} onClick={followInApp(href)}>
        {isPortrait(game.portraits.them) ? (
          <span className="daily-row__portrait">
            <PortraitArt portrait={game.portraits.them} />
          </span>
        ) : null}
        <span className="daily-row__who">
          <span className="daily-row__name">
            <Username name={game.opponent.username} />
          </span>
          <span className="daily-row__meta">
            {modeLine(game)} · {t("Turn {n}", { n: game.turn })}
          </span>
          <span className="daily-row__life">{t("You {me} – {them} Opponent", { me: game.life.me, them: game.life.them })}</span>
        </span>
        <span className="daily-row__state" data-testid={dailyTestid.timeLeft} data-urgent={urgent ? "true" : "false"}>
          {game.over ? resultLine(game) : left === null ? null : moveLine(game.hasMove, left)}
        </span>
      </a>
    </li>
  );
}

function InviteRow({
  invite,
  direction,
  busy,
  onAccept,
  onDecline,
  onCancel,
}: {
  invite: DailyInvite;
  direction: "incoming" | "outgoing" | "seek";
  busy: boolean;
  onAccept: () => void;
  onDecline: () => void;
  onCancel: () => void;
}): ReactElement {
  return (
    <li
      className="daily-row"
      data-testid={dailyTestid.inviteRow}
      data-invite-id={invite.id}
      data-kind={invite.kind}
      data-direction={direction}
    >
      <span className="daily-row__who">
        <span className="daily-row__name">
          {direction === "incoming" ? <Named text={t("{name} challenged you")} name={invite.from.username} /> : null}
          {direction === "outgoing" ? <Named text={t("Waiting for {name}")} name={invite.to?.username ?? ""} /> : null}
          {direction === "seek" ? t("Waiting for an opponent") : null}
        </span>
        <span className="daily-row__meta">{modeLine(invite)}</span>
      </span>
      <span className="daily-row__actions">
        {direction === "incoming" ? (
          <>
            <button type="button" className="button-primary" data-testid={dailyTestid.accept} disabled={busy} onClick={onAccept}>
              {t("Accept")}
            </button>
            <button type="button" data-testid={dailyTestid.decline} disabled={busy} onClick={onDecline}>
              {t("Decline")}
            </button>
          </>
        ) : (
          <button type="button" data-testid={dailyTestid.cancel} disabled={busy} onClick={onCancel}>
            {t("Cancel")}
          </button>
        )}
      </span>
    </li>
  );
}

/** Accept a friend's challenge with a deck (Best of 1) or a lean (All random): the challenge's own mode and time. */
function AcceptDialog({ invite, onClose }: { invite: DailyInvite; onClose: () => void }): ReactElement {
  const pick = useDailyPick(invite.mode);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const body = acceptBodyOf(pick);
  const accept = (): void => {
    if (body === null || sending) return;
    setSending(true);
    setError(null);
    attempt(() => acceptDailyChallenge(invite.id, body))
      .then(
        (answer) => {
          navigate(paths.dailyGame(answer.matchId));
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
      title={<Named text={t("Daily game with {name}")} name={invite.from.username} />}
      description={modeLine(invite)}
      onClose={onClose}
      cancel={{ label: t("Cancel"), kind: "secondary", testid: dailyTestid.acceptClose }}
      actions={[
        {
          label: t("Accept"),
          kind: "primary",
          onClick: accept,
          testid: dailyTestid.acceptSend,
          disabled: body === null || sending,
          busy: sending,
        },
      ]}
      testid={dailyTestid.acceptDialog}
      className="daily-dialog"
    >
      <DailyFields pick={pick} fixedMode withTime={false} disabled={sending} />
      {error === null ? null : (
        <p className="notice" data-testid={dailyTestid.dialogError} role="alert">
          {t(error)}
        </p>
      )}
    </Dialog>
  );
}

export default function DailyRoute(): ReactElement {
  const locale = useLocale();
  const [list, reload] = usePolled(getDaily, null);
  // The social read's count of the games waiting on the player: a change is a move or an end to show.
  const { dailyWaiting } = useSocialPoll();
  const now = useNow();
  const [creating, setCreating] = useState(false);
  const [accepting, setAccepting] = useState<DailyInvite | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const opener = useRef<HTMLButtonElement>(null);

  const waitingSeen = useRef(dailyWaiting);
  useEffect(() => {
    if (waitingSeen.current === dailyWaiting) return;
    waitingSeen.current = dailyWaiting;
    reload();
  }, [dailyWaiting, reload]);

  const sections = list.kind === "ready" ? dailySections(list.value, now) : null;
  const waiting = sections?.yourMove.length ?? 0;
  useEffect(() => {
    document.title = waiting > 0 ? `${yourMoveTitle(waiting)} · ${SITE_NAME}` : documentTitleFor(paths.daily);
  }, [waiting, locale]);

  const act = (work: () => Promise<void>): void => {
    if (busy) return;
    setBusy(true);
    setError(null);
    attempt(work)
      .then(
        () => undefined,
        (cause: unknown) => {
          setError(messageOf(cause));
        },
      )
      .finally(() => {
        setBusy(false);
        reload();
      });
  };

  const closeNew = (): void => {
    setCreating(false);
    opener.current?.focus({ preventScroll: true });
  };

  const empty =
    sections !== null && Object.values(sections).every((entries: readonly unknown[]) => entries.length === 0);

  return (
    <div className="app-shell tavern lobby daily-screen" data-testid={dailyTestid.root}>
      <BackLink title={t("Daily games")} />

      <div className="daily-top">
        <p className="lobby-note">{t("A move every few days, on your own time. We'll tell you when it's your move.")}</p>
        <button
          ref={opener}
          type="button"
          className="button-primary"
          data-testid={dailyTestid.newGame}
          onClick={() => {
            setCreating(true);
          }}
        >
          {t("New daily game")}
        </button>
      </div>

      {error === null ? null : (
        <p className="notice" data-testid={dailyTestid.error} role="alert">
          {t(error)}
        </p>
      )}

      <div className="daily-layout">
        <div className="daily-panel">
          {list.kind === "loading" ? <Skeleton variant="rows" label={t("Loading your daily games…")} /> : null}
          {list.kind === "failed" ? (
            <AccountError
              message={list.message}
              retryAfterMs={list.retryAfterMs}
              onRetry={reload}
              testId={dailyTestid.listError}
              retryTestId={dailyTestid.listRetry}
            />
          ) : null}
          {empty ? (
            <EmptyState text={t("No daily games yet. Start one, and play a move whenever you have a minute.")} testId={dailyTestid.empty} />
          ) : null}
          {sections === null
            ? null
            : SECTION_HEADINGS.map(([name, heading]) => {
                const entries = sections[name];
                if (entries.length === 0) return null;
                const headingId = `daily-section-${name}`;
                return (
                  <section key={name} className="lobby-card daily-panel" aria-labelledby={headingId} data-testid={dailyTestid.section} data-section={name}>
                    <h2 id={headingId} className="daily-panel__heading">
                      {t(heading)}
                    </h2>
                    <ul className="daily-list">
                      {name === "yourMove" || name === "theirMove" || name === "finished"
                        ? sections[name].map((game) => <GameRow key={game.matchId} game={game} now={now} />)
                        : sections[name].map((invite) => (
                            <InviteRow
                              key={invite.id}
                              invite={invite}
                              direction={name === "incoming" ? "incoming" : name === "outgoing" ? "outgoing" : "seek"}
                              busy={busy}
                              onAccept={() => {
                                setAccepting(invite);
                              }}
                              onDecline={() => {
                                act(() => declineDailyChallenge(invite.id));
                              }}
                              onCancel={() => {
                                act(() => (name === "seeks" ? cancelDailySeek(invite.id) : cancelDailyChallenge(invite.id)));
                              }}
                            />
                          ))}
                    </ul>
                  </section>
                );
              })}
        </div>
        <NotificationSettings />
      </div>

      {creating ? (
        <NewDailyDialog
          onClose={closeNew}
          onDone={(matchId) => {
            setCreating(false);
            if (matchId !== null) navigate(paths.dailyGame(matchId));
            else reload();
          }}
        />
      ) : null}
      {accepting === null ? null : (
        <AcceptDialog
          invite={accepting}
          onClose={() => {
            setAccepting(null);
            reload();
          }}
        />
      )}
    </div>
  );
}
