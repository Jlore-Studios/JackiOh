// R3405001: Double Header's signature (doubleHeader.ts), cue by cue, past what signatures.fields.test.ts
// holds: its floodlights and torn stub in Physical's sport look, every one after its entry's lead
// (R3405000), mirrored for the landing's seat only when the entrance knows both the board and the landing;
// the copy's ticket half in the same look, peeling off the last card its owner resolved (never the other
// player's, never its own) while the board still shows that card on the field, else off its own zone; and
// its gallery fixture's copy, added to its owner's hand.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { card, faceUpBackrow, fullBoardView } from "../../../test/fixtures.ts";
import { added, envOf, run, tid } from "../../../test/signatures.ts";
import { CATALOG } from "../../../wire/cards.ts";
import { frac } from "../../build.ts";
import { planCardFx } from "../../cardFx.ts";
import { FX_INTENSITY_SCALE, FX_SLAM_AT } from "../../constants.ts";
import { figureTint } from "../../figures.ts";
import { paletteLook, type FlavorLook } from "../../flavors.ts";
import { galleryEntry } from "../../motifs/gallery.ts";
import type { FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA } from "../recipe.ts";
import { doubleHeaderFixture, floodlights } from "./doubleHeader.ts";

const DOUBLE_HEADER = "meditative-011";
const D = 1000;
const LEAD = 200;
const TGT = testid.zone("you", "backrow", 1);
const OWN = tid(testid.card("ex"));
const NORMAL = FX_INTENSITY_SCALE.normal;

function lookOf(palette: string): FlavorLook {
  const look = paletteLook(palette);
  if (look === undefined) throw new Error(`no look ${palette}`);
  return look;
}

/** Physical's sport sub-look, a ballpark at night: not Physical's own sparks. */
const SPORT = lookOf("Physical/sport");

const landing = (player: PlayerId): Extract<GameEvent, { type: "summoned" }> => ({ type: "summoned", player, instanceId: "ex", defId: DOUBLE_HEADER, row: "backrow", lane: 1 });
const resolved = (player: PlayerId, instanceId: string, defId = "core-004"): GameEvent => ({ type: "cardResolved", player, instanceId, defId, permanent: true, costPaid: 1 });

/** The board with Double Header face up as "ex" in your first backrow zone, a card "fx-bounced" back in your hand. */
function fielded(): PlayerView {
  const view = fullBoardView();
  const hand = Array.isArray(view.you.hand) ? view.you.hand : [];
  return { ...view, you: { ...view.you, backrow: [faceUpBackrow("p1", { instanceId: "ex", defId: DOUBLE_HEADER }), ...view.you.backrow.slice(1)], hand: [...hand, card({ instanceId: "fx-bounced" })] } };
}

/** The copy's cues under its trigger, after `before` (what the turn resolved first). */
function copyAfter(view: PlayerView, before: readonly GameEvent[]): FxCue[] {
  const env = envOf();
  const copy = added("p1", "fx-copy", "core-004");
  env.memory.remember([...before, { type: "hookResolving", sourceId: "ex", defId: DOUBLE_HEADER, controller: "p1", hook: "whenever" }, copy]);
  return planCardFx(copy, { entry: { ...galleryEntry([copy], view), durationMs: D }, view, env, D })?.cues ?? [];
}

/** Where the copy peels off: its burst's anchor, which its ghost leaves from too. */
function peelsFrom(cues: readonly FxCue[]): unknown {
  const [burst, , ghost] = cues;
  expect(ghost?.kind === "ghost" ? ghost.from : null).toEqual(burst?.kind === "burst" ? burst.at : undefined);
  return burst?.kind === "burst" ? burst.at : undefined;
}

