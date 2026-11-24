// R3405001: Jade Beauty's signature (jadeBeauty.ts), cue by cue, past what signatures.units-b.test.ts holds:
// her entrance's boulder, crack, ring, sleeve dust and shake in Hustle's jade look, every one after her entry's
// lead (R3405000), the boulder mirrored for the landing's seat only when the entrance knows both the board and
// the landing; her Allure's ribbons, a fan lifted at the first, under her play or her turn hook alone (another
// mark, Allure lifted, another event or another hook of hers is none of them); her gallery fixture's mark on
// the other player's first Unit; and the points and look her module builds as it loads.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it, vi } from "vitest";

import { ALLURE_MARK, ALLURE_MARK_COLOR, DUO_MARK, paletteFor } from "../../../cards/marks.ts";
import { testid } from "../../../game/contract.ts";
import { baseView, emptySide, fullBoardView, unit } from "../../../test/fixtures.ts";
import { run, tid } from "../../../test/signatures.ts";
import { frac } from "../../build.ts";
import { FX_CENTER, FX_INTENSITY_SCALE, FX_SLAM_AT } from "../../constants.ts";
import { figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook, type FlavorLook } from "../../flavors.ts";
import { ENTRANCE_TRAUMA } from "../recipe.ts";
import { jadeBeautyFixture, jadeBoulder } from "./jadeBeauty.ts";

const JADE_BEAUTY = "meditative-039-5";
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

/** Hustle's jade sub-look, her aura's: not Hustle's own gold. */
const JADE = lookOf("Hustle/jade");

/** Her ribbon: Allure's pink core and rim, and her jade's rim. */
const PINK = paletteFor(ALLURE_MARK_COLOR);
const RIBBON = [PINK.core, PINK.rim, JADE.colors[1]];

const landing = (player: PlayerId): Extract<GameEvent, { type: "summoned" }> => ({ type: "summoned", player, instanceId: "ex", defId: JADE_BEAUTY, row: "units", lane: LANE });
const marked = (instanceId: string, mark: string = ALLURE_MARK, added = true): GameEvent => ({ type: "marked", instanceId, mark, color: ALLURE_MARK_COLOR, added });

/** The board with her standing as "ex" in your lane LANE. */
function standing(): PlayerView {
  const view = fullBoardView();
  return { ...view, you: { ...view.you, units: view.you.units.map((shown, k) => (k === LANE - 1 && shown !== null ? { ...shown, instanceId: "ex" } : shown)) } };
}

/** The opponent's Unit in board slot `k`. */
function foeAt(view: PlayerView, k: number): string {
  const foe = view.opponent.units[k];
  if (foe === undefined || foe === null) throw new Error(`no foe in slot ${String(k)}`);
  return foe.instanceId;
}

