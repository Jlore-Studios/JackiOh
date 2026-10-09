// Turning clicks into actions — and nothing else (CLAUDE.md rule 7, BUILD M5-T2, SPEC §10.2).
//
// THE ONE RULE OF THIS FILE IS THAT IT HAS NO RULES. Every question of the form "may I?" is
// answered by searching the `ActionBody[]` that `legalActions(state, player)` handed us. There is
// no `cost <= mana`, no `keywords.includes("Taunt")`, no "a zone is open when it is null" and no
// summoning-sickness check anywhere below:
//
//   * a hand card is playable because a `play` naming it is in the array, and so is a card in the
//     viewer's graveyard (B5 E11: a permission lets `legalActions` list it; its pile's "Play");
//   * a zone is legal because some candidate `play` names that row and lane;
//   * a unit may attack because an `attack` names it as `attackerId`, and it may hit a target
//     because a candidate names that `targetId`;
//   * a unit may switch because a `switchPosition` names it;
//   * a card's Activate control is live because an `activate` names the card (and its ability),
//     and a Heroic Power's because an `activatePower` (or an `activate`) names it (R384, R43);
//   * `end-turn`, `offer-draw` and `concede` light up because the matching action type is in the
//     array;
//   * the green glow (`Highlight.glow`) is a subset of those testids, chosen by action type and
//     interaction stage alone (see `highlightFor`). The yellow glow is not decided here at all: it
//     is `CardView.conditionActive`, which the engine computes (R195).
//
// `min` and `max` on a picker come from `PendingView`, which the engine built. The engine decides;
// this module narrows a list it was given.
//
// R81: zone, X, embiggen, Tribute and a card's declared targets and modes are NOT prompts. They
// travel inside the `play` action, and `legalActions` enumerates them; so does a play's new payment,
// the Plague Counters that pay part of a graveyard play's price (`plague`, Classic #74) — while a
// targeting cost's discards (Classic #89) are random at pay time (R682) and travel nowhere. R384: an
// activation is built the same way — its targets, modes and Tribute travel in the `activate`
// action — so a play and an activation are one "build" here (`BuildBody`), narrowed by one set of
// functions. Everything chosen during resolution — Discover, chained steps, Echo repeats, casts,
// triggers, the mulligan — arrives as a `PendingChoice` and is answered with an `answer` action (or,
// for the mulligan, its own `mulligan` action).
//
// R391 (B4.5): a Tribute may pay for its own zone, and `legalActions` pairs each zone with the
// Tribute sets that leave it open. Narrowing keeps whole candidates, never a field on its own, so a
// zone and a Tribute picked in either order only ever finish a pair the engine listed.
//
// CARRY_THROUGH: a candidate that leaves `tributes`, `targets` or `modes` unset accepts what a
// *picker* chose for it (`pickInPlay`), carried into the emitted action verbatim and validated by
// the engine per R90: the client reports the player's pick, it does not rule on it. A *board
// click* is never carried through — a click no candidate accounts for changes nothing, so the board
// can never be clicked into an illegal play. The `plague` payment is never carried: a candidate
// without it pays none, which is what the engine listed it to mean.

import type {
  Action,
  ActionBody,
  ActivationView,
  PendingOption,
  PendingView,
  PlayerId,
  PlayerView,
  RevealAt,
  Row,
  Selection,
  ZoneChoice,
} from "@jackioh/shared";
import { RANDOM_ATTACK_TARGET } from "@jackioh/engine/config";

import {
  LANES,
  NO_HIGHLIGHT,
  namedAbility,
  playerOf,
  sideOf,
  testid,
  type BoardControl,
  type ClickTarget,
  type Highlight,
} from "./contract.ts";

type PlayBody = Extract<ActionBody, { type: "play" }>;
type AttackBody = Extract<ActionBody, { type: "attack" }>;
type ActivateBody = Extract<ActionBody, { type: "activate" }>;
type PowerBody = Extract<ActionBody, { type: "activatePower" }>;
/** R384: an activation, of a card's Activate ability or (its alias) of a Heroic Power. */
export type ActivationBody = ActivateBody | PowerBody;
/** An action the client builds choice by choice from its candidates (R81, R384). */
export type BuildBody = PlayBody | ActivationBody;

/** B5 E11, E19: Plague Counters paying part of a graveyard play's price (Classic #74). */
export type PlagueSpend = NonNullable<PlayBody["plague"]>;
/** A plague payment as the player picks it: tokens off a card, or none (the whole price in mana). */
export type PlagueChoice = PlagueSpend | "none";

/**
 * ME-ALTPLAY (R1040, R1044): how the card is played — face-up, or face-down as a Trap revealing
 * at the chosen timing.
 */
export type FaceDownChoice = RevealAt | "up";

/** The R81 play-time choices (and an activation's), as the client accumulates them. */
export type PlayBuild = {
  zone?: ZoneChoice;
  x?: number;
  embiggen?: boolean;
  tributes?: string[];
  targets?: Selection[];
  modes?: string[];
  /** B5 E11, E19: how a graveyard play's price is paid in Plague Counters. */
  plague?: PlagueChoice;
  /** ME-ALTPLAY: face-up, or the reveal timing it is set with. */
  faceDown?: FaceDownChoice;
};

type Playing = { stage: "playing"; instanceId: string; candidates: ActionBody[]; picked: Partial<PlayBuild> };
/** R384: an Activate ability (or a Heroic Power) being built; `ability` as the control named it. */
type Activating = {
  stage: "activating";
  instanceId: string;
  ability?: string;
  candidates: ActionBody[];
  picked: Partial<PlayBuild>;
};
type Building = Playing | Activating;

