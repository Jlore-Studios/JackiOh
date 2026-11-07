// R2800000: each board clear's set piece as the effects layer plans it (setPieces.ts): the vortex or
// the fold over the whole layer, every card the wipe takes drawn in, concealed at the swallow and
// imploded into the centre, all inside its entry and gone within FX_MAX_TAIL_MS of its end (R200). It
// keys on the hook's readable card alone, and no cue names a definition (R202). Ceaseless Void lands
// with its own collapse (R670) and its takeover yields to its fold (R2722001).

import type { GameEvent, PlayerView, UnitView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import {
  ANIMATIONS,
  SET_PIECE_HOLD_CAP_MS,
  SET_PIECE_SHORT_HOLD_MS,
  WIPE_MS,
  planEntries,
  setPieceEntries,
  type AnimationEntry,
} from "../game/animations.ts";
import { testid } from "../game/contract.ts";
import { createPlayTracker } from "../game/runs.ts";
import { fullBoardView } from "../test/fixtures.ts";
import { entranceKeyOf } from "./cardFx.ts";
import { FX_CARD_TABLE } from "./cards/tables.ts";
import { FX_HOLD_MAX_MS, FX_INTENSITY_SCALE, FX_MAX_PARTICLE_LIFE_MS, FX_MAX_TAIL_MS, FX_MOTIF_CENTER_LANE } from "./constants.ts";
import { planFx } from "./cues.ts";
import { ENTRANCES } from "./entrances.ts";
import { createFxMemory } from "./memory.ts";
import {
  SET_PIECE_KEYS,
  SET_PIECE_SWALLOW_AT,
  SET_PIECE_TAIL_MS,
  SET_PIECES,
  hasSetPiece,
  planSetPiece,
  setPieceKeyOf,
  takenOf,
  wipeClaim,
  type SetPieceKey,
} from "./setPieces.ts";
import { entranceCues } from "./takeover.ts";
import type { FxCue, FxPlanEnv, FxSetPieceCue } from "./types.ts";

const env = (intensity: number = FX_INTENSITY_SCALE.normal): FxPlanEnv => ({ intensity, card: () => undefined, memory: createFxMemory() });

/** The first card whose table entry names `key`. */
function cardOf(key: SetPieceKey): string {
  const found = [...FX_CARD_TABLE].find(([, entry]) => entry.kind === "setPiece" && entry.key === key);
  if (found === undefined) throw new Error(`no card names ${key}`);
  return found[0];
}

const unitsOf = (view: PlayerView, side: "you" | "opponent"): UnitView[] => view[side].units.flatMap((u) => (u === null ? [] : [u]));

function removal(type: "destroyed" | "exiled", u: UnitView): GameEvent {
  return type === "destroyed"
    ? { type, instanceId: u.instanceId, defId: u.defId, owner: u.owner, controller: u.controller, attack: u.attack, maxHealth: u.maxHealth, killerId: null }
    : { type, instanceId: u.instanceId, defId: u.defId, owner: u.owner };
}

/** The wipe entry of `defId`'s hook over `units` from p1, held at `holdMs` (none: unheld). */
function wipeEntry(defId: string, units: readonly UnitView[], holdMs?: number, view: PlayerView = fullBoardView()): AnimationEntry {
  const type = defId === "core-100" ? "exiled" : "destroyed";
  const events: GameEvent[] = [
    { type: "cardPlayed", player: "p1", instanceId: "src", defId, costPaid: 4 },
    { type: "hookResolving", sourceId: "src", defId, controller: "p1", hook: "cry" },
    ...units.map((u) => removal(type, u)),
  ];
  const planned = planEntries(events, view, false, createPlayTracker(), hasSetPiece);
  const held = holdMs === undefined ? planned : setPieceEntries(planned, (entry) => (wipeClaim(entry) === null ? null : { key: wipeClaim(entry)?.key ?? "", holdMs }), {
    reduced: false,
    bigMoments: "full",
    speed: 1,
  });
  const found = held.find((entry) => entry.wipe !== undefined);
  if (found === undefined) throw new Error("no wipe");
  return found;
}

const pieceOf = (cues: readonly FxCue[]): FxSetPieceCue | undefined => cues.find((c): c is FxSetPieceCue => c.kind === "setPiece");

function endOf(cue: FxCue): number {
  switch (cue.kind) {
    case "burst":
      return cue.delayMs + FX_MAX_PARTICLE_LIFE_MS;
    case "shake":
      return cue.delayMs;
    default:
      return cue.delayMs + ("durationMs" in cue ? cue.durationMs : 0);
  }
}

describe("R2800000 a board clear's set piece", () => {
  it.each(SET_PIECE_KEYS)("R2800000 each set piece plans its cue, pulls, conceals and implosion: %s", (key) => {
    const view = fullBoardView();
    const units = [...unitsOf(view, "you"), ...unitsOf(view, "opponent")];
    const defId = cardOf(key);
    const spec = SET_PIECES[key];
    const entry = wipeEntry(defId, units, spec.holdMs, view);
    expect(entry.setPiece).toEqual({ key, holdMs: spec.holdMs });
    const plan = planSetPiece(entry, view, env());
    expect(plan).not.toBeNull();
    const cues = plan?.cues ?? [];
    const piece = pieceOf(cues);
    expect(piece).toMatchObject({ shape: spec.shape, spin: spec.spin, side: null, holdMs: spec.holdMs, delayMs: 0, durationMs: entry.durationMs + SET_PIECE_TAIL_MS });
    // A Spell's wipe of both sides centres on the board.
    expect(piece?.at).toEqual({ kind: "testid", testid: testid.board });
    for (const u of units) {
      const card = testid.card(u.instanceId);
      const pull = cues.find((c) => c.kind === "pull" && c.testid === card);
      // The vortex draws each card into its centre; the fold sinks it where it stands.
      expect(pull, card).toEqual({ kind: "pull", testid: card, to: { kind: "testid", testid: spec.shape === "fold" ? card : testid.board }, delayMs: 0, durationMs: FX_HOLD_MAX_MS });
      expect(cues.find((c) => c.kind === "conceal" && c.testid === card), card).toEqual({ kind: "conceal", testid: card, mode: "after", delayMs: spec.holdMs, durationMs: FX_HOLD_MAX_MS });
    }
    // The swallow at the end of the hold: an implosion, a ring and the shake.
    const atSwallow = cues.filter((c) => c.delayMs === spec.holdMs).map((c) => c.kind);
    expect(atSwallow).toEqual(expect.arrayContaining(["burst", "ring", "shake", "conceal"]));
    if (spec.shape === "fold") expect(atSwallow).toContain("rays");
    // The wipe's removals play inside the set piece: planFx draws it and none of their own rows.
    expect(plan?.replaces.size).toBe(units.length);
    expect(planFx(entry, view, env())).toEqual(cues);
  });

  it("R2800000 R200 every set piece cue ends within FX_MAX_TAIL_MS of its entry", () => {
    const view = fullBoardView();
    const units = [...unitsOf(view, "you"), ...unitsOf(view, "opponent")];
    for (const key of SET_PIECE_KEYS) {
      for (const holdMs of [undefined, SET_PIECE_SHORT_HOLD_MS, SET_PIECES[key].holdMs, SET_PIECE_HOLD_CAP_MS]) {
        const entry = wipeEntry(cardOf(key), units, holdMs, view);
        const D = entry.durationMs;
        expect(D).toBe(WIPE_MS + (holdMs ?? 0));
        const cues = planSetPiece(entry, view, env())?.cues ?? [];
        expect(cues.length).toBeGreaterThan(0);
        // Unheld, it swallows inside the entry, at SET_PIECE_SWALLOW_AT of it.
        if (holdMs === undefined) expect(pieceOf(cues)?.holdMs).toBe(Math.round(SET_PIECE_SWALLOW_AT * D));
        for (const cue of cues) {
          const where = `${key} held ${String(holdMs)}: ${cue.kind}`;
          expect(cue.delayMs, where).toBeGreaterThanOrEqual(0);
          expect(cue.delayMs, where).toBeLessThanOrEqual(D);
          // A pull and a conceal are the stage's: the board's next view ends them, FX_HOLD_MAX_MS at the latest.
          if (cue.kind === "pull" || cue.kind === "conceal") continue;
          expect(endOf(cue), where).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
        }
      }
    }
  });

  it("R2800000 one side's vortex centres on that side", () => {
    const view = fullBoardView();
    const entry = wipeEntry(cardOf("nether"), unitsOf(view, "opponent"), SET_PIECES.nether.holdMs, view);
    const cues = planSetPiece(entry, view, env())?.cues ?? [];
    const centre = { kind: "testid", testid: testid.zone("opponent", "units", FX_MOTIF_CENTER_LANE) };
    expect(pieceOf(cues)).toMatchObject({ side: "opponent", at: centre });
    // It draws in the enemy cards alone; the caster's side is untouched.
    const pulled = cues.flatMap((c) => (c.kind === "pull" ? [c.testid] : []));
    expect(pulled).toEqual(unitsOf(view, "opponent").map((u) => testid.card(u.instanceId)));
    for (const u of unitsOf(view, "you")) expect(JSON.stringify(cues)).not.toContain(`"${testid.card(u.instanceId)}"`);
  });

  it("R2800000 R202 a sentinel source plans nothing; no cue names a card", () => {
    const view = fullBoardView();
    const units = [...unitsOf(view, "you"), ...unitsOf(view, "opponent")];
    const entry = wipeEntry(cardOf("vortex"), units, SET_PIECES.vortex.holdMs, view);
    const wipe = entry.wipe;
    if (wipe === undefined) throw new Error("no wipe");
    expect(setPieceKeyOf("hidden")).toBeUndefined();
    expect(planSetPiece({ ...entry, wipe: { ...wipe, by: "hidden" } }, view, env())).toBeNull();
    // A card with no set piece plans none either.
    expect(planSetPiece({ ...entry, wipe: { ...wipe, by: "core-034" } }, view, env())).toBeNull();
    // No cue carries a definition: only the board's own testids.
    const text = JSON.stringify(planSetPiece(entry, view, env())?.cues);
    for (const defId of new Set([...units.map((u) => u.defId), wipe.by])) expect(text).not.toContain(defId);
  });

  it("R2800000 a bounce into a hidden hand takes the side's next card", () => {
    const view = fullBoardView();
    const [first, second] = unitsOf(view, "opponent");
    const named = unitsOf(view, "you")[1];
    if (first === undefined || second === undefined || named === undefined) throw new Error("units");
    const bounced = (instanceId: string, owner: "p1" | "p2"): GameEvent => ({ type: "bounced", instanceId, defId: instanceId, owner });
    const defId = cardOf("vortex");
    const events: GameEvent[] = [
      { type: "cardPlayed", player: "p1", instanceId: "src", defId, costPaid: 4 },
      { type: "hookResolving", sourceId: "src", defId, controller: "p1", hook: "cry" },
      bounced("hidden", "p2"),
      bounced(named.instanceId, "p1"),
      bounced("hidden", "p2"),
    ];
    const entry = planEntries(events, view, false, createPlayTracker(), hasSetPiece).find((e) => e.wipe !== undefined);
    if (entry === undefined) throw new Error("no wipe");
    expect(entry.wipe?.sides).toEqual(["you", "opponent"]);
    // The named card, then the opponent's first two cards in lane order for its two hidden ones.
    expect(takenOf(entry, view)).toEqual([testid.card(named.instanceId), testid.card(first.instanceId), testid.card(second.instanceId)]);
  });

  it("R2800000 R2722001 a yielded entrance draws no takeover", () => {
    const view = fullBoardView();
    const [played] = planEntries([{ type: "cardPlayed", player: "p1", instanceId: "void", defId: "core-100", costPaid: 0 }], view, false);
    if (played === undefined) throw new Error("no entry");
    const entry: AnimationEntry = {
      ...played,
      entrance: { tier: "mythic", instanceId: "void", defId: "core-100", player: "p1", reveal: false, anchor: testid.zone("you", "units", 3) },
    };
    const kinds = (e: AnimationEntry): string[] => entranceCues(e, env(), e.durationMs).map((c) => c.kind);
    expect(kinds(entry)).toContain("takeover");
    const yielded = kinds({ ...entry, entrance: { ...entry.entrance!, yielded: true } });
    expect(yielded).not.toContain("takeover");
    // Its rays and its landing still play: only the board's takeover goes to the fold.
    expect(yielded).toContain("rays");
  });

  it("R2800000 R670 the Void lands with voidCollapse", () => {
    expect(entranceKeyOf("core-100")).toBe("voidCollapse");
    // R3332000: the entrance stands beside the set piece in the Void's table entry.
    expect(FX_CARD_TABLE.get("core-100")).toEqual({ kind: "setPiece", key: "voidFold", entrance: "voidCollapse", takeover: "void" });
    const view = fullBoardView();
    const summoned: GameEvent = { type: "summoned", player: "p1", instanceId: "void", defId: "core-100", row: "units", lane: 3 };
    const [entry] = planEntries([summoned], view, false);
    if (entry === undefined) throw new Error("no entry");
    const D = ANIMATIONS.summoned.durationMs;
    const expected = ENTRANCES.voidCollapse({ D, tgt: testid.zone("you", "units", 3), intensity: FX_INTENSITY_SCALE.normal });
    const cues = planFx(entry, view, env());
    for (const cue of expected) expect(cues).toContainEqual(cue);
  });

  it("R2800000 the effects off plan nothing", () => {
    const view = fullBoardView();
    const entry = wipeEntry(cardOf("nether"), unitsOf(view, "you"), SET_PIECES.nether.holdMs, view);
    expect(planSetPiece(entry, view, env(0))).toBeNull();
    expect(planFx(entry, view, env(0))).toEqual([]);
    // Reduce motion: an entry of no duration.
    expect(planSetPiece({ ...entry, durationMs: 0 }, view, env())).toBeNull();
  });
});
