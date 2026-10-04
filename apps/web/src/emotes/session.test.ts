// The emote session (session.ts) driven through its injected clock — no timers, no React.
//
// R643 is what is proved here: an admitted send shows for that player at once, a send the shared
// `emoteGate` drops shows nothing (and notifies nobody — the same silent drop the server gives),
// a received emote shows unless its player is muted, one emote per player stands at a time, and
// `expire` clears only the show it was armed for. The state is keyed by PlayerId throughout, so a
// hotseat hand-over moves nothing.

import { describe, expect, it, vi } from "vitest";

import {
  EMOTE_COOLDOWN_MS,
  EMOTE_WINDOW_MAX,
  EMOTE_WINDOW_MS,
  type PlayerId,
} from "@jackioh/shared";

import { createEmoteSession, type EmoteSession } from "./session.ts";

/** A session on a clock the test winds by hand. */
function harness(): { session: EmoteSession; now: (value: number) => void; tick: () => number } {
  let at = 0;
  return {
    session: createEmoteSession({ now: () => at }),
    now: (value) => {
      at = value;
    },
    tick: () => at,
  };
}

const HOLD = 3_000;

describe("R643 the emote session", () => {
  it("R643 an admitted send shows for that player immediately, before any relay could come back", () => {
    const { session, tick } = harness();

    const sent = session.send("p1", "greetings", "Hello.", HOLD);

    expect(sent).toBe(true);
    expect(session.visible("p1")).toEqual({
      key: 1,
      emote: "greetings",
      text: "Hello.",
      until: tick() + HOLD,
    });
  });

  it("R643 a send inside the cooldown is dropped silently: nothing new shows and nobody is told", () => {
    const { session, now } = harness();
    const listener = vi.fn();
    session.subscribe(listener);

    expect(session.send("p1", "greetings", "Hello.", HOLD)).toBe(true);
    expect(listener).toHaveBeenCalledTimes(1);

    now(EMOTE_COOLDOWN_MS - 1);
    const gate = session.gate("p1");
    expect(gate.ok).toBe(false);
    if (gate.ok === false) expect(gate.retryAfterMs).toBe(1);

    // The menu stays open and the first emote keeps standing: the drop changes nothing on screen.
    expect(session.send("p1", "oops", "Oops.", HOLD)).toBe(false);
    expect(session.visible("p1")).toMatchObject({ key: 1, emote: "greetings" });
    expect(listener).toHaveBeenCalledTimes(1);
  });

  it("R643 the gate re-admits once the cooldown has passed, and the other seat's limiter is untouched", () => {
    const { session, now } = harness();

    expect(session.send("p1", "greetings", "Hello.", HOLD)).toBe(true);
    now(EMOTE_COOLDOWN_MS - 100);
    expect(session.send("p2", "oops", null, HOLD)).toBe(true); // the other seat is unaffected
    expect(session.send("p1", "oops", "Oops.", HOLD)).toBe(false);

    now(EMOTE_COOLDOWN_MS);
    expect(session.gate("p1").ok).toBe(true);
    expect(session.send("p1", "oops", "Oops.", HOLD)).toBe(true);
  });

  it("R643 the window cap drops the sixth emote of a rolling window and re-admits once it rolls off", () => {
    const { session, now } = harness();

    // Five sends, each past the previous one's cooldown but all inside one window.
    for (let i = 0; i < EMOTE_WINDOW_MAX; i += 1) {
      expect(session.send("p1", "laugh", null, HOLD), `send ${i + 1}`).toBe(true);
      now((i + 1) * (EMOTE_COOLDOWN_MS + 1));
    }

    const gate = session.gate("p1");
    expect(gate.ok).toBe(false);
    if (gate.ok === false) {
      // The oldest send leaves the window EMOTE_WINDOW_MS after it landed.
      expect(gate.retryAfterMs).toBe(EMOTE_WINDOW_MS - EMOTE_WINDOW_MAX * (EMOTE_COOLDOWN_MS + 1));
    }
    expect(session.send("p1", "laugh", null, HOLD)).toBe(false);
    expect(session.visible("p1")?.emote).toBe("laugh");

    now(EMOTE_WINDOW_MS);
    expect(session.gate("p1").ok).toBe(true);
    expect(session.send("p1", "laugh", null, HOLD)).toBe(true);
  });

  it("R643 an emote received from the opponent shows", () => {
    const { session, tick } = harness();

    expect(session.receive("p2", "sob", null, HOLD)).toBe(true);
    expect(session.visible("p2")).toEqual({ key: 1, emote: "sob", text: null, until: tick() + HOLD });
  });

  it("R643 a muted player's emote never lands — not shown, not kept, nobody notified", () => {
    const { session } = harness();
    const listener = vi.fn();
    session.subscribe(listener);

    session.mute("p2");

    expect(session.muted("p2")).toBe(true);
    expect(session.receive("p2", "laugh", null, HOLD)).toBe(false);
    expect(session.visible("p2")).toBeNull();
    // The mute itself emits only when a show is removed; there was none, and the drop emits never.
    expect(listener).not.toHaveBeenCalled();
  });

  it("R643 muting a player takes their current emote off the board too", () => {
    const { session } = harness();
    const listener = vi.fn();

    expect(session.receive("p2", "angry", null, HOLD)).toBe(true);
    expect(session.visible("p2")).not.toBeNull();

    session.subscribe(listener);
    session.mute("p2");

    expect(session.visible("p2")).toBeNull();
    expect(listener).toHaveBeenCalledTimes(1);
  });

  it("R643 a second emote from the same player replaces the first at once", () => {
    const { session } = harness();

    expect(session.receive("p2", "sob", null, HOLD)).toBe(true);
    const first = session.visible("p2");
    expect(session.receive("p2", "wahWah", null, HOLD)).toBe(true);

    const second = session.visible("p2");
    expect(second?.emote).toBe("wahWah");
    expect(second?.key).not.toBe(first?.key);
  });

  it("R643 a send replaces that player's own standing emote too", () => {
    const { session, now } = harness();

    expect(session.send("p1", "greetings", "Hello.", HOLD)).toBe(true);
    now(EMOTE_COOLDOWN_MS);
    expect(session.send("p1", "thanks", "Thanks.", HOLD)).toBe(true);

    expect(session.visible("p1")).toMatchObject({ emote: "thanks", text: "Thanks." });
  });

  it("R643 expire clears only the show it was armed for — a newer one stands", () => {
    const { session } = harness();

    session.receive("p2", "sob", null, HOLD);
    const stale = session.visible("p2");
    session.receive("p2", "laugh", null, HOLD);
    const current = session.visible("p2");
    if (stale === null || current === null) throw new Error("both shows should have landed");

    // The stale show's timer firing late must not remove its replacement.
    session.expire("p2", stale.key);
    expect(session.visible("p2")).toBe(current);

    session.expire("p2", current.key);
    expect(session.visible("p2")).toBeNull();
  });

  it("R643 the state is keyed by PlayerId: one seat's emote never touches the other's", () => {
    const { session } = harness();
    const players: PlayerId[] = ["p1", "p2"];

    session.send("p1", "greetings", "Hello.", HOLD);

    expect(session.visible("p2")).toBeNull();
    for (const player of players) {
      expect(session.gate(player).ok, player).toBe(player === "p1" ? false : true);
    }

    // Muting one seat leaves the other able to show.
    session.mute("p1");
    expect(session.receive("p2", "yawn", null, HOLD)).toBe(true);
    expect(session.visible("p2")?.emote).toBe("yawn");
  });
});
