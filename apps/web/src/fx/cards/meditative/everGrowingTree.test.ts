// R3405001: Ever Growing Tree's signature (everGrowingTree.ts), cue by cue, past what
// signatures.fields.test.ts holds: its sapling, seed, ring, roots and shake in Nature's green, every one
// after its entry's lead (R3405000), the sapling mirrored for the landing's seat only when the entrance knows
// both the board and the landing; its Activate's glow (its own Activate's alone), its owner's Discover's
// sheen (their own prompt's alone) and the fusion's branch outlasting the entry by 200 ms; and its gallery
// fixture, its Activate played out.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { testid } from "../../../game/contract.ts";
import { faceUpBackrow, fullBoardView } from "../../../test/fixtures.ts";
import { run, tid } from "../../../test/signatures.ts";
import { frac } from "../../build.ts";
import { FX_INTENSITY_SCALE, FX_SLAM_AT } from "../../constants.ts";
import { figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import { ENTRANCE_TRAUMA, FOOT } from "../recipe.ts";
import { everGrowingTreeFixture, treeSprout } from "./everGrowingTree.ts";

const TREE = "meditative-075";
const D = 1000;
const LEAD = 200;
const TGT = testid.zone("you", "backrow", 1);
const OWN = tid(testid.card("ex"));
const NORMAL = FX_INTENSITY_SCALE.normal;
const NATURE = FLAVOR_LOOKS.Nature;
const UNCLAIMED = { cues: [], row: undefined };

const landing = (player: PlayerId): Extract<GameEvent, { type: "summoned" }> => ({ type: "summoned", player, instanceId: "ex", defId: TREE, row: "backrow", lane: 1 });

/** The board with the Tree face up as "ex" in your first backrow zone. */
function fielded(): PlayerView {
  const view = fullBoardView();
  return { ...view, you: { ...view.you, backrow: [faceUpBackrow("p1", { instanceId: "ex", defId: TREE }), ...view.you.backrow.slice(1)] } };
}

describe("R3405001 Ever Growing Tree's signature, cue by cue", () => {
  it("R3405000 its sapling rises from its entry's lead and outlasts it by 300 ms; its seed splits 30% of the way from the lead to the slam, and its ring, roots and shake land at the slam after the lead, in Nature's green", () => {
    const slam = LEAD + frac(FX_SLAM_AT, D - LEAD);
    const split = LEAD + frac(0.3, slam - LEAD);
    const cues = treeSprout({ D, tgt: TGT, intensity: NORMAL, view: fullBoardView(), landing: landing("p1"), leadMs: LEAD });
    expect(cues).toEqual([
      { kind: "figure", figure: "sapling", at: tid(TGT), tint: figureTint(NATURE), delayMs: LEAD, durationMs: D - LEAD + 300, side: "you" },
      { kind: "burst", preset: NATURE.particle, at: tid(TGT, FOOT), delayMs: split, count: 8, spread: "point", power: 0.4, colors: NATURE.colors },
      { kind: "ring", preset: NATURE.particle, at: tid(TGT), delayMs: slam, durationMs: expect.any(Number) as number },
      { kind: "burst", preset: NATURE.particle, at: tid(TGT, FOOT), delayMs: slam, count: 20, spread: "ring", power: 1, colors: NATURE.colors },
      { kind: "shake", trauma: ENTRANCE_TRAUMA.treeSprout, delayMs: slam },
    ]);
    // With no lead the sapling rises at once.
    expect(treeSprout({ D, tgt: TGT, intensity: NORMAL })[0]).toMatchObject({ delayMs: 0, durationMs: D + 300 });
  });

  it("R3405001 its sapling faces the landing's seat only when the entrance has both the board and the landing", () => {
    const view = fullBoardView();
    expect(treeSprout({ D, tgt: TGT, intensity: NORMAL, view, landing: landing("p2") })[0]).toMatchObject({ side: "opponent" });
    for (const plan of [{}, { view }, { landing: landing("p2") }]) {
      expect(treeSprout({ D, tgt: TGT, intensity: NORMAL, ...plan })[0]).not.toHaveProperty("side");
    }
  });

  it("R2797000 its own Activate glows over it in green, and its owner's Discover sheens over it; another card's Activate and the other player's prompt keep their own rows", () => {
    const view = fielded();
    const [glow, theirPrompt, otherActivate] = run(
      TREE,
      [
        { type: "activated", player: "p1", instanceId: "ex", defId: TREE, ability: "grow" },
        { type: "promptOpened", player: "p2" },
        { type: "activated", player: "p1", instanceId: "fx-other", defId: "core-020", ability: "grow" },
      ],
      D,
      "p1",
      view,
      "activate",
    );
    expect(glow).toEqual({
      cues: [
        { kind: "ring", preset: NATURE.particle, at: OWN, delayMs: 0, durationMs: expect.any(Number) as number },
        { kind: "burst", preset: NATURE.particle, at: OWN, delayMs: 0, count: 12, spread: "area", power: 0.5, colors: NATURE.colors },
      ],
      row: "replace",
    });
    expect([theirPrompt, otherActivate]).toEqual([UNCLAIMED, UNCLAIMED]);
  });

  it("R3405001 the Field Spell fused into it sprouts a green branch over it, outlasting the entry by 200 ms, a ring with it", () => {
    const [branch] = run(TREE, [{ type: "fused", instanceIds: ["ex", "fx-field"], resultInstanceId: "ex", defId: TREE }], D, "p1", fielded(), "activate");
    expect(branch).toEqual({
      cues: [
        { kind: "figure", figure: "branch", at: OWN, tint: figureTint(NATURE), delayMs: 0, durationMs: D + 200, side: "you" },
        { kind: "ring", preset: NATURE.particle, at: OWN, delayMs: 0, durationMs: expect.any(Number) as number },
      ],
      row: "replace",
    });
  });

  it("R3405001 its gallery fixture plays its Activate out: the tree glows, its owner's Discover opens, and the Field Spell chosen fuses into it", () => {
    const view = fielded();
    const fixture = everGrowingTreeFixture(view, "ex", "p1");
    expect(fixture).toEqual([
      { type: "activated", player: "p1", instanceId: "ex", defId: TREE, ability: "grow" },
      { type: "promptOpened", player: "p1" },
      { type: "fused", instanceIds: ["ex", "fx-fused-field"], resultInstanceId: "ex", defId: TREE },
    ]);
    expect(run(TREE, fixture, D, "p1", view, "activate").map((result) => result.cues.map((cue) => (cue.kind === "figure" ? cue.figure : cue.kind)))).toEqual([["ring", "burst"], ["sheen"], ["branch", "ring"]]);
    expect(everGrowingTreeFixture(view, "fx-tree", "p2")[0]).toMatchObject({ player: "p2", instanceId: "fx-tree" });
  });
});
