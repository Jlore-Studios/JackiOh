// A conversation with one friend (#936, B53, B58): the newest page of it, read again every
// `CHAT_POLL_MS` while the panel is open (a read marks it read), earlier pages on request, a reply
// box, and a report on each of the friend's messages.
//
// CLAUDE.md rule 7: the box keeps a draft to `CHAT_MAX_CHARS` and will not send an empty one, as
// UX; what a message may say, how many may be sent, and the word filter are the server's, and its
// refusal is shown in its own words. A message is drawn as text, never as markup.

import { useCallback, useEffect, useId, useRef, useState, type FormEvent, type ReactElement } from "react";

import { CHAT_MAX_CHARS, CHAT_PAGE, CHAT_POLL_MS } from "@jackioh/server-config";

import { formatDateTime, t, useLocale } from "../i18n/i18n.ts";
import { getChat, reportMessage, sendChat, type ChatMessage, type PlayerRef } from "../net/social.ts";
import { attempt, messageOf } from "./session.ts";
import { chatTestid } from "./testids.ts";
import { Named } from "./words.tsx";

import "./together.css";

/** What a report says happened; the server stores it as given. */
const REPORT_REASONS = ["harassment", "spam", "other"] as const;
type ReportReason = (typeof REPORT_REASONS)[number];

const REPORT_WORDS: Readonly<Record<ReportReason, string>> = {
  harassment: "Harassment or hate",
  spam: "Spam",
  other: "Something else",
};

/** When a message was sent, as the reader's clock says a time; nothing for a time that is not one. */
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

export function ChatPanel({ friend, onClose }: { friend: PlayerRef; onClose: () => void }): ReactElement {
  useLocale();
  const titleId = useId();
  const [messages, setMessages] = useState<readonly ChatMessage[]>([]);
  const [loaded, setLoaded] = useState(false);
  /** Whether a page before the oldest held may exist: the last page read before it was a full one. */
  const [earlier, setEarlier] = useState(false);
  const [loadingEarlier, setLoadingEarlier] = useState(false);
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [reporting, setReporting] = useState<string | null>(null);
  const [reason, setReason] = useState<ReportReason>("harassment");
  const [reported, setReported] = useState<ReadonlySet<string>>(() => new Set());
  const list = useRef<HTMLOListElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const friendId = friend.profileId;

  // The newest page now and every CHAT_POLL_MS while open (B58).
  useEffect(() => {
    let cancelled = false;
    let first = true;
    const read = (): void => {
      attempt(() => getChat(friendId)).then(
        (page) => {
          if (cancelled) return;
          setMessages((held) => mergeMessages(held, page.messages));
          if (first) setEarlier(page.messages.length >= CHAT_PAGE);
          first = false;
          setLoaded(true);
        },
        (cause: unknown) => {
          if (cancelled) return;
          setError(messageOf(cause));
          setLoaded(true);
        },
      );
    };
    read();
    const handle = window.setInterval(read, CHAT_POLL_MS);
    return () => {
      cancelled = true;
      window.clearInterval(handle);
    };
  }, [friendId]);

  // The newest message in view whenever one arrives.
  const newest = messages.at(-1)?.id ?? null;
  useEffect(() => {
    const box = list.current;
    if (box !== null) box.scrollTop = box.scrollHeight;
  }, [newest]);

  useEffect(() => {
    input.current?.focus({ preventScroll: true });
  }, []);

  const loadEarlier = useCallback((): void => {
    const oldest = messages[0];
    if (oldest === undefined || loadingEarlier) return;
    setLoadingEarlier(true);
    attempt(() => getChat(friendId, oldest.createdAt))
      .then(
        (page) => {
          setMessages((held) => mergeMessages(held, page.messages));
          setEarlier(page.messages.length >= CHAT_PAGE);
        },
        (cause: unknown) => {
          setError(messageOf(cause));
        },
      )
      .finally(() => {
        setLoadingEarlier(false);
      });
  }, [friendId, messages, loadingEarlier]);

  const send = (event: FormEvent<HTMLFormElement>): void => {
    event.preventDefault();
    const body = draft.trim();
    if (body.length === 0 || sending) return;
    setSending(true);
    setError(null);
    attempt(() => sendChat(friendId, body))
      .then(
        (answer) => {
          setMessages((held) => mergeMessages(held, [answer.message]));
          setDraft("");
        },
        (cause: unknown) => {
          setError(messageOf(cause));
        },
      )
      .finally(() => {
        setSending(false);
      });
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

  return (
    <section
      className="chat-panel"
      data-testid={chatTestid.panel}
      data-friend-id={friendId}
      role="dialog"
      aria-labelledby={titleId}
      onKeyDown={(event) => {
        if (event.key !== "Escape") return;
        // The board's own Escape (a selection, a prompt) is not this panel's to close.
        event.stopPropagation();
        onClose();
      }}
    >
      <header className="chat-panel__head">
        <h2 className="chat-panel__title" id={titleId}>
          <Named text={t("Chat with {name}")} name={friend.username} />
        </h2>
        <button type="button" className="link-button" data-testid={chatTestid.close} aria-label={t("Close the chat")} onClick={onClose}>
          ✕
        </button>
      </header>

      <ol className="chat-panel__messages" ref={list} aria-live="polite">
        {earlier ? (
          <li className="chat-panel__earlier">
            <button type="button" className="link-button" data-testid={chatTestid.earlier} disabled={loadingEarlier} onClick={loadEarlier}>
              {loadingEarlier ? t("Loading…") : t("Show earlier messages")}
            </button>
          </li>
        ) : null}
        {loaded && messages.length === 0 ? <li className="chat-panel__empty">{t("No messages yet. Say hello!")}</li> : null}
        {messages.map((message) => {
          const yours = message.from !== friendId;
          return (
            <li
              key={message.id}
              className="chat-message"
              data-testid={chatTestid.message}
              data-message-id={message.id}
              data-from={yours ? "you" : "friend"}
            >
              <p className="chat-message__body">{message.body}</p>
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
      </ol>

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
        <button type="submit" className="button-primary" data-testid={chatTestid.send} disabled={sending || draft.trim().length === 0}>
          {t("Send")}
        </button>
      </form>
    </section>
  );
}
