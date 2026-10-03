// R631 (SPEC §10.11 "Music"): the game's music director. It follows the viewer's own view, starts a
// Mythic theme or a station switch at the moment R204 gives a card's cast line and only for a card
// the viewer can read, ends a theme on the viewer's next hit, and opens a match on its sting.

import { beforeEach, describe, expect, it, vi } from "vitest";

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { DEFAULT_AUDIO_SETTINGS } from "./settings.ts";
import type { MusicRequest } from "./music.ts";
import { MUSIC_ROTATION_KEY } from "./constants.ts";
import { LOW_HEALTH_AT, createMusicDirector, rotationFor, type MusicDirector } from "./musicDirector.ts";
import type { AudioSettings } from "./types.ts";
import { baseView, emptySide } from "../test/fixtures.ts";

/** #97 Zephyrs (a Mythic Spell), #96 My Pawn (a Mythic Trap), #50 K-Pop Fanatic (EDM), #4 Gary (Tavern). */
const ZEPHYRS = "core-097";
const MY_PAWN = "core-096";
const CEASELESS_VOID = "core-100";
const K_POP = "core-050";

type Rig = {
  director: MusicDirector;
  requests: MusicRequest[];
  last(): MusicRequest;
  resets: number;
  preloads: string[][];
  settings: AudioSettings;
  setSettings(patch: Partial<AudioSettings>): void;
  /** Fires the pending timer (a Mythic theme's end), if one is armed. */
  fireTimer(): void;
  timers: { ms: number; fn: () => void; cancelled: boolean }[];
};

function rig(options: { settings?: Partial<AudioSettings>; rotation?: number } = {}): Rig {
  const requests: MusicRequest[] = [];
  const listeners = new Set<(s: AudioSettings) => void>();
  const timers: Rig["timers"] = [];
  const r: Rig = {
    director: null as unknown as MusicDirector,
    requests,
    last: () => {
      const at = requests.at(-1);
      if (at === undefined) throw new Error("no request yet");
      return at;
    },
    resets: 0,
    preloads: [],
    settings: { ...DEFAULT_AUDIO_SETTINGS, ...options.settings },
    setSettings(patch) {
      r.settings = { ...r.settings, ...patch };
      for (const l of [...listeners]) l(r.settings);
    },
    fireTimer() {
      const live = timers.filter((t) => !t.cancelled);
      const timer = live.at(-1);
      if (timer === undefined) throw new Error("no timer armed");
      timer.cancelled = true;
      timer.fn();
    },
    timers,
  };
  r.director = createMusicDirector({
    sink: {
      request: (req) => requests.push(req),
      resetResume: () => {
        r.resets += 1;
      },
      preload: (ids) => r.preloads.push([...ids]),
    },
    settings: () => r.settings,
    subscribeSettings: (l) => {
      listeners.add(l);
      return () => listeners.delete(l);
    },
    rotation: () => options.rotation ?? 0,
    later: (ms, fn) => {
      const timer = { ms, fn, cancelled: false };
      timers.push(timer);
      return () => {
        timer.cancelled = true;
      };
    },
  });
  return r;
}

const opening = (over: Partial<PlayerView> = {}): PlayerView => baseView({ turn: 1, phase: "mulligan", ...over });
const midGame = (over: Partial<PlayerView> = {}): PlayerView => baseView({ turn: 6, ...over });
const withHealth = (health: number, over: Partial<PlayerView> = {}): PlayerView =>
  midGame({ you: emptySide("p1", { hero: { health, armor: 0, powers: [], power: null } }), ...over });

const played = (defId: string, player: "p1" | "p2" = "p1"): GameEvent => ({ type: "cardPlayed", player, instanceId: `i-${defId}`, defId, costPaid: 3 });
const hitOn = (player: "p1" | "p2", amount = 2): GameEvent => ({ type: "damage", sourceId: "u9", targetId: `hero-${player}`, amount, combat: true });

beforeEach(() => {
  vi.useRealTimers();
});

