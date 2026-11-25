// R3405001: Jlockeed's Headquarters' signature (jlockeedHq.ts), cue by cue, past what
// signatures.units-a.test.ts holds: its entrance's fortress, beacons, ring, exhaust and shake in Tech's rocket
// look, every one after its entry's lead (R3405000), the fortress mirrored for the landing's seat only when the
// entrance knows both the board and the landing; its Activate's convoys, each rolled out of its hangar (or its
// hero, off the board) into the zone a Unit fills, under its play or its Activate alone (an event of another
// type, another player's summon, or a summon under another hook of its is none of them); its gallery fixture's
// Unit, into the first empty unit zone; and the points and look its module builds as it loads.

import type { GameEvent, PlayerId, PlayerView, UnitView } from "@jackioh/shared";
import { describe, expect, it, vi } from "vitest";

import { testid } from "../../../game/contract.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import { landOf, run, tid } from "../../../test/signatures.ts";
import { CATALOG } from "../../../wire/cards.ts";
import { frac } from "../../build.ts";
import { FX_INTENSITY_SCALE, FX_SLAM_AT } from "../../constants.ts";
import { figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook, type FlavorLook } from "../../flavors.ts";
import { ENTRANCE_TRAUMA, FOOT } from "../recipe.ts";
import { fortressRise, jlockeedHqFixture } from "./jlockeedHq.ts";

const HQ = "meditative-097-9";
const SHREDDER = "core-013";
const D = 1000;
const LEAD = 200;
const LANE = 3;
const TGT = testid.zone("you", "units", LANE);
const NORMAL = FX_INTENSITY_SCALE.normal;
const UNCLAIMED = { cues: [], row: undefined };
const BEACONS = [
  { x: 0.12, y: 0.08 },
  { x: 0.88, y: 0.08 },
];

function lookOf(palette: string): FlavorLook {
  const look = paletteLook(palette);
  if (look === undefined) throw new Error(`no look ${palette}`);
  return look;
}

/** Tech's rocket sub-look, its aura's: not Tech's own blue circuits. */
const ROCKET = lookOf("Tech/rocket");

const landing = (player: PlayerId): Extract<GameEvent, { type: "summoned" }> => ({ type: "summoned", player, instanceId: "ex", defId: HQ, row: "units", lane: LANE });
const summon = (player: PlayerId, lane: number): GameEvent => ({ type: "summoned", player, instanceId: `fx-unit-${String(lane)}`, defId: SHREDDER, row: "units", lane });

/** `units` with the slots in `open` emptied. */
const emptied = (units: readonly (UnitView | null)[], open: readonly number[]): (UnitView | null)[] => units.map((shown, k) => (open.includes(k) ? null : shown));

/** The board with it standing as "ex" in your lane LANE and your lanes past it empty. */
function standing(): PlayerView {
  const view = fullBoardView();
  const units = view.you.units.map((shown, k) => (k === LANE - 1 && shown !== null ? { ...shown, instanceId: "ex" } : k >= LANE ? null : shown));
  return { ...view, you: { ...view.you, units } };
}

