// R3407000: Shockwave447's signature (shockwave447.ts), planned as FxLayer plans it: after his own
// takeover's hold his diagrams scrawl out over the board and crash into his zone from its edges in a double
// shockwave, his keywords snapping on as icons orbiting his card; his Cry's shockwave beams to each hand pile
// as its first card goes to the graveyard, and each card refilling a hand drops in as a back. His gallery play
// within a Mythic's ceiling, R200 at any length, nothing with the effects off, and from the opponent's seat
// no cue names a card (R202).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import {
  asTheirsAll,
  beatsOf,
  ceilingOf,
  cuesOfRun,
  envOf,
  expectCuesR200,
  expectGalleryWithinCeiling,
  expectNoCard,
  latestEnd,
  planWithOffsets,
} from "../../../test/signaturePass.ts";
import { frac } from "../../build.ts";
import { FX_INTENSITY_SCALE, FX_SLAM_AT } from "../../constants.ts";
import { ENTRANCES } from "../../entrances.ts";
import type { FxAnchor, FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA } from "../recipe.ts";
import { FX_CARD_TABLE } from "../tables.ts";
import { shockwaveLectureFixture } from "./shockwave447.ts";

const SHOCKWAVE = "stinkyscape-111";
const DS = [120, 200, 400, 732, 1400] as const;

const tid = (value: string, at?: { x: number; y: number }): FxAnchor => (at === undefined ? { kind: "testid", testid: value } : { kind: "testid", testid: value, at });
const kinds = (cues: readonly FxCue[] | undefined): string[] => (cues ?? []).map((cue) => cue.kind);
const played = (player: PlayerId): GameEvent => ({ type: "cardPlayed", player, instanceId: "sw", defId: SHOCKWAVE, costPaid: 7 });
const resolved = (player: PlayerId): GameEvent => ({ type: "cardResolved", player, instanceId: "sw", defId: SHOCKWAVE, permanent: true, costPaid: 7 });
const binned = (owner: PlayerId, id: string): GameEvent => ({ type: "enteredGraveyard", instanceId: id, defId: "core-002", owner });
const added = (player: PlayerId, id: string): GameEvent => ({ type: "addedToHand", player, instanceId: id, defId: "core-003" });
/** His Cry over both hands: each side's old cards to its graveyard, then the new ones. */
const lecture = (player: PlayerId, other: PlayerId): GameEvent[] => [binned(player, "h1"), binned(player, "h2"), binned(other, "o1"), binned(other, "o2"), added(player, "n1"), added(other, "m1")];

/** The viewer's first Unit as him, landed in lane 1 with its keywords. */
function landedView(): { view: PlayerView; landing: Extract<GameEvent, { type: "summoned" }> } {
  const view = fullBoardView();
  const unit = view.you.units[0];
  if (unit === null || unit === undefined) throw new Error("no unit to land");
  return { view, landing: { type: "summoned", player: "p1", instanceId: unit.instanceId, defId: SHOCKWAVE, row: "units", lane: 1 } };
}