describe("R3405001 Jade Beauty's signature, cue by cue", () => {
  it("R3405000 her boulder rises from her entry's lead and outlasts it by 300 ms; it cracks and rings at the slam after the lead, dust falls from both sleeves at 80% of the motion, all in jade", () => {
    const span = D - LEAD;
    const slam = LEAD + frac(FX_SLAM_AT, span);
    const sleeves = LEAD + frac(0.8, span);
    const cues = jadeBoulder({ D, tgt: TGT, intensity: NORMAL, view: fullBoardView(), landing: landing("p1"), leadMs: LEAD });
    expect(cues).toEqual([
      { kind: "figure", figure: "jadeBoulder", at: tid(TGT), tint: figureTint(JADE), delayMs: LEAD, durationMs: span + 300, side: "you" },
      { kind: "burst", preset: JADE.particle, at: tid(TGT), delayMs: slam, count: 22, spread: "ring", power: 1.1, colors: JADE.colors },
      { kind: "ring", preset: JADE.particle, at: tid(TGT), delayMs: slam, durationMs: expect.any(Number) as number },
      { kind: "burst", preset: "dust", at: tid(TGT, { x: 0.2, y: 0.55 }), delayMs: sleeves, count: 6, spread: "point", power: 0.4, colors: JADE.colors },
      { kind: "burst", preset: "dust", at: tid(TGT, { x: 0.8, y: 0.55 }), delayMs: sleeves, count: 6, spread: "point", power: 0.4, colors: JADE.colors },
      { kind: "shake", trauma: ENTRANCE_TRAUMA.jadeBoulder, delayMs: slam },
    ]);
    expect(JADE.colors).not.toEqual(FLAVOR_LOOKS.Hustle.colors);
    // With no lead her boulder rises at once.
    expect(jadeBoulder({ D, tgt: TGT, intensity: NORMAL })[0]).toMatchObject({ delayMs: 0, durationMs: D + 300 });
  });

  it("R3405001 her boulder faces the landing's seat only when the entrance has both the board and the landing", () => {
    const view = fullBoardView();
    expect(jadeBoulder({ D, tgt: TGT, intensity: NORMAL, view, landing: landing("p2") })[0]).toMatchObject({ side: "opponent" });
    for (const plan of [{}, { view }, { landing: landing("p2") }]) {
      expect(jadeBoulder({ D, tgt: TGT, intensity: NORMAL, ...plan })[0]).not.toHaveProperty("side");
    }
  });

  it("R3405001 at her Allure's first mark she lifts her silk fan, 300 ms past the entry; a pink-jade ribbon drifts from her at 15% of the entry and charms its Unit in sparkles at 70%, one at the board's centre for a Unit this seat cannot see", () => {
    const view = standing();
    const her = tid(testid.card("ex"));
    const charmed = tid(testid.card(foeAt(view, 0)));
    const drift = frac(0.15, D);
    const land = frac(0.7, D);
    const ribbon = (to: ReturnType<typeof tid>) => [
      { kind: "shot", shot: { motion: "bolt", trail: "sparkle", wave: true }, colors: RIBBON, preset: "sparkle", from: her, to, delayMs: drift, flightMs: land - drift, density: NORMAL },
      { kind: "burst", preset: "sparkle", at: to, delayMs: land, count: 10, spread: "ring", power: 0.6, colors: RIBBON },
    ];
    const [first, hidden] = run(JADE_BEAUTY, [marked(foeAt(view, 0)), marked("hidden")], D, "p1", view, "endOfTurn");
    expect(first).toEqual({
      cues: [{ kind: "figure", figure: "silkFan", at: her, tint: figureTint(JADE), delayMs: 0, durationMs: D + 300, side: "you" }, ...ribbon(charmed)],
      row: "replace",
    });
    expect(hidden).toEqual({ cues: ribbon(tid(testid.board, FX_CENTER)), row: "replace" });
  });

  it("R2797000 only Allure laid, under her play or her turn hook, is hers: another mark, Allure lifted, another event, and Allure under her Death or an Activate keep their own rows", () => {
    const view = standing();
    const foe = foeAt(view, 0);
    const locked: GameEvent = { type: "locked", player: "p2", row: "units", lane: 1 };
    expect(run(JADE_BEAUTY, [marked(foe, DUO_MARK), marked(foe, ALLURE_MARK, false), locked], D, "p1", view, "endOfTurn")).toEqual([UNCLAIMED, UNCLAIMED, UNCLAIMED]);
    for (const hook of ["death", "activate"] as const) expect(run(JADE_BEAUTY, [marked(foe)], D, "p1", view, hook)).toEqual([UNCLAIMED]);
    // Her play's own mark, as the gallery plays it, is hers.
    expect(run(JADE_BEAUTY, [marked(foe)], D, "p1", view)[0]?.row).toBe("replace");
  });

  it("R3405001 her gallery fixture lays Allure on the other player's first Unit, and none on an empty board", () => {
    const view = baseView({
      you: emptySide("p1", { units: [null, unit("p1", { instanceId: "mine" }), null, null, null] }),
      opponent: emptySide("p2", { hand: { count: 4 }, units: [null, null, unit("p2", { instanceId: "foe" }), null, null] }),
    });
    expect(jadeBeautyFixture(view, "ex", "p1")).toEqual([{ type: "marked", instanceId: "foe", mark: ALLURE_MARK, color: ALLURE_MARK_COLOR, added: true }]);
    expect(jadeBeautyFixture(view, "ex", "p2")).toEqual([{ type: "marked", instanceId: "mine", mark: ALLURE_MARK, color: ALLURE_MARK_COLOR, added: true }]);
    expect(jadeBeautyFixture(baseView(), "ex", "p1")).toEqual([]);
  });
});

describe("R3405001 Jade Beauty's signature as a page load builds it", () => {
  it("R3405001 her look is the jade sub-look and her sleeves hang at either side of her zone", async () => {
    // Her look and her sleeves' points are built as the module loads: read from a module loaded inside the test, as scornboi.test.ts does.
    vi.resetModules();
    const fresh = await import("./jadeBeauty.ts");
    const cues = fresh.jadeBoulder({ D, tgt: TGT, intensity: NORMAL });
    expect(cues[0]).toMatchObject({ figure: "jadeBoulder", tint: figureTint(JADE) });
    expect(cues.filter((cue) => cue.kind === "burst" && cue.preset === "dust").map((cue) => (cue.kind === "burst" ? cue.at : undefined))).toEqual([
      tid(TGT, { x: 0.2, y: 0.55 }),
      tid(TGT, { x: 0.8, y: 0.55 }),
    ]);
  });
});
