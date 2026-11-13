// `/together` — Friends (#936, SPEC S8, B54–B59; R3732002): five tabs by `?section=`, Friends, Requests,
// Recently played, Blocked and Shared with me (`anchors.ts`). The Friends tab is the drawer's list with room to
// breathe (`FriendList.tsx`); the others are `TogetherPanels.tsx`. Co-op lobbies are made and joined from
// `/play`; `/together?coop=` still opens a lobby (`lobby/LobbyPanel.tsx`'s link).
// Behind the gate like `/play` (B54). Every list is the server's answer (`net/social.ts`), read again
// without a reload: the friends list every `FRIENDS_POLL_MS`, the pace a status changes at, when the
// tab's social socket says it changed (R3214000), and each list at once after each of this page's own
// changes. Nothing here is a rule (CLAUDE.md rule 7): which
// friend can be challenged or watched (B56) is what the server says of them (`status`, `watchable`),
// and every refusal is shown in the server's words.
// R2119000: Remove asks nothing: the friend leaves the list at once and the unfriend waits behind
// "Friend removed · Undo" (`undo/hold.ts`), so Undo sends nothing.

import { useEffect, useRef, useState, type ReactElement } from "react";

import { AccountError } from "../auth/AccountError.tsx";
import { CatalogContext } from "../game/catalog.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { withSession } from "../net/gate.ts";
import { navigate, useSearch } from "../net/navigate.ts";
import { cachedFriends, getFriends, type PlayerRef, type SharedReplay } from "../net/social.ts";
import OpenedReplay from "../replay/OpenedReplay.tsx";
import { openOnlineMatchReplay } from "../replay/sources.ts";
import { BackLink } from "../routes/nav.tsx";
import { Wordmark } from "../routes/Wordmark.tsx";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { Tabs } from "../tabs/Tabs.tsx";
import type { UndoTimer } from "../undo/hold.ts";
import { UndoToast } from "../undo/UndoToast.tsx";
import { FRIENDS_SECTIONS, friendsSectionOf, friendsSectionPath, type FriendsSection } from "./anchors.ts";
import { useCatalogLookup } from "./catalog.ts";
import { ChallengeDialog } from "./ChallengeDialog.tsx";
import { ChatPanel } from "./ChatPanel.tsx";
import { FRIENDS_POLL_MS } from "./config.ts";
import { FriendList } from "./FriendList.tsx";
import { AddFriend, type Challenging } from "./FriendRows.tsx";
import { usePolled } from "./session.ts";
import { noteFriends, onFriendsPush, reloadSocial, useSocialPoll } from "./socialPoll.ts";
import { BlockedTab, RecentTab, RequestsTab, SharedTab } from "./TogetherPanels.tsx";
import { togetherTabTestid, togetherTestid } from "./testids.ts";
import { useFriendActions } from "./useFriendActions.ts";

import "../auth/tavern.css";
import "../routes/lobby.css";
import "./together.css";

/** Each text is its own literal, so the translation guard sees it. */
function sectionLabel(section: FriendsSection): string {
  switch (section) {
    case "friends":
      return t("Friends");
    case "requests":
      return t("Requests");
    case "recent":
      return t("Recently played");
    case "blocked":
      return t("Blocked");
    case "shared":
      return t("Shared with me");
  }
}