describe("R3407000 Shockwave447's signature", () => {
  it("R3407000 his table entry is his signature, with his own takeover look", () => {
    expect(FX_CARD_TABLE.get(SHOCKWAVE)).toEqual({ kind: "bespoke", key: "shockwaveLecture", entrance: "shockwaveRing", aura: { look: "arcflick", palette: "Tech", scope: "tile" }, takeover: "shockwave", signature: true });
  });

  it("R3407000 his diagrams scrawl out and crash into his zone from three edges in a double shockwave, a callout per keyword he shows orbiting his card", () => {
    const { view, landing } = landedView();
    const D = 400;
    const tgt = testid.zone("you", "units", 1);
    const slam = frac(FX_SLAM_AT, D);
    const cues = ENTRANCES.shockwaveRing({ D, tgt, intensity: 1, view, landing });
    expect(kinds(cues)).toEqual(["sheen", "burst", "shot", "shot", "shot", "ring", "ring", "burst", "shake", "callout"]);
    expect(cues[0]).toMatchObject({ at: tid(testid.board) });
    for (const shot of cues.filter((cue) => cue.kind === "shot")) expect(shot).toMatchObject({ to: tid(tgt) });
    expect(cues[8]).toEqual({ kind: "shake", trauma: ENTRANCE_TRAUMA.shockwaveRing, delayMs: slam });
    expect(cues[9]).toMatchObject({ kind: "callout", keyword: "Taunt" });
    // A landing the view does not show pops no callout.
    expect(kinds(ENTRANCES.shockwaveRing({ D, tgt, intensity: 1, view, landing: { ...landing, instanceId: "elsewhere" } }))).not.toContain("callout");
    const wait = 1000;
    const held = ENTRANCES.shockwaveRing({ D: 1400, tgt, intensity: 1, view, landing, wait });
    for (const cue of held) expect(cue.delayMs, cue.kind).toBeGreaterThanOrEqual(wait);
  });

  it("R3407000 his Cry beams to each hand pile as its first card goes, a back falling to its graveyard; each refill drops in as a back to its pile", () => {
    const view = fullBoardView();
    const D = 300;
    const [first, second, theirs, , mine, refilled] = beatsOf(lecture("p1", "p2"), view, D, [played("p1")]);
    expect(kinds(first?.cues)).toEqual(["shot", "ring", "burst", "burst", "ghost"]);
    expect(first?.cues[0]).toMatchObject({ from: tid(testid.hero("you")), to: tid(animTestid.hand("you")) });
    expect(first?.cues[4]).toMatchObject({ from: tid(animTestid.hand("you")), to: tid(animTestid.graveyard("you")) });
    expect(second).toBeNull();
    expect(theirs?.cues[0]).toMatchObject({ to: tid(animTestid.hand("opponent")) });
    expect(mine).toEqual({ cues: [{ kind: "ghost", from: tid(testid.hero("you")), to: tid(animTestid.hand("you")), delayMs: 0, durationMs: D }], row: "replace" });
    expect(refilled?.cues[0]).toMatchObject({ kind: "ghost", to: tid(animTestid.hand("opponent")) });
    // Another card's play is not his lecture.
    expect(beatsOf([binned("p1", "x")], view, D, [{ ...played("p1"), instanceId: "x2", defId: "core-002" }])).toEqual([null]);
  });

  it("R3407000 R200 every cue starts inside its entry and ends within FX_MAX_TAIL_MS, at any length; nothing draws with the effects off", () => {
    const { view, landing } = landedView();
    const tgt = testid.zone("you", "units", 1);
    for (const D of DS) {
      expectCuesR200(ENTRANCES.shockwaveRing({ D, tgt, intensity: 1, view, landing }), D, "shockwaveRing", ENTRANCE_TRAUMA.shockwaveRing);
      const cry = beatsOf(lecture("p1", "p2"), view, D, [played("p1")]).flatMap((beat) => beat?.cues ?? []);
      expect(cry.length).toBeGreaterThan(0);
      expectCuesR200(cry, D, "shockwaveLecture");
    }
    expect(cuesOfRun(planWithOffsets([played("p1"), landing, ...lecture("p1", "p2"), resolved("p1")], view, envOf(FX_INTENSITY_SCALE.off)))).toEqual([]);
  });

  it("R3407000 his gallery play, and the first refill after its graveyard, end within a Mythic's ceiling", () => {
    expectGalleryWithinCeiling(SHOCKWAVE);
    const { view, landing } = landedView();
    const run = planWithOffsets([played("p1"), landing, ...shockwaveLectureFixture(view, "sw", "p1"), added("p1", "n1"), resolved("p1")], view);
    expect(latestEnd(run)).toBeLessThanOrEqual(ceilingOf(SHOCKWAVE));
  });

  it("R3407000 R202 from the opponent's seat the cards refilling the hands are backs to their piles and no cue names a card", () => {
    const view = fullBoardView();
    const cues = cuesOfRun(planWithOffsets(asTheirsAll([played("p1"), ...lecture("p1", "p2"), resolved("p1")]), view));
    const ghosts = cues.filter((cue) => cue.kind === "ghost");
    expect(ghosts.length).toBeGreaterThan(0);
    expectNoCard(cues, SHOCKWAVE);
  });
});
