// R2800000: a board clear's wipe. A set-piece card's hook (`hookResolving`, or a Trap's `trapFired`)
// makes its first run of removals one pinned WIPE_MS entry, which its set piece claims as the action's
// one hold (R2785000): capped, scaled by the effects speed and set by Big moments. A hidden hook, a card
// with no set piece and a caller that names none keep one entry per removal. Ceaseless Void's takeover
// (R2722001) yields to its own fold, so the action still holds once.

import { CATALOG } from "@jackioh/cards";
import type { CardType, GameEvent, PlayerId, PlayerView, UnitView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { hasSetPiece, SET_PIECES, VOID_FOLD_HOLD_MS, wipeClaim } from "../fx/setPieces.ts";
import type { FxBigMoments } from "../fx/settings.ts";
import {
  BURST_BUDGET_MS,
  SET_PIECE_HOLD_MS,
  SET_PIECE_SHORT_HOLD_MS,
  TAKEOVER_HOLD_MS,
  TAKEOVER_SET_PIECE,
  WIPE_MS,
  createAnimationQueue,
  planEntries,
  scaleForSpeed,
  type AnimationEntry,
  type EntranceResolver,
} from "./animations.ts";
import { testid } from "./contract.ts";
import { createPlayTracker } from "./runs.ts";
import { baseView, emptySide, fullBoardView, unit, withEvents } from "../test/fixtures.ts";

const NETHER = "core-088";
const VOID = "core-100";
const DOOM_SHROOM = "classicplus-001";
const FLOOD = "core-017";
/** #34 Collateral Damage: a card with no set piece. */
const PLAIN = "core-034";

const resolve: EntranceResolver = (defId) => {
  const def = CATALOG[defId];
  return def === undefined ? null : { type: def.type as CardType, rarity: def.rarity };
};

const play = (defId: string, instanceId: string, player: PlayerId = "p1"): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 4 });
const marker = (defId: string, sourceId: string, controller: PlayerId = "p1"): GameEvent => ({ type: "hookResolving", sourceId, defId, controller, hook: "cry" });
const resolved = (defId: string, instanceId: string, player: PlayerId = "p1"): GameEvent => ({
  type: "cardResolved",
  player,
  instanceId,
  defId,
  permanent: false,
  costPaid: 4,
});
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
const exiled = (u: UnitView): GameEvent => ({ type: "exiled", instanceId: u.instanceId, defId: u.defId, owner: u.owner });

function unitsOf(view: PlayerView, side: "you" | "opponent"): UnitView[] {
  return view[side].units.flatMap((u) => (u === null ? [] : [u]));
}

/** Twisting Nether cast by p1 over `units`, as the engine streams it: the play, the marker, the deaths. */
function nether(units: readonly UnitView[], defId = NETHER): GameEvent[] {
  return [play(defId, "nx"), marker(defId, "nx"), ...units.map(destroyed), resolved(defId, "nx")];
}

const wipes = (entries: readonly AnimationEntry[]): AnimationEntry[] => entries.filter((entry) => entry.wipe !== undefined);

/** A queue whose timers run when the test says, naming the cards with a set piece as Game does. */
function queueOf(settings: { speed?: number; bigMoments?: FxBigMoments } = {}, burstBudgetMs = BURST_BUDGET_MS) {
  const timers: { ms: number; run: () => void }[] = [];
  const started: AnimationEntry[] = [];
  const queue = createAnimationQueue({
    reducedMotion: false,
    schedule: (run, ms) => {
      timers.push({ ms, run });
    },
    settings: () => ({ speed: settings.speed ?? 1, motion: "system", intensity: "normal", bigMoments: settings.bigMoments ?? "full" }),
    entranceOf: resolve,
    wipeOf: hasSetPiece,
    setPieceOf: wipeClaim,
    burstBudgetMs,
  });
  queue.subscribeSignals((signal) => {
    if (signal.kind === "start") started.push(signal.entry);
  });
  /** Runs every timer until the queue is idle, and returns the entries it started, in order. */
  const playAll = (): AnimationEntry[] => {
    for (let guard = 0; guard < 200 && timers.length > 0; guard += 1) timers.shift()?.run();
    return started;
  };
  return { queue, playAll };
}

