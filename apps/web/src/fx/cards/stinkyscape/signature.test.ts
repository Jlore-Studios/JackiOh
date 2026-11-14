// R3407000: what Stinkyscape part two's signatures share (signature.ts): a recipe's beat at its card's motif
// scale and an entrance's at the plan's intensity, each in its palette's look over the event rows; an
// entrance's moments read back from its landing; a ballot dropping; and the other seat.

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { testid } from "../../../game/contract.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import { envOf } from "../../../test/signatures.ts";
import { ringCue, tid } from "../../build.ts";
import { FX_INTENSITY_SCALE } from "../../constants.ts";
import { paletteLook } from "../../flavors.ts";
import { galleryEntry } from "../../motifs/gallery.ts";
import { motifScale } from "../../motifs/build.ts";
import { ballot, entranceSpec, entranceTimes, landMsAfter, otherPlayer, sigSpec } from "./signature.ts";

const SCORNBOI = "stinkyscape-115";
const BLOOD = paletteLook("Dark/blood");

describe("R3407000 what part two's signatures share", () => {
  it("R3407000 a recipe's beat is in its palette's look at its card's motif scale, an entrance's at the plan's intensity, each over the event rows", () => {
    const view = fullBoardView();
    const env = envOf();
    const event: GameEvent = { type: "cardPlayed", player: "p1", instanceId: "scorn", defId: SCORNBOI, costPaid: 3 };
    env.memory.remember([event]);
    const play = env.memory.resolving();
    if (play === undefined || BLOOD === undefined) throw new Error("no play or look");
    const p = { entry: { ...galleryEntry([event], view), durationMs: 400 }, view, env, D: 400 };
    expect(sigSpec(p, play, "Dark/blood")).toEqual({ look: BLOOD, scale: motifScale(SCORNBOI, view, env.intensity), row: "keep" });
    const intensity = FX_INTENSITY_SCALE.low;
    expect(entranceSpec({ D: 400, tgt: testid.zone("you", "units", 3), intensity }, "Dark/blood")).toEqual({ look: BLOOD, scale: intensity, row: "keep" });
  });

  it("R3407000 an entrance reads its hold back from its landing; a landing on no whole ms reads back the nearest hold, never NaN", () => {
    const tgt = testid.zone("you", "units", 3);
    for (const D of [400, 1400]) {
      for (const wait of [0, 1, 2, 3, 120, 333, 500].filter((hold) => hold < D)) {
        const landMs = landMsAfter(wait, D);
        const times = entranceTimes({ D, tgt, intensity: 1, landMs });
        expect(times.slam, `${String(D)} ${String(wait)}`).toBe(landMs);
        expect(landMsAfter(times.wait, D), `${String(D)} ${String(wait)}`).toBe(landMs);
      }
    }
    // At D 1400, a hold of 500 lands at 500 + 0.6 × 900 = 1040; half a ms on, (1040.5 − 840) / 0.4 = 501.25 reads back 501.
    expect(landMsAfter(500, 1400)).toBe(1040);
    const off = entranceTimes({ D: 1400, tgt, intensity: 1, landMs: 1040.5 });
    expect(off.wait).toBe(501);
    expect(off.at(0)).toBe(501);
  });

  it("R3407000 a ballot drops as a ring and a small burst at its point, in the spec's look", () => {
    if (BLOOD === undefined) throw new Error("no look");
    const spec = { look: BLOOD, scale: 1, row: "keep" as const };
    const at = tid(testid.zone("opponent", "units", 2));
    const [ring, burst] = ballot(spec, 400, at, 100);
    expect(ring).toEqual(ringCue(400, BLOOD.particle, at, 100));
    expect(burst).toMatchObject({ kind: "burst", preset: BLOOD.particle, at, spread: "point", delayMs: 100, colors: BLOOD.colors });
  });

  it("R3407000 the other seat", () => {
    expect(otherPlayer("p1")).toBe("p2");
    expect(otherPlayer("p2")).toBe("p1");
  });
});
