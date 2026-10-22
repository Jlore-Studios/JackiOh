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

import { CATALOG } from "@jackioh/cards";
import type { GameEvent, PlayerId, PlayerView, SideView, UnitView } from "@jackioh/shared";

import { ALLURE_MARK, ALLURE_MARK_COLOR } from "../cards/marks.ts";
import { planFx } from "../fx/cues.ts";
import { catalogFacts } from "../fx/FxLayer.tsx";
import type { FxCue, FxMemory } from "../fx/types.ts";
import { planEntries, type AnimationEntry } from "../game/animations.ts";
import { lookupFromDefs } from "../game/catalog.ts";
import type { Side } from "../game/contract.ts";
import { HERO_HEALTH } from "../wire/engineConfig.ts";

/** The one card's instance id; the stage's card is `testid.card(STAGE_INSTANCE_ID)`. */
export const STAGE_INSTANCE_ID = "landing-stat";

/** The lane the card stands in, on whichever side holds it. */
export const STAGE_LANE = 0;

const STAGE_TURN = 1;

const STAGE_PLAYERS: Readonly<Record<Side, PlayerId>> = { you: "p1", opponent: "p2" };

/** animations.css's `--lunge-dir` for an opponent's unit: its lunge goes down the screen. */
export const OPPONENT_LUNGE_DIR = 1;

const LOOKUP = lookupFromDefs(CATALOG);

/** The board animations a stat can ask for. */
export const STAGE_ANIMATIONS = ["death", "allure", "trap", "entrance", "lunge", "radiant", "shield"] as const;
export type StageAnimation = (typeof STAGE_ANIMATIONS)[number];

/** One animation of a stat's card: the board's entry, and the side whose zone holds the card while it plays. */
export type StageBeat = { readonly entry: AnimationEntry; readonly side: Side };

/** What a stat's card plays, in order. A card that dies comes back (`returns`) once it has. */
export type StageScript = { readonly side: Side; readonly beats: readonly StageBeat[]; readonly returns: boolean };

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
  }
}

/** The beats a card on `side` plays for `animation`: the board's entries for the events a stat stands for. */
export function stageScript(animation: StageAnimation, defId: string, side: Side): StageScript {
  const view = stageView(defId, side);
  const beats = stageEvents(animation, defId, side).flatMap(([event, beatSide]) =>
    planEntries([event], view, false).map((entry) => ({ entry, side: beatSide })),
  );
  return { side, beats, returns: animation === "death" };
}

/** The effects layer's cues for one beat: what FxLayer plays for an entry, less the board's stage cues. */
export function stageCues(beat: StageBeat, intensity: number, memory: FxMemory): FxCue[] {
  memory.remember(beat.entry.events);
  return planFx(beat.entry, beat.entry.view, { intensity, card: (defId) => catalogFacts(LOOKUP, defId), memory });
}
