// R4039001: the inbox's Recent section, under what waits on the player: the notification feed the server keeps
// (R4039000), newest first, each row in words, how long ago, and Open, which goes where it happened. A row the
// player had not read when the panel opened carries a dot; opening it marks them read (`feed.ts`). A row the
// socket pushes while the panel is open reads the newest page again. "Show older" reads the page before. A
// failed read keeps the rows shown and says so here, apart from the panel's alert for a refused answer.
// CLAUDE.md rule 7: the rows are the server's.

import { useEffect, type ReactElement } from "react";

import { t, useLocale } from "../i18n/i18n.ts";
import { navigate } from "../net/navigate.ts";
import { agoWords } from "../together/friendGroups.ts";
import { onNotificationPush } from "../together/socialPoll.ts";
import { Named } from "../together/words.tsx";
import { feedName, feedPath, feedSentence, loadOlderFeed, openFeed, useFeed } from "./feed.ts";
import { inboxTestid } from "./testids.ts";

export type RecentSectionProps = {
  /** Epoch ms the panel's clock reads, for each row's time ago. */
  now: number;
  /** Closes the panel, as a row's Open goes. */
  onClose: () => void;
};

export function RecentSection({ now, onClose }: RecentSectionProps): ReactElement {
  useLocale();
  const feed = useFeed();

  // Opening the panel reads the newest page, and so does each row pushed while it is open.
  useEffect(() => {
    void openFeed();
    return onNotificationPush(() => {
      void openFeed();
    });
  }, []);

  return (
    <div className="inbox-recent" data-testid={inboxTestid.recent}>
      {feed.error === null ? null : (
        <p className="notice" data-testid={inboxTestid.recentError} role="alert">
          {t(feed.error)}
        </p>
      )}
      {feed.loaded && feed.rows.length === 0 ? (
        <p className="inbox-recent__empty" data-testid={inboxTestid.recentEmpty}>
          {t("Nothing yet.")}
        </p>
      ) : null}
      {feed.rows.length === 0 ? null : (
        <ul className="together-list">
          {feed.rows.map((row) => {
            const path = feedPath(row);
            return (
              <li key={row.id} className="together-row inbox-row" data-testid={inboxTestid.recentRow} data-kind={row.kind} data-unread={row.read ? "false" : "true"}>
                {row.read ? null : <span className="inbox-recent__dot" data-testid={inboxTestid.recentDot} role="img" aria-label={t("New")} />}
                <span className="inbox-row__text">
                  <span>
                    <Named text={feedSentence(row)} name={feedName(row)} />
                  </span>
                  <span className="inbox-row__meta">{agoWords(Math.max(0, now - row.createdAt))}</span>
                </span>
                {path === null ? null : (
                  <span className="together-row__actions">
                    <button
                      type="button"
                      data-testid={inboxTestid.recentOpen}
                      onClick={() => {
                        // A guarded page may hold the move; the panel closes as the player goes.
                        if (navigate(path)) onClose();
                      }}
                    >
                      {t("Open")}
                    </button>
                  </span>
                )}
              </li>
            );
          })}
        </ul>
      )}
      {feed.nextBefore === null ? null : (
        <button
          type="button"
          className="link-button inbox-recent__older"
          data-testid={inboxTestid.older}
          onClick={() => {
            void loadOlderFeed();
          }}
        >
          {t("Show older")}
        </button>
      )}
    </div>
  );
}