describe("R631 a match's music opens on its station", () => {
  it("R631 the first view of a new match plays the station's sting into this rotation's in-game track", () => {
    const r = rig({ rotation: 1 });
    r.director.onView(opening());
    expect(r.last()).toEqual({ track: "tavern-2", intro: "tavern-start", opponentTurn: false });
    expect(r.resets).toBe(1);
    // The sting and the in-game track are asked for; the urgency track and the result stings are fetched ahead.
    expect(r.preloads[0]).toEqual(["tavern-danger", "victory", "defeat", "draw"]);
  });

  it("R631 starts on the station the settings name", () => {
    const r = rig({ settings: { station: "lofi" } });
    r.director.onView(opening());
    expect(r.last()).toMatchObject({ track: "lofi-1", intro: "lofi-start" });
  });

  it("R631 a first view past the first turn (a reconnect) skips the sting", () => {
    const r = rig();
    r.director.onView(midGame());
    expect(r.last()).toEqual({ track: "tavern-1", intro: null, opponentTurn: false });
  });

  it("R631 a board mounted twice on the same first view (StrictMode) moves the rotation once", () => {
    localStorage.removeItem(MUSIC_ROTATION_KEY);
    const first = opening();
    expect(rotationFor(first)).toBe(0);
    expect(rotationFor(first)).toBe(0);
    expect(localStorage.getItem(MUSIC_ROTATION_KEY)).toBe("1");
    expect(rotationFor(opening())).toBe(1);
    localStorage.removeItem(MUSIC_ROTATION_KEY);
  });

  it("R631 later requests carry no sting", () => {
    const r = rig();
    r.director.onView(opening());
    r.director.onView(midGame({ active: "p2" }));
    r.director.settle();
    expect(r.last()).toEqual({ track: "tavern-1", intro: null, opponentTurn: true });
  });
});

describe("R631 the viewer's own turn, health and result", () => {
  it("R631 applies the newest view only when the board settles", () => {
    const r = rig();
    r.director.onView(midGame());
    const before = r.requests.length;
    r.director.onView(midGame({ active: "p2" }));
    expect(r.requests.length).toBe(before);
    r.director.settle();
    expect(r.last().opponentTurn).toBe(true);
  });

  it("R631 the opponent's turn is a mix, and the mulligan is nobody's turn", () => {
    const r = rig();
    r.director.onView(opening({ active: "p2" }));
    expect(r.last().opponentTurn).toBe(false);
    r.director.onView(midGame({ active: "p2", phase: "start" }));
    r.director.settle();
    expect(r.last()).toMatchObject({ track: "tavern-1", opponentTurn: true });
    r.director.onView(midGame({ active: "p1" }));
    r.director.settle();
    expect(r.last()).toMatchObject({ track: "tavern-1", opponentTurn: false });
  });

  it("R631 the viewer's hero at a quarter of HERO_HEALTH plays the urgency track, and recovering ends it", () => {
    expect(LOW_HEALTH_AT).toBe(7);
    const r = rig();
    r.director.onView(withHealth(LOW_HEALTH_AT + 1));
    expect(r.last().track).toBe("tavern-1");
    r.director.onView(withHealth(LOW_HEALTH_AT));
    r.director.settle();
    expect(r.last().track).toBe("tavern-danger");
    r.director.onView(withHealth(LOW_HEALTH_AT + 3));
    r.director.settle();
    expect(r.last().track).toBe("tavern-1");
  });

  it("R631 only the viewer's own hero counts: the opponent at low health changes nothing", () => {
    const r = rig();
    r.director.onView(midGame({ opponent: emptySide("p2", { hero: { health: 2, armor: 0, powers: [], power: null } }) }));
    expect(r.last().track).toBe("tavern-1");
  });

  it("R631 names the result from the viewer's side", () => {
    for (const [winner, track] of [["p1", "victory"], ["p2", "defeat"], ["draw", "draw"]] as const) {
      const r = rig();
      r.director.onView(midGame());
      r.director.onView(midGame({ phase: "over", result: { winner, reason: "hero-death" } }));
      r.director.settle();
      expect(r.last().track, winner).toBe(track);
    }
  });
});