describe("R3405001 Jlockeed's Headquarters' signature, cue by cue", () => {
  it("R3405000 its fortress shrinks into its zone from its entry's lead and outlasts it by 300 ms; both beacons flash at 15% and at 40% of the motion, and it settles at the slam after the lead in a ring and a puff of exhaust, all in rocket light", () => {
    const span = D - LEAD;
    const slam = LEAD + frac(FX_SLAM_AT, span);
    const flash = (at: number) =>
      BEACONS.map((beacon) => ({ kind: "burst", preset: "spark", at: tid(TGT, beacon), delayMs: LEAD + frac(at, span), count: 5, spread: "point", power: 0.4, colors: ROCKET.colors }));
    const cues = fortressRise({ D, tgt: TGT, intensity: NORMAL, view: fullBoardView(), landing: landing("p1"), leadMs: LEAD });
    expect(cues).toEqual([
      { kind: "figure", figure: "fortress", at: tid(TGT), tint: figureTint(ROCKET), delayMs: LEAD, durationMs: span + 300, side: "you" },
      ...flash(0.15),
      ...flash(0.4),
      { kind: "ring", preset: ROCKET.particle, at: tid(TGT), delayMs: slam, durationMs: expect.any(Number) as number },
      { kind: "burst", preset: ROCKET.particle, at: tid(TGT, FOOT), delayMs: slam, count: 18, spread: "area", power: 1, colors: ROCKET.colors },
      { kind: "shake", trauma: ENTRANCE_TRAUMA.fortressRise, delayMs: slam },
    ]);
    expect(ROCKET.colors).not.toEqual(FLAVOR_LOOKS.Tech.colors);
    // With no lead its fortress lands at once.
    expect(fortressRise({ D, tgt: TGT, intensity: NORMAL })[0]).toMatchObject({ delayMs: 0, durationMs: D + 300 });
  });

  it("R3405001 its fortress faces the landing's seat only when the entrance has both the board and the landing", () => {
    const view = fullBoardView();
    expect(fortressRise({ D, tgt: TGT, intensity: NORMAL, view, landing: landing("p2") })[0]).toMatchObject({ side: "opponent" });
    for (const plan of [{}, { view }, { landing: landing("p2") }]) {
      expect(fortressRise({ D, tgt: TGT, intensity: NORMAL, ...plan })[0]).not.toHaveProperty("side");
    }
  });

  it("R3405001 its Activate's convoy: the hangar doors open for 300 ms past the entry, a truck rolls out at 25% of it to the zone the Unit fills and unloads there in dust at its end", () => {
    const hq = tid(testid.card("ex"));
    const zone = testid.zone("you", "units", 4);
    const roll = frac(0.25, D);
    const [convoy] = run(HQ, [summon("p1", 4)], D, "p1", standing(), "activate");
    expect(convoy).toEqual({
      cues: [
        { kind: "figure", figure: "convoy", at: hq, tint: figureTint(ROCKET), delayMs: 0, durationMs: D + 300, side: "you" },
        { kind: "shot", shot: ROCKET.projectile, colors: ROCKET.colors, preset: ROCKET.particle, from: hq, to: tid(zone), delayMs: roll, flightMs: D - roll, density: NORMAL },
        { kind: "burst", preset: "dust", at: tid(zone, FOOT), delayMs: D, count: 10, spread: "point", power: 0.7, colors: ROCKET.colors },
      ],
      row: "replace",
    });
    // Off the board, its trucks roll out from its controller's hero.
    expect(run(HQ, [summon("p1", 4)], D, "p1", fullBoardView(), "activate")[0]?.cues[1]).toMatchObject({ kind: "shot", from: tid(testid.hero("you")) });
  });

  it("R2797000 only another Unit summoned for its controller under its play or its Activate is its convoy's: its own landing plays its entrance alone, and an event of another type, the other player's summon, and a summon under its Death or a turn hook keep their own rows", () => {
    const view = standing();
    const own = landing("p1");
    expect(landOf(own, D, 0, view)).toEqual({ cues: fortressRise({ D, tgt: TGT, intensity: NORMAL, view, landing: own, leadMs: 0, landMs: frac(FX_SLAM_AT, D) }), row: "replace" });
    const flickered: GameEvent = { type: "flickered", player: "p1", instanceId: "fx-unit-4", defId: SHREDDER, row: "units", lane: 4 };
    expect(run(HQ, [flickered, summon("p2", 4)], D, "p1", view, "activate")).toEqual([UNCLAIMED, UNCLAIMED]);
    for (const hook of ["death", "endOfTurn"] as const) expect(run(HQ, [summon("p1", 4)], D, "p1", view, hook)).toEqual([UNCLAIMED]);
    // Its play's own summon, as the gallery plays it, is its convoy's.
    expect(run(HQ, [summon("p1", 4)], D, "p1", view)[0]?.row).toBe("replace");
  });

  it("R3405001 its gallery fixture unloads a plain Jlockeed Unit into the summoning player's first empty unit zone, the first on a full board", () => {
    const board = fullBoardView();
    const view: PlayerView = { ...board, you: { ...board.you, units: emptied(board.you.units, [3, 4]) }, opponent: { ...board.opponent, units: emptied(board.opponent.units, [1]) } };
    const fixture = (player: PlayerId, on: PlayerView = view): GameEvent[] => jlockeedHqFixture(on, "ex", player);
    expect(fixture("p1")).toEqual([{ type: "summoned", player: "p1", instanceId: "fx-jlockeed", defId: SHREDDER, row: "units", lane: 4 }]);
    expect(fixture("p2")).toEqual([{ type: "summoned", player: "p2", instanceId: "fx-jlockeed", defId: SHREDDER, row: "units", lane: 2 }]);
    expect(fixture("p1", board)).toMatchObject([{ lane: 1 }]);
    expect(fixture("p2", board)).toMatchObject([{ lane: 1 }]);
    expect(CATALOG[SHREDDER]).toMatchObject({ type: "Unit", rarity: "Common" });
  });
});

describe("R3405001 Jlockeed's Headquarters' signature as a page load builds it", () => {
  it("R3405001 its look is the rocket sub-look, its beacons sit at its zone's top corners, and its fixture's Unit is Jlockeed Shredder-10", async () => {
    // Its look, its beacons' points and its fixture's card are built as the module loads: read from a module loaded inside the test, as scornboi.test.ts does.
    vi.resetModules();
    const fresh = await import("./jlockeedHq.ts");
    const cues = fresh.fortressRise({ D, tgt: TGT, intensity: NORMAL });
    expect(cues[0]).toMatchObject({ figure: "fortress", tint: figureTint(ROCKET) });
    expect(cues.filter((cue) => cue.kind === "burst" && cue.preset === "spark").map((cue) => (cue.kind === "burst" ? cue.at : undefined))).toEqual([
      tid(TGT, BEACONS[0]),
      tid(TGT, BEACONS[1]),
      tid(TGT, BEACONS[0]),
      tid(TGT, BEACONS[1]),
    ]);
    expect(fresh.jlockeedHqFixture(fullBoardView(), "ex", "p1")).toMatchObject([{ defId: SHREDDER }]);
  });
});
