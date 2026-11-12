import { describe, expect, it } from "vitest";

import type { LobbySeat, LobbyView } from "./api.ts";
import { canStart, deckWords, seatsOf } from "./seats.ts";

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
