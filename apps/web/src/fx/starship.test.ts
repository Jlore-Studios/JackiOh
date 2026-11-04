// R2801000: the Starship's moment as the effects layer plans it (starship.ts). A fuse run flies each
// dead piece's card back to the Starship a stagger apart and bolts it on; a launch counts down through
// its hold, ignites, lifts off with a contrail and lands, its exhaust, contrail and shake grown with its
// pieces (`STARSHIP_TUNING`). Everything lands inside its entry and is gone within FX_MAX_TAIL_MS of it
// (R200); no cue names a card (R202). The cards that touch the Starship play their motifs in Tech's
// rocket look: Launch Sequencer's tithe glints on the launch's price, Retrofitting Program's send flies a
// card to the hand, and each Launcher lands in a conjure.

import type { GameEvent, PlayerId, PlayerView, UnitView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import {
  ANIMATIONS,
  SET_PIECE_HOLD_CAP_MS,
  SET_PIECE_SHORT_HOLD_MS,
  STARSHIP_FUSE_MS,
  STARSHIP_LAUNCH_HOLD_MS,
  STARSHIP_SET_PIECE,
  animTestid,
  planEntries,
  setPieceEntries,
  starshipFuseMs,
  type AnimationEntry,
} from "../game/animations.ts";
import { testid } from "../game/contract.ts";
import { createPlayTracker } from "../game/runs.ts";
import { baseView, dormantStarship, emptySide, unit } from "../test/fixtures.ts";
import { planCardMotif } from "./cardFx.ts";
import { FX_CARD_TABLE } from "./cards/tables.ts";
import {
  FX_INTENSITY_SCALE,
  FX_MAX_PARTICLE_LIFE_MS,
  FX_MAX_TAIL_MS,
  FX_STARSHIP_COUNT_AT,
  FX_STARSHIP_FUSE_LAND_AT,
  FX_STARSHIP_FUSE_STAGGER_MS,
  FX_STARSHIP_FUSE_TRAUMA,
  FX_STARSHIP_IGNITE_AT,
  FX_STARSHIP_STILL_MS,
  FX_STARSHIP_TAIL_MS,
} from "./constants.ts";
import { planFx } from "./cues.ts";
import { FLAVOR_SUB_LOOKS } from "./flavors.ts";
import { createFxMemory } from "./memory.ts";
import { STARSHIP_TUNING, dormantStarshipOf, planStarship, planStarshipStill, starshipClaim, starshipSizeOf } from "./starship.ts";
import type { FxBurstCue, FxCue, FxLaunchCue, FxPlanEnv, FxProjectileCue } from "./types.ts";

const HULL = "stinkyscape-t-dormant-starship";
const LAUNCHERS = ["stinkyscape-140", "stinkyscape-141", "stinkyscape-142", "stinkyscape-143"] as const;
const RETROFIT = "stinkyscape-130";
const SEQUENCER = "stinkyscape-t-ai-05";
const ROCKET = FLAVOR_SUB_LOOKS.Tech?.rocket?.colors;

const env = (intensity: number = FX_INTENSITY_SCALE.normal): FxPlanEnv => ({ intensity, card: () => undefined, memory: createFxMemory() });

function endOf(cue: FxCue): number {
  switch (cue.kind) {
    case "burst":
      return cue.delayMs + FX_MAX_PARTICLE_LIFE_MS;
    case "projectile":
      return cue.delayMs + cue.flightMs + FX_MAX_PARTICLE_LIFE_MS;
    case "shake":
      return cue.delayMs;
    default:
      return cue.delayMs + ("durationMs" in cue ? cue.durationMs : 0);
  }
}

/** p1's board: `pieces` on its units from lane 1, and its Dormant Starship in lane 5. */
function board(pieces: readonly UnitView[] = []): PlayerView {
  const units: (UnitView | null)[] = [null, null, null, null, dormantStarship("p1", ["core-008"], { instanceId: "ship" })];
  pieces.forEach((piece, k) => {
    units[k] = piece;
  });
  return baseView({ you: emptySide("p1", { units }), opponent: emptySide("p2") });
}

const piece = (instanceId: string, defId: string = LAUNCHERS[0]): UnitView => unit("p1", { instanceId, defId, attack: 0 });

function fusionsOf(pieces: readonly UnitView[]): GameEvent[] {
  return pieces.flatMap((p, k): GameEvent[] => [
    { type: "fused", instanceIds: ["ship", `${p.instanceId}-copy`], resultInstanceId: "ship", defId: HULL },
    { type: "starshipFused", player: "p1", instanceId: "ship", pieceId: p.instanceId, defId: p.defId, pieces: k + 2 },
  ]);
}

function fuseEntry(pieces: readonly UnitView[], view: PlayerView = board(pieces)): AnimationEntry {
  const found = planEntries(fusionsOf(pieces), view, false, createPlayTracker()).find((entry) => entry.fuse !== undefined);
  if (found === undefined) throw new Error("no fuse run");
  return found;
}

const launched = (pieces: number, instanceId = "ship"): GameEvent => ({
  type: "starshipLaunched",
  player: "p1",
  instanceId,
  defId: HULL,
  name: "Ember of Departure",
  pieces: Array.from({ length: pieces }, (_, k) => LAUNCHERS[k % LAUNCHERS.length] ?? HULL),
});

/** The launch entry of a Starship of `pieces` pieces, held at `holdMs` (none: unheld). */
function launchEntry(pieces: number, holdMs?: number, view: PlayerView = board()): AnimationEntry {
  const planned = planEntries([launched(pieces)], view, false, createPlayTracker());
  const held =
    holdMs === undefined
      ? planned
      : setPieceEntries(planned, (entry) => (starshipClaim(entry) === null ? null : { key: STARSHIP_SET_PIECE, holdMs }), { reduced: false, bigMoments: "full", speed: 1 });
  const found = held.find((entry) => entry.launch !== undefined);
  if (found === undefined) throw new Error("no launch");
  return found;
}

const launchOf = (cues: readonly FxCue[]): FxLaunchCue | undefined => cues.find((c): c is FxLaunchCue => c.kind === "launch");
const exhaustOf = (cues: readonly FxCue[]): FxBurstCue[] => cues.filter((c): c is FxBurstCue => c.kind === "burst" && c.preset === "exhaust");
const contrailOf = (cues: readonly FxCue[]): FxProjectileCue | undefined => cues.find((c): c is FxProjectileCue => c.kind === "projectile");
const shakesOf = (cues: readonly FxCue[]): { trauma: number; delayMs: number }[] =>
  cues.flatMap((c) => (c.kind === "shake" ? [{ trauma: c.trauma, delayMs: c.delayMs }] : []));

describe("R2801000 the fuse run", () => {
  it("R2801000 fuse ghosts land FX_STARSHIP_FUSE_STAGGER_MS apart with spark, ring, shake", () => {
    const pieces = [piece("a"), piece("b", LAUNCHERS[1]), piece("c", LAUNCHERS[2])];
    const view = board(pieces);
    const entry = fuseEntry(pieces, view);
    expect(entry.durationMs).toBe(starshipFuseMs(3));
    const plan = planStarship(entry, view, env());
    const cues = plan?.cues ?? [];
    const ship = { kind: "testid", testid: testid.card("ship") };
    const ghosts = cues.filter((c) => c.kind === "ghost");
    const flight = Math.round(STARSHIP_FUSE_MS * FX_STARSHIP_FUSE_LAND_AT);
    expect(ghosts).toEqual(
      pieces.map((p, k) => ({ kind: "ghost", from: { kind: "testid", testid: testid.card(p.instanceId) }, to: ship, delayMs: k * FX_STARSHIP_FUSE_STAGGER_MS, durationMs: flight })),
    );
    for (let k = 0; k < pieces.length; k += 1) {
      const land = k * FX_STARSHIP_FUSE_STAGGER_MS + flight;
      const at = cues.filter((c) => c.delayMs === land).map((c) => (c.kind === "burst" ? `burst:${c.preset}` : c.kind));
      expect(at, `piece ${String(k)}`).toEqual(["burst:spark", "ring", "shake"]);
    }
    expect(shakesOf(cues).map((s) => s.trauma)).toEqual(pieces.map(() => FX_STARSHIP_FUSE_TRAUMA));
    expect(cues.filter((c): c is FxBurstCue => c.kind === "burst").every((c) => c.colors === ROCKET)).toBe(true);
    // It draws the fusions in place of their rows: planFx adds no transmute of the `fused`.
    expect(plan?.replaces).toEqual(new Set(entry.events));
    expect(planFx(entry, view, env())).toEqual(cues);
    // R2791000's Equip beat stays off a fuse run; an Equip fused onto the Starship from hand keeps it.
    expect(cues.some((c) => c.kind === "callout")).toBe(false);
    const equip: GameEvent = { type: "fused", instanceIds: ["ship", "from-hand"], resultInstanceId: "ship", defId: HULL };
    const [equipped] = planEntries([equip], view, false, createPlayTracker());
    expect(equipped?.fuse).toBeUndefined();
    expect(equipped === undefined ? [] : planFx(equipped, view, env()).filter((c) => c.kind === "callout")).toMatchObject([{ keyword: "Equip" }]);

    // A piece the board does not show flies from its owner's graveyard; the budget's squeeze keeps the shares.
    const gone = board([]);
    const squeezed = { ...fuseEntry(pieces, gone), durationMs: Math.round(starshipFuseMs(3) / 2) };
    const fromGrave = (planStarship(squeezed, gone, env())?.cues ?? []).filter((c) => c.kind === "ghost");
    expect(fromGrave.map((c) => (c.kind === "ghost" ? c.from : null))).toEqual(pieces.map(() => ({ kind: "testid", testid: animTestid.graveyard("you") })));
    expect(fromGrave.map((c) => c.delayMs)).toEqual(pieces.map((_, k) => Math.round(k * FX_STARSHIP_FUSE_STAGGER_MS * 0.5)));
  });
});

describe("R2801000 the launch", () => {
  it("R2801000 the launch at 1, 4 and 8 pieces grows in exhaust count, contrail reach and trauma", () => {
    expect([1, 4, 8].map(starshipSizeOf)).toEqual(["small", "medium", "large"]);
    expect(starshipSizeOf(0)).toBe("small");
    const plans = [1, 4, 8].map((n) => planStarship(launchEntry(n, STARSHIP_LAUNCH_HOLD_MS), board(), env())?.cues ?? []);
    const counts = plans.map((cues) => Math.max(...exhaustOf(cues).map((c) => c.count)));
    const reach = plans.map((cues) => {
      const to = contrailOf(cues)?.to;
      return to?.kind === "testid" ? -(to.at?.y ?? 0) : 0;
    });
    const trauma = plans.map((cues) => Math.max(...shakesOf(cues).map((s) => s.trauma)));
    const density = plans.map((cues) => contrailOf(cues)?.density ?? 0);
    expect(counts).toEqual([STARSHIP_TUNING.small.exhaust.count, STARSHIP_TUNING.medium.exhaust.count, STARSHIP_TUNING.large.exhaust.count]);
    expect(reach).toEqual([STARSHIP_TUNING.small.contrail, STARSHIP_TUNING.medium.contrail, STARSHIP_TUNING.large.contrail]);
    expect(trauma).toEqual([STARSHIP_TUNING.small.trauma, STARSHIP_TUNING.medium.trauma, STARSHIP_TUNING.large.trauma]);
    for (const series of [counts, reach, trauma, density]) {
      expect(series[0]).toBeLessThan(series[1] ?? 0);
      expect(series[1]).toBeLessThan(series[2] ?? 0);
    }
    expect(plans.map((cues) => launchOf(cues)?.size)).toEqual(["small", "medium", "large"]);
  });

  it("R2801000 held: three counts, each shake harder; unheld: none", () => {
    const H = STARSHIP_LAUNCH_HOLD_MS;
    const entry = launchEntry(4, H);
    const D = entry.durationMs;
    expect(D).toBe(ANIMATIONS.starshipLaunched.durationMs + H);
    const cues = planStarship(entry, board(), env())?.cues ?? [];
    const launch = launchOf(cues);
    const countsMs = FX_STARSHIP_COUNT_AT.map((at) => Math.round(at * H));
    expect(launch).toEqual({
      kind: "launch",
      at: { kind: "testid", testid: testid.card("ship") },
      tint: { glow: ROCKET?.[0], rim: ROCKET?.[1], core: ROCKET?.[2] },
      size: "medium",
      countsMs,
      igniteMs: Math.round(FX_STARSHIP_IGNITE_AT * H),
      delayMs: 0,
      durationMs: D + FX_STARSHIP_TAIL_MS,
    });
    const onCounts = shakesOf(cues).filter((s) => s.delayMs < H);
    expect(onCounts.map((s) => s.delayMs)).toEqual(countsMs);
    const tune = STARSHIP_TUNING.medium.trauma;
    expect(onCounts.map((s) => s.trauma)).toEqual(countsMs.map((_, k) => (tune * (k + 1)) / countsMs.length));
    // The ignition's exhaust at its time, the lift-off's as the hold ends, the contrail from it.
    expect(exhaustOf(cues).map((c) => c.delayMs)).toEqual([launch?.igniteMs, H]);
    expect(contrailOf(cues)?.delayMs).toBe(H);
    // The Dormant badge burns away at the lift-off.
    expect(cues.some((c) => c.kind === "burst" && c.preset === "ember" && c.delayMs === H)).toBe(true);
    expect(planFx(entry, board(), env())).toEqual(cues);

    const unheld = planStarship(launchEntry(4), board(), env())?.cues ?? [];
    expect(launchOf(unheld)?.countsMs).toEqual([]);
    expect(contrailOf(unheld)?.delayMs).toBe(0);
    expect(shakesOf(unheld).every((s) => s.trauma === tune)).toBe(true);
  });

  it("R2801000 R200 every cue ends within FX_MAX_TAIL_MS", () => {
    for (const holdMs of [undefined, SET_PIECE_SHORT_HOLD_MS, STARSHIP_LAUNCH_HOLD_MS, SET_PIECE_HOLD_CAP_MS]) {
      for (const n of [1, 4, 8]) {
        const entry = launchEntry(n, holdMs);
        const D = entry.durationMs;
        const cues = planStarship(entry, board(), env(FX_INTENSITY_SCALE.high))?.cues ?? [];
        expect(cues.length).toBeGreaterThan(0);
        for (const cue of cues) {
          const where = `launch of ${String(n)} held ${String(holdMs)}: ${cue.kind}`;
          expect(cue.delayMs, where).toBeGreaterThanOrEqual(0);
          expect(cue.delayMs, where).toBeLessThanOrEqual(D);
          expect(endOf(cue), where).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
        }
      }
    }
    for (const n of [1, 3, 9]) {
      const pieces = Array.from({ length: n }, (_, k) => piece(`p${String(k)}`));
      const entry = fuseEntry(pieces);
      const D = entry.durationMs;
      for (const cue of planStarship(entry, board(pieces), env())?.cues ?? []) {
        expect(cue.delayMs).toBeLessThanOrEqual(D);
        if (cue.kind === "ghost") expect(cue.delayMs + cue.durationMs).toBeLessThanOrEqual(D);
        expect(endOf(cue)).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
      }
    }
  });

  it("R2801000 R202 no cue names a defId; intensity 0 plans nothing", () => {
    const pieces = [piece("a"), piece("b", LAUNCHERS[3])];
    const all = [
      ...(planStarship(fuseEntry(pieces), board(pieces), env())?.cues ?? []),
      ...(planStarship(launchEntry(8, STARSHIP_LAUNCH_HOLD_MS), board(), env())?.cues ?? []),
      ...planStarshipStill(launchEntry(8), board(), env()),
    ];
    const text = JSON.stringify(all);
    for (const id of [HULL, ...LAUNCHERS, "core-008", "Ember of Departure"]) expect(text).not.toContain(id);
    expect(planStarship(launchEntry(8, STARSHIP_LAUNCH_HOLD_MS), board(), env(0))).toBeNull();
    expect(planStarship(fuseEntry(pieces), board(pieces), env(0))).toBeNull();
    expect(planFx(launchEntry(8, STARSHIP_LAUNCH_HOLD_MS), board(), env(0))).toEqual([]);
    expect(planStarshipStill(launchEntry(8), board(), env(0))).toEqual([]);
    // A Starship the seat cannot place plans nothing; its launch's own row plays.
    const hidden = planEntries([launched(3, "hidden")], board(), false, createPlayTracker())[0];
    expect(hidden === undefined ? "none" : planStarship(hidden, board(), env())).toBeNull();
    // Reduce motion: an entry of no duration draws no motion.
    const still = planEntries([launched(3)], board(), true, createPlayTracker())[0];
    expect(still === undefined ? "none" : planStarship(still, board(), env())).toBeNull();
  });

  it("R2801000 planStarshipStill is one still cue", () => {
    const still = planEntries([launched(8)], board(), true, createPlayTracker())[0];
    if (still === undefined) throw new Error("no entry");
    expect(planStarshipStill(still, board(), env())).toEqual([
      {
        kind: "launch",
        at: { kind: "testid", testid: testid.card("ship") },
        tint: { glow: ROCKET?.[0], rim: ROCKET?.[1], core: ROCKET?.[2] },
        size: "large",
        countsMs: [],
        igniteMs: 0,
        delayMs: 0,
        durationMs: FX_STARSHIP_STILL_MS,
        still: true,
      },
    ]);
    // Anything but a launch has no still frame.
    expect(planStarshipStill(fuseEntry([piece("a")]), board([piece("a")]), env())).toEqual([]);
  });
});

describe("R2801000 the cards that touch the Starship", () => {
  const played = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 1 });

  /** The entry of `events` as the board plans it while `defId`'s play resolves. */
  function asPlay(defId: string, events: GameEvent[], view: PlayerView) {
    const e = env();
    e.memory.remember([played("p1", "play", defId), ...events]);
    const entry: AnimationEntry = { events, type: events[0]?.type ?? "cardPlayed", durationMs: 400, frames: new Map(), view };
    return planCardMotif(entry, view, e);
  }

  it("R2801000 each table entry names its motif in the rocket's look", () => {
    expect(FX_CARD_TABLE.get(SEQUENCER)).toEqual({ kind: "hooks", hooks: { cast: { motif: "tithe", palette: "Tech/rocket" } } });
    expect(FX_CARD_TABLE.get(RETROFIT)).toEqual({ kind: "hooks", hooks: { cast: { motif: "send", palette: "Tech/rocket" } } });
    for (const id of LAUNCHERS) expect(FX_CARD_TABLE.get(id), id).toEqual({ kind: "hooks", hooks: { cry: { motif: "conjure", palette: "Tech/rocket" } } });
    expect(FX_CARD_TABLE.get(HULL)).toMatchObject({ kind: "none" });
  });

  it("R2801000 Launch Sequencer's tithe glints modifiers-you and the Starship", () => {
    const view = board();
    expect(dormantStarshipOf(view, "you")).toBe(testid.card("ship"));
    const priced: GameEvent = { type: "modifierChanged", player: "p1", modifierId: "m1", added: true };
    const motif = asPlay(SEQUENCER, [priced], view);
    const glints = (motif?.cues ?? []).filter((c): c is FxBurstCue => c.kind === "burst");
    expect(glints.map((c) => (c.at.kind === "testid" ? c.at.testid : c.at.kind))).toEqual([animTestid.modifiers("you"), testid.card("ship")]);
    expect(glints.every((c) => c.colors === ROCKET)).toBe(true);
    // The badge's own row still plays: tithe replaces no modifier.
    expect(motif?.replaces.size).toBe(0);
    // With no Dormant Starship yet (R1881003: the price waits), it glints on the badges alone.
    const empty = baseView({ you: emptySide("p1"), opponent: emptySide("p2") });
    expect(dormantStarshipOf(empty, "you")).toBeNull();
    const alone = (asPlay(SEQUENCER, [priced], empty)?.cues ?? []).filter((c) => c.kind === "burst");
    expect(alone).toHaveLength(1);
    // A modifier spent glints nothing.
    expect(asPlay(SEQUENCER, [{ ...priced, added: false }], view)).toBeNull();
  });

  it("R2801000 Retrofitting's send flies a card back from the hero to the hand in the rocket's exhaust", () => {
    const view = board();
    const added: GameEvent = { type: "addedToHand", player: "p1", instanceId: "jl", defId: "stinkyscape-002" };
    const cues = asPlay(RETROFIT, [added], view)?.cues ?? [];
    expect(cues.find((c) => c.kind === "ghost")).toMatchObject({
      from: { kind: "testid", testid: testid.hero("you") },
      to: { kind: "testid", testid: animTestid.hand("you") },
    });
    const bursts = cues.filter((c): c is FxBurstCue => c.kind === "burst");
    expect(bursts.map((c) => c.preset)).toEqual(["exhaust", "sparkle"]);
    expect(bursts.every((c) => c.colors === ROCKET)).toBe(true);
  });

  it("R2801000 each Launcher's conjure is exhaust", () => {
    for (const id of LAUNCHERS) {
      const view = board();
      const landed: GameEvent = { type: "summoned", player: "p1", instanceId: "play", defId: id, row: "units", lane: 1 };
      const cues = asPlay(id, [landed], view)?.cues ?? [];
      const bursts = cues.filter((c): c is FxBurstCue => c.kind === "burst");
      expect(bursts.length, id).toBeGreaterThan(0);
      expect(bursts.every((c) => c.preset === "exhaust" && c.colors === ROCKET), id).toBe(true);
      expect(bursts.every((c) => c.at.kind === "testid" && c.at.testid === testid.zone("you", "units", 1)), id).toBe(true);
      expect(cues.some((c) => c.kind === "ring" && c.preset === "exhaust"), id).toBe(true);
    }
  });
});
