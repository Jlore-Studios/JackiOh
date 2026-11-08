// R2784000 #2798: Stinkyscape's pass, part two, over its animation table (stinkyscape-2.json5, `stinkyscape-091`
// to the end, the set's tokens included): no line pending, at least FX_SET_PASS_FLOOR of its non-token cards
// wear an entry other than `none`, every Spell, Field Spell, Trap and Field Trap has a `cast` (or a recipe or
// set piece of its own) but the Starship's own cards, which the Starship part of #2782 draws, every Field
// Spell an aura, every hook a drawn motif, and every play's motif draws on its gallery fixture as that card's
// play, a hidden play keying none (R202). stinkyscape-2.md, the pass's per-card table, holds one row per
// card of the file that matches the table. R3334004: three of the Starship's own cards have signatures of
// their own (`STARSHIP_SIGNATURES`), each planned on its gallery play within R200's bounds and its tier's
// ceiling and loudness (R3332000), and none keyed by a hidden play.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { entranceEntries, planEntries, setPieceEntries, takeoverClaim, type EntranceResolver } from "../../game/animations.ts";
import { fullBoardView, withEvents } from "../../test/fixtures.ts";
import { describeEntry } from "../../test/setPass.ts";
import { CATALOG } from "../../wire/cards.ts";
import { planCardFx, planCardMotif } from "../cardFx.ts";
import {
  FX_INTENSITY_SCALE,
  FX_MAX_PARTICLE_LIFE_MS,
  FX_MAX_TAIL_MS,
  FX_SET_PASS_FLOOR,
  SIGNATURE_LEGENDARY_MAX_MS,
  SIGNATURE_PERMANENT_MAX_TRAUMA,
  SIGNATURE_SPELL_MAX_TRAUMA,
  SIGNATURE_SPELL_SHARE,
} from "../constants.ts";
import { planFx } from "../cues.ts";
import { createFxMemory } from "../memory.ts";
import { motifScale } from "../motifs/build.ts";
import { galleryBoardOf, galleryCardEvents, galleryEntry, galleryFieldView, galleryPlayOf, motifFixture, RECIPE_FIXTURES } from "../motifs/gallery.ts";
import { isDrawnMotif } from "../motifs/names.ts";
import { hasSetPiece, wipeClaim } from "../setPieces.ts";
import type { FxCue, FxPlanEnv } from "../types.ts";
import { isCardRecipeKey } from "./bespoke.ts";
import rawTable from "./stinkyscape-2.md?raw";
import { FX_CARD_FILES, isSignature, parseFxCardTable, type FxCardEntry } from "./tables.ts";

const FILE = FX_CARD_FILES.find((candidate) => candidate.file === "stinkyscape-2.json5");
if (FILE === undefined) throw new Error("no stinkyscape-2.json5");
const TABLE = parseFxCardTable(FILE.file, FILE.source);
const RANGE = [...TABLE.keys()];
const CARDS = RANGE.filter((id) => CATALOG[id]?.token !== true);

/** The Starship's own cards (#2782's Starship part): its fuse and launch, Launch Sequencer, the Retrofitting Program and the Launchers. */
const STARSHIP_PARTS = [
  "stinkyscape-106",
  "stinkyscape-130",
  "stinkyscape-133",
  "stinkyscape-136",
  "stinkyscape-137",
  "stinkyscape-140",
  "stinkyscape-141",
  "stinkyscape-142",
  "stinkyscape-143",
  "stinkyscape-176",
  "stinkyscape-t-dormant-starship",
  "stinkyscape-t-ai-05",
];
const STARSHIP_PART = "Starship part";
/** R2801000: the Starship parts the Starship's moment gives an entry: the Retrofitting Program's send, the Launchers' conjure and Launch Sequencer's tithe. */
const STARSHIP_ENTRIES = ["stinkyscape-130", "stinkyscape-140", "stinkyscape-141", "stinkyscape-142", "stinkyscape-143", "stinkyscape-t-ai-05"];
/** R3334004: the Starship parts with signatures of their own: Starfleet Commander's and the Dormant Starship's entrances, Preemptive Strike's recipe. */
const STARSHIP_SIGNATURES = ["stinkyscape-137", "stinkyscape-176", "stinkyscape-t-dormant-starship"];

