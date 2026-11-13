// R3732002, R3732001: how a friend list is grouped, which button a row carries, what its ⋯ menu holds and
// what its status line says. Pure reads of the server's facts about each friend.

import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { resetLocaleForTests } from "../i18n/i18n.ts";
import type { Friend } from "../net/social.ts";
import {
  FRIEND_GROUP_ORDER,
  agoWords,
  groupFriends,
  groupHeading,
  groupOf,
  menuItemsOf,
  primaryActionOf,
  statusLineOf,
  type Offered,
} from "./friendGroups.ts";

const NOW = Date.parse("2026-11-13T12:00:00.000Z");
const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

function friend(profileId: string, over: Partial<Friend> = {}): Friend {
  return {
    profileId,
    username: profileId,
    status: "online",
    matchId: null,
    watchable: false,
    since: "2026-01-01T00:00:00.000Z",
    ...over,
  };
}

/** A friend in each state the table names. */
const FREE = friend("Cy", { status: "online" });
const LOBBY = friend("Di", { status: "online", lobbyCode: "ABC234" });
const WATCHABLE = friend("Bo", { status: "inMatch", matchId: "m-1", watchable: true, turn: 7 });
const FULL = friend("Gus", { status: "inMatch", matchId: "m-2", watchable: false });
const SERIES = friend("Sid", { status: "inMatch", matchId: null });
const COOP = friend("Eli", { status: "inMatch", matchId: "m-3", watchable: true, duos: true });
const AWAY = friend("Fay", { status: "offline", lastSeenAt: new Date(NOW - 3 * HOUR).toISOString() });
const UNSEEN = friend("Zed", { status: "offline" });

const ALL: Offered = { lobby: true, remove: true, block: true };

