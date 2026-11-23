// R3337003: an Anomaly's set piece as the effects layer plans it (anomaly.ts). Held in full, a rift
// tears down the caster's side, every card it takes is pulled in and concealed as it flares, the title
// card stamps, and the new board's cards fly out of the rift onto their zones, each as its board lands;
// "short" draws only the title card and the landing, "off" the title card alone, Reduce Motion one still
// title card. Everything lands inside its entry and is gone within FX_MAX_TAIL_MS of it (R200); no cue
// names a card (R202). The card motif of whichever card laid it yields to it.

import type { GameEvent, PlayerId, PlayerView, UnitView } from "@jackioh/shared";
import { describe, expect, it, vi } from "vitest";

import { ANOMALIES, ANOMALY_CODES, type AnomalyCode } from "../cards/anomalies.ts";
import { anomalyMarkup } from "../cards/art/svg.ts";
import { FLAVOR_FALLBACK } from "../cards/flavor.ts";
import {
  ANOMALY_LAND_MS,
  ANOMALY_RIFT_HOLD_MS,
  ANOMALY_SET_PIECE,
  SET_PIECE_SHORT_HOLD_MS,
  planEntries,
  setPieceEntries,
  type AnimationEntry,
} from "../game/animations.ts";
import { testid } from "../game/contract.ts";
import { createPlayTracker } from "../game/runs.ts";
import { baseView, emptySide, unit } from "../test/fixtures.ts";
import { endOf } from "../test/signatures.ts";
import {
  ANOMALY_FLARE_AT,
  ANOMALY_FX,
  ANOMALY_LAND_FLIGHT,
  ANOMALY_LAND_SPREAD,
  ANOMALY_LAND_STAGGER_MS,
  ANOMALY_LAND_TRAUMA,
  ANOMALY_STAMP_AT,
  ANOMALY_STILL_MS,
  ANOMALY_TAIL_MS,
  ANOMALY_TEAR_AT,
  ANOMALY_TITLE_LINGER_AT,
  ANOMALY_TRAUMA,
  ANOMALY_TUNING,
  anomalyClaim,
  planAnomaly,
  planAnomalyStill,
} from "./anomaly.ts";
import { raysCue, ringCue, tunedBurst } from "./build.ts";
import { planCardMotif } from "./cardFx.ts";
import { FX_CENTER, FX_HOLD_MAX_MS, FX_INTENSITY_SCALE, FX_MAX_TAIL_MS, FX_MOTIF_CENTER_LANE } from "./constants.ts";
import { planFx } from "./cues.ts";
import { FLAVOR_LOOKS, paletteLook } from "./flavors.ts";
import { createFxMemory } from "./memory.ts";
import { tintOf } from "./setPieces.ts";
import { planStage } from "./stage.ts";
import type { FxAnchor, FxCue, FxPlanEnv } from "./types.ts";

const ANOMALY_FOUND = "stinkyscape-t-anomaly-found";
const CHAOS_SCAPE = "stinkyscape-151";

const env = (intensity: number = FX_INTENSITY_SCALE.normal): FxPlanEnv => ({ intensity, card: () => undefined, memory: createFxMemory() });

const exiled = (u: UnitView): GameEvent => ({ type: "exiled", instanceId: u.instanceId, defId: u.defId, owner: u.owner });
const laid = (instanceId: string, lane: number, player: PlayerId = "p1"): GameEvent => ({ type: "summoned", player, instanceId, defId: "stinkyscape-171", row: "units", lane });

/** p1's side holds two Units in lanes 1 and 2. */
function board(): { view: PlayerView; a: UnitView; b: UnitView } {
  const a = unit("p1", { instanceId: "a" });
  const b = unit("p1", { instanceId: "b" });
  const view = baseView({
    you: emptySide("p1", { units: [a, b, null, null, null] }),
    opponent: emptySide("p2", { units: [unit("p2", { instanceId: "foe" }), null, null, null, null] }),
  });
  return { view, a, b };
}