function entryOf(id: string): FxCardEntry {
  const entry = TABLE.get(id);
  if (entry === undefined) throw new Error(`${id} has no entry`);
  return entry;
}

function envOf(): FxPlanEnv {
  return { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
}

const played = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 1 });

/** The play's motif on its gallery fixture, as the board plans it once the play's `cardPlayed` is remembered. */
function playMotif(defId: string, instanceId: string, motif: Parameters<typeof motifFixture>[0]) {
  const view = fullBoardView();
  const env = envOf();
  env.memory.remember([played("p1", instanceId, defId)]);
  const entry = { ...galleryEntry(motifFixture(motif, view, instanceId, "p1"), view), durationMs: 400 };
  return planCardMotif(entry, view, env);
}

/** Each card's play motif: its `cry` (a Unit) or `cast`, when a drawn motif. */
function playMotifs(): { id: string; motif: Parameters<typeof motifFixture>[0] }[] {
  return RANGE.flatMap((id) => {
    const entry = entryOf(id);
    if (entry.kind !== "hooks") return [];
    const hook = entry.hooks[CATALOG[id]?.type === "Unit" ? "cry" : "cast"];
    return hook !== undefined && isDrawnMotif(hook.motif) ? [{ id, motif: hook.motif }] : [];
  });
}

/** As the board resolves an entrance: the card's type and rarity, a token's the one it prints (B2.5). */
const resolve: EntranceResolver = (defId) => {
  const def = CATALOG[defId];
  return def === undefined ? null : { type: def.type, rarity: def.printedRarity ?? def.rarity };
};

/** A cue's end, as R200's tests count it (tiers.test.ts): bursts burn, shots fly then burst, shakes ride, the rest run out. */
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

/** R202: `events` as a seat that may not read the card sees its play: its play, resolution and landing name no definition. */
function hidden(events: readonly GameEvent[]): GameEvent[] {
  return events.map((event): GameEvent => {
    if (event.type === "cardPlayed" || event.type === "cardResolved") return { ...event, defId: "hidden", instanceId: "hidden" };
    if (event.type === "summoned") return { ...event, defId: "hidden" };
    return event;
  });
}

/**
 * `defId`'s gallery play over its gallery board, entered and held as the queue holds it (speed 1, Big
 * moments full), each entry remembered then planned as the board's effects layer plans it: every cue
 * (`planFx`, its rarity's entrance among them), the signature's own (`planCardFx`, its entrance or
 * recipe), and each entry's start in the play.
 */
function galleryPlan(defId: string, events: readonly GameEvent[], board: PlayerView) {
  const fielded = withEvents(galleryFieldView(defId, board), [...events]);
  const entered = entranceEntries(planEntries(events, fielded, false, undefined, hasSetPiece), resolve, { seen: new Set() });
  const planned = setPieceEntries(entered, (entry) => wipeClaim(entry) ?? takeoverClaim(entry), { reduced: false, bigMoments: "full", speed: 1 });
  const env = envOf();
  let offset = 0;
  return planned.map((entry) => {
    env.memory.remember(entry.events);
    const D = entry.durationMs;
    const own = entry.events.flatMap((event) => planCardFx(event, { entry, view: entry.view, env, D })?.cues ?? []);
    const at = offset;
    offset += D;
    return { D, at, own, cues: planFx(entry, entry.view, env) };
  });
}