describe("R3732002 the friends list's groups, buttons and menus", () => {
  beforeEach(() => {
    resetLocaleForTests();
  });
  afterEach(() => {
    resetLocaleForTests();
  });

  it("R3732002 groups friends in game, online, offline and filters by name", () => {
    const friends = [AWAY, FREE, WATCHABLE, LOBBY, SERIES, UNSEEN];

    const groups = groupFriends(friends, "");

    // Always the three groups, in the order, the input's order kept inside each.
    expect(FRIEND_GROUP_ORDER).toEqual(["inGame", "online", "offline"]);
    expect(groups.map((group) => group.id)).toEqual(["inGame", "online", "offline"]);
    expect(groups.map((group) => group.friends.map((f) => f.profileId))).toEqual([
      ["Bo", "Sid"],
      ["Cy", "Di"],
      ["Fay", "Zed"],
    ]);
    expect(groupOf(COOP)).toBe("inGame");
    // A group with no friend is empty, not left out.
    expect(groupFriends([FREE], "").map((group) => group.friends.length)).toEqual([0, 1, 0]);

    // The search: a trimmed, case-insensitive part of the name, whatever the group.
    expect(groupFriends(friends, "  c ").flatMap((group) => group.friends.map((f) => f.profileId))).toEqual(["Cy"]);
    expect(groupFriends(friends, "I").flatMap((group) => group.friends.map((f) => f.profileId))).toEqual(["Sid", "Di"]);
    expect(groupFriends(friends, "nobody").flatMap((group) => group.friends)).toEqual([]);

    // The headings carry the counts.
    expect(groupHeading("requests", 1)).toBe("Requests (1)");
    expect(groupHeading("inGame", 2)).toBe("In a game (2)");
    expect(groupHeading("online", 3)).toBe("Online (3)");
    expect(groupHeading("offline", 12)).toBe("Offline (12)");
  });

  it("R3732002 each friend state has one primary action", () => {
    expect(primaryActionOf(FREE)).toBe("challenge");
    expect(primaryActionOf(WATCHABLE)).toBe("watch");
    expect(primaryActionOf(COOP)).toBe("watch");
    expect(primaryActionOf(LOBBY)).toBe("join");
    expect(primaryActionOf(AWAY)).toBe("daily");
    expect(primaryActionOf(UNSEEN)).toBe("daily");
    // A game that cannot be watched, and a series between its games, leave a daily challenge.
    expect(primaryActionOf(FULL)).toBe("daily");
    expect(primaryActionOf(SERIES)).toBe("daily");
    // A watchable game with no match to open is no Watch.
    expect(primaryActionOf(friend("X", { status: "inMatch", watchable: true, matchId: null }))).toBe("daily");
    // A lobby only means Join while the friend is online.
    expect(primaryActionOf(friend("Y", { status: "offline", lobbyCode: "ABC234" }))).toBe("daily");
  });

  it("R3732002 the menu holds what the primary leaves", () => {
    // Online and free: Chat, Daily challenge, Invite to lobby, Remove, Block.
    expect(menuItemsOf(FREE, "challenge", ALL)).toEqual(["chat", "daily", "lobby", "remove", "block"]);
    // In a game the player may watch: Watch is the primary, so the menu has no Watch.
    expect(menuItemsOf(WATCHABLE, "watch", ALL)).toEqual(["chat", "daily", "lobby", "remove", "block"]);
    // In a lobby: Join is the primary and Watch is not offered.
    expect(menuItemsOf(LOBBY, "join", ALL)).toEqual(["chat", "daily", "lobby", "remove", "block"]);
    // Offline: Daily is the primary, no Invite to lobby, since an offline friend is sent none.
    expect(menuItemsOf(AWAY, "daily", ALL)).toEqual(["chat", "remove", "block"]);
    // A friend in a game the player may watch whose primary is not Watch has it in the menu.
    expect(menuItemsOf(WATCHABLE, "challenge", ALL)).toEqual(["chat", "daily", "lobby", "watch", "remove", "block"]);
    // A game that cannot be watched: no Watch anywhere.
    expect(menuItemsOf(FULL, "daily", ALL)).toEqual(["chat", "lobby", "remove", "block"]);
    // A screen that offers no Remove, Block or lobby leaves them out.
    expect(menuItemsOf(FREE, "challenge", { lobby: false, remove: false, block: false })).toEqual(["chat", "daily"]);
  });

  it("R3732001 the status line says what a friend is doing", () => {
    expect(statusLineOf(WATCHABLE, NOW)).toBe("In a game · turn 7");
    expect(statusLineOf(FULL, NOW)).toBe("In a game");
    expect(statusLineOf(SERIES, NOW)).toBe("In a series");
    expect(statusLineOf(COOP, NOW)).toBe("In a co-op game");
    expect(statusLineOf(LOBBY, NOW)).toBe("In a lobby");
    expect(statusLineOf(FREE, NOW)).toBe("Online");
    expect(statusLineOf(AWAY, NOW)).toBe("Last seen 3 h ago");
    // No heartbeat on record: no line, so the row shows none.
    expect(statusLineOf(UNSEEN, NOW)).toBe("");
    expect(statusLineOf(friend("W", { status: "offline", lastSeenAt: "not a time" }), NOW)).toBe("");
    // A last-seen time in the future (a clock apart) reads as just now.
    expect(statusLineOf(friend("V", { status: "offline", lastSeenAt: new Date(NOW + HOUR).toISOString() }), NOW)).toBe(
      "Last seen just now",
    );
  });

  it("R3732001 the time since a friend was seen is said in the coarsest whole unit", () => {
    expect(agoWords(0)).toBe("just now");
    expect(agoWords(MINUTE - 1)).toBe("just now");
    expect(agoWords(MINUTE)).toBe("1 min ago");
    expect(agoWords(59 * MINUTE + 59_000)).toBe("59 min ago");
    expect(agoWords(HOUR)).toBe("1 h ago");
    expect(agoWords(23 * HOUR)).toBe("23 h ago");
    expect(agoWords(DAY)).toBe("1 d ago");
    expect(agoWords(40 * DAY)).toBe("40 d ago");
  });
});
