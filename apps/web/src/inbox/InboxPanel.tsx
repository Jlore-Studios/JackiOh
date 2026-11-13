// The inbox's panel (R3729002): everything waiting on the player in one list, each row with its answer inline.
// A live challenge (Accept, Decline; Open in lobby when there is no deck to join with) shows the time it has
// left, ticking; a lobby invite (Join, Dismiss); a daily game on the player's move (Play); a daily challenge
// (Open, which leads to the Daily screen where accepting asks for a deck; Decline); a friend request (Accept,
// Decline); a replay a friend shared (Watch). The order and the lists are `order.ts`'s and `store.ts`'s.
//
// CLAUDE.md rule 7: the panel sends intent and shows what the server answered. Whether a challenge can be
// joined, a request accepted or a deck played is the server's, and every refusal is shown in its words, in
// the panel's one alert. Opening the panel reads the lists again; any answer reads them and the social read
// again. Opening it also marks the shared replays it lists as seen, so the bell stops counting them.

import { useEffect, useState, type ReactElement, type ReactNode } from "react";

import Username from "../auth/Username.tsx";
import { isUrgent, perMoveRowText } from "../daily/order.ts";
import { moveLine } from "../daily/DailyClockPill.tsx";
import { EmptyState } from "../empty/EmptyState.tsx";
import { Dialog } from "../game/Dialog.tsx";
import { t, useLocale } from "../i18n/i18n.ts";
import { declineDailyChallenge } from "../net/daily.ts";
import { attempt, messageOf } from "../net/errors.ts";
import { getDecks, joinRoom } from "../net/api.ts";
import { withSession } from "../net/gate.ts";
import { leaveFirst, navigate, paths } from "../net/navigate.ts";
import { lobbyLinkPath } from "../net/roomLink.ts";
import { acceptFriendRequest, declineChallenge, declineFriendRequest } from "../net/social.ts";
import { followInApp } from "../routes/nav.tsx";
import { gameAfterJoin, rememberedChoice, type SavedPicks } from "../together/challengeAnswer.ts";
import { rememberChallengeChoice } from "../together/challengeChoice.ts";
import { reloadSocial } from "../together/socialPoll.ts";
import { sharedWithMePath } from "../together/anchors.ts";
import { MODE_WORDS, Named } from "../together/words.tsx";
import { useEngineReady } from "../wasm/useEngineReady.ts";
import { dropInvite } from "./invites.ts";
import { challengeLeftText, type InboxItem } from "./order.ts";
import { markSeen, refreshInbox } from "./store.ts";
import { inboxTestid } from "./testids.ts";

import "../auth/tavern.css";
import "../routes/lobby.css";
import "../together/together.css";
import "./inbox.css";

type Item<K extends InboxItem["kind"]> = Extract<InboxItem, { kind: K }>;

/** Adds `key` to a set without changing the one React holds. */
function including(keys: ReadonlySet<string>, key: string): ReadonlySet<string> {
  const next = new Set(keys);
  next.add(key);
  return next;
}

/** What a row's answers need from the panel: whether one is out (`busy`), and the way to send an answer. */
type Answering = {
  /** The key of the row whose answer is out, if any: every row waits on it. */
  busy: string | null;
  /** The key of the row the server refused, whose Open in lobby shows. */
  refused: string | null;
  /** Runs one answer for the row `key`; its refusal is the panel's alert. */
  answer: (key: string, work: () => Promise<unknown>) => void;
  onClose: () => void;
};

function Row({ item, children }: { item: InboxItem; children: ReactNode }): ReactElement {
  return (
    <li className="together-row inbox-row" data-testid={inboxTestid.item} data-kind={item.kind} data-key={item.key}>
      {children}
    </li>
  );
}

