// R2797000 #2797: the first half of Stinkyscape's pass over its animation table (stinkyscape-1.json5,
// stinkyscape-001 to stinkyscape-090 and the tokens listed among them): at least FX_SET_PASS_FLOOR of its
// non-token cards wear an entry other than `none`, every Spell, Field Spell, Trap and Field Trap but a
// token has a `cast` (or a recipe or set piece of its own), every Field Spell an aura, every hook a drawn
// motif, and the /dev/fx gallery plays every entry other than `none` as its card's play, a Death, trigger
// or Activate from its marker, a hidden play or marker keying none (R202). stinkyscape-1.md, the pass's
// per-card table, holds one row per card of the file that matches it.

import type { GameEvent, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { HIDDEN_ID, planEntries } from "../../game/animations.ts";
import { galleryCards } from "../../routes/dev/fx.tsx";
import { fullBoardView, withEvents } from "../../test/fixtures.ts";
import { CATALOG } from "../../wire/cards.ts";
import { planCardFx, planCardMotif } from "../cardFx.ts";
import { FX_INTENSITY_SCALE, FX_SET_PASS_FLOOR } from "../constants.ts";
import { planFx } from "../cues.ts";
import { createFxMemory } from "../memory.ts";
import { motifScale } from "../motifs/build.ts";
import { galleryCardEvents, galleryFieldView, galleryPlayOf, type GalleryCardPlay } from "../motifs/gallery.ts";
import { FX_CARD_HOOKS, isDrawnMotif } from "../motifs/names.ts";
import { hasSetPiece } from "../setPieces.ts";
import type { FxPlanEnv } from "../types.ts";
import rawTable from "./stinkyscape-1.md?raw";
import { FX_CARD_FILES, parseFxCardTable, type FxCardEntry } from "./tables.ts";

/** The card types whose play is a `cast` the pass holds to one. */
const CAST_TYPES: ReadonlySet<string> = new Set(["Spell", "Field Spell", "Trap", "Field Trap"]);

const FILE = FX_CARD_FILES.find((file) => file.file === "stinkyscape-1.json5");
if (FILE === undefined) throw new Error("fx/cards/stinkyscape-1.json5 is missing");
const TABLE: ReadonlyMap<string, FxCardEntry> = parseFxCardTable(FILE.file, FILE.source);
const IDS = [...TABLE.keys()];
const CARDS = IDS.filter((id) => CATALOG[id]?.token !== true);

function entryOf(id: string): FxCardEntry {
  const entry = TABLE.get(id);
  if (entry === undefined) throw new Error(`${id} has no entry`);
  return entry;
}

/** An entry as stinkyscape-1.md's Entry column writes it (core.md's form). */
function describeEntry(entry: FxCardEntry): string {
  switch (entry.kind) {
    case "none":
      return `none: ${entry.reason}`;
    case "bespoke":
      return `bespoke ${entry.key}`;
    case "setPiece":
      return `setPiece ${entry.key}`;
    case "hooks": {
      const parts: string[] = [];
      for (const hook of FX_CARD_HOOKS) {
        const spec = entry.hooks[hook];
        if (spec === undefined) continue;
        parts.push([hook, spec.motif, spec.palette, spec.row === "keep" ? "keep" : undefined].filter((part) => part !== undefined).join(" "));
      }
      if (entry.aura !== undefined) parts.push(["aura", entry.aura.look, entry.aura.scope, entry.aura.palette].filter((part) => part !== undefined).join(" "));
      return parts.join("; ");
    }
  }
}

function envOf(): FxPlanEnv {
  return { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
}

/**
 * Whether the board's effects layer draws `play` of `defId` on `events` as it plans them (remember, then
 * plan): a motif, from the play or from the hook's marker, a recipe on some event of its entry, or the
 * card's set piece. An aura-only entry stands on the board and is held to its aura elsewhere.
 */
function draws(defId: string, play: GalleryCardPlay, events: readonly GameEvent[], view: PlayerView): boolean {
  const env = envOf();
  let drawn = false;
  for (const planned of planEntries(events, withEvents(view, [...events]), false, undefined, hasSetPiece)) {
    env.memory.remember(planned.events);
    if ((play.kind === "board" || play.kind === "hook") && planCardMotif(planned, planned.view, env) !== null) drawn = true;
    if (play.kind === "recipe" && planned.events.some((event) => planCardFx(event, { entry: planned, view: planned.view, env, D: planned.durationMs }) !== null)) drawn = true;
    if (play.kind === "setPiece" && planned.wipe?.by === defId) drawn = true;
  }
  return drawn;
}

/** The same stream with its play's, marker's and firing's card read as the sentinel, as the other seat sees a hidden card (R97). */
function hidden(events: readonly GameEvent[]): GameEvent[] {
  return events.map((event) => {
    if (event.type === "cardPlayed" || event.type === "cardResolved") return { ...event, instanceId: HIDDEN_ID, defId: HIDDEN_ID };
    if (event.type === "hookResolving") return { ...event, sourceId: HIDDEN_ID, defId: HIDDEN_ID };
    if (event.type === "trapFired") return { ...event, instanceId: HIDDEN_ID, defId: HIDDEN_ID };
    return event;
  });
}

/** Every entry of the file the gallery plays, with how. */
function galleryPlays(): { id: string; play: GalleryCardPlay }[] {
  return IDS.flatMap((id) => {
    const play = galleryPlayOf(id, entryOf(id));
    return play === null ? [] : [{ id, play }];
  });
}

describe("R2797000 #2797 Stinkyscape 001 to 090's card animation pass", () => {
  it("R2797000 at least FX_SET_PASS_FLOOR of the range's non-token cards, and of all its entries, are not none", () => {
    expect(IDS).toHaveLength(110);
    expect(CARDS).toHaveLength(90);
    const shown = (ids: readonly string[]): number => ids.filter((id) => entryOf(id).kind !== "none").length;
    expect(shown(CARDS)).toBeGreaterThanOrEqual(Math.ceil(FX_SET_PASS_FLOOR * CARDS.length));
    expect(shown(IDS)).toBeGreaterThanOrEqual(Math.ceil(FX_SET_PASS_FLOOR * IDS.length));
  });

  it("R2797000 every non-token Spell, Field Spell, Trap and Field Trap has a cast, bespoke or set piece, and every Field Spell an aura", () => {
    const fields = IDS.filter((id) => CATALOG[id]?.type === "Field Spell");
    expect(fields).toHaveLength(11);
    for (const id of IDS) {
      const def = CATALOG[id];
      const entry = entryOf(id);
      if (def?.type === "Field Spell") expect(entry.kind === "hooks" ? entry.aura : undefined, id).toBeDefined();
      if (def === undefined || def.token || !CAST_TYPES.has(def.type)) continue;
      const plays = entry.kind === "bespoke" || entry.kind === "setPiece" || (entry.kind === "hooks" && entry.hooks.cast !== undefined);
      expect(plays, `${id} (${def.type})`).toBe(true);
    }
  });

  it("R2797000 no entry is pending and every hook names a drawn motif", () => {
    for (const id of IDS) {
      const entry = entryOf(id);
      if (entry.kind === "none") expect(entry.reason, id).not.toBe("pending");
      if (entry.kind !== "hooks") continue;
      for (const [hook, spec] of Object.entries(entry.hooks)) expect(isDrawnMotif(spec.motif), `${id} ${hook} ${spec.motif}`).toBe(true);
    }
  });

  it("R2797000 the /dev/fx gallery lists every entry other than none, and each draws on the board as its card's play", () => {
    const view = fullBoardView();
    const plays = galleryPlays();
    expect(plays.map(({ id }) => id)).toEqual(IDS.filter((id) => entryOf(id).kind !== "none"));
    expect(galleryCards().filter((card) => TABLE.has(card.defId)).map((card) => card.defId)).toEqual(plays.map(({ id }) => id));
    // Every play kind the file uses, the hooks' among them.
    expect(new Set(plays.map(({ play }) => play.kind))).toEqual(new Set(["board", "hook", "recipe", "setPiece"]));
    for (const { id, play } of plays) {
      const shown = galleryFieldView(id, view);
      expect(draws(id, play, galleryCardEvents(id, play, shown, 1), shown), `${id} ${play.kind}`).toBe(true);
    }
  });

  it("R2797000 R202 the same plays with the card hidden key none of it, and no cue names it", () => {
    const view = fullBoardView();
    for (const { id, play } of galleryPlays()) {
      if (play.kind === "setPiece") continue;
      const events = hidden(galleryCardEvents(id, play, view, 1));
      expect(draws(id, play, events, view), id).toBe(false);
      const env = envOf();
      for (const planned of planEntries(events, withEvents(view, events), false)) {
        env.memory.remember(planned.events);
        expect(JSON.stringify(planFx(planned, planned.view, env)), id).not.toContain(`"${id}"`);
      }
    }
  });

  it("R2797000 stinkyscape-1.md has one row per card of the file, in order, with its name, type, entry, scale, census and why", () => {
    const rows = rawTable
      .split("\n")
      .filter((line) => line.startsWith("| `stinkyscape-"))
      .map((line) => line.slice(1, -1).split(" | ").map((cell) => cell.trim()));
    expect(rows.map((row) => row[0])).toEqual(IDS.map((id) => `\`${id}\``));
    const view = fullBoardView();
    for (const [cell, name, type, entry, scale, census, why] of rows) {
      const id = (cell ?? "").replace(/`/g, "");
      const def = CATALOG[id];
      const own = entryOf(id);
      expect(name, id).toBe(def?.name.replace(/\s+/g, " ").trim());
      expect(type, id).toBe(def?.type);
      expect(entry, id).toBe(describeEntry(own));
      expect(scale, id).toBe(own.kind === "none" ? "—" : motifScale(id, view, 1).toFixed(2));
      expect(census, id).toMatch(/\S/);
      expect(why, id).toMatch(/\S/);
    }
  });
});
