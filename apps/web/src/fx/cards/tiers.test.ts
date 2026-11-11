// R2803000: the audit's tier test over every fx/cards table entry (docs/polish/9-card-animations.md,
// "The tables"): every cue its gallery play plans ends within its entry plus FX_MAX_TAIL_MS (R200);
// every set piece's hold stays within SET_PIECE_HOLD_CAP_MS, and only one set piece holds per action
// (SET_PIECES_PER_ACTION, R2785000). Planned as the board's effects layer plans it: remember, then
// plan (the gallery's own pipeline in routes/dev/fx.test.tsx). An `aura` play plans no cues: the aura
// layer draws the standing look, fading in and out by its own constants (R2787000).
// R3332000, R3399005: each of a signature's own cues (its entrance or takeover, its motif or recipes, its set
// piece; not the rows every card's events share) ends within its rarity's budget of the start of the entry it
// plays in (SIGNATURE_LEGENDARY_MAX_MS, SIGNATURE_MYTHIC_MAX_MS), a Spell's within SIGNATURE_SPELL_SHARE of it but
// a Mythic's takeover, its hold included, within the Mythic's whole; its shakes stay within its type's
// (SIGNATURE_SPELL_MAX_TRAUMA, SIGNATURE_PERMANENT_MAX_TRAUMA), and its aura within AURA_SIGNATURE_MAX_INTENSITY.
// Each is planned over its gallery board (`galleryBoardOf`).
// R3389000: a signature bespoke's aura is held to it too (`entryAura`), its own look's numbers (`auraLookSpec`).
// R3395001: a signature's ceiling counts its own beats: another signature's own entrance its play summons
// (the MAGI's under Imitated Instrumentality, R3407000) is that card's, held to its own ceiling in its own
// gallery play, and still to R200 by the test above.
// R3402000: a later action of a gallery play (a Trap's firing, a trigger at a turn's end, an Activate on a later
// click) comes in a game a turn or more after the play, so it is timed from its own start: R3399005's measure
// does so, timing each own cue from the start of its own entry.

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import {
  entranceEntries,
  planEntries,
  SET_PIECE_HOLD_CAP_MS,
  SET_PIECES_PER_ACTION,
  STARSHIP_LAUNCH_HOLD_MS,
  TAKEOVER_HOLD_MS,
  setPieceEntries,
  takeoverClaim,
  type AnimationEntry,
  type EntranceResolver,
} from "../../game/animations.ts";
import { fullBoardView, withEvents } from "../../test/fixtures.ts";
import { CATALOG } from "../../wire/cards.ts";
import { planCardFx, planCardMotif, planSignatureSweep, planSignatureZone } from "../cardFx.ts";
import { entryAura, FX_CARD_TABLE, isSignature } from "./tables.ts";
import { auraLookSpec } from "../aura/looks.ts";
import {
  AURA_SIGNATURE_MAX_INTENSITY,
  FX_HOLD_MAX_MS,
  FX_INTENSITY_SCALE,
  FX_MAX_PARTICLE_LIFE_MS,
  FX_MAX_TAIL_MS,
  SIGNATURE_LEGENDARY_MAX_MS,
  SIGNATURE_MYTHIC_MAX_MS,
  SIGNATURE_PERMANENT_MAX_TRAUMA,
  SIGNATURE_SPELL_MAX_TRAUMA,
  SIGNATURE_SPELL_SHARE,
} from "../constants.ts";
import { planFx } from "../cues.ts";
import { createFxMemory } from "../memory.ts";
import { galleryBoardOf, galleryCardEvents, galleryFieldView, galleryPlayOf } from "../motifs/gallery.ts";
import { hasSetPiece, SET_PIECE_KEYS, SET_PIECES, planSetPiece, signatureRunOf, wipeClaim } from "../setPieces.ts";
import { planTerminus } from "../terminus/index.ts";
import { entranceCues } from "../takeover.ts";
import type { FxCue, FxPlanEnv } from "../types.ts";

/** A cue's end, as R2800000's own R200 test counts it: bursts burn, shots fly then burst, shakes ride, the rest run out. */
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

