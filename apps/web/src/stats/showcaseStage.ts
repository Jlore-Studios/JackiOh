// R1584000: the one-card stage the landing's stat showcase plays a board animation on.
//
// It makes no animation of its own. A stat names the board event its card would have taken part in
// (a unit destroyed, a mark and a steal, a trap fired, ...); the stage hands that event to the
// board's own planner (`planEntries`, game/animations.ts) against a view with that card on one lane,
// and the entry it gets back is what the board would play: its duration, the `data-animating` type
// animations.css keys a keyframe on, and the cues the effects layer's `planFx` draws. The stage's
// elements carry the board's testids, so the entry's frames land on them unchanged.
//
// It decides no rule and keeps no state (CLAUDE.md rule 7): the events are made here, for show, and
// nothing reads them back. `planStage` (fx/stage.ts) is left out: it acts on the board's elements,
// and the landing has none.
//
// R2648000: `cast` is the board's `cardPlayed` entry (`jk-card-played`), for a Spell's lunge stat.
//
// R2649000: fifteen more, each a board event whose target is the card on the field. `crumble`, `expire`
// and `inTime` lead with the aura the board's card wears before what it warns of (`aura`): Brittle's
// last count in gray (#2270), Temporary's in fuchsia (#2437), as Allure's mark leads its steal.

import { CATALOG } from "@jackioh/cards";
import type { GameEvent, PlayerId, PlayerView, SideView, UnitView } from "@jackioh/shared";

import {
  ALLURE_MARK,
  ALLURE_MARK_COLOR,
  BRITTLE_LAST_COLOR,
  DESTROY_MARK,
  DESTROY_MARK_COLOR,
  FROZEN_MARK,
  FROZEN_MARK_COLOR,
  TEMPORARY_LAST_COLOR,
  type MarkColor,
} from "../cards/marks.ts";
import { planFx } from "../fx/cues.ts";
import { catalogFacts } from "../fx/FxLayer.tsx";
import type { FxCue, FxMemory } from "../fx/types.ts";
import { planEntries, type AnimationEntry } from "../game/animations.ts";
import { lookupFromDefs } from "../game/catalog.ts";
import type { Side } from "../game/contract.ts";
import { HERO_HEALTH } from "../wire/engineConfig.ts";
import { SHOWCASE_BEAT_GAP_MS, SHOWCASE_LEAD_MS } from "./config.ts";

/** The one card's instance id; the stage's card is `testid.card(STAGE_INSTANCE_ID)`. */
export const STAGE_INSTANCE_ID = "landing-stat";

/** The lane the card stands in, on whichever side holds it. */
export const STAGE_LANE = 0;

const STAGE_TURN = 1;

const STAGE_PLAYERS: Readonly<Record<Side, PlayerId>> = { you: "p1", opponent: "p2" };

/** animations.css's `--lunge-dir` for an opponent's unit: its lunge goes down the screen. */
export const OPPONENT_LUNGE_DIR = 1;

const LOOKUP = lookupFromDefs(CATALOG);

/** R2649000: the stage's hit, heal and Armor's absorb, a token number the event needs (the stage shows none). */
const STAGE_AMOUNT = 1;

/** R2649000: `stamp`'s summons, one after another like a production line. */
const STAMP_BEATS = 3;

/** R2649000: `windfury`'s lunges, Windfury's two attacks (R636). */
const WINDFURY_BEATS = 2;

/** The board animations a stat can ask for. */
export const STAGE_ANIMATIONS = [
  "death",
  "allure",
  "trap",
  "entrance",
  "lunge",
  "radiant",
  "shield",
  "cast",
  "crumble",
  "expire",
  "inTime",
  "doom",
  "stamp",
  "exile",
  "bounce",
  "hit",
  "heal",
  "armor",
  "reborn",
  "windfury",
  "freeze",
  "thaw",
  "launch",
] as const;
export type StageAnimation = (typeof STAGE_ANIMATIONS)[number];

