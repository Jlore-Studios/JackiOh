// R3407000: Job Application's and Paycheck's signatures (jobApplication.ts), planned as FxLayer plans them:
// a pneumatic capsule thunks into the Quest's zone and lays its resume and office door under a flickering
// light; its Activate zips the resume in from the hand pile as a back, each stage lights a step and completes
// with the door swinging open, and each card its hooks add flies out as a back. Paycheck's slip feeds out over
// its caster's hero and snaps into a chip; its mana, draw and Credits ring and slide in as backs. Each run within
// its Legendary ceiling, R200 at any length, nothing with the effects off, and from the opponent's seat no cue
// names a card (R202).

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
import { FX_INTENSITY_SCALE, FX_SLAM_AT } from "../../constants.ts";
import { ENTRANCES } from "../../entrances.ts";
import type { FxAnchor, FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA } from "../recipe.ts";
import { FX_CARD_TABLE } from "../tables.ts";
import { jobInterviewFixture, paydayFixture } from "./jobApplication.ts";

const JOB = "stinkyscape-112";
const PAYCHECK = "stinkyscape-112-1";
const DS = [120, 200, 400, 732, 1400] as const;

const tid = (value: string, at?: { x: number; y: number }): FxAnchor => (at === undefined ? { kind: "testid", testid: value } : { kind: "testid", testid: value, at });
const kinds = (cues: readonly FxCue[] | undefined): string[] => (cues ?? []).map((cue) => cue.kind);
const hook = (player: PlayerId, kind: "activate" | "startOfTurn"): GameEvent => ({ type: "hookResolving", sourceId: "job", defId: JOB, controller: player, hook: kind });
const progressed = (player: PlayerId): Extract<GameEvent, { type: "questProgressed" }> => ({ type: "questProgressed", player, instanceId: "job", quest: "interview", progress: 1, goal: 3 });
const completed = (player: PlayerId): GameEvent => ({ type: "questCompleted", player, instanceId: "job", quest: "interview" });
const added = (player: PlayerId): GameEvent => ({ type: "addedToHand", player, instanceId: "fx-paycheck", defId: PAYCHECK });
const cashed = (player: PlayerId): GameEvent => ({ type: "cardPlayed", player, instanceId: "pay", defId: PAYCHECK, costPaid: 0 });
const banked = (player: PlayerId): GameEvent => ({ type: "cardResolved", player, instanceId: "pay", defId: PAYCHECK, permanent: false, costPaid: 0 });
/** Paycheck's whole Payday, as its caster's seat streams it. */
const payday = (player: PlayerId): GameEvent[] => [
  cashed(player),
  { type: "manaChanged", player, current: 6, max: 4 },
  ...paydayFixture(fullBoardView(), "pay", player),
  { type: "marketCreditsChanged", player, value: 5 },
  banked(player),
];

/** Job Application standing face up in the viewer's second backrow zone, under "job". */
function standingView(): PlayerView {
  const view = fullBoardView();
  const backrow = view.you.backrow.map((slot, k) => (k === 1 ? faceUpBackrow("p1", { defId: JOB, type: "Field Spell", instanceId: "job" }) : slot));
  return { ...view, you: { ...view.you, backrow } };
}

