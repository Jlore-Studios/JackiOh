// SCRATCH (#3405 builder): delete before delivery. Counts every cue each Meditative signature's gallery play plans.

import { it } from "vitest";

import { entranceEntries, planEntries, setPieceEntries, takeoverClaim, type EntranceResolver } from "../../../game/animations.ts";
import { fullBoardView, withEvents } from "../../../test/fixtures.ts";
import { CATALOG } from "../../../wire/cards.ts";
import { FX_CARD_TABLE, isSignature } from "../tables.ts";
import { FX_INTENSITY_SCALE } from "../../constants.ts";
import { planFx } from "../../cues.ts";
import { createFxMemory } from "../../memory.ts";
import { galleryBoardOf, galleryCardEvents, galleryFieldView, galleryPlayOf } from "../../motifs/gallery.ts";
import { hasSetPiece, wipeClaim } from "../../setPieces.ts";
import type { FxPlanEnv } from "../../types.ts";

const resolve: EntranceResolver = (defId) => {
  const def = CATALOG[defId];
  return def === undefined ? null : { type: def.type, rarity: def.printedRarity ?? def.rarity };
};

it("counts", () => {
  const rows: [string, number, number][] = [];
  for (const [defId, entry] of FX_CARD_TABLE) {
    if (!defId.startsWith("meditative-") || !isSignature(entry)) continue;
    const view = fullBoardView();
    const play = galleryPlayOf(defId, entry);
    if (play === null) continue;
    const board = galleryBoardOf(defId, view);
    const events = galleryCardEvents(defId, play, board, 1);
    const fielded = withEvents(galleryFieldView(defId, board), events);
    const planned = setPieceEntries(entranceEntries(planEntries(events, fielded, false, undefined, hasSetPiece), resolve, { seen: new Set() }), (e) => wipeClaim(e) ?? takeoverClaim(e), { reduced: false, bigMoments: "full", speed: 1 });
    const env: FxPlanEnv = { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
    let cues = 0;
    let particles = 0;
    for (const p of planned) {
      env.memory.remember(p.events);
      const all = planFx(p, p.view, env);
      cues += all.length;
      for (const c of all) if (c.kind === "burst") particles += c.count;
    }
    rows.push([defId, cues, particles]);
  }
  rows.sort((a, b) => b[1] - a[1]);
  console.log(rows.map((r) => r.join(" ")).join("\n"));
});
