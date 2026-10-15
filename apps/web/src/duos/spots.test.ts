// #936 B64's ping places and S6's socket frames, as pure functions: a board's elements read back as
// spots, a spot as S6's `PingTarget` in the duel that board shows (and back again), the shape check a
// relayed ping passes, and where the co-op socket lives. No engine and no socket.

import { describe, expect, it } from "vitest";

import { baseView, card, emptySide } from "../test/fixtures.ts";
import { parsePingTarget } from "./pings.ts";
import { duosSocketUrl, parseDuosFrame } from "./socket.ts";
import { spotOfElement, spotOfTarget, spotTestid, targetOfSpot } from "./spots.ts";

/** The element `selector` finds in `html`, parsed into a detached container. */
function element(html: string, selector: string): Element {
  const host = document.createElement("div");
  host.innerHTML = html;
  const found = host.querySelector(selector);
  if (found === null) throw new Error(`no ${selector} in ${html}`);
  return found;
}

describe("#936 B64 where a ping points", () => {
  it("B64 reads a hand card, a hero and a zone (from a card inside it) off the board's testids, and nothing else", () => {
    expect(spotOfElement(element('<div data-testid="hand-card-c7"><span class="x"></span></div>', ".x"))).toEqual({
      at: "hand",
      instanceId: "c7",
    });
    expect(spotOfElement(element('<div data-testid="hero-opponent"><b class="x"></b></div>', ".x"))).toEqual({
      at: "hero",
      side: "opponent",
    });
    expect(
      spotOfElement(
        element(
          '<div data-testid="zone-you-backrow-3" data-side="you" data-row="backrow" data-lane="3"><div data-testid="card-u1" class="x"></div></div>',
          ".x",
        ),
      ),
    ).toEqual({ at: "zone", side: "you", row: "backrow", lane: 3 });
    expect(spotOfElement(element('<button data-testid="end-turn" class="x"></button>', ".x"))).toBeNull();
    expect(spotOfElement(element('<div data-testid="revealed-hand-card-c1" class="x"></div>', ".x"))).toBeNull();
  });

  it("B64 a spot is S6's target in the board's duel, lanes counted from 0, and the target finds the same spot", () => {
    const hand = [card({ instanceId: "c1" }), card({ instanceId: "c2" })];
    const view = baseView({ viewer: "p2", active: "p2", you: emptySide("p2", { hand }), opponent: emptySide("p1", { hand: { count: 3 } }) });
    const cases = [
      [{ at: "hand", instanceId: "c2" }, { duel: 1, at: "hand", side: "p2", index: 1 }],
      [{ at: "hero", side: "opponent" }, { duel: 1, at: "hero", side: "p1" }],
      [{ at: "zone", side: "you", row: "units", lane: 5 }, { duel: 1, at: "unit", side: "p2", lane: 4 }],
      [{ at: "zone", side: "opponent", row: "backrow", lane: 1 }, { duel: 1, at: "backrow", side: "p1", lane: 0 }],
    ] as const;
    for (const [spot, target] of cases) {
      expect(targetOfSpot(spot, view, 1)).toEqual(target);
      expect(spotOfTarget(target, view, 1)).toEqual(spot);
      // The other duel's board has no such place: the portal flashes instead.
      expect(spotOfTarget(target, view, 0)).toBeNull();
    }
    expect(spotTestid({ at: "zone", side: "you", row: "units", lane: 5 })).toBe("zone-you-units-5");
    // A hand card the view no longer holds points nowhere, and a hand index past the hand finds nothing.
    expect(targetOfSpot({ at: "hand", instanceId: "gone" }, view, 1)).toBeNull();
    expect(spotOfTarget({ duel: 1, at: "hand", side: "p2", index: 2 }, view, 1)).toBeNull();
    // Only the team's own hand is held as cards: an enemy hand target names nothing on the board.
    expect(spotOfTarget({ duel: 1, at: "hand", side: "p1", index: 0 }, view, 1)).toBeNull();
  });

  it("B39 a relayed target passes the shape check only in one of S6's three shapes", () => {
    expect(parsePingTarget({ duel: 0, at: "hero", side: "p1" })).toEqual({ duel: 0, at: "hero", side: "p1" });
    expect(parsePingTarget({ duel: 1, at: "unit", side: "p2", lane: 4 })).toEqual({ duel: 1, at: "unit", side: "p2", lane: 4 });
    expect(parsePingTarget({ duel: 1, at: "hand", side: "p2", index: 0 })).toEqual({ duel: 1, at: "hand", side: "p2", index: 0 });
    expect(parsePingTarget({ duel: 1, at: "unit", side: "p2", lane: 5 })).toBeNull();
    expect(parsePingTarget({ duel: 2, at: "hero", side: "p1" })).toBeNull();
    expect(parsePingTarget({ duel: 0, at: "hand", side: "p1", index: -1 })).toBeNull();
    expect(parsePingTarget({ duel: 0, at: "graveyard", side: "p1" })).toBeNull();
    expect(parsePingTarget("hero")).toBeNull();
  });
});

describe("#936 S6 the co-op socket", () => {
  it("S6 reads a relayed ping, a clock and the teammate's link, and drops anything else", () => {
    expect(
      parseDuosFrame(JSON.stringify({ type: "ping", from: "a2", ping: "careful", target: { duel: 0, at: "hero", side: "p1" } })),
    ).toEqual({ type: "ping", from: "a2", ping: "careful", target: { duel: 0, at: "hero", side: "p1" } });
    expect(
      parseDuosFrame(JSON.stringify({ type: "ping", from: "c1", ping: "careful", target: { duel: 0, at: "hero", side: "p1" } })),
    ).toBeNull();
    expect(parseDuosFrame(JSON.stringify({ type: "ping", from: "a2", ping: "wave", target: { duel: 0, at: "hero", side: "p1" } }))).toBeNull();
    expect(parseDuosFrame(JSON.stringify({ type: "clock", now: 5, roundDeadline: 90, graceDeadlines: { a1: null, a2: 7 } }))).toEqual({
      type: "clock",
      now: 5,
      roundDeadline: 90,
      graceDeadlines: { a1: null, a2: 7 },
    });
    expect(parseDuosFrame(JSON.stringify({ type: "teammate", connected: false, aiPlaying: true }))).toEqual({
      type: "teammate",
      connected: false,
      aiPlaying: true,
    });
    expect(parseDuosFrame(JSON.stringify({ type: "view", view: { seat: "a1" }, legal: [{ kind: "concede" }, 3] }))).toEqual({
      type: "view",
      view: { seat: "a1" },
      legal: [{ kind: "concede" }],
    });
    expect(parseDuosFrame("not json")).toBeNull();
    expect(parseDuosFrame(JSON.stringify({ type: "watchers", count: 2 }))).toBeNull();
  });

  it("S6 the co-op socket is the match socket's server at /ws/duos", () => {
    expect(duosSocketUrl("wss://server.example/ws/match")).toBe("wss://server.example/ws/duos");
    expect(duosSocketUrl("ws://localhost:8787/ws/match")).toBe("ws://localhost:8787/ws/duos");
  });
});