/** The client's in-progress selection. Not state the engine knows or cares about. */
export type Interaction =
  | { stage: "idle" }
  | Playing
  | Activating
  | { stage: "attacking"; attackerId: string; candidates: ActionBody[] };

export const IDLE: Interaction = { stage: "idle" };

/** A play or an activation is being built: the stages whose choices `outstandingNeed` reads. */
export function isBuilding(interaction: Interaction): interaction is Building {
  return interaction.stage === "playing" || interaction.stage === "activating";
}

/** What is still unchosen about the play in flight, derived only from the candidates. */
export type PlayNeed =
  | { kind: "faceDown"; min: number; max: number; options: FaceDownChoice[] }
  | { kind: "zone"; min: number; max: number; zones: ZoneChoice[] }
  | { kind: "x"; min: number; max: number; values: number[] }
  | { kind: "embiggen"; min: number; max: number; values: boolean[] }
  | { kind: "plague"; min: number; max: number; options: PlagueChoice[] }
  | { kind: "tribute"; min: number; max: number; instanceIds: string[] }
  | { kind: "target"; min: number; max: number; selections: Selection[] }
  | { kind: "mode"; min: number; max: number; options: string[] };

export type ClickResult = { interaction: Interaction; action?: ActionBody };

// ---------------------------------------------------------------------------------------------
// Keys. Presentation-only identity for a choice, so a picker can round-trip it through the DOM.
// ---------------------------------------------------------------------------------------------

export function zoneKey(zone: ZoneChoice): string {
  return `${zone.row}:${zone.lane}`;
}

export function parseZoneKey(key: string): ZoneChoice | null {
  const [row, lane] = key.split(":");
  if (row !== "units" && row !== "backrow") return null;
  const n = Number(lane);
  if (lane === undefined || lane === "" || !Number.isInteger(n)) return null;
  return { row, lane: n };
}

export function selectionKey(selection: Selection): string {
  switch (selection.pick) {
    case "instance":
      return `instance:${selection.instanceId}`;
    case "hero":
      return `hero:${selection.player}`;
    case "zone":
      return `zone:${selection.player}:${selection.row}:${selection.lane}`;
    case "mode":
      return `mode:${selection.option}`;
    case "craft":
      return `craft:${JSON.stringify(selection.recipe)}`;
    case "none":
      return "none";
  }
}

/** A plague payment's identity: "none", or the card and the count (JSON, so no id can collide). */
export function plagueKey(choice: PlagueChoice | undefined): string {
  return choice === undefined || choice === "none" ? "none" : JSON.stringify([choice.from, choice.tokens]);
}

function listKey(values: readonly string[]): string {
  return JSON.stringify([...values].sort());
}

function targetsKey(selections: readonly Selection[]): string {
  return listKey(selections.map(selectionKey));
}

// ---------------------------------------------------------------------------------------------
// Searching the legal array. Nothing here knows a rule; it all reads `ActionBody`s.
// ---------------------------------------------------------------------------------------------

function isPlay(body: ActionBody): body is PlayBody {
  return body.type === "play";
}

function isAttack(body: ActionBody): body is AttackBody {
  return body.type === "attack";
}

export function isActivation(body: ActionBody): body is ActivationBody {
  return body.type === "activate" || body.type === "activatePower";
}

function isBuildBody(body: ActionBody): body is BuildBody {
  return isPlay(body) || isActivation(body);
}

function playsFor(legal: readonly ActionBody[], instanceId: string): PlayBody[] {
  return legal.filter(isPlay).filter((body) => body.instanceId === instanceId);
}

function attacksBy(legal: readonly ActionBody[], attackerId: string): AttackBody[] {
  return legal.filter(isAttack).filter((body) => body.attackerId === attackerId);
}

/**
 * R384: the activations an Activate control stands for. A control that names an ability (a card
 * listing several) takes that ability's `activate`s; one that names none takes every `activate` of
 * the card and, for a Heroic Power, its `activatePower` (R43's alias, B3.2 rule 10).
 */
export function activationsFor(legal: readonly ActionBody[], instanceId: string, ability?: string): ActivationBody[] {
  return legal.filter(isActivation).filter((body) => {
    if (body.instanceId !== instanceId) return false;
    if (body.type === "activatePower") return ability === undefined;
    return ability === undefined || body.ability === ability;
  });
}

/**
 * How an `attack` names a hero: the literal string `hero-<playerId>`, e.g. `"hero-p2"`.
 * Learned from the engine's combat tests, `crates/engine/tests/rules/combat_validation.rs` (a
 * refused attack on `"hero-p1"` reads `"no target hero-p1"`) and
 * `combat_positions.rs` (every face attack sends `targetId: "hero-p2"`), and from
 * `crates/engine/src/damage.rs`, whose `target_id()` builds the same id for the `damage` and
 * `healed` events. There is no exported helper in `@jackioh/shared` for
 * it — reported as an M5-T2 finding — so this is the client's copy of that one string.
 */
export function heroTargetId(player: PlayerId): string {
  return `hero-${player}`;
}

/** Decodes an `attack` target back to a player, tolerating a bare player id defensively. */
export function heroPlayerOfTargetId(view: PlayerView, targetId: string): PlayerId | null {
  for (const side of [view.you, view.opponent]) {
    if (targetId === heroTargetId(side.player) || targetId === side.player) return side.player;
  }
  return null;
}

/** The `data-testid` an `attack`'s `targetId` points at. */
export function attackTargetTestid(view: PlayerView, targetId: string): string {
  const player = heroPlayerOfTargetId(view, targetId);
  return player === null ? testid.card(targetId) : testid.hero(sideOf(view, player));
}

