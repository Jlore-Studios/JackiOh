import { it } from "vitest";
import { appendFileSync } from "node:fs";
const log = (...a: unknown[]) => appendFileSync("/tmp/entries.txt", a.map(String).join(" ") + "\n");
import { entranceEntries, planEntries, setPieceEntries, takeoverClaim, type EntranceResolver } from "../game/animations.ts";
import { fullBoardView, withEvents } from "../test/fixtures.ts";
import { CATALOG } from "../wire/cards.ts";
import { FX_CARD_TABLE } from "../fx/cards/tables.ts";
import { galleryBoardOf, galleryCardEvents, galleryFieldView, galleryPlayOf } from "../fx/motifs/gallery.ts";
import { hasSetPiece, wipeClaim } from "../fx/setPieces.ts";
const resolve: EntranceResolver = (defId) => { const d = CATALOG[defId]; return d ? { type: d.type, rarity: d.printedRarity ?? d.rarity } : null; };
it("scratch", () => {
  const view = fullBoardView();
  for (const defId of (process.env.IDS ?? "core-092").split(",")) {
    const entry = FX_CARD_TABLE.get(defId)!;
    const play = galleryPlayOf(defId, entry);
    if (!play) { log(defId, "no play"); continue; }
    const board = galleryBoardOf(defId, view);
    const events = galleryCardEvents(defId, play, board, 1);
    const fielded = withEvents(galleryFieldView(defId, board), events);
    const entered = entranceEntries(planEntries(events, fielded, false, undefined, hasSetPiece), resolve, { seen: new Set() });
    const planned = setPieceEntries(entered, (e) => wipeClaim(e) ?? takeoverClaim(e), { reduced: false, bigMoments: "full", speed: 1 });
    let off = 0;
    log(defId, play.kind);
    for (const e of planned) { log("  @", off, "D", e.durationMs, e.events.map((x) => x.type).join(","), e.entrance ? `entrance:${e.entrance.tier}` : "", e.setPiece ? `hold:${e.setPiece.holdMs}` : "", e.slam ? `slam:${JSON.stringify(e.slam)}` : ""); off += e.durationMs; }
  }
});
