// The cue table (docs/polish/2-sound.md, B17 to B23), and the proofs of R203 (what audio may
// reveal) and R204 (which moments speak) at the level of a single event.
//
// Every test builds its own CueContext from `baseView()` (viewer p1) and an inline voice table, so
// these rows are checked against the table's rules and not against the shipped lines' content.
// "Readable" is the design's word: the defId is not the sentinel and the table has an entry for it.

import { GAME_EVENT_TYPES, type GameEvent, type GameEventType, type PlayerId, type UnitView } from "@jackioh/shared";
import { describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";

import {
  BLEAT_DELAY_MS,
  BLOOD_BEAN_DEF_ID,
  CARD_EFFECT_DELAY_MS,
  CHAOS_REVEAL_MAX,
  DEATH_VOICE_DELAY_MS,
  GOLD_BURST_DELAY_MS,
  HIDDEN_DEF_ID,
  HINDER_DEF_ID,
  LANE_PAN_MAX,
  NEXT_REFRESH_MODIFIER_ID,
  OVERKILL_DELAY_MS,
  OVERKILL_MIN_EXCESS,
  SHEEP_DEF_IDS,
  STING_DELAY_MS,
  VOICE_DELAY_MS,
  VOICE_PRIORITY,
} from "./constants.ts";
import {
  SOUND_CUES,
  cuesFor,
  healthBefore,
  isOverkill,
  lanePan,
  timbreFor,
  type CueCard,
  type CueContext,
  type PlayFrame,
} from "./cues.ts";
import { SFX_IDS, SFX_TIMBRES } from "./sfx.ts";
import type { SfxId, SoundCue, CardAudioTable } from "./types.ts";
import { cueCard } from "./useGameAudio.ts";
import { themeFor } from "../cards/art/themes.ts";
import { armorTookHalf } from "../game/damageFeel.ts";
import { lookupFromDefs } from "../game/catalog.ts";
import { baseView, emptySide, unit } from "../test/fixtures.ts";

/* --------------------------------------------------------------------------------------------- *
 * Fixtures
 * --------------------------------------------------------------------------------------------- */

const LINES: CardAudioTable = {
  voices: {
    hustler: { say: "Rocko (English (US))", rate: 215, pbas: 50, pmod: 45, web: { pitch: 1, rate: 1.15 } },
    narrator: { say: "Eddy (English (UK))", rate: 185, pbas: 45, pmod: 35, web: { pitch: 1, rate: 1 } },
    crone: { say: "Grandma (English (US))", rate: 170, pbas: 50, pmod: 40, web: { pitch: 1.1, rate: 0.9 } },
    snob: { say: "Albert", rate: 170, pbas: 40, pmod: 30, web: { pitch: 0.8, rate: 0.9 } },
    sheep: { say: "Bahh", rate: 180, pbas: 55, pmod: 40, web: { pitch: 1.5, rate: 1 } },
  },
  effects: {},
  cards: {
    "core-004": { kind: "unit", play: { voice: "hustler", text: "Double or nothing, baby!" }, death: { voice: "hustler", text: "House always wins." } },
    "core-005": { kind: "spell", cast: { voice: "narrator", text: "Hoarding is self care." } },
    "core-006": { kind: "spell", cast: { voice: "crone", text: "Drink up, dearie." } },
    "core-018": { kind: "trap", cast: { voice: "crone", text: "Waste not, want toast." } },
    "core-096": { kind: "trap", cast: { voice: "snob", text: "Checkmate, puppet." } },
    "core-t-sheep": { kind: "unit", play: { voice: "sheep", text: "Baa?" }, death: { voice: "sheep", text: "Baa..." } },
  },
  emotes: {},
};

/** A unit (#4), a Spell (#5), a Field Spell (#6), a Field Trap (#18), a Trap (#96), a token. */
const UNIT = "core-004";
const SPELL = "core-005";
const FIELD_SPELL = "core-006";
const FIELD_TRAP = "core-018";
const TRAP = "core-096";
const TOKEN = "core-t-sheep";
const NOT_IN_TABLE = "core-999";

function ctx(over: Partial<CueContext> = {}): CueContext {
  return { view: baseView(), lines: LINES, manaBefore: () => 0, ...over };
}

/** A cue as one comparable string: "sfx:play@0", "voice:core-004/play@150", "effect:core-066/play@150". */
function said(cue: SoundCue): string {
  if (cue.kind === "sfx") return `sfx:${cue.id}@${String(cue.delayMs)}`;
  if (cue.kind === "effect") return `effect:${cue.defId}/${cue.hook}@${String(cue.delayMs)}`;
  return `voice:${cue.defId}/${cue.line}@${String(cue.delayMs)}`;
}

/** Every voice cue's line and priority, e.g. "core-004/death!2". */
function ranked(event: GameEvent, context: CueContext = ctx()): string[] {
  return cuesFor(event, context).flatMap((c) => (c.kind === "voice" ? [`${c.defId}/${c.line}!${String(c.priority)}`] : []));
}

/** The cues an event gives, order-free. */
function shape(event: GameEvent, context: CueContext = ctx()): string[] {
  return cuesFor(event, context).map(said).sort();
}

function voices(event: GameEvent, context: CueContext = ctx()): SoundCue[] {
  return cuesFor(event, context).filter((c) => c.kind === "voice");
}

function onlySfx(event: GameEvent, context: CueContext = ctx()): Extract<SoundCue, { kind: "sfx" }> {
  const cues = cuesFor(event, context);
  expect(cues, JSON.stringify(event)).toHaveLength(1);
  const [cue] = cues;
  if (cue === undefined || cue.kind !== "sfx") throw new Error(`expected one sfx cue for ${event.type}`);
  return cue;
}

const sfx = (id: SfxId, delayMs = 0): string => `sfx:${id}@${String(delayMs)}`;
const voice = (defId: string, line: string, delayMs: number): string => `voice:${defId}/${line}@${String(delayMs)}`;
const effect = (defId: string, hook: string, delayMs: number): string => `effect:${defId}/${hook}@${String(delayMs)}`;

const played = (defId: string, player: PlayerId = "p1", instanceId = "c1"): GameEvent => ({
  type: "cardPlayed",
  player,
  instanceId,
  defId,
  costPaid: 2,
});

const destroyed = (defId: string, owner: PlayerId = "p1", instanceId = "u1"): GameEvent => ({
  type: "destroyed",
  instanceId,
  defId,
  owner,
  controller: owner,
  attack: 3,
  maxHealth: 4,
  killerId: null,
});

const trapFired = (defId: string, controller: PlayerId = "p1"): GameEvent => ({
  type: "trapFired",
  instanceId: defId === HIDDEN_DEF_ID ? HIDDEN_DEF_ID : "b1",
  defId,
  controller,
  row: "backrow",
  lane: 3,
});

/** One event of every type, for the table-wide checks. */
const SAMPLES: { [K in GameEventType]: Extract<GameEvent, { type: K }> } = {
  cardPlayed: { type: "cardPlayed", player: "p1", instanceId: "c1", defId: UNIT, costPaid: 3 },
  cardResolved: { type: "cardResolved", player: "p1", instanceId: "c1", defId: UNIT, permanent: true, costPaid: 3 },
  summoned: { type: "summoned", player: "p1", instanceId: "c1", defId: UNIT, row: "units", lane: 2 },
  damage: { type: "damage", sourceId: "u1", targetId: "hero-p2", amount: 4, combat: true },
  healthLost: { type: "healthLost", player: "p1", amount: 3 },
  healed: { type: "healed", targetId: "hero-p1", amount: 2 },
  divineShieldLost: { type: "divineShieldLost", instanceId: "u6" },
  destroyed: { type: "destroyed", instanceId: "u1", defId: UNIT, owner: "p1", controller: "p1", attack: 2, maxHealth: 3, killerId: "u6" },
  enteredGraveyard: { type: "enteredGraveyard", instanceId: "u1", defId: UNIT, owner: "p1" },
  exiled: { type: "exiled", instanceId: "u2", defId: UNIT, owner: "p1" },
  bounced: { type: "bounced", instanceId: "u3", defId: UNIT, owner: "p1" },
  burned: { type: "burned", instanceId: "cX", defId: SPELL, owner: "p2" },
  fatigue: { type: "fatigue", player: "p1", count: 2, amount: 2 },
  libraryOverflow: { type: "libraryOverflow", player: "p2", instanceId: "c80", defId: SPELL, outcome: "notCreated" },
  discarded: { type: "discarded", instanceId: "c11", defId: SPELL, owner: "p1" },
  drawn: { type: "drawn", player: "p1", instanceId: "cY", defId: UNIT },
  addedToHand: { type: "addedToHand", player: "p2", instanceId: "cZ", defId: UNIT },
  shuffledIn: { type: "shuffledIn", player: "p1", instanceId: "cW", defId: UNIT, position: 3 },
  buffed: { type: "buffed", instanceId: "u2", attack: 1, health: 1 },
  keywordGranted: { type: "keywordGranted", instanceId: "u2", keyword: { kind: "Taunt" } },
  counterChanged: { type: "counterChanged", instanceId: "u2", counter: "plague", value: 3 },
  costChanged: { type: "costChanged", instanceId: "c11", cost: 0 },
  modifierChanged: { type: "modifierChanged", player: "p2", modifierId: "m4", added: true },
  radiantSet: { type: "radiantSet", instanceId: "u3", defId: UNIT, zone: { z: "field", player: "p1", row: "units", lane: 2 } },
  deradianted: { type: "deradianted", instanceId: "u3", defId: UNIT, zone: { z: "field", player: "p1", row: "units", lane: 2 } },
  transformed: { type: "transformed", instanceId: "u3", fromDefId: UNIT, toDefId: TOKEN, newInstanceId: "c90" },
  fused: { type: "fused", instanceIds: ["u1", "u2"], resultInstanceId: "c91", defId: UNIT },
  positionSwitched: { type: "positionSwitched", instanceId: "u3", position: "DEF" },
  controlChanged: { type: "controlChanged", instanceId: "u6", controller: "p1", row: "units", lane: 4 },
  rotated: { type: "rotated", direction: "left" },
  swapped: { type: "swapped", what: "health" },
  locked: { type: "locked", player: "p2", row: "backrow", lane: 1 },
  trapFired: { type: "trapFired", instanceId: "b5", defId: TRAP, controller: "p1", row: "backrow", lane: 3 },
  attackDeclared: { type: "attackDeclared", attackerId: "u1", targetId: "u6", forced: false },
  attackCancelled: { type: "attackCancelled", attackerId: "u1", targetId: "u6", byInstanceId: "b5" },
  manaChanged: { type: "manaChanged", player: "p1", current: 3, max: 4 },
  manaSpent: { type: "manaSpent", player: "p1", amount: 1, for: "activate" },
  turnStarted: { type: "turnStarted", player: "p1", turn: 3 },
  turnEnded: { type: "turnEnded", player: "p1", turn: 3, unspentMana: 2 },
  turnAutoEnded: { type: "turnAutoEnded", player: "p1", turn: 3 },
  promptOpened: { type: "promptOpened", player: "p1", choiceId: "ch1", kind: "discover" },
  promptAnswered: { type: "promptAnswered", player: "p1", choiceId: "ch1" },
  drawOffered: { type: "drawOffered", player: "p2" },
  drawAnswered: { type: "drawAnswered", player: "p1", accept: false },
  gameOver: { type: "gameOver", winner: "p1", reason: "hero-death" },
  // Patch v0.2.0 (docs/classic-sets.md B3, B5).
  cardAnnounced: { type: "cardAnnounced", player: "p1", instanceId: "c1", defId: SPELL, cardType: "Spell", costPaid: 1, targets: ["hero-p2"] },
  countered: { type: "countered", player: "p1", instanceId: "c1", defId: SPELL, byInstanceId: "b5", to: "graveyard" },
  stolen: { type: "stolen", instanceId: "c1", defId: SPELL, from: "p2", to: "p1", zone: "hand" },
  unlocked: { type: "unlocked", player: "p2", row: "backrow", lane: 1 },
  activated: { type: "activated", player: "p1", instanceId: "u1", defId: UNIT, ability: "activate" },
  animated: { type: "animated", player: "p1", instanceId: "b5", defId: TRAP, backrowLane: 3, unitLane: 3 },
  deanimated: { type: "deanimated", player: "p1", instanceId: "b5", defId: TRAP, unitLane: 3, backrowLane: 3 },
  crumbled: { type: "crumbled", instanceId: "u2", defId: UNIT, owner: "p1", zone: "field" },
  degraded: { type: "degraded", instanceId: "u2", defId: UNIT, change: { kind: "cost", delta: 1 } },
  upgraded: { type: "upgraded", instanceId: "u2", defId: UNIT, change: { kind: "stats", attack: 2, health: 2 } },
  numberChanged: { type: "numberChanged", instanceId: "u2", defId: UNIT, key: "attack", value: 3 },
  redirected: { type: "redirected", what: "damage", fromId: "hero-p1", toId: "hero-p2", byInstanceId: "b5" },
  healthSet: { type: "healthSet", player: "p2", health: 13, sourceId: "c1" },
  questProgressed: { type: "questProgressed", player: "p1", instanceId: "b5", quest: "1", progress: 1, goal: 2 },
  questCompleted: { type: "questCompleted", player: "p1", instanceId: "b5", quest: "1" },
  rolledBack: { type: "rolledBack", player: "p1", turnsAgo: 2, sides: ["p1", "p2"] },
  chaosRolled: { type: "chaosRolled", player: "p1", instanceId: "c1", defId: SPELL, effects: ["Destroy all enemy permanents"] },
  flickered: { type: "flickered", player: "p1", instanceId: "u2", defId: UNIT, row: "units", lane: 2 },
  drawLimited: { type: "drawLimited", player: "p2" },
  turnCutShort: { type: "turnCutShort", player: "p2", byInstanceId: "b5" },
  marked: { type: "marked", instanceId: "u6", mark: "steal", color: "purple", added: true },
  glitched: { type: "glitched", player: "p1", outcome: "swap" },
  translated: { type: "translated", instanceId: "u6" },
  // Patch v0.3.X (MN05).
  damageAbsorbed: { type: "damageAbsorbed", sourceId: "u1", targetId: "u6", absorbed: 2, combat: true },
};

/** The design's sfx column, row by row (null is an explicit silence). */
const HEADLINE: Record<GameEventType, SfxId | null> = {
  cardPlayed: "play",
  cardResolved: null,
  summoned: "summon",
  damage: "impact",
  healthLost: "drain",
  healed: "heal",
  divineShieldLost: "shieldShatter",
  destroyed: "death",
  enteredGraveyard: null,
  exiled: "poof",
  bounced: "whoosh",
  burned: "burn",
  fatigue: "fatigue",
  libraryOverflow: "refuse",
  discarded: "draw",
  drawn: "draw",
  addedToHand: "draw",
  shuffledIn: "whoosh",
  buffed: "buff",
  keywordGranted: "buff",
  counterChanged: "uiClick",
  costChanged: null,
  modifierChanged: "notify",
  radiantSet: "radiant",
  deradianted: "cancel",
  manaSpent: null,
  transformed: "poof",
  fused: "fuse",
  positionSwitched: "whoosh",
  controlChanged: "whoosh",
  rotated: "whoosh",
  swapped: "whoosh",
  locked: "lock",
  trapFired: "trapSting",
  attackDeclared: "attack",
  attackCancelled: "cancel",
  manaChanged: "mana",
  turnStarted: "turnStart",
  turnEnded: null,
  turnAutoEnded: "notify",
  promptOpened: "notify",
  promptAnswered: null,
  drawOffered: "notify",
  drawAnswered: "cancel",
  gameOver: "victory",
  // Patch v0.2.0.
  cardAnnounced: null,
  countered: "counterspell",
  stolen: "steal",
  unlocked: "unlock",
  activated: "spell",
  animated: "summon",
  deanimated: "whoosh",
  crumbled: "crumble",
  degraded: "degrade",
  upgraded: "upgrade",
  numberChanged: "uiClick",
  redirected: "whoosh",
  healthSet: "drain",
  questProgressed: "uiClick",
  questCompleted: "radiant",
  rolledBack: "whoosh",
  chaosRolled: "chaosRoll",
  flickered: "poof",
  drawLimited: "cancel",
  turnCutShort: "notify",
  marked: "brand",
  glitched: "whoosh",
  translated: null,
  // Patch v0.3.X (MN05).
  damageAbsorbed: "armorRing",
};

/** Rows that return exactly their headline sound, whatever the payload (summoned: B56, below). */
const UNCONDITIONAL: readonly GameEventType[] = [
  "divineShieldLost",
  "exiled",
  "bounced",
  "burned",
  "fatigue",
  "libraryOverflow",
  "discarded",
  "drawn",
  "addedToHand",
  "shuffledIn",
  "keywordGranted",
  "counterChanged",
  "radiantSet",
  "deradianted",
  "fused",
  "positionSwitched",
  "controlChanged",
  "rotated",
  "swapped",
  "locked",
  "attackDeclared",
  "attackCancelled",
  "turnAutoEnded",
  "countered",
  "stolen",
  "unlocked",
  "deanimated",
  "crumbled",
  "degraded",
  "upgraded",
  "numberChanged",
  "redirected",
  "questProgressed",
  "rolledBack",
  "chaosRolled",
  "flickered",
  "drawLimited",
  "marked",
  "damageAbsorbed",
];

/* --------------------------------------------------------------------------------------------- *
 * B17: a total table
 * --------------------------------------------------------------------------------------------- */

describe("B17 SOUND_CUES is total over GameEvent types", () => {
  it("B17 has exactly one row per GAME_EVENT_TYPES member, and no other", () => {
    expect(Object.keys(SOUND_CUES).sort()).toEqual([...GAME_EVENT_TYPES].sort());
  });

  it("B17 every silent row says why, and only a silent row carries a reason", () => {
    const wrong: string[] = [];
    for (const type of GAME_EVENT_TYPES) {
      const row = SOUND_CUES[type];
      const reason = row.silentBecause;
      const hasReason = typeof reason === "string" && reason.trim().length > 0;
      if (row.sfx === null && !hasReason) wrong.push(`${type} is silent with no silentBecause`);
      if (row.sfx !== null && reason !== undefined) wrong.push(`${type} names ${row.sfx} and also a silentBecause`);
    }
    expect(wrong).toEqual([]);
  });

  it("B17 each row's headline sound is the design's", () => {
    const actual = Object.fromEntries(GAME_EVENT_TYPES.map((t) => [t, SOUND_CUES[t].sfx]));
    expect(actual).toEqual(HEADLINE);
  });

  it("B17 every headline, and every sfx a row returns for the samples, is an SfxId", () => {
    const ids = new Set<string>(SFX_IDS);
    const variants: GameEvent[] = [
      ...GAME_EVENT_TYPES.map((t) => SAMPLES[t]),
      played(SPELL),
      played(TRAP),
      played(HIDDEN_DEF_ID, "p2", HIDDEN_DEF_ID),
      destroyed(TOKEN),
      trapFired(HIDDEN_DEF_ID, "p2"),
      { type: "buffed", instanceId: "u2", attack: -3, health: 0 },
      { type: "gameOver", winner: "p2", reason: "concede" },
      { type: "gameOver", winner: "draw", reason: "draw-accepted" },
      { type: "turnStarted", player: "p2", turn: 4 },
      { type: "drawOffered", player: "p2" },
    ];
    const outside: string[] = [];
    for (const type of GAME_EVENT_TYPES) {
      const headline = SOUND_CUES[type].sfx;
      if (headline !== null && !ids.has(headline)) outside.push(`${type} headline ${headline}`);
    }
    for (const event of variants) {
      for (const cue of cuesFor(event, ctx())) {
        if (cue.kind === "sfx" && !ids.has(cue.id)) outside.push(`${event.type} → ${cue.id}`);
        if (cue.kind === "voice" && !["play", "death", "cast"].includes(cue.line)) outside.push(`${event.type} → line ${cue.line}`);
        if (!Number.isFinite(cue.delayMs) || cue.delayMs < 0) outside.push(`${event.type} → delay ${String(cue.delayMs)}`);
      }
    }
    expect(outside).toEqual([]);
  });

  it("B17 a silent row returns no cues", () => {
    for (const type of GAME_EVENT_TYPES.filter((t) => HEADLINE[t] === null)) {
      expect(cuesFor(SAMPLES[type], ctx()), type).toEqual([]);
    }
  });

  it("B17 each unconditional row returns exactly its headline sound, at delay 0", () => {
    for (const type of UNCONDITIONAL) {
      expect(shape(SAMPLES[type]), type).toEqual([sfx(HEADLINE[type] ?? "notify")]);
    }
  });

  it("B17 cuesFor answers through the event's own row", () => {
    for (const type of GAME_EVENT_TYPES) {
      const event = SAMPLES[type];
      const row = SOUND_CUES[type] as unknown as { cues: (e: GameEvent, c: CueContext) => readonly SoundCue[] };
      expect(cuesFor(event, ctx()).map(said), type).toEqual(row.cues(event, ctx()).map(said));
    }
  });

  it("B17 modifierChanged notifies when a modifier is added and stays silent when one is removed", () => {
    expect(shape({ type: "modifierChanged", player: "p1", modifierId: "m1", added: true })).toEqual([sfx("notify")]);
    expect(shape({ type: "modifierChanged", player: "p2", modifierId: "m1", added: false })).toEqual([]);
  });
});

/* --------------------------------------------------------------------------------------------- *
 * B18 and B19: which moments speak (R204)
 * --------------------------------------------------------------------------------------------- */

describe("R319: the overflows' sounds", () => {
  it("R319 a fatigue draw knocks, a full library refuses, a full hand burns: three sounds of their own", () => {
    const own = new Set([SOUND_CUES.fatigue.sfx, SOUND_CUES.libraryOverflow.sfx, SOUND_CUES.burned.sfx]);
    expect([...own].sort()).toEqual(["burn", "fatigue", "refuse"]);
    // Their own: no other row's headline is a knock or a refusal.
    const others = GAME_EVENT_TYPES.filter((t) => t !== "fatigue" && t !== "libraryOverflow").map((t) => SOUND_CUES[t].sfx);
    expect(others).not.toContain("fatigue");
    expect(others).not.toContain("refuse");
  });

  it("R319 each plays exactly its sound on either seat, for any count and outcome", () => {
    for (const player of ["p1", "p2"] as const) {
      for (const count of [1, 9]) {
        expect(shape({ type: "fatigue", player, count, amount: count })).toEqual([sfx("fatigue")]);
      }
      for (const outcome of ["notCreated", "graveyard", "ceased"] as const) {
        expect(shape({ type: "libraryOverflow", player, instanceId: "c80", defId: SPELL, outcome })).toEqual([sfx("refuse")]);
      }
      expect(shape({ type: "burned", instanceId: "cX", defId: UNIT, owner: player })).toEqual([sfx("burn")]);
    }
  });

  it("R319 a card behind the sentinel sounds like any other, and none of the three speaks (R203, R204)", () => {
    const refusedReadable = cuesFor({ type: "libraryOverflow", player: "p2", instanceId: "c80", defId: UNIT, outcome: "ceased" }, ctx());
    const refusedHidden = cuesFor(
      { type: "libraryOverflow", player: "p2", instanceId: HIDDEN_DEF_ID, defId: HIDDEN_DEF_ID, outcome: "ceased" },
      ctx(),
    );
    expect(refusedHidden).toEqual(refusedReadable);
    const burnedReadable = cuesFor({ type: "burned", instanceId: "cX", defId: UNIT, owner: "p2" }, ctx());
    const burnedHidden = cuesFor({ type: "burned", instanceId: HIDDEN_DEF_ID, defId: HIDDEN_DEF_ID, owner: "p2" }, ctx());
    expect(burnedHidden).toEqual(burnedReadable);
    // #4 has play and death lines in the table: neither is spoken when it burns or is refused.
    expect(voices({ type: "burned", instanceId: "cX", defId: UNIT, owner: "p1" })).toEqual([]);
    expect(voices({ type: "libraryOverflow", player: "p1", instanceId: "c80", defId: UNIT, outcome: "graveyard" })).toEqual([]);
    expect(voices({ type: "fatigue", player: "p1", count: 1, amount: 1 })).toEqual([]);
  });
});

describe("R204: which moments speak", () => {
  it("R204 (B18) a readable unit's cardPlayed gives the play whoosh and its play line at VOICE_DELAY_MS", () => {
    expect(shape(played(UNIT))).toEqual([sfx("play"), voice(UNIT, "play", VOICE_DELAY_MS)].sort());
    expect(shape(played(TOKEN))).toEqual([sfx("play"), voice(TOKEN, "play", VOICE_DELAY_MS)].sort());
  });

  it("R204 (B18) the opponent's unit speaks its play line too: the card is public once played", () => {
    expect(shape(played(UNIT, "p2", "c7"))).toEqual([sfx("play"), voice(UNIT, "play", VOICE_DELAY_MS)].sort());
  });

  it("R204 (B18) a readable Spell's or Field Spell's cardPlayed gives play, spell at 60 ms and its cast line", () => {
    for (const defId of [SPELL, FIELD_SPELL]) {
      expect(shape(played(defId)), defId).toEqual([sfx("play"), sfx("spell", 60), voice(defId, "cast", VOICE_DELAY_MS)].sort());
    }
  });

  it("R204 (B18) a cardPlayed whose defId is not in the table gives play only", () => {
    expect(shape(played(NOT_IN_TABLE))).toEqual([sfx("play")]);
    expect(shape(played(NOT_IN_TABLE, "p2"))).toEqual([sfx("play")]);
  });

  it("R204 (B18) with an empty voice table nothing speaks, and a play still whooshes", () => {
    const empty: CardAudioTable = { voices: {}, effects: {}, cards: {}, emotes: {} };
    expect(shape(played(UNIT), ctx({ lines: empty }))).toEqual([sfx("play")]);
    expect(shape(played(SPELL), ctx({ lines: empty }))).toEqual([sfx("play")]);
    expect(shape(destroyed(UNIT), ctx({ lines: empty }))).toEqual([sfx("death")]);
  });

  it("R204 (B19) a readable unit's destroyed gives death and its death line at DEATH_VOICE_DELAY_MS, tokens included", () => {
    expect(shape(destroyed(UNIT))).toEqual([sfx("death"), voice(UNIT, "death", DEATH_VOICE_DELAY_MS)].sort());
    expect(shape(destroyed(UNIT, "p2", "u8"))).toEqual([sfx("death"), voice(UNIT, "death", DEATH_VOICE_DELAY_MS)].sort());
    expect(shape(destroyed(TOKEN))).toEqual([sfx("death"), voice(TOKEN, "death", DEATH_VOICE_DELAY_MS)].sort());
  });

  it("R204 (B19) a destroyed Spell, Field Spell, Trap or Field Trap gives death only", () => {
    for (const defId of [SPELL, FIELD_SPELL, TRAP, FIELD_TRAP]) {
      expect(shape(destroyed(defId)), defId).toEqual([sfx("death")]);
    }
  });

  it("R204 (B19) a destroyed defId not in the table gives death only", () => {
    expect(shape(destroyed(NOT_IN_TABLE))).toEqual([sfx("death")]);
  });

  it("R204 (B19) bounced, exiled, transformed and fused are not deaths or plays, and never speak", () => {
    const events: GameEvent[] = [
      { type: "bounced", instanceId: "u1", defId: UNIT, owner: "p1" },
      { type: "exiled", instanceId: "u1", defId: UNIT, owner: "p2" },
      { type: "transformed", instanceId: "u1", fromDefId: TOKEN, toDefId: UNIT, newInstanceId: "u9" },
      { type: "fused", instanceIds: ["u1", "u2"], resultInstanceId: "u9", defId: UNIT },
    ];
    for (const event of events) expect(voices(event), event.type).toEqual([]);
    // R1365: a fuse has its own sound; a transform into anything but a Sheep is the plain puff.
    expect(events.map((e) => onlySfx(e).id)).toEqual(["whoosh", "poof", "poof", "fuse"]);
    // A transform into a Sheep bleats as an effect, and still speaks no line.
    expect(voices(SAMPLES.transformed)).toEqual([]);
  });

  it("R204 (B56) a unit summoned without a play (a token, a Recruit, a Reborn, a copy) speaks its play line at the lowest priority", () => {
    const token: GameEvent = { type: "summoned", player: "p2", instanceId: "t1", defId: TOKEN, row: "units", lane: 2 };
    const recruited: GameEvent = { type: "summoned", player: "p1", instanceId: "c1", defId: UNIT, row: "units", lane: 1 };
    expect(shape(token)).toEqual([sfx("summon"), voice(TOKEN, "play", VOICE_DELAY_MS)].sort());
    expect(ranked(token)).toEqual([`${TOKEN}/play!${String(VOICE_PRIORITY.summon)}`]);
    expect(ranked(recruited, ctx({ wasPlayed: () => false }))).toEqual([`${UNIT}/play!${String(VOICE_PRIORITY.summon)}`]);
  });

  it("R204 (B56) a unit whose own cardPlayed has sounded does not speak again when it lands", () => {
    const landed: GameEvent = { type: "summoned", player: "p1", instanceId: "c1", defId: UNIT, row: "units", lane: 1 };
    const context = ctx({ wasPlayed: (id) => id === "c1" });
    expect(voices(landed, context)).toEqual([]);
    expect(shape(landed, context)).toEqual([sfx("summon")]);
  });

  it("R204 (B56) a summoned Spell-table entry, the sentinel or an unknown id never speaks", () => {
    for (const defId of [SPELL, TRAP, HIDDEN_DEF_ID, NOT_IN_TABLE]) {
      const event: GameEvent = { type: "summoned", player: "p1", instanceId: "c5", defId, row: "units", lane: 1 };
      expect(voices(event), defId).toEqual([]);
    }
  });

  it("R204 (B56) every line carries its priority: play and cast lines on a play, death and trap lines react", () => {
    expect(ranked(played(UNIT))).toEqual([`${UNIT}/play!${String(VOICE_PRIORITY.play)}`]);
    expect(ranked(played(SPELL))).toEqual([`${SPELL}/cast!${String(VOICE_PRIORITY.play)}`]);
    expect(ranked(destroyed(UNIT))).toEqual([`${UNIT}/death!${String(VOICE_PRIORITY.react)}`]);
    expect(ranked(trapFired(TRAP))).toEqual([`${TRAP}/cast!${String(VOICE_PRIORITY.react)}`]);
    expect(VOICE_PRIORITY.react).toBeGreaterThan(VOICE_PRIORITY.play);
    expect(VOICE_PRIORITY.play).toBeGreaterThan(VOICE_PRIORITY.summon);
  });

  it("R204 (B19) a cardResolved never speaks a second time", () => {
    expect(cuesFor({ type: "cardResolved", player: "p1", instanceId: "c1", defId: UNIT, permanent: true, costPaid: 3 }, ctx())).toEqual([]);
    expect(cuesFor({ type: "cardResolved", player: "p1", instanceId: "c2", defId: SPELL, permanent: false, costPaid: 1 }, ctx())).toEqual([]);
  });
});

/* --------------------------------------------------------------------------------------------- *
 * B20: what audio may reveal (R203)
 * --------------------------------------------------------------------------------------------- */

describe("R203: audio reveals no more than the screen", () => {
  it("R203 (B20) HIDDEN_DEF_ID is the sentinel a redacted event carries", () => {
    expect(HIDDEN_DEF_ID).toBe("hidden");
  });

  it("R203 (B20) a cardPlayed whose defId is the sentinel plays the generic whoosh and never speaks", () => {
    const event = played(HIDDEN_DEF_ID, "p2", HIDDEN_DEF_ID);
    expect(voices(event)).toEqual([]);
    expect(shape(event)).toEqual([sfx("play")]);
  });

  it("R203 (B20) a destroyed whose defId is the sentinel plays death and never speaks", () => {
    const event = destroyed(HIDDEN_DEF_ID, "p2", HIDDEN_DEF_ID);
    expect(voices(event)).toEqual([]);
    expect(shape(event)).toEqual([sfx("death")]);
  });

  it("R203 (B20) setting the viewer's own Trap or Field Trap gives trapSet only, and never speaks", () => {
    expect(shape(played(TRAP))).toEqual([sfx("trapSet")]);
    expect(shape(played(FIELD_TRAP))).toEqual([sfx("trapSet")]);
  });

  it("R203 (B20) a trapFired with a readable defId gives the sting and its cast line on the controller's seat", () => {
    expect(shape(trapFired(TRAP))).toEqual([sfx("trapSting"), voice(TRAP, "cast", VOICE_DELAY_MS)].sort());
    expect(shape(trapFired(FIELD_TRAP))).toEqual([sfx("trapSting"), voice(FIELD_TRAP, "cast", VOICE_DELAY_MS)].sort());
  });

  it("R203 (B20) a trapFired carrying the sentinel gives the sting alone", () => {
    expect(shape(trapFired(HIDDEN_DEF_ID, "p2"))).toEqual([sfx("trapSting")]);
  });

  it("R203 (B20) the sentinel never speaks, even from a table that has an entry under that name", () => {
    const poisoned: CardAudioTable = {
      ...LINES,
      cards: { ...LINES.cards, [HIDDEN_DEF_ID]: { kind: "unit", play: { voice: "hustler", text: "Guess who." }, death: { voice: "hustler", text: "Guess who." } } },
    };
    const context = ctx({ lines: poisoned });
    for (const event of [played(HIDDEN_DEF_ID, "p2", HIDDEN_DEF_ID), destroyed(HIDDEN_DEF_ID, "p2", HIDDEN_DEF_ID), trapFired(HIDDEN_DEF_ID, "p2")]) {
      expect(voices(event, context), event.type).toEqual([]);
    }
  });

  it("R203 (B20) the other seat hears a set trap as the generic play, with no line", () => {
    // p1 sets a trap; on p2's seat the event arrives redacted to the sentinel.
    const opponentSeat = ctx({ view: baseView({ viewer: "p2", you: emptySide("p2"), opponent: emptySide("p1") }) });
    const redacted = played(HIDDEN_DEF_ID, "p1", HIDDEN_DEF_ID);
    expect(voices(redacted, opponentSeat)).toEqual([]);
    expect(shape(redacted, opponentSeat)).toEqual([sfx("play")]);
  });
});

/* --------------------------------------------------------------------------------------------- *
 * B21: amounts
 * --------------------------------------------------------------------------------------------- */

describe("B21 amounts", () => {
  it("B21 damage, healed and healthLost with amount 0 give nothing", () => {
    expect(cuesFor({ type: "damage", sourceId: null, targetId: "u1", amount: 0, combat: false }, ctx())).toEqual([]);
    expect(cuesFor({ type: "healed", targetId: "hero-p1", amount: 0 }, ctx())).toEqual([]);
    expect(cuesFor({ type: "healthLost", player: "p1", amount: 0 }, ctx())).toEqual([]);
  });

  it("B21 a negative amount gives nothing either", () => {
    expect(cuesFor({ type: "damage", sourceId: null, targetId: "u1", amount: -2, combat: false }, ctx())).toEqual([]);
    expect(cuesFor({ type: "healed", targetId: "u1", amount: -1 }, ctx())).toEqual([]);
    expect(cuesFor({ type: "healthLost", player: "p2", amount: -5 }, ctx())).toEqual([]);
  });

  it("B21 damage n gives impact, healed n gives heal and healthLost n gives drain, each with params.amount n", () => {
    for (const n of [1, 7, 25]) {
      const hit = onlySfx({ type: "damage", sourceId: "u1", targetId: "u2", amount: n, combat: true });
      expect([hit.id, hit.params?.amount, hit.delayMs]).toEqual(["impact", n, 0]);
      const heal = onlySfx({ type: "healed", targetId: "hero-p2", amount: n });
      expect([heal.id, heal.params?.amount, heal.delayMs]).toEqual(["heal", n, 0]);
      const drain = onlySfx({ type: "healthLost", player: "p2", amount: n });
      expect([drain.id, drain.params?.amount, drain.delayMs]).toEqual(["drain", n, 0]);
    }
  });

  it("B21 gives repeated identical impacts independent ±5% pitch samples", () => {
    const random = vi.spyOn(Math, "random").mockReturnValueOnce(0).mockReturnValueOnce(1);
    const event: GameEvent = { type: "damage", sourceId: "u1", targetId: "u2", amount: 15, combat: true };
    expect(onlySfx(event).params?.variation).toBe(0);
    expect(onlySfx(event).params?.variation).toBe(1);
    random.mockRestore();
  });

  it("B21 buffed gives buff when attack + health is 0 or more", () => {
    for (const [attack, health] of [
      [1, 1],
      [0, 0],
      [2, -2],
      [-1, 3],
    ] as const) {
      expect(shape({ type: "buffed", instanceId: "u2", attack, health }), `${String(attack)}/${String(health)}`).toEqual([sfx("buff")]);
    }
  });

  it("B21 buffed gives debuff when attack + health is below 0", () => {
    for (const [attack, health] of [
      [-1, 0],
      [0, -1],
      [3, -4],
      [-2, -2],
    ] as const) {
      expect(shape({ type: "buffed", instanceId: "u2", attack, health }), `${String(attack)}/${String(health)}`).toEqual([sfx("debuff")]);
    }
  });
});

/* --------------------------------------------------------------------------------------------- *
 * B22: mana against a baseline
 * --------------------------------------------------------------------------------------------- */

describe("B22 manaChanged", () => {
  const before = (values: Partial<Record<PlayerId, number>>) => (player: PlayerId): number => values[player] ?? 0;

  it("B22 a gain above the baseline gives mana with the gain as amount, mine for the viewer", () => {
    const cue = onlySfx({ type: "manaChanged", player: "p1", current: 5, max: 5 }, ctx({ manaBefore: before({ p1: 3 }) }));
    expect(cue.id).toBe("mana");
    expect(cue.params?.amount).toBe(2);
    expect(cue.params?.mine).toBe(true);
    expect(cue.delayMs).toBe(0);
  });

  it("B22 the other seat's gain is not mine", () => {
    const cue = onlySfx({ type: "manaChanged", player: "p2", current: 4, max: 4 }, ctx({ manaBefore: before({ p2: 1 }) }));
    expect(cue.id).toBe("mana");
    expect(cue.params?.amount).toBe(3);
    expect(cue.params?.mine ?? false).toBe(false);
  });

  it("B22 mine follows the viewer, so the same gain is mine on p2's seat", () => {
    const p2Seat = ctx({ view: baseView({ viewer: "p2", you: emptySide("p2"), opponent: emptySide("p1") }), manaBefore: before({ p2: 0 }) });
    expect(onlySfx({ type: "manaChanged", player: "p2", current: 1, max: 1 }, p2Seat).params?.mine).toBe(true);
  });

  it("B22 a manaChanged at the baseline gives nothing", () => {
    expect(cuesFor({ type: "manaChanged", player: "p1", current: 3, max: 5 }, ctx({ manaBefore: before({ p1: 3 }) }))).toEqual([]);
  });

  it("B22 spending, a manaChanged below the baseline, gives nothing", () => {
    expect(cuesFor({ type: "manaChanged", player: "p1", current: 1, max: 5 }, ctx({ manaBefore: before({ p1: 4 }) }))).toEqual([]);
    expect(cuesFor({ type: "manaChanged", player: "p2", current: 0, max: 5 }, ctx({ manaBefore: before({ p2: 5 }) }))).toEqual([]);
  });

  it("B22 the baseline is read for the event's own player", () => {
    const context = ctx({ manaBefore: before({ p1: 9, p2: 0 }) });
    expect(shape({ type: "manaChanged", player: "p2", current: 2, max: 2 }, context)).toEqual([sfx("mana")]);
    expect(shape({ type: "manaChanged", player: "p1", current: 2, max: 2 }, context)).toEqual([]);
  });
});

/* --------------------------------------------------------------------------------------------- *
 * B23: seat-relative rows
 * --------------------------------------------------------------------------------------------- */

describe("B23 seat-relative rows", () => {
  const p2Seat = (): CueContext => ctx({ view: baseView({ viewer: "p2", you: emptySide("p2"), opponent: emptySide("p1") }) });

  it("B23 gameOver plays victory on the winner's seat", () => {
    expect(shape({ type: "gameOver", winner: "p1", reason: "hero-death" })).toEqual([sfx("victory")]);
    expect(shape({ type: "gameOver", winner: "p2", reason: "concede" }, p2Seat())).toEqual([sfx("victory")]);
  });

  it("B23 gameOver plays defeat on the other seat", () => {
    expect(shape({ type: "gameOver", winner: "p2", reason: "hero-death" })).toEqual([sfx("defeat")]);
    expect(shape({ type: "gameOver", winner: "p1", reason: "turn-cap" }, p2Seat())).toEqual([sfx("defeat")]);
  });

  it("B23 gameOver on a draw plays notify on both seats", () => {
    expect(shape({ type: "gameOver", winner: "draw", reason: "draw-accepted" })).toEqual([sfx("notify")]);
    expect(shape({ type: "gameOver", winner: "draw", reason: "both-heroes-dead" }, p2Seat())).toEqual([sfx("notify")]);
  });

  it("B23 promptOpened sounds only for the player holding the prompt", () => {
    expect(shape({ type: "promptOpened", player: "p1", choiceId: "ch1", kind: "target" })).toEqual([sfx("notify")]);
    expect(shape({ type: "promptOpened", player: "p2", choiceId: "ch2", kind: "target" })).toEqual([]);
    expect(shape({ type: "promptOpened", player: "p2", choiceId: "ch2", kind: "target" }, p2Seat())).toEqual([sfx("notify")]);
  });

  it("B23 drawOffered sounds only on the seat that must answer, never for the offerer", () => {
    expect(shape({ type: "drawOffered", player: "p2" })).toEqual([sfx("notify")]);
    expect(shape({ type: "drawOffered", player: "p1" })).toEqual([]);
    expect(shape({ type: "drawOffered", player: "p2" }, p2Seat())).toEqual([]);
  });

  it("drawOffered rings as a question (the urgent notify), not as the routine blips", () => {
    expect(onlySfx({ type: "drawOffered", player: "p2" }).params).toEqual({ urgent: true });
    expect(onlySfx({ type: "drawOffered", player: "p1" }, p2Seat()).params).toEqual({ urgent: true });
    // Every other notify stays the plain one.
    expect(onlySfx({ type: "turnAutoEnded", player: "p1", turn: 3 }).params).toBeUndefined();
    expect(onlySfx({ type: "promptOpened", player: "p1", choiceId: "ch1", kind: "target" }).params).toBeUndefined();
  });

  it("drawAnswered is the offerer's news: a decline sounds cancel, an acceptance leaves the draw to gameOver, and the answering seat hears nothing", () => {
    // p1 is the viewer. p2 answered p1's offer: p1 is the offerer.
    expect(shape({ type: "drawAnswered", player: "p2", accept: false })).toEqual([sfx("cancel")]);
    expect(shape({ type: "drawAnswered", player: "p2", accept: true })).toEqual([]);
    // p1 answered p2's offer: its own click already ticked.
    expect(shape({ type: "drawAnswered", player: "p1", accept: false })).toEqual([]);
    expect(shape({ type: "drawAnswered", player: "p1", accept: true })).toEqual([]);
    // The same from p2's seat.
    expect(shape({ type: "drawAnswered", player: "p1", accept: false }, p2Seat())).toEqual([sfx("cancel")]);
    expect(shape({ type: "drawAnswered", player: "p2", accept: false }, p2Seat())).toEqual([]);
  });

  it("B23 turnStarted carries mine for the viewer's own turn only", () => {
    const mine = onlySfx({ type: "turnStarted", player: "p1", turn: 5 });
    expect([mine.id, mine.params?.mine]).toEqual(["turnStart", true]);
    const theirs = onlySfx({ type: "turnStarted", player: "p2", turn: 6 });
    expect(theirs.id).toBe("turnStart");
    expect(theirs.params?.mine ?? false).toBe(false);
    expect(onlySfx({ type: "turnStarted", player: "p2", turn: 6 }, p2Seat()).params?.mine).toBe(true);
  });
});

/* --------------------------------------------------------------------------------------------- *
 * B56: a summon sounds its size
 * --------------------------------------------------------------------------------------------- */

describe("B56 a summon is sized by the unit that lands", () => {
  const landed: GameEvent = { type: "summoned", player: "p1", instanceId: "c1", defId: UNIT, row: "units", lane: 1 };
  const played1 = ctx({ wasPlayed: () => true });

  it("B56 the summon thud carries the unit's attack plus health, as the newest view shows it", () => {
    const big = unit("p1", { instanceId: "c1", defId: UNIT, attack: 7, health: 7, maxHealth: 7 });
    const cues = cuesFor(landed, { ...played1, unitNow: (id) => (id === "c1" ? big : null) });
    expect(cues).toEqual([{ kind: "sfx", id: "summon", params: { amount: 14 }, delayMs: 0 }]);
  });

  it("B56 a unit the view cannot find sounds the plain thud", () => {
    expect(cuesFor(landed, { ...played1, unitNow: () => null })).toEqual([{ kind: "sfx", id: "summon", delayMs: 0 }]);
    expect(cuesFor(landed, played1)).toEqual([{ kind: "sfx", id: "summon", delayMs: 0 }]);
  });

  it("B56 a Radiant unit lands with a golden glint on top of the thud", () => {
    const golden = unit("p1", { instanceId: "c1", defId: UNIT, attack: 2, health: 3, radiant: true });
    const ids = cuesFor(landed, { ...played1, unitNow: () => golden }).map((c) => (c.kind === "sfx" ? c.id : c.kind));
    expect(ids).toEqual(["summon", "radiant"]);
  });
});

/* --------------------------------------------------------------------------------------------- *
 * Integration: the public catalog colours a card the viewer can name (docs/polish/reference.md,
 * audio x cards), and R203 still bounds it.
 * --------------------------------------------------------------------------------------------- */

describe("the catalog colours summons and spells, never what the viewer cannot name", () => {
  const CATALOG: Record<string, CueCard> = {
    [UNIT]: { type: "Unit", tags: ["Felinor"], rarity: "Common" },
    [SPELL]: { type: "Spell", tags: ["Call to Chaos"], rarity: "Rare" },
    [FIELD_SPELL]: { type: "Field Spell", tags: [], rarity: "Epic" },
    [TRAP]: { type: "Trap", tags: ["KY"], rarity: "Rare" },
    "core-legend": { type: "Unit", tags: ["Human"], rarity: "Legendary" },
    "core-mythic": { type: "Unit", tags: [], rarity: "Mythic" },
  };
  const withCatalog = (over: Partial<CueContext> = {}): CueContext =>
    ctx({ card: (defId) => CATALOG[defId], ...over });
  const summoned = (defId: string, row: "units" | "backrow" = "units"): GameEvent => ({
    type: "summoned",
    player: "p1",
    instanceId: "c1",
    defId,
    row,
    lane: 1,
  });
  const sfxCue = (cues: readonly SoundCue[], id: SfxId): Extract<SoundCue, { kind: "sfx" }> | undefined =>
    cues.find((cue): cue is Extract<SoundCue, { kind: "sfx" }> => cue.kind === "sfx" && cue.id === id);

  it("timbreFor follows the card art's theme: tags first, then Token, then a Field Spell's type", () => {
    expect(timbreFor({ type: "Unit", tags: ["Human", "Felinor"] })).toBe("felinor");
    expect(timbreFor({ type: "Spell", tags: ["Call to Chaos", "KY"] })).toBe("chaos");
    expect(timbreFor({ type: "Unit", tags: ["Token"] })).toBe("token");
    expect(timbreFor({ type: "Field Spell", tags: [] })).toBe("field");
    expect(timbreFor({ type: "Unit", tags: [] })).toBeUndefined();
    expect(timbreFor({ type: "Spell", tags: [] })).toBeUndefined();
  });

  it("a unit the viewer can name lands with its family's accent on the thud", () => {
    const cue = sfxCue(cuesFor(summoned(UNIT), withCatalog()), "summon");
    expect(cue?.params?.timbre).toBe("felinor");
  });

  it("a Legendary unit enters with the brass sting, a Mythic one with the prismatic sting, at the thud", () => {
    const legend = cuesFor(summoned("core-legend"), withCatalog());
    expect(sfxCue(legend, "entrance")).toEqual({ kind: "sfx", id: "entrance", delayMs: 0 });
    expect(sfxCue(legend, "summon")?.params?.timbre).toBe("human");

    const mythic = cuesFor(summoned("core-mythic"), withCatalog());
    expect(sfxCue(mythic, "entrance")).toEqual({ kind: "sfx", id: "entrance", params: { mythic: true }, delayMs: 0 });
    expect(sfxCue(cuesFor(summoned(UNIT), withCatalog()), "entrance")).toBeUndefined();
  });

  it("a cast spell rings in its family's chimes, a Field Spell in the field's", () => {
    expect(sfxCue(cuesFor(played(SPELL), withCatalog()), "spell")?.params).toEqual({ timbre: "chaos" });
    expect(sfxCue(cuesFor(played(FIELD_SPELL), withCatalog()), "spell")?.params).toEqual({ timbre: "field" });
  });

  it("with no catalog every card keeps its type's plain sounds", () => {
    expect(sfxCue(cuesFor(summoned("core-legend"), ctx()), "entrance")).toBeUndefined();
    expect(sfxCue(cuesFor(summoned(UNIT), ctx()), "summon")?.params).toBeUndefined();
    expect(sfxCue(cuesFor(played(SPELL), ctx()), "spell")?.params).toBeUndefined();
  });

  it("R203 a card behind the sentinel is never looked up: no accent, no sting, the generic sound", () => {
    const asked: string[] = [];
    const context = withCatalog({
      card: (defId) => {
        asked.push(defId);
        return CATALOG["core-mythic"];
      },
    });
    const cues = cuesFor(summoned(HIDDEN_DEF_ID), context);
    expect(sfxCue(cues, "entrance")).toBeUndefined();
    expect(sfxCue(cues, "summon")?.params?.timbre).toBeUndefined();
    expect(shape(played(HIDDEN_DEF_ID), context)).toEqual([sfx("play")]);
    expect(asked).toEqual([]);
  });

  it("R203 a Trap sounds like every Trap: its set and its backrow arrival take no family", () => {
    expect(shape(played(TRAP), withCatalog())).toEqual([sfx("trapSet")]);
    const arrival = sfxCue(cuesFor(summoned(TRAP, "backrow"), withCatalog()), "summon");
    expect(arrival?.params?.timbre).toBeUndefined();
    expect(sfxCue(cuesFor(summoned("core-mythic", "backrow"), withCatalog()), "entrance")).toBeUndefined();
  });
});

/* --------------------------------------------------------------------------------------------- *
 * Patch v0.2.0 (R506): the Book, Pancake and AI families, a token's printed rarity, the card
 * moments, Call to Chaos's roll and a mark. Real catalog cards, through the board's own lookup.
 * --------------------------------------------------------------------------------------------- */

const LOOKUP = lookupFromDefs(CATALOG);
const realCard = (defId: string): CueCard | undefined => cueCard(LOOKUP, defId);
/** The inline voice table, with the Book spell's entry: a spell's shimmer rides its table kind. */
const REAL_LINES: CardAudioTable = {
  ...LINES,
  cards: { ...LINES.cards, "classic-016": { kind: "spell", cast: { voice: "narrator", text: "Hot off the press." } } },
};
const withRealCatalog = (over: Partial<CueContext> = {}): CueContext => ctx({ card: realCard, lines: REAL_LINES, ...over });
const BOOK_SPELL = "classic-016"; // Book of Flame
const PANCAKE_UNIT = "classicplus-012"; // The Mother Pancake, Legendary
const AI_TOKEN_UNIT = "classicplus-t-ai-01"; // Helpful Assistant
const GOLEM_TOKEN = "classicplus-073-1"; // Classic Golem: a token that prints Legendary
const EPIC_TOKEN = "classicplus-038-1"; // Solarius Prime: a token that prints Epic
const LEGENDARY_FIELD_TOKEN = "classicplus-012-5"; // Anti-Waffle Shell: a Field Spell token printing Legendary

const landedAs = (defId: string, row: "units" | "backrow" = "units", instanceId = "u9"): GameEvent => ({
  type: "summoned",
  player: "p2",
  instanceId,
  defId,
  row,
  lane: 2,
});
const sfxOf = (cues: readonly SoundCue[], id: SfxId): Extract<SoundCue, { kind: "sfx" }> | undefined =>
  cues.find((cue): cue is Extract<SoundCue, { kind: "sfx" }> => cue.kind === "sfx" && cue.id === id);
const frame = (defId: string, instanceId = "c7", player: PlayerId = "p2"): PlayFrame => ({ defId, instanceId, player, castOnDraw: false });
const inside = (defId: string | null): Partial<CueContext> => ({ playing: () => (defId === null ? null : frame(defId)) });

describe("R506 the Book, Pancake and AI families", () => {
  it("R506 the real catalog's Book spell, Pancake unit and AI token take their tag's family", () => {
    expect(timbreFor(realCard(BOOK_SPELL) ?? { type: "Unit", tags: [] })).toBe("book");
    expect(timbreFor(realCard(PANCAKE_UNIT) ?? { type: "Unit", tags: [] })).toBe("pancake");
    expect(timbreFor(realCard(AI_TOKEN_UNIT) ?? { type: "Unit", tags: [] })).toBe("ai");
  });

  it("R506 wherever the card art names a family for a catalog card, the sound takes that same family", () => {
    const families = new Set<string>(SFX_TIMBRES.filter((t) => t !== "field" && t !== "token"));
    const differ: string[] = [];
    for (const def of Object.values(CATALOG)) {
      const theme: string = themeFor(def.tags, def.type);
      if (!families.has(theme)) continue;
      const timbre = timbreFor({ type: def.type, tags: def.tags });
      if (timbre !== theme) differ.push(`${def.id}: art ${theme}, sound ${String(timbre)}`);
    }
    expect(differ).toEqual([]);
  });

  it("R506 a Book, Pancake or AI card the art gives no family of its own still takes its tag's, ahead of Token and its type", () => {
    const wrong: string[] = [];
    const tagged: Record<string, string> = { Book: "book", Pancake: "pancake", AI: "ai" };
    const known = new Set<string>(SFX_TIMBRES);
    for (const def of Object.values(CATALOG)) {
      const tag = def.tags.find((t) => t in tagged);
      if (tag === undefined) continue;
      const theme: string = themeFor(def.tags, def.type);
      if (known.has(theme) && theme !== "token" && theme !== "field") continue; // the art's family decides (above)
      const timbre = timbreFor({ type: def.type, tags: def.tags });
      if (timbre !== tagged[tag]) wrong.push(`${def.id} (${def.tags.join("+")}): ${String(timbre)}`);
    }
    expect(wrong).toEqual([]);
  });

  it("R506 a Pancake unit lands with the pancake accent, a Book spell rings in the book's chimes, an AI token lands with the AI's", () => {
    expect(sfxOf(cuesFor(landedAs(PANCAKE_UNIT), withRealCatalog()), "summon")?.params?.timbre).toBe("pancake");
    expect(sfxOf(cuesFor(played(BOOK_SPELL, "p2"), withRealCatalog()), "spell")?.params).toEqual({ timbre: "book" });
    expect(sfxOf(cuesFor(landedAs(AI_TOKEN_UNIT), withRealCatalog()), "summon")?.params?.timbre).toBe("ai");
  });

  it("R203 a new family never reaches a card behind the sentinel", () => {
    const cues = cuesFor(landedAs(HIDDEN_DEF_ID, "units", HIDDEN_DEF_ID), withRealCatalog());
    expect(sfxOf(cues, "summon")?.params?.timbre).toBeUndefined();
    expect(shape(played(HIDDEN_DEF_ID, "p2", HIDDEN_DEF_ID), withRealCatalog())).toEqual([sfx("play")]);
  });
});

describe("R506 a token's printed rarity gives its summon the rarity sting", () => {
  it("R506 the board's catalog facts carry a token's printed rarity beside its Token rarity", () => {
    expect(realCard(GOLEM_TOKEN)).toMatchObject({ rarity: "Token", printedRarity: "Legendary" });
    expect(realCard(PANCAKE_UNIT)?.printedRarity).toBeUndefined();
  });

  it("R506 a unit token that prints Legendary enters with the Legendary sting; one that prints Epic does not", () => {
    expect(sfxOf(cuesFor(landedAs(GOLEM_TOKEN), withRealCatalog()), "entrance")).toEqual({ kind: "sfx", id: "entrance", delayMs: 0 });
    expect(sfxOf(cuesFor(landedAs(EPIC_TOKEN), withRealCatalog()), "entrance")).toBeUndefined();
    expect(sfxOf(cuesFor(landedAs(AI_TOKEN_UNIT), withRealCatalog()), "entrance")).toBeUndefined();
  });

  it("R506 a printed Mythic enters with the Mythic sting", () => {
    const mythicToken: CueCard = { type: "Unit", tags: ["Token"], rarity: "Token", printedRarity: "Mythic" };
    const cues = cuesFor(landedAs("core-t-x"), ctx({ card: () => mythicToken }));
    expect(sfxOf(cues, "entrance")).toEqual({ kind: "sfx", id: "entrance", params: { mythic: true }, delayMs: 0 });
  });

  it("R203 a printed rarity never colours the backrow or a card behind the sentinel", () => {
    expect(sfxOf(cuesFor(landedAs(LEGENDARY_FIELD_TOKEN, "backrow"), withRealCatalog()), "entrance")).toBeUndefined();
    const asked: string[] = [];
    const context = ctx({
      card: (defId) => {
        asked.push(defId);
        return realCard(GOLEM_TOKEN);
      },
    });
    expect(sfxOf(cuesFor(landedAs(HIDDEN_DEF_ID, "units", HIDDEN_DEF_ID), context), "entrance")).toBeUndefined();
    expect(asked).toEqual([]);
  });
});

describe("R506 a card cast as it is drawn stings", () => {
  const cast = (instanceId: string): Partial<CueContext> => ({ castOnDraw: (id) => id === instanceId });

  it("R506 a readable spell cast on draw stings in place of the play whoosh, keeping its shimmer and its line", () => {
    expect(shape(played(SPELL, "p2", "c7"), ctx(cast("c7")))).toEqual(
      [sfx("castOnDraw"), sfx("spell", 60), voice(SPELL, "cast", VOICE_DELAY_MS)].sort(),
    );
    expect(shape(played(UNIT, "p2", "c7"), ctx(cast("c7")))).toEqual([sfx("castOnDraw"), voice(UNIT, "play", VOICE_DELAY_MS)].sort());
  });

  it("R506 only the card the director marks stings: any other play whooshes as before", () => {
    expect(shape(played(SPELL, "p2", "c8"), ctx(cast("c7")))).toContain(sfx("play"));
    expect(shape(played(SPELL, "p2", "c7"))).toContain(sfx("play"));
    expect(shape(played(NOT_IN_TABLE, "p2", "c7"), ctx(cast("c7")))).toEqual([sfx("castOnDraw")]);
  });

  it("R203 a cast on draw behind the sentinel, or a Trap's, sounds like any other play of it", () => {
    expect(shape(played(HIDDEN_DEF_ID, "p2", HIDDEN_DEF_ID), ctx(cast(HIDDEN_DEF_ID)))).toEqual([sfx("play")]);
    expect(shape(played(TRAP, "p1", "c7"), ctx(cast("c7")))).toEqual([sfx("trapSet")]);
    const trapCard: CueCard = { type: "Trap", tags: [] };
    expect(shape(played(NOT_IN_TABLE, "p1", "c7"), ctx({ ...cast("c7"), card: () => trapCard }))).toEqual([sfx("play")]);
  });
});

describe("R506 #21 Hinder cracks the victim's mana", () => {
  const rider = (added: boolean, player: PlayerId = "p1"): GameEvent => ({ type: "modifierChanged", player, modifierId: NEXT_REFRESH_MODIFIER_ID, added });

  it("R506 the next-refresh rider landing inside a readable Hinder play is a mana crack, on either seat", () => {
    expect(shape(rider(true), ctx(inside(HINDER_DEF_ID)))).toEqual([sfx("manaCrack")]);
    const p2Seat = ctx({ ...inside(HINDER_DEF_ID), view: baseView({ viewer: "p2", you: emptySide("p2"), opponent: emptySide("p1") }) });
    expect(shape(rider(true), p2Seat)).toEqual([sfx("manaCrack")]);
  });

  it("R506 it cracks even when it cancels a rider already there (the badge goes, R169)", () => {
    expect(shape(rider(false), ctx(inside(HINDER_DEF_ID)))).toEqual([sfx("manaCrack")]);
  });

  it("R506 any other rider, or any other modifier inside Hinder's play, sounds as before", () => {
    expect(shape(rider(true), ctx(inside("core-024")))).toEqual([sfx("notify")]);
    expect(shape(rider(true), ctx(inside(null)))).toEqual([sfx("notify")]);
    expect(shape(rider(false))).toEqual([]);
    expect(shape({ type: "modifierChanged", player: "p1", modifierId: "m1", added: true }, ctx(inside(HINDER_DEF_ID)))).toEqual([sfx("notify")]);
  });

  it("R203 a play behind the sentinel is no Hinder", () => {
    expect(shape(rider(true), ctx(inside(HIDDEN_DEF_ID)))).toEqual([sfx("notify")]);
  });
});

describe("R506 #27 Blood Ridden Glowy Jelly Bean: a blood drain, then a gold burst", () => {
  const radiantInHand = (instanceId: string, defId: string, player: PlayerId): GameEvent => ({
    type: "radiantSet",
    instanceId,
    defId,
    zone: { z: "hand", player },
  });
  const BURST = [sfx("bloodDrain"), sfx("goldBurst", GOLD_BURST_DELAY_MS)].sort();

  it("R506 a card #27 turns Radiant is its blood drain and gold burst, the burst GOLD_BURST_DELAY_MS behind", () => {
    expect(shape(radiantInHand("c3", UNIT, "p1"), ctx(inside(BLOOD_BEAN_DEF_ID)))).toEqual(BURST);
  });

  it("R203 on the other seat the card is the sentinel, and the same sound plays: it reads the public cast alone", () => {
    const p1Seat = ctx(inside(BLOOD_BEAN_DEF_ID));
    expect(shape(radiantInHand(HIDDEN_DEF_ID, HIDDEN_DEF_ID, "p2"), p1Seat)).toEqual(BURST);
    // …and whatever the hidden card, the same.
    expect(shape(radiantInHand(HIDDEN_DEF_ID, HIDDEN_DEF_ID, "p2"), p1Seat)).toEqual(shape(radiantInHand("c3", UNIT, "p2"), p1Seat));
  });

  it("R506 any other Radiant, or #27 behind the sentinel, is the plain glint", () => {
    expect(shape(radiantInHand("c3", UNIT, "p1"), ctx(inside("core-026")))).toEqual([sfx("radiant")]);
    expect(shape(radiantInHand("c3", UNIT, "p1"))).toEqual([sfx("radiant")]);
    expect(shape(radiantInHand("c3", UNIT, "p1"), ctx(inside(HIDDEN_DEF_ID)))).toEqual([sfx("radiant")]);
  });
});

describe("R506 Call to Chaos's roll (R436) and a mark (R437)", () => {
  const rolled = (effects: string[], instanceId = "c1", defId = SPELL): GameEvent => ({ type: "chaosRolled", player: "p2", instanceId, defId, effects });

  it("R506 the roll dings once for each effect it names, up to CHAOS_REVEAL_MAX", () => {
    expect(onlySfx(rolled(["Heal your hero 30"])).params).toEqual({ amount: 1 });
    expect(onlySfx(rolled(["a", "b", "c"])).params).toEqual({ amount: 3 });
    expect(onlySfx(rolled(["a", "b", "c", "d", "e"])).params).toEqual({ amount: CHAOS_REVEAL_MAX });
    expect(onlySfx(rolled([])).params).toEqual({ amount: 0 });
  });

  it("R203 both seats hear the same roll, and a card behind the sentinel changes nothing", () => {
    const p2Seat = ctx({ view: baseView({ viewer: "p2", you: emptySide("p2"), opponent: emptySide("p1") }) });
    const event = rolled(["a", "b"]);
    expect(cuesFor(event, p2Seat)).toEqual(cuesFor(event, ctx()));
    expect(cuesFor(rolled(["a", "b"], HIDDEN_DEF_ID, HIDDEN_DEF_ID), ctx())).toEqual(cuesFor(event, ctx()));
  });

  it("R506 a mark brands its card as it lands and lets go softly as it lifts", () => {
    const mark = (added: boolean, instanceId = "u6"): GameEvent => ({ type: "marked", instanceId, mark: "steal", color: "purple", added });
    expect(onlySfx(mark(true))).toEqual({ kind: "sfx", id: "brand", delayMs: 0 });
    expect(onlySfx(mark(false))).toEqual({ kind: "sfx", id: "brand", params: { release: true }, delayMs: 0 });
    expect(cuesFor(mark(true, HIDDEN_DEF_ID), ctx())).toEqual(cuesFor(mark(true), ctx()));
    const recoloured: GameEvent = { type: "marked", instanceId: "u6", mark: "curse", color: "green", added: true };
    expect(cuesFor(recoloured, ctx())).toEqual(cuesFor(mark(true), ctx()));
  });
});

describe("R506 patch v0.2.0's moments sound the way they went", () => {
  it("R506 a hero's health set is a heal of the gain or a drain of the loss, and a notice when nothing changed", () => {
    const view = baseView();
    const health = view.opponent.hero.health;
    const set = (to: number): GameEvent => ({ type: "healthSet", player: view.opponent.player, health: to, sourceId: null });
    expect(cuesFor(set(health + 4), ctx({ view }))).toEqual([{ kind: "sfx", id: "heal", params: { amount: 4 }, delayMs: 0 }]);
    expect(cuesFor(set(health - 9), ctx({ view }))).toEqual([{ kind: "sfx", id: "drain", params: { amount: 9 }, delayMs: 0 }]);
    expect(shape(set(health), ctx({ view }))).toEqual([sfx("notify")]);
  });

  it("R506 R1365 a crumbling card cracks dry and falls apart in a sound of its own, and never speaks", () => {
    expect(shape(SAMPLES.crumbled)).toEqual(["sfx:crumble@0"]);
    expect(voices(SAMPLES.crumbled)).toEqual([]);
  });

  it("R203 R506 an Animated card lands with a summon sized by the Unit it is now, and no family accent", () => {
    const unit = { attack: 4, health: 4, armor: 0, keywords: [] } as unknown as UnitView;
    const cues = cuesFor(SAMPLES.animated, ctx({ unitNow: () => unit, card: () => ({ type: "Field Trap", tags: ["Human"] }) }));
    // #185: and with its slam's tier (4 + 4 is Small) and a pitch sample.
    expect(cues).toEqual([
      { kind: "sfx", id: "summon", params: { amount: 8, slamTier: "small", variation: expect.any(Number) }, delayMs: 0 },
    ]);
    expect(cuesFor(SAMPLES.animated, ctx())).toEqual([{ kind: "sfx", id: "summon", delayMs: 0 }]);
  });
});

/* --------------------------------------------------------------------------------------------- *
 * R655: a hook's effect plays at the moment its line would speak
 * --------------------------------------------------------------------------------------------- */

describe("R655 a card's hook may carry an effect beside or instead of its line", () => {
  /** The Rock (#66) with an effect and a line on its play and only an effect on its death; #5, #18 and #96 likewise. */
  const FX: CardAudioTable = {
    voices: LINES.voices,
    effects: {
      thud: { sfx: "impact", pitch: 0.6, gain: 1 },
      crumble: { sfx: "death", pitch: 1.3, gain: 0.7 },
      zap: { sfx: "castOnDraw", pitch: 1.3, gain: 1 },
      gong: { sfx: "entrance", pitch: 0.7, gain: 0.6 },
      rumble: { sfx: "death", pitch: 0.5, gain: 0.8 },
    },
    cards: {
      ...LINES.cards,
      "core-066": {
        kind: "unit",
        play: { voice: "hustler", text: "Rock solid.", effect: "thud" },
        attack: { effect: "rumble" },
        death: { effect: "crumble" },
      },
      [SPELL]: { kind: "spell", cast: { voice: "narrator", text: "Hoarding is self care.", effect: "zap" } },
      [FIELD_SPELL]: { kind: "spell", cast: { effect: "zap" } },
      [TRAP]: { kind: "trap", cast: { voice: "snob", text: "Checkmate, puppet.", effect: "gong" } },
    },
    emotes: {},
  };
  const ROCK = "core-066";
  const fx = (over: Partial<CueContext> = {}): CueContext => ctx({ lines: FX, ...over });
  const summonedUnit = (defId: string): GameEvent => ({ type: "summoned", player: "p2", instanceId: "t1", defId, row: "units", lane: 1 });

  it("R655 a unit's play starts its effect at the line's moment and its line CARD_EFFECT_DELAY_MS later", () => {
    expect(shape(played(ROCK), fx())).toEqual(
      [sfx("play"), effect(ROCK, "play", VOICE_DELAY_MS), voice(ROCK, "play", VOICE_DELAY_MS + CARD_EFFECT_DELAY_MS)].sort(),
    );
    // Both carry the play's priority, so a flush ranks them alike.
    const cues = cuesFor(played(ROCK), fx()).filter((c) => c.kind !== "sfx");
    expect(cues.map((c) => c.priority)).toEqual([VOICE_PRIORITY.play, VOICE_PRIORITY.play]);
  });

  it("R655 a unit an effect summons plays its play effect at the lowest priority, as its line would", () => {
    const cues = cuesFor(summonedUnit(ROCK), fx()).filter((c) => c.kind === "effect");
    expect(cues).toEqual([{ kind: "effect", defId: ROCK, hook: "play", delayMs: VOICE_DELAY_MS, priority: VOICE_PRIORITY.summon }]);
  });

  it("R655 a death hook that is only an effect plays the effect and speaks nothing", () => {
    expect(shape(destroyed(ROCK), fx())).toEqual([sfx("death"), effect(ROCK, "death", DEATH_VOICE_DELAY_MS)].sort());
    expect(voices(destroyed(ROCK), fx())).toEqual([]);
  });

  it("R655 a spell's cast and a firing trap's cast carry their effects; a Field Spell's may be an effect alone", () => {
    expect(shape(played(SPELL), fx())).toContain(effect(SPELL, "cast", VOICE_DELAY_MS));
    expect(shape(played(SPELL), fx())).toContain(voice(SPELL, "cast", VOICE_DELAY_MS + CARD_EFFECT_DELAY_MS));
    expect(shape(played(FIELD_SPELL), fx()).filter((c) => !c.startsWith("sfx:"))).toEqual([effect(FIELD_SPELL, "cast", VOICE_DELAY_MS)]);
    expect(shape(trapFired(TRAP), fx())).toEqual(
      [sfx("trapSting"), effect(TRAP, "cast", VOICE_DELAY_MS), voice(TRAP, "cast", VOICE_DELAY_MS + CARD_EFFECT_DELAY_MS)].sort(),
    );
    const trapCues = cuesFor(trapFired(TRAP), fx()).filter((c) => c.kind === "effect");
    expect(trapCues.map((c) => c.priority)).toEqual([VOICE_PRIORITY.react]);
  });

  it("R655 (R203) a trap's set plays no effect: its own seat hears the set, the other the plain whoosh", () => {
    expect(shape(played(TRAP), fx())).toEqual([sfx("trapSet")]);
    expect(shape(played(HIDDEN_DEF_ID, "p2", HIDDEN_DEF_ID), fx())).toEqual([sfx("play")]);
  });

  it("R655 (R203) the sentinel plays no effect, even from a table that has one under that name", () => {
    const poisoned: CardAudioTable = {
      ...FX,
      cards: { ...FX.cards, [HIDDEN_DEF_ID]: { kind: "unit", play: { effect: "thud" }, death: { effect: "crumble" } } },
    };
    const context = ctx({ lines: poisoned });
    for (const event of [played(HIDDEN_DEF_ID, "p2", HIDDEN_DEF_ID), destroyed(HIDDEN_DEF_ID, "p2", HIDDEN_DEF_ID), trapFired(HIDDEN_DEF_ID, "p2")]) {
      expect(cuesFor(event, context).filter((c) => c.kind === "effect"), event.type).toEqual([]);
    }
  });

  it("R655 the attack hook is no event's: an attack declared plays the plain attack sound alone", () => {
    const declared: GameEvent = { type: "attackDeclared", attackerId: "u1", targetId: "u6", forced: false };
    expect(shape(declared, fx())).toEqual([sfx("attack")]);
  });
});

/* --------------------------------------------------------------------------------------------- *
 * R669 (#259): a sting for every readable play, and lane panning
 * --------------------------------------------------------------------------------------------- */

describe("R669 every card the viewer can read stings as it is played, by its rarity", () => {
  const catalogOf =
    (rarity: CueCard["rarity"], type: CueCard["type"] = "Unit") =>
    (): CueCard => ({ type, tags: [], ...(rarity === undefined ? {} : { rarity }) });
  const stingOf = (event: GameEvent, context: CueContext): SoundCue | undefined =>
    cuesFor(event, context).find((c) => c.kind === "sfx" && (c.id === "sting" || c.id === "entrance"));

  it("R669 a Common, Rare or Epic Unit or Spell stings on its cardPlayed, sized by its rarity, just after the whoosh", () => {
    for (const [rarity, params] of [
      ["Common", undefined],
      ["Rare", { tier: "rare" }],
      ["Epic", { tier: "epic" }],
    ] as const) {
      for (const [defId, type] of [
        [UNIT, "Unit"],
        [SPELL, "Spell"],
      ] as const) {
        const cue = stingOf(played(defId), ctx({ card: catalogOf(rarity, type) }));
        expect(cue, `${rarity} ${type}`).toEqual(
          params === undefined
            ? { kind: "sfx", id: "sting", delayMs: STING_DELAY_MS }
            : { kind: "sfx", id: "sting", params, delayMs: STING_DELAY_MS },
        );
      }
    }
  });

  it("R669 a Legendary or Mythic Spell enters with the entrance on its cast; a Legendary or Mythic Unit keeps its entrance on summoned", () => {
    expect(stingOf(played(SPELL), ctx({ card: catalogOf("Legendary", "Spell") }))).toEqual({
      kind: "sfx",
      id: "entrance",
      delayMs: STING_DELAY_MS,
    });
    expect(stingOf(played(SPELL), ctx({ card: catalogOf("Mythic", "Spell") }))).toMatchObject({
      id: "entrance",
      params: { mythic: true },
    });
    expect(stingOf(played(UNIT), ctx({ card: catalogOf("Legendary") }))).toBeUndefined();
    const summon: GameEvent = { type: "summoned", player: "p1", instanceId: "c1", defId: UNIT, row: "units", lane: 3 };
    expect(stingOf(summon, ctx({ card: catalogOf("Legendary") }))).toMatchObject({ id: "entrance" });
  });

  it("R669 no sting for the sentinel, a Trap's set, a card cast as it is drawn, or a Token with no printed rarity", () => {
    expect(stingOf(played(HIDDEN_DEF_ID, "p2"), ctx({ card: catalogOf("Epic") }))).toBeUndefined();
    expect(stingOf(played(TRAP), ctx({ card: catalogOf("Epic", "Trap") }))).toBeUndefined();
    expect(stingOf(played(FIELD_TRAP), ctx({ card: catalogOf("Epic", "Field Trap") }))).toBeUndefined();
    expect(stingOf(played(SPELL), ctx({ card: catalogOf("Rare", "Spell"), castOnDraw: () => true }))).toBeUndefined();
    expect(stingOf(played(TOKEN), ctx({ card: catalogOf("Token") }))).toBeUndefined();
    expect(stingOf(played(UNIT), ctx()), "no catalog: the plain sounds").toBeUndefined();
  });

  it("R669 a Token that prints a rarity stings with it", () => {
    const card = (): CueCard => ({ type: "Unit", tags: [], rarity: "Token", printedRarity: "Rare" });
    expect(stingOf(played(TOKEN), ctx({ card }))).toMatchObject({ id: "sting", params: { tier: "rare" } });
  });
});

describe("R669 an effect about a unit on the field comes from its lane", () => {
  it("R669 lanePan centres the middle lane and puts the outer ones LANE_PAN_MAX either way", () => {
    expect([0, 1, 2, 3, 4].map((i) => lanePan(i, 5))).toEqual([-LANE_PAN_MAX, -LANE_PAN_MAX / 2, 0, LANE_PAN_MAX / 2, LANE_PAN_MAX]);
    expect(lanePan(0, 1)).toBe(0);
  });

  it("R669 a hit on the opponent's lane-1 unit pans left, the viewer's lane-5 unit's death right, and a hero hit stays centred", () => {
    const enemy = unit("p2", { instanceId: "e1" });
    const mine = unit("p1", { instanceId: "m5" });
    const view = baseView({
      you: emptySide("p1", { units: [null, null, null, null, mine] }),
      opponent: emptySide("p2", { units: [enemy, null, null, null, null] }),
    });
    const hit = cuesFor({ type: "damage", sourceId: "m5", targetId: "e1", amount: 3, combat: true }, ctx({ view }));
    // #57 adds the hit's tier and a variation sample beside the pan; this test is about the pan.
    expect(hit).toEqual([
      { kind: "sfx", id: "impact", params: expect.objectContaining({ amount: 3, pan: -LANE_PAN_MAX }), delayMs: 0 },
    ]);
    const death = cuesFor(destroyed(UNIT, "p1", "m5"), ctx({ view }));
    expect(death.filter((c) => c.kind === "sfx").map((c) => (c.kind === "sfx" ? c.params?.pan : null))).toEqual([LANE_PAN_MAX]);
    expect(death.some((c) => c.kind === "voice"), "a line is never panned, and still speaks").toBe(true);
    const face = cuesFor({ type: "damage", sourceId: "m5", targetId: "hero-p2", amount: 3, combat: true }, ctx({ view }));
    expect(face).toEqual([{ kind: "sfx", id: "impact", params: expect.objectContaining({ amount: 3 }), delayMs: 0 }]);
    expect(face[0]?.kind === "sfx" ? face[0].params?.pan : "missing").toBeUndefined();
    const middle = baseView({ you: emptySide("p1", { units: [null, null, mine, null, null] }) });
    expect(cuesFor(destroyed(UNIT, "p1", "m5"), ctx({ view: middle })).find((c) => c.kind === "sfx")).toEqual({
      kind: "sfx",
      id: "death",
      delayMs: 0,
    });
  });

  it("R669 a unit that has just arrived is found in the newest view", () => {
    const arrived = unit("p1", { instanceId: "c1" });
    const newest = baseView({ you: emptySide("p1", { units: [null, null, null, arrived, null] }) });
    const summon: GameEvent = { type: "summoned", player: "p1", instanceId: "c1", defId: UNIT, row: "units", lane: 4 };
    const cues = cuesFor(summon, ctx({ newestView: () => newest }));
    expect(cues.find((c) => c.kind === "sfx" && c.id === "summon")).toMatchObject({ params: { pan: LANE_PAN_MAX / 2 } });
  });
});

/* --------------------------------------------------------------------------------------------- *
 * MN05 (docs/meditative-set.md M8): Armor's sounds and the niche moments
 * --------------------------------------------------------------------------------------------- */

/** A hit of `amount` with `absorbed` taken off it by Armor, on `targetId`. */
type DamageEvent = Extract<GameEvent, { type: "damage" }>;

function hitOn(targetId: string, amount: number, absorbed?: number): DamageEvent {
  const event: DamageEvent = { type: "damage", sourceId: "s1", targetId, amount, combat: false };
  return absorbed === undefined ? event : { ...event, absorbed };
}

/** The sfx ids an event plays, in order. */
function ids(event: GameEvent, context: CueContext = ctx()): SfxId[] {
  return cuesFor(event, context).flatMap((cue) => (cue.kind === "sfx" ? [cue.id] : []));
}

describe("R1363 Armor clanks when it takes half or more of a hit, and rings when it takes all of it", () => {
  it("R1363 the half threshold is absorbed × 2 >= absorbed + amount, and a hit Armor had no part in never meets it", () => {
    expect(armorTookHalf(2, 2)).toBe(true); // exactly half
    expect(armorTookHalf(3, 2)).toBe(true);
    expect(armorTookHalf(7, 3)).toBe(true);
    expect(armorTookHalf(1, 2)).toBe(false); // a third
    expect(armorTookHalf(2, 3)).toBe(false);
    expect(armorTookHalf(0, 1)).toBe(false);
    expect(armorTookHalf(undefined, 1)).toBe(false);
  });

  it("R1363 a hit Armor took half or more of clanks dully under its impact; less than half, or none, is the plain impact", () => {
    expect(ids(hitOn("hero-p2", 2, 2))).toEqual(["impact", "armorClank"]);
    expect(ids(hitOn("hero-p2", 3, 7))).toEqual(["impact", "armorClank"]);
    expect(ids(hitOn("hero-p2", 3, 2))).toEqual(["impact"]);
    expect(ids(hitOn("hero-p2", 3))).toEqual(["impact"]);
    // A report of 0 dealt is silent, as before.
    expect(ids(hitOn("hero-p2", 0, 3))).toEqual([]);
  });

  it("R1363 a hit Armor took whole rings bright, whatever its size, on either seat", () => {
    const whole = (targetId: string, absorbed: number): GameEvent => ({ type: "damageAbsorbed", sourceId: null, targetId, absorbed, combat: false });
    expect(ids(whole("hero-p1", 1))).toEqual(["armorRing"]);
    expect(ids(whole("u6", 9))).toEqual(["armorRing"]);
    expect(ids(whole("u6", 2), ctx({ view: baseView({ viewer: "p2", you: emptySide("p2"), opponent: emptySide("p1") }) }))).toEqual(["armorRing"]);
  });

  it("R1363 R669 both come from the unit's lane, and a hero's stay centred", () => {
    const guard = unit("p2", { instanceId: "g1", health: 9, maxHealth: 9 });
    const view = baseView({ opponent: emptySide("p2", { units: [guard, null, null, null, null] }) });
    const clank = cuesFor(hitOn("g1", 1, 3), ctx({ view }));
    expect(clank.map((cue) => (cue.kind === "sfx" ? [cue.id, cue.params?.pan] : null))).toEqual([
      ["impact", -LANE_PAN_MAX],
      ["armorClank", -LANE_PAN_MAX],
    ]);
    const ring = cuesFor({ type: "damageAbsorbed", sourceId: null, targetId: "g1", absorbed: 3, combat: true }, ctx({ view }));
    expect(ring).toEqual([{ kind: "sfx", id: "armorRing", params: { pan: -LANE_PAN_MAX }, delayMs: 0 }]);
    const hero = cuesFor({ type: "damageAbsorbed", sourceId: null, targetId: "hero-p2", absorbed: 3, combat: false }, ctx({ view }));
    expect(hero).toEqual([{ kind: "sfx", id: "armorRing", delayMs: 0 }]);
  });

  it("R1363 R203 a hidden source changes nothing: the sound reads the event's numbers alone", () => {
    expect(ids({ type: "damageAbsorbed", sourceId: HIDDEN_DEF_ID, targetId: "u6", absorbed: 4, combat: false })).toEqual(["armorRing"]);
    expect(ids({ ...hitOn("hero-p1", 1, 4), sourceId: HIDDEN_DEF_ID })).toEqual(["impact", "armorClank"]);
  });
});

describe("R1364 a hit that does far more than the health left is overkill", () => {
  it("R1364 overkill is at least OVERKILL_MIN_EXCESS beyond the health, and at least that health beyond it", () => {
    expect(OVERKILL_MIN_EXCESS).toBe(3);
    expect(isOverkill(4, 1)).toBe(true); // 3 beyond 1
    expect(isOverkill(3, 1)).toBe(false); // 2 beyond
    expect(isOverkill(10, 5)).toBe(true); // 5 beyond 5
    expect(isOverkill(9, 5)).toBe(false); // 4 beyond 5
    expect(isOverkill(5, 0)).toBe(false); // already dead: nothing left to overkill
    expect(isOverkill(5, null)).toBe(false);
  });

  it("R1364 the health is the view's, a unit's or a hero's, as the entry was planned against it", () => {
    const small = unit("p2", { instanceId: "s2", health: 2, maxHealth: 3 });
    const view = baseView({ opponent: emptySide("p2", { units: [null, null, small, null, null] }) });
    expect(healthBefore(view, "s2")).toBe(2);
    expect(healthBefore(view, "hero-p1")).toBe(view.you.hero.health);
    expect(healthBefore(view, "gone")).toBeNull();
  });

  it("R1364 an overkill crunches just after the impact; a hit that only kills does not", () => {
    const small = unit("p2", { instanceId: "s2", health: 2, maxHealth: 3 });
    const view = baseView({ opponent: emptySide("p2", { units: [null, null, small, null, null] }) });
    expect(shape(hitOn("s2", 5), ctx({ view }))).toEqual([sfx("impact"), sfx("overkill", OVERKILL_DELAY_MS)].sort());
    expect(ids(hitOn("s2", 4), ctx({ view }))).toEqual(["impact"]);
    expect(ids(hitOn("s2", 2), ctx({ view }))).toEqual(["impact"]);
    // A hero at 3 taking 7 is overkilled too; one at 30 taking 7 is not.
    const low = baseView({ opponent: emptySide("p2", { hero: { ...emptySide("p2").hero, health: 3 } }) });
    expect(ids(hitOn("hero-p2", 7), ctx({ view: low }))).toEqual(["impact", "overkill"]);
    expect(ids(hitOn("hero-p2", 7))).toEqual(["impact"]);
  });
});

describe("R1365 the niche moments each have a sound of their own", () => {
  it("R1365 a crumble, an unlock, a Counter, a fuse, a Nerf and a Buff each play their own sound and no other", () => {
    expect(ids(SAMPLES.crumbled)).toEqual(["crumble"]);
    expect(ids(SAMPLES.unlocked)).toEqual(["unlock"]);
    expect(ids(SAMPLES.locked)).toEqual(["lock"]);
    expect(ids(SAMPLES.countered)).toEqual(["counterspell"]);
    expect(ids(SAMPLES.fused)).toEqual(["fuse"]);
    expect(ids(SAMPLES.degraded)).toEqual(["degrade"]);
    expect(ids(SAMPLES.upgraded)).toEqual(["upgrade"]);
    // Each is that row's alone.
    const own = ["crumble", "unlock", "counterspell", "fuse", "degrade", "upgrade", "armorRing"] as const;
    for (const id of own) {
      const rows = GAME_EVENT_TYPES.filter((type) => SOUND_CUES[type].sfx === id);
      expect(rows, id).toHaveLength(1);
    }
  });

  it("R1365 R440 a Nerf or a Buff sounds the same whatever it changed, on a card the viewer cannot read too", () => {
    const changes = [
      { kind: "cost", delta: 1 },
      { kind: "stats", attack: -2, health: -2 },
      { kind: "keyword", keyword: { kind: "Taunt" }, added: false },
      { kind: "none" },
    ] as const;
    for (const change of changes) {
      expect(ids({ type: "degraded", instanceId: "c1", defId: HIDDEN_DEF_ID, change })).toEqual(["degrade"]);
      expect(ids({ type: "upgraded", instanceId: "c1", defId: UNIT, change })).toEqual(["upgrade"]);
    }
  });

  it("R1365 a card transformed into a Sheep the viewer can read bleats as it comes out of the puff", () => {
    expect(SHEEP_DEF_IDS).toContain(TOKEN);
    expect(shape(SAMPLES.transformed)).toEqual([sfx("poof"), sfx("bleat", BLEAT_DELAY_MS)].sort());
    // Into anything else, the plain puff.
    expect(shape({ type: "transformed", instanceId: "u3", fromDefId: TOKEN, toDefId: UNIT, newInstanceId: "c90" })).toEqual([sfx("poof")]);
  });

  it("R1365 R203 a transform the viewer cannot read never bleats", () => {
    const hidden: GameEvent = { type: "transformed", instanceId: HIDDEN_DEF_ID, fromDefId: HIDDEN_DEF_ID, toDefId: HIDDEN_DEF_ID, newInstanceId: HIDDEN_DEF_ID };
    expect(shape(hidden)).toEqual([sfx("poof")]);
  });
});

describe("R1366 a steal and a give", () => {
  type ControlEvent = Extract<GameEvent, { type: "controlChanged" }>;
  const control = (controller: PlayerId, how?: "steal" | "give"): ControlEvent => {
    const event: ControlEvent = { type: "controlChanged", instanceId: "u6", controller, row: "units", lane: 2 };
    return how === undefined ? event : { ...event, how };
  };

  it("R1366 a card taken into a hand is always a steal", () => {
    expect(ids(SAMPLES.stolen)).toEqual(["steal"]);
    expect(ids({ type: "stolen", instanceId: HIDDEN_DEF_ID, defId: HIDDEN_DEF_ID, from: "p1", to: "p2", zone: "library" })).toEqual(["steal"]);
  });

  it("R1366 a unit taken is a steal and one handed over a give, whichever seat it goes to and whoever's turn it is", () => {
    for (const controller of ["p1", "p2"] as const) {
      for (const view of [baseView(), baseView({ active: "p2" })]) {
        expect(ids(control(controller, "steal"), ctx({ view }))).toEqual(["steal"]);
        expect(ids(control(controller, "give"), ctx({ view }))).toEqual(["give"]);
      }
    }
  });

  it("R1366 a card a board move carries across (a swap, a rotation, a rollback) keeps the plain whoosh", () => {
    expect(ids(control("p1"))).toEqual(["whoosh"]);
    expect(ids({ type: "controlChanged", instanceId: HIDDEN_DEF_ID, controller: "p2", row: "backrow", lane: 1, how: "steal" })).toEqual(["steal"]);
  });

  it("R1366 R669 a unit changing sides is heard from its lane", () => {
    const moved = unit("p2", { instanceId: "u6" });
    const view = baseView({ opponent: emptySide("p2", { units: [null, null, null, null, moved] }) });
    expect(cuesFor(control("p1", "steal"), ctx({ view }))).toEqual([{ kind: "sfx", id: "steal", params: { pan: LANE_PAN_MAX }, delayMs: 0 }]);
  });
});
