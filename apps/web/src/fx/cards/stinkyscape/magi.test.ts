// R3407000: the three MAGI's signatures (magi.ts), planned as FxLayer plans them: each boots in its zone
// with an entrance of its own, its Activate and its Start of turn ballot draw their beats, its gallery play
// and its ballot each end within a Legendary's ceiling, and every cue keeps R200 at any entry length.
// Nothing draws with the effects off; a hand is only ever its pile and a card only ever its back (R202).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { ALLURE_MARK, ALLURE_MARK_COLOR } from "../../../cards/marks.ts";
import { animTestid } from "../../../game/animations.ts";
import { LANES, testid } from "../../../game/contract.ts";
import { faceUpBackrow, fullBoardView } from "../../../test/fixtures.ts";
import {
  ceilingOf,
  cuesOfRun,
  endOf,
  envOf,
  expectGalleryWithinCeiling,
  expectR200,
  latestEnd,
  planWithOffsets,
} from "../../../test/signaturePass.ts";
import { frac } from "../../build.ts";
import { planCardFx } from "../../cardFx.ts";
import { FX_INTENSITY_SCALE, FX_MAX_TAIL_MS, FX_SLAM_AT } from "../../constants.ts";
import { ENTRANCES } from "../../entrances.ts";
import { createFxMemory } from "../../memory.ts";
import { galleryEntry } from "../../motifs/gallery.ts";
import type { FxAnchor, FxCue, FxPlanEnv } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT } from "../recipe.ts";
import { FX_CARD_TABLE } from "../tables.ts";
import { balthasarCradleFixture, casperKissFixture, magiBallotFixture, melchiorTallyFixture } from "./magi.ts";
import { landMsAfter } from "./signature.ts";

const MELCHIOR = "stinkyscape-018-1";
const BALTHASAR = "stinkyscape-018-2";
const CASPER = "stinkyscape-018-3";
const MAGI = [MELCHIOR, BALTHASAR, CASPER] as const;
const ENTRANCE_OF = { [MELCHIOR]: "melchiorBoot", [BALTHASAR]: "balthasarLamp", [CASPER]: "casperMirror" } as const;

const tid = (value: string, at?: { x: number; y: number }): FxAnchor => (at === undefined ? { kind: "testid", testid: value } : { kind: "testid", testid: value, at });
const kinds = (cues: readonly FxCue[]): string[] => cues.map((cue) => cue.kind);
const landing = (player: PlayerId, instanceId: string, defId: string, lane = 2): Extract<GameEvent, { type: "summoned" }> => ({
  type: "summoned",
  player,
  instanceId,
  defId,
  row: "backrow",
  lane,
});

/** A MAGI standing in the viewer's second backrow zone, face up, under `instanceId`. */
function standingView(defId: string, instanceId = "magi"): PlayerView {
  const view = fullBoardView();
  const backrow = view.you.backrow.map((slot, k) => (k === 1 ? faceUpBackrow("p1", { defId, type: "Field Spell", instanceId }) : slot));
  return { ...view, you: { ...view.you, backrow } };
}

/** Each of `events` as its own entry lasting D, the play or hook remembered first: what the recipe draws for it. */
function beats(events: readonly GameEvent[], view: PlayerView, D: number, env: FxPlanEnv = envOf()): (FxCue[] | null)[] {
  return events.map((event) => {
    env.memory.remember([event]);
    const entry = { ...galleryEntry([event], view), durationMs: D };
    return planCardFx(event, { entry, view, env, D })?.cues ?? null;
  });
}

