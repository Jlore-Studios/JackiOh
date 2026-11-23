// R2784000 #2796: what Meditative's and Meditative+'s Flourishes draw with (flourish.ts), past what
// flourishes.test.ts holds: a Flourish's look is the palette it names, else its card's own flavor's, else
// the fallback flavor's for a card the viewer cannot read (R202); it plays at the card's motif scale times
// the intensity, over the event's own row; and it draws nothing with the effects off or in an entry of no
// duration.

import type { PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { FLAVOR_FALLBACK } from "../../../cards/flavor.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import { envOf } from "../../../test/signatures.ts";
import { FX_INTENSITY_SCALE } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { galleryEntry } from "../../motifs/gallery.ts";
import { motifScale } from "../../motifs/build.ts";
import type { FxPlay } from "../../types.ts";
import type { CardFxPlan } from "../recipe.ts";
import { flourishSpec, flourishes } from "./flourish.ts";

/** Spiritually 中国, a Spirit Spell; the hidden id, a card no viewer reads. */
const ZHONGGUO = "meditative-032";
const HIDDEN = "hidden";
const D = 1000;

const planOf = (view: PlayerView, intensity: number = FX_INTENSITY_SCALE.normal, duration = D): CardFxPlan => ({ entry: galleryEntry([], view), view, env: envOf({ intensity }), D: duration });
const playOf = (defId: string): FxPlay => ({ player: "p1", instanceId: "ex", defId, castOnDraw: false, step: 0, seen: {} });

describe("R2784000 a Flourish's look, scale and row", () => {
  it("R2784000 it wears the palette it names over its card's own flavor, at the card's motif scale times the intensity, over the event's own row", () => {
    const view = fullBoardView();
    const p = planOf(view, FX_INTENSITY_SCALE.high);
    expect(flourishSpec(p, playOf(ZHONGGUO), "Hustle/jade")).toEqual({ look: paletteLook("Hustle/jade"), scale: motifScale(ZHONGGUO, view, FX_INTENSITY_SCALE.high), row: "keep" });
    expect(paletteLook("Hustle/jade")).not.toEqual(FLAVOR_LOOKS.Spirit);
  });

  it("R2784000 with no palette it wears its card's own flavor's look, and the fallback flavor's for a card the viewer cannot read (R202)", () => {
    const view = fullBoardView();
    const p = planOf(view);
    expect(flourishSpec(p, playOf(ZHONGGUO))).toEqual({ look: FLAVOR_LOOKS.Spirit, scale: motifScale(ZHONGGUO, view, FX_INTENSITY_SCALE.normal), row: "keep" });
    expect(flourishSpec(p, playOf(HIDDEN)).look).toEqual(FLAVOR_LOOKS[FLAVOR_FALLBACK]);
    expect(FLAVOR_LOOKS.Spirit).not.toEqual(FLAVOR_LOOKS[FLAVOR_FALLBACK]);
  });

  it("R200 it draws only with the effects on and in an entry of some duration", () => {
    const view = fullBoardView();
    expect(flourishes(planOf(view))).toBe(true);
    expect(flourishes(planOf(view, FX_INTENSITY_SCALE.off))).toBe(false);
    expect(flourishes(planOf(view, FX_INTENSITY_SCALE.normal, 0))).toBe(false);
  });
});
