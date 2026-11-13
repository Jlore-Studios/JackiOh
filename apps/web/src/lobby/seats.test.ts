import { describe, expect, it } from "vitest";

import type { LobbySeat, LobbyView } from "./api.ts";
import { canStart, deckWords, moveTargetOf, otherTeamHasRoom, seatsOf, type SeatShown } from "./seats.ts";

function seat(over: Partial<LobbySeat> = {}): LobbySeat {
  return {
    role: "host",
    player: { profileId: "p-ana", username: "Ana#1", status: "online" },
    state: "choosing",
    deck: null,
    random: false,
    ...over,
  };
}

function view(over: Partial<LobbyView> = {}): LobbyView {
  return {
    kind: "lobby",
    code: "K7Q2XM",
    settings: { format: "duel", mode: "bo1" },
    expiresAt: 0,
    you: "host",
    yourPick: null,
    seats: [seat(), seat({ role: "guest", player: null })],
    startedAt: null,
    matchId: null,
    seriesId: null,
    ...over,
  };
}

describe("the lobby's seats", () => {
  it("R3730000 a duel has two seats, and its empty one is a member's to invite into and a visitor's to take", () => {
    const [team, ...rest] = seatsOf(view());
    expect(rest).toEqual([]);
    expect(team?.map((shown) => shown.kind)).toEqual(["player", "invite"]);
    expect(team?.[0]).toMatchObject({ kind: "player", host: true, you: true });
    const visitor = seatsOf(view({ you: null }))[0];
    expect(visitor?.map((shown) => shown.kind)).toEqual(["player", "open"]);
  });

  it("R3730000 a co-op lobby has four seats in two teams, the second the AI's", () => {
    const teams = seatsOf(view({ settings: { format: "coop", difficulty: "easy" }, you: "guest", seats: [seat(), seat({ role: "guest" })] }));
    expect(teams.map((team) => team.map((shown) => shown.kind))).toEqual([
      ["player", "player"],
      ["ai", "ai"],
    ]);
    expect(teams[0]?.[1]).toMatchObject({ kind: "player", host: false, you: true });
  });

  it("R3731000 a 2v2 lobby's seats are the host's team then the other team", () => {
    const bo = { profileId: "p-bo", username: "Bo#2", status: "online" as const };
    const kai = { profileId: "p-kai", username: "Kai#3", status: "online" as const };
    const dee = { profileId: "p-dee", username: "Dee#4", status: "online" as const };
    const twoVTwo = (seats: LobbySeat[], you: LobbyView["you"] = "guest2"): LobbyView => view({ settings: { format: "2v2" }, you, seats });
    const roleOf = (shown: SeatShown): string => (shown.kind === "player" ? shown.seat.role : shown.kind === "ai" ? "ai" : shown.role);
    const open = twoVTwo([seat(), seat({ role: "guest", player: null }), seat({ role: "guest2", player: bo }), seat({ role: "guest3", player: kai })]);

    // Two teams of people, no AI seat: the host's (host, guest) and the other (guest2, guest3).
    const teams = seatsOf(open);
    expect(teams.map((team) => team.map(roleOf))).toEqual([
      ["host", "guest"],
      ["guest2", "guest3"],
    ]);
    expect(teams.map((team) => team.map((shown) => shown.kind))).toEqual([
      ["player", "invite"],
      ["player", "player"],
    ]);
    expect(teams[0]?.[0]).toMatchObject({ kind: "player", host: true, you: false });
    expect(teams[1]?.[0]).toMatchObject({ kind: "player", host: false, you: true });
    expect(seatsOf(twoVTwo(open.seats, null))[0]?.[1]).toEqual({ kind: "open", role: "guest" });

    // Switch team lights only while the other team has an empty seat.
    expect(otherTeamHasRoom(open, "guest2")).toBe(true);
    expect(otherTeamHasRoom(open, "host")).toBe(false);

    // The host's Move: to the other team's first empty seat but the host's, else its first seat but the host's.
    expect(moveTargetOf(open, "guest2")).toBe("guest");
    expect(moveTargetOf(open, "host")).toBeNull();
    const full = twoVTwo([seat(), seat({ role: "guest", player: bo }), seat({ role: "guest2", player: kai }), seat({ role: "guest3", player: dee })]);
    expect(moveTargetOf(full, "guest")).toBe("guest2");
    expect(moveTargetOf(full, "guest3")).toBe("guest");
    const roomAtThree = twoVTwo([seat(), seat({ role: "guest", player: bo }), seat({ role: "guest2", player: kai }), seat({ role: "guest3", player: null })]);
    expect(moveTargetOf(roomAtThree, "guest")).toBe("guest3");

    // The Start needs all four seats filled and ready.
    const ready = full.seats.map((each) => ({ ...each, state: "ready" as const }));
    expect(canStart({ ...full, you: "host", seats: ready })).toBe(true);
    expect(canStart({ ...full, you: "host", seats: ready.map((each, index) => (index === 3 ? { ...each, state: "chosen" as const } : each)) })).toBe(false);
    expect(canStart({ ...open, you: "host", seats: open.seats.map((each) => ({ ...each, state: "ready" as const })) })).toBe(false);
  });

  it("R3730001 each deck state reads as its words, a hidden or random deck by its kind", () => {
    expect(deckWords(seat())).toBe("Choosing a deck…");
    expect(deckWords(seat({ state: "chosen", deck: "Pyre Control" }))).toBe("Pyre Control");
    expect(deckWords(seat({ state: "chosen" }))).toBe("Deck chosen");
    expect(deckWords(seat({ state: "chosen", random: true }))).toBe("Random deck");
    expect(deckWords(seat({ state: "ready", deck: "Pyre Control" }))).toBe("✓ Ready");
  });

  it("R3730002 the host's Start lights only once every seat is filled and ready", () => {
    const ready = seat({ state: "ready" });
    expect(canStart(view({ seats: [ready, seat({ role: "guest", player: null, state: "choosing" })] }))).toBe(false);
    expect(canStart(view({ seats: [ready, seat({ role: "guest", state: "chosen" })] }))).toBe(false);
    expect(canStart(view({ seats: [ready, seat({ role: "guest", state: "ready" })] }))).toBe(true);
    expect(canStart(view({ you: "guest", seats: [ready, seat({ role: "guest", state: "ready" })] }))).toBe(false);
    expect(canStart(view({ startedAt: 1, seats: [ready, seat({ role: "guest", state: "ready" })] }))).toBe(false);
  });
});
