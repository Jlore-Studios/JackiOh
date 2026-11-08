// SCRATCH (#3405 builder): delete before delivery. The R3332000 loudness budget of tiers.test.ts, for the
// ids in M3405_IDS (comma-separated) only, each card its own test, logging each card's end and ceiling.

import { describe, expect, it } from "vitest";

import { entranceEntries, planEntries, setPieceEntries, takeoverClaim, type AnimationEntry, type EntranceResolver } from "../../../game/animations.ts";
import { fullBoardView, withEvents } from "../../../test/fixtures.ts";
import { CATALOG } from "../../../wire/cards.ts";
import { planCardFx } from "../../cardFx.ts";
import { FX_CARD_TABLE } from "../tables.ts";
import { FX_INTENSITY_SCALE, FX_MAX_PARTICLE_LIFE_MS, FX_MAX_TAIL_MS, SIGNATURE_LEGENDARY_MAX_MS, SIGNATURE_MYTHIC_MAX_MS, SIGNATURE_PERMANENT_MAX_TRAUMA, SIGNATURE_SPELL_MAX_TRAUMA, SIGNATURE_SPELL_SHARE } from "../../constants.ts";
import { planFx } from "../../cues.ts";
import { createFxMemory } from "../../memory.ts";
import { galleryBoardOf, galleryCardEvents, galleryFieldView, galleryPlayOf } from "../../motifs/gallery.ts";
import { hasSetPiece, wipeClaim } from "../../setPieces.ts";
import { entranceCues } from "../../takeover.ts";
import type { FxCue, FxPlanEnv } from "../../types.ts";

function endOf(cue: FxCue): number {
  switch (cue.kind) {
    case "burst":
      return cue.delayMs + FX_MAX_PARTICLE_LIFE_MS;
    case "shot":
    case "projectile":
      return cue.delayMs + cue.flightMs + FX_MAX_PARTICLE_LIFE_MS;
    case "shake":
      return cue.delayMs;
    default:
      return cue.delayMs + ("durationMs" in cue ? cue.durationMs : 0);
  }
}

const resolve: EntranceResolver = (defId) => {
  const def = CATALOG[defId];
  return def === undefined ? null : { type: def.type, rarity: def.printedRarity ?? def.rarity };
};

function cuesOf(planned: AnimationEntry, env: FxPlanEnv): FxCue[] {
  const cues: FxCue[] = [...entranceCues(planned, env, planned.durationMs), ...planFx(planned, planned.view, env)];
  for (const event of planned.events) {
    const recipe = planCardFx(event, { entry: planned, view: planned.view, env, D: planned.durationMs });
    if (recipe !== null) cues.push(...recipe.cues);
  }
  return cues;
}

const IDS = (process.env.M3405_IDS ?? "").split(",").filter((id) => id !== "");

describe("scratch budget", () => {
  it.each(IDS.length > 0 ? IDS : ["meditative-007"])("%s keeps its budget", (defId) => {
    const view = fullBoardView();
    const entry = FX_CARD_TABLE.get(defId);
    if (entry === undefined) throw new Error("no entry");
    const def = CATALOG[defId];
    const rarity = def?.printedRarity ?? def?.rarity;
    const spell = def?.type === "Spell";
    const ceiling = (rarity === "Mythic" ? SIGNATURE_MYTHIC_MAX_MS : SIGNATURE_LEGENDARY_MAX_MS) * (spell ? SIGNATURE_SPELL_SHARE : 1);
    const maxTrauma = spell ? SIGNATURE_SPELL_MAX_TRAUMA : SIGNATURE_PERMANENT_MAX_TRAUMA;
    const play = galleryPlayOf(defId, entry);
    if (play === null) throw new Error("no gallery play");
    const board = galleryBoardOf(defId, view);
    const events = galleryCardEvents(defId, play, board, 1);
    const fielded = withEvents(galleryFieldView(defId, board), events);
    const entered = entranceEntries(planEntries(events, fielded, false, undefined, hasSetPiece), resolve, { seen: new Set() });
    const planned = setPieceEntries(entered, (e) => wipeClaim(e) ?? takeoverClaim(e), { reduced: false, bigMoments: "full", speed: 1 });
    const env: FxPlanEnv = { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
    let offset = 0;
    let end = 0;
    const log: string[] = [];
    for (const p of planned) {
      env.memory.remember(p.events);
      const cues = cuesOf(p, env);
      log.push(`  entry @${String(offset)} D=${String(p.durationMs)} [${p.events.map((e) => e.type).join(",")}] cues=${cues.map((c) => `${c.kind}${c.kind === "figure" ? `:${c.figure}` : ""}@${String(c.delayMs)}`).join(" ")}`);
      for (const cue of cues) {
        if (cue.kind === "pull" || cue.kind === "conceal") continue;
        const where = `${defId} ${cue.kind} at ${String(offset)}`;
        expect(cue.delayMs, where).toBeGreaterThanOrEqual(0);
        expect(cue.delayMs, where).toBeLessThanOrEqual(p.durationMs);
        expect(endOf(cue), where).toBeLessThanOrEqual(p.durationMs + FX_MAX_TAIL_MS);
        end = Math.max(end, offset + endOf(cue));
        if (cue.kind === "shake") expect(cue.trauma, where).toBeLessThanOrEqual(maxTrauma);
      }
      offset += p.durationMs;
    }
    console.log(`${defId}: ends ${String(end)} of ceiling ${String(ceiling)}\n${log.join("\n")}`);
    expect(end, `${defId} ends within its ceiling`).toBeLessThanOrEqual(ceiling);
  });
});