/** R2649000: the animations whose card leaves the stage and comes back once they have played. */
export const RETURNS: ReadonlySet<StageAnimation> = new Set<StageAnimation>(["death", "crumble", "expire", "doom", "exile", "bounce"]);

/** R2649000: the aura the board's card wears before what it warns of: Brittle's last count (#2270), Temporary's (#2437). */
const AURAS: Partial<Record<StageAnimation, MarkColor>> = {
  crumble: BRITTLE_LAST_COLOR,
  expire: TEMPORARY_LAST_COLOR,
  inTime: TEMPORARY_LAST_COLOR,
};

/** One animation of a stat's card: the board's entry, and the side whose zone holds the card while it plays. */
export type StageBeat = { readonly entry: AnimationEntry; readonly side: Side };

/**
 * What a stat's card plays, in order. A card that leaves comes back (`returns`) once it has. R2649000:
 * `aura`, when set, is drawn on the card from its way in until the first beat plays.
 */
export type StageScript = {
  readonly side: Side;
  readonly beats: readonly StageBeat[];
  readonly returns: boolean;
  readonly aura?: MarkColor;
};

function stageSide(player: PlayerId, unit: UnitView | null): SideView {
  return {
    player,
    hero: { health: HERO_HEALTH, armor: 0, powers: [], power: null },
    modifiers: [],
    mana: { current: 0, max: 0 },
    hand: [],
    libraryCount: 0,
    graveyard: [],
    exile: [],
    resolving: [],
    units: [unit],
    backrow: [null],
    locks: { units: [false], backrow: [false] },
    reserved: { units: [false], backrow: [false] },
    fatigueCount: 0,
  };
}

/** A view of two one-lane sides, `side`'s holding the card as a unit. */
export function stageView(defId: string, side: Side): PlayerView {
  const player = STAGE_PLAYERS[side];
  const base = CATALOG[defId]?.base;
  const unit: UnitView = {
    instanceId: STAGE_INSTANCE_ID,
    defId,
    radiant: false,
    cost: 0,
    owner: player,
    controller: player,
    attack: base?.attack ?? 0,
    maxHealth: base?.health ?? 0,
    health: base?.health ?? 0,
    keywords: [],
    armor: 0,
    position: "ATK",
    counters: {},
    buried: 0,
    canAct: false,
  };
  const you = stageSide("p1", side === "you" ? unit : null);
  const opponent = stageSide("p2", side === "opponent" ? unit : null);
  return {
    viewer: "p1",
    turn: STAGE_TURN,
    active: "p1",
    phase: "main",
    you,
    opponent: { ...opponent, hand: { count: 0 } },
    pending: null,
    events: [],
    result: null,
    clockMs: null,
  };
}

