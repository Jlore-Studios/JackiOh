// R3334000–R3334003: the Starship's moments as the effects layer plans them (starship.ts). A fused piece's
// module docks on its own hardpoint, clamped and welded, humming higher as the count grows; a held launch
// at Big moments "full" is a cinematic over the ship's own picture, else a still title card; a War
// Doctrine is announced over the ship; a launched Starship's attack is a broadside by each weapon's kind,
// or a ram; and its death is a wreck. Only public events drive it (R202), each inside its entry and gone
// within FX_MAX_TAIL_MS of it (R200).

import type { GameEvent, PlayerView, UnitView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { starshipArtOf } from "../cards/art/starshipHulls.ts";
import { starshipMountPoints } from "../cards/art/stinkyscape.ts";
import {
  ANIMATIONS,
  SET_PIECE_HOLD_CAP_MS,
  SET_PIECE_SHORT_HOLD_MS,
  STARSHIP_LAUNCH_HOLD_MS,
  STARSHIP_SET_PIECE,
  planEntries,
  setPieceEntries,
  type AnimationEntry,
} from "../game/animations.ts";
import { testid } from "../game/contract.ts";
import { createPlayTracker } from "../game/runs.ts";
import { baseView, dormantStarship, emptySide, unit } from "../test/fixtures.ts";
import { FX_INTENSITY_SCALE, FX_LUNGE_CONTACT_AT, FX_MAX_PARTICLE_LIFE_MS, FX_MAX_TAIL_MS, FX_STARSHIP_TAIL_MS } from "./constants.ts";
import { planFx } from "./cues.ts";
import { createFxMemory } from "./memory.ts";
import { planStage } from "./stage.ts";
import {
  BROADSIDE_WEAPONS,
  STARSHIP_BROADSIDE,
  STARSHIP_CINEMATIC,
  STARSHIP_DOCK,
  STARSHIP_LETTERING,
  STARSHIP_TUNING,
  STARSHIP_WRECK,
  WAR_DOCTRINES,
  WAR_DOCTRINE_LOOKS,
  broadsideFireMs,
  cameraPoint,
  planStarship,
  starshipClaim,
  starshipFuseLandMs,
} from "./starship.ts";
import type { FxCinematicCue, FxConcealCue, FxCue, FxModuleCue, FxPlanEnv, FxShotCue, FxWeldCue } from "./types.ts";

const HULL = "stinkyscape-t-dormant-starship";
const BEAM = "stinkyscape-050";
const REPEATER = "stinkyscape-051";
const CANNON = "stinkyscape-052";
const FLAK = "stinkyscape-053";
const RUSH = "stinkyscape-140";
const FRUIT = "stinkyscape-141";
const JADE = "stinkyscape-142";
const BOOK = "stinkyscape-143";
const ENGINE = "stinkyscape-090";
const SHIELD = "stinkyscape-038";
const DRILL = "stinkyscape-062";
const NAME = "The Ninth Life";

const env = (over: Partial<FxPlanEnv> = {}): FxPlanEnv => ({ intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory(), ...over });

/** A ship of `n` pieces, cycling through a mix of hardpoints. */
const MIX = [BEAM, RUSH, ENGINE, SHIELD, CANNON, FRUIT, DRILL, FLAK] as const;
const piecesOf = (n: number): string[] => Array.from({ length: n }, (_, k) => MIX[k % MIX.length] ?? BEAM);

function endOf(cue: FxCue): number {
  switch (cue.kind) {
    case "burst":
      return cue.delayMs + FX_MAX_PARTICLE_LIFE_MS;
    case "projectile":
    case "shot":
      return cue.delayMs + cue.flightMs + FX_MAX_PARTICLE_LIFE_MS;
    case "shake":
      return cue.delayMs;
    case "conceal":
      // The stage's: the board's next view ends it, FX_HOLD_MAX_MS at the latest (R200); it paints nothing.
      return cue.delayMs;
    default:
      return cue.delayMs + ("durationMs" in cue ? cue.durationMs : 0);
  }
}

function expectBounded(cues: readonly FxCue[], D: number, where: string): void {
  expect(cues.length, where).toBeGreaterThan(0);
  for (const cue of cues) {
    expect(cue.delayMs, `${where}: ${cue.kind}`).toBeGreaterThanOrEqual(0);
    expect(cue.delayMs, `${where}: ${cue.kind}`).toBeLessThanOrEqual(D);
    expect(endOf(cue), `${where}: ${cue.kind}`).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
  }
}

/** p1's board: `pieces` on its units from lane 1, and its Dormant Starship of `held` in lane 5. */
function dormantBoard(held: readonly string[], pieces: readonly UnitView[] = []): PlayerView {
  const units: (UnitView | null)[] = [null, null, null, null, dormantStarship("p1", held, { instanceId: "ship" })];
  pieces.forEach((p, k) => {
    units[k] = p;
  });
  return baseView({ you: emptySide("p1", { units }), opponent: emptySide("p2", { units: [null, null, null, null, unit("p2", { instanceId: "foe" })] }) });
}

/** p1's launched Starship of `held` in lane 3, facing p2's unit `foe`. */
function launchedBoard(held: readonly string[], over: Partial<UnitView> = {}): PlayerView {
  const ship = unit("p1", { instanceId: "ship", defId: HULL, starship: { pieces: held.map((defId) => ({ defId })), name: NAME, launched: 1 }, ...over });
  return baseView({
    you: emptySide("p1", { units: [null, null, ship, null, null] }),
    opponent: emptySide("p2", { units: [null, null, unit("p2", { instanceId: "foe" }), null, null] }),
  });
}

const fusions = (held: number, defIds: readonly string[], ids: readonly string[]): GameEvent[] =>
  defIds.flatMap((defId, k): GameEvent[] => [
    { type: "fused", instanceIds: ["ship", `${ids[k] ?? ""}-copy`], resultInstanceId: "ship", defId: HULL },
    { type: "starshipFused", player: "p1", instanceId: "ship", pieceId: ids[k] ?? "", defId, pieces: held + k + 1 },
  ]);

function entryOf(events: GameEvent[], view: PlayerView, pick: (entry: AnimationEntry) => boolean): AnimationEntry {
  const found = planEntries(events, view, false, createPlayTracker()).find(pick);
  if (found === undefined) throw new Error("no entry");
  return found;
}

const launchEvent = (pieces: readonly string[], instanceId = "ship"): GameEvent => ({ type: "starshipLaunched", player: "p1", instanceId, defId: HULL, name: NAME, pieces: [...pieces] });

function launchEntry(pieces: readonly string[], view: PlayerView, bigMoments: "full" | "short" | "off" | null): AnimationEntry {
  const planned = planEntries([launchEvent(pieces)], view, false, createPlayTracker());
  const held = bigMoments === null ? planned : setPieceEntries(planned, starshipClaim, { reduced: false, bigMoments, speed: 1 });
  const found = held.find((entry) => entry.launch !== undefined);
  if (found === undefined) throw new Error("no launch");
  return found;
}

const attack = (targetId = "foe"): GameEvent => ({ type: "attackDeclared", attackerId: "ship", targetId, forced: false });
const cinematicOf = (cues: readonly FxCue[]): FxCinematicCue | undefined => cues.find((c): c is FxCinematicCue => c.kind === "cinematic");
const shotsOf = (cues: readonly FxCue[]): FxShotCue[] => cues.filter((c): c is FxShotCue => c.kind === "shot");
/**
 * R3334001: on a phone every burst of a moment throws `phoneShare` as many particles, never none. Each burst's
 * count is `max(1, round(tuning·i))` at full size, so the phone's is at most half of it, rounded up: a burst left
 * at full size (of 2 or more) fails.
 */
const expectPhoneShare = (cues: readonly FxCue[], compact: readonly FxCue[], label: string): void => {
  const counts = (of: readonly FxCue[]) => of.flatMap((c) => (c.kind === "burst" ? [c.count] : []));
  const [full, phone] = [counts(cues), counts(compact)];
  expect(phone, label).toHaveLength(full.length);
  expect(full.length, label).toBeGreaterThan(0);
  full.forEach((n, k) => expect(phone[k], `${label}: burst ${String(k)} of ${String(n)}`).toBeLessThanOrEqual(Math.max(1, Math.ceil(n * STARSHIP_CINEMATIC.phoneShare))));
};

describe("R3334000 docking", () => {
  it("R3334000 fuses of 1 and 3 dock in order at their hardpoints, weld, hum rising", () => {
    for (const [held, fused] of [[[BEAM, RUSH], [CANNON]], [[BEAM], [REPEATER, FRUIT, ENGINE]]] as const) {
      const pieces = fused.map((defId, k) => unit("p1", { instanceId: `piece-${String(k)}`, defId, attack: 0 }));
      const view = dormantBoard(held, pieces);
      const events = fusions(held.length, fused, pieces.map((p) => p.instanceId));
      const entry = entryOf(events, view, (e) => e.fuse !== undefined);
      const D = entry.durationMs;
      const cues = planStarship(entry, view, env())?.cues ?? [];
      const docks = cues.filter((c): c is FxModuleCue => c.kind === "module");
      const welds = cues.filter((c): c is FxWeldCue => c.kind === "weld");
      expect(docks).toHaveLength(fused.length);
      expect(welds).toHaveLength(fused.length);
      // Each piece's module flies from where it died to its own hardpoint of the ship's final picture.
      const mounts = starshipMountPoints(starshipArtOf([...held, ...fused].map((defId) => ({ defId, radiant: false })), false, undefined, false));
      docks.forEach((dock, k) => {
        expect(dock.motion).toBe("dock");
        expect(dock.from).toEqual({ kind: "testid", testid: testid.card(`piece-${String(k)}`) });
        const point = mounts[held.length + k];
        expect(dock.to).toEqual({ kind: "testid", testid: testid.card("ship"), at: { x: (point?.x ?? 0) / 100, y: (point?.y ?? 0) / 100 }, art: "picture" });
        expect(dock.share).toBe(STARSHIP_DOCK.moduleShare);
        expect(dock.uri).toMatch(/^data:image\/svg\+xml/);
        // In order: each leaves after the one before and lands as its weld starts.
        const land = starshipFuseLandMs(k, fused.length, D);
        expect(dock.delayMs + dock.landMs).toBe(land);
        expect(welds[k]?.delayMs).toBe(land);
        expect(welds[k]?.at).toEqual(dock.to);
        if (k > 0) expect(dock.delayMs).toBeGreaterThan(docks[k - 1]?.delayMs ?? 0);
      });
      // The hum rises with the count, full at STARSHIP_DOCK.humFull pieces.
      const hums = welds.map((w) => w.hum);
      expect(hums).toEqual(fused.map((_, k) => Math.min(1, (held.length + k + 1) / STARSHIP_DOCK.humFull)));
      for (let k = 1; k < hums.length; k += 1) expect(hums[k]).toBeGreaterThan(hums[k - 1] ?? 1);
      expectBounded(cues, D, `fuse of ${String(fused.length)}`);
      // On a phone: the same docks, every spark burst at its share.
      const compact = planStarship(entry, view, env({ compact: true }))?.cues ?? [];
      expect(compact.map((c) => `${c.kind}@${String(c.delayMs)}`)).toEqual(cues.map((c) => `${c.kind}@${String(c.delayMs)}`));
      expectPhoneShare(cues, compact, `fuse of ${String(fused.length)}`);
    }
  });

  it("R3334000 two fuse runs of one action dock on the hardpoints of the list each run leaves", () => {
    // Pieces B and C die in two death passes of one action, a damage between them: two fuse runs, both
    // planned on the board before the action (the ship holds [A]), the board after it holding [A, B, C].
    const held = [BEAM] as const;
    const fused = [CANNON, FRUIT] as const;
    const pieces = fused.map((defId, k) => unit("p1", { instanceId: `piece-${String(k)}`, defId, attack: 0 }));
    const view = dormantBoard(held, pieces);
    const after = dormantBoard([...held, ...fused]);
    const [first, second] = [fusions(1, [CANNON], ["piece-0"]), fusions(2, [FRUIT], ["piece-1"])];
    const events: GameEvent[] = [...(first ?? []), { type: "damage", sourceId: null, targetId: "foe", amount: 1, combat: false }, ...(second ?? [])];
    const runs = planEntries(events, view, false, createPlayTracker()).filter((e) => e.fuse !== undefined);
    expect(runs).toHaveLength(2);
    runs.forEach((entry, k) => {
      expect(entry.view).toBe(view);
      const cues = planStarship(entry, entry.view, env({ next: after }))?.cues ?? [];
      const dock = cues.find((c): c is FxModuleCue => c.kind === "module");
      const weld = cues.find((c): c is FxWeldCue => c.kind === "weld");
      // Run k leaves the ship holding A and the first k + 1 fused pieces; its piece docks on the last of them.
      const list = [...held, ...fused.slice(0, k + 1)];
      const point = starshipMountPoints(starshipArtOf(list.map((defId) => ({ defId, radiant: false })), false, undefined, false))[list.length - 1];
      expect(dock?.to, `run ${String(k)}`).toEqual({ kind: "testid", testid: testid.card("ship"), at: { x: (point?.x ?? 0) / 100, y: (point?.y ?? 0) / 100 }, art: "picture" });
      // The picture agrees with the hum (and the dock's sound): both count the pieces after the run.
      expect(weld?.hum).toBe(Math.min(1, list.length / STARSHIP_DOCK.humFull));
    });
  });

  it("R3334000 R202 no defId; hidden ship plans nothing", () => {
    const pieces = [unit("p1", { instanceId: "piece-0", defId: JADE, attack: 0 })];
    const view = dormantBoard([BEAM], pieces);
    const entry = entryOf(fusions(1, [JADE], ["piece-0"]), view, (e) => e.fuse !== undefined);
    const text = JSON.stringify(planStarship(entry, view, env())?.cues ?? []);
    for (const id of [HULL, BEAM, JADE, NAME]) expect(text).not.toContain(id);
    // A piece the viewer may not read (R97) docks as the spare plate; a ship it may not, plans nothing.
    const unread = entryOf(
      [{ type: "fused", instanceIds: ["ship", "x"], resultInstanceId: "ship", defId: HULL }, { type: "starshipFused", player: "p1", instanceId: "ship", pieceId: "hidden", defId: "hidden", pieces: 2 }],
      view,
      (e) => e.fuse !== undefined,
    );
    expect((planStarship(unread, view, env())?.cues ?? []).filter((c) => c.kind === "module")).toHaveLength(1);
    const hidden = planEntries([{ type: "starshipFused", player: "p1", instanceId: "hidden", pieceId: "piece-0", defId: JADE, pieces: 2 }], view, false, createPlayTracker()).find(
      (e) => e.fuse !== undefined,
    );
    expect(hidden === undefined ? null : planStarship(hidden, view, env())).toBeNull();
    // The broadside and the wreck of a ship the view does not show plan nothing either.
    const nothing = launchedBoard([BEAM]);
    expect(planStarship(entryOf([{ ...attack(), attackerId: "hidden" } as GameEvent], nothing, () => true), nothing, env())).toBeNull();
  });
});

describe("R3334001 the launch", () => {
  it("R3334001 held full: one stop per module, engines in turn, the name, within SET_PIECE_HOLD_CAP_MS and FX_MAX_TAIL_MS", () => {
    const held = [BEAM, RUSH, ENGINE, SHIELD, CANNON, "stinkyscape-101"];
    const view = dormantBoard(held);
    const entry = launchEntry(held, view, "full");
    const H = entry.setPiece?.holdMs ?? 0;
    expect(entry.setPiece?.key).toBe(STARSHIP_SET_PIECE);
    expect(H).toBe(STARSHIP_LAUNCH_HOLD_MS);
    expect(H).toBeLessThanOrEqual(SET_PIECE_HOLD_CAP_MS);
    const D = entry.durationMs;
    const cues = planStarship(entry, view, env())?.cues ?? [];
    const film = cinematicOf(cues);
    if (film === undefined) throw new Error("no cinematic");
    expect(film.still).toBeUndefined();
    expect(film.name).toBe(NAME);
    expect(film.lettering).toEqual(STARSHIP_LETTERING["ninth-life"]);
    expect(film.art).toMatch(/^data:image\/svg\+xml/);
    expect(film.at).toEqual({ kind: "testid", testid: testid.card("ship") });
    // One stop per mounted module, each inside the pan, timed by how far along the hull it stands.
    expect(film.stops).toHaveLength(held.length);
    for (const stop of film.stops) {
      expect(stop.atMs).toBeGreaterThanOrEqual(film.panAtMs);
      expect(stop.atMs).toBeLessThanOrEqual(film.panEndMs);
    }
    const byX = [...film.stops].sort((a, b) => a.x - b.x);
    for (let k = 1; k < byX.length; k += 1) expect(byX[k]?.atMs).toBeGreaterThanOrEqual(byX[k - 1]?.atMs ?? 0);
    expect(film.cameraFrom.x).toBeLessThan(film.cameraTo.x);
    // The engines light one after another (two stern mounts: the After Burner and Overdrive), each shaking harder.
    expect(film.engines).toHaveLength(2);
    expect(film.engines.map((e) => e.atMs)).toEqual([Math.round(STARSHIP_CINEMATIC.enginesFrom * H), Math.round(STARSHIP_CINEMATIC.enginesTo * H)]);
    const shakes = cues.flatMap((c) => (c.kind === "shake" ? [c] : []));
    const onEngines = film.engines.map((e) => shakes.find((s) => s.delayMs === e.atMs)?.trauma ?? 0);
    expect(onEngines[0]).toBeLessThan(onEngines[1] ?? 0);
    // Then the name stamps and the ship climbs out, all inside the hold, the cue gone soon after it.
    expect(film.stampMs).toBe(Math.round(STARSHIP_CINEMATIC.stampAt * H));
    expect(film.climbMs).toBe(Math.round(STARSHIP_CINEMATIC.climbAt * H));
    expect(film.climbMs).toBeLessThan(H);
    expect(shakes.some((s) => s.delayMs === film.climbMs)).toBe(true);
    expect(film.durationMs).toBe(H + STARSHIP_CINEMATIC.tailMs);
    expectBounded(cues, D, "held launch");
    // At the cap, still bounded.
    const capped = setPieceEntries(planEntries([launchEvent(held)], view, false, createPlayTracker()), () => ({ key: STARSHIP_SET_PIECE, holdMs: SET_PIECE_HOLD_CAP_MS }), {
      reduced: false,
      bigMoments: "full",
      speed: 1,
    }).find((e) => e.launch !== undefined);
    if (capped === undefined) throw new Error("no capped launch");
    expectBounded(planStarship(capped, view, env())?.cues ?? [], capped.durationMs, "capped launch");
  });

  it("R3334001 the camera looks only where the zoomed picture still fills the screen", () => {
    for (const size of ["small", "medium", "large"] as const) {
      const zoom = STARSHIP_TUNING[size].zoom;
      // In frame is 1/zoom of the picture's side, so the camera may move half the rest from the middle.
      const reach = (1 - 1 / zoom) / 2;
      for (const v of [0, 0.1, 0.3, 0.5, 0.7, 0.9, 1]) {
        const p = cameraPoint({ x: v, y: 1 - v }, zoom);
        for (const c of [p.x, p.y]) {
          expect(c - 1 / (2 * zoom), `${size} ${String(v)}`).toBeGreaterThanOrEqual(-1e-9);
          expect(c + 1 / (2 * zoom), `${size} ${String(v)}`).toBeLessThanOrEqual(1 + 1e-9);
        }
        // A point already in reach is the camera's own.
        if (Math.abs(v - 0.5) <= reach) expect(p.x).toBeCloseTo(v, 9);
        else expect(Math.abs(p.x - 0.5)).toBeCloseTo(reach, 9);
      }
    }
    // A held launch's pan is kept in reach, and still runs stern to bow.
    const held = [BEAM, RUSH, ENGINE, SHIELD, CANNON];
    const view = dormantBoard(held);
    const film = cinematicOf(planStarship(launchEntry(held, view, "full"), view, env())?.cues ?? []);
    if (film === undefined) throw new Error("no cinematic");
    expect(film.cameraFrom).toEqual(cameraPoint(film.cameraFrom, film.zoom));
    expect(film.cameraTo).toEqual(cameraPoint(film.cameraTo, film.zoom));
    expect(film.cameraFrom.x).toBeLessThan(film.cameraTo.x);
  });

  it("R3334001 short, off and unheld draw the title card", () => {
    const held = [BEAM, RUSH];
    const view = dormantBoard(held);
    for (const bigMoments of ["short", "off", null] as const) {
      const entry = launchEntry(held, view, bigMoments);
      const D = entry.durationMs;
      expect(entry.setPiece?.holdMs ?? 0, String(bigMoments)).toBe(bigMoments === "short" ? SET_PIECE_SHORT_HOLD_MS : 0);
      const cues = planStarship(entry, view, env({ bigMoments: bigMoments ?? "full" }))?.cues ?? [];
      const card = cinematicOf(cues);
      expect(card?.still, String(bigMoments)).toBe(true);
      expect(card?.name).toBe(NAME);
      expect(card?.sparks).toBe(0);
      expect(card?.durationMs).toBe(D + FX_STARSHIP_TAIL_MS);
      // The tile's lift-off and landing play as ever.
      expect(cues.some((c) => c.kind === "launch")).toBe(true);
      expectBounded(cues, D, String(bigMoments));
    }
    // Held at "full" but planned under "short" (the setting changed): the title card, not the cinematic.
    const full = launchEntry(held, view, "full");
    expect(cinematicOf(planStarship(full, view, env({ bigMoments: "short" }))?.cues ?? [])?.still).toBe(true);
  });

  it("R3334001 1/4/8 pieces grow it; compact halves particles, same beats", () => {
    const films = [1, 4, 8].map((n) => {
      const held = piecesOf(n);
      const view = dormantBoard(held);
      return { n, cues: planStarship(launchEntry(held, view, "full"), view, env())?.cues ?? [], compact: planStarship(launchEntry(held, view, "full"), view, env({ compact: true }))?.cues ?? [] };
    });
    const sizes = films.map(({ cues }) => cinematicOf(cues)?.size);
    expect(sizes).toEqual(["small", "medium", "large"]);
    const zooms = films.map(({ cues }) => cinematicOf(cues)?.zoom ?? 0);
    expect(zooms).toEqual([STARSHIP_TUNING.small.zoom, STARSHIP_TUNING.medium.zoom, STARSHIP_TUNING.large.zoom]);
    const sparks = films.map(({ cues }) => cinematicOf(cues)?.sparks ?? 0);
    expect(sparks[0]).toBeLessThan(sparks[1] ?? 0);
    expect(sparks[1]).toBeLessThan(sparks[2] ?? 0);
    expect(films.map(({ cues }) => cinematicOf(cues)?.stops.length)).toEqual([1, 4, 8]);
    for (const { n, cues, compact } of films) {
      const full = cinematicOf(cues);
      const phone = cinematicOf(compact);
      // The same beats at the same times…
      expect(compact.map((c) => `${c.kind}@${String(c.delayMs)}`), `${String(n)} pieces`).toEqual(cues.map((c) => `${c.kind}@${String(c.delayMs)}`));
      expect(phone?.stops).toEqual(full?.stops);
      expect(phone?.engines).toEqual(full?.engines);
      // …with fewer particles: half the sparks, and fewer burst particles in all.
      expect(phone?.sparks).toBe(Math.max(1, Math.round((full?.sparks ?? 0) * STARSHIP_CINEMATIC.phoneShare)));
      expectPhoneShare(cues, compact, `launch of ${String(n)}`);
    }
  });

  it("R3334001 each doctrine announced over the ship; hidden none", () => {
    const view = launchedBoard([BEAM, RUSH]);
    for (const [defId, doctrine] of Object.entries(WAR_DOCTRINES)) {
      const played: GameEvent = { type: "cardPlayed", player: "p1", instanceId: `cast-${defId}`, defId, costPaid: 0 };
      const entry = entryOf([played], view, () => true);
      const cues = planStarship(entry, view, env())?.cues ?? [];
      const cue = cues.find((c) => c.kind === "doctrine");
      expect(cue, defId).toMatchObject({ kind: "doctrine", doctrine, tint: WAR_DOCTRINE_LOOKS[doctrine].tint, at: { kind: "testid", testid: testid.card("ship") } });
      expect(cue?.kind === "doctrine" ? cue.text : "", defId).toMatch(/Doctrine/);
      expect(cues.some((c) => c.kind === "burst" && c.preset === WAR_DOCTRINE_LOOKS[doctrine].preset)).toBe(true);
      expectBounded(cues, entry.durationMs, defId);
      // The cast's own row plays on.
      expect(planFx(entry, view, env()).some((c) => c.kind === "doctrine")).toBe(true);
    }
    // With no launched ship it is announced over the board's middle.
    const bare = baseView({ you: emptySide("p1"), opponent: emptySide("p2") });
    const alone = entryOf([{ type: "cardPlayed", player: "p1", instanceId: "cast", defId: "stinkyscape-137-4", costPaid: 0 }], bare, () => true);
    expect(planStarship(alone, bare, env())?.cues.find((c) => c.kind === "doctrine")).toMatchObject({ at: { kind: "viewport" } });
    // A cast the seat cannot read (R97) is no doctrine; nor is any other card.
    for (const defId of ["hidden", "stinkyscape-137"]) {
      const other = entryOf([{ type: "cardPlayed", player: "p2", instanceId: "hidden", defId, costPaid: 0 }], view, () => true);
      expect(planStarship(other, view, env())).toBeNull();
    }
  });
});

describe("R3334002 the broadside", () => {
  it("R3334002 each kind's broadside within attackDeclared's row + FX_MAX_TAIL_MS", () => {
    const kinds: readonly [string, (cues: readonly FxCue[]) => void][] = [
      [BEAM, (cues) => expect(shotsOf(cues).filter((s) => s.shot.motion === "beam")).toHaveLength(STARSHIP_BROADSIDE.beamSweep)],
      [REPEATER, (cues) => expect(shotsOf(cues).filter((s) => s.shot.motion === "bolt" && s.shot.trail === "spark")).toHaveLength(STARSHIP_BROADSIDE.repeaterShots)],
      [
        CANNON,
        (cues) => {
          expect(cues.filter((c) => c.kind === "module" && c.motion === "recoil")).toHaveLength(1);
          expect(shotsOf(cues).filter((s) => s.shot.motion === "bolt" && s.shot.trail === "smoke")).toHaveLength(1);
          expect(cues.some((c) => c.kind === "shake")).toBe(true);
        },
      ],
      [
        FLAK,
        (cues) => {
          expect(shotsOf(cues)).toHaveLength(1);
          expect(cues.filter((c) => c.kind === "burst" && c.preset === "smoke")).toHaveLength(STARSHIP_BROADSIDE.flakClouds);
        },
      ],
      ...([
        [RUSH, "token"],
        [FRUIT, "fruit"],
        [JADE, "jade"],
        [BOOK, "book"],
      ] as const).map(([id, sprite]): [string, (cues: readonly FxCue[]) => void] => [
        id,
        (cues) => expect(shotsOf(cues).map((s) => (s.shot.motion === "arc" ? s.shot.sprites : null))).toEqual([[sprite]]),
      ]),
    ];
    for (const [defId, check] of kinds) {
      const view = launchedBoard([defId, SHIELD]);
      const entry = entryOf([attack()], view, () => true);
      const D = entry.durationMs;
      expect(D).toBe(ANIMATIONS.attackDeclared.durationMs);
      const plan = planStarship(entry, view, env());
      const cues = plan?.cues ?? [];
      check(cues);
      // Every shot leaves the gun's point of the picture and lands on the target as the row ends.
      for (const shot of shotsOf(cues)) {
        expect(shot.from).toMatchObject({ kind: "testid", testid: testid.card("ship"), art: "picture" });
        expect(shot.to).toMatchObject({ kind: "testid", testid: testid.card("foe") });
        expect(shot.delayMs + shot.flightMs).toBe(D);
        expect(shot.delayMs).toBeGreaterThanOrEqual(broadsideFireMs(0, 1, D));
      }
      // It draws the attack in place of its row.
      expect(plan?.replaces.has(entry.events[0] as GameEvent)).toBe(true);
      expectBounded(cues, D, defId);
    }
    // Every weapon kind fires as one of the five.
    expect(new Set(Object.values(BROADSIDE_WEAPONS).map((w) => w?.fire))).toEqual(new Set(["beam", "repeater", "cannon", "flak", "launcher"]));
    // Eight guns fire one after another, the last at STARSHIP_BROADSIDE.lastAt; a ninth stays silent.
    const nine = launchedBoard([BEAM, REPEATER, CANNON, FLAK, RUSH, FRUIT, JADE, BOOK, BEAM]);
    const big = entryOf([attack("hero-p2")], nine, () => true);
    const fired = planStarship(big, nine, env())?.cues ?? [];
    const guns = Array.from({ length: STARSHIP_BROADSIDE.gunsMax }, (_, j) => broadsideFireMs(j, STARSHIP_BROADSIDE.gunsMax, big.durationMs));
    expect(guns[guns.length - 1]).toBe(Math.round(STARSHIP_BROADSIDE.lastAt * big.durationMs));
    expect(new Set(shotsOf(fired).map((s) => s.from.kind === "testid" ? JSON.stringify(s.from.at) : ""))).toHaveProperty("size", STARSHIP_BROADSIDE.gunsMax);
    expect(shotsOf(fired).every((s) => s.to.kind === "testid" && s.to.testid === testid.hero("opponent"))).toBe(true);
    expectBounded(fired, big.durationMs, "eight guns");
  });

  it("R3334002 weaponless rams with a turned lunge; armed still lunge", () => {
    const unarmed = launchedBoard([SHIELD, ENGINE, DRILL]);
    const entry = entryOf([attack()], unarmed, () => true);
    const D = entry.durationMs;
    const contact = Math.round(FX_LUNGE_CONTACT_AT * D);
    const cues = planStarship(entry, unarmed, env())?.cues ?? [];
    expect(shotsOf(cues)).toHaveLength(0);
    // Exhaust from its engine as it starts, its prow sparking and a ring of shards at the contact.
    expect(cues.find((c) => c.kind === "burst" && c.preset === "exhaust")).toMatchObject({ delayMs: 0, at: { art: "picture" } });
    expect(cues.find((c) => c.kind === "burst" && c.preset === "spark")).toMatchObject({ delayMs: contact, at: { testid: testid.card("ship"), art: "picture" } });
    expect(cues.find((c) => c.kind === "ring")).toMatchObject({ preset: "shard", delayMs: contact, at: { testid: testid.card("foe") } });
    expect(cues.find((c) => c.kind === "shake")).toMatchObject({ trauma: STARSHIP_BROADSIDE.ramTrauma, delayMs: contact });
    expectBounded(cues, D, "ram");
    const stage = planStage(entry, unarmed, env());
    expect(stage.find((c) => c.kind === "lunge")).toMatchObject({ attacker: testid.card("ship"), target: testid.card("foe"), ram: true });
    expect(stage.find((c) => c.kind === "lunge" && c.still === true)).toBeUndefined();
    // With Flying it arcs as it rams.
    const flier = launchedBoard([SHIELD], { keywords: [{ kind: "Flying" }] });
    expect(planStage(entryOf([attack()], flier, () => true), flier, env()).find((c) => c.kind === "lunge")).toMatchObject({ ram: true, arc: true });

    // An armed ship fires where it stands: a still lunge, no ram, no contact spark of the stage's.
    const armed = launchedBoard([BEAM]);
    const fire = planStage(entryOf([attack()], armed, () => true), armed, env());
    expect(fire).toEqual([{ kind: "lunge", attacker: testid.card("ship"), target: testid.card("foe"), delayMs: 0, durationMs: D, still: true }]);
    // A Dormant ship never attacks as one; any other unit lunges as ever.
    const plain = baseView({
      you: emptySide("p1", { units: [null, null, unit("p1", { instanceId: "ship" }), null, null] }),
      opponent: emptySide("p2", { units: [null, null, unit("p2", { instanceId: "foe" }), null, null] }),
    });
    const walk = planStage(entryOf([attack()], plain, () => true), plain, env()).find((c) => c.kind === "lunge");
    expect(walk).toMatchObject({ kind: "lunge" });
    expect(walk !== undefined && "ram" in walk).toBe(false);
    expect(planStarship(entryOf([attack()], plain, () => true), plain, env())).toBeNull();
  });
});

describe("R3334003 the wreck", () => {
  it("R3334003 the wreck: blow-offs, break, debris, scorch, inside FX_MAX_TAIL_MS", () => {
    for (const n of [1, 4, 8]) {
      const held = piecesOf(n);
      const view = launchedBoard(held);
      const death: GameEvent = { type: "destroyed", instanceId: "ship", defId: HULL, owner: "p1", controller: "p1", attack: 3, maxHealth: 5 } as GameEvent;
      const entry = entryOf([death], view, () => true);
      const D = entry.durationMs;
      const plan = planStarship(entry, view, env());
      const cues = plan?.cues ?? [];
      // Its newest modules blow off one at a time, newest first.
      const off = cues.filter((c): c is FxModuleCue => c.kind === "module" && c.motion === "blowOff");
      expect(off).toHaveLength(Math.min(n, STARSHIP_WRECK.modulesMax));
      for (let k = 1; k < off.length; k += 1) expect(off[k]?.delayMs).toBeGreaterThan(off[k - 1]?.delayMs ?? 0);
      const breakMs = Math.round(STARSHIP_WRECK.breakAt * D);
      expect(off.every((c) => c.delayMs < breakMs)).toBe(true);
      // The hull breaks in two over its art as the card is hidden.
      expect(cues.find((c) => c.kind === "wreck")).toMatchObject({ at: { testid: testid.card("ship"), art: "frame" }, breakMs, turn: STARSHIP_WRECK.halfTurnDeg });
      const conceal = cues.find((c): c is FxConcealCue => c.kind === "conceal");
      expect(conceal).toMatchObject({ testid: testid.card("ship"), mode: "after", delayMs: breakMs });
      // The stage hides the dead card too, in the same mode: the wreck's conceal outlasts it, so its undo never
      // shows the dead ship again while the board has not yet moved on.
      const staged = planStage(entry, view, env()).find((c): c is FxConcealCue => c.kind === "conceal" && c.testid === testid.card("ship"));
      expect(staged).toMatchObject({ mode: "after" });
      expect((conceal?.delayMs ?? 0) + (conceal?.durationMs ?? 0)).toBeGreaterThanOrEqual((staged?.delayMs ?? 0) + (staged?.durationMs ?? Infinity));
      expect(cues.find((c) => c.kind === "shake")).toMatchObject({ delayMs: breakMs });
      // Debris over its backrow, a scorch on its zone.
      expect(cues.find((c) => c.kind === "burst" && c.preset === "dust")).toMatchObject({ at: { testid: testid.zone("you", "backrow", 3) }, delayMs: Math.round(STARSHIP_WRECK.debrisAt * D) });
      expect(cues.find((c) => c.kind === "scorch")).toMatchObject({ at: { testid: testid.zone("you", "units", 3) }, delayMs: Math.round(STARSHIP_WRECK.scorchAt * D) });
      expect(plan?.replaces.has(death)).toBe(true);
      // No death row of its own; quieter than the launch.
      expect(planFx(entry, view, env()).filter((c) => c.kind === "burst" && c.preset === "smoke").length).toBeGreaterThan(0);
      expectBounded(cues, D, `wreck of ${String(n)}`);
    }
    // A Dormant ship wrecks too; a wipe's set piece swallows a Starship whole, so none there; any other unit dies as ever.
    const dormant = dormantBoard([BEAM]);
    const dies = (instanceId: string): GameEvent => ({ type: "destroyed", instanceId, defId: HULL, owner: "p1", controller: "p1", attack: 0, maxHealth: 5 }) as GameEvent;
    expect(planStarship(entryOf([dies("ship")], dormant, () => true), dormant, env())?.cues.some((c) => c.kind === "wreck")).toBe(true);
    const wiped = { ...entryOf([dies("ship")], dormant, () => true), wipe: { by: "x", sourceId: "x", player: "p1" } } as unknown as AnimationEntry;
    expect(planStarship(wiped, dormant, env())).toBeNull();
    expect(planStarship(entryOf([dies("foe")], dormant, () => true), dormant, env())).toBeNull();
  });
});