describe("R2800000 a board clear's wipe is one entry", () => {
  it("R2800000 removals after the marker play as one pinned WIPE_MS entry", () => {
    const view = fullBoardView();
    const units = [...unitsOf(view, "you"), ...unitsOf(view, "opponent")];
    const entries = planEntries(nether(units), view, false, createPlayTracker(), hasSetPiece);
    const [wipe, ...others] = wipes(entries);
    expect(others).toEqual([]);
    expect(wipe?.events.map((e) => e.type)).toEqual(units.map(() => "destroyed"));
    expect(wipe?.durationMs).toBe(WIPE_MS);
    expect(wipe?.wipe).toEqual({ by: NETHER, sourceId: "nx", sides: ["you", "opponent"], anchor: null });
    // Every card it takes is marked on the board, as one removal's entry marks its own.
    for (const u of units) expect(wipe?.frames.get(testid.card(u.instanceId))).toBe("destroyed");

    // Pinned: a burst budget smaller than the wipe never squeezes it.
    const { queue, playAll } = queueOf({ bigMoments: "off" }, WIPE_MS / 4);
    queue.enqueue(nether(units), view);
    const held = playAll().find((entry) => entry.wipe !== undefined);
    expect(held?.durationMs).toBe(WIPE_MS);
  });

  it("R2800000 a hidden hook or a card without a set piece keeps one entry per removal", () => {
    const view = fullBoardView();
    const units = unitsOf(view, "opponent");
    const perRemoval = (events: readonly GameEvent[], wipeOf?: (defId: string) => boolean): number =>
      planEntries(events, view, false, createPlayTracker(), wipeOf).filter((entry) => entry.events.some((e) => e.type === "destroyed")).length;
    // R202: the other seat cannot read the hook's card, so no set piece keys on it.
    const hidden = [play("hidden", "hidden", "p2"), marker("hidden", "", "p2"), ...units.map(destroyed), resolved("hidden", "hidden", "p2")];
    expect(perRemoval(hidden, hasSetPiece)).toBe(units.length);
    expect(wipes(planEntries(hidden, view, false, createPlayTracker(), hasSetPiece))).toEqual([]);
    expect(perRemoval(nether(units, PLAIN), hasSetPiece)).toBe(units.length);
    // A caller that names no set pieces (the showcase, the replay GIF) plans as it always did.
    expect(perRemoval(nether(units))).toBe(units.length);
    // Removals with no marker before them (combat, a state check) are no wipe either.
    expect(perRemoval(units.map(destroyed), hasSetPiece)).toBe(units.length);
  });

  it("R2800000 a Trap's wipe is anchored on its zone", () => {
    const view = fullBoardView();
    const units = unitsOf(view, "you");
    // R154: Doom Shroom fires from the opponent's backrow lane 2 as the viewer's Unit attacks.
    const attacker = units[0];
    if (attacker === undefined) throw new Error("no attacker");
    const events: GameEvent[] = [
      { type: "attackDeclared", attackerId: attacker.instanceId, targetId: "hero-p2", forced: false },
      { type: "trapFired", instanceId: "shroom", defId: DOOM_SHROOM, controller: "p2", row: "backrow", lane: 2 },
      ...units.map(exiled),
    ];
    const [wipe] = wipes(planEntries(events, view, false, createPlayTracker(), hasSetPiece));
    expect(wipe?.wipe).toEqual({ by: DOOM_SHROOM, sourceId: "shroom", sides: ["you"], anchor: testid.zone("opponent", "backrow", 2) });
    expect(wipe?.events).toHaveLength(units.length);
  });

  it("R2800000 enemy-only names one side", () => {
    const view = fullBoardView();
    const [wipe] = wipes(planEntries(nether(unitsOf(view, "opponent")), view, false, createPlayTracker(), hasSetPiece));
    expect(wipe?.wipe?.sides).toEqual(["opponent"]);
    expect(wipe?.wipe?.anchor).toBeNull();
    // A removal the board does not show (a face-down card's) counts on its controller's side.
    const unseen: GameEvent = { type: "destroyed", instanceId: "gone", defId: "core-085", owner: "p1", controller: "p1", attack: 0, maxHealth: 0, killerId: null };
    const [both] = wipes(planEntries(nether([...unitsOf(view, "opponent")]).toSpliced(2, 0, unseen), view, false, createPlayTracker(), hasSetPiece));
    expect(both?.wipe?.sides).toEqual(["you", "opponent"]);
  });

  it("R2800000 a hook makes one wipe; a bounce's run carries each card's arrival in its hand", () => {
    const view = fullBoardView();
    const [first, second, ...rest] = unitsOf(view, "opponent");
    if (first === undefined || second === undefined) throw new Error("two units");
    // The marker's first run is the wipe; a later removal in the same hook plays as its own entry.
    const events = [
      play(NETHER, "nx"),
      marker(NETHER, "nx"),
      destroyed(first),
      { type: "enteredGraveyard", instanceId: first.instanceId, defId: first.defId, owner: "p2" } as GameEvent,
      destroyed(second),
      resolved(NETHER, "nx"),
    ];
    const entries = planEntries(events, view, false, createPlayTracker(), hasSetPiece);
    expect(wipes(entries).map((entry) => entry.events.length)).toEqual([1]);
    expect(entries.filter((entry) => entry.events.some((e) => e.type === "destroyed"))).toHaveLength(2);

    // Flood bounces every Unit: the engine streams each bounce with its `addedToHand`, the opponent's as R97's sentinel.
    const bounces = [first, second, ...rest].flatMap((u): GameEvent[] => [
      { type: "bounced", instanceId: "hidden", defId: "hidden", owner: u.owner },
      { type: "addedToHand", player: u.owner, instanceId: "hidden", defId: "hidden" },
    ]);
    const flood = planEntries([play(FLOOD, "fl"), marker(FLOOD, "fl"), ...bounces, resolved(FLOOD, "fl")], view, false, createPlayTracker(), hasSetPiece);
    const [wipe, ...more] = wipes(flood);
    expect(more).toEqual([]);
    expect(wipe?.events).toEqual(bounces);
    expect(wipe?.wipe?.sides).toEqual(["opponent"]);
  });

  it("R2800000 the queue holds it; short, off and speed scale it", () => {
    const view = fullBoardView();
    const units = [...unitsOf(view, "you"), ...unitsOf(view, "opponent")];
    const heldBy = (settings: { speed?: number; bigMoments?: FxBigMoments }): AnimationEntry | undefined => {
      const { queue, playAll } = queueOf(settings);
      queue.enqueue(nether(units), view);
      return playAll().find((entry) => entry.wipe !== undefined);
    };
    const full = heldBy({});
    expect(full?.setPiece).toEqual({ key: "nether", holdMs: SET_PIECES.nether.holdMs });
    expect(SET_PIECES.nether.holdMs).toBe(SET_PIECE_HOLD_MS);
    expect(full?.durationMs).toBe(WIPE_MS + SET_PIECE_HOLD_MS);
    expect(heldBy({ bigMoments: "short" })?.setPiece?.holdMs).toBe(SET_PIECE_SHORT_HOLD_MS);
    const off = heldBy({ bigMoments: "off" });
    expect(off?.setPiece).toBeUndefined();
    expect(off?.durationMs).toBe(WIPE_MS);
    const fast = heldBy({ speed: 2 });
    expect(fast?.setPiece?.holdMs).toBe(scaleForSpeed(SET_PIECE_HOLD_MS, 2));
    expect(fast?.durationMs).toBe(scaleForSpeed(WIPE_MS, 2) + scaleForSpeed(SET_PIECE_HOLD_MS, 2));
  });
});

