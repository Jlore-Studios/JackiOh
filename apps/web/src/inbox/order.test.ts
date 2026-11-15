// R3729002: what the inbox lists and counts, and in what order, worked out from the lists the client reads and
// a time. Pure: no server and no screen.

import { describe, expect, it } from "vitest";

import type { DailyGameSummary, DailyInvite, DailyList } from "../net/daily.ts";
import type { Challenge, FriendRequest, SharedReplay } from "../net/social.ts";
import { INBOX_INVITE_KEEP_MS } from "./config.ts";
import type { InboxInvite } from "./invites.ts";
import { INBOX_KINDS, challengeLeftText, inboxBadge, inboxItems, replayKey, type InboxInput } from "./order.ts";

const NOW = Date.parse("2026-11-13T12:00:00.000Z");
const ME = { profileId: "me", username: "Me#1" };
const ANN = { profileId: "p-ann", username: "Ann#2" };
const BOB = { profileId: "p-bob", username: "Bob#3" };

function at(offsetMs: number): string {
  return new Date(NOW + offsetMs).toISOString();
}

function challenge(challengeId: string, expiresIn: number): Challenge {
  return { challengeId, from: ANN, to: ME, roomCode: `ROOM-${challengeId}`, mode: "bo1", expiresAt: at(expiresIn) };
}

function lobbyInvite(code: string, receivedAgo: number): InboxInvite {
  return { code, format: "duel", from: ANN, receivedAt: NOW - receivedAgo, toastHidden: false };
}

function game(matchId: string, over: Partial<DailyGameSummary> = {}): DailyGameSummary {
  return {
    matchId,
    mode: "bo1",
    perMoveDays: 3,
    seat: "p1",
    opponent: BOB,
    portraits: { me: "vanilla", them: "gary" },
    turn: 4,
    life: { me: 30, them: 22 },
    hasMove: true,
    theirMove: false,
    deadline: NOW + 1000,
    over: false,
    result: null,
    endedAt: null,
    createdAt: NOW - 1000,
    ...over,
  };
}

function dailyInvite(id: string, over: Partial<DailyInvite> = {}): DailyInvite {
  return { id, kind: "challenge", from: ANN, to: ME, mode: "random", perMoveDays: 1, expiresAt: NOW + 5000, createdAt: NOW - 1000, ...over };
}

function request(requestId: string, createdAgo: number): FriendRequest {
  return { requestId, from: BOB, to: ME, createdAt: at(-createdAgo) };
}

function replay(matchId: string, createdAgo: number, sharer = ANN): SharedReplay {
  return { matchId, sharer, createdAt: at(-createdAgo), endedAt: at(-createdAgo - 60_000) };
}

function list(over: Partial<DailyList> = {}): DailyList {
  return { games: [], challenges: { incoming: [], outgoing: [] }, seeks: [], waiting: 0, ...over };
}

const NOTHING: InboxInput = { challenges: [], invites: [], daily: null, requests: [], shared: [] };

