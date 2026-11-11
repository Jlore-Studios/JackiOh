// R3407000: Sanctum (sanctum.ts), planned as FxLayer plans it (remember, then plan): its table entry, its
// entrance at D 400 and after a hold, its echo on its own landing from its owner's graveyard and deck piles
// beside that entrance, its Radiant Cry's mana ring; its gallery run within its Legendary ceiling, nothing
// with the effects off, R200 at five D's, and from the other seat no cue names a card or a hand card while
// every beat still draws (R202).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import { ceilingOf, cuesOfRun, endOf, envOf, expectGalleryWithinCeiling, galleryRun, planWithOffsets } from "../../../test/signaturePass.ts";
import { frac, tid } from "../../build.ts";
import { planCardFx } from "../../cardFx.ts";
import { FX_INTENSITY_SCALE, FX_MAX_TAIL_MS, FX_SLAM_AT, SIGNATURE_PERMANENT_MAX_TRAUMA } from "../../constants.ts";
import { ENTRANCES } from "../../entrances.ts";
import { paletteLook } from "../../flavors.ts";
import { galleryEntry } from "../../motifs/gallery.ts";
import type { FxCue, FxPlanEnv } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT } from "../recipe.ts";
import { FX_CARD_TABLE } from "../tables.ts";
import { sanctumEchoFixture } from "./sanctum.ts";
import { landMsAfter } from "./signature.ts";

const SANCTUM = "stinkyscape-152";
const D = 400;
const DS = [120, 200, 400, 732, 1400] as const;
const WAIT = 200;
const CARD_ID = /(core|classic|meditative|stinkyscape|traininggrounds)[a-z]*-\d/;
const SILVER = paletteLook("Physical/steel")?.colors;

type Planned = { cues: FxCue[]; row: string | undefined };

const beats = (cues: readonly FxCue[]): string[] => cues.map((cue) => `${cue.kind}@${String(cue.delayMs)}`);
const played = (player: PlayerId, instanceId: string): GameEvent => ({ type: "cardPlayed", player, instanceId, defId: SANCTUM, costPaid: 4 });
const resolved = (player: PlayerId, instanceId: string): GameEvent => ({ type: "cardResolved", player, instanceId, defId: SANCTUM, permanent: true, costPaid: 4 });
const landing = (player: PlayerId, instanceId: string, lane = 1): Extract<GameEvent, { type: "summoned" }> => ({ type: "summoned", player, instanceId, defId: SANCTUM, row: "backrow", lane });
const mana = (player: PlayerId): GameEvent => ({ type: "manaChanged", player, current: 6, max: 4 });

/** Each of `events` as its own entry lasting `ms`, after `before` is remembered: remember, then plan through the dispatch. */
function planEach(events: readonly GameEvent[], ms: number, before: readonly GameEvent[] = [], view: PlayerView = fullBoardView(), env: FxPlanEnv = envOf()): Planned[] {
  env.memory.remember(before);
  return events.map((event) => {
    env.memory.remember([event]);
    const entry = { ...galleryEntry([event], view), durationMs: ms };
    const result = planCardFx(event, { entry, view, env, D: ms });
    return { cues: result?.cues ?? [], row: result?.row };
  });
}

/** R200 for one beat planned at `ms`: it draws, starts inside its entry, lands by its end and fades within FX_MAX_TAIL_MS. */
function expectInside(cues: readonly FxCue[], ms: number, where: string): void {
  expect(cues.length, `${where} at D=${String(ms)}`).toBeGreaterThan(0);
  for (const cue of cues) {
    const label = `${where} ${cue.kind} at D=${String(ms)}`;
    expect(cue.delayMs, label).toBeGreaterThanOrEqual(0);
    expect(cue.delayMs, label).toBeLessThanOrEqual(ms);
    if (cue.kind === "shot") expect(cue.delayMs + cue.flightMs, label).toBeLessThanOrEqual(ms);
    expect(endOf(cue), label).toBeLessThanOrEqual(ms + FX_MAX_TAIL_MS);
    if (cue.kind === "shake") expect(cue.trauma, label).toBeLessThanOrEqual(SIGNATURE_PERMANENT_MAX_TRAUMA);
  }
}