describe("R3405001 Double Header's signature, cue by cue", () => {
  it("R3405000 its floodlights swing on from its entry's lead and outlast it by 300 ms; the stub drops at the slam after the lead and outlasts it by 300 ms, in sport colours", () => {
    const slam = LEAD + frac(FX_SLAM_AT, D - LEAD);
    const tint = figureTint(SPORT);
    const cues = floodlights({ D, tgt: TGT, intensity: NORMAL, view: fullBoardView(), landing: landing("p1"), leadMs: LEAD });
    expect(cues).toEqual([
      { kind: "figure", figure: "floodlights", at: tid(TGT), tint, delayMs: LEAD, durationMs: D - LEAD + 300, side: "you" },
      { kind: "figure", figure: "ticketStub", at: tid(TGT), tint, delayMs: slam, durationMs: D - slam + 300, side: "you" },
      { kind: "ring", preset: SPORT.particle, at: tid(TGT), delayMs: slam, durationMs: expect.any(Number) as number },
      { kind: "burst", preset: SPORT.particle, at: tid(TGT), delayMs: slam, count: 18, spread: "ring", power: 0.9, colors: SPORT.colors },
      { kind: "shake", trauma: ENTRANCE_TRAUMA.floodlights, delayMs: slam },
    ]);
    // With no lead the floodlights swing on at once.
    expect(floodlights({ D, tgt: TGT, intensity: NORMAL })[0]).toMatchObject({ delayMs: 0, durationMs: D + 300 });
  });

  it("R3405001 its floodlights and stub face the landing's seat only when the entrance has both the board and the landing", () => {
    const view = fullBoardView();
    const theirs = floodlights({ D, tgt: TGT, intensity: NORMAL, view, landing: landing("p2") });
    expect(theirs.slice(0, 2)).toMatchObject([{ side: "opponent" }, { side: "opponent" }]);
    for (const plan of [{}, { view }, { landing: landing("p2") }]) {
      const cues = floodlights({ D, tgt: TGT, intensity: NORMAL, ...plan });
      expect(cues[0]).not.toHaveProperty("side");
      expect(cues[1]).not.toHaveProperty("side");
    }
  });

  it("R3405001 the copy peels off as an arcane after-image in sport colours, a small ticket half, folding into a card back at 30% of the entry, its sheen on its owner's seat", () => {
    const hand = tid(animTestid.hand("you"));
    const fold = frac(0.3, D);
    const [copy] = run(DOUBLE_HEADER, [added("p1", "fx-copy", "core-055")], D, "p1", fielded(), "whenever");
    expect(copy).toEqual({
      cues: [
        { kind: "burst", preset: "arcane", at: OWN, delayMs: 0, count: 12, spread: "point", power: 0.6, colors: SPORT.colors },
        { kind: "figure", figure: "ticketStub", at: OWN, tint: figureTint(SPORT), delayMs: 0, durationMs: D, side: "you", size: "small" },
        { kind: "ghost", from: OWN, to: hand, delayMs: fold, durationMs: D - fold },
        { kind: "sheen", at: hand, delayMs: fold, durationMs: D - fold },
      ],
      row: "replace",
    });
  });

  it("R202 the copy peels off the last card its owner resolved while the board shows it on the field: not an earlier one, not the other player's, not Double Header itself, and not one returned to its owner's hand", () => {
    const view = fielded();
    const [first, second] = view.you.units.map((unit) => unit?.instanceId ?? "");
    const theirs = view.opponent.units[0]?.instanceId ?? "";
    const on = (id: string): unknown => tid(testid.card(id));
    // The later of two of its owner's cards.
    expect(peelsFrom(copyAfter(view, [resolved("p1", first ?? ""), resolved("p1", second ?? "")]))).toEqual(on(second ?? ""));
    // Its owner's, past a card the other player resolved since.
    expect(peelsFrom(copyAfter(view, [resolved("p1", first ?? ""), resolved("p2", theirs)]))).toEqual(on(first ?? ""));
    // Its owner's, past Double Header's own resolution.
    expect(peelsFrom(copyAfter(view, [resolved("p1", first ?? ""), resolved("p1", "ex", DOUBLE_HEADER)]))).toEqual(on(first ?? ""));
    // A card its owner resolved that is back in their hand is off the field: its own zone.
    expect(peelsFrom(copyAfter(view, [resolved("p1", first ?? ""), resolved("p1", "fx-bounced")]))).toEqual(OWN);
  });

  it("R3405001 its gallery fixture adds the turn's first card's copy to its owner's hand", () => {
    const view = fullBoardView();
    expect(doubleHeaderFixture(view, "ex", "p1")).toEqual([{ type: "addedToHand", player: "p1", instanceId: "fx-second-game", defId: "core-055" }]);
    expect(doubleHeaderFixture(view, "ex", "p2")).toEqual([{ type: "addedToHand", player: "p2", instanceId: "fx-second-game", defId: "core-055" }]);
    expect(CATALOG["core-055"]?.name).toBe("Lava Golem");
  });
});
