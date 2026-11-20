// R3405001: Devin Bot's signature (devinBot.ts), cue by cue, past what signatures.units-a.test.ts holds: his
// entrance's cushion, puff, ring, shake and dimming eye-lights in Tech's hologram look, every one after his
// entry's lead (R3405000), the cushion mirrored for the landing's seat only when the entrance knows both the
// board and the landing; his Cry's wireframes, beamed into his own unit zones under his play alone (a
// backrow summon, a unit flickered or a summon under a hook of his is none of them); and his gallery
// fixture's 1/1, into the summoning player's first empty unit zone.

import type { GameEvent, PlayerId, PlayerView, UnitView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { testid } from "../../../game/contract.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import { landOf, run, tid } from "../../../test/signatures.ts";
import { CATALOG } from "../../../wire/cards.ts";
import { frac } from "../../build.ts";
import { FX_INTENSITY_SCALE, FX_SLAM_AT } from "../../constants.ts";
import { figureTint } from "../../figures.ts";
import { paletteLook, type FlavorLook } from "../../flavors.ts";
import { ENTRANCE_TRAUMA, FOOT } from "../recipe.ts";
import { cushionDrop, devinBotFixture } from "./devinBot.ts";

const DEVIN = "meditative-051";
const ONE_ONE = "core-004";
const D = 1000;
const LEAD = 200;
const LANE = 3;
const TGT = testid.zone("you", "units", LANE);
const NORMAL = FX_INTENSITY_SCALE.normal;
const UNCLAIMED = { cues: [], row: undefined };

function lookOf(palette: string): FlavorLook {
  const look = paletteLook(palette);
  if (look === undefined) throw new Error(`no look ${palette}`);
  return look;
}

/** Tech's hologram sub-look, his aura's: not Tech's own blue circuits. */
const HOLOGRAM = lookOf("Tech/hologram");

const landing = (player: PlayerId): Extract<GameEvent, { type: "summoned" }> => ({ type: "summoned", player, instanceId: "ex", defId: DEVIN, row: "units", lane: LANE });
const summon = (player: PlayerId, lane: number, row: "units" | "backrow" = "units"): GameEvent => ({ type: "summoned", player, instanceId: `fx-unit-${String(lane)}`, defId: ONE_ONE, row, lane });

/** `units` with the slots in `open` emptied. */
const emptied = (units: readonly (UnitView | null)[], open: readonly number[]): (UnitView | null)[] => units.map((unit, k) => (open.includes(k) ? null : unit));

/** The board with him standing as "ex" in your lane LANE and your lanes past it empty. */
function standing(): PlayerView {
  const view = fullBoardView();
  const units = view.you.units.map((unit, k) => (k === LANE - 1 && unit !== null ? { ...unit, instanceId: "ex" } : k >= LANE ? null : unit));
  return { ...view, you: { ...view.you, units } };
}

describe("R3405001 Devin Bot's signature, cue by cue", () => {
  it("R3405000 his cushion drops from his entry's lead and outlasts it by 300 ms; the puff, ring and shake land at the slam after the lead and his eye-lights dim at 80% of the motion, all in hologram light", () => {
    const span = D - LEAD;
    const slam = LEAD + frac(FX_SLAM_AT, span);
    const cues = cushionDrop({ D, tgt: TGT, intensity: NORMAL, view: fullBoardView(), landing: landing("p1"), leadMs: LEAD });
    expect(cues).toEqual([
      { kind: "figure", figure: "cushion", at: tid(TGT), tint: figureTint(HOLOGRAM), delayMs: LEAD, durationMs: span + 300, side: "you" },
      { kind: "burst", preset: "dust", at: tid(TGT, FOOT), delayMs: slam, count: 14, spread: "area", power: 0.6, colors: HOLOGRAM.colors },
      { kind: "ring", preset: HOLOGRAM.particle, at: tid(TGT), delayMs: slam, durationMs: expect.any(Number) as number },
      { kind: "shake", trauma: ENTRANCE_TRAUMA.cushionDrop, delayMs: slam },
      { kind: "burst", preset: HOLOGRAM.particle, at: tid(TGT, { x: 0.5, y: 0.35 }), delayMs: LEAD + frac(0.8, span), count: 6, spread: "point", power: 0.3, colors: HOLOGRAM.colors },
    ]);
    // With no lead his cushion lands at once.
    expect(cushionDrop({ D, tgt: TGT, intensity: NORMAL })[0]).toMatchObject({ delayMs: 0, durationMs: D + 300 });
  });

  it("R3405001 his cushion faces the landing's seat only when the entrance has both the board and the landing", () => {
    const view = fullBoardView();
    expect(cushionDrop({ D, tgt: TGT, intensity: NORMAL, view, landing: landing("p2") })[0]).toMatchObject({ side: "opponent" });
    for (const plan of [{}, { view }, { landing: landing("p2") }]) {
      expect(cushionDrop({ D, tgt: TGT, intensity: NORMAL, ...plan })[0]).not.toHaveProperty("side");
    }
  });

  it("R3405001 his Cry hums, beams a hologram wireframe into the zone it fills and bounces it solid at 60% of the entry, the wireframe outlasting the entry by 300 ms", () => {
    const view = standing();
    const him = tid(testid.card("ex"));
    const zone = testid.zone("you", "units", 4);
    const [fill] = run(DEVIN, [summon("p1", 4)], D, "p1", view);
    expect(fill).toEqual({
      cues: [
        { kind: "ring", preset: HOLOGRAM.particle, at: him, delayMs: 0, durationMs: expect.any(Number) as number },
        { kind: "shot", shot: HOLOGRAM.projectile, colors: HOLOGRAM.colors, preset: HOLOGRAM.particle, from: him, to: tid(zone), delayMs: 0, flightMs: frac(0.45, D), density: NORMAL },
        { kind: "figure", figure: "wireframe", at: tid(zone), tint: figureTint(HOLOGRAM), delayMs: 0, durationMs: D + 300, side: "you" },
        { kind: "burst", preset: HOLOGRAM.particle, at: tid(zone, FOOT), delayMs: frac(0.6, D), count: 8, spread: "point", power: 0.6, colors: HOLOGRAM.colors },
      ],
      row: "replace",
    });
  });

  it("R2797000 only another Unit summoned onto his own unit row under his play is his Cry's: his own landing plays his entrance alone, and a backrow summon, a unit flickered back and a summon under one of his hooks keep their own rows", () => {
    const view = standing();
    const own = landing("p1");
    expect(landOf(own, D, 0, view)).toEqual({ cues: cushionDrop({ D, tgt: TGT, intensity: NORMAL, view, landing: own, leadMs: 0, landMs: frac(FX_SLAM_AT, D) }), row: "replace" });
    const flickered: GameEvent = { type: "flickered", player: "p1", instanceId: "fx-unit-4", defId: ONE_ONE, row: "units", lane: 4 };
    const locked: GameEvent = { type: "locked", player: "p1", row: "units", lane: 4 };
    expect(run(DEVIN, [summon("p1", 2, "backrow"), flickered, locked], D, "p1", view)).toEqual([UNCLAIMED, UNCLAIMED, UNCLAIMED]);
    for (const hook of ["death", "endOfTurn", "activate"] as const) expect(run(DEVIN, [summon("p1", 4)], D, "p1", view, hook)).toEqual([UNCLAIMED]);
  });

  it("R3405001 his gallery fixture summons a plain 1/1 into the summoning player's first empty unit zone, the first on a full board", () => {
    const board = fullBoardView();
    const view: PlayerView = { ...board, you: { ...board.you, units: emptied(board.you.units, [3, 4]) }, opponent: { ...board.opponent, units: emptied(board.opponent.units, [1]) } };
    const fixture = (player: PlayerId, on: PlayerView = view): GameEvent[] => devinBotFixture(on, "ex", player);
    expect(fixture("p1")).toEqual([{ type: "summoned", player: "p1", instanceId: "fx-devin-1-1", defId: ONE_ONE, row: "units", lane: 4 }]);
    expect(fixture("p2")).toEqual([{ type: "summoned", player: "p2", instanceId: "fx-devin-1-1", defId: ONE_ONE, row: "units", lane: 2 }]);
    expect(fixture("p1", board)).toMatchObject([{ lane: 1 }]);
    expect(fixture("p2", board)).toMatchObject([{ lane: 1 }]);
    expect(CATALOG[ONE_ONE]).toMatchObject({ name: "Gary the Gambler", type: "Unit", rarity: "Common" });
  });
});