describe("R3407000 Sanctum", () => {
  it("R3407000 Sanctum is a signature with its entrance, recipe and zone aura", () => {
    expect(FX_CARD_TABLE.get(SANCTUM)).toEqual({ kind: "bespoke", key: "sanctumEcho", entrance: "sanctumRise", aura: { look: "pages", palette: "Holy", scope: "zone" }, signature: true });
  });

  it("R3407000 its entrance: a silver porthole opens, the asteroid library rises through it at the slam, its shelves unfold and its pool ripples twice", () => {
    const view = fullBoardView();
    const tgt = testid.zone("you", "backrow", 1);
    const slam = frac(FX_SLAM_AT, D);
    const cues = ENTRANCES.sanctumRise({ D, tgt, intensity: FX_INTENSITY_SCALE.normal, view, landing: landing("p1", "sa") });
    expect(beats(cues)).toEqual(["ring@0", "rays@0", "burst@80", `burst@${String(slam)}`, `sheen@${String(slam)}`, "ring@300", "ring@348", `shake@${String(slam)}`]);
    expect(cues[0]).toMatchObject({ preset: "holy", at: tid(tgt) });
    expect(cues[1]).toMatchObject({ tone: "holy", at: tid(tgt) });
    expect(cues[2]).toMatchObject({ preset: "holy", spread: "ring", at: tid(tgt), colors: SILVER });
    expect(cues[3]).toMatchObject({ preset: "crumb", spread: "area", at: tid(tgt, FOOT), colors: SILVER });
    expect(cues[4]).toEqual({ kind: "sheen", at: tid(tgt), delayMs: slam, durationMs: D - slam });
    for (const ripple of cues.slice(5, 7)) expect(ripple).toMatchObject({ at: tid(tgt, FOOT) });
    expect(cues[7]).toEqual({ kind: "shake", trauma: ENTRANCE_TRAUMA.sanctumRise, delayMs: slam });
  });

  it("R3407000 its entrance follows its entry's hold, landing at the slam of the motion after it", () => {
    const cues = ENTRANCES.sanctumRise({ D, tgt: testid.zone("you", "backrow", 1), intensity: 1, view: fullBoardView(), landing: landing("p1", "sa"), landMs: landMsAfter(WAIT, D) });
    for (const cue of cues) expect(cue.delayMs, cue.kind).toBeGreaterThanOrEqual(WAIT);
    expect(cues.find((cue) => cue.kind === "shake")?.delayMs).toBe(WAIT + frac(FX_SLAM_AT, D - WAIT));
    expectInside(cues, D, "sanctumRise");
  });

  it("R3407000 its landing lights Flashback on its owner's graveyard and deck piles and a silver echo rises from each to its zone, beside its entrance", () => {
    const view = fullBoardView();
    const lands = landing("p1", "sa", 2);
    const zone = tid(testid.zone("you", "backrow", 2));
    const graveyard = tid(animTestid.graveyard("you"));
    const deck = tid(animTestid.library("you"));
    const [, landed] = planEach([played("p1", "sa"), lands], D, [], view);
    const entrance = ENTRANCES.sanctumRise({ D, tgt: testid.zone("you", "backrow", 2), intensity: 1, view, landing: lands });
    expect(landed?.row).toBe("replace");
    expect(landed?.cues.slice(0, entrance.length)).toEqual(entrance);
    const echo = landed?.cues.slice(entrance.length) ?? [];
    expect(beats(echo)).toEqual(["ring@0", "shot@200", "ring@0", "shot@200", "burst@360"]);
    expect(echo[0]).toMatchObject({ at: graveyard });
    expect(echo[1]).toMatchObject({ shot: { motion: "bolt" }, from: graveyard, to: zone, flightMs: 160, colors: SILVER });
    expect(echo[2]).toMatchObject({ at: deck });
    expect(echo[3]).toMatchObject({ from: deck, to: zone, flightMs: 160 });
    expect(echo[4]).toMatchObject({ spread: "point", at: zone, colors: SILVER });
  });

  it("R3407000 its Radiant Cry's mana rings its owner's mana pile in silver over the mana row; another's mana, or one outside its play, draws nothing of it", () => {
    const view = fullBoardView();
    const [cry] = planEach([mana("p1")], D, [played("p1", "sa"), landing("p1", "sa")], view);
    expect(cry).toEqual({ cues: [expect.objectContaining({ kind: "ring", preset: "holy", at: tid(animTestid.mana("you")), delayMs: 40 })], row: "keep" });
    expect(planEach([mana("p2")], D, [played("p1", "sa")], view)[0]).toEqual({ cues: [], row: undefined });
    expect(planEach([mana("p1")], D, [], view)[0]).toEqual({ cues: [], row: undefined });
  });

  it("R3407000 its gallery run, its landing and its Cry's mana, fits its Legendary ceiling", () => {
    expect(ceilingOf(SANCTUM)).toBe(1600);
    expectGalleryWithinCeiling(SANCTUM);
    expect(galleryRun(SANCTUM).events.map((event) => event.type)).toEqual(["cardPlayed", "summoned", "manaChanged", "cardResolved"]);
  });

  it("R3407000 nothing draws with the effects off", () => {
    expect(cuesOfRun(galleryRun(SANCTUM, envOf(FX_INTENSITY_SCALE.off)).run)).toEqual([]);
  });

  it("R3407000 R200 its entrance and each recipe beat start inside their entry and are gone within FX_MAX_TAIL_MS at D 120, 200, 400, 732 and 1400", () => {
    const view = fullBoardView();
    for (const ms of DS) {
      expectInside(ENTRANCES.sanctumRise({ D: ms, tgt: testid.zone("you", "backrow", 1), intensity: 1, view, landing: landing("p1", "sa") }), ms, "sanctumRise");
      const [, landed, cry] = planEach([played("p1", "sa"), landing("p1", "sa"), ...sanctumEchoFixture(view, "sa", "p1")], ms, [], view);
      expectInside(landed?.cues ?? [], ms, "sanctumEcho landing");
      expectInside(cry?.cues ?? [], ms, "sanctumEcho mana");
    }
  });

  it("R3407000 R202 from the other seat its beats still draw on their piles, naming no card and no hand card", () => {
    const view = fullBoardView();
    expect(view.viewer).toBe("p1");
    const run = planWithOffsets([played("p2", "theirs"), landing("p2", "theirs"), ...sanctumEchoFixture(view, "theirs", "p2"), resolved("p2", "theirs")], view);
    const cues = cuesOfRun(run);
    for (const pile of [animTestid.graveyard("opponent"), animTestid.library("opponent")]) {
      expect(cues.some((cue) => cue.kind === "shot" && cue.from.kind === "testid" && cue.from.testid === pile), pile).toBe(true);
    }
    expect(cues.some((cue) => cue.kind === "ring" && cue.at.kind === "testid" && cue.at.testid === animTestid.mana("opponent"))).toBe(true);
    const json = JSON.stringify(cues);
    expect(json).not.toMatch(CARD_ID);
    expect(json).not.toMatch(/hand-card-/);
  });
});