describe("R3407000 Job Application's and Paycheck's signatures", () => {
  it("R3407000 their table entries are their signatures", () => {
    expect(FX_CARD_TABLE.get(JOB)).toEqual({ kind: "bespoke", key: "jobInterview", entrance: "resumeTube", aura: { look: "steplights", palette: "Hustle/cash", scope: "zone" }, signature: true });
    expect(FX_CARD_TABLE.get(PAYCHECK)).toEqual({ kind: "bespoke", key: "payday", signature: true });
  });

  it("R3407000 a capsule thunks into its zone, the resume and the door laid there under a light flickering on", () => {
    const D = 400;
    const tgt = testid.zone("you", "backrow", 2);
    const slam = frac(FX_SLAM_AT, D);
    const cues = ENTRANCES.resumeTube({ D, tgt, intensity: 1, view: fullBoardView() });
    expect(kinds(cues)).toEqual(["shot", "ring", "burst", "figure", "figure", "burst", "burst", "rays", "shake"]);
    expect(cues[0]).toMatchObject({ to: tid(tgt) });
    expect(cues[1]).toMatchObject({ at: tid(tgt), delayMs: slam });
    expect(cues.at(-1)).toEqual({ kind: "shake", trauma: ENTRANCE_TRAUMA.resumeTube, delayMs: slam });
  });

  it("R3407000 its Activate zips the resume in as a back; its stages light a step and swing the door open; its hooks' cards fly out as backs", () => {
    const view = standingView();
    const D = 400;
    const office = tid(testid.card("job"));
    const [marker, held] = beatsOf(jobInterviewFixture(view, "job", "p1"), view, D);
    expect(marker).toBeNull();
    expect(held?.cues[0]).toMatchObject({ kind: "ghost", from: tid(animTestid.hand("you")), to: office });
    expect(kinds(held?.cues)).toEqual(["ghost", "burst"]);
    const [step, done] = beatsOf([progressed("p1"), completed("p1")], view, D, [hook("p1", "activate")]);
    expect(kinds(step?.cues)).toEqual(["burst", "rays"]);
    expect(step?.cues[0]).toMatchObject({ at: tid(testid.card("job"), { x: 0.5, y: 0.1 }) });
    expect(kinds(done?.cues)).toEqual(["ring", "burst"]);
    expect(done?.cues[1]).toMatchObject({ at: office, preset: "gold" });
    const [paid] = beatsOf([added("p1")], view, D, [hook("p1", "startOfTurn")]);
    expect(paid?.cues).toEqual([expect.objectContaining({ kind: "ghost", from: office, to: tid(animTestid.hand("you")) })]);
    // Another quest's stage is not its beat.
    expect(beatsOf([{ ...progressed("p1"), instanceId: "else" }], view, D, [hook("p1", "activate")])).toEqual([null]);
  });

  it("R3407000 Paycheck's slip feeds out over its caster's hero and snaps into a chip; its mana and Credits ring, its draw slides in as a back", () => {
    const view = fullBoardView();
    const D = 400;
    const hero = tid(testid.hero("you"));
    const [cast, mana, draw, credits] = beatsOf(payday("p1").slice(0, -1), view, D);
    expect(cast?.row).toBe("replace");
    expect(kinds(cast?.cues)).toEqual(["figure", "sheen", "burst"]);
    expect(cast?.cues[2]).toMatchObject({ at: hero, preset: "gold" });
    expect(mana?.cues).toEqual([expect.objectContaining({ kind: "ring", at: tid(animTestid.mana("you")) })]);
    expect(draw).toEqual({ cues: [{ kind: "ghost", from: tid(animTestid.library("you")), to: tid(animTestid.hand("you")), delayMs: 0, durationMs: D }], row: "replace" });
    expect(credits?.cues).toEqual([expect.objectContaining({ kind: "ring", at: hero })]);
    // The other player's mana is not its Payday.
    expect(beatsOf([{ type: "manaChanged", player: "p2", current: 1, max: 1 }], view, D, [cashed("p1")])).toEqual([null]);
  });

  it("R3407000 R200 every cue of both starts inside its entry and ends within FX_MAX_TAIL_MS, at any length; nothing draws with the effects off", () => {
    const view = standingView();
    const tgt = testid.zone("you", "backrow", 2);
    for (const D of DS) {
      expectCuesR200(ENTRANCES.resumeTube({ D, tgt, intensity: 1, view }), D, "resumeTube", ENTRANCE_TRAUMA.resumeTube);
      const quest = [
        ...beatsOf(jobInterviewFixture(view, "job", "p1"), view, D),
        ...beatsOf([progressed("p1"), completed("p1")], view, D, [hook("p1", "activate")]),
        ...beatsOf([added("p1")], view, D, [hook("p1", "startOfTurn")]),
      ].flatMap((beat) => beat?.cues ?? []);
      const pay = beatsOf(payday("p1"), view, D).flatMap((beat) => beat?.cues ?? []);
      expect(quest.length).toBeGreaterThan(0);
      expectCuesR200(quest, D, "jobInterview");
      expectCuesR200(pay, D, "payday");
    }
    expect(cuesOfRun(planWithOffsets(payday("p1"), view, envOf(FX_INTENSITY_SCALE.off)))).toEqual([]);
    expect(cuesOfRun(planWithOffsets([...jobInterviewFixture(view, "job", "p1"), progressed("p1"), completed("p1")], view, envOf(FX_INTENSITY_SCALE.off)))).toEqual([]);
  });

  it("R3407000 each gallery play, the stages from its Activate's marker and each Payday beat from its own entry end within their Legendary ceilings", () => {
    expectGalleryWithinCeiling(JOB);
    expectGalleryWithinCeiling(PAYCHECK);
    const view = standingView();
    const stages = planWithOffsets([hook("p1", "activate"), progressed("p1"), completed("p1")], view);
    expect(latestEnd(stages)).toBeLessThanOrEqual(ceilingOf(JOB));
    expectR200(stages, JOB);
    // The gallery plays its cast and its draw; its mana and its Credits each follow in an entry of their own.
    for (const beat of payday("p1").slice(1, -1)) {
      const env = envOf();
      env.memory.remember([cashed("p1")]);
      const run = planWithOffsets([beat, banked("p1")], fullBoardView(), env);
      expect(cuesOfRun(run).length, beat.type).toBeGreaterThan(0);
      expect(latestEnd(run), beat.type).toBeLessThanOrEqual(ceilingOf(PAYCHECK));
      expectR200(run, PAYCHECK);
    }
  });

  it("R3407000 R202 from the opponent's seat the resume, the cards it adds and Paycheck's draw are backs between piles and no cue names a card", () => {
    const view = fullBoardView();
    const cues = cuesOfRun([
      ...planWithOffsets(asTheirsAll([...jobInterviewFixture(view, "job", "p2"), hook("p1", "startOfTurn"), added("p1")]), view),
      ...planWithOffsets(asTheirsAll(payday("p1")), view),
    ]);
    expect(cues.filter((cue) => cue.kind === "ghost").length).toBeGreaterThan(1);
    expectNoCard(cues, JOB);
  });
});
