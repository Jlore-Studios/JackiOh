// The sideline session (session.ts) on a clock the test winds by hand: the watcher's gate at the
// server's numbers for a watcher (R2990002), one show per watcher with the newest last, and a mute that
// keeps the sender's own (R2990004).

import { describe, expect, it, vi } from "vitest";

import { SPECTATOR_EMOTE_COOLDOWN_MS, SPECTATOR_EMOTE_WINDOW_MAX, SPECTATOR_EMOTE_WINDOW_MS } from "@jackioh/server-config";
import { emoteGate, SEAT_EMOTE_LIMITS } from "@jackioh/shared";

import { SELF, SPECTATOR_EMOTE_LIMITS, createSidelineSession, type SidelineSession } from "./session.ts";

function harness(): { session: SidelineSession; now: (value: number) => void } {
  let at = 0;
  return {
    session: createSidelineSession({ now: () => at }),
    now: (value) => {
      at = value;
    },
  };
}

const HOLD = 2_000;

describe("R2990002 the watcher's gate", () => {
  it("R2990002 the gate is the server's for a watcher: its cooldown and its window, never a seat's", () => {
    expect(SPECTATOR_EMOTE_LIMITS).toEqual({
      cooldownMs: SPECTATOR_EMOTE_COOLDOWN_MS,
      windowMs: SPECTATOR_EMOTE_WINDOW_MS,
      windowMax: SPECTATOR_EMOTE_WINDOW_MAX,
    });
    expect(SPECTATOR_EMOTE_LIMITS).not.toEqual(SEAT_EMOTE_LIMITS);
  });

  it("R2990002 a send inside the cooldown is dropped silently, and re-admitted once it has passed", () => {
    const { session, now } = harness();
    const listener = vi.fn();
    session.subscribe(listener);

    expect(session.send("wave", HOLD)).toBe(true);
    expect(listener).toHaveBeenCalledTimes(1);

    now(SPECTATOR_EMOTE_COOLDOWN_MS - 1);
    const gate = session.gate();
    expect(gate.ok).toBe(false);
    if (gate.ok === false) expect(gate.retryAfterMs).toBe(1);
    expect(session.send("clap", HOLD)).toBe(false);
    expect(session.shows()).toMatchObject([{ watcher: SELF, emote: "wave", self: true }]);
    expect(listener).toHaveBeenCalledTimes(1);

    now(SPECTATOR_EMOTE_COOLDOWN_MS);
    expect(session.gate().ok).toBe(true);
    expect(session.send("clap", HOLD)).toBe(true);
    expect(session.shows()).toMatchObject([{ watcher: SELF, emote: "clap" }]);
  });

  it("R2990002 the window holds the watcher to SPECTATOR_EMOTE_WINDOW_MAX sends, as the shared gate reads it", () => {
    const { session, now } = harness();
    const sent: number[] = [];
    for (let i = 0; i < SPECTATOR_EMOTE_WINDOW_MAX; i += 1) {
      const at = i * SPECTATOR_EMOTE_COOLDOWN_MS;
      now(at);
      expect(session.send("fire", HOLD), `send ${String(i + 1)}`).toBe(true);
      sent.push(at);
    }

    const at = SPECTATOR_EMOTE_WINDOW_MAX * SPECTATOR_EMOTE_COOLDOWN_MS;
    now(at);
    expect(session.gate()).toEqual(emoteGate(sent, at, SPECTATOR_EMOTE_LIMITS));
    expect(session.gate().ok).toBe(false);
    expect(session.send("fire", HOLD)).toBe(false);

    now(SPECTATOR_EMOTE_WINDOW_MS);
    expect(session.gate().ok).toBe(true);
    expect(session.send("fire", HOLD)).toBe(true);
  });
});

describe("R2990004 the strip's shows and its mute", () => {
  it("R2990004 one show per watcher, a watcher's new emote replacing their last and standing newest", () => {
    const { session, now } = harness();

    expect(session.receive("Wes#13", "sob", HOLD)).toBe(true);
    expect(session.receive("Ann#2", "laugh", HOLD)).toBe(true);
    now(500);
    expect(session.receive("Wes#13", "party", HOLD)).toBe(true);

    expect(session.shows().map((show) => [show.watcher, show.emote, show.self])).toEqual([
      ["Ann#2", "laugh", false],
      ["Wes#13", "party", false],
    ]);
    expect(session.shows().at(-1)?.until).toBe(500 + HOLD);
    expect(session.heard()).toBe(true);
  });

  it("R2990004 expire clears only the show it was armed for: a newer one from the same watcher stands", () => {
    const { session } = harness();
    session.receive("Wes#13", "sob", HOLD);
    const stale = session.shows()[0];
    session.receive("Wes#13", "cool", HOLD);
    const current = session.shows()[0];
    if (stale === undefined || current === undefined) throw new Error("both shows should have landed");

    session.expire("Wes#13", stale.key);
    expect(session.shows()).toEqual([current]);
    session.expire("Wes#13", current.key);
    expect(session.shows()).toEqual([]);
    // What was shown stays heard, so a player's strip keeps its Mute.
    expect(session.heard()).toBe(true);
  });

  it("R2990004 mute drops every other watcher's show for the session and keeps the sender's own", () => {
    const { session, now } = harness();
    expect(session.heard()).toBe(false);
    session.receive("Wes#13", "sob", HOLD);
    session.send("wave", HOLD);
    const listener = vi.fn();
    session.subscribe(listener);

    session.mute();

    expect(session.muted()).toBe(true);
    expect(listener).toHaveBeenCalledTimes(1);
    expect(session.shows()).toMatchObject([{ watcher: SELF, emote: "wave", self: true }]);
    expect(session.receive("Ann#2", "laugh", HOLD)).toBe(false);
    expect(session.shows()).toHaveLength(1);

    now(SPECTATOR_EMOTE_COOLDOWN_MS);
    expect(session.send("heart", HOLD)).toBe(true);
    expect(session.shows()).toMatchObject([{ watcher: SELF, emote: "heart" }]);
  });
});
