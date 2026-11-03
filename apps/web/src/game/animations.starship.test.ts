// R2801000: the Starship's moment in the runner. Pieces fusing one after another into one Dormant
// Starship play as one fuse run, timed by `starshipFuseMs`, and a launch is a set piece the Starship
// claims (`starshipClaim`), held as R2785000 holds every set piece: capped, scaled by the effects speed
// and set by Big moments. A Starship the seat cannot place (R97's sentinel) claims nothing.

import type { GameEvent, PlayerId, UnitView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { hasSetPiece, wipeClaim } from "../fx/setPieces.ts";
import type { FxBigMoments } from "../fx/settings.ts";
import { starshipClaim } from "../fx/starship.ts";
import {
  ANIMATIONS,
  BURST_BUDGET_MS,
  MIN_ENTRY_MS,
  SET_PIECE_HOLD_CAP_MS,
  SET_PIECE_SHORT_HOLD_MS,
  STARSHIP_FUSE_MS,
  STARSHIP_FUSE_STAGGERED_MAX,
  STARSHIP_LAUNCH_HOLD_MS,
  STARSHIP_SET_PIECE,
  WIPE_MS,
  createAnimationQueue,
  planEntries,
  scaleForSpeed,
  starshipFuseMs,
  type AnimationEntry,
} from "./animations.ts";
import { testid } from "./contract.ts";
import { createPlayTracker } from "./runs.ts";
import { baseView, dormantStarship, emptySide, unit } from "../test/fixtures.ts";
import { FX_STARSHIP_FUSE_STAGGER_MS } from "../fx/constants.ts";

const HULL = "stinkyscape-t-dormant-starship";
const RUSH_LAUNCHER = "stinkyscape-140";
const FRUIT_LAUNCHER = "stinkyscape-141";
const NETHER = "core-088";

const destroyed = (u: UnitView): GameEvent => ({
  type: "destroyed",
  instanceId: u.instanceId,
  defId: u.defId,
  owner: u.owner,
  controller: u.controller,
  attack: u.attack,
  maxHealth: u.maxHealth,
  killerId: null,
});

/** The engine's Fuse into the Starship, then its report (R1852000). */
function fusion(shipId: string, piece: UnitView, pieces: number, player: PlayerId = "p1"): GameEvent[] {
  return [
    { type: "fused", instanceIds: [shipId, `${piece.instanceId}-copy`], resultInstanceId: shipId, defId: HULL },
    { type: "starshipFused", player, instanceId: shipId, pieceId: piece.instanceId, defId: piece.defId, pieces },
  ];
}

const launched = (instanceId: string, pieces: readonly string[], player: PlayerId = "p1"): GameEvent => ({
  type: "starshipLaunched",
  player,
  instanceId,
  defId: HULL,
  name: "Ember of Departure",
  pieces: [...pieces],
});

/** p1's board: two Launchers in lanes 1 and 2 and, unless `noShip`, a Dormant Starship in lane 3. */
function board(noShip = false) {
  const rush = unit("p1", { instanceId: "rush", defId: RUSH_LAUNCHER, attack: 0 });
  const fruit = unit("p1", { instanceId: "fruit", defId: FRUIT_LAUNCHER, attack: 0 });
  const ship = dormantStarship("p1", [], { instanceId: "ship" });
  const view = baseView({
    you: emptySide("p1", { units: [rush, fruit, noShip ? null : ship, null, null] }),
    opponent: emptySide("p2", { units: [unit("p2", { instanceId: "foe" }), null, null, null, null] }),
  });
  return { view, rush, fruit, ship };
}

const fuses = (entries: readonly AnimationEntry[]): AnimationEntry[] => entries.filter((entry) => entry.fuse !== undefined);

/** A queue whose timers run when the test says, claiming set pieces as Game does. */
function queueOf(settings: { speed?: number; bigMoments?: FxBigMoments } = {}, burstBudgetMs = BURST_BUDGET_MS) {
  const timers: { ms: number; run: () => void }[] = [];
  const started: AnimationEntry[] = [];
  const queue = createAnimationQueue({
    reducedMotion: false,
    schedule: (run, ms) => {
      timers.push({ ms, run });
    },
    settings: () => ({ speed: settings.speed ?? 1, motion: "system", intensity: "normal", bigMoments: settings.bigMoments ?? "full" }),
    wipeOf: hasSetPiece,
    setPieceOf: (entry) => wipeClaim(entry) ?? starshipClaim(entry),
    burstBudgetMs,
  });
  queue.subscribeSignals((signal) => {
    if (signal.kind === "start") started.push(signal.entry);
  });
  const playAll = (): AnimationEntry[] => {
    for (let guard = 0; guard < 200 && timers.length > 0; guard += 1) timers.shift()?.run();
    return started;
  };
  return { queue, playAll };
}

describe("R2801000 a fuse run", () => {
  it("R2801000 two fusions are one entry of starshipFuseMs(2), pieces in order", () => {
    const { view, rush, fruit, ship } = board();
    const events = [destroyed(rush), destroyed(fruit), ...fusion(ship.instanceId, rush, 1), ...fusion(ship.instanceId, fruit, 2)];
    const entries = planEntries(events, view, false, createPlayTracker());
    const [run, ...others] = fuses(entries);
    expect(others).toEqual([]);
    expect(run?.type).toBe("starshipFused");
    expect(run?.events).toEqual(events.slice(2));
    expect(run?.durationMs).toBe(starshipFuseMs(2));
    expect(starshipFuseMs(2)).toBe(STARSHIP_FUSE_MS + FX_STARSHIP_FUSE_STAGGER_MS);
    expect(run?.fuse).toEqual({
      shipId: "ship",
      player: "p1",
      anchor: testid.card("ship"),
      pieces: [
        { pieceId: "rush", from: testid.card("rush") },
        { pieceId: "fruit", from: testid.card("fruit") },
      ],
      count: 2,
    });
    expect(run?.frames.get(testid.card("ship"))).toBe("starshipFused");
    // The deaths before it keep their own entries.
    expect(entries.filter((entry) => entry.type === "destroyed")).toHaveLength(2);

    // The run's length stops growing at STARSHIP_FUSE_STAGGERED_MAX pieces.
    expect(starshipFuseMs(STARSHIP_FUSE_STAGGERED_MAX + 3)).toBe(starshipFuseMs(STARSHIP_FUSE_STAGGERED_MAX));
    expect(starshipFuseMs(0)).toBe(STARSHIP_FUSE_MS);
  });

  it("R2801000 a Starship the pass summons centres on its zone; an Equip's fuse and another Starship's are their own entries", () => {
    const { view, rush, fruit } = board(true);
    const summoned: GameEvent = { type: "summoned", player: "p1", instanceId: "new-ship", defId: HULL, row: "backrow", lane: 3 };
    const equip: GameEvent = { type: "fused", instanceIds: ["foe", "rush"], resultInstanceId: "foe", defId: "core-004" };
    const theirs: GameEvent[] = [
      { type: "fused", instanceIds: ["their-ship", "x"], resultInstanceId: "their-ship", defId: HULL },
      { type: "starshipFused", player: "p2", instanceId: "their-ship", pieceId: "hidden", defId: "hidden", pieces: 1 },
    ];
    const events = [summoned, ...fusion("new-ship", rush, 1), ...theirs, equip, ...fusion("new-ship", fruit, 2)];
    const runs = fuses(planEntries(events, view, false, createPlayTracker()));
    expect(runs.map((entry) => entry.events.length)).toEqual([2, 2, 2]);
    expect(runs[0]?.fuse?.anchor).toBe(testid.zone("you", "backrow", 3));
    expect(runs[0]?.fuse?.count).toBe(1);
    // The other seat's piece the board does not show flies from its owner's graveyard.
    expect(runs[1]?.fuse).toMatchObject({ shipId: "their-ship", player: "p2", anchor: null, pieces: [{ pieceId: "hidden", from: null }] });
    expect(runs[2]?.fuse?.anchor).toBe(testid.zone("you", "backrow", 3));
    // The Equip's fuse is no Starship's.
    const plain = planEntries([equip], view, false, createPlayTracker());
    expect(plain.map((entry) => [entry.type, entry.fuse])).toEqual([["fused", undefined]]);
  });

  it("R2801000 a fuse run is not pinned: the burst budget squeezes it", () => {
    const { view, rush, fruit, ship } = board();
    const { queue, playAll } = queueOf({}, starshipFuseMs(2) / 4);
    queue.enqueue([...fusion(ship.instanceId, rush, 1), ...fusion(ship.instanceId, fruit, 2)], view);
    const [run] = fuses(playAll());
    expect(run?.durationMs).toBeLessThan(starshipFuseMs(2));
    expect(run?.durationMs).toBeGreaterThanOrEqual(MIN_ENTRY_MS);
    expect(run?.setPiece).toBeUndefined();
  });
});

describe("R2801000 a launch is the Starship's set piece", () => {
  it("R2801000 a hidden Starship: anchor null, no claim", () => {
    const { view } = board();
    const [hidden] = planEntries([launched("hidden", ["core-008"])], view, false, createPlayTracker());
    expect(hidden?.launch).toEqual({ shipId: "hidden", player: "p1", anchor: null, pieces: 1, name: "Ember of Departure" });
    expect(hidden !== undefined && starshipClaim(hidden)).toBeNull();
    const { queue, playAll } = queueOf();
    queue.enqueue([launched("hidden", ["core-008"])], view);
    expect(playAll().filter((entry) => entry.setPiece !== undefined)).toEqual([]);
    // R97: a hidden Starship's fusions place nothing either.
    const [fuse] = fuses(planEntries(fusion("hidden", unit("p1", { instanceId: "hidden" }), 1), view, false, createPlayTracker()));
    expect(fuse?.fuse?.anchor).toBeNull();
  });

  it("R2801000 the launch holds as starship: full, short, off, speed 2, the cap", () => {
    const { view } = board();
    const heldBy = (settings: { speed?: number; bigMoments?: FxBigMoments }): AnimationEntry | undefined => {
      const { queue, playAll } = queueOf(settings);
      queue.enqueue([launched("ship", ["core-008", "core-011"])], view);
      return playAll().find((entry) => entry.launch !== undefined);
    };
    const row = ANIMATIONS.starshipLaunched.durationMs;
    const full = heldBy({});
    expect(full?.launch).toEqual({ shipId: "ship", player: "p1", anchor: testid.card("ship"), pieces: 2, name: "Ember of Departure" });
    expect(full?.setPiece).toEqual({ key: STARSHIP_SET_PIECE, holdMs: STARSHIP_LAUNCH_HOLD_MS });
    expect(full?.durationMs).toBe(row + STARSHIP_LAUNCH_HOLD_MS);
    expect(full?.frames.get(testid.card("ship"))).toBe("starshipLaunched");
    expect(heldBy({ bigMoments: "short" })?.setPiece?.holdMs).toBe(SET_PIECE_SHORT_HOLD_MS);
    const off = heldBy({ bigMoments: "off" });
    expect(off?.setPiece).toBeUndefined();
    expect(off?.durationMs).toBe(row);
    const fast = heldBy({ speed: 2 });
    expect(fast?.setPiece?.holdMs).toBe(scaleForSpeed(STARSHIP_LAUNCH_HOLD_MS, 2));
    expect(fast?.durationMs).toBe(scaleForSpeed(row, 2) + scaleForSpeed(STARSHIP_LAUNCH_HOLD_MS, 2));
    // A quarter speed asks four times the beat; the cap holds it to SET_PIECE_HOLD_CAP_MS of real time.
    expect(scaleForSpeed(STARSHIP_LAUNCH_HOLD_MS, 0.25)).toBeGreaterThan(SET_PIECE_HOLD_CAP_MS);
    expect(heldBy({ speed: 0.25 })?.setPiece?.holdMs).toBe(SET_PIECE_HOLD_CAP_MS);
  });

  it("R2801000 a launch after a held wipe in one action holds nothing", () => {
    const { view, rush, fruit } = board();
    const nether: GameEvent[] = [
      { type: "cardPlayed", player: "p1", instanceId: "nx", defId: NETHER, costPaid: 8 },
      { type: "hookResolving", sourceId: "nx", defId: NETHER, controller: "p1", hook: "cry" },
      destroyed(rush),
      destroyed(fruit),
      { type: "cardResolved", player: "p1", instanceId: "nx", defId: NETHER, permanent: false, costPaid: 8 },
    ];
    const { queue, playAll } = queueOf();
    queue.enqueue([...nether, launched("ship", ["core-008"])], view);
    const started = playAll();
    const wipe = started.find((entry) => entry.wipe !== undefined);
    expect(wipe?.setPiece?.key).toBe("nether");
    expect(wipe?.durationMs).toBeGreaterThan(WIPE_MS);
    // Unheld, it is a plain entry: no hold, and the burst budget the held wipe spent may squeeze it.
    const launch = started.find((entry) => entry.launch !== undefined);
    expect(launch?.setPiece).toBeUndefined();
    expect(launch?.durationMs).toBeLessThanOrEqual(ANIMATIONS.starshipLaunched.durationMs);
    expect(started.filter((entry) => entry.setPiece !== undefined)).toHaveLength(1);
  });

  it("R2801000 under reduced motion a launch keeps its `launch` and plays no time", () => {
    const { view } = board();
    const [still] = planEntries([launched("ship", ["core-008"])], view, true, createPlayTracker());
    expect(still?.durationMs).toBe(0);
    expect(still?.launch?.anchor).toBe(testid.card("ship"));
  });
});