/** The rift entry of `code` laying `landings` units, held `holdMs` (0: unheld), as the queue gives it. */
function riftEntry(code: AnomalyCode, holdMs: number, landings = 3): { entry: AnimationEntry; view: PlayerView } {
  const { view, a, b } = board();
  const events: GameEvent[] = [{ type: "boardReplaced", player: "p1", anomaly: code }, exiled(a), exiled(b)];
  for (let k = 0; k < landings; k += 1) events.push(laid(`new-${String(k)}`, k + 1));
  const [planned] = planEntries(events, view, false, createPlayTracker());
  if (planned?.rift === undefined) throw new Error("no rift");
  if (holdMs <= 0) return { entry: planned, view };
  return { entry: { ...planned, setPiece: { key: ANOMALY_SET_PIECE, holdMs }, durationMs: planned.durationMs + holdMs }, view };
}

const kinds = (cues: readonly FxCue[]): string[] => [...new Set(cues.map((cue) => cue.kind))].sort();

describe("R3337003 an Anomaly's set piece", () => {
  it("R3337003 held in full: the rift, the pulls and conceals at the flare, the flare, the title card at the stamp and the landing", () => {
    const { entry, view } = riftEntry("WH", ANOMALY_RIFT_HOLD_MS);
    const plan = planAnomaly(entry, view, env());
    expect(plan).not.toBeNull();
    const cues = plan?.cues ?? [];
    const H = ANOMALY_RIFT_HOLD_MS;
    const flare = Math.round(ANOMALY_FLARE_AT * H);
    const centre = { kind: "testid", testid: testid.zone("you", "units", FX_MOTIF_CENTER_LANE) };
    expect(kinds(cues)).toEqual(["anomalyTitle", "burst", "conceal", "ghost", "pull", "rays", "rift", "ring", "shake"]);
    expect(cues.find((cue) => cue.kind === "rift")).toMatchObject({ at: centre, side: "you", tearMs: Math.round(ANOMALY_TEAR_AT * H), flareMs: flare, delayMs: 0 });
    for (const card of [testid.card("a"), testid.card("b")]) {
      expect(cues.find((cue) => cue.kind === "pull" && cue.testid === card), card).toMatchObject({ to: centre, delayMs: 0 });
      expect(cues.find((cue) => cue.kind === "conceal" && cue.testid === card), card).toMatchObject({ mode: "after", delayMs: flare });
    }
    expect(cues.filter((cue) => cue.kind === "rays" || cue.kind === "shake").every((cue) => cue.delayMs >= flare)).toBe(true);
    expect(cues.find((cue) => cue.kind === "anomalyTitle")?.delayMs).toBe(Math.round(ANOMALY_STAMP_AT * H));
    // The landings fly out of the rift after the hold, one after another, each to its zone.
    const ghosts = cues.filter((cue) => cue.kind === "ghost");
    expect(ghosts.map((cue) => (cue.kind === "ghost" && cue.to.kind === "testid" ? cue.to.testid : ""))).toEqual([1, 2, 3].map((lane) => testid.zone("you", "units", lane)));
    expect(ghosts.every((cue) => cue.kind === "ghost" && cue.from.kind === "testid" && cue.from.testid === centre.testid)).toBe(true);
    expect(ghosts.every((cue) => cue.delayMs >= H && cue.delayMs + cue.durationMs <= entry.durationMs)).toBe(true);
    expect([...(plan?.replaces ?? [])].map((event) => event.type).sort()).toEqual(["boardReplaced", "exiled", "exiled", "summoned", "summoned", "summoned"]);
  });

  it("R3337003 short draws the title card from the start and the landing, no rift and no pulls; off the title card alone", () => {
    const { entry: short, view } = riftEntry("AL", SET_PIECE_SHORT_HOLD_MS);
    const shortCues = planAnomaly(short, view, env())?.cues ?? [];
    expect(kinds(shortCues)).not.toContain("rift");
    expect(kinds(shortCues)).not.toContain("pull");
    expect(kinds(shortCues)).toContain("ghost");
    expect(shortCues.find((cue) => cue.kind === "anomalyTitle")?.delayMs).toBe(0);
    expect(shortCues.filter((cue) => cue.kind === "conceal").every((cue) => cue.kind === "conceal" && cue.mode === "now")).toBe(true);

    const { entry: off } = riftEntry("AL", 0);
    expect(off.setPiece).toBeUndefined();
    const offCues = planAnomaly(off, view, env())?.cues ?? [];
    expect(kinds(offCues)).toEqual(["anomalyTitle", "conceal"]);
    expect(offCues.find((cue) => cue.kind === "anomalyTitle")).toMatchObject({ delayMs: 0, durationMs: ANOMALY_LAND_MS });
  });

  it("R3337003 each Anomaly lands as its board does, and every cue ends within its tail", () => {
    for (const code of ANOMALY_CODES) {
      for (const holdMs of [ANOMALY_RIFT_HOLD_MS, SET_PIECE_SHORT_HOLD_MS, 0]) {
        const { entry, view } = riftEntry(code, holdMs, 8);
        const cues = planAnomaly(entry, view, env())?.cues ?? [];
        expect(cues.length, `${code} ${String(holdMs)}`).toBeGreaterThan(0);
        for (const cue of cues) {
          if (cue.kind === "pull" || cue.kind === "conceal") continue;
          expect(cue.delayMs, `${code} ${cue.kind}`).toBeGreaterThanOrEqual(0);
          expect(endOf(cue), `${code} ${String(holdMs)} ${cue.kind}`).toBeLessThanOrEqual(entry.durationMs + FX_MAX_TAIL_MS);
          if (cue.kind === "shake") expect(cue.trauma).toBeLessThanOrEqual(1);
        }
        if (holdMs === 0) continue;
        const ghosts = cues.filter((cue) => cue.kind === "ghost");
        expect(ghosts, code).toHaveLength(8);
        const landing = ANOMALY_FX[code].landing;
        expect(ghosts.every((cue) => cue.kind === "ghost" && (cue.spin === true) === (landing === "tumble")), code).toBe(true);
        const presets = new Set(cues.flatMap((cue) => (cue.kind === "burst" && cue.delayMs > holdMs ? [cue.preset] : [])));
        expect(presets.has(landing === "tumble" ? "dust" : landing === "power" ? "spark" : "sparkle"), `${code} ${landing}`).toBe(true);
      }
    }
  });

  it("R3337003 the title card names the Anomaly over its picture, and no cue names a card", () => {
    for (const code of ANOMALY_CODES) {
      const { entry, view } = riftEntry(code, ANOMALY_RIFT_HOLD_MS);
      const cues = planAnomaly(entry, view, env())?.cues ?? [];
      const title = cues.find((cue) => cue.kind === "anomalyTitle");
      expect(title, code).toMatchObject({ at: { kind: "viewport", at: FX_CENTER }, text: ANOMALIES[code].name, picture: anomalyMarkup(code, ANOMALIES[code].rich) });
      const named = JSON.stringify(cues.filter((cue) => cue.kind !== "anomalyTitle"));
      expect(named, code).not.toMatch(/stinkyscape-|core-/);
    }
  });

  it("R3337003 under Reduce Motion the title card stands still; the effects off plan nothing", () => {
    const { entry, view } = riftEntry("FO", 0);
    const still = planAnomalyStill({ ...entry, durationMs: 0 }, view, env());
    expect(still).toEqual([expect.objectContaining({ kind: "anomalyTitle", still: true, delayMs: 0, durationMs: ANOMALY_STILL_MS, text: "Frontier Outpost" })]);
    expect(planAnomaly({ ...entry, durationMs: 0 }, view, env())).toBeNull();
    expect(planAnomaly(entry, view, env(0))).toBeNull();
    expect(planAnomalyStill(entry, view, env(0))).toEqual([]);
    const { view: plain } = board();
    const [replaced] = planEntries([{ type: "boardReplaced", player: "p1" }], plain, false, createPlayTracker());
    expect(replaced !== undefined && planAnomaly(replaced, plain, env())).toBeNull();
    expect(replaced !== undefined && planAnomalyStill(replaced, plain, env())).toEqual([]);
  });

  it("R3337003 the claim, the card motif that yields, and the stage's landing that yields to a held rift", () => {
    const { entry, view } = riftEntry("DH", 0);
    expect(anomalyClaim(entry)).toEqual({ key: ANOMALY_SET_PIECE, holdMs: ANOMALY_RIFT_HOLD_MS });
    const [held] = setPieceEntries([entry], anomalyClaim, { reduced: false, bigMoments: "full", speed: 1 });
    expect(held?.setPiece?.key).toBe(ANOMALY_SET_PIECE);
    // Call to Chaos (Scape Edition)'s wipe motif yields: the rift is its own set piece.
    for (const defId of [CHAOS_SCAPE, ANOMALY_FOUND]) {
      const memory = createFxMemory();
      memory.remember([{ type: "cardPlayed", player: "p1", instanceId: "caster", defId, costPaid: 4 }]);
      memory.remember(entry.events);
      expect(planCardMotif(entry, view, { ...env(), memory }), defId).toBeNull();
    }
    // The stage lands the board's cards under a held rift no more: the set piece does.
    const stageHeld = planStage(held as AnimationEntry, view, env());
    expect(stageHeld.some((cue) => cue.kind === "hold")).toBe(false);
    expect(planStage(entry, view, env()).some((cue) => cue.kind === "hold")).toBe(true);
    // planFx draws the set piece and no system ring of the board's own.
    const cues = planFx(held as AnimationEntry, view, env());
    expect(cues.some((cue) => cue.kind === "rift")).toBe(true);
    expect(cues.some((cue) => cue.kind === "ring" && cue.at.kind === "testid" && cue.at.testid === testid.board)).toBe(false);
  });
});