function envOf(): FxPlanEnv {
  return { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
}

/** As the board resolves an entrance: the card's type and rarity, a token's the one it prints (B2.5). */
const resolve: EntranceResolver = (defId) => {
  const def = CATALOG[defId];
  if (def === undefined) return null;
  return { type: def.type, rarity: def.printedRarity ?? def.rarity };
};

/** Every cue the layer plans for `planned`: its entrance, its motif and recipes, and the full plan. */
function cuesOf(planned: AnimationEntry, env: FxPlanEnv): FxCue[] {
  const cues: FxCue[] = [...entranceCues(planned, env, planned.durationMs), ...planFx(planned, planned.view, env)];
  for (const event of planned.events) {
    const recipe = planCardFx(event, { entry: planned, view: planned.view, env, D: planned.durationMs });
    if (recipe !== null) cues.push(...recipe.cues);
  }
  return cues;
}

/**
 * R3399005: a signature's own cues in one entry, as `planFx` draws them for its card: its entrance (a Mythic's takeover
 * among them), and its motif, set piece or Terminus run and its recipes, or in a pile's one wave its own recipe's alone.
 * The rows every card's events share (and a cast's, a keyword's beat, a set's system moment) are no signature's.
 */
function ownCuesOf(planned: AnimationEntry, env: FxPlanEnv): { entrance: FxCue[]; rest: FxCue[] } {
  const { view } = planned;
  const entrance = entranceCues(planned, env, planned.durationMs);
  if (planned.sweep === undefined && planned.zone !== undefined) return { entrance, rest: planSignatureZone(planned, view, env) };
  const rest: FxCue[] = [
    ...(planCardMotif(planned, view, env)?.cues ?? []),
    ...(planSetPiece(planned, view, env)?.cues ?? []),
    ...(planTerminus(planned, view, env)?.cues ?? []),
  ];
  if (planned.sweep !== undefined) {
    rest.push(...planSignatureSweep(planned, view, env).cues);
  } else {
    for (const event of planned.events) {
      const recipe = planCardFx(event, { entry: planned, view, env, D: planned.durationMs });
      if (recipe !== null) rest.push(...recipe.cues);
    }
  }
  return { entrance, rest };
}

describe("R2803000 every table entry ends within FX_MAX_TAIL_MS", () => {
  it("R2803000 every table entry's cues end within FX_MAX_TAIL_MS of its entry", () => {
    const view = fullBoardView();
    const drawable = [...FX_CARD_TABLE].filter(([, entry]) => entry.kind !== "none");
    expect(drawable.length).toBeGreaterThan(0);
    for (const [defId, entry] of drawable) {
      const play = galleryPlayOf(defId, entry);
      // An `aura` play stands its look and plans no cues (R2787000).
      if (play === null || play.kind === "aura") continue;
      // R3332001: over the board its entrance reads, as the gallery plays it.
      const board = galleryBoardOf(defId, view);
      const events = galleryCardEvents(defId, play, board, 1);
      const fielded = withEvents(galleryFieldView(defId, board), events);
      const env = envOf();
      const entered = entranceEntries(planEntries(events, fielded, false, undefined, hasSetPiece), resolve, { seen: new Set() });
      const planned = entered.length > 0 ? entered : planEntries(events, fielded, false, undefined, hasSetPiece);
      const all: FxCue[] = [];
      // A resolving tail (its `cardResolved` alone) plans nothing; the play's entry carries the motif.
      let drawn = false;
      for (const entryPlanned of planned) {
        env.memory.remember(entryPlanned.events);
        const D = entryPlanned.durationMs;
        const cues = cuesOf(entryPlanned, env);
        if (cues.length > 0) drawn = true;
        for (const cue of cues) {
          const where = `${defId} ${play.kind} ${cue.kind}`;
          expect(cue.delayMs, where).toBeGreaterThanOrEqual(0);
          expect(cue.delayMs, where).toBeLessThanOrEqual(D);
          // A pull and a conceal are the stage's: the board's next view ends them (R200).
          if (cue.kind === "pull" || cue.kind === "conceal") continue;
          expect(endOf(cue), where).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
        }
        all.push(...cues);
      }
      expect(drawn, `${defId} ${play.kind} plans`).toBe(true);
      // R202: no cue names the card.
      expect(JSON.stringify(all), `${defId} names no card`).not.toContain(defId);
    }
  });

  it("R2803000 a hidden play keys no table entry", () => {
    const view = fullBoardView();
    for (const [defId, entry] of FX_CARD_TABLE) {
      if (entry.kind !== "hooks") continue;
      const play = galleryPlayOf(defId, entry);
      if (play === null || (play.kind !== "board" && play.kind !== "hook")) continue;
      const events = galleryCardEvents(defId, play, view, 1).map((event): GameEvent => {
        if (event.type === "cardPlayed" || event.type === "cardResolved") return { ...event, defId: "hidden", instanceId: "hidden" };
        if (event.type === "hookResolving" || event.type === "trapFired") return { ...event, defId: "hidden" };
        return event;
      });
      const env = envOf();
      const entered = planEntries(events, withEvents(view, events), false, undefined, hasSetPiece);
      for (const plannedEntry of entered) {
        env.memory.remember(plannedEntry.events);
        expect(planCardMotif(plannedEntry, plannedEntry.view, env), `${defId} hidden`).toBeNull();
      }
    }
  });
});

/**
 * R3395001: the cues in `planned` of another signature's own entrance, a card `defId`'s play summons (each
 * MAGI landing under Imitated Instrumentality's boots, R3407000): that card's own beats, planned by its entry
 * alone and held to its own ceiling in its own gallery play, so not counted against `defId`'s.
 */
function othersEntrances(defId: string, planned: AnimationEntry, env: FxPlanEnv): ReadonlySet<string> {
  const theirs = new Set<string>();
  for (const event of planned.events) {
    if (event.type !== "summoned" || event.defId === defId) continue;
    const own = FX_CARD_TABLE.get(event.defId);
    if (own === undefined || own.kind === "none" || own.entrance === undefined || !isSignature(own)) continue;
    const p = { entry: planned, view: planned.view, env, D: planned.durationMs };
    const entrance = (planCardFx(event, p, new Map([[event.defId, own]]))?.cues ?? []).map((cue) => JSON.stringify(cue));
    // What the play's own recipe draws beside it: everything the full plan draws but each entrance cue once.
    const rest = [...entrance];
    const mine = new Set((planCardFx(event, p)?.cues ?? []).map((cue) => JSON.stringify(cue)).filter((cue) => {
      const at = rest.indexOf(cue);
      if (at >= 0) rest.splice(at, 1);
      return at < 0;
    }));
    // A cue the recipe draws too stays counted: only the other card's alone is skipped.
    for (const cue of entrance) if (!mine.has(cue)) theirs.add(cue);
  }
  return theirs;
}

describe("R3332000 every signature keeps its loudness budget", () => {
  it("R3332000 R3399005 R3402000 each of a signature's own cues ends within its rarity's and type's ceiling of its entry's start, a later action's among them, shakes within its type's, and its aura within AURA_SIGNATURE_MAX_INTENSITY", () => {
    const view = fullBoardView();
    const signatures = [...FX_CARD_TABLE].filter(([, entry]) => isSignature(entry));
    expect(signatures.length).toBeGreaterThan(0);
    for (const [defId, entry] of signatures) {
      const def = CATALOG[defId];
      const rarity = def?.printedRarity ?? def?.rarity;
      const spell = def?.type === "Spell";
      const ceiling = (rarity === "Mythic" ? SIGNATURE_MYTHIC_MAX_MS : SIGNATURE_LEGENDARY_MAX_MS) * (spell ? SIGNATURE_SPELL_SHARE : 1);
      // R3399005: a Mythic's entrance beat is its takeover, its hold included, and keeps the Mythic's whole budget.
      const entranceCeiling = rarity === "Mythic" ? SIGNATURE_MYTHIC_MAX_MS : ceiling;
      const maxTrauma = spell ? SIGNATURE_SPELL_MAX_TRAUMA : SIGNATURE_PERMANENT_MAX_TRAUMA;
      const play = galleryPlayOf(defId, entry);
      if (play === null) throw new Error(`${defId}'s signature has no gallery play`);
      // Its gallery play over its own board, entered and held as the queue holds it: a Mythic's takeover or
      // a wipe's set piece, at speed 1 with Big moments full, its loudest.
      const board = galleryBoardOf(defId, view);
      const events = galleryCardEvents(defId, play, board, 1);
      const fielded = withEvents(galleryFieldView(defId, board), events);
      const entered = entranceEntries(planEntries(events, fielded, false, undefined, hasSetPiece), resolve, { seen: new Set() });
      const planned = setPieceEntries(entered, (e) => wipeClaim(e) ?? takeoverClaim(e), { reduced: false, bigMoments: "full", speed: 1 });
      const env = envOf();
      let drawn = false;
      for (const entryPlanned of planned) {
        env.memory.remember(entryPlanned.events);
        if (cuesOf(entryPlanned, env).length > 0) drawn = true;
        // Each own cue from the start of the entry it plays in: the shared rows of the card's text's events, in
        // entries after its cast or its landing, count against no signature (R3399005); a later action's entries
        // (R3402000) among them.
        const { entrance, rest } = ownCuesOf(entryPlanned, env);
        const beats: readonly (readonly [readonly FxCue[], number])[] = [
          [entrance, entranceCeiling],
          [rest, ceiling],
        ];
        const others = othersEntrances(defId, entryPlanned, env);
        for (const [cues, limit] of beats) {
          for (const cue of cues) {
            const where = `${defId} ${cue.kind} at ${String(cue.delayMs)} of ${String(entryPlanned.durationMs)}`;
            // A pull and a conceal are the stage's: the board's next view ends them (R200).
            if (cue.kind === "pull" || cue.kind === "conceal") continue;
            // R3395001: another signature's own entrance is held to its own ceiling.
            if (others.has(JSON.stringify(cue))) continue;
            expect(endOf(cue), where).toBeLessThanOrEqual(limit);
            if (cue.kind === "shake") expect(cue.trauma, where).toBeLessThanOrEqual(maxTrauma);
          }
        }
      }
      expect(drawn, `${defId}'s signature plans`).toBe(true);
      // R3389000: a signature bespoke's aura too (R3400000), its own look among them.
      const aura = entryAura(entry);
      if (aura !== undefined) {
        const look = auraLookSpec(aura.look);
        for (const value of [look.alpha, look.tint, look.edge]) expect(value, `${defId} aura ${aura.look}`).toBeLessThanOrEqual(AURA_SIGNATURE_MAX_INTENSITY);
      }
    }
  });
});

describe("R2803000 every set piece hold is capped and single per action", () => {
  it("R2803000 every set piece hold is capped at SET_PIECE_HOLD_CAP_MS", () => {
    expect(TAKEOVER_HOLD_MS, "takeover").toBeLessThanOrEqual(SET_PIECE_HOLD_CAP_MS);
    expect(STARSHIP_LAUNCH_HOLD_MS, "starship").toBeLessThanOrEqual(SET_PIECE_HOLD_CAP_MS);
    for (const key of SET_PIECE_KEYS) {
      expect(SET_PIECES[key].holdMs, key).toBeLessThanOrEqual(SET_PIECE_HOLD_CAP_MS);
    }
  });

  it("R2803000 one action takes at most one set piece hold", () => {
    expect(SET_PIECES_PER_ACTION).toBe(1);
  });

  it("R2803000 every set piece cue ends within FX_MAX_TAIL_MS of its entry", () => {
    const view = fullBoardView();
    for (const key of SET_PIECE_KEYS) {
      const [defId] = [...FX_CARD_TABLE].find(([, entry]) => entry.kind === "setPiece" && entry.key === key) ?? [];
      if (defId === undefined) throw new Error(`no card names ${key}`);
      const events = galleryCardEvents(defId, { kind: "setPiece", key }, view, 1);
      const env = envOf();
      // R3338000: a board clear's is its wipe, a Terminus card's its signature.
      const entered = planEntries(events, withEvents(view, events), false, undefined, hasSetPiece, signatureRunOf);
      const piece = entered.find((entry) => entry.wipe !== undefined || entry.signature !== undefined);
      if (piece === undefined) throw new Error(`${defId} has no set piece entry`);
      const D = piece.durationMs;
      const cues = (planSetPiece(piece, view, env) ?? planTerminus(piece, view, env))?.cues ?? [];
      expect(cues.length, key).toBeGreaterThan(0);
      for (const cue of cues) {
        if (cue.kind === "pull" || cue.kind === "conceal") {
          // The stage's: the board's next view ends them, FX_HOLD_MAX_MS at the latest (R200).
          expect(cue.durationMs, `${key} ${cue.kind}`).toBe(FX_HOLD_MAX_MS);
          continue;
        }
        expect(endOf(cue), `${key} ${cue.kind}`).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
      }
    }
  });
});