describe("R631 Mythic themes", () => {
  it("R631 a readable Mythic cast starts its theme, from either seat", () => {
    for (const seat of ["p1", "p2"] as const) {
      const r = rig();
      const view = midGame();
      r.director.onView(view);
      r.director.onEvent(played(ZEPHYRS, seat), view);
      expect(r.last().track, seat).toBe("mythic-zephyrs");
    }
  });

  it("R631 a card the viewer cannot read starts nothing", () => {
    const r = rig();
    const view = midGame();
    r.director.onView(view);
    r.director.onEvent({ type: "cardPlayed", player: "p2", instanceId: "hidden", defId: "hidden", costPaid: 3 }, view);
    expect(r.last().track).toBe("tavern-1");
  });

  it("R631 a Mythic Trap's theme starts when it fires, never when it is set", () => {
    const r = rig();
    const view = midGame();
    r.director.onView(view);
    r.director.onEvent(played(MY_PAWN), view);
    expect(r.last().track).toBe("tavern-1");
    r.director.onEvent({ type: "trapFired", instanceId: "t1", defId: MY_PAWN, controller: "p1", row: "backrow", lane: 2 }, view);
    expect(r.last().track).toBe("mythic-my-pawn");
  });

  it("R631 a Field Trap that fires again (Classic+ #74 on each fuse) plays its theme at its first firing only", () => {
    const r = rig();
    const view = midGame();
    r.director.onView(view);
    const fires: GameEvent = { type: "trapFired", instanceId: "t5", defId: "classicplus-074", controller: "p1", row: "backrow", lane: 3 };
    r.director.onEvent(fires, view);
    expect(r.last().track).toBe("mythic-twice-forward");
    r.director.onEvent(hitOn("p1"), view);
    expect(r.last().track).toBe("tavern-1");
    r.director.onEvent(fires, view);
    expect(r.last().track).toBe("tavern-1");
  });

  it("R631 the same Mythic again does not restart its theme", () => {
    const r = rig();
    const view = midGame();
    r.director.onView(view);
    r.director.onEvent(played(ZEPHYRS), view);
    const armed = r.timers.length;
    r.director.onEvent(played(ZEPHYRS), view);
    expect(r.timers.length).toBe(armed);
    expect(r.last().track).toBe("mythic-zephyrs");
  });

  it("R631 another Mythic replaces the theme", () => {
    const r = rig();
    const view = midGame();
    r.director.onView(view);
    r.director.onEvent(played(ZEPHYRS), view);
    r.director.onEvent(played(CEASELESS_VOID, "p2"), view);
    expect(r.last().track).toBe("mythic-ceaseless-void");
  });

  it("R631 the viewer's hero taking a hit ends the theme; a hit on the opponent does not", () => {
    const r = rig();
    const view = midGame();
    r.director.onView(view);
    r.director.onEvent(played(ZEPHYRS), view);
    r.director.onEvent(hitOn("p2"), view);
    expect(r.last().track).toBe("mythic-zephyrs");
    r.director.onEvent(hitOn("p1", 0), view);
    expect(r.last().track).toBe("mythic-zephyrs");
    r.director.onEvent({ type: "healthLost", player: "p1", amount: 1 }, view);
    expect(r.last().track).toBe("tavern-1");
  });

  it("R631 at low health the theme holds past its plays until the viewer's next hit, then urgency resumes", () => {
    const r = rig();
    const low = withHealth(5);
    r.director.onView(low);
    expect(r.last().track).toBe("tavern-danger");
    r.director.onEvent(played(ZEPHYRS), low);
    expect(r.last().track).toBe("mythic-zephyrs");
    r.fireTimer();
    r.director.settle();
    expect(r.last().track).toBe("mythic-zephyrs");
    r.director.onEvent(hitOn("p1"), low);
    expect(r.last().track).toBe("tavern-danger");
  });

  it("R631 away from low health the theme ends once it has played twice", () => {
    const r = rig();
    const view = midGame();
    r.director.onView(view);
    r.director.onEvent(played(ZEPHYRS), view);
    const timer = r.timers.at(-1);
    // Its first pass to the loop's end, and one more loop.
    expect(timer?.ms).toBeGreaterThan(20_000);
    r.fireTimer();
    expect(r.last().track).toBe("tavern-1");
  });

  it("R631 a theme held at low health lets go when the viewer recovers", () => {
    const r = rig();
    r.director.onView(withHealth(5));
    r.director.onEvent(played(ZEPHYRS), withHealth(5));
    r.fireTimer();
    r.director.onView(withHealth(20));
    r.director.settle();
    expect(r.last().track).toBe("tavern-1");
  });

  it("R631 the result outranks a theme and ends it", () => {
    const r = rig();
    const view = midGame();
    r.director.onView(view);
    r.director.onEvent(played(ZEPHYRS), view);
    r.director.onView(midGame({ phase: "over", result: { winner: "p1", reason: "hero-death" } }));
    r.director.settle();
    expect(r.last().track).toBe("victory");
  });
});