const at = (value: string): FxAnchor => ({ kind: "testid", testid: value });
const CENTRE = at(testid.zone("you", "units", FX_MOTIF_CENTER_LANE));
const I = FX_INTENSITY_SCALE.normal;

/** `code`'s rift entry laying `landings` units, held `H` and landing over `M` after it (the effects speed's share, R201). */
function heldRift(code: AnomalyCode, H: number, M: number, landings = 3): { entry: AnimationEntry; view: PlayerView } {
  const { entry, view } = riftEntry(code, H, landings);
  return { entry: { ...entry, durationMs: H + M }, view };
}

/** The ghost each landing flies out of the rift, `start` after its entry starts, for `flight`. */
const ghostOf = (lane: number, start: number, flight: number, spin: boolean): FxCue => ({
  kind: "ghost",
  from: CENTRE,
  to: at(testid.zone("you", "units", lane)),
  delayMs: start,
  durationMs: flight,
  ...(spin ? { spin: true as const } : {}),
});

/** The title card of `code`, its name over its picture in its palette's tint. */
function titleOf(code: AnomalyCode, palette: string, delayMs: number, durationMs: number): FxCue {
  const look = paletteLook(palette);
  if (look === undefined) throw new Error(palette);
  return { kind: "anomalyTitle", at: { kind: "viewport", at: FX_CENTER }, text: ANOMALIES[code].name, picture: anomalyMarkup(code, ANOMALIES[code].rich), tint: tintOf(look), delayMs, durationMs };
}