/** Where a `Selection` lives on the board. `mode` and `none` point at nothing. */
export function selectionTestid(view: PlayerView, selection: Selection): string | null {
  switch (selection.pick) {
    case "instance": {
      const hand = Array.isArray(view.you.hand) ? view.you.hand : [];
      return hand.some((card) => card.instanceId === selection.instanceId)
        ? testid.handCard(selection.instanceId)
        : testid.card(selection.instanceId);
    }
    case "hero":
      return testid.hero(sideOf(view, selection.player));
    case "zone":
      return testid.zone(sideOf(view, selection.player), selection.row, selection.lane);
    case "mode":
    case "craft":
    case "none":
      return null;
  }
}

/** B5 E11: is this card in the viewer's own graveyard (where a permission may let it be played)? */
function inOwnGraveyard(view: PlayerView, instanceId: string): boolean {
  return view.you.graveyard.some((card) => card.instanceId === instanceId);
}

/**
 * The element a `play` is started from: the hand card, or the "Play" on a card in the viewer's
 * graveyard pile (B5 E11). A card the view places nowhere is taken for a hand card.
 */
export function playSourceTestid(view: PlayerView, instanceId: string): string {
  return inOwnGraveyard(view, instanceId) ? testid.pilePlay(instanceId) : testid.handCard(instanceId);
}

/**
 * R384: the abilities the view lists on a card, wherever on the field it stands (a stolen card
 * may sit in the other seat's backrow, R33), or null when the view names no such card.
 */
export function activationsOnField(view: PlayerView, instanceId: string): readonly ActivationView[] | null {
  for (const side of [view.you, view.opponent]) {
    for (const pile of side.units) {
      if (pile !== null && pile.instanceId === instanceId) return pile.activations ?? null;
    }
    for (const slot of side.backrow) {
      if (slot !== null && !slot.faceDown && slot.instanceId === instanceId) return slot.activations ?? null;
    }
  }
  return null;
}

/**
 * R384, R510: every control an activation lights. A Heroic Power the viewer controls is its hero
 * panel button (`power` for the first, `power-<id>` for the rest); a card listing `activations` is
 * its own Activate control. A body the view places nowhere lights the control its type has always
 * meant: `power` for `activatePower`, `activate-<id>` for `activate`.
 */
export function activationControlTestids(view: PlayerView, body: ActivationBody): string[] {
  const out: string[] = [];
  const powers = view.you.hero.powers ?? [];
  if (view.you.hero.power?.instanceId === body.instanceId) out.push(testid.power);
  else if (powers.some((power) => power.instanceId === body.instanceId)) out.push(testid.powerOf(body.instanceId));
  if (body.type === "activate") {
    const listed = activationsOnField(view, body.instanceId);
    if (listed !== null && listed.length > 0) {
      const ability = body.ability === undefined ? undefined : namedAbility(listed.length, body.ability);
      out.push(testid.activate(body.instanceId, ability));
    }
  }
  if (out.length === 0) out.push(body.type === "activatePower" ? testid.power : testid.activate(body.instanceId));
  return out;
}

// ---------------------------------------------------------------------------------------------
// Narrowing a build. `picked` is what the player has said; candidates are what the engine allows.
// ---------------------------------------------------------------------------------------------

/** Every choice field a build body can carry, read the same way off a play or an activation. */
type BuildFields = {
  zone?: ZoneChoice;
  x?: number;
  embiggen?: boolean;
  tributes?: string[];
  targets?: Selection[];
  modes?: string[];
  plague?: PlagueSpend;
  /** ME-ALTPLAY: the reveal timing a set play carries, absent face-up. */
  faceDown?: RevealAt;
};

function fieldsOf(body: BuildBody): BuildFields {
  return body;
}

function containsAll(fixed: readonly string[], wanted: readonly string[]): boolean {
  const have = new Set(fixed);
  return wanted.every((value) => have.has(value));
}

/**
 * A candidate still matches when every field it fixes equals what the player picked. A field the
 * candidate leaves unset is carried through (CARRY_THROUGH) rather than treated as a refusal — except
 * the payments, which a candidate without them pays none of.
 */
function matches(candidate: BuildBody, picked: Partial<PlayBuild>): boolean {
  const fixed = fieldsOf(candidate);
  // ME-ALTPLAY: face-up is the absence of a timing, compared strictly.
  if (picked.faceDown !== undefined && (fixed.faceDown ?? "up") !== picked.faceDown) return false;
  // A picked zone excludes a candidate that fixes none: a board click never carries through.
  if (picked.zone !== undefined) {
    if (fixed.zone === undefined || zoneKey(fixed.zone) !== zoneKey(picked.zone)) return false;
  }
  if (picked.x !== undefined && fixed.x !== undefined && fixed.x !== picked.x) return false;
  if (picked.embiggen !== undefined && fixed.embiggen !== undefined && fixed.embiggen !== picked.embiggen) {
    return false;
  }
  if (picked.tributes !== undefined && fixed.tributes !== undefined) {
    if (!containsAll(fixed.tributes, picked.tributes)) return false;
  }
  // ME-ALTPLAY: a set play declares nothing, so a picked target or mode drops it.
  if (fixed.faceDown !== undefined && (picked.targets !== undefined || picked.modes !== undefined)) {
    return false;
  }
  if (picked.targets !== undefined && fixed.targets !== undefined) {
    if (!containsAll(fixed.targets.map(selectionKey), picked.targets.map(selectionKey))) return false;
  }
  if (picked.modes !== undefined && fixed.modes !== undefined) {
    if (!containsAll(fixed.modes, picked.modes)) return false;
  }
  if (picked.plague !== undefined && plagueKey(fixed.plague) !== plagueKey(picked.plague)) return false;
  return true;
}

