// R2800000: a board clear's set piece sounds a bed of `sfx.ts` recipes across its hold (setPieceBeds.ts,
// #2785's sound hook), sent as its held entry starts, and its wipe's removals sound once, condensed, as
// the hold ends. Twisting Nether's cast keeps its own "alien" voice and "rumble" (card-audio.json5).
// R3338000: each Terminus card's bed opens with an organ chord of its own and ends in a Mythic's choir,
// and its signature run sounds once, condensed, after the hold.

import { CATALOG } from "@jackioh/cards";
import type { GameEvent, PlayerView, UnitView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { SET_PIECE_KEYS, hasSetPiece, wipeClaim, type SetPieceKey } from "../fx/setPieces.ts";
import type { FxBigMoments } from "../fx/settings.ts";
import { FX_STARSHIP_LAND_AT } from "../fx/constants.ts";
import { STARSHIP_CINEMATIC, starshipClaim, starshipFuseLandMs } from "../fx/starship.ts";
import {
  ANIMATIONS,
  STARSHIP_LAUNCH_HOLD_MS,
  STARSHIP_SET_PIECE,
  TAKEOVER_SET_PIECE,
  newEventsSince,
  planEntries,
  setPieceEntries,
  type AnimationEntry,
} from "../game/animations.ts";
import { lookupFromDefs } from "../game/catalog.ts";
import { createPlayTracker } from "../game/runs.ts";
import { baseView, dormantStarship, emptySide, fullBoardView, unit, withEvents } from "../test/fixtures.ts";
import { createSoundDirector } from "./director.ts";
import { SET_PIECE_BEDS, STARSHIP_LAUNCH_BED, setPieceBed } from "./setPieceBeds.ts";
import { SFX_IDS } from "./sfx.ts";
import type { CardHook, PlayableLineKind, SfxId, SoundSink } from "./types.ts";
import { cueCard } from "./useGameAudio.ts";

const NETHER = "core-088";
const lookup = lookupFromDefs(CATALOG);
const START: GameEvent[] = [{ type: "turnStarted", player: "p1", turn: 3 }];

type Sent =
  | { kind: "sfx"; id: SfxId; delayMs: number }
  | { kind: "voice"; defId: string; line: PlayableLineKind; delayMs: number }
  | { kind: "effect"; defId: string; hook: CardHook; delayMs: number };

function recorder(): SoundSink & { sent: Sent[] } {
  const sent: Sent[] = [];
  return {
    sent,
    playSfx(id, _params, delayMs) {
      sent.push({ kind: "sfx", id, delayMs: delayMs ?? 0 });
      return true;
    },
    playVoice(defId, line, delayMs) {
      sent.push({ kind: "voice", defId, line, delayMs: delayMs ?? 0 });
      return true;
    },
    playEffect(defId, hook, delayMs) {
      sent.push({ kind: "effect", defId, hook, delayMs: delayMs ?? 0 });
      return true;
    },
  };
}

const unitsOf = (view: PlayerView): UnitView[] => [...view.you.units, ...view.opponent.units].flatMap((u) => (u === null ? [] : [u]));

/** p1's Twisting Nether over the whole board, voiced by a director as Game's board voices it. */
function voiceNether(bigMoments: FxBigMoments = "full") {
  const view = fullBoardView();
  const events: GameEvent[] = [
    { type: "cardPlayed", player: "p1", instanceId: "nx", defId: NETHER, costPaid: 4 },
    { type: "hookResolving", sourceId: "nx", defId: NETHER, controller: "p1", hook: "cry" },
    ...unitsOf(view).map((u): GameEvent => ({
      type: "destroyed",
      instanceId: u.instanceId,
      defId: u.defId,
      owner: u.owner,
      controller: u.controller,
      attack: u.attack,
      maxHealth: u.maxHealth,
      killerId: null,
    })),
    { type: "cardResolved", player: "p1", instanceId: "nx", defId: NETHER, permanent: false, costPaid: 4 },
  ];
  const before = withEvents(view, START);
  const after = withEvents(view, [...START, ...events]);
  const sink = recorder();
  const director = createSoundDirector(sink, undefined, (defId) => cueCard(lookup, defId), undefined, () => 0);
  director.onView(before);
  director.onView(after);
  const planned = planEntries(newEventsSince(before.events, after.events), before, false, createPlayTracker(), hasSetPiece);
  const entries = setPieceEntries(planned, wipeClaim, { reduced: false, bigMoments, speed: 1 });
  const byEntry = new Map<AnimationEntry, Sent[]>();
  for (const entry of entries) {
    const from = sink.sent.length;
    director.onEntryStart(entry);
    byEntry.set(entry, sink.sent.slice(from));
  }
  director.onIdle();
  const wipe = entries.find((entry) => entry.wipe !== undefined);
  if (wipe === undefined) throw new Error("no wipe");
  return { wipe, wipeSent: byEntry.get(wipe) ?? [], sent: sink.sent, deaths: unitsOf(view).length };
}

describe("R2800000 a set piece's sound bed", () => {
  it("R2800000 the bed plays across the hold", () => {
    for (const key of SET_PIECE_KEYS) {
      const holdMs = 2_000;
      const bed = setPieceBed({ setPiece: { key, holdMs } });
      expect(bed.map((cue) => (cue.kind === "sfx" ? cue.id : cue.kind)), key).toEqual(SET_PIECE_BEDS[key].map((step) => step.id));
      expect(bed.map((cue) => cue.delayMs), key).toEqual(SET_PIECE_BEDS[key].map((step) => Math.round(step.at * holdMs)));
      // Every step is an existing recipe, inside the hold.
      for (const step of SET_PIECE_BEDS[key]) {
        expect(SFX_IDS, key).toContain(step.id);
        expect(step.at, key).toBeGreaterThanOrEqual(0);
        expect(step.at, key).toBeLessThanOrEqual(1);
      }
    }
    expect(setPieceBed({ setPiece: { key: "voidFold", holdMs: 1_000 } }).at(-1)).toEqual({ kind: "sfx", id: "entrance", params: { mythic: true }, delayMs: 1_000 });
    // The takeover has its intro already, and an entry with no set piece has no bed.
    expect(setPieceBed({ setPiece: { key: TAKEOVER_SET_PIECE, holdMs: 1_000 } })).toEqual([]);
    expect(setPieceBed({})).toEqual([]);
  });

  it("R2800000 a wipe is voiced condensed after the hold", () => {
    const { wipe, wipeSent, deaths } = voiceNether();
    const holdMs = wipe.setPiece?.holdMs ?? 0;
    expect(holdMs).toBeGreaterThan(0);
    const bed = setPieceBed(wipe).map((cue) => ({ kind: "sfx", id: cue.kind === "sfx" ? cue.id : "", delayMs: cue.delayMs }));
    expect(wipeSent.slice(0, bed.length)).toEqual(bed);
    const removals = wipeSent.slice(bed.length);
    const sfx = removals.flatMap((s) => (s.kind === "sfx" ? [s.id] : []));
    // One of each sound for the whole board, not one a card, and the most important death line alone.
    expect(sfx.length).toBeGreaterThan(0);
    expect(new Set(sfx).size).toBe(sfx.length);
    expect(sfx.filter((id) => id === "death").length).toBeLessThan(deaths);
    expect(removals.filter((s) => s.kind === "voice").length).toBeLessThanOrEqual(1);
    // All of it as the hold ends, when the vortex swallows the board.
    for (const s of removals) expect(s.delayMs, `${s.kind}`).toBeGreaterThanOrEqual(holdMs);

    // Big moments "off": no hold, no bed, and the same condensed burst at once.
    const off = voiceNether("off");
    expect(off.wipe.setPiece).toBeUndefined();
    const offSfx = off.wipeSent.flatMap((s) => (s.kind === "sfx" ? [s.id] : []));
    expect(offSfx).toEqual(sfx);
    expect(Math.min(...off.wipeSent.map((s) => s.delayMs))).toBe(0);
  });

  it("R2800000 Nether's cast keeps alien and rumble", () => {
    const { sent } = voiceNether();
    expect(sent).toContainEqual(expect.objectContaining({ kind: "voice", defId: NETHER, line: "cast" }));
    expect(sent).toContainEqual(expect.objectContaining({ kind: "effect", defId: NETHER, hook: "cast" }));
  });
});

describe("R2801000 a Starship launch's sound bed", () => {
  const HULL = "stinkyscape-t-dormant-starship";
  const PIECES = ["stinkyscape-140", "stinkyscape-141"];

  /** p1's Starship launching, or two pieces fusing into it, voiced by a director as Game's board voices it. */
  function voiceStarship(events: GameEvent[], bigMoments: FxBigMoments = "full") {
    const view = baseView({ you: emptySide("p1", { units: [null, null, dormantStarship("p1", PIECES, { instanceId: "ship" }), null, null] }) });
    const before = withEvents(view, START);
    const after = withEvents(view, [...START, ...events]);
    const sink = recorder();
    const director = createSoundDirector(sink, undefined, (defId) => cueCard(lookup, defId), undefined, () => 0);
    director.onView(before);
    director.onView(after);
    const planned = planEntries(newEventsSince(before.events, after.events), before, false, createPlayTracker());
    const entries = setPieceEntries(planned, starshipClaim, { reduced: false, bigMoments, speed: 1 });
    const byEntry = new Map<AnimationEntry, Sent[]>();
    for (const entry of entries) {
      const from = sink.sent.length;
      director.onEntryStart(entry);
      byEntry.set(entry, sink.sent.slice(from));
    }
    director.onIdle();
    const starship = entries.find((entry) => entry.launch !== undefined || entry.fuse !== undefined);
    if (starship === undefined) throw new Error("no Starship entry");
    return { entry: starship, sent: byEntry.get(starship) ?? [] };
  }

  const LAUNCH: GameEvent[] = [{ type: "starshipLaunched", player: "p1", instanceId: "ship", defId: HULL, name: "Ember of Departure", pieces: PIECES }];

  it("R3334001 bed: ignition, roar, landing", () => {
    const holdMs = STARSHIP_LAUNCH_HOLD_MS;
    const motionMs = ANIMATIONS.starshipLaunched.durationMs;
    const bed = setPieceBed({ setPiece: { key: STARSHIP_SET_PIECE, holdMs }, durationMs: holdMs + motionMs });
    // The engines ignite as the cinematic lights them, the roar as the ship climbs out, and, after the
    // hold, the landing's heavy thud.
    expect(bed).toEqual([
      { kind: "sfx", id: "starshipIgnite", delayMs: Math.round(STARSHIP_CINEMATIC.enginesFrom * holdMs) },
      { kind: "sfx", id: "starshipRoar", delayMs: Math.round(STARSHIP_CINEMATIC.climbAt * holdMs) },
      { kind: "sfx", id: "summon", params: { slamTier: "huge" }, delayMs: Math.round(holdMs + FX_STARSHIP_LAND_AT * motionMs) },
    ]);
    for (const step of STARSHIP_LAUNCH_BED) expect(SFX_IDS).toContain(step.id);
    // The countdown is gone, and with it its ticks; the ignition and the roar sound inside the hold.
    expect(STARSHIP_LAUNCH_BED.map((step) => step.id)).not.toContain("clockTick");
    const held = STARSHIP_LAUNCH_BED.filter((step) => step.after !== true).map((step) => step.at);
    expect(held).toEqual([...held].sort((a, b) => a - b));
    for (const at of held) expect(at).toBeLessThanOrEqual(1);

    // On the board: the bed as the held launch starts, then its own roar (`entrance`) as the hold ends.
    const { entry, sent } = voiceStarship(LAUNCH);
    expect(entry.setPiece).toEqual({ key: STARSHIP_SET_PIECE, holdMs });
    const asSent = setPieceBed(entry).map((cue) => ({ kind: "sfx", id: cue.kind === "sfx" ? cue.id : "", delayMs: cue.delayMs }));
    expect(sent.slice(0, asSent.length)).toEqual(asSent);
    expect(sent.slice(asSent.length)).toEqual([{ kind: "sfx", id: "entrance", delayMs: holdMs }]);

    // Big moments "off": no hold, no bed, and its roar at once.
    const off = voiceStarship(LAUNCH, "off");
    expect(off.entry.setPiece).toBeUndefined();
    expect(off.sent).toEqual([{ kind: "sfx", id: "entrance", delayMs: 0 }]);
  });

  it("R2801000 R3334000 a fuse run sounds its rows once and docks each piece", () => {
    const fusion = (piece: string, pieces: number): GameEvent[] => [
      { type: "fused", instanceIds: ["ship", `${piece}-copy`], resultInstanceId: "ship", defId: HULL },
      { type: "starshipFused", player: "p1", instanceId: "ship", pieceId: piece, defId: PIECES[0] ?? HULL, pieces },
    ];
    const { entry, sent } = voiceStarship([...fusion("a", 3), ...fusion("b", 4), ...fusion("c", 5)]);
    expect(entry.fuse?.pieces).toHaveLength(3);
    // Each piece docks with a clank of its own as it lands (starshipSounds.ts) ...
    const docks = sent.filter((s) => s.kind === "sfx" && s.id === "starshipDock");
    expect(docks.map((s) => s.delayMs)).toEqual([0, 1, 2].map((k) => starshipFuseLandMs(k, 3, entry.durationMs)));
    // ... and the run's rows sound once, condensed: one of each, the starshipFused row's buff among them.
    const sfx = sent.flatMap((s) => (s.kind === "sfx" && s.id !== "starshipDock" ? [s.id] : []));
    expect(sfx.length).toBeGreaterThan(0);
    expect(new Set(sfx).size).toBe(sfx.length);
    expect(sfx).toContain("buff");
  });
});

describe("R3338000 a Terminus set piece's sting", () => {
  /** Ceaseless Void's fold, then the Terminus set pieces in SET_PIECE_KEYS order. */
  const TERMINUS: readonly SetPieceKey[] = [
    "voidFold",
    "terminusRift",
    "farewell",
    "lastExam",
    "eventHorizon",
    "lastWord",
    "finalDestination",
    "bigCrunch",
    "diceOfDoom",
  ];
  const MYTHIC_CHOIR = { id: "entrance", at: 1, params: { mythic: true } };

  it("R3338000 each Terminus bed opens with its own organ", () => {
    expect(TERMINUS).toEqual(SET_PIECE_KEYS.filter((key) => key !== "nether" && key !== "vortex"));
    const amounts = TERMINUS.map((key) => {
      const first = SET_PIECE_BEDS[key][0];
      expect(first?.id, key).toBe("organ");
      expect(first?.at, key).toBe(0);
      return first?.params?.amount;
    });
    // 1 to 9, one key each: the fold's 1, then each in SET_PIECE_KEYS order.
    expect(amounts).toEqual(TERMINUS.map((_, i) => i + 1));
    expect(new Set(amounts).size).toBe(TERMINUS.length);
    for (const key of TERMINUS) {
      const bed = SET_PIECE_BEDS[key];
      // Each ends in the Mythic's choir (the fold's last step, as ever).
      expect(bed.at(-1), key).toEqual(MYTHIC_CHOIR);
      // Two or three recipes of its moment between, every step inside the hold and in order.
      const between = new Set(bed.slice(1, -1).map((step) => step.id));
      expect(between.size, key).toBeGreaterThanOrEqual(2);
      expect(between.size, key).toBeLessThanOrEqual(3);
      expect([...between], key).not.toContain("organ");
      const ats = bed.map((step) => step.at);
      expect(ats, key).toEqual([...ats].sort((a, b) => a - b));
      for (const step of bed) expect(step.after, key).toBeUndefined();
    }
    // No other bed plays the organ.
    for (const key of SET_PIECE_KEYS.filter((k) => !TERMINUS.includes(k))) expect(SET_PIECE_BEDS[key].map((step) => step.id), key).not.toContain("organ");
    expect(STARSHIP_LAUNCH_BED.map((step) => step.id)).not.toContain("organ");

    // As sent for a held entry: its organ at once, the choir as the hold ends.
    const holdMs = 2_400;
    TERMINUS.forEach((key, i) => {
      const bed = setPieceBed({ setPiece: { key, holdMs } });
      expect(bed[0], key).toEqual({ kind: "sfx", id: "organ", params: { amount: i + 1 }, delayMs: 0 });
      expect(bed.at(-1), key).toEqual({ kind: "sfx", id: "entrance", params: { mythic: true }, delayMs: holdMs });
    });
  });

  it("R3338000 a signature run sends its organ at once and is voiced once, condensed, after its hold", () => {
    // KY's Last Exam's run: its damage to both of the other side's Units, held as its set piece.
    const view = baseView({
      you: emptySide("p1"),
      opponent: emptySide("p2", { units: [unit("p2", { instanceId: "e1" }), unit("p2", { instanceId: "e2" }), null, null, null] }),
    });
    const events: GameEvent[] = [
      { type: "damage", sourceId: null, targetId: "e1", amount: 2, combat: false },
      { type: "damage", sourceId: null, targetId: "e2", amount: 2, combat: false },
    ];
    const before = withEvents(view, START);
    const sink = recorder();
    const director = createSoundDirector(sink, undefined, (defId) => cueCard(lookup, defId), undefined, () => 0);
    director.onView(before);
    director.onView(withEvents(view, [...START, ...events]));
    const holdMs = 2_400;
    const entry: AnimationEntry = {
      events,
      type: "damage",
      durationMs: holdMs + 1_200,
      frames: new Map(),
      view: before,
      setPiece: { key: "lastExam", holdMs },
      signature: { by: "stinkyscape-004-2", sourceId: "exam", player: "p1" },
    };
    director.onEntryStart(entry);

    expect(sink.sent[0]).toEqual({ kind: "sfx", id: "organ", delayMs: 0 });
    const bed = setPieceBed(entry).map((cue) => ({ kind: "sfx", id: cue.kind === "sfx" ? cue.id : "", delayMs: cue.delayMs }));
    expect(sink.sent.slice(0, bed.length)).toEqual(bed);
    // The run's own sounds: one of each, not one a hit, all as the hold ends.
    const run = sink.sent.slice(bed.length);
    const sfx = run.flatMap((s) => (s.kind === "sfx" ? [s.id] : []));
    expect(sfx.length).toBeGreaterThan(0);
    expect(new Set(sfx).size).toBe(sfx.length);
    for (const s of run) expect(s.delayMs, `${s.kind}`).toBeGreaterThanOrEqual(holdMs);
    expect(director.owedCount()).toBe(0);
  });
});