function ChallengeRow({
  item,
  now,
  saved,
  panel,
}: {
  item: Item<"challenge">;
  now: number;
  saved: SavedPicks | "failed" | null;
  panel: Answering;
}): ReactElement {
  const { challenge } = item;
  const { busy, refused, answer, onClose } = panel;
  const choice = rememberedChoice(challenge.mode, saved === "failed" ? null : saved);
  const waiting = busy !== null;
  // Nothing to join with (no deck or trio saved, or the decks would not read), or the join was refused: the room's lobby.
  const lobby = refused === item.key || (challenge.mode !== "random" && saved !== null && choice === null);
  return (
    <Row item={item}>
      <span className="inbox-row__text">
        <span>
          <Named text={t("{name} challenges you to {mode}.", { mode: t(MODE_WORDS[challenge.mode]) })} name={challenge.from.username} />
        </span>
        {Number.isFinite(item.expiresAt) ? (
          <span className="inbox-row__meta" data-testid={inboxTestid.left}>
            {challengeLeftText(item.expiresAt - now)}
          </span>
        ) : null}
        {challenge.mode !== "random" && saved === null ? <span className="inbox-row__meta">{t("Loading your decks…")}</span> : null}
      </span>
      <span className="together-row__actions">
        {choice === null ? null : (
          <button
            type="button"
            className="button-primary"
            data-testid={inboxTestid.accept}
            disabled={waiting}
            aria-busy={busy === item.key}
            onClick={() => {
              // R1754000: unsaved work may hold the join; until the player goes, the row stays.
              leaveFirst(() => {
                answer(item.key, async () => {
                  const joined = await withSession((token) => joinRoom(token, challenge.roomCode, choice));
                  rememberChallengeChoice(choice, false);
                  onClose();
                  // The player let any unsaved work go before the join (`leaveFirst`).
                  navigate(gameAfterJoin(joined), { force: true });
                });
              });
            }}
          >
            {t("Accept")}
          </button>
        )}
        {lobby ? (
          <button
            type="button"
            data-testid={inboxTestid.lobby}
            data-route={lobbyLinkPath(challenge.roomCode)}
            disabled={waiting}
            onClick={() => {
              // R3730006: the room's lobby screen joins it with a pick; a guarded page may hold the move.
              if (navigate(lobbyLinkPath(challenge.roomCode))) onClose();
            }}
          >
            {t("Open in lobby")}
          </button>
        ) : null}
        <button
          type="button"
          data-testid={inboxTestid.decline}
          disabled={waiting}
          aria-busy={busy === item.key}
          onClick={() => {
            answer(item.key, () => declineChallenge(challenge.challengeId));
          }}
        >
          {t("Decline")}
        </button>
      </span>
    </Row>
  );
}

function InviteRow({ item, onClose }: { item: Item<"invite">; onClose: () => void }): ReactElement {
  const { invite } = item;
  const sentence = invite.format === "coop" ? t("{name} invites you to a co-op lobby") : t("{name} invites you to a 1v1 lobby");
  return (
    <Row item={item}>
      <span className="inbox-row__text">
        <span>
          <Named text={sentence} name={invite.from.username} />
        </span>
      </span>
      <span className="together-row__actions">
        <button
          type="button"
          className="button-primary"
          data-testid={inboxTestid.join}
          onClick={() => {
            // The lobby's own screen enters it; a guarded page may hold the move, and the invite stays until it goes.
            if (navigate(paths.lobby(invite.code))) {
              dropInvite(invite.code);
              onClose();
            }
          }}
        >
          {t("Join")}
        </button>
        <button
          type="button"
          data-testid={inboxTestid.dismiss}
          onClick={() => {
            dropInvite(invite.code);
          }}
        >
          {t("Dismiss")}
        </button>
      </span>
    </Row>
  );
}

function DailyMoveRow({ item, now, onClose }: { item: Item<"dailyMove">; now: number; onClose: () => void }): ReactElement {
  const { game } = item;
  // A time left is worked out by the engine (`dailyConstants`); a screen that opens before it is in says only whose move it is.
  const engineReady = useEngineReady();
  const left = game.deadline === null ? null : game.deadline - now;
  const href = paths.dailyGame(game.matchId);
  return (
    <Row item={item}>
      <span className="inbox-row__text">
        <span>
          <Username name={game.opponent.username} />
        </span>
        <span className="inbox-row__meta" data-urgent={engineReady && left !== null && isUrgent(left) ? "true" : "false"}>
          {engineReady && left !== null ? moveLine(true, left) : t("Your move")}
        </span>
      </span>
      <span className="together-row__actions">
        <a
          href={href}
          className="button-primary"
          data-testid={inboxTestid.play}
          onClick={(event) => {
            onClose();
            followInApp(href)(event);
          }}
        >
          {t("Play")}
        </a>
      </span>
    </Row>
  );
}

function DailyChallengeRow({ item, panel }: { item: Item<"dailyChallenge">; panel: Answering }): ReactElement {
  const { invite } = item;
  const { busy, answer, onClose } = panel;
  return (
    <Row item={item}>
      <span className="inbox-row__text">
        <span>
          <Named text={t("{name} challenged you")} name={invite.from.username} />
        </span>
        <span className="inbox-row__meta">{`${t(MODE_WORDS[invite.mode])} · ${perMoveRowText(invite.perMoveDays)}`}</span>
      </span>
      <span className="together-row__actions">
        {/* Accepting asks for a deck, so it happens on the Daily screen (its Accept dialog). */}
        <a
          href={paths.daily}
          className="button-primary"
          data-testid={inboxTestid.open}
          onClick={(event) => {
            onClose();
            followInApp(paths.daily)(event);
          }}
        >
          {t("Open")}
        </a>
        <button
          type="button"
          data-testid={inboxTestid.decline}
          disabled={busy !== null}
          aria-busy={busy === item.key}
          onClick={() => {
            answer(item.key, () => declineDailyChallenge(invite.id));
          }}
        >
          {t("Decline")}
        </button>
      </span>
    </Row>
  );
}

