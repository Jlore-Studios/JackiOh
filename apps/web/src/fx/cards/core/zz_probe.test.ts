import type { GameEvent } from "@jackioh/shared";
import { it } from "vitest";
import { entranceEntries, planEntries, setPieceEntries, takeoverClaim, type AnimationEntry, type EntranceResolver } from "../../../game/animations.ts";
import { fullBoardView, withEvents } from "../../../test/fixtures.ts";
import { CATALOG } from "../../../wire/cards.ts";
import { FX_CARD_TABLE } from "../tables.ts";
import { galleryBoardOf, galleryCardEvents, galleryFieldView, galleryPlayOf } from "../../motifs/gallery.ts";
import { hasSetPiece, wipeClaim } from "../../setPieces.ts";
import { planCardFx } from "../../cardFx.ts";
import { planFx } from "../../cues.ts";
import { entranceCues } from "../../takeover.ts";
import { createFxMemory } from "../../memory.ts";
import { FX_INTENSITY_SCALE, FX_MAX_PARTICLE_LIFE_MS, SIGNATURE_LEGENDARY_MAX_MS, SIGNATURE_MYTHIC_MAX_MS, SIGNATURE_SPELL_SHARE } from "../../constants.ts";
import type { FxCue, FxPlanEnv } from "../../types.ts";
const resolve: EntranceResolver = (defId) => { const def = CATALOG[defId]; return def === undefined ? null : { type: def.type, rarity: def.printedRarity ?? def.rarity }; };
function endOf(cue: FxCue): number { switch (cue.kind) { case "burst": return cue.delayMs + FX_MAX_PARTICLE_LIFE_MS; case "shot": case "projectile": return cue.delayMs + cue.flightMs + FX_MAX_PARTICLE_LIFE_MS; case "shake": return cue.delayMs; default: return cue.delayMs + ("durationMs" in cue ? cue.durationMs : 0); } }
function cuesOf(planned: AnimationEntry, env: FxPlanEnv): FxCue[] { return [...entranceCues(planned, env, planned.durationMs), ...planFx(planned, planned.view, env)]; }
function actionsOf(events: readonly GameEvent[]): GameEvent[][] { const a: GameEvent[][] = []; for (const e of events) { const later = e.type === "trapFired" || (e.type === "hookResolving" && e.hook !== "cry"); const last = a[a.length - 1]; if (last === undefined || (later && last.length > 0)) a.push([e]); else last.push(e); } return a; }
it("probe", () => {
  const view = fullBoardView();
  for (const id of ["core-052","core-083","core-085","core-087","core-093","core-095","core-096","core-097","core-098","core-099"]) {
    const entry = FX_CARD_TABLE.get(id)!; const def = CATALOG[id]!;
    const ceiling = (def.rarity === "Mythic" ? SIGNATURE_MYTHIC_MAX_MS : SIGNATURE_LEGENDARY_MAX_MS) * (def.type === "Spell" ? SIGNATURE_SPELL_SHARE : 1);
    const play = galleryPlayOf(id, entry)!; const board = galleryBoardOf(id, view);
    const events = galleryCardEvents(id, play, board, 1); const fielded = withEvents(galleryFieldView(id, board), events);
    const env: FxPlanEnv = { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
    const seen = new Set<string>(); const lines: string[] = [];
    for (const action of actionsOf(events)) {
      const planned = setPieceEntries(entranceEntries(planEntries(action, fielded, false, undefined, hasSetPiece), resolve, { seen }), (e) => wipeClaim(e) ?? takeoverClaim(e), { reduced: false, bigMoments: "full", speed: 1 });
      let offset = 0;
      for (const e of planned) {
        env.memory.remember(e.events);
        const cues = cuesOf(e, env);
        const over = cues.filter((c) => c.kind !== "pull" && c.kind !== "conceal" && offset + endOf(c) > ceiling).map((c) => `${c.kind}${"preset" in c ? ":" + String(c.preset) : ""}@${c.delayMs}->${offset + endOf(c)}`);
        lines.push(`  [${e.events.map((x) => x.type).join(",")}] off=${offset} D=${e.durationMs} cues=${cues.length}${over.length ? " OVER " + over.join(" ") : ""}`);
        offset += e.durationMs;
      }
      lines.push("  --");
    }
    console.log(`PROBE ${id} ${def.type} ${def.rarity} ceiling=${ceiling}\n${lines.join("\n")}`);
  }
});
