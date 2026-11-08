// SCRATCH (#3405 fields builder): delete before delivery.
import { it } from "vitest";

import { entranceEntries, planEntries, setPieceEntries, takeoverClaim, type EntranceResolver } from "../../../game/animations.ts";
import { fullBoardView, withEvents } from "../../../test/fixtures.ts";
import { CATALOG } from "../../../wire/cards.ts";
import { FX_INTENSITY_SCALE } from "../../constants.ts";
import { planFx } from "../../cues.ts";
import { createFxMemory } from "../../memory.ts";
import { galleryBoardOf, galleryCardEvents, galleryFieldView, galleryPlayOf } from "../../motifs/gallery.ts";
import { hasSetPiece, wipeClaim } from "../../setPieces.ts";
import { FX_CARD_TABLE } from "../tables.ts";

const resolve: EntranceResolver = (defId) => {
  const def = CATALOG[defId];
  return def === undefined ? null : { type: def.type, rarity: def.printedRarity ?? def.rarity };
};

it("dump", () => {
  const defId = process.env.M3405_ID ?? "meditative-026";
  const view = fullBoardView();
  const entry = FX_CARD_TABLE.get(defId);
  if (entry === undefined) throw new Error("none");
  const play = galleryPlayOf(defId, entry);
  if (play === null) throw new Error("none");
  const board = galleryBoardOf(defId, view);
  const events = galleryCardEvents(defId, play, board, 1);
  const fielded = withEvents(galleryFieldView(defId, board), events);
  const entered = entranceEntries(planEntries(events, fielded, false, undefined, hasSetPiece), resolve, { seen: new Set() });
  const planned = setPieceEntries(entered, (e) => wipeClaim(e) ?? takeoverClaim(e), { reduced: false, bigMoments: "full", speed: 1 });
  const env = { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
  process.stderr.write("\n" + JSON.stringify(play));
  for (const p of planned) {
    env.memory.remember(p.events);
    process.stderr.write("\n" + `entry D=${String(p.durationMs)} ${JSON.stringify(p.events)}`);
    for (const cue of planFx(p, p.view, env)) process.stderr.write("\n    " + JSON.stringify(cue));
  }
});
