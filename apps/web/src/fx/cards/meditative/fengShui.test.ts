// R3405001: Feng Shui's signature (fengShui.ts), cue by cue, past what signatures.fields.test.ts holds: its
// compass, ring, alignment burst and shake in Hustle's jade, every one after its entry's lead (R3405000),
// the compass mirrored for the landing's seat only when the entrance knows both the board and the landing;
// its Activate's blossom and the Rock rolling off it in the same jade, for its owner's own card alone (not
// the other player's, nor another event of its owner's); and its gallery fixture's Rock, added to its
// owner's hand.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { faceUpBackrow, fullBoardView } from "../../../test/fixtures.ts";
import { added, run, tid } from "../../../test/signatures.ts";
import { CATALOG } from "../../../wire/cards.ts";
import { frac } from "../../build.ts";
import { FX_INTENSITY_SCALE, FX_SLAM_AT } from "../../constants.ts";
import { figureTint } from "../../figures.ts";
import { paletteLook, type FlavorLook } from "../../flavors.ts";
import { ENTRANCE_TRAUMA } from "../recipe.ts";
import { fengShuiFixture, fiveCompass } from "./fengShui.ts";

const FENG_SHUI = "meditative-040";
const ROCK = "meditative-039-1";
const D = 1000;
const LEAD = 200;
const TGT = testid.zone("you", "backrow", 1);
const OWN = tid(testid.card("ex"));
const NORMAL = FX_INTENSITY_SCALE.normal;
const UNCLAIMED = { cues: [], row: undefined };

function lookOf(palette: string): FlavorLook {
  const look = paletteLook(palette);
  if (look === undefined) throw new Error(`no look ${palette}`);
  return look;
}

/** Hustle's jade, the CN set's: not Hustle's own gold. */
const JADE = lookOf("Hustle/jade");

const landing = (player: PlayerId): Extract<GameEvent, { type: "summoned" }> => ({ type: "summoned", player, instanceId: "ex", defId: FENG_SHUI, row: "backrow", lane: 1 });

/** The board with Feng Shui face up as "ex" in your first backrow zone. */
function fielded(): PlayerView {
  const view = fullBoardView();
  return { ...view, you: { ...view.you, backrow: [faceUpBackrow("p1", { instanceId: "ex", defId: FENG_SHUI }), ...view.you.backrow.slice(1)] } };
}

describe("R3405001 Feng Shui's signature, cue by cue", () => {
  it("R3405000 its compass unfolds from its entry's lead and outlasts it by 300 ms; it clicks into alignment at the slam after the lead, in jade", () => {
    const slam = LEAD + frac(FX_SLAM_AT, D - LEAD);
    const cues = fiveCompass({ D, tgt: TGT, intensity: NORMAL, view: fullBoardView(), landing: landing("p1"), leadMs: LEAD });
    expect(cues).toEqual([
      { kind: "figure", figure: "baguaCompass", at: tid(TGT), tint: figureTint(JADE), delayMs: LEAD, durationMs: D - LEAD + 300, side: "you" },
      { kind: "ring", preset: JADE.particle, at: tid(TGT), delayMs: slam, durationMs: expect.any(Number) as number },
      { kind: "burst", preset: JADE.particle, at: tid(TGT), delayMs: slam, count: 20, spread: "ring", power: 1, colors: JADE.colors },
      { kind: "shake", trauma: ENTRANCE_TRAUMA.fiveCompass, delayMs: slam },
    ]);
    // With no lead the compass unfolds at once.
    expect(fiveCompass({ D, tgt: TGT, intensity: NORMAL })[0]).toMatchObject({ delayMs: 0, durationMs: D + 300 });
  });

  it("R3405001 its compass faces the landing's seat only when the entrance has both the board and the landing", () => {
    const view = fullBoardView();
    expect(fiveCompass({ D, tgt: TGT, intensity: NORMAL, view, landing: landing("p2") })[0]).toMatchObject({ side: "opponent" });
    for (const plan of [{}, { view }, { landing: landing("p2") }]) {
      expect(fiveCompass({ D, tgt: TGT, intensity: NORMAL, ...plan })[0]).not.toHaveProperty("side");
    }
  });

  it("R3405001 its Activate opens a jade blossom over it and rolls the Rock off it at 25% of the entry, tumbling into its owner's hand pile under their sheen", () => {
    const hand = tid(animTestid.hand("you"));
    const roll = frac(0.25, D);
    const [rock] = run(FENG_SHUI, [added("p1", "fx-rock", ROCK)], D, "p1", fielded(), "activate");
    expect(rock).toEqual({
      cues: [
        { kind: "figure", figure: "fengBlossom", at: OWN, tint: figureTint(JADE), delayMs: 0, durationMs: D, side: "you" },
        { kind: "burst", preset: JADE.particle, at: OWN, delayMs: 0, count: 12, spread: "area", power: 0.6, colors: JADE.colors },
        { kind: "ghost", from: OWN, to: hand, delayMs: roll, durationMs: D - roll, spin: true },
        { kind: "sheen", at: hand, delayMs: roll, durationMs: D - roll },
      ],
      row: "replace",
    });
  });

  it("R2797000 under its Activate, a card the other player gets and another event of its owner's keep their own rows", () => {
    const drawn: GameEvent = { type: "drawn", player: "p1", instanceId: "fx-drawn", defId: ROCK };
    expect(run(FENG_SHUI, [added("p2", "hidden", "hidden"), drawn], D, "p1", fielded(), "activate")).toEqual([UNCLAIMED, UNCLAIMED]);
  });

  it("R3405001 its gallery fixture adds its Activate's Auspicious Rock to its owner's hand", () => {
    const view = fullBoardView();
    expect(fengShuiFixture(view, "ex", "p1")).toEqual([{ type: "addedToHand", player: "p1", instanceId: "fx-rock", defId: ROCK }]);
    expect(fengShuiFixture(view, "ex", "p2")).toEqual([{ type: "addedToHand", player: "p2", instanceId: "fx-rock", defId: ROCK }]);
    expect(CATALOG[ROCK]?.name).toBe("Auspicious Rock");
  });
});
