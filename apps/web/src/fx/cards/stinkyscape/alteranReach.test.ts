// R3407000: Alteran Reach's and Edge of the Universe's signatures (alteranReach.ts), planned as FxLayer plans
// them: the Field Spell dives into its zone after its takeover's hold, its Activate jumps the ship along its
// path, rays at the edge and a card back to the hand; the Edge's cast lights its caster's hero and greys the
// other, its `fled` peels the board in from its edges. Each run within its Mythic ceiling, R200 at any length,
// nothing with the effects off, and from the opponent's seat no cue names a card (R202).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { faceUpBackrow, fullBoardView } from "../../../test/fixtures.ts";
import {
  asTheirsAll,
  beatsOf,
  ceilingOf,
  cuesOfRun,
  envOf,
  expectCuesR200,
  expectGalleryWithinCeiling,
  expectNoCard,
  expectR200,
  latestEnd,
  planWithOffsets,
} from "../../../test/signaturePass.ts";
import { frac } from "../../build.ts";
import { FX_INTENSITY_SCALE, FX_SLAM_AT, SIGNATURE_SPELL_MAX_TRAUMA } from "../../constants.ts";
import { ENTRANCES } from "../../entrances.ts";
import type { FxAnchor, FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA } from "../recipe.ts";
import { FX_CARD_TABLE } from "../tables.ts";
import { edgeFleeFixture, starscapeJumpFixture } from "./alteranReach.ts";
import { landMsAfter } from "./signature.ts";

const REACH = "stinkyscape-148";
const EDGE = "stinkyscape-148-1";
const DS = [120, 200, 400, 732, 1400] as const;

const tid = (value: string, at?: { x: number; y: number }): FxAnchor => (at === undefined ? { kind: "testid", testid: value } : { kind: "testid", testid: value, at });
const kinds = (cues: readonly FxCue[] | undefined): string[] => (cues ?? []).map((cue) => cue.kind);
const marker = (player: PlayerId, defId = REACH): GameEvent => ({ type: "hookResolving", sourceId: "reach", defId, controller: player, hook: "activate" });
const added = (player: PlayerId): GameEvent => ({ type: "addedToHand", player, instanceId: "fx-edge", defId: EDGE });
const played = (player: PlayerId): GameEvent => ({ type: "cardPlayed", player, instanceId: "edge", defId: EDGE, costPaid: 0 });
const resolved = (player: PlayerId): GameEvent => ({ type: "cardResolved", player, instanceId: "edge", defId: EDGE, permanent: false, costPaid: 0 });
const fled = (player: PlayerId): GameEvent => ({ type: "fled", player, opponent: "draw" });

/** Alteran Reach standing face up in the viewer's second backrow zone, under "reach". */
function standingView(): PlayerView {
  const view = fullBoardView();
  const backrow = view.you.backrow.map((slot, k) => (k === 1 ? faceUpBackrow("p1", { defId: REACH, type: "Field Spell", instanceId: "reach" }) : slot));
  return { ...view, you: { ...view.you, backrow } };
}

