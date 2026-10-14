// R631 (SPEC §10.11 "Music"): the game's music director. It follows the viewer's own view, starts a
// station switch at the moment R204 gives a card's cast line and only for a card the viewer can read,
// and opens a match on its sting. R822000: no card plays a theme.
// R1350, R1351: a Legendary's or a Mythic's intro at its card's moment, and what cuts it.

import { beforeEach, describe, expect, it, vi } from "vitest";

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { DEFAULT_AUDIO_SETTINGS } from "./settings.ts";
import type { MusicRequest } from "./music.ts";
import { MUSIC_ROTATION_KEY } from "./constants.ts";
import { LOW_HEALTH_AT, createMusicDirector, rotationFor, type MusicDirector } from "./musicDirector.ts";
import type { AudioSettings } from "./types.ts";
import { baseView, card, emptySide } from "../test/fixtures.ts";

/** #97 Zephyrs (a Mythic Spell), #96 My Pawn (a Mythic Trap), #50 K-Pop Fanatic (EDM), #4 Gary (Tavern). */
const ZEPHYRS = "core-097";
const MY_PAWN = "core-096";
const CEASELESS_VOID = "core-100";
const K_POP = "core-050";
/** #52 Silly Silas (a Legendary Unit). */
const SILLY_SILAS = "core-052";
/** C+ #37 Wardrum (a Legendary Unit that switches to Epic Orchestral). */
const WARDRUM = "classicplus-037";

type Rig = {
  director: MusicDirector;
  requests: MusicRequest[];
  last(): MusicRequest;
  resets: number;
  preloads: string[][];
  /** R1350: the intros asked for, in order; how many times one was cut; the intros fetched ahead. */
  intros: string[];
  stops: number;
  introPreloads: string[][];
  /** Every intro and request, in the order the director made them. */
  order: string[];
  settings: AudioSettings;
  setSettings(patch: Partial<AudioSettings>): void;
};