describe("R3337003 an Anomaly's set piece, beat by beat", () => {
  it("R3337003 held in full, the Wormhole tears, flares, stamps its title card to its linger and powers each card on in turn", () => {
    const H = ANOMALY_RIFT_HOLD_MS;
    const { entry, view } = heldRift("WH", H, ANOMALY_LAND_MS);
    const D = entry.durationMs;
    const M = ANOMALY_LAND_MS;
    // The Wormhole's palette names a look of its own, not the fallback flavor's.
    const look = paletteLook("Tech/starfield");
    if (look === undefined) throw new Error("Tech/starfield");
    expect(look.colors).not.toEqual(FLAVOR_LOOKS[FLAVOR_FALLBACK].colors);
    const tint = tintOf(look);
    const flare = Math.round(ANOMALY_FLARE_AT * H);
    const stamp = Math.round(ANOMALY_STAMP_AT * H);
    const linger = H + Math.round(ANOMALY_TITLE_LINGER_AT * M);
    // Three cards over half the landing would stand M / 4 apart, more than the most one waits: each waits that most.
    expect((M * ANOMALY_LAND_SPREAD) / 2).toBeGreaterThan(ANOMALY_LAND_STAGGER_MS);
    const flight = Math.round(ANOMALY_LAND_FLIGHT * M);
    const landings = [1, 2, 3].flatMap((lane, k) => {
      const start = H + k * ANOMALY_LAND_STAGGER_MS;
      const zone = at(testid.zone("you", "units", lane));
      return [
        ghostOf(lane, start, flight, false),
        ringCue(D, "spark", zone, start + flight),
        { ...tunedBurst(I, "spark", zone, "point", start + flight, ANOMALY_TUNING.spark), colors: look.colors },
      ];
    });
    expect(planAnomaly(entry, view, env())?.cues).toEqual([
      { kind: "rift", at: CENTRE, side: "you", tint, tearMs: Math.round(ANOMALY_TEAR_AT * H), flareMs: flare, delayMs: 0, durationMs: D + ANOMALY_TAIL_MS },
      { kind: "pull", testid: testid.card("a"), to: CENTRE, delayMs: 0, durationMs: FX_HOLD_MAX_MS },
      { kind: "conceal", testid: testid.card("a"), mode: "after", delayMs: flare, durationMs: FX_HOLD_MAX_MS },
      { kind: "pull", testid: testid.card("b"), to: CENTRE, delayMs: 0, durationMs: FX_HOLD_MAX_MS },
      { kind: "conceal", testid: testid.card("b"), mode: "after", delayMs: flare, durationMs: FX_HOLD_MAX_MS },
      raysCue(D, "radiant", CENTRE, flare),
      { ...tunedBurst(I, "prismatic", CENTRE, "area", flare, ANOMALY_TUNING.flare), colors: look.colors },
      ringCue(D, "void", CENTRE, flare),
      { kind: "shake", trauma: Math.min(1, ANOMALY_TRAUMA * I), delayMs: flare },
      titleOf("WH", "Tech/starfield", stamp, linger - stamp),
      ...landings,
    ]);
  });

  it("R3337003 short, an Asteroid Field's title card stands from the start to its linger, the old side goes at once and its rocks tumble in with dust and a shake", () => {
    const H = SET_PIECE_SHORT_HOLD_MS;
    const M = ANOMALY_LAND_MS;
    const { entry, view } = heldRift("AL", H, M);
    const flight = Math.round(ANOMALY_LAND_FLIGHT * M);
    const landings = [1, 2, 3].flatMap((lane, k) => {
      const land = H + k * ANOMALY_LAND_STAGGER_MS + flight;
      const zone = at(testid.zone("you", "units", lane));
      return [ghostOf(lane, land - flight, flight, true), tunedBurst(I, "dust", zone, "area", land, ANOMALY_TUNING.dust), { kind: "shake", trauma: Math.min(1, ANOMALY_LAND_TRAUMA * I), delayMs: land }];
    });
    expect(planAnomaly(entry, view, env())?.cues).toEqual([
      titleOf("AL", "Hustle/ore", 0, H + Math.round(ANOMALY_TITLE_LINGER_AT * M)),
      { kind: "conceal", testid: testid.card("a"), mode: "now", delayMs: 0, durationMs: FX_HOLD_MAX_MS },
      { kind: "conceal", testid: testid.card("b"), mode: "now", delayMs: 0, durationMs: FX_HOLD_MAX_MS },
      ...landings,
    ]);
  });

  it("R3337003 the Frontier Outpost's lights come up on each card it lands; unheld, its title card stands over the entry and the old side goes at once", () => {
    const H = ANOMALY_RIFT_HOLD_MS;
    const M = ANOMALY_LAND_MS;
    const { entry, view } = heldRift("FO", H, M, 1);
    const look = paletteLook("Tech/laser");
    if (look === undefined) throw new Error("Tech/laser");
    const flight = Math.round(ANOMALY_LAND_FLIGHT * M);
    const zone = at(testid.zone("you", "units", 1));
    expect(planAnomaly(entry, view, env())?.cues.slice(-2)).toEqual([ghostOf(1, H, flight, false), { ...tunedBurst(I, "sparkle", zone, "point", H + flight, ANOMALY_TUNING.lights), colors: look.colors }]);

    const { entry: off } = riftEntry("FO", 0);
    expect(planAnomaly(off, view, env())?.cues).toEqual([
      titleOf("FO", "Tech/laser", 0, off.durationMs),
      { kind: "conceal", testid: testid.card("a"), mode: "now", delayMs: 0, durationMs: FX_HOLD_MAX_MS },
      { kind: "conceal", testid: testid.card("b"), mode: "now", delayMs: 0, durationMs: FX_HOLD_MAX_MS },
    ]);
  });

  it("R3337003 a short landing spreads its cards over half of it; one card lands as the hold ends, even with no landing left", () => {
    const H = ANOMALY_RIFT_HOLD_MS;
    const M = 200;
    // Three cards over half of 200 ms stand 50 ms apart, under the most one waits.
    const step = (M * ANOMALY_LAND_SPREAD) / 2;
    expect(step).toBeLessThan(ANOMALY_LAND_STAGGER_MS);
    const flight = Math.round(ANOMALY_LAND_FLIGHT * M);
    const { entry, view } = heldRift("WH", H, M);
    const ghosts = (planAnomaly(entry, view, env())?.cues ?? []).filter((cue) => cue.kind === "ghost");
    expect(ghosts).toEqual([ghostOf(1, H, flight, false), ghostOf(2, H + step, flight, false), ghostOf(3, H + 2 * step, flight, false)]);
    // An entry all hold: its one card still flies at the hold's end.
    const { entry: bare } = heldRift("WH", H, 0, 1);
    expect((planAnomaly(bare, view, env())?.cues ?? []).filter((cue) => cue.kind === "ghost")).toEqual([ghostOf(1, H, 0, false)]);
  });

  it("R3337003 the set piece draws only the replacement's, the exiles' and the landings' rows in their place", () => {
    const { view, a, b } = board();
    const draw: GameEvent = { type: "drawn", player: "p1", instanceId: "d1", defId: "core-021" };
    const events: GameEvent[] = [{ type: "boardReplaced", player: "p1", anomaly: "MT" }, exiled(a), exiled(b), laid("new-0", 1), draw];
    const [entry] = planEntries(events, view, false, createPlayTracker());
    expect(entry?.events).toEqual(events);
    const replaces = entry === undefined ? undefined : planAnomaly(entry, view, env())?.replaces;
    expect([...(replaces ?? [])]).toEqual(events.slice(0, 4));
  });
});