describe("R3407000 the MAGI's signatures", () => {
  it("R3407000 each MAGI is a signature: its own recipe, entrance and standing look on its zone", () => {
    expect(FX_CARD_TABLE.get(MELCHIOR)).toEqual({ kind: "bespoke", key: "melchiorTally", entrance: "melchiorBoot", aura: { look: "scanline", palette: "Tech/hologram", scope: "zone" }, signature: true });
    expect(FX_CARD_TABLE.get(BALTHASAR)).toEqual({ kind: "bespoke", key: "balthasarCradle", entrance: "balthasarLamp", aura: { look: "heartbeat", palette: "Clout", scope: "zone" }, signature: true });
    expect(FX_CARD_TABLE.get(CASPER)).toEqual({ kind: "bespoke", key: "casperKiss", entrance: "casperMirror", aura: { look: "glint", palette: "Dark", scope: "zone" }, signature: true });
  });

  it("R3407000 MELCHIOR boots: a core column at its foot, a scan line over the board, a figure over each of its side's lanes, landing with a ring and a shake", () => {
    const view = fullBoardView();
    const D = 400;
    const tgt = testid.zone("you", "backrow", 2);
    const slam = frac(FX_SLAM_AT, D);
    const cues = ENTRANCES.melchiorBoot({ D, tgt, intensity: FX_INTENSITY_SCALE.normal, view, landing: landing("p1", "m", MELCHIOR) });
    expect(cues[0]).toMatchObject({ kind: "burst", at: tid(tgt, FOOT) });
    expect(cues[1]).toMatchObject({ kind: "sheen", at: tid(testid.board) });
    expect(cues.filter((cue) => cue.kind === "ring")).toEqual([expect.objectContaining({ at: tid(tgt), delayMs: slam })]);
    const figures = cues.filter((cue) => cue.kind === "burst" && cue.at.kind === "testid" && cue.at.testid.startsWith("zone-you-units"));
    expect(figures.map((cue) => (cue.kind === "burst" && cue.at.kind === "testid" ? cue.at.testid : ""))).toEqual(LANES.map((lane) => testid.zone("you", "units", lane)));
    expect(cues.at(-1)).toEqual({ kind: "shake", trauma: ENTRANCE_TRAUMA.melchiorBoot, delayMs: slam });
    // On the other seat its figures stand over its own side, the opponent's.
    const theirs = ENTRANCES.melchiorBoot({ D, tgt: testid.zone("opponent", "backrow", 2), intensity: 1, view, landing: landing("p2", "m", MELCHIOR) });
    expect(theirs.some((cue) => cue.kind === "burst" && cue.at.kind === "testid" && cue.at.testid === testid.zone("opponent", "units", 1))).toBe(true);
  });

  it("R3407000 BALTHASAR warms on like a nursery lamp and traces a heart line to its owner's hero; CASPER flickers twice behind a mirror sheen", () => {
    const view = fullBoardView();
    const D = 400;
    const tgt = testid.zone("you", "backrow", 2);
    const slam = frac(FX_SLAM_AT, D);
    const lamp = ENTRANCES.balthasarLamp({ D, tgt, intensity: 1, view, landing: landing("p1", "b", BALTHASAR) });
    expect(kinds(lamp)).toEqual(["rays", "burst", "ring", "shot", "shake"]);
    expect(lamp[3]).toMatchObject({ from: tid(tgt), to: tid(testid.hero("you")) });
    expect(lamp[4]).toEqual({ kind: "shake", trauma: ENTRANCE_TRAUMA.balthasarLamp, delayMs: slam });
    const mirror = ENTRANCES.casperMirror({ D, tgt, intensity: 1, view, landing: landing("p1", "c", CASPER) });
    expect(kinds(mirror)).toEqual(["burst", "burst", "sheen", "ring", "burst", "shake"]);
    expect(mirror[2]).toMatchObject({ at: tid(tgt) });
    expect(mirror[5]).toEqual({ kind: "shake", trauma: ENTRANCE_TRAUMA.casperMirror, delayMs: slam });
  });

  it("R3407000 an entrance after a hold times its beats after it", () => {
    const view = fullBoardView();
    const D = 1400;
    const wait = 1000;
    const tgt = testid.zone("you", "backrow", 2);
    for (const defId of MAGI) {
      const key = ENTRANCE_OF[defId];
      const held = ENTRANCES[key]({ D, tgt, intensity: 1, view, landing: landing("p1", "m", defId), landMs: landMsAfter(wait, D) });
      for (const cue of held) expect(cue.delayMs, `${key} ${cue.kind}`).toBeGreaterThanOrEqual(wait);
      const shake = held.find((cue) => cue.kind === "shake");
      expect(shake?.delayMs, key).toBe(wait + frac(FX_SLAM_AT, D - wait));
    }
  });

  it("R3407000 MELCHIOR's Activate beams from its zone to each Unit it arms, keeping the keyword's own callout", () => {
    const view = standingView(MELCHIOR);
    const D = 200;
    const unit = view.you.units.find((u) => u !== null)?.instanceId ?? "";
    const [marker, grant] = beats(melchiorTallyFixture(view, "magi", "p1", MELCHIOR), view, D);
    expect(marker).toBeNull();
    expect(kinds(grant ?? [])).toEqual(["shot", "ring"]);
    expect(grant?.[0]).toMatchObject({ from: tid(testid.card("magi")), to: tid(testid.card(unit)) });
    // A grant outside its Activate is not its beat.
    const env = envOf();
    env.memory.remember([{ type: "hookResolving", sourceId: "magi", defId: MELCHIOR, controller: "p1", hook: "startOfTurn" }]);
    expect(beats([{ type: "keywordGranted", instanceId: unit, keyword: { kind: "Taunt" } }], view, D, env)).toEqual([null]);
  });

  it("R3407000 each MAGI's ballot drops on its zone in its colour; MELCHIOR's ticks each face-up Unit first, BALTHASAR's lines to its hero", () => {
    const D = 400;
    for (const defId of MAGI) {
      const view = standingView(defId);
      const [, vote] = beats(magiBallotFixture("magi", "p1", defId), view, D);
      expect(vote?.filter((cue) => cue.kind === "ring"), defId).toEqual([expect.objectContaining({ at: tid(testid.card("magi")) })]);
    }
    const melchior = standingView(MELCHIOR);
    const shown = [...melchior.you.units, ...melchior.opponent.units].filter((unit) => unit !== null && unit.defId !== "hidden").length;
    const [, tally] = beats(magiBallotFixture("magi", "p1", MELCHIOR), melchior, D);
    expect(tally?.filter((cue) => cue.kind === "burst").length).toBe(shown + 1);
    const [, worried] = beats(magiBallotFixture("magi", "p1", BALTHASAR), standingView(BALTHASAR), D);
    expect(worried?.find((cue) => cue.kind === "shot")).toMatchObject({ to: tid(testid.hero("you")) });
    // Another card's vote is not its beat.
    const env = envOf();
    env.memory.remember([{ type: "hookResolving", sourceId: "magi", defId: CASPER, controller: "p1", hook: "startOfTurn" }]);
    expect(beats([{ type: "voteCast", player: "p1", instanceId: "else", defId: "core-004", votes: 1, needed: 3 }], standingView(CASPER), D, env)).toEqual([null]);
  });

  it("R3407000 BALTHASAR's Activate cradles a card back from its zone into its owner's hand pile, on either seat", () => {
    const D = 250;
    const view = standingView(BALTHASAR);
    const [, cradled] = beats(balthasarCradleFixture(view, "magi", "p1", BALTHASAR), view, D);
    expect(cradled).toEqual([{ kind: "ghost", from: tid(testid.card("magi")), to: tid(animTestid.hand("you")), delayMs: frac(0.15, D), durationMs: frac(0.7, D) }]);
    // R202: from the other seat the card is hidden; the back still flies, to that player's pile.
    const env = envOf();
    env.memory.remember([{ type: "hookResolving", sourceId: "theirs", defId: BALTHASAR, controller: "p2", hook: "activate" }]);
    const [back] = beats([{ type: "addedToHand", player: "p2", instanceId: "hidden", defId: "hidden" }], fullBoardView(), D, env);
    expect(back).toEqual([expect.objectContaining({ kind: "ghost", to: tid(animTestid.hand("opponent")) })]);
    expect(JSON.stringify(back)).not.toMatch(/(core|classic|meditative|stinkyscape|traininggrounds)[a-z]*-\d|handCard/);
  });

  it("R3407000 CASPER's Allure blows a kiss of rose light at the Unit it marks, and a Unit won over bursts", () => {
    const view = standingView(CASPER);
    const D = 400;
    const foe = view.opponent.units.find((u) => u !== null && u.defId !== "hidden")?.instanceId ?? "";
    const [, kiss] = beats(casperKissFixture(view, "magi", "p1", CASPER), view, D);
    expect(kinds(kiss ?? [])).toEqual(["shot", "ring"]);
    expect(kiss?.[0]).toMatchObject({ from: tid(testid.card("magi")), to: tid(testid.card(foe)) });
    const env = envOf();
    env.memory.remember([{ type: "hookResolving", sourceId: "magi", defId: CASPER, controller: "p1", hook: "activate" }]);
    const [won] = beats([{ type: "controlChanged", instanceId: foe, controller: "p1", row: "units", lane: 2 }], view, D, env);
    expect(kinds(won ?? [])).toEqual(["shot", "burst"]);
    // Another mark is not its kiss.
    expect(beats([{ type: "marked", instanceId: foe, mark: "frozen", color: "#9ad", added: true }], view, D, env)).toEqual([null]);
    expect(beats([{ type: "marked", instanceId: foe, mark: ALLURE_MARK, color: ALLURE_MARK_COLOR, added: false }], view, D, env)).toEqual([null]);
  });

  it("R3407000 each MAGI's gallery play, and its ballot from its marker, end within a Legendary's ceiling", () => {
    for (const defId of MAGI) {
      expectGalleryWithinCeiling(defId);
      const run = planWithOffsets(magiBallotFixture("magi", "p1", defId), standingView(defId));
      expect(cuesOfRun(run).length, defId).toBeGreaterThan(0);
      expect(latestEnd(run), defId).toBeLessThanOrEqual(ceilingOf(defId));
      expectR200(run, defId);
    }
  });

  it("R3407000 R200 every MAGI cue starts inside its entry and ends within FX_MAX_TAIL_MS, at any length; nothing draws with the effects off", () => {
    const tgt = testid.zone("you", "backrow", 2);
    for (const D of [120, 200, 400, 732, 1400]) {
      for (const defId of MAGI) {
        const view = standingView(defId);
        const fixtures = [...magiBallotFixture("magi", "p1", defId), ...(defId === MELCHIOR ? melchiorTallyFixture(view, "magi", "p1", defId) : defId === BALTHASAR ? balthasarCradleFixture(view, "magi", "p1", defId) : casperKissFixture(view, "magi", "p1", defId))];
        const cues = [...ENTRANCES[ENTRANCE_OF[defId]]({ D, tgt, intensity: 1, view, landing: landing("p1", "magi", defId) }), ...beats(fixtures, view, D).flatMap((drawn) => drawn ?? [])];
        expect(cues.length, defId).toBeGreaterThan(0);
        for (const cue of cues) {
          const label = `${defId} ${cue.kind} at D=${String(D)}`;
          expect(cue.delayMs, label).toBeGreaterThanOrEqual(0);
          expect(cue.delayMs, label).toBeLessThanOrEqual(D);
          expect(endOf(cue), label).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
          if (cue.kind === "shake") expect(cue.trauma, label).toBeLessThanOrEqual(ENTRANCE_TRAUMA[ENTRANCE_OF[defId]]);
        }
      }
    }
    for (const defId of MAGI) {
      const env: FxPlanEnv = { intensity: FX_INTENSITY_SCALE.off, card: () => undefined, memory: createFxMemory() };
      const run = planWithOffsets([{ type: "cardPlayed", player: "p1", instanceId: "magi", defId, costPaid: 1 }, landing("p1", "magi", defId), ...magiBallotFixture("magi", "p1", defId)], fullBoardView(), env);
      expect(cuesOfRun(run), defId).toEqual([]);
    }
  });
});
