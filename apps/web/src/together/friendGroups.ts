// How the friends drawer and the Friends page read a friend list (R3732002): which group a friend sits in,
// the one primary button on their row, the items of their ⋯ menu and the words of their status line.
// All of it is read off the server's facts about the friend (`status`, `watchable`, `matchId`, `lobbyCode`,
// `turn`, `lastSeenAt`, R3732001) and decides no rule (CLAUDE.md rule 7): the server refuses every Join,
// Challenge and Watch that is not allowed.

import { t } from "../i18n/i18n.ts";
import type { Friend } from "../net/social.ts";

/** The groups the friends are listed in, in this order; the requests come before them (`FriendList.tsx`). */
export const FRIEND_GROUP_ORDER = ["inGame", "online", "offline"] as const;
export type FriendGroupId = (typeof FRIEND_GROUP_ORDER)[number];

export type FriendGroup = { id: FriendGroupId; friends: Friend[] };

/** The group a friend sits in: a friend in a game, a series or a co-op game is "In a game". */
export function groupOf(friend: Friend): FriendGroupId {
  return friend.status === "inMatch" ? "inGame" : friend.status;
}

/**
 * The friends in their three groups, always all three in `FRIEND_GROUP_ORDER` (a group with no friend is
 * empty), the server's order kept within each. `query` keeps the friends whose name holds it, ignoring
 * case and the spaces around it.
 */
export function groupFriends(friends: readonly Friend[], query: string): FriendGroup[] {
  const wanted = query.trim().toLowerCase();
  const kept = wanted === "" ? friends : friends.filter((friend) => friend.username.toLowerCase().includes(wanted));
  return FRIEND_GROUP_ORDER.map((id) => ({ id, friends: kept.filter((friend) => groupOf(friend) === id) }));
}

/** A group's heading with its count, as the drawer and the page draw it. */
export function groupHeading(id: FriendGroupId | "requests", count: number): string {
  switch (id) {
    case "requests":
      return t("Requests ({count})", { count });
    case "inGame":
      return t("In a game ({count})", { count });
    case "online":
      return t("Online ({count})", { count });
    case "offline":
      return t("Offline ({count})", { count });
  }
}

/** The one button a friend's row carries. */
export type PrimaryAction = "watch" | "join" | "challenge" | "daily";

/**
 * Watch for a friend in a game the player may watch; Join for an online friend with a lobby open; Challenge
 * for any other online friend; Daily, a daily challenge, for everyone else (an offline friend, and a friend
 * in a game that cannot be watched).
 */
export function primaryActionOf(friend: Friend): PrimaryAction {
  if (friend.status === "inMatch" && friend.watchable && friend.matchId !== null) return "watch";
  if (friend.status === "online" && friend.lobbyCode !== undefined) return "join";
  if (friend.status === "online") return "challenge";
  return "daily";
}

/** An item of a row's ⋯ menu. */
export type MenuItem = "chat" | "daily" | "lobby" | "watch" | "remove" | "block";

/** What the screen offers of the actions only some screens do: Invite to lobby, Remove and Block. */
export type Offered = { lobby: boolean; remove: boolean; block: boolean };

/**
 * What the primary button leaves, in this order: Chat; Daily challenge (unless Daily is the primary);
 * Invite to lobby (offered, and not for an offline friend); Watch (a game the player may watch, unless Watch
 * is the primary); Remove; Block.
 */
export function menuItemsOf(friend: Friend, primary: PrimaryAction, offered: Offered): MenuItem[] {
  const items: MenuItem[] = ["chat"];
  if (primary !== "daily") items.push("daily");
  if (offered.lobby && friend.status !== "offline") items.push("lobby");
  if (primary !== "watch" && friend.watchable && friend.matchId !== null) items.push("watch");
  if (offered.remove) items.push("remove");
  if (offered.block) items.push("block");
  return items;
}

const MINUTE_MS = 60_000;
const HOUR_MS = 60 * MINUTE_MS;
const DAY_MS = 24 * HOUR_MS;

/** How long ago `ms` milliseconds is, in the coarsest whole unit: "just now", "5 min ago", "3 h ago", "2 d ago". */
export function agoWords(ms: number): string {
  if (ms < MINUTE_MS) return t("just now");
  if (ms < HOUR_MS) return t("{count} min ago", { count: Math.floor(ms / MINUTE_MS) });
  if (ms < DAY_MS) return t("{count} h ago", { count: Math.floor(ms / HOUR_MS) });
  return t("{count} d ago", { count: Math.floor(ms / DAY_MS) });
}

/**
 * What the friend is doing, in words (R3732001): "In a game · turn 7", "In a game", "In a series", "In a
 * co-op game", "In a lobby", "Online", "Last seen 3 h ago"; nothing for an offline friend with no heartbeat on
 * record. `now` is epoch milliseconds.
 */
export function statusLineOf(friend: Friend, now: number): string {
  if (friend.status === "inMatch") {
    if (friend.duos === true) return t("In a co-op game");
    if (friend.turn !== undefined) return t("In a game · turn {turn}", { turn: friend.turn });
    return friend.matchId === null ? t("In a series") : t("In a game");
  }
  if (friend.status === "online") return friend.lobbyCode === undefined ? t("Online") : t("In a lobby");
  if (friend.lastSeenAt === undefined) return "";
  const seen = Date.parse(friend.lastSeenAt);
  return Number.isFinite(seen) ? t("Last seen {when}", { when: agoWords(Math.max(0, now - seen)) }) : "";
}
