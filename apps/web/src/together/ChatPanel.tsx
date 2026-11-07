// A conversation with one friend (#936, B53, B58): the newest page, read again every `CHAT_POLL_MS`
// while the panel is open (a read marks it read), and earlier pages by cursor (R1384000). R3214000: while
// the tab's social socket is live (`socialPoll.ts`) there is no poll: the newest page is read when the
// server pushes a message to or from this friend, and once each time the socket comes up or drops; a
// hidden tab holds such a read until it shows (R3205000), since a read marks the page read.
// CLAUDE.md rule 7: the box keeps a draft to `CHAT_MAX_CHARS` and will not send an empty one, as
// UX; what a message may say, how many may be sent, and the word filter are the server's, and its
// refusal is shown in its own words. A message is drawn as text, never as markup.
// R3221000: a modal sheet, the one Dialog's: Escape, Back and its ✕ close it, and the focus starts in the box.

import { useCallback, useEffect, useRef, useState, type FormEvent, type ReactElement } from "react";

import { CHAT_MAX_CHARS, CHAT_POLL_MS } from "@jackioh/server-config";

import { TryAgainButton } from "../auth/AccountError.tsx";
import { Dialog } from "../game/Dialog.tsx";
import { pageWake, pollWhileVisible } from "../game/net.ts";
import { formatDateTime, t, useLocale } from "../i18n/i18n.ts";
import { readFailureOf, type ReadFailure } from "../net/gate.ts";
import { navigate } from "../net/navigate.ts";
import { findRoomLinkInText } from "../net/roomLink.ts";
import { getChat, reportMessage, sendChat, type ChatMessage, type PlayerRef } from "../net/social.ts";
import { onChatPush, useSocialReads } from "./socialPoll.ts";
import { chatTestid } from "./testids.ts";
import { Named } from "./words.tsx";
import { attempt, messageOf } from "../net/errors.ts";

import "./together.css";

/** What a report says happened; the server stores it as given. */
const REPORT_REASONS = ["harassment", "spam", "other"] as const;
type ReportReason = (typeof REPORT_REASONS)[number];

const REPORT_WORDS: Readonly<Record<ReportReason, string>> = {
  harassment: "Harassment or hate",
  spam: "Spam",
  other: "Something else",
};

function sentAt(iso: string): string {
  const at = Date.parse(iso);
  return Number.isFinite(at) ? formatDateTime(at, { timeStyle: "short" }) : "";
}

/** Oldest first by time sent, then by id, so two reads of the same messages draw them alike. */
function byTime(a: ChatMessage, b: ChatMessage): number {
  const at = Date.parse(a.createdAt) - Date.parse(b.createdAt);
  if (at !== 0 && Number.isFinite(at)) return at;
  return a.id < b.id ? -1 : a.id > b.id ? 1 : 0;
}

/** The messages held so far with a page merged in; a message read twice is kept once, as last read. */
export function mergeMessages(held: readonly ChatMessage[], page: readonly ChatMessage[]): ChatMessage[] {
  const all = new Map<string, ChatMessage>();
  for (const message of held) all.set(message.id, message);
  for (const message of page) all.set(message.id, message);
  return [...all.values()].sort(byTime);
}

/** A send no answer of the server's has replaced yet (R2120000); `key` is the panel's own, never a message id. */
type Outgoing = { key: string; body: string } & ({ state: "sending" } | { state: "failed"; error: string });