describe("R3337003 every Anomaly's look as a page load builds it", () => {
  it("R3337003 each board's palette, landing and backdrop: the Asteroid Fields tumble over drift, the drones and hulls power on, the outpost's lights come up over holy motes", async () => {
    // Built as the module loads: read from a module loaded inside the test, as cards/settings.test.ts does.
    vi.resetModules();
    const fresh = await import("./anomaly.ts");
    expect(fresh.ANOMALY_FX).toEqual({
      WH: { palette: "Tech/starfield", landing: "power", backdrop: "void" },
      BF: { palette: "Fire", landing: "power", backdrop: "ember" },
      AL: { palette: "Hustle/ore", landing: "tumble", backdrop: "drift" },
      AT: { palette: "Hustle/ore", landing: "tumble", backdrop: "drift" },
      SP: { palette: "Fire", landing: "power", backdrop: "toxic" },
      MT: { palette: "Hustle/ore", landing: "tumble", backdrop: "drift" },
      DH: { palette: "Tech/hologram", landing: "power", backdrop: "arcane" },
      FO: { palette: "Tech/laser", landing: "lights", backdrop: "holy" },
      AN: { palette: "Dark/blood", landing: "tumble", backdrop: "drift" },
      AA: { palette: "Hustle/ore", landing: "tumble", backdrop: "drift" },
    });
    for (const code of ANOMALY_CODES) expect(paletteLook(fresh.ANOMALY_FX[code].palette), code).toBeDefined();
  });
});
