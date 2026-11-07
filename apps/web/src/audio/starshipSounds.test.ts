// R3334000–R3334003 (issue #3334): each of the Starship's moments sounds its own (starshipSounds.ts), at
// the beat the effects layer draws it (fx/starship.ts), sent by the director as its entry starts: a clank
// for each piece a fuse run docks, its hum rising with the ship; one sound for each kind of weapon a
// broadside fires, or an unarmed ship's ram; and the wreck's groan, in place of the death sting. A
// Starship the seat cannot place, a wipe's and an entry of no duration sound none of it.

import { CATALOG } from "@jackioh/cards";
import type { GameEvent, PlayerView, UnitView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { FX_LUNGE_CONTACT_AT } from "../fx/constants.ts";
import { STARSHIP_WRECK, broadsideFireMs, starshipBroadside, starshipFuseLandMs, type BroadsideFire } from "../fx/starship.ts";
import { newEventsSince, planEntries, type AnimationEntry } from "../game/animations.ts";
import { lookupFromDefs } from "../game/catalog.ts";
import { createPlayTracker } from "../game/runs.ts";
import { baseView, dormantStarship, emptySide, unit, withEvents } from "../test/fixtures.ts";
import { HIDDEN_DEF_ID } from "./constants.ts";
import { cuesFor, type CueContext } from "./cues.ts";
import { createSoundDirector } from "./director.ts";
import { SFX } from "./sfx.ts";
import { BROADSIDE_SOUNDS, starshipBed } from "./starshipSounds.ts";
import { FakeAudio } from "./test/fakeAudio.ts";
import type { CardHook, PlayableLineKind, SfxId, SfxParams, SoundCue, SoundSink } from "./types.ts";
import { cueCard } from "./useGameAudio.ts";
import { CARD_AUDIO } from "./voiceData.ts";

const lookup = lookupFromDefs(CATALOG);
const START: GameEvent[] = [{ type: "turnStarted", player: "p1", turn: 3 }];
const HULL = "stinkyscape-t-dormant-starship";
// Starship Pieces by the module they mount (cards/art/starshipModules.ts STARSHIP_MODULE_IDS).
const BEAM = "stinkyscape-050";
const REPEATER = "stinkyscape-051";
const CANNON = "stinkyscape-052";
const FLAK = "stinkyscape-053";
const ROCKETS = "stinkyscape-140";
const BIG_BEAM = "stinkyscape-054";
const ENGINE = "stinkyscape-090";

type Sent =
  | { kind: "sfx"; id: SfxId; params?: SfxParams; delayMs: number }
  | { kind: "voice"; defId: string; line: PlayableLineKind; delayMs: number }
  | { kind: "effect"; defId: string; hook: CardHook; delayMs: number };

function recorder(): SoundSink & { sent: Sent[] } {
  const sent: Sent[] = [];
  return {
    sent,
    playSfx(id, params, delayMs) {
      sent.push(params === undefined ? { kind: "sfx", id, delayMs: delayMs ?? 0 } : { kind: "sfx", id, params, delayMs: delayMs ?? 0 });
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

/** p1's launched Starship holding `pieces`, in the order they fused. */
function launched(pieces: readonly string[]): UnitView {
  return unit("p1", { instanceId: "ship", defId: HULL, starship: { pieces: pieces.map((defId) => ({ defId })), name: "The Ninth Life", launched: 1 } });
}

/** p1's `ship` in lane 3, facing p2's `e1`. */
function board(ship: UnitView): PlayerView {
  return baseView({
    you: emptySide("p1", { units: [null, null, ship, null, null] }),
    opponent: emptySide("p2", { units: [null, null, unit("p2", { instanceId: "e1" }), null, null] }),
  });
}

/** `events` on `view`, voiced by a director as Game's board voices it: each entry and what it sent. */
function voiced(view: PlayerView, events: GameEvent[]): { entry: AnimationEntry; sent: Sent[] }[] {
  const before = withEvents(view, START);
  const after = withEvents(view, [...START, ...events]);
  const sink = recorder();
  const director = createSoundDirector(sink, CARD_AUDIO, (defId) => cueCard(lookup, defId), undefined, () => 0);
  director.onView(before);
  director.onView(after);
  const out = planEntries(newEventsSince(before.events, after.events), before, false, createPlayTracker()).map((entry) => {
    const from = sink.sent.length;
    director.onEntryStart(entry);
    return { entry, sent: sink.sent.slice(from) };
  });
  director.onIdle();
  expect(director.owedCount()).toBe(0);
  return out;
}

/** The one entry `events` plays as on `view`, with what the director sent for it. */
function voicedOne(view: PlayerView, events: GameEvent[], pick: (entry: AnimationEntry) => boolean): { entry: AnimationEntry; sent: Sent[] } {
  const found = voiced(view, events).find(({ entry }) => pick(entry));
  if (found === undefined) throw new Error("no such entry");
  return found;
}

const sfxIds = (sent: readonly Sent[]): SfxId[] => sent.flatMap((s) => (s.kind === "sfx" ? [s.id] : []));
const asSent = (cues: readonly SoundCue[]): Sent[] =>
  cues.flatMap((cue): Sent[] => (cue.kind === "sfx" ? [cue.params === undefined ? { kind: "sfx", id: cue.id, delayMs: cue.delayMs } : { kind: "sfx", id: cue.id, params: cue.params, delayMs: cue.delayMs }] : []));

/** A row's cues against `view`, as the director asks them. */
const rowCues = (event: GameEvent, view: PlayerView): readonly SoundCue[] => cuesFor(event, { view, lines: CARD_AUDIO, manaBefore: () => 0 } satisfies CueContext);
const rowSfx = (event: GameEvent, view: PlayerView): SfxId[] => rowCues(event, view).flatMap((cue) => (cue.kind === "sfx" ? [cue.id] : []));

const attack = (attackerId: string): GameEvent => ({ type: "attackDeclared", attackerId, targetId: "e1", forced: false });
const destroyed = (instanceId: string, defId = instanceId === HIDDEN_DEF_ID ? HIDDEN_DEF_ID : HULL): GameEvent => ({
  type: "destroyed",
  instanceId,
  defId,
  owner: "p1",
  controller: "p1",
  attack: 0,
  maxHealth: 8,
  killerId: null,
});

describe("R3334000 a docking piece's clank", () => {
  it("R3334000 a clank per piece, pitch rising", () => {
    const view = board(dormantStarship("p1", [ENGINE], { instanceId: "ship" }));
    const fusion = (piece: string, defId: string, pieces: number): GameEvent[] => [
      { type: "fused", instanceIds: ["ship", `${piece}-copy`], resultInstanceId: "ship", defId: HULL },
      { type: "starshipFused", player: "p1", instanceId: "ship", pieceId: piece, defId, pieces },
    ];
    const { entry, sent } = voicedOne(view, [...fusion("a", BEAM, 2), ...fusion("b", CANNON, 3), ...fusion("c", FLAK, 4)], (e) => e.fuse !== undefined);
    expect(entry.fuse?.pieces).toHaveLength(3);
    const D = entry.durationMs;
    const bed = starshipBed(entry, entry.view);
    // One clank for each piece as it docks, naming the pieces the ship then holds: 2, 3, then 4.
    expect(bed).toEqual([0, 1, 2].map((k) => ({ kind: "sfx", id: "starshipDock", params: { amount: k + 2 }, delayMs: starshipFuseLandMs(k, 3, D) })));
    const delays = bed.map((cue) => cue.delayMs);
    expect(delays).toEqual([...delays].sort((a, b) => a - b));
    expect(new Set(delays).size).toBe(3);
    for (const delay of delays) expect(delay).toBeLessThanOrEqual(D);
    // The director sends them as the run starts, beside its condensed rows (the starshipFused row's buff once).
    expect(sent.filter((s) => s.kind === "sfx" && s.id === "starshipDock")).toEqual(asSent(bed));
    expect(sfxIds(sent).filter((id) => id === "buff")).toEqual(["buff"]);

    // Each clank's hum is higher than the one before.
    const hums = bed.map((cue) => {
      const audio = new FakeAudio({ state: "running" });
      SFX.starshipDock.recipe(audio.context, audio.context.createGain(), 1, cue.kind === "sfx" ? (cue.params ?? {}) : {});
      return audio.nodes.find((n) => n.kind === "oscillator" && n.type === "triangle")?.param("frequency").settled() ?? 0;
    });
    for (let k = 1; k < hums.length; k += 1) expect(hums[k] ?? 0, `piece ${String(k + 1)}`).toBeGreaterThan(hums[k - 1] ?? Infinity);

    // A run the seat cannot place (R97's sentinel) docks nothing, nor an entry of no duration (Reduce Motion).
    const fuse = entry.fuse;
    if (fuse === undefined) throw new Error("no fuse run");
    expect(starshipBed({ ...entry, fuse: { ...fuse, anchor: null } }, entry.view)).toEqual([]);
    expect(starshipBed({ ...entry, durationMs: 0 }, entry.view)).toEqual([]);
  });
});

describe("R3334002 a broadside's weapons", () => {
  it("R3334002 one sound per kind; ram", () => {
    const view = board(launched([BEAM, REPEATER, CANNON, FLAK, ROCKETS, BIG_BEAM]));
    const guns = starshipBroadside(view, "ship") ?? [];
    const kinds: readonly BroadsideFire[] = ["beam", "repeater", "cannon", "flak", "launcher"];
    expect(new Set(guns.map((gun) => gun.weapon.fire))).toEqual(new Set(kinds));
    // Two beams, one sound: each kind sounds as its first gun fires.
    expect(guns.filter((gun) => gun.weapon.fire === "beam").length).toBeGreaterThan(1);
    const { entry, sent } = voicedOne(view, [attack("ship")], (e) => e.events[0]?.type === "attackDeclared");
    const D = entry.durationMs;
    const first = (fire: BroadsideFire): number => guns.findIndex((gun) => gun.weapon.fire === fire);
    const expected = [...kinds]
      .sort((a, b) => first(a) - first(b))
      .map((fire) => ({ kind: "sfx", id: BROADSIDE_SOUNDS[fire], delayMs: broadsideFireMs(first(fire), guns.length, D) }));
    const bed = starshipBed(entry, entry.view);
    expect(bed).toEqual(expected);
    expect(new Set(bed.map((cue) => (cue.kind === "sfx" ? cue.id : ""))).size).toBe(kinds.length);
    for (const cue of bed) expect(cue.delayMs).toBeLessThanOrEqual(D);
    // The broadside stands in for the swing: the row sounds nothing, and the director sends only the guns.
    expect(rowCues(attack("ship"), view)).toEqual([]);
    expect(sent.filter((s) => s.kind === "sfx")).toEqual(asSent(bed));
    expect(sfxIds(sent)).not.toContain("attack");

    // An unarmed Starship rams prow first, at the lunge's contact.
    const unarmed = board(launched([ENGINE]));
    expect(starshipBroadside(unarmed, "ship")).toEqual([]);
    const ram = voicedOne(unarmed, [attack("ship")], (e) => e.events[0]?.type === "attackDeclared");
    expect(starshipBed(ram.entry, ram.entry.view)).toEqual([{ kind: "sfx", id: "starshipRam", delayMs: Math.round(FX_LUNGE_CONTACT_AT * ram.entry.durationMs) }]);
    expect(sfxIds(ram.sent)).toEqual(["starshipRam"]);
    expect(rowCues(attack("ship"), unarmed)).toEqual([]);

    // Any other attacker swings as ever, with no broadside, and so does R97's sentinel.
    const plain = board(unit("p1", { instanceId: "ship" }));
    const swing = voicedOne(plain, [attack("ship")], (e) => e.events[0]?.type === "attackDeclared");
    expect(starshipBed(swing.entry, swing.entry.view)).toEqual([]);
    expect(sfxIds(swing.sent)).toEqual(["attack"]);
    expect(starshipBed({ ...entry, events: [attack(HIDDEN_DEF_ID)] }, entry.view)).toEqual([]);
    expect(rowSfx(attack(HIDDEN_DEF_ID), view)).toEqual(["attack"]);
    expect(starshipBed({ ...entry, durationMs: 0 }, entry.view)).toEqual([]);
  });
});

describe("R3334003 a wreck's groan", () => {
  it("R3334003 groan, no death sting", () => {
    for (const ship of [launched([BEAM, CANNON]), dormantStarship("p1", [ENGINE], { instanceId: "ship" })]) {
      const view = board(ship);
      const { entry, sent } = voicedOne(view, [destroyed("ship")], (e) => e.events.some((event) => event.type === "destroyed"));
      const bed = starshipBed(entry, entry.view);
      expect(bed).toEqual([{ kind: "sfx", id: "starshipWreck", delayMs: Math.round(STARSHIP_WRECK.breakAt * entry.durationMs) }]);
      // The groan stands in for the death sting; the hull's death line still speaks.
      expect(rowSfx(destroyed("ship"), view)).toEqual([]);
      expect(sfxIds(sent)).toEqual(["starshipWreck"]);
      expect(sent).toContainEqual(expect.objectContaining({ kind: "voice", defId: HULL, line: "death" }));

      // Inside a wipe its set piece swallows the ship: no groan.
      expect(starshipBed({ ...entry, wipe: { by: "core-088", sourceId: "nx", sides: ["you"], anchor: null } }, entry.view)).toEqual([]);
      expect(starshipBed({ ...entry, durationMs: 0 }, entry.view)).toEqual([]);
    }

    // Any other death keeps its sting, and R97's sentinel sounds as ever, with no groan.
    const plain = board(unit("p1", { instanceId: "ship", defId: "core-004" }));
    const death = voicedOne(plain, [destroyed("ship", "core-004")], (e) => e.events.some((event) => event.type === "destroyed"));
    expect(starshipBed(death.entry, death.entry.view)).toEqual([]);
    expect(sfxIds(death.sent)).toEqual(["death"]);
    const view = board(launched([BEAM]));
    const hidden = destroyed(HIDDEN_DEF_ID);
    expect(starshipBed({ events: [hidden], type: "destroyed", durationMs: 600, frames: new Map(), view }, view)).toEqual([]);
    expect(rowSfx(hidden, view)).toEqual(["death"]);
  });
});