/** A list that is empty is no list at all: the engine never lists one, and neither does the client. */
function nonEmpty<T>(values: readonly T[] | undefined): T[] | undefined {
  return values === undefined || values.length === 0 ? undefined : [...values];
}

/**
 * The emitted body is the candidate the engine listed, every field it carries kept, plus only the
 * choices it left unset (CARRY_THROUGH): never a field its action type does not have.
 */
function mergePicked(candidate: BuildBody, picked: Partial<PlayBuild>): BuildBody {
  const tributes = nonEmpty(candidate.type === "activatePower" ? undefined : (candidate.tributes ?? picked.tributes));
  const targets = nonEmpty(candidate.targets ?? picked.targets);
  const modes = nonEmpty(candidate.type === "activatePower" ? undefined : (candidate.modes ?? picked.modes));
  switch (candidate.type) {
    case "play": {
      const { tributes: _t, targets: _g, modes: _m, ...rest } = candidate;
      const body: PlayBody = { ...rest };
      // ME-ALTPLAY: a picked zone is never added to a candidate that fixes none.
      const zone = candidate.zone;
      const x = candidate.x ?? picked.x;
      const embiggen = candidate.embiggen ?? picked.embiggen;
      if (zone !== undefined) body.zone = zone;
      if (x !== undefined) body.x = x;
      if (embiggen !== undefined) body.embiggen = embiggen;
      if (tributes !== undefined) body.tributes = tributes;
      if (targets !== undefined) body.targets = targets;
      if (modes !== undefined) body.modes = modes;
      return body;
    }
    case "activate": {
      const { tributes: _t, targets: _g, modes: _m, ...rest } = candidate;
      const body: ActivateBody = { ...rest };
      if (tributes !== undefined) body.tributes = tributes;
      if (targets !== undefined) body.targets = targets;
      if (modes !== undefined) body.modes = modes;
      return body;
    }
    case "activatePower": {
      const { targets: _g, ...rest } = candidate;
      const body: PowerBody = { ...rest };
      if (targets !== undefined) body.targets = targets;
      return body;
    }
  }
}

function remainingCandidates(interaction: Building): BuildBody[] {
  return interaction.candidates.filter(isBuildBody).filter((candidate) => matches(candidate, interaction.picked));
}

function distinctBy<T>(values: readonly T[], key: (value: T) => string): T[] {
  const out = new Map<string, T>();
  for (const value of values) out.set(key(value), value);
  return [...out.values()];
}

/**
 * The next choice the build still needs, or null when the candidates agree on everything. Derived
 * purely from the candidate array: two candidates that differ only in `x` mean the player must
 * pick an X, and nothing else. Asked in cost order (X, embiggen and the Plague Counters change what
 * is paid), then the Tribute, the targets, the modes, and the board-driven zone last so a zone
 * click finishes the play — except that the modes come before the targets when some candidate
 * wants no target at all: a target-first order would force a target pick and strand the targetless
 * mode where no click can reach it (Classic #20's Discard, which is shown but could never be
 * selected).
 */
export function outstandingNeed(interaction: Interaction): PlayNeed | null {
  if (!isBuilding(interaction)) return null;
  const remaining = remainingCandidates(interaction);
  if (remaining.length < 2) return null;
  const fields = remaining.map(fieldsOf);

  // ME-ALTPLAY: how it is played is asked before anything else, face-up first.
  if (interaction.picked.faceDown === undefined) {
    const options = distinctBy(
      fields.map((c): FaceDownChoice => c.faceDown ?? "up"),
      (choice) => choice,
    );
    options.sort((a, b) => (a === "up" ? -1 : b === "up" ? 1 : 0));
    if (options.length > 1) return { kind: "faceDown", min: 1, max: 1, options };
  }

  if (interaction.picked.x === undefined) {
    const values = [...new Set(fields.flatMap((c) => (c.x === undefined ? [] : [c.x])))].sort((a, b) => a - b);
    if (values.length > 1) return { kind: "x", min: 1, max: 1, values };
  }

  if (interaction.picked.embiggen === undefined) {
    const values = [...new Set(fields.flatMap((c) => (c.embiggen === undefined ? [] : [c.embiggen])))].sort(
      (a, b) => Number(a) - Number(b),
    );
    if (values.length > 1) return { kind: "embiggen", min: 1, max: 1, values };
  }

  if (interaction.picked.plague === undefined) {
    // In the engine's order: mana alone first, then each card's counts from fewest tokens up.
    const options = distinctBy(
      fields.map((c): PlagueChoice => c.plague ?? "none"),
      plagueKey,
    );
    if (options.length > 1) return { kind: "plague", min: 1, max: 1, options };
  }

  const tributeSets = distinctBy(
    fields.flatMap((c) => (c.tributes === undefined ? [] : [c.tributes])),
    listKey,
  );
  if (tributeSets.length > 1) {
    const lengths = tributeSets.map((set) => set.length);
    return {
      kind: "tribute",
      min: Math.min(...lengths),
      max: Math.max(...lengths),
      instanceIds: [...new Set(tributeSets.flat())],
    };
  }

  // Classic #20: when a targetless candidate shares the build with targeted ones, the modes
  // decide which targets even exist, so they are asked first. Otherwise the target order below
  // stands, and a mode shared by every remaining candidate is never asked twice.
  if (fields.some((c) => c.targets === undefined)) {
    const mixedModes = distinctBy(
      fields.flatMap((c) => (c.modes === undefined ? [] : [c.modes])),
      listKey,
    );
    if (mixedModes.length > 1) {
      const lengths = mixedModes.map((list) => list.length);
      return {
        kind: "mode",
        min: Math.min(...lengths),
        max: Math.max(...lengths),
        options: [...new Set(mixedModes.flat())],
      };
    }
  }

  const targetLists = distinctBy(
    fields.flatMap((c) => (c.targets === undefined ? [] : [c.targets])),
    targetsKey,
  );
  if (targetLists.length > 1) {
    const lengths = targetLists.map((list) => list.length);
    return {
      kind: "target",
      min: Math.min(...lengths),
      max: Math.max(...lengths),
      selections: distinctBy(targetLists.flat(), selectionKey),
    };
  }

  const modeLists = distinctBy(
    fields.flatMap((c) => (c.modes === undefined ? [] : [c.modes])),
    listKey,
  );
  if (modeLists.length > 1) {
    const lengths = modeLists.map((list) => list.length);
    return {
      kind: "mode",
      min: Math.min(...lengths),
      max: Math.max(...lengths),
      options: [...new Set(modeLists.flat())],
    };
  }

  const zones = distinctBy(
    fields.flatMap((c) => (c.zone === undefined ? [] : [c.zone])),
    zoneKey,
  );
  if (zones.length > 1) return { kind: "zone", min: 1, max: 1, zones };

  return null;
}