/** The board events a stat's animation is made of, each with the side whose zone the card is in when it plays. */
function stageEvents(animation: StageAnimation, defId: string, side: Side): readonly (readonly [GameEvent, Side])[] {
  const other: Side = side === "you" ? "opponent" : "you";
  const p = STAGE_PLAYERS[side];
  const q = STAGE_PLAYERS[other];
  const id = STAGE_INSTANCE_ID;
  switch (animation) {
    case "death":
      return [[{ type: "destroyed", instanceId: id, defId, owner: p, controller: p, attack: 0, maxHealth: 0, killerId: null }, side]];
    case "allure":
      return [
        [{ type: "marked", instanceId: id, mark: ALLURE_MARK, color: ALLURE_MARK_COLOR, added: true }, side],
        [{ type: "controlChanged", instanceId: id, controller: q, row: "units", lane: STAGE_LANE, how: "steal" }, other],
      ];
    case "trap":
      return [[{ type: "trapFired", instanceId: id, defId, controller: p, row: "units", lane: STAGE_LANE }, side]];
    case "entrance":
      return [[{ type: "summoned", player: p, instanceId: id, defId, row: "units", lane: STAGE_LANE }, side]];
    case "lunge":
      return [[{ type: "attackDeclared", attackerId: id, targetId: `hero-${q}`, forced: false }, side]];
    case "radiant":
      return [[{ type: "radiantSet", instanceId: id, defId, zone: { z: "field", player: p, row: "units", lane: STAGE_LANE } }, side]];
    case "shield":
      return [[{ type: "divineShieldLost", instanceId: id }, side]];
    case "cast":
      // R2648000: the viewer's seat plays it on either side, so the board's target (the card where the
      // view shows it, `locateInstance`) is the stage's card, never a hand the stage does not draw.
      return [[{ type: "cardPlayed", player: STAGE_PLAYERS.you, instanceId: id, defId, costPaid: 0 }, side]];
    // R2649000: the game's own mechanics, each on the card where it stands.
    case "crumble":
      return [[{ type: "crumbled", instanceId: id, defId, owner: p, zone: "field" }, side]];
    case "expire":
    case "exile":
      // The board's `discarded` aims at a hand the stage does not draw; `exiled` fades the card where it stands.
      return [[{ type: "exiled", instanceId: id, defId, owner: p }, side]];
    case "inTime":
      return stageEvents("cast", defId, side);
    case "doom":
      // Classic #20's mark for death (R437), then the death, as Allure's mark leads its steal.
      return [[{ type: "marked", instanceId: id, mark: DESTROY_MARK, color: DESTROY_MARK_COLOR, added: true }, side], ...stageEvents("death", defId, side)];
    case "stamp":
      return Array.from({ length: STAMP_BEATS }).flatMap(() => stageEvents("entrance", defId, side));
    case "bounce":
      return [[{ type: "bounced", instanceId: id, defId, owner: p }, side]];
    case "hit":
      return [[{ type: "damage", sourceId: null, targetId: id, amount: STAGE_AMOUNT, combat: false }, side]];
    case "heal":
      return [[{ type: "healed", targetId: id, amount: STAGE_AMOUNT }, side]];
    case "armor":
      return [[{ type: "damageAbsorbed", sourceId: null, targetId: id, absorbed: STAGE_AMOUNT, combat: false }, side]];
    case "reborn":
      return [...stageEvents("death", defId, side), ...stageEvents("entrance", defId, side)];
    case "windfury":
      return Array.from({ length: WINDFURY_BEATS }).flatMap(() => stageEvents("lunge", defId, side));
    case "freeze":
      return [[{ type: "marked", instanceId: id, mark: FROZEN_MARK, color: FROZEN_MARK_COLOR, added: true }, side]];
    case "thaw":
      return [
        ...stageEvents("freeze", defId, side),
        [{ type: "marked", instanceId: id, mark: FROZEN_MARK, color: FROZEN_MARK_COLOR, added: false }, side],
      ];
    case "launch":
      return [[{ type: "starshipLaunched", player: p, instanceId: id, defId, name: CATALOG[defId]?.name ?? defId, pieces: [defId] }, side]];
  }
}

/** The beats a card on `side` plays for `animation`: the board's entries for the events a stat stands for. */
export function stageScript(animation: StageAnimation, defId: string, side: Side): StageScript {
  const view = stageView(defId, side);
  const beats = stageEvents(animation, defId, side).flatMap(([event, beatSide]) =>
    planEntries([event], view, false).map((entry) => ({ entry, side: beatSide })),
  );
  return { side, beats, returns: RETURNS.has(animation), aura: AURAS[animation] };
}

/** R2649000: when a script's first beat plays: after the way in, and a beat's gap more when an aura leads it, so the warning reads first. */
export function firstBeatAt(script: StageScript): number {
  return SHOWCASE_LEAD_MS + (script.aura === undefined ? 0 : SHOWCASE_BEAT_GAP_MS);
}

/** The effects layer's cues for one beat: what FxLayer plays for an entry, less the board's stage cues. */
export function stageCues(beat: StageBeat, intensity: number, memory: FxMemory): FxCue[] {
  memory.remember(beat.entry.events);
  return planFx(beat.entry, beat.entry.view, { intensity, card: (defId) => catalogFacts(LOOKUP, defId), memory });
}
