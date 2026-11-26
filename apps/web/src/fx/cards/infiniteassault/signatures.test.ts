// R4596000: the Infinite Assault's twenty signatures (commanders.ts, mythics.ts), planned as FxLayer plans
// them: each Commander's entry names its own entrance, each doctrine Mythic's its own cast recipe; every
// gallery play draws within its signature ceiling, each cue inside its entry plus FX_MAX_TAIL_MS (R200) and
// naming no card (R202); each entrance shakes once at its slam, a Mythic's after its takeover's hold; a
// hidden play keys no recipe; a cast draws on its own play alone.

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { testid } from "../../../game/contract.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import { beatsOf, cuesOfRun, envOf, expectGalleryWithinCeiling, expectNoCard, galleryRun } from "../../../test/signaturePass.ts";
import { CATALOG } from "../../../wire/cards.ts";
import { planCardFx } from "../../cardFx.ts";
import { FX_INTENSITY_SCALE, FX_SLAM_AT, SIGNATURE_PERMANENT_MAX_TRAUMA, SIGNATURE_SPELL_MAX_TRAUMA } from "../../constants.ts";
import { ENTRANCES } from "../../entrances.ts";
import { galleryEntry } from "../../motifs/gallery.ts";
import { ENTRANCE_TRAUMA } from "../recipe.ts";
import { FX_CARD_TABLE, isSignature } from "../tables.ts";
import { MYTHIC_TRAUMA } from "./mythics.ts";

/** The twelve Commanders' entrances and the eight doctrine Mythics' casts, by card. */
const ENTRANCE_OF: Readonly<Record<string, keyof typeof ENTRANCES>> = {
  "infiniteassault-217": "echoBoot",
  "infiniteassault-219": "redlineBoot",
  "infiniteassault-221": "hiveBoot",
  "infiniteassault-223": "loopBoot",
  "infiniteassault-225": "rootkitBoot",
  "infiniteassault-227": "bastionBoot",
  "infiniteassault-229": "harvestBoot",
  "infiniteassault-231": "blackBoxBoot",
  "infiniteassault-233": "collapseBoot",
  "infiniteassault-235": "primeBoot",
  "infiniteassault-237": "runawayBoot",
  "infiniteassault-239": "zeroDayBoot",
};
const CAST_OF: Readonly<Record<string, keyof typeof MYTHIC_TRAUMA>> = {
  "infiniteassault-048": "egoSwap",
  "infiniteassault-072": "overclockSurge",
  "infiniteassault-096": "growthFork",
  "infiniteassault-120": "returnLoop",
  "infiniteassault-144": "compromiseBreach",
  "infiniteassault-168": "impenetrableWall",
  "infiniteassault-192": "dataSwap",
  "infiniteassault-216": "singularityCollapse",
};
const ALL = [...Object.keys(ENTRANCE_OF), ...Object.keys(CAST_OF)];

const played = (defId: string, instanceId = "ex"): GameEvent => ({ type: "cardPlayed", player: "p1", instanceId, defId, costPaid: 4 });

describe("R4596000 the Infinite Assault's Commanders' and Mythics' signatures", () => {
  it("R4596000 the twelve Commanders and the eight doctrine Mythics are signatures, each with its own entrance or cast", () => {
    const commanders = Object.keys(CATALOG).filter((id) => CATALOG[id]?.set === "Infinite Assault" && CATALOG[id]?.type === "Unit" && CATALOG[id]?.token !== true && CATALOG[id]?.tags.includes("Commander"));
    expect(commanders).toEqual(Object.keys(ENTRANCE_OF));
    const mythicSpells = Object.keys(CATALOG).filter((id) => CATALOG[id]?.set === "Infinite Assault" && CATALOG[id]?.rarity === "Mythic" && CATALOG[id]?.type === "Spell");
    expect(mythicSpells).toEqual(Object.keys(CAST_OF));
    for (const [defId, key] of Object.entries(ENTRANCE_OF)) {
      const entry = FX_CARD_TABLE.get(defId);
      expect(isSignature(entry), defId).toBe(true);
      expect(entry?.kind !== "none" ? entry?.entrance : undefined, defId).toBe(key);
    }
    for (const [defId, key] of Object.entries(CAST_OF)) {
      const entry = FX_CARD_TABLE.get(defId);
      expect(isSignature(entry), defId).toBe(true);
      expect(entry?.kind === "bespoke" ? entry.key : undefined, defId).toBe(key);
    }
    expect(new Set(Object.values(ENTRANCE_OF)).size).toBe(12);
    expect(new Set(Object.values(CAST_OF)).size).toBe(8);
  });

  it("R4596000 R200 R202 every gallery play draws within its signature ceiling and names no card", () => {
    for (const defId of ALL) {
      expectGalleryWithinCeiling(defId);
      expectNoCard(cuesOfRun(galleryRun(defId).run), defId);
    }
  });

  it("R4596000 each entrance shakes once at its slam, a Mythic's after its takeover's hold, nothing under the hold", () => {
    const D = 400;
    const hold = 1000;
    for (const [defId, key] of Object.entries(ENTRANCE_OF)) {
      const tgt = testid.zone("you", "units", 3);
      expect(ENTRANCE_TRAUMA[key], key).toBeLessThanOrEqual(SIGNATURE_PERMANENT_MAX_TRAUMA);
      const plain = ENTRANCES[key]({ D, tgt, intensity: FX_INTENSITY_SCALE.normal });
      expect(plain.length, defId).toBeGreaterThan(3);
      expect(plain.filter((cue) => cue.kind === "shake"), defId).toEqual([{ kind: "shake", trauma: ENTRANCE_TRAUMA[key], delayMs: Math.round(FX_SLAM_AT * D) }]);
      if (CATALOG[defId]?.rarity !== "Mythic") continue;
      const held = ENTRANCES[key]({ D: hold + D, tgt, intensity: FX_INTENSITY_SCALE.normal, leadMs: hold });
      for (const cue of held) expect(cue.delayMs, `${defId} ${cue.kind}`).toBeGreaterThanOrEqual(hold);
    }
    for (const key of Object.values(CAST_OF)) expect(MYTHIC_TRAUMA[key], key).toBeLessThanOrEqual(SIGNATURE_SPELL_MAX_TRAUMA);
  });

  it("R4596000 R202 a hidden play keys none of the casts", () => {
    const view = fullBoardView();
    for (const defId of Object.keys(CAST_OF)) {
      const env = envOf();
      env.memory.remember([{ type: "cardPlayed", player: "p2", instanceId: "hidden", defId: "hidden", costPaid: 4 }]);
      const event: GameEvent = { type: "summoned", player: "p2", instanceId: "s", defId: "hidden", row: "units", lane: 1 };
      expect(planCardFx(event, { entry: galleryEntry([event], view), view, env, D: 250 }), defId).toBeNull();
    }
  });

  it("R4596000 each cast draws on its own play alone, leaving what its effect reports to its rows", () => {
    const view = fullBoardView();
    const theirs = view.opponent.units.find((unit) => unit !== null);
    const after: GameEvent[] = [
      { type: "summoned", player: "p1", instanceId: "copy", defId: "core-002", row: "units", lane: 2 },
      { type: "controlChanged", instanceId: theirs?.instanceId ?? "t", controller: "p1", row: "units", lane: 2 },
      { type: "swapped", what: "library" },
    ];
    for (const defId of Object.keys(CAST_OF)) {
      expect(beatsOf([played(defId)], view, 400)[0]?.row, defId).toBe("replace");
      expect(beatsOf(after, view, 400, [played(defId)]), defId).toEqual([null, null, null]);
    }
  });

});