/** Each recipe entry's cues over its gallery fixture, the play `defId` resolving, each event its own entry. */
function recipeCues(defId: string, key: string, instanceId: string): number {
  if (!isCardRecipeKey(key)) throw new Error(`${key} is no card recipe`);
  const view = fullBoardView();
  const env = envOf();
  env.memory.remember([played("p1", instanceId, defId)]);
  // R3407000: a signature's fixture is told the card it plays, so a hook's marker names it.
  return RECIPE_FIXTURES[key](view, instanceId, "p1", defId).reduce((n, event) => {
    env.memory.remember([event]);
    const entry = { ...galleryEntry([event], view), durationMs: 400 };
    return n + (planCardFx(event, { entry, view, env, D: 400 })?.cues.length ?? 0);
  }, 0);
}

describe("R2784000 #2798 Stinkyscape's card animation pass, part two", () => {
  it("R2784000 #2798 the file runs from stinkyscape-091 to the end, tokens included, and no line is pending", () => {
    expect(RANGE[0]).toBe("stinkyscape-091");
    expect(RANGE).toHaveLength(120);
    expect(CARDS).toHaveLength(88);
    for (const id of RANGE) expect(describeEntry(entryOf(id)), id).not.toBe("none: pending");
  });

  it("R2784000 #2798 at least FX_SET_PASS_FLOOR of the range's non-token cards have an entry other than none", () => {
    const drawn = CARDS.filter((id) => entryOf(id).kind !== "none");
    expect(drawn.length).toBeGreaterThanOrEqual(Math.ceil(FX_SET_PASS_FLOOR * CARDS.length));
  });

  it("R2784000 #2798 every non-token non-Unit has a cast, bespoke or set piece but the Starship parts, and every Field Spell, tokens included, an aura", () => {
    // 11 of the set's 17 Field Spells, two of them tokens (Wardrum Blob, Drone Uplink).
    const fields = RANGE.filter((id) => CATALOG[id]?.type === "Field Spell");
    expect(fields).toHaveLength(11);
    for (const id of RANGE) {
      const type = CATALOG[id]?.type;
      const entry = entryOf(id);
      // R3407000: a signature's bespoke entry wears its aura beside its recipe.
      if (type === "Field Spell") expect(entry.kind === "hooks" || entry.kind === "bespoke" ? entry.aura : undefined, id).toBeDefined();
      if (type === "Unit" || CATALOG[id]?.token === true || STARSHIP_PARTS.includes(id)) continue;
      const plays = entry.kind === "bespoke" || entry.kind === "setPiece" || (entry.kind === "hooks" && entry.hooks.cast !== undefined);
      expect(plays, `${id} (${String(type)})`).toBe(true);
    }
  });

  it("R2784000 #2798 exactly the Starship parts without an entry of their own are none: Starship part", () => {
    const held = RANGE.filter((id) => {
      const entry = entryOf(id);
      return entry.kind === "none" && entry.reason === STARSHIP_PART;
    });
    expect(held).toEqual(STARSHIP_PARTS.filter((id) => !STARSHIP_ENTRIES.includes(id) && !STARSHIP_SIGNATURES.includes(id)));
    for (const id of STARSHIP_ENTRIES) expect(entryOf(id).kind, id).toBe("hooks");
    // R3334004: and the three with signatures of their own.
    for (const id of STARSHIP_SIGNATURES) expect(isSignature(entryOf(id)), id).toBe(true);
  });

  it("R2784000 #2798 every hook names a drawn motif, and every recipe is a card recipe", () => {
    for (const id of RANGE) {
      const entry = entryOf(id);
      if (entry.kind === "bespoke") expect(isCardRecipeKey(entry.key), id).toBe(true);
      if (entry.kind !== "hooks") continue;
      for (const [hook, spec] of Object.entries(entry.hooks)) expect(isDrawnMotif(spec.motif), `${id} ${hook} ${spec.motif}`).toBe(true);
    }
  });

  it("R2784000 #2798 every cast and cry motif draws on its gallery fixture as its card's play, and every recipe on its own", () => {
    const motifs = playMotifs();
    expect(motifs.length).toBeGreaterThan(0);
    for (const { id, motif } of motifs) {
      const result = playMotif(id, "ex", motif);
      expect(result, `${id} ${motif}`).not.toBeNull();
      expect(result?.cues.length ?? 0, `${id} ${motif}`).toBeGreaterThan(0);
      // R202: no cue names a definition.
      expect(JSON.stringify(result?.cues), id).not.toMatch(/(core|stinkyscape)-\d{3}/);
    }
    for (const id of RANGE) {
      const entry = entryOf(id);
      if (entry.kind === "bespoke") expect(recipeCues(id, entry.key, "ex"), id).toBeGreaterThan(0);
    }
  });

  it("R202 #2798 a hidden play keys none of the file's entries", () => {
    for (const { id, motif } of playMotifs()) expect(playMotif("hidden", "hidden", motif), `${id} ${motif}`).toBeNull();
    for (const id of RANGE) {
      const entry = entryOf(id);
      if (entry.kind === "bespoke") expect(recipeCues("hidden", entry.key, "hidden"), id).toBe(0);
    }
  });

  it("R3334004 each signature plans on its gallery play, none hidden", () => {
    const view = fullBoardView();
    expect(STARSHIP_SIGNATURES.every((id) => STARSHIP_PARTS.includes(id))).toBe(true);
    for (const defId of STARSHIP_SIGNATURES) {
      const def = CATALOG[defId];
      expect(def?.rarity, defId).toBe("Legendary");
      const spell = def?.type === "Spell";
      const ceiling = SIGNATURE_LEGENDARY_MAX_MS * (spell ? SIGNATURE_SPELL_SHARE : 1);
      const maxTrauma = spell ? SIGNATURE_SPELL_MAX_TRAUMA : SIGNATURE_PERMANENT_MAX_TRAUMA;
      const play = galleryPlayOf(defId, entryOf(defId));
      if (play === null) throw new Error(`${defId} has no gallery play`);
      const board = galleryBoardOf(defId, view);
      const events = galleryCardEvents(defId, play, board, 1);
      const plan = galleryPlan(defId, events, board);
      // Its signature plans on its gallery play, and no cue names a card (R202).
      const own = plan.flatMap((entry) => entry.own);
      expect(own.length, `${defId} plans`).toBeGreaterThan(0);
      expect(JSON.stringify(plan.map((entry) => entry.cues)), defId).not.toMatch(/stinkyscape-/);
      for (const { D, at, cues } of plan) {
        for (const cue of cues) {
          const where = `${defId} ${cue.kind} at ${String(at)}`;
          // A pull and a conceal are the stage's: the board's next view ends them (R200).
          if (cue.kind === "pull" || cue.kind === "conceal") continue;
          // R200: it starts inside its entry and ends within FX_MAX_TAIL_MS of it.
          expect(cue.delayMs, where).toBeGreaterThanOrEqual(0);
          expect(cue.delayMs, where).toBeLessThanOrEqual(D);
          expect(endOf(cue), where).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
          // R3332000: within a Legendary's ceiling, a Spell's share of it, and its type's loudest shake.
          expect(at + endOf(cue), where).toBeLessThanOrEqual(ceiling);
          if (cue.kind === "shake") expect(cue.trauma, where).toBeLessThanOrEqual(maxTrauma);
        }
      }
      // R202, R97: the same play hidden keys none of it.
      const unread = galleryPlan(defId, hidden(events), board);
      expect(unread.flatMap((entry) => entry.own), `${defId} hidden`).toEqual([]);
      expect(JSON.stringify(unread.map((entry) => entry.cues)), `${defId} hidden`).not.toMatch(/stinkyscape-/);
    }
  });

  it("R2784000 #2798 stinkyscape-2.md has one row per card of the file, in order, with its name, type, entry, scale, census and why", () => {
    const rows = rawTable
      .split("\n")
      .filter((line) => line.startsWith("| `stinkyscape-"))
      .map((line) => line.slice(1, -1).split(" | ").map((cell) => cell.trim()));
    expect(rows.map((row) => row[0])).toEqual(RANGE.map((id) => `\`${id}\``));
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