/** `undoTimer`: the undo window's timers (R2119000); a test hands a manual one. */
export function TogetherRoute({ undoTimer }: { undoTimer?: UndoTimer } = {}): ReactElement {
  useLocale();
  const section = friendsSectionOf(useSearch(), window.location.hash);
  const [friends, reloadFriends] = usePolled(getFriends, FRIENDS_POLL_MS, cachedFriends);
  const list = friends.kind === "ready" ? friends.value : null;
  const actions = useFriendActions({ friends: list, reloadFriends, undoTimer });
  // The challenges and unread counts are the tab's one read of them (`socialPoll.ts`).
  const social = useSocialPoll();
  const lookup = useCatalogLookup();
  const [chatWith, setChatWith] = useState<PlayerRef | null>(null);
  const [challenging, setChallenging] = useState<Challenging | null>(null);
  const [watching, setWatching] = useState<SharedReplay | null>(null);
  const challengeOpener = useRef<HTMLElement | null>(null);

  // The Friends button's badge counts the requests this page last read.
  useEffect(() => {
    if (friends.kind === "ready") noteFriends(friends.value);
  }, [friends]);

  // R3214000: the server said the friends list changed (a request, a friend made or lost, a block).
  useEffect(() => onFriendsPush(reloadFriends), [reloadFriends]);

  /**
   * The focus goes back to the button that opened the dialog (WAI-ARIA APG; `Game.tsx`'s
   * `refocusConcede`), kept when pressed, not read off the focus, which a click does not move everywhere.
   */
  const closeChallenge = (): void => {
    setChallenging(null);
    const opener = challengeOpener.current;
    challengeOpener.current = null;
    if (opener !== null && opener.isConnected) opener.focus({ preventScroll: true });
  };

  if (watching !== null) {
    return (
      <CatalogContext.Provider value={lookup}>
        <OpenedReplay
          key={watching.matchId}
          load={() => withSession((token) => openOnlineMatchReplay(token, watching.matchId))}
          onLeave={() => {
            setWatching(null);
          }}
        />
      </CatalogContext.Provider>
    );
  }

  const panel = (id: FriendsSection): ReactElement => {
    switch (id) {
      case "friends":
        return (
          <section className="lobby-card together-panel" aria-label={t("Friends")}>
            <AddFriend onAdded={reloadFriends} />
            {friends.kind === "loading" ? <Skeleton variant="rows" label={t("Loading your friends…")} /> : null}
            {friends.kind === "failed" ? (
              <AccountError
                message={friends.message}
                retryAfterMs={friends.retryAfterMs}
                onRetry={reloadFriends}
                testId={togetherTestid.friendsError}
                retryTestId={togetherTestid.friendsRetry}
              />
            ) : null}
            <FriendList
              friends={actions.shown}
              requests={Array.isArray(list?.incoming) ? list.incoming : []}
              unread={social.unread}
              actions={actions}
              challengeRefusal={null}
              headingLevel="h2"
              onChat={setChatWith}
              onChallenge={(friend, opener) => {
                challengeOpener.current = opener;
                setChallenging(friend);
              }}
            />
            {/* R2615000: no switch; the friendship is the way out. */}
            <p className="lobby-note" data-testid={togetherTestid.watchingNote}>
              {t(
                "Your friends can always watch your online games, a few seconds behind. To stop someone watching, remove them as a friend or block them.",
              )}
            </p>
          </section>
        );
      case "requests":
        return <RequestsTab list={list} challenges={social.outgoing} actions={actions} />;
      case "recent":
        return <RecentTab onAdded={reloadFriends} />;
      case "blocked":
        return <BlockedTab list={list} actions={actions} />;
      case "shared":
        return (
          <SharedTab
            onOpen={(replay) => {
              // R2119000: the replay leaves this screen, so a held Remove goes now.
              actions.sendNow();
              setWatching(replay);
            }}
          />
        );
    }
  };

  return (
    <div className="app-shell tavern lobby together-screen one-screen" data-testid={togetherTestid.root} data-section={section}>
      <BackLink title={t("Friends")} />

      <header className="together-hero">
        <Wordmark />
      </header>

      {actions.error === null ? null : (
        <p className="notice" data-testid={togetherTestid.error} role="alert">
          {t(actions.error)}
        </p>
      )}

      <Tabs<FriendsSection>
        label={t("Friends")}
        idPrefix="together"
        className="together-tabs"
        tablistClassName="together-tablist"
        panelsClassName="together-panels"
        value={section}
        onChange={(next) => {
          navigate(friendsSectionPath(next), { replace: true });
        }}
        items={FRIENDS_SECTIONS.map((id) => ({
          id,
          label: sectionLabel(id),
          testId: togetherTabTestid(id),
          panel: panel(id),
        }))}
      />

      {chatWith === null ? null : (
        <ChatPanel
          key={chatWith.profileId}
          friend={chatWith}
          drawer
          onClose={() => {
            setChatWith(null);
            reloadSocial();
          }}
        />
      )}
      {challenging === null ? null : (
        <ChallengeDialog
          friend={challenging}
          liveRefusal={challenging.liveRefusal}
          {...(challenging.kind === undefined ? {} : { initialKind: challenging.kind })}
          onClose={closeChallenge}
          onSent={() => {
            closeChallenge();
            reloadSocial();
          }}
        />
      )}
      <UndoToast held={actions.held} onUndo={actions.undo} />
    </div>
  );
}
