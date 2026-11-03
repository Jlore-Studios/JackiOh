// R2797000: Stinkyscape's first half (fx/cards/stinkyscape-1.json5, stinkyscape-001 to -090 and the tokens
// listed among them) held to its pass's measure (issue #2797): most of its cards play something, every
// Spell, Field Spell, Trap and Field Trap but a token plays on its cast, every Field Spell wears an aura,
// nothing is pending, and the /dev/fx gallery lists and plays every entry it can, from the redacted
// stream alone (R202).

import type { GameEvent, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { HIDDEN_ID, planEntries } from "../../game/animations.ts";
import { galleryCards } from "../../routes/dev/fx.tsx";
import { fullBoardView, withEvents } from "../../test/fixtures.ts";
import { CATALOG } from "../../wire/cards.ts";
import { planCardFx, planCardMotif } from "../cardFx.ts";
import { FX_INTENSITY_SCALE } from "../constants.ts";
import { planFx } from "../cues.ts";
import { createFxMemory } from "../memory.ts";
import { cardFixtureEvents } from "../motifs/gallery.ts";
import { isDrawnMotif } from "../motifs/names.ts";
import type { FxPlanEnv } from "../types.ts";
import { FX_CARD_FILES, parseFxCardTable, type FxCardEntry } from "./tables.ts";

/** The issue's floor: at least this share of the file's entries are no `none`. */
const NOT_NONE_MIN_SHARE = 0.55;

/** The card types whose play is a `cast` the measure holds to one. */
const CAST_TYPES: ReadonlySet<string> = new Set(["Spell", "Field Spell", "Trap", "Field Trap"]);

const FILE = FX_CARD_FILES.find((file) => file.file === "stinkyscape-1.json5");
if (FILE === undefined) throw new Error("fx/cards/stinkyscape-1.json5 is missing");
const TABLE: ReadonlyMap<string, FxCardEntry> = parseFxCardTable(FILE.file, FILE.source);

function envOf(): FxPlanEnv {
  return { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
}

/** Whether the planner draws anything of the card's own for `events`: its motif on some entry, or its recipe on some event. */
function plays(events: readonly GameEvent[], view: PlayerView): boolean {
  const env = envOf();
  let drawn = false;
  for (const entry of planEntries(events, withEvents(view, [...events]), false)) {
    env.memory.remember(entry.events);
    if (planCardMotif(entry, entry.view, env) !== null) drawn = true;
    for (const event of entry.events) if (planCardFx(event, { entry, view: entry.view, env, D: entry.durationMs }) !== null) drawn = true;
  }
  return drawn;
}

/** The same stream with its play's or marker's card read as the sentinel, as the other seat would see a hidden card (R97). */
function hidden(events: readonly GameEvent[]): GameEvent[] {
  return events.map((event) => {
    if (event.type === "cardPlayed" || event.type === "cardResolved") return { ...event, instanceId: HIDDEN_ID, defId: HIDDEN_ID };
    if (event.type === "hookResolving") return { ...event, sourceId: HIDDEN_ID, defId: HIDDEN_ID };
    return event;
  });
}

describe("R2797000 Stinkyscape 001 to 090", () => {
  it("R2797000 at least NOT_NONE_MIN_SHARE of its entries are no none, and every non-token Spell, Field Spell, Trap and Field Trap plays on its cast", () => {
    const entries = [...TABLE.values()];
    const shown = entries.filter((entry) => entry.kind !== "none").length;
    expect(shown / entries.length).toBeGreaterThanOrEqual(NOT_NONE_MIN_SHARE);
    for (const [defId, entry] of TABLE) {
      const def = CATALOG[defId];
      if (def === undefined || def.token || !CAST_TYPES.has(def.type)) continue;
      const cast = entry.kind === "hooks" ? entry.hooks.cast?.motif : undefined;
      expect(entry.kind === "bespoke" || entry.kind === "setPiece" || (cast !== undefined && isDrawnMotif(cast)), `${defId} ${def.type}`).toBe(true);
    }
  });

  it("R2797000 every Field Spell, tokens included, wears an aura", () => {
    const fields = [...TABLE.keys()].filter((defId) => CATALOG[defId]?.type === "Field Spell");
    expect(fields.length).toBeGreaterThan(0);
    for (const defId of fields) {
      const entry = TABLE.get(defId);
      expect(entry?.kind === "hooks" ? entry.aura : undefined, defId).toBeDefined();
    }
  });

  it("R2797000 no entry is pending and every hook names a drawn motif", () => {
    for (const [defId, entry] of TABLE) {
      if (entry.kind === "none") expect(entry.reason, defId).not.toBe("pending");
      if (entry.kind !== "hooks") continue;
      for (const [hook, spec] of Object.entries(entry.hooks)) expect(isDrawnMotif(spec.motif), `${defId} ${hook}`).toBe(true);
    }
  });

  it("R2797000 the /dev/fx gallery lists every hooks and bespoke entry and each plays", () => {
    const view = fullBoardView();
    const listed = galleryCards().filter((card) => TABLE.has(card.defId));
    const expected = [...TABLE].filter(([, entry]) => entry.kind === "bespoke" || (entry.kind === "hooks" && Object.keys(entry.hooks).length > 0)).map(([defId]) => defId);
    expect(listed.map((card) => card.defId)).toEqual(expected);
    for (const card of listed) {
      expect(plays(cardFixtureEvents(card.defId, card.motif, view, 0, card.hook), view), `${card.defId} ${card.hook} ${card.motif}`).toBe(true);
    }
  });

  it("R2797000 R202 the same fixtures under the sentinel key none of it", () => {
    const view = fullBoardView();
    for (const card of galleryCards().filter((candidate) => TABLE.has(candidate.defId))) {
      const events = hidden(cardFixtureEvents(card.defId, card.motif, view, 0, card.hook));
      expect(plays(events, view), card.defId).toBe(false);
      const env = envOf();
      for (const entry of planEntries(events, withEvents(view, events), false)) {
        env.memory.remember(entry.events);
        expect(JSON.stringify(planFx(entry, entry.view, env)), card.defId).not.toContain("stinkyscape-");
      }
    }
  });
});
