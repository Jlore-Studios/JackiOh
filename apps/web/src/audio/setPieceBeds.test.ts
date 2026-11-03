// R2800000: a board clear's set piece sounds a bed of `sfx.ts` recipes across its hold (setPieceBeds.ts,
// #2785's sound hook), sent as its held entry starts, and its wipe's removals sound once, condensed, as
// the hold ends. Twisting Nether's cast keeps its own "alien" voice and "rumble" (card-audio.json5).

import { CATALOG } from "@jackioh/cards";
import type { GameEvent, PlayerView, UnitView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { SET_PIECE_KEYS, hasSetPiece, wipeClaim } from "../fx/setPieces.ts";
import type { FxBigMoments } from "../fx/settings.ts";
import { TAKEOVER_SET_PIECE, newEventsSince, planEntries, setPieceEntries, type AnimationEntry } from "../game/animations.ts";
import { lookupFromDefs } from "../game/catalog.ts";
import { createPlayTracker } from "../game/runs.ts";
import { fullBoardView, withEvents } from "../test/fixtures.ts";
import { createSoundDirector } from "./director.ts";
import { SET_PIECE_BEDS, setPieceBed } from "./setPieceBeds.ts";
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