describe("R631 station switches", () => {
  it("R631 the viewer's own cast switches their station for the rest of the match", () => {
    const r = rig();
    const view = midGame();
    r.director.onView(view);
    r.director.onEvent(played(K_POP, "p1"), view);
    expect(r.last().track).toBe("edm-1");
    expect(r.preloads.at(-1)).toEqual(["edm-danger"]);
    r.director.onView(withHealth(3));
    r.director.settle();
    expect(r.last().track).toBe("edm-danger");
  });

  it("R631 the opponent's cast switches only the opponent's station", () => {
    const r = rig();
    const view = midGame();
    r.director.onView(view);
    r.director.onEvent(played(K_POP, "p2"), view);
    expect(r.last().track).toBe("tavern-1");
  });

  it("R631 the next match starts on the station the settings name", () => {
    const first = rig();
    first.director.onView(midGame());
    first.director.onEvent(played(K_POP), midGame());
    first.director.dispose();
    const next = rig();
    next.director.onView(opening());
    expect(next.last()).toMatchObject({ track: "tavern-1", intro: "tavern-start" });
  });

  it("R631 changing the station in the settings mid-match moves the music at once", () => {
    const r = rig();
    r.director.onView(midGame());
    r.setSettings({ station: "epic" });
    expect(r.last().track).toBe("epic-1");
  });
});

describe("R631 dynamic music off", () => {
  it("R631 plays the station's in-game track only: no sting, no turn mix, no theme, no urgency, no switch", () => {
    const r = rig({ settings: { dynamicMusic: false } });
    r.director.onView(opening({ active: "p2" }));
    expect(r.last()).toEqual({ track: "tavern-1", intro: null, opponentTurn: false });
    const view = withHealth(2, { active: "p2" });
    r.director.onView(view);
    r.director.settle();
    r.director.onEvent(played(ZEPHYRS), view);
    r.director.onEvent(played(K_POP), view);
    expect(r.last()).toEqual({ track: "tavern-1", intro: null, opponentTurn: false });
    expect(r.timers).toHaveLength(0);
  });

  it("R631 turning it off mid-theme drops the theme", () => {
    const r = rig();
    const view = midGame();
    r.director.onView(view);
    r.director.onEvent(played(ZEPHYRS), view);
    r.setSettings({ dynamicMusic: false });
    expect(r.last().track).toBe("tavern-1");
    r.setSettings({ dynamicMusic: true });
    expect(r.last().track).toBe("tavern-1");
  });
});

describe("R631 a hotseat hand-over", () => {
  it("R631 plays the arriving seat's own music, with no theme carried over", () => {
    const r = rig();
    const p1 = midGame();
    r.director.onView(p1);
    r.director.onEvent(played(K_POP, "p1"), p1);
    r.director.onEvent(played(ZEPHYRS, "p1"), p1);
    expect(r.last().track).toBe("mythic-zephyrs");
    const p2 = midGame({ viewer: "p2", active: "p2", you: emptySide("p2"), opponent: emptySide("p1") });
    r.director.onView(p2);
    expect(r.last()).toMatchObject({ track: "tavern-1", opponentTurn: false });
    // An event planned against the other seat's view is not this seat's.
    r.director.onEvent(played(ZEPHYRS, "p1"), p1);
    expect(r.last().track).toBe("tavern-1");
  });
});