describe("R3729002 the inbox's order", () => {
  it("R3729002 the kinds come as challenge, invite, daily move, daily challenge, request, replay", () => {
    expect(INBOX_KINDS).toEqual(["challenge", "invite", "dailyMove", "dailyChallenge", "request", "replay"]);
    const items = inboxItems(
      {
        shared: [replay("m-shared", 1000)],
        requests: [request("rq-1", 1000)],
        daily: list({ games: [game("g-1")], challenges: { incoming: [dailyInvite("dc-1")], outgoing: [] } }),
        invites: [lobbyInvite("LOBBY1", 1000)],
        challenges: [challenge("ch-1", 60_000)],
      },
      NOW,
    );
    expect(items.map((item) => item.kind)).toEqual(["challenge", "invite", "dailyMove", "dailyChallenge", "request", "replay"]);
    expect(items.map((item) => item.key)).toEqual([
      "challenge:ch-1",
      "invite:LOBBY1",
      "dailyMove:g-1",
      "dailyChallenge:dc-1",
      "request:rq-1",
      `replay:m-shared:${ANN.profileId}`,
    ]);
    expect(inboxItems(NOTHING, NOW)).toEqual([]);
  });

  it("R3729002 within a kind: challenges soonest to lapse first, invites newest first, requests oldest first, replays newest first", () => {
    const keys = (input: Partial<InboxInput>): string[] => inboxItems({ ...NOTHING, ...input }, NOW).map((item) => item.key);

    expect(keys({ challenges: [challenge("late", 90_000), challenge("soon", 10_000), challenge("mid", 40_000)] })).toEqual([
      "challenge:soon",
      "challenge:mid",
      "challenge:late",
    ]);
    expect(keys({ invites: [lobbyInvite("OLD", 5000), lobbyInvite("NEW", 100), lobbyInvite("MID", 2000)] })).toEqual([
      "invite:NEW",
      "invite:MID",
      "invite:OLD",
    ]);
    expect(keys({ requests: [request("newer", 100), request("oldest", 9000), request("older", 5000)] })).toEqual([
      "request:oldest",
      "request:older",
      "request:newer",
    ]);
    expect(keys({ shared: [replay("old", 9000), replay("new", 100), replay("mid", 5000)] })).toEqual([
      `replay:new:${ANN.profileId}`,
      `replay:mid:${ANN.profileId}`,
      `replay:old:${ANN.profileId}`,
    ]);
  });

  it("R3729002 the daily kinds follow the Daily screen's own order: least time left first, challenges oldest first", () => {
    const items = inboxItems(
      {
        ...NOTHING,
        daily: list({
          games: [
            game("late", { deadline: NOW + 9000 }),
            game("soon", { deadline: NOW + 2000 }),
            game("theirs", { hasMove: false, theirMove: true }),
            game("over", { over: true, deadline: null, endedAt: NOW - 5 }),
          ],
          challenges: {
            incoming: [dailyInvite("newer", { createdAt: NOW - 10 }), dailyInvite("older", { createdAt: NOW - 20 })],
            outgoing: [dailyInvite("sent")],
          },
        }),
      },
      NOW,
    );
    // Only the games on the player's move and the challenges to them: their move, finished games and challenges they sent wait on nobody here.
    expect(items.map((item) => item.key)).toEqual(["dailyMove:soon", "dailyMove:late", "dailyChallenge:older", "dailyChallenge:newer"]);
  });

  it("R3729002 a lapsed challenge, an invite older than its room lives and a daily challenge past its time are dropped", () => {
    const items = inboxItems(
      {
        ...NOTHING,
        challenges: [challenge("lapsed", -1), challenge("now", 0), challenge("open", 1)],
        invites: [lobbyInvite("STALE", INBOX_INVITE_KEEP_MS), lobbyInvite("KEPT", INBOX_INVITE_KEEP_MS - 1)],
        daily: list({ challenges: { incoming: [dailyInvite("gone", { expiresAt: NOW }), dailyInvite("live", { expiresAt: NOW + 1 })], outgoing: [] } }),
      },
      NOW,
    );
    expect(items.map((item) => item.key)).toEqual(["challenge:open", "invite:KEPT", "dailyChallenge:live"]);
    // A clock later, the one that was open has lapsed too.
    expect(inboxItems({ ...NOTHING, challenges: [challenge("open", 1)] }, NOW + 1)).toEqual([]);
  });

  it("R3729002 a challenge whose time does not read is kept, after every one that has a time, and has no clock", () => {
    const odd: Challenge = { ...challenge("odd", 0), expiresAt: "soon" };
    const items = inboxItems({ ...NOTHING, challenges: [odd, challenge("real", 5000)] }, NOW);
    expect(items.map((item) => item.key)).toEqual(["challenge:real", "challenge:odd"]);
    const last = items[1];
    expect(last?.kind === "challenge" && Number.isFinite(last.expiresAt)).toBe(false);
  });

  it("R3729002 a list that is not read yet, or an odd answer, is no item and no failure", () => {
    expect(inboxItems({ ...NOTHING, daily: null }, NOW)).toEqual([]);
    // A server's older answer without the lists a newer one has.
    const odd = { games: "no", challenges: { incoming: 7 } } as unknown as DailyList;
    expect(inboxItems({ ...NOTHING, daily: odd }, NOW)).toEqual([]);
  });

  it("R3729002 two friends sharing the same match are two replays, each its own key", () => {
    const items = inboxItems({ ...NOTHING, shared: [replay("m-1", 1000, ANN), replay("m-1", 2000, BOB)] }, NOW);
    expect(items.map((item) => item.key)).toEqual([replayKey(replay("m-1", 0, ANN)), replayKey(replay("m-1", 0, BOB))]);
    expect(new Set(items.map((item) => item.key)).size).toBe(2);
  });
});

describe("R3729002 the inbox's badge", () => {
  const everything: InboxInput = {
    challenges: [challenge("ch-1", 60_000)],
    invites: [lobbyInvite("LOBBY1", 1000)],
    daily: list({ games: [game("g-1")], challenges: { incoming: [dailyInvite("dc-1")], outgoing: [] } }),
    requests: [request("rq-1", 1000), request("rq-2", 2000)],
    shared: [replay("m-1", 1000), replay("m-2", 2000)],
  };

  it("R3729002 it counts every item, a shared replay included until it is seen", () => {
    const items = inboxItems(everything, NOW);
    expect(items).toHaveLength(8);
    expect(inboxBadge(items, new Set())).toBe(8);
  });

  it("R3729002 a seen shared replay is not counted, and nothing else is ever left out by being seen", () => {
    const items = inboxItems(everything, NOW);
    const seen = new Set([replayKey(replay("m-1", 0)), "request:rq-1", "challenge:ch-1"]);
    expect(inboxBadge(items, seen)).toBe(7);
    expect(inboxBadge(items, new Set(items.filter((item) => item.kind === "replay").map((item) => item.key)))).toBe(6);
    expect(inboxBadge([], new Set())).toBe(0);
  });

  it("R3729002 a lapsed challenge leaves the badge with the list", () => {
    expect(inboxBadge(inboxItems({ ...NOTHING, challenges: [challenge("ch-1", 1000)] }, NOW), new Set())).toBe(1);
    expect(inboxBadge(inboxItems({ ...NOTHING, challenges: [challenge("ch-1", 1000)] }, NOW + 1000), new Set())).toBe(0);
  });
});

describe("R3729002 a challenge's time left", () => {
  it("R3729002 reads as minutes and two-digit seconds, rounded up so it reads zero only once lapsed", () => {
    expect(challengeLeftText(120_000)).toBe("2:00 left");
    expect(challengeLeftText(119_001)).toBe("2:00 left");
    expect(challengeLeftText(119_000)).toBe("1:59 left");
    expect(challengeLeftText(65_000)).toBe("1:05 left");
    expect(challengeLeftText(1)).toBe("0:01 left");
    expect(challengeLeftText(0)).toBe("0:00 left");
    expect(challengeLeftText(-5000)).toBe("0:00 left");
  });
});