function rig(options: { settings?: Partial<AudioSettings>; rotation?: number } = {}): Rig {
  const requests: MusicRequest[] = [];
  const listeners = new Set<(s: AudioSettings) => void>();
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
    intros: [],
    stops: 0,
    introPreloads: [],
    order: [],
    settings: { ...DEFAULT_AUDIO_SETTINGS, ...options.settings },
    setSettings(patch) {
      r.settings = { ...r.settings, ...patch };
      for (const l of [...listeners]) l(r.settings);
    },
  };
  r.director = createMusicDirector({
    sink: {
      request: (req) => {
        requests.push(req);
        r.order.push(`request ${String(req.track)}`);
      },
      resetResume: () => {
        r.resets += 1;
      },
      preload: (ids) => r.preloads.push([...ids]),
      playIntro: (id) => {
        r.intros.push(id);
        r.order.push(`intro ${id}`);
      },
      stopIntro: () => {
        r.stops += 1;
      },
      preloadIntros: (ids) => r.introPreloads.push([...ids]),
    },
    settings: () => r.settings,
    subscribeSettings: (l) => {
      listeners.add(l);
      return () => listeners.delete(l);
    },
    rotation: () => options.rotation ?? 0,
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

describe("R631 station switches", () => {
  it("R631 a card the viewer cannot read switches nothing", () => {
    const r = rig();
    const view = midGame();
    r.director.onView(view);
    r.director.onEvent({ type: "cardPlayed", player: "p2", instanceId: "hidden", defId: "hidden", costPaid: 3 }, view);
    expect(r.last().track).toBe("tavern-1");
  });

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

describe("R822000 no card plays a theme", () => {
  it("R822000 a readable Mythic or Legendary, from either seat, opens its intro and leaves the station's in-game track playing", () => {
    for (const [defId, seat] of [[ZEPHYRS, "p1"], [CEASELESS_VOID, "p2"], [SILLY_SILAS, "p1"]] as const) {
      const r = rig();
      const view = midGame();
      r.director.onView(view);
      r.order.length = 0;
      r.director.onEvent(played(defId, seat), view);
      expect(r.order, defId).toEqual([`intro intro-${defId}`]);
      expect(r.last().track, defId).toBe("tavern-1");
    }
  });

  it("R822000 a Mythic Trap's firing opens its intro and nothing else", () => {
    const r = rig();
    const view = midGame();
    r.director.onView(view);
    r.order.length = 0;
    r.director.onEvent({ type: "trapFired", instanceId: "t1", defId: MY_PAWN, controller: "p1", row: "backrow", lane: 2 }, view);
    expect(r.order).toEqual(["intro intro-core-096"]);
    expect(r.last().track).toBe("tavern-1");
  });

  it("R822000 a Legendary that switches its caster's station (Classic+ #37 Wardrum) opens its intro, then moves to Epic Orchestral", () => {
    const r = rig();
    const view = midGame();
    r.director.onView(view);
    r.order.length = 0;
    r.director.onEvent(played(WARDRUM), view);
    expect(r.order).toEqual(["intro intro-classicplus-037", "request epic-1"]);
  });
});

describe("R631 dynamic music off", () => {
  it("R631 plays the station's in-game track only: no sting, no turn mix, no urgency, no switch", () => {
    const r = rig({ settings: { dynamicMusic: false } });
    r.director.onView(opening({ active: "p2" }));
    expect(r.last()).toEqual({ track: "tavern-1", intro: null, opponentTurn: false });
    const view = withHealth(2, { active: "p2" });
    r.director.onView(view);
    r.director.settle();
    r.director.onEvent(played(ZEPHYRS), view);
    r.director.onEvent(played(K_POP), view);
    expect(r.last()).toEqual({ track: "tavern-1", intro: null, opponentTurn: false });
  });
});

describe("R631 a hotseat hand-over", () => {
  it("R631 plays the arriving seat's own music, with no station carried over", () => {
    const r = rig();
    const p1 = midGame();
    r.director.onView(p1);
    r.director.onEvent(played(K_POP, "p1"), p1);
    expect(r.last().track).toBe("edm-1");
    const p2 = midGame({ viewer: "p2", active: "p2", you: emptySide("p2"), opponent: emptySide("p1") });
    r.director.onView(p2);
    expect(r.last()).toMatchObject({ track: "tavern-1", opponentTurn: false });
    // An event planned against the other seat's view is not this seat's.
    r.director.onEvent(played(K_POP, "p2"), p1);
    expect(r.last().track).toBe("tavern-1");
  });
});

/** Units put onto the field: #52 Silly Silas (played, a Legendary) and C+ #19.1 Top Loser (a token printed Legendary). */
const TOP_LOSER = "classicplus-019-1";
const summonedOf = (defId: string, instanceId: string, row: "units" | "backrow" = "units"): GameEvent => ({ type: "summoned", player: "p1", instanceId, defId, row, lane: 1 });

describe("R1350 a Legendary's or a Mythic's intro", () => {
  it("R1350 a readable Legendary or Mythic opens its play with its own intro, from either seat", () => {
    for (const seat of ["p1", "p2"] as const) {
      const r = rig();
      const view = midGame();
      r.director.onView(view);
      r.order.length = 0;
      r.director.onEvent(played(ZEPHYRS, seat), view);
      expect(r.intros, seat).toEqual(["intro-core-097"]);
      expect(r.order, seat).toEqual(["intro intro-core-097"]);
    }
  });

  it("R1350 a played Unit opens on its cardPlayed, and its own summoned adds nothing", () => {
    const r = rig();
    const view = midGame();
    r.director.onView(view);
    r.director.onEvent(played(SILLY_SILAS), view);
    r.director.onEvent(summonedOf(SILLY_SILAS, `i-${SILLY_SILAS}`), view);
    expect(r.intros).toEqual(["intro-core-052"]);
  });

  it("R1350 a Unit an effect puts onto the field without playing it (a token) opens on its summoned", () => {
    const r = rig();
    const view = midGame();
    r.director.onView(view);
    r.director.onEvent(summonedOf(TOP_LOSER, "t1"), view);
    expect(r.intros).toEqual(["intro-classicplus-019-1"]);
    // A summon to the backrow is a Trap's arrival, which sounds like every Trap's (R203).
    r.director.onEvent(summonedOf(TOP_LOSER, "t2", "backrow"), view);
    expect(r.intros).toHaveLength(1);
  });

  it("R1350 a card behind the sentinel opens nothing, played or summoned", () => {
    const r = rig();
    const view = midGame();
    r.director.onView(view);
    r.director.onEvent({ type: "cardPlayed", player: "p2", instanceId: "hidden", defId: "hidden", costPaid: 3 }, view);
    r.director.onEvent(summonedOf("hidden", "hidden"), view);
    r.director.onEvent({ type: "trapFired", instanceId: "t1", defId: "hidden", controller: "p2", row: "backrow", lane: 2 }, view);
    expect(r.intros).toEqual([]);
  });

  it("R1350 a Trap's set opens nothing; its first firing opens its intro, and a firing again does not", () => {
    const r = rig();
    const view = midGame();
    r.director.onView(view);
    r.director.onEvent(played(MY_PAWN), view);
    expect(r.intros).toEqual([]);
    r.director.onEvent({ type: "trapFired", instanceId: "t1", defId: MY_PAWN, controller: "p1", row: "backrow", lane: 2 }, view);
    expect(r.intros).toEqual(["intro-core-096"]);
    const fuse: GameEvent = { type: "trapFired", instanceId: "t5", defId: "classicplus-074", controller: "p1", row: "backrow", lane: 3 };
    r.director.onEvent(fuse, view);
    r.director.onEvent(fuse, view);
    expect(r.intros).toEqual(["intro-core-096", "intro-classicplus-074"]);
  });

  it("R1350 a card that is neither Legendary nor Mythic opens nothing", () => {
    const r = rig();
    const view = midGame();
    r.director.onView(view);
    r.director.onEvent(played(K_POP), view);
    expect(r.intros).toEqual([]);
  });

  it("R1350 with dynamic music off no card opens an intro", () => {
    const r = rig({ settings: { dynamicMusic: false } });
    const view = midGame();
    r.director.onView(view);
    r.director.onEvent(played(ZEPHYRS), view);
    r.director.onEvent(summonedOf(TOP_LOSER, "t1"), view);
    expect(r.intros).toEqual([]);
  });

  it("R1350 the same card played again asks for its intro again (the player keeps one already playing, R1351)", () => {
    const r = rig();
    const view = midGame();
    r.director.onView(view);
    r.director.onEvent(played(ZEPHYRS), view);
    r.director.onEvent({ type: "cardPlayed", player: "p1", instanceId: "i-second", defId: ZEPHYRS, costPaid: 3 }, view);
    expect(r.intros).toEqual(["intro-core-097", "intro-core-097"]);
  });

  it("R1350 fetches the intros of the viewer's own hand ahead, once each", () => {
    const r = rig();
    const view = midGame({ you: emptySide("p1", { hand: [card({ defId: ZEPHYRS }), card({ defId: K_POP }), card({ defId: SILLY_SILAS })] }) });
    r.director.onView(view);
    r.director.settle();
    r.director.settle();
    expect(r.introPreloads).toEqual([["intro-core-097", "intro-core-052"]]);
  });
});

describe("R1351 what cuts an intro short", () => {
  it("R1351 the game's end cuts it", () => {
    const r = rig();
    const view = midGame();
    r.director.onView(view);
    r.director.onEvent(played(ZEPHYRS), view);
    const before = r.stops;
    r.director.onView(midGame({ phase: "over", result: { winner: "p1", reason: "hero-death" } }));
    r.director.settle();
    expect(r.stops).toBeGreaterThan(before);
    expect(r.last().track).toBe("victory");
  });

  it("R1351 a hotseat hand-over cuts it, and the arriving seat's own plays open theirs", () => {
    const r = rig();
    const p1 = midGame();
    r.director.onView(p1);
    r.director.onEvent(played(SILLY_SILAS, "p1"), p1);
    const before = r.stops;
    const p2 = midGame({ viewer: "p2", active: "p2", you: emptySide("p2"), opponent: emptySide("p1") });
    r.director.onView(p2);
    expect(r.stops).toBeGreaterThan(before);
    // A Unit the last seat's view saw played is the arriving seat's news only by its own events.
    r.director.onEvent(summonedOf(SILLY_SILAS, `i-${SILLY_SILAS}`), p2);
    expect(r.intros).toEqual(["intro-core-052", "intro-core-052"]);
  });

  it("R1351 turning dynamic music off cuts it", () => {
    const r = rig();
    const view = midGame();
    r.director.onView(view);
    r.director.onEvent(played(ZEPHYRS), view);
    const before = r.stops;
    r.setSettings({ dynamicMusic: false });
    expect(r.stops).toBeGreaterThan(before);
  });

  it("R1351 the board leaving cuts it", () => {
    const r = rig();
    const view = midGame();
    r.director.onView(view);
    r.director.onEvent(played(ZEPHYRS), view);
    const before = r.stops;
    r.director.dispose();
    expect(r.stops).toBe(before + 1);
  });

  it("R1351 the viewer's hero taking a hit leaves the intro playing", () => {
    const r = rig();
    const view = midGame();
    r.director.onView(view);
    r.director.onEvent(played(ZEPHYRS), view);
    const before = r.stops;
    r.director.onEvent(hitOn("p1"), view);
    expect(r.stops).toBe(before);
    expect(r.last().track).toBe("tavern-1");
  });
});