describe("R2800000 R2722001 Ceaseless Void's fold and its takeover share one hold", () => {
  /** p1's board: three of its own Units and two of the opponent's, lane 3 empty for the Void. */
  function voidBoard(): { view: PlayerView; others: UnitView[] } {
    const mine = [unit("p1", { instanceId: "m1" }), unit("p1", { instanceId: "m2" }), null, unit("p1", { instanceId: "m4" }), null];
    const theirs = [unit("p2", { instanceId: "t1" }), null, null, null, unit("p2", { instanceId: "t5" })];
    const view = baseView({ you: emptySide("p1", { units: mine }), opponent: emptySide("p2", { units: theirs }) });
    return { view, others: [...mine, ...theirs].flatMap((u) => (u === null ? [] : [u])) };
  }

  const theVoid = (exiles: readonly GameEvent[]): GameEvent[] => [
    play(VOID, "void"),
    { type: "summoned", player: "p1", instanceId: "void", defId: VOID, row: "units", lane: 3 },
    marker(VOID, "void"),
    ...exiles,
    resolved(VOID, "void"),
  ];

  it("R2800000 R2722001 the Void yields: one hold, on voidFold", () => {
    const { view, others } = voidBoard();
    const { queue, playAll } = queueOf();
    queue.enqueue(theVoid(others.map(exiled)), withEvents(view, []));
    const started = playAll();
    const landing = started.find((entry) => entry.entrance !== undefined);
    expect(landing?.entrance).toMatchObject({ tier: "mythic", instanceId: "void", yielded: true });
    expect(landing?.setPiece).toBeUndefined();
    const fold = started.find((entry) => entry.wipe !== undefined);
    expect(fold?.setPiece).toEqual({ key: "voidFold", holdMs: VOID_FOLD_HOLD_MS });
    expect(fold?.wipe).toEqual({ by: VOID, sourceId: "void", sides: ["you", "opponent"], anchor: testid.zone("you", "units", 3) });
    expect(started.filter((entry) => entry.setPiece !== undefined)).toHaveLength(1);
  });

  it("R2800000 R2722001 with nothing to exile the Void keeps its takeover", () => {
    const { view } = voidBoard();
    const { queue, playAll } = queueOf();
    queue.enqueue(theVoid([]), view);
    const started = playAll();
    const landing = started.find((entry) => entry.entrance !== undefined);
    expect(landing?.entrance?.yielded).toBeUndefined();
    expect(landing?.setPiece).toEqual({ key: TAKEOVER_SET_PIECE, holdMs: TAKEOVER_HOLD_MS });
    expect(started.some((entry) => entry.wipe !== undefined)).toBe(false);
  });
});