/** One legal candidate left and nothing outstanding: the build is ready to send. */
function readyAction(interaction: Interaction): ActionBody | undefined {
  if (!isBuilding(interaction)) return undefined;
  if (outstandingNeed(interaction) !== null) return undefined;
  const first = remainingCandidates(interaction)[0];
  return first === undefined ? undefined : mergePicked(first, interaction.picked);
}

function settle(next: Interaction): ClickResult {
  const action = readyAction(next);
  return action === undefined ? { interaction: next } : { interaction: IDLE, action };
}

function narrowedBy(interaction: Building, picked: Partial<PlayBuild>): Interaction | null {
  const next: Building = { ...interaction, picked: { ...interaction.picked, ...picked } };
  const remaining = remainingCandidates(next);
  if (remaining.length === 0) return null;
  return { ...next, candidates: remaining };
}

/** Does any candidate actually fix this zone? A board click is only accepted when one does. */
function someCandidateFixesZone(interaction: Building, zone: ZoneChoice): boolean {
  return remainingCandidates(interaction).some((c) => {
    const fixed = fieldsOf(c).zone;
    return fixed !== undefined && zoneKey(fixed) === zoneKey(zone);
  });
}

/** Does any candidate name this selection as one of its declared targets (R81)? */
function someCandidateFixesTarget(interaction: Building, selection: Selection): boolean {
  const key = selectionKey(selection);
  return remainingCandidates(interaction).some((c) => {
    const targets = fieldsOf(c).targets;
    return targets !== undefined && targets.some((s) => selectionKey(s) === key);
  });
}

/** Does any candidate sacrifice this unit as one of its Tributes (§6.3, R81, R384)? */
function someCandidateFixesTribute(interaction: Building, instanceId: string): boolean {
  return remainingCandidates(interaction).some((c) => {
    const tributes = fieldsOf(c).tributes;
    return tributes !== undefined && tributes.includes(instanceId);
  });
}

function appended<T>(existing: readonly T[] | undefined, value: T): T[] {
  return [...(existing ?? []), value];
}

/** Records one more declared target (R81: a declared `hand` or `zone` pick travels in `targets`). */
function narrowByTarget(interaction: Building, selection: Selection): Interaction | null {
  if (!someCandidateFixesTarget(interaction, selection)) return null;
  return narrowedBy(interaction, { targets: appended(interaction.picked.targets, selection) });
}

function narrowByTribute(interaction: Building, instanceId: string): Interaction | null {
  if (!someCandidateFixesTribute(interaction, instanceId)) return null;
  return narrowedBy(interaction, { tributes: appended(interaction.picked.tributes, instanceId) });
}

// ---------------------------------------------------------------------------------------------
// Highlighting: the set of testids the engine has already blessed.
// ---------------------------------------------------------------------------------------------

/** The board controls a listed action type lights. A Heroic Power's is lit by its own instance. */
const CONTROL_FOR_TYPE: Partial<Record<ActionBody["type"], BoardControl>> = {
  endTurn: "end-turn",
  offerDraw: "offer-draw",
  concede: "concede",
};

const CONTROL_TESTID: Record<BoardControl, string> = {
  "end-turn": testid.endTurn,
  "offer-draw": testid.offerDraw,
  power: testid.power,
  concede: testid.concede,
};

/** Board cells an open prompt's own options point at, so a `target` prompt can be answered there. */
export function pendingHighlight(view: PlayerView, pending: PendingView | null): ReadonlySet<string> {
  const out = new Set<string>();
  if (pending === null || !pending.forYou) return out;
  for (const option of pending.options) {
    const where = selectionTestid(view, selectionForOption(option));
    if (where !== null) out.add(where);
  }
  return out;
}

/** `glow` keeps only what `legal` already holds, so the green can never outrun the engine. */
function withinLegal(glow: ReadonlySet<string>, legalIds: ReadonlySet<string>): Set<string> {
  return new Set([...glow].filter((id) => legalIds.has(id)));
}

/** The controls the activation being built stands on: what reads as "selected" while it is built. */
function buildingSources(view: PlayerView, interaction: Building): string[] {
  if (interaction.stage === "playing") return [playSourceTestid(view, interaction.instanceId)];
  const bodies = interaction.candidates.filter(isActivation);
  return [...new Set(bodies.flatMap((body) => activationControlTestids(view, body)))];
}

/**
 * Every `data-testid` the board may light up, plus the selected set. Everything in `legal` got
 * there because an `ActionBody` (or an open prompt's own option list) named it.
 *
 * `glow` is the green (Hearthstone's "can act") and is narrower than `legal`: in idle, the cards a
 * `play` names (in the hand, or "Play" in the graveyard pile, which glows as well), the units an
 * `attack` names, the Activate controls an activation names, and the open prompt's cells;
 * `end-turn` only once none of those is listed and no prompt is open. While a play or an
 * activation is built, the remaining candidates' zones, Tributes and declared targets; while
 * attacking, the selected attacker's targets. `switchPosition`, `offerDraw` and `concede` stay
 * clickable but never glow.
 */
