// The friends as the drawer and the Friends page's first tab draw them (R3732002): a search, the requests
// waiting, and the friends in three groups, In a game, Online and Offline (collapsed), each heading with its
// count and each row with one primary button and a ⋯ menu (`FriendRows.tsx`). The groups and the buttons are
// `friendGroups.ts`'s reads of the server's facts; nothing here is a rule (CLAUDE.md rule 7).

import { useId, useState, type ReactElement } from "react";

import { EmptyState } from "../empty/EmptyState.tsx";
import { t, useLocale } from "../i18n/i18n.ts";
import { acceptFriendRequest, declineFriendRequest, type Friend, type FriendRequest, type PlayerRef } from "../net/social.ts";
import { FriendRow, RequestRow, liveRefusalOf, type Challenging } from "./FriendRows.tsx";
import { FRIEND_GROUP_ORDER, groupFriends, groupHeading, type FriendGroupId } from "./friendGroups.ts";
import { togetherTestid } from "./testids.ts";
import type { FriendActions } from "./useFriendActions.ts";

export type FriendListProps = {
  /** The friends to list (`FriendActions.shown`); null while the list is unread, when the requests alone show. */
  friends: readonly Friend[] | null;
  /** The incoming requests, Accept and Decline each. */
  requests: readonly FriendRequest[];
  unread: Readonly<Record<string, number>>;
  actions: FriendActions;
  /** Why the player cannot play anyone live now (the server's sentence), or null. */
  challengeRefusal: string | null;
  onChat: (friend: PlayerRef) => void;
  /** `opener` has the focus back once the dialog closes. */
  onChallenge: (friend: Challenging, opener: HTMLElement) => void;
  /** Runs before Watch, Join or Invite to lobby takes the player away (the drawer closes). */
  onLeave?: () => void;
  /** The groups' heading level: under the drawer's title, h3; under the page's, h2 (no level is skipped). */
  headingLevel?: "h2" | "h3";
};

export function FriendList({
  friends,
  requests,
  unread,
  actions,
  challengeRefusal,
  onChat,
  onChallenge,
  onLeave,
  headingLevel = "h3",
}: FriendListProps): ReactElement {
  useLocale();
  const [query, setQuery] = useState("");
  const [offlineOpen, setOfflineOpen] = useState(false);
  const offlineId = useId();
  const searching = query.trim() !== "";
  const groups = friends === null ? [] : groupFriends(friends, query);
  const now = Date.now();
  const { busy } = actions;
  const Heading = headingLevel;

  const row = (friend: Friend): ReactElement => (
    <FriendRow
      key={friend.profileId}
      friend={friend}
      unread={unread[friend.profileId] ?? 0}
      busy={busy}
      now={now}
      challengeRefusal={challengeRefusal}
      onChallenge={(opener, kind) => {
        onChallenge({ profileId: friend.profileId, username: friend.username, liveRefusal: liveRefusalOf(friend, challengeRefusal), kind }, opener);
      }}
      onChat={() => {
        onChat({ profileId: friend.profileId, username: friend.username });
      }}
      onLeave={onLeave}
      onLobby={() => {
        actions.inviteToLobby(friend, onLeave);
      }}
      onRemove={() => {
        actions.remove(friend);
      }}
      onBlock={() => {
        actions.block(friend);
      }}
    />
  );

  const shownCount = groups.reduce((sum, group) => sum + group.friends.length, 0);

  return (
    <div className="friend-list">
      {requests.length === 0 ? null : (
        <section className="together-group" data-testid={togetherTestid.group} data-group="requests">
          <Heading className="together-group__heading">{groupHeading("requests", requests.length)}</Heading>
          <ul className="together-list">
            {requests.map((request) => (
              <RequestRow
                key={request.requestId}
                request={request}
                direction="incoming"
                busy={busy}
                onAccept={() => {
                  actions.act(() => acceptFriendRequest(request.requestId));
                }}
                onDecline={() => {
                  actions.act(() => declineFriendRequest(request.requestId));
                }}
              />
            ))}
          </ul>
        </section>
      )}

      {friends !== null && friends.length === 0 ? (
        <EmptyState
          text={t("No friends yet. Ask a friend for their username, with its #tag if it has one, and add them above.")}
          testId={togetherTestid.noFriends}
        />
      ) : null}

      {friends !== null && friends.length > 0 ? (
        <>
          <input
            type="search"
            className="together-search"
            data-testid={togetherTestid.search}
            value={query}
            autoComplete="off"
            spellCheck={false}
            aria-label={t("Search friends")}
            placeholder={t("Search friends…")}
            onChange={(event) => {
              setQuery(event.target.value);
            }}
          />
          {searching && shownCount === 0 ? <p className="lobby-note">{t("No friend has a name like that.")}</p> : null}
          {FRIEND_GROUP_ORDER.map((id: FriendGroupId) => {
            const group = groups.find((candidate) => candidate.id === id);
            if (group === undefined || group.friends.length === 0) return null;
            if (id !== "offline") {
              return (
                <section key={id} className="together-group" data-testid={togetherTestid.group} data-group={id}>
                  <Heading className="together-group__heading">{groupHeading(id, group.friends.length)}</Heading>
                  <ul className="together-list">{group.friends.map(row)}</ul>
                </section>
              );
            }
            // Offline is collapsed until opened, and open while a search is typed: a match must show.
            const open = offlineOpen || searching;
            return (
              <section key={id} className="together-group" data-testid={togetherTestid.group} data-group={id}>
                <Heading className="together-group__heading">
                  <button
                    type="button"
                    className="together-group__toggle"
                    data-testid={togetherTestid.offlineToggle}
                    aria-expanded={open}
                    aria-controls={offlineId}
                    onClick={() => {
                      setOfflineOpen(!open);
                    }}
                  >
                    <span aria-hidden="true">{open ? "▾" : "▸"}</span> {groupHeading(id, group.friends.length)}
                  </button>
                </Heading>
                {open ? (
                  <ul className="together-list" id={offlineId}>
                    {group.friends.map(row)}
                  </ul>
                ) : null}
              </section>
            );
          })}
        </>
      ) : null}
    </div>
  );
}
