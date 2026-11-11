// R3407000: Stimmycheck's signature (stimmycheck.ts), planned as FxLayer plans it: a delivery drone swoops
// from the board's top-left edge to its caster's hero and the envelope bursts in credit chips there, then its
// mana rings on the mana pile in place of the mana row. Its gallery play within the Legendary Spell's ceiling,
// R200 at any length, nothing with the effects off, and the same beats from the opponent's seat (R202).

import type { GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import { asTheirsAll, beatsOf, cuesOfRun, envOf, expectCuesR200, expectGalleryWithinCeiling, expectNoCard, planWithOffsets } from "../../../test/signaturePass.ts";
import { frac } from "../../build.ts";
import { FX_INTENSITY_SCALE, SIGNATURE_SPELL_MAX_TRAUMA } from "../../constants.ts";
import type { FxAnchor, FxCue } from "../../types.ts";
import { FX_CARD_TABLE } from "../tables.ts";
import { stimmyDropFixture } from "./stimmycheck.ts";

const STIMMY = "stinkyscape-153";
const DS = [120, 200, 400, 732, 1400] as const;

const tid = (value: string, at?: { x: number; y: number }): FxAnchor => (at === undefined ? { kind: "testid", testid: value } : { kind: "testid", testid: value, at });
const kinds = (cues: readonly FxCue[] | undefined): string[] => (cues ?? []).map((cue) => cue.kind);
const played = (player: PlayerId): GameEvent => ({ type: "cardPlayed", player, instanceId: "stimmy", defId: STIMMY, costPaid: 0 });
const resolved = (player: PlayerId): GameEvent => ({ type: "cardResolved", player, instanceId: "stimmy", defId: STIMMY, permanent: false, costPaid: 0 });
const playOf = (player: PlayerId): GameEvent[] => [played(player), ...stimmyDropFixture(fullBoardView(), "stimmy", player), resolved(player)];

describe("R3407000 Stimmycheck's signature", () => {
  it("R3407000 its table entry is its signature", () => {
    expect(FX_CARD_TABLE.get(STIMMY)).toEqual({ kind: "bespoke", key: "stimmyDrop", signature: true });
  });

  it("R3407000 a drone swoops from the top-left edge to its caster's hero, the envelope bursting in credit chips; its mana rings on the mana pile", () => {
    const view = fullBoardView();
    const D = 400;
    const hero = tid(testid.hero("you"));
    const [cast, mana] = beatsOf([played("p1"), ...stimmyDropFixture(view, "stimmy", "p1")], view, D);
    expect(cast?.row).toBe("replace");
    expect(kinds(cast?.cues)).toEqual(["shot", "ring", "burst", "shake"]);
    expect(cast?.cues[0]).toMatchObject({ from: tid(testid.board, { x: 0, y: 0 }), to: hero, delayMs: 0, flightMs: frac(0.4, D) });
    expect(cast?.cues[2]).toMatchObject({ at: hero, preset: "gold", delayMs: frac(0.4, D) });
    expect(mana).toEqual({ cues: [expect.objectContaining({ kind: "ring", preset: "gold", at: tid(animTestid.mana("you")) })], row: "replace" });
    // Another card's mana, or the other player's, is not its beat.
    expect(beatsOf([{ type: "manaChanged", player: "p1", current: 2, max: 2 }], view, D, [{ type: "cardPlayed", player: "p1", instanceId: "x", defId: "core-002", costPaid: 1 }])).toEqual([null]);
    expect(beatsOf([{ type: "manaChanged", player: "p2", current: 2, max: 2 }], view, D, [played("p1")])).toEqual([null]);
  });

  it("R3407000 R200 every cue starts inside its entry and ends within FX_MAX_TAIL_MS, at any length; nothing draws with the effects off", () => {
    const view = fullBoardView();
    for (const D of DS) {
      const cues = beatsOf(playOf("p1"), view, D).flatMap((beat) => beat?.cues ?? []);
      expect(cues.length).toBeGreaterThan(0);
      expectCuesR200(cues, D, STIMMY, SIGNATURE_SPELL_MAX_TRAUMA);
    }
    expect(cuesOfRun(planWithOffsets(playOf("p1"), view, envOf(FX_INTENSITY_SCALE.off)))).toEqual([]);
  });

  it("R3407000 its gallery play ends within the Legendary Spell's ceiling", () => {
    expectGalleryWithinCeiling(STIMMY);
  });

  it("R3407000 R202 from the opponent's seat it draws at their hero and their mana pile, naming no card", () => {
    const cues = cuesOfRun(planWithOffsets(asTheirsAll(playOf("p1")), fullBoardView()));
    expect(cues.some((cue) => cue.kind === "burst" && JSON.stringify(cue.at) === JSON.stringify(tid(testid.hero("opponent"))))).toBe(true);
    expect(cues.some((cue) => cue.kind === "ring" && JSON.stringify(cue.at) === JSON.stringify(tid(animTestid.mana("opponent"))))).toBe(true);
    expectNoCard(cues, STIMMY);
  });
});