export function highlightFor(
  view: PlayerView,
  legal: readonly ActionBody[],
  interaction: Interaction,
): Highlight {
  // Nothing but concede is legal, so a selection that was in flight when the engine's answer
  // changed under it is dropped, and the board reads as idle. This reads the engine's answer rather
  // than checking `view.result` or whose turn it is: `legalActions` returns [] once the game is
  // over, and only concede while the other seat holds a prompt (R211).
  if (interaction.stage !== "idle" && legal.every((body) => body.type === "concede")) {
    return legal.length === 0 ? NO_HIGHLIGHT : highlightFor(view, legal, IDLE);
  }

  const legalIds = new Set<string>();
  const selected = new Set<string>();
  const glow = new Set<string>();

  // The controls the engine listed stay live through a selection: nothing the player is halfway
  // through building takes `end-turn` or `concede` away from them.
  for (const body of legal) {
    const control = CONTROL_FOR_TYPE[body.type];
    if (control !== undefined) legalIds.add(CONTROL_TESTID[control]);
  }

  if (isBuilding(interaction)) {
    const sources = buildingSources(view, interaction);
    for (const id of sources) selected.add(id);
    // R384: the card whose ability is being built reads as selected too (never as clickable).
    if (interaction.stage === "activating") selected.add(testid.card(interaction.instanceId));
    // Every playable card and every listed activation stays clickable: a second click on the one in
    // flight puts it back down, and a click on another picks that one up instead.
    for (const body of legal) {
      if (isPlay(body)) legalIds.add(playSourceTestid(view, body.instanceId));
      else if (isActivation(body)) for (const id of activationControlTestids(view, body)) legalIds.add(id);
    }
    for (const id of sources) legalIds.add(id);
    const remaining = remainingCandidates(interaction);
    // The glow is where the build can go next: a zone, a Tribute or a declared target. The other
    // playable cards and activations stay clickable (above) but do not glow.
    for (const candidate of remaining) {
      const fixed = fieldsOf(candidate);
      if (fixed.zone !== undefined) {
        const zone = testid.zone("you", fixed.zone.row, fixed.zone.lane);
        legalIds.add(zone);
        glow.add(zone);
      }
      for (const id of fixed.tributes ?? []) {
        legalIds.add(testid.card(id));
        glow.add(testid.card(id));
      }
      for (const selection of fixed.targets ?? []) {
        const where = selectionTestid(view, selection);
        if (where !== null) {
          legalIds.add(where);
          glow.add(where);
        }
      }
    }
    for (const id of sources) glow.delete(id);
    const picked = interaction.picked;
    if (picked.zone !== undefined) selected.add(testid.zone("you", picked.zone.row, picked.zone.lane));
    for (const id of picked.tributes ?? []) selected.add(testid.card(id));
    for (const selection of picked.targets ?? []) {
      const where = selectionTestid(view, selection);
      if (where !== null) selected.add(where);
    }
    return { legal: legalIds, selected, glow: withinLegal(glow, legalIds) };
  }

  if (interaction.stage === "attacking") {
    selected.add(testid.card(interaction.attackerId));
    // Any other unit the engine listed as an attacker stays clickable, so the player can switch.
    for (const body of legal) if (body.type === "attack") legalIds.add(testid.card(body.attackerId));
    legalIds.add(testid.card(interaction.attackerId));
    // Only the selected attacker's targets glow; the other attackers stay clickable but dark.
    for (const candidate of interaction.candidates.filter(isAttack)) {
      if (candidate.attackerId !== interaction.attackerId) continue;
      const where = attackTargetTestid(view, candidate.targetId);
      legalIds.add(where);
      glow.add(where);
    }
    return { legal: legalIds, selected, glow: withinLegal(glow, legalIds) };
  }

  let canAct = false;
  for (const body of legal) {
    switch (body.type) {
      case "play": {
        const source = playSourceTestid(view, body.instanceId);
        legalIds.add(source);
        glow.add(source);
        // B5 E11: the graveyard pile itself glows while a card in it may be played.
        if (inOwnGraveyard(view, body.instanceId)) {
          legalIds.add(testid.graveyard("you"));
          glow.add(testid.graveyard("you"));
        }
        canAct = true;
        break;
      }
      case "attack":
        legalIds.add(testid.card(body.attackerId));
        glow.add(testid.card(body.attackerId));
        canAct = true;
        break;
      case "activate":
      case "activatePower":
        for (const id of activationControlTestids(view, body)) {
          legalIds.add(id);
          glow.add(id);
        }
        canAct = true;
        break;
      case "switchPosition":
        // Clickable, never glowing: turning a unit sideways is not what Hearthstone lights up.
        legalIds.add(testid.card(body.instanceId));
        legalIds.add(testid.switchPosition(body.instanceId));
        break;
      default:
        // Controls were added above, before the stage split.
        break;
    }
  }

  // With a prompt open `legalActions` offers only that prompt's answers (§10.7), so the board
  // would otherwise go entirely grey; the prompt's own options say which cells may be clicked.
  for (const where of pendingHighlight(view, view.pending)) {
    legalIds.add(where);
    glow.add(where);
  }

  // Hearthstone's End Turn lights up once nothing else can be done this turn.
  const endTurnListed = legal.some((body) => body.type === "endTurn");
  if (endTurnListed && !canAct && view.pending === null) glow.add(testid.endTurn);

  return { legal: legalIds, selected, glow: withinLegal(glow, legalIds) };
}

