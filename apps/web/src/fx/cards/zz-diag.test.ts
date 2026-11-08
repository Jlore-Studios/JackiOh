import { it } from "vitest";
import { entranceEntries, planEntries, setPieceEntries, takeoverClaim, type EntranceResolver } from "../../game/animations.ts";
import { fullBoardView, withEvents } from "../../test/fixtures.ts";
import { CATALOG } from "../../wire/cards.ts";
import { FX_CARD_TABLE } from "./tables.ts";
import { galleryBoardOf, galleryCardEvents, galleryFieldView, galleryPlayOf } from "../motifs/gallery.ts";
import { hasSetPiece, wipeClaim } from "../setPieces.ts";
const resolve: EntranceResolver = (defId) => { const d = CATALOG[defId]; return d === undefined ? null : { type: d.type, rarity: d.printedRarity ?? d.rarity }; };
it("diag", () => {
  for (const defId of (process.env.DIAG ?? "meditativeplus-024,traininggrounds-040,meditative-049,traininggrounds-037").split(",")) {
    const entry = FX_CARD_TABLE.get(defId); if (!entry) continue;
    const play = galleryPlayOf(defId, entry); if (!play) continue;
    const board = galleryBoardOf(defId, fullBoardView());
    const events = galleryCardEvents(defId, play, board, 1);
    const fielded = withEvents(galleryFieldView(defId, board), events);
    const entered = entranceEntries(planEntries(events, fielded, false, undefined, hasSetPiece), resolve, { seen: new Set() });
    const planned = setPieceEntries(entered, (e) => wipeClaim(e) ?? takeoverClaim(e), { reduced: false, bigMoments: "full", speed: 1 });
    console.log(defId, JSON.stringify(entry), planned.map((p) => `${p.events.map((e) => e.type).join("+")}:${p.durationMs}${p.setPiece ? `(hold ${p.setPiece.holdMs})` : ""}`).join(" | "));
  }
});