describe("R3407000 Alteran Reach's and Edge of the Universe's signatures", () => {
  it("R3407000 their table entries are their signatures, each Mythic with its own takeover look", () => {
    expect(FX_CARD_TABLE.get(REACH)).toEqual({ kind: "bespoke", key: "starscapeJump", entrance: "alteranDive", aura: { look: "starpath", palette: "Tech/starfield", scope: "zone" }, takeover: "alteran", signature: true });
    expect(FX_CARD_TABLE.get(EDGE)).toEqual({ kind: "bespoke", key: "edgeFlee", takeover: "edge", signature: true });
  });

  it("R3407000 Alteran Reach dives into its zone: a sheen over the board, three stations, a star streak from the top edge, its star map at the slam", () => {
    const D = 400;
    const tgt = testid.zone("you", "backrow", 2);
    const slam = frac(FX_SLAM_AT, D);
    const cues = ENTRANCES.alteranDive({ D, tgt, intensity: 1, view: fullBoardView() });
    expect(kinds(cues)).toEqual(["sheen", "burst", "burst", "burst", "shot", "ring", "burst", "burst", "sheen", "shake"]);
    expect(cues[0]).toMatchObject({ at: tid(testid.board) });
    expect(cues[4]).toMatchObject({ from: tid(testid.board, { x: 0.5, y: 0 }), to: tid(tgt) });
    expect(cues[5]).toMatchObject({ at: tid(tgt), delayMs: slam });
    expect(cues.at(-1)).toEqual({ kind: "shake", trauma: ENTRANCE_TRAUMA.alteranDive, delayMs: slam });
    // After its takeover's hold, every beat waits for it.
    const wait = 1000;
    const held = ENTRANCES.alteranDive({ D: 1400, tgt, intensity: 1, landMs: landMsAfter(wait, 1400) });
    for (const cue of held) expect(cue.delayMs, cue.kind).toBeGreaterThanOrEqual(wait);
    expect(held.at(-1)?.delayMs).toBe(wait + frac(FX_SLAM_AT, 1400 - wait));
  });

  it("R3407000 its Activate jumps the ship across its zone, rays at the edge, and the Edge it adds flies out as a card back", () => {
    const view = standingView();
    const D = 400;
    const zone = testid.card("reach");
    const [hook, edge, back] = beatsOf([...starscapeJumpFixture(view, "reach", "p1"), added("p1")], view, D);
    expect(hook).toBeNull();
    expect(kinds(edge?.cues)).toEqual(["shot", "burst", "rays"]);
    expect(edge?.cues[0]).toMatchObject({ from: tid(zone, { x: 0.2, y: 0.5 }), to: tid(zone, { x: 0.8, y: 0.5 }) });
    expect(edge?.cues[2]).toMatchObject({ at: tid(zone) });
    expect(back).toEqual({ cues: [{ kind: "ghost", from: tid(zone), to: tid(animTestid.hand("you")), delayMs: 0, durationMs: frac(0.8, D) }], row: "replace" });
    // A node short of the edge draws no rays.
    const [step] = beatsOf([{ type: "starscapeTravelled", player: "p1", node: 3, reward: "heal" }], view, D, [marker("p1")]);
    expect(kinds(step?.cues)).toEqual(["shot", "burst"]);
    // Another card's travel, or one outside its Activate, is not its beat.
    expect(beatsOf([{ type: "starscapeTravelled", player: "p1", node: 3, reward: "heal" }], view, D, [marker("p1", "core-004")])).toEqual([null]);
    expect(beatsOf([{ type: "starscapeTravelled", player: "p1", node: 3, reward: "heal" }], view, D)).toEqual([null]);
  });

  it("R3407000 Edge of the Universe lights its caster's hero and greys the other's; its fleeing peels the board in from its edges", () => {
    const view = fullBoardView();
    const D = 400;
    expect(edgeFleeFixture(view, "edge", "p1")).toEqual([]);
    const [cast, flight] = beatsOf([played("p1"), fled("p1")], view, D);
    expect(kinds(cast?.cues)).toEqual(["ring", "burst", "burst", "shake"]);
    expect(cast?.cues[0]).toMatchObject({ at: tid(testid.hero("you")) });
    expect(cast?.cues[2]).toMatchObject({ at: tid(testid.hero("opponent")), preset: "smoke" });
    expect(kinds(flight?.cues)).toEqual(["walls", "sheen", "ring"]);
    expect(flight?.cues[0]).toMatchObject({ at: tid(testid.board) });
    expect(flight?.cues[1]).toMatchObject({ at: tid(testid.hero("you")) });
    // The other player's fleeing is not its beat.
    expect(beatsOf([{ type: "fled", player: "p2", opponent: "draw" }], view, D, [played("p1")])).toEqual([null]);
  });

  it("R3407000 R200 every cue of both starts inside its entry and ends within FX_MAX_TAIL_MS, at any length; nothing draws with the effects off", () => {
    const tgt = testid.zone("you", "backrow", 2);
    const view = standingView();
    for (const D of DS) {
      expectCuesR200(ENTRANCES.alteranDive({ D, tgt, intensity: 1, view }), D, "alteranDive", ENTRANCE_TRAUMA.alteranDive);
      const reach = beatsOf([...starscapeJumpFixture(view, "reach", "p1"), added("p1")], view, D).flatMap((beat) => beat?.cues ?? []);
      const edge = beatsOf([played("p1"), fled("p1")], view, D).flatMap((beat) => beat?.cues ?? []);
      expect(reach.length).toBeGreaterThan(0);
      expectCuesR200(reach, D, "starscapeJump");
      expectCuesR200(edge, D, "edgeFlee", SIGNATURE_SPELL_MAX_TRAUMA);
    }
    const off = envOf(FX_INTENSITY_SCALE.off);
    expect(cuesOfRun(planWithOffsets([played("p1"), fled("p1"), resolved("p1")], view, off))).toEqual([]);
    expect(cuesOfRun(planWithOffsets([...starscapeJumpFixture(view, "reach", "p1"), added("p1")], view, envOf(FX_INTENSITY_SCALE.off)))).toEqual([]);
  });

  it("R3407000 each gallery play, the Edge's card back from its Activate's marker and the fleeing from its own entry end within their Mythic ceilings", () => {
    expectGalleryWithinCeiling(REACH);
    expectGalleryWithinCeiling(EDGE);
    const view = standingView();
    const run = planWithOffsets([...starscapeJumpFixture(view, "reach", "p1"), added("p1")], view);
    expect(latestEnd(run)).toBeLessThanOrEqual(ceilingOf(REACH));
    expectR200(run, REACH);
    const env = envOf();
    env.memory.remember([played("p1")]);
    const flight = planWithOffsets([fled("p1"), resolved("p1")], fullBoardView(), env);
    expect(cuesOfRun(flight).some((cue) => cue.kind === "walls")).toBe(true);
    expect(latestEnd(flight)).toBeLessThanOrEqual(ceilingOf(EDGE));
    expectR200(flight, EDGE);
  });

  it("R3407000 R202 from the opponent's seat the Edge it adds is a back to their pile and no cue names a card", () => {
    const view = standingView();
    const theirs: PlayerView = { ...view, you: view.opponent, opponent: view.you };
    const events = asTheirsAll([...starscapeJumpFixture(view, "reach", "p1"), added("p1")]);
    const [, , back] = beatsOf(events, fullBoardView(), 400);
    expect(back?.cues).toEqual([expect.objectContaining({ kind: "ghost", to: tid(animTestid.hand("opponent")) })]);
    const cues = cuesOfRun(planWithOffsets(asTheirsAll([played("p1"), ...edgeFleeFixture(theirs, "edge", "p1"), fled("p1"), resolved("p1")]), fullBoardView()));
    expect(cues.length).toBeGreaterThan(0);
    expectNoCard(cues, EDGE);
    expectNoCard(back?.cues ?? [], REACH);
  });
});