// ---------------------------------------------------------------------------------------------
// The click reducer.
// ---------------------------------------------------------------------------------------------

/** Picks a card up to play it (from the hand, or from the graveyard pile's "Play", B5 E11). */
function startPlay(legal: readonly ActionBody[], interaction: Interaction, instanceId: string): ClickResult {
  const candidates = playsFor(legal, instanceId);
  if (candidates.length === 0) return { interaction };
  return settle({ stage: "playing", instanceId, candidates, picked: {} });
}

/**
 * R384: an Activate control pressed. Its activations become the candidates, built exactly as a
 * play's: one candidate is sent at once; several wait for a target on the board, a Tribute, or a
 * mode in the inline picker. Pressing the control of the build in flight puts it back down.
 */
function startActivation(
  legal: readonly ActionBody[],
  interaction: Interaction,
  target: Extract<ClickTarget, { on: "activate" }>,
): ClickResult {
  if (
    interaction.stage === "activating" &&
    interaction.instanceId === target.instanceId &&
    interaction.ability === target.ability
  ) {
    return { interaction: IDLE };
  }
  const candidates = activationsFor(legal, target.instanceId, target.ability);
  if (candidates.length === 0) return { interaction };
  return settle({
    stage: "activating",
    instanceId: target.instanceId,
    ...(target.ability === undefined ? {} : { ability: target.ability }),
    candidates,
    picked: {},
  });
}

/**
 * One click. A click that no candidate can account for returns the interaction unchanged and no
 * action: an illegal click is simply not a move.
 */
export function onClickTarget(
  view: PlayerView,
  legal: readonly ActionBody[],
  interaction: Interaction,
  target: ClickTarget,
): ClickResult {
  switch (target.on) {
    case "hand": {
      if (isBuilding(interaction)) {
        if (interaction.stage === "playing" && interaction.instanceId === target.instanceId) return { interaction: IDLE };
        // R81: a declared hand pick (Glowy Jelly Bean) travels in `targets`, not as a prompt. It
        // wins over re-selecting only when a candidate actually names this card as a target.
        const asTarget = narrowByTarget(interaction, { pick: "instance", instanceId: target.instanceId });
        if (asTarget !== null) return settle(asTarget);
      }
      return startPlay(legal, interaction, target.instanceId);
    }

    case "graveyard": {
      if (interaction.stage === "playing" && interaction.instanceId === target.instanceId) return { interaction: IDLE };
      return startPlay(legal, interaction, target.instanceId);
    }

    case "activate":
      return startActivation(legal, interaction, target);

    case "zone": {
      if (!isBuilding(interaction)) return { interaction };
      const zone: ZoneChoice = { row: target.row, lane: target.lane };
      if (target.side === "you" && someCandidateFixesZone(interaction, zone)) {
        const next = narrowedBy(interaction, { zone });
        return next === null ? { interaction } : settle(next);
      }
      // A declared zone pick (R81) travels in `targets` and may name either side of the board.
      const asTarget = narrowByTarget(interaction, {
        pick: "zone",
        player: playerOf(view, target.side),
        row: target.row,
        lane: target.lane,
      });
      return asTarget === null ? { interaction } : settle(asTarget);
    }

    case "unit":
    case "backrow": {
      if (interaction.stage === "attacking") {
        if (interaction.attackerId === target.instanceId) return { interaction: IDLE };
        const action = interaction.candidates
          .filter(isAttack)
          .find(
            (candidate) =>
              candidate.attackerId === interaction.attackerId &&
              attackTargetTestid(view, candidate.targetId) === testid.card(target.instanceId),
          );
        return action === undefined ? { interaction } : { interaction: IDLE, action };
      }
      if (isBuilding(interaction)) {
        const asTribute = target.on === "unit" ? narrowByTribute(interaction, target.instanceId) : null;
        if (asTribute !== null) return settle(asTribute);
        const asTarget = narrowByTarget(interaction, { pick: "instance", instanceId: target.instanceId });
        if (asTarget !== null) return settle(asTarget);
        // R446: a backrow card covers its zone, so a click on a carrier's card is a click on its
        // zone — otherwise a Unit could never be placed on an Ivory Tower by tapping the Tower.
        if (target.on === "backrow" && target.side === "you") {
          const zone: ZoneChoice = { row: "backrow", lane: target.lane };
          if (someCandidateFixesZone(interaction, zone)) {
            const next = narrowedBy(interaction, { zone });
            return next === null ? { interaction } : settle(next);
          }
        }
        return { interaction };
      }
      const candidates = attacksBy(legal, target.instanceId);
      if (candidates.length === 0) return { interaction };
      // R1200: while a Mayor acts the attack names no target, so the click sends it at once.
      const only = candidates.length === 1 ? candidates[0] : undefined;
      if (only?.targetId === RANDOM_ATTACK_TARGET) return { interaction: IDLE, action: only };
      return { interaction: { stage: "attacking", attackerId: target.instanceId, candidates } };
    }

    case "hero": {
      const player = playerOf(view, target.side);
      if (interaction.stage === "attacking") {
        const action = interaction.candidates
          .filter(isAttack)
          .find(
            (candidate) =>
              candidate.attackerId === interaction.attackerId &&
              attackTargetTestid(view, candidate.targetId) === testid.hero(target.side),
          );
        return action === undefined ? { interaction } : { interaction: IDLE, action };
      }
      if (isBuilding(interaction)) {
        const asTarget = narrowByTarget(interaction, { pick: "hero", player });
        return asTarget === null ? { interaction } : settle(asTarget);
      }
      return { interaction };
    }

    case "switch": {
      const action = legal.find(
        (body) => body.type === "switchPosition" && body.instanceId === target.instanceId,
      );
      return action === undefined ? { interaction } : { interaction: IDLE, action };
    }
  }
}