export function ChatPanel({ friend, onClose }: { friend: PlayerRef; onClose: () => void }): ReactElement {
  useLocale();
  const [messages, setMessages] = useState<readonly ChatMessage[]>([]);
  /** A read of the newest page has answered: before one has, no messages is no empty conversation (#1898). */
  const [loaded, setLoaded] = useState(false);
  /** The cursor of the page before the oldest held (R1384000); null when nothing older is left. */
  const [earlier, setEarlier] = useState<string | null>(null);
  const [loadingEarlier, setLoadingEarlier] = useState(false);
  const [draft, setDraft] = useState("");
  const [outgoing, setOutgoing] = useState<readonly Outgoing[]>([]);
  const [error, setError] = useState<string | null>(null);
  /** The newest page's read failed and none has answered since (#1898); a refused send or report is `error`'s. */
  const [readFailure, setReadFailure] = useState<ReadFailure | null>(null);
  const [reporting, setReporting] = useState<string | null>(null);
  const [reason, setReason] = useState<ReportReason>("harassment");
  const [reported, setReported] = useState<ReadonlySet<string>>(() => new Set());
  const list = useRef<HTMLOListElement>(null);
  const input = useRef<HTMLInputElement>(null);
  /** The poll's read, for Try again to make at once (#1898). */
  const readNow = useRef<() => void>(() => undefined);
  const sendCount = useRef(0);
  /** The last send queued: each waits for the one before it to settle, so the server stores them in the order sent (R2120000). */
  const sendQueue = useRef<Promise<void>>(Promise.resolve());
  /** Sends queued or on their way. While any is, a read takes only the friend's messages (R2120000). */
  const sendsOpen = useRef(0);
  const friendId = friend.profileId;
  const { live } = useSocialReads();
  /** Whether the social socket was live when the poll's effect last ran; null before it has. */
  const lastLive = useRef<boolean | null>(null);
  /** R3205000: a read the socket asked for while the tab was hidden, made once when it shows. */
  const held = useRef(false);
  /** Reads now, or while the tab is hidden holds the read for when it shows (a read marks the page read). */
  const readWhenShown = useRef<() => void>(() => undefined);

  useEffect(() => {
    let canceled = false;
    let first = true;
    held.current = false;
    const read = (): void => {
      attempt(() => getChat(friendId)).then(
        (page) => {
          if (canceled) return;
          // R2120000: while a send is open the player's own messages wait for the sends' answers, so a message a
          // read brings back before its send has answered is not drawn beside its pending copy.
          const fresh = sendsOpen.current === 0 ? page.messages : page.messages.filter((message) => message.from === friendId);
          setMessages((held) => mergeMessages(held, fresh));
          if (first) setEarlier(page.nextCursor);
          first = false;
          setLoaded(true);
          setReadFailure(null);
        },
        (cause: unknown) => {
          if (canceled) return;
          // The failure first shown stays until a read answers, so a wait it stated is counted once.
          setReadFailure((shown) => shown ?? readFailureOf(cause));
        },
      );
    };
    readNow.current = read;
    readWhenShown.current = () => {
      if (document.visibilityState === "hidden") held.current = true;
      else read();
    };
    read();
    // R3214000: a message to or from this friend, pushed over the live social socket.
    const stopPushes = onChatPush((from) => {
      if (from === friendId) readWhenShown.current();
    });
    return () => {
      canceled = true;
      stopPushes();
    };
  }, [friendId]);

  useEffect(() => {
    // R3214000: the socket came up or dropped, so a message between the last read and now is read.
    if (lastLive.current !== null && lastLive.current !== live) readWhenShown.current();
    lastLive.current = live;
    // R3205000: a hidden tab reads nothing, and reads once when it shows again: live, only a read it held.
    if (live) {
      return pageWake(() => {
        // `pageWake` also wakes on the network's return, hidden or not.
        if (!held.current || document.visibilityState === "hidden") return;
        held.current = false;
        readNow.current();
      });
    }
    return pollWhileVisible(() => {
      held.current = false;
      readNow.current();
    }, CHAT_POLL_MS);
  }, [friendId, live]);

  const newest = messages.at(-1)?.id ?? null;
  const newestSend = outgoing.at(-1)?.key ?? null;
  useEffect(() => {
    const box = list.current;
    if (box !== null) box.scrollTop = box.scrollHeight;
  }, [newest, newestSend]);

  const loadEarlier = useCallback((): void => {
    if (earlier === null || loadingEarlier) return;
    setLoadingEarlier(true);
    attempt(() => getChat(friendId, earlier))
      .then(
        (page) => {
          setMessages((held) => mergeMessages(held, page.messages));
          setEarlier(page.nextCursor);
        },
        (cause: unknown) => {
          setError(messageOf(cause));
        },
      )
      .finally(() => {
        setLoadingEarlier(false);
      });
  }, [friendId, earlier, loadingEarlier]);

  const dispatch = (key: string, body: string): void => {
    sendsOpen.current += 1;
    sendQueue.current = sendQueue.current.then(() =>
      attempt(() => sendChat(friendId, body)).then(
        (answer) => {
          sendsOpen.current -= 1;
          setMessages((held) => mergeMessages(held, [answer.message]));
          setOutgoing((sends) => sends.filter((send) => send.key !== key));
        },
        (cause: unknown) => {
          sendsOpen.current -= 1;
          const error = messageOf(cause);
          setOutgoing((sends) => sends.map((send): Outgoing => (send.key === key ? { key, body, state: "failed", error } : send)));
        },
      ),
    );
  };

  const send = (event: FormEvent<HTMLFormElement>): void => {
    event.preventDefault();
    const body = draft.trim();
    if (body.length === 0) return;
    setError(null);
    sendCount.current += 1;
    const entry: Outgoing = { key: `send-${String(sendCount.current)}`, body, state: "sending" };
    setOutgoing((sends) => [...sends, entry]);
    setDraft("");
    input.current?.focus({ preventScroll: true });
    dispatch(entry.key, body);
  };

  const retry = (key: string, body: string): void => {
    setOutgoing((sends) => sends.map((send): Outgoing => (send.key === key ? { key, body, state: "sending" } : send)));
    dispatch(key, body);
  };

  const discard = (key: string): void => {
    setOutgoing((sends) => sends.filter((send) => send.key !== key));
  };

  const report = (messageId: string): void => {
    setError(null);
    attempt(() => reportMessage(messageId, reason)).then(
      () => {
        setReported((ids) => new Set(ids).add(messageId));
        setReporting(null);
      },
      (cause: unknown) => {
        setError(messageOf(cause));
      },
    );
  };

  const retryRead = (): void => {
    setReadFailure(null);
    readNow.current();
  };

  // R2127000: Back closes the conversation as its ✕ does, and the board's own Escape (a selection, a
  // prompt) does not reach the board while it is open (Dialog's keys).
  return (
    <Dialog
      role="dialog"
      sheet
      title={<Named text={t("Chat with {name}")} name={friend.username} />}
      onClose={onClose}
      actions={[]}
      testid={chatTestid.panel}
      data={{ "data-friend-id": friendId }}
      closeTestid={chatTestid.close}
      closeLabel={t("Close the chat")}
      className="chat-panel"
      focus={input}
    >
      <ol className="chat-panel__messages" ref={list} aria-live="polite">
        {earlier !== null ? (
          <li className="chat-panel__earlier">
            <button type="button" className="link-button" data-testid={chatTestid.earlier} disabled={loadingEarlier} onClick={loadEarlier}>
              {loadingEarlier ? t("Loading…") : t("Show earlier messages")}
            </button>
          </li>
        ) : null}
        {loaded && readFailure === null && messages.length === 0 && outgoing.length === 0 ? <li className="chat-panel__empty">{t("No messages yet. Say hello!")}</li> : null}
        {messages.map((message) => {
          const yours = message.from !== friendId;
          // R2094000: the button is built from the parsed link, never from message markup.
          const link = findRoomLinkInText(message.body, window.location.origin);
          return (
            <li
              key={message.id}
              className="chat-message"
              data-testid={chatTestid.message}
              data-message-id={message.id}
              data-from={yours ? "you" : "friend"}
            >
              <p className="chat-message__body">{message.body}</p>
              {link === null ? null : (
                <button
                  type="button"
                  data-testid={chatTestid.join}
                  data-route={link.path}
                  onClick={() => {
                    navigate(link.path);
                  }}
                >
                  {t("Join")}
                </button>
              )}
              <p className="chat-message__meta">
                <time dateTime={message.createdAt}>{sentAt(message.createdAt)}</time>
                {yours ? null : reported.has(message.id) ? (
                  <span data-testid={chatTestid.reported}> · {t("Reported")}</span>
                ) : reporting === message.id ? (
                  <span className="chat-message__report">
                    {" "}
                    <select
                      data-testid={chatTestid.reportReason}
                      aria-label={t("Why are you reporting this message?")}
                      value={reason}
                      onChange={(event) => {
                        setReason(event.target.value as ReportReason);
                      }}
                    >
                      {REPORT_REASONS.map((option) => (
                        <option key={option} value={option}>
                          {t(REPORT_WORDS[option])}
                        </option>
                      ))}
                    </select>{" "}
                    <button
                      type="button"
                      className="link-button"
                      data-testid={chatTestid.reportSend}
                      onClick={() => {
                        report(message.id);
                      }}
                    >
                      {t("Send report")}
                    </button>{" "}
                    <button
                      type="button"
                      className="link-button"
                      onClick={() => {
                        setReporting(null);
                      }}
                    >
                      {t("Cancel")}
                    </button>
                  </span>
                ) : (
                  <>
                    {" · "}
                    <button
                      type="button"
                      className="link-button"
                      data-testid={chatTestid.report}
                      onClick={() => {
                        setReporting(message.id);
                      }}
                    >
                      {t("Report")}
                    </button>
                  </>
                )}
              </p>
            </li>
          );
        })}
        {outgoing.map((send) => (
          <li key={send.key} className="chat-message" data-testid={chatTestid.pending} data-from="you" data-state={send.state}>
            <p className="chat-message__body">{send.body}</p>
            {send.state === "sending" ? (
              <p className="chat-message__meta">{t("Sending…")}</p>
            ) : (
              <p className="chat-message__meta">
                {t(send.error)}
                {" · "}
                <button
                  type="button"
                  className="link-button"
                  data-testid={chatTestid.pendingRetry}
                  onClick={() => {
                    retry(send.key, send.body);
                  }}
                >
                  {t("Retry")}
                </button>
                {" · "}
                <button
                  type="button"
                  className="link-button"
                  data-testid={chatTestid.pendingDiscard}
                  onClick={() => {
                    discard(send.key);
                  }}
                >
                  {t("Discard")}
                </button>
              </p>
            )}
          </li>
        ))}
      </ol>

      {readFailure === null ? null : (
        <p className="notice chat-panel__error" data-testid={chatTestid.readError} role="alert">
          {t(readFailure.message)}{" "}
          <TryAgainButton retryAfterMs={readFailure.retryAfterMs} onRetry={retryRead} testId={chatTestid.retry} className="link-button" />
        </p>
      )}

      {error === null ? null : (
        <p className="notice chat-panel__error" data-testid={chatTestid.error} role="alert">
          {t(error)}
        </p>
      )}

      <form className="chat-panel__compose" onSubmit={send}>
        <input
          ref={input}
          data-testid={chatTestid.input}
          value={draft}
          maxLength={CHAT_MAX_CHARS}
          autoComplete="off"
          aria-label={t("Message")}
          placeholder={t("Write a message")}
          onChange={(event) => {
            setDraft(event.target.value);
          }}
        />
        <button type="submit" className="button-primary" data-testid={chatTestid.send} disabled={draft.trim().length === 0}>
          {t("Send")}
        </button>
      </form>
    </Dialog>
  );
}