function RequestRow({ item, panel }: { item: Item<"request">; panel: Answering }): ReactElement {
  const { request } = item;
  const { busy, answer } = panel;
  return (
    <Row item={item}>
      <span className="inbox-row__text">
        <span>
          <Named text={t("{name} wants to be friends")} name={request.from.username} />
        </span>
      </span>
      <span className="together-row__actions">
        <button
          type="button"
          className="button-primary"
          data-testid={inboxTestid.accept}
          disabled={busy !== null}
          aria-busy={busy === item.key}
          onClick={() => {
            answer(item.key, () => acceptFriendRequest(request.requestId));
          }}
        >
          {t("Accept")}
        </button>
        <button
          type="button"
          data-testid={inboxTestid.decline}
          disabled={busy !== null}
          onClick={() => {
            answer(item.key, () => declineFriendRequest(request.requestId));
          }}
        >
          {t("Decline")}
        </button>
      </span>
    </Row>
  );
}

function ReplayRow({ item, onClose }: { item: Item<"replay">; onClose: () => void }): ReactElement {
  const { replay } = item;
  return (
    <Row item={item}>
      <span className="inbox-row__text">
        <span>
          <Named text={t("Shared by {name}")} name={replay.sharer.username} />
        </span>
      </span>
      <span className="together-row__actions">
        <button
          type="button"
          className="button-primary"
          data-testid={inboxTestid.watch}
          onClick={() => {
            onClose();
            // Play together opens the replay (`?replay=`, `TogetherRoute.tsx`).
            navigate(sharedWithMePath(replay.matchId));
          }}
        >
          {t("Watch")}
        </button>
      </span>
    </Row>
  );
}

export type InboxPanelProps = {
  /** The panel's `id`, for the bell's `aria-controls`. */
  id: string;
  /** What is waiting, in the inbox's order (`inboxItems`), as of `now`. */
  items: readonly InboxItem[];
  now: number;
  onClose: () => void;
};

export function InboxPanel({ id, items, now, onClose }: InboxPanelProps): ReactElement {
  useLocale();
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [refused, setRefused] = useState<string | null>(null);
  /** Rows answered here: gone before the next read says so. */
  const [gone, setGone] = useState<ReadonlySet<string>>(() => new Set());
  /** The player's decks and trios, read once a Best of 1 or Conquest challenge is listed; null until read. */
  const [saved, setSaved] = useState<SavedPicks | "failed" | null>(null);

  const shown = items.filter((item) => !gone.has(item.key));
  const needsDecks = shown.some((item) => item.kind === "challenge" && item.challenge.mode !== "random");
  const replayKeys = shown.filter((item) => item.kind === "replay").map((item) => item.key);
  const seen = replayKeys.join("\n");

  // Opening the panel reads the lists again.
  useEffect(() => {
    void refreshInbox(true);
  }, []);

  // The replays listed are seen, those that arrive while the panel is open too.
  useEffect(() => {
    markSeen(seen === "" ? [] : seen.split("\n"));
  }, [seen]);

  // A challenge that needs a deck reads the player's decks once.
  useEffect(() => {
    if (!needsDecks || saved !== null) return undefined;
    let canceled = false;
    withSession(getDecks).then(
      (answer) => {
        if (!canceled) setSaved({ decks: answer.decks, trios: answer.trios });
      },
      () => {
        if (!canceled) setSaved("failed");
      },
    );
    return () => {
      canceled = true;
    };
  }, [needsDecks, saved]);

  /** One answer, then the lists read again; a refusal is shown in the server's words. */
  const answer = (key: string, work: () => Promise<unknown>): void => {
    if (busy !== null) return;
    setBusy(key);
    setError(null);
    setRefused(null);
    attempt(work)
      .then(
        () => {
          setGone((keys) => including(keys, key));
        },
        (cause: unknown) => {
          setError(messageOf(cause));
          setRefused(key);
        },
      )
      .finally(() => {
        setBusy(null);
        void refreshInbox(true);
        reloadSocial();
      });
  };

  const panel: Answering = { busy, refused, answer, onClose };

  return (
    <Dialog role="dialog" sheet id={id} title={t("Inbox")} onClose={onClose} actions={[]} testid={inboxTestid.panel} closeTestid={inboxTestid.close} className="inbox-panel">
      {error === null ? null : (
        <p className="notice" data-testid={inboxTestid.error} role="alert">
          {t(error)}
        </p>
      )}
      {shown.length === 0 ? (
        <EmptyState text={t("Nothing is waiting for you.")} testId={inboxTestid.empty} />
      ) : (
        <ul className="together-list">
          {shown.map((item) => {
            switch (item.kind) {
              case "challenge":
                return <ChallengeRow key={item.key} item={item} now={now} saved={saved} panel={panel} />;
              case "invite":
                return <InviteRow key={item.key} item={item} onClose={onClose} />;
              case "dailyMove":
                return <DailyMoveRow key={item.key} item={item} now={now} onClose={onClose} />;
              case "dailyChallenge":
                return <DailyChallengeRow key={item.key} item={item} panel={panel} />;
              case "request":
                return <RequestRow key={item.key} item={item} panel={panel} />;
              case "replay":
                return <ReplayRow key={item.key} item={item} onClose={onClose} />;
            }
          })}
        </ul>
      )}
    </Dialog>
  );
}