/** A choice made in a picker rather than on the board: the X stepper, the embiggen toggle, … */
export function pickInPlay(interaction: Interaction, patch: Partial<PlayBuild>): ClickResult {
  if (!isBuilding(interaction)) return { interaction };
  const next = narrowedBy(interaction, patch);
  return next === null ? { interaction } : settle(next);
}

/**
 * The listed action a board control sends. `power` is the first `activatePower` listed, kept for a
 * caller with no view; the board's power button builds its activation through `onClickTarget`
 * (`{ on: "activate" }`), so a power with targets to choose is built like any activation.
 */
export function onControl(legal: readonly ActionBody[], control: BoardControl): ActionBody | undefined {
  if (control === "power") return legal.find((body) => body.type === "activatePower");
  return legal.find((body) => CONTROL_FOR_TYPE[body.type] === control);
}

// ---------------------------------------------------------------------------------------------
// Answering a prompt.
// ---------------------------------------------------------------------------------------------

/** The `<pick>:` prefixes the engine puts on an option key (`crates/engine/src/effects/choose.rs`). */
const KEY_PREFIXES = ["instance:", "hero:", "zone:", "mode:", "craft:"] as const;

function withoutPickPrefix(key: string): string {
  for (const prefix of KEY_PREFIXES) if (key.startsWith(prefix)) return key.slice(prefix.length);
  return key;
}

/**
 * A `PendingOption` back to the `Selection` the engine stored for it.
 *
 * FINDING against SPEC §10.6 / §10.8: the engine's own `PromptOption` is `{ key, label,
 * selection }`, but the view's `PendingOption` flattens that to `{ key, label, instanceId?,
 * defId?, player?, row?, lane? }`. The `Selection` is gone, so the client has to rebuild it — and
 * a Discover option, whose stored selection is `{ pick: "mode", option: "<defId>" }`, is
 * indistinguishable from a card-in-hand option except by what fields happen to be set. Carrying
 * the `Selection` verbatim in `PendingOption` would delete this function.
 */
export function selectionForOption(option: PendingOption): Selection {
  // ME-CRAFT (Meditative #17, R880): a craft preset carries its recipe; nothing else sets it.
  if (option.recipe !== undefined) return { pick: "craft", recipe: option.recipe };
  if (option.row !== undefined && option.lane !== undefined && option.player !== undefined) {
    return { pick: "zone", player: option.player, row: option.row, lane: option.lane };
  }
  if (option.instanceId !== undefined) return { pick: "instance", instanceId: option.instanceId };
  if (option.player !== undefined) return { pick: "hero", player: option.player };
  if (option.key === "none" || option.key.startsWith("none:")) return { pick: "none" };
  // Everything else is a mode option: a Discover def id, a "Choose one" mode, a direction, or an
  // X / embiggen value, none of which `Selection` has a member for beyond `{ pick: "mode" }`.
  return { pick: "mode", option: option.defId ?? withoutPickPrefix(option.key) };
}

/**
 * The action that answers the open prompt with the options the player picked.
 *
 * The mulligan is not an `answer`: §10.2 gives it its own `mulligan {keep[]}` action, and
 * `legalActions` enumerates it as subsets of the prompt's option keys, so the picked keys *are*
 * the kept cards. They are re-ordered into hand order so the same picks always build the same
 * action (the engine reads `keep` as a set, `crates/engine/src/setup.rs`).
 */
export function answerAction(
  pending: Extract<PendingView, { forYou: true }>,
  keys: readonly string[],
  view: PlayerView,
  legal: readonly ActionBody[] = [],
): ActionBody {
  if (pending.kind === "mulligan") {
    const hand = Array.isArray(view.you.hand) ? view.you.hand.map((card) => card.instanceId) : [];
    const chosen = new Set(keys);
    const inHandOrder = hand.filter((id) => chosen.has(id));
    const rest = keys.filter((key) => !hand.includes(key));
    const keep = [...inHandOrder, ...rest];
    const listed = legal.find((body) => body.type === "mulligan" && listKey(body.keep) === listKey(keep));
    return listed ?? { type: "mulligan", keep };
  }

  const byKey = new Map(pending.options.map((option) => [option.key, option]));
  const selection = keys.map((key) => {
    const option = byKey.get(key);
    return option === undefined ? { pick: "mode" as const, option: withoutPickPrefix(key) } : selectionForOption(option);
  });

  // `promptAnswers` (engine `prompts.ts`) enumerates every answer the open prompt would accept, so
  // when the caller hands the array over, the action sent is the engine's own body rather than the
  // client's reconstruction of it — option order included. The reconstruction is the fallback for
  // a caller that has no array, and for a prompt whose answers ran past the engine's own cap.
  const wanted = targetsKey(selection);
  const listed = legal.find(
    (body) => body.type === "answer" && body.choiceId === pending.choiceId && targetsKey(body.selection) === wanted,
  );
  return listed ?? { type: "answer", choiceId: pending.choiceId, selection };
}

/** The nonce is handed in, never generated: this module is as pure as the engine (CLAUDE.md rule 4). */
export function withNonce(body: ActionBody, playerId: PlayerId, nonce: string): Action {
  return { ...body, playerId, nonce };
}

/** Zone choices in board order, for a picker that wants to draw a grid. */
export function zonesInBoardOrder(zones: readonly ZoneChoice[]): ZoneChoice[] {
  const rows: Row[] = ["units", "backrow"];
  return rows.flatMap((row) =>
    LANES.flatMap((lane) => zones.filter((zone) => zone.row === row && zone.lane === lane)),
  );
}
