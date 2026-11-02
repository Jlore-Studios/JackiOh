// Turning clicks into actions, and nothing else (CLAUDE.md rule 7, BUILD M5-T2, SPEC §10.2).
//
// THE ONE RULE OF THIS FILE IS THAT IT HAS NO RULES. Every question of the form "may I?" is answered by
// searching the `ActionBody[]` that `legalActions(state, player)` handed us. The yellow glow is
// `CardView.conditionActive`, which the engine computes (R195).
//
// R81: zone, X, embiggen, Tribute, a card's declared targets and modes, and the `plague` payment (Classic
// #74) are NOT prompts: they travel inside the `play` action. A targeting cost's discards (Classic #89) are
// random at pay time (R682) and travel nowhere. Everything chosen during resolution arrives as a `PendingChoice`.
//
// R391 (B4.5): a Tribute may pay for its own zone. Narrowing keeps whole candidates, never a field on its own,
// so a zone and a Tribute picked in either order only ever finish a pair the engine listed.
//
// CARRY_THROUGH: a candidate that leaves `tributes`, `targets` or `modes` unset accepts what a *picker* chose
// for it (`pickInPlay`), validated by the engine per R90. A *board click* is never carried through, so the
// board can never be clicked into an illegal play.

import type {
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
  playerOf,
  sideOf,
  testid,
  type BoardControl,
  type ClickTarget,
  type Highlight,
  type Side,
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

/** ME-ALTPLAY (R1040, R1044): face-up, or face-down as a Trap revealing at the chosen timing. */
export type FaceDownChoice = RevealAt | "up";

/** The R81 play-time choices (and an activation's), as the client accumulates them. */
export type PlayBuild = {
  ability?: string;
  zone?: ZoneChoice;
  x?: number;
  embiggen?: boolean;
  /** R1086: an Equip play onto the zone's host Unit. */
  equip?: boolean;
  tributes?: string[];
  targets?: Selection[];
  modes?: string[];
  plague?: PlagueChoice;
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
  cardAbilities?: string[];
};
type Building = Playing | Activating;

/** The client's in-progress selection. Not state the engine knows or cares about. */
export type Interaction =
  | { stage: "idle" }
  | Playing
  | Activating
  | { stage: "attacking"; attackerId: string; candidates: ActionBody[] };

export const IDLE: Interaction = { stage: "idle" };

export function isBuilding(interaction: Interaction): interaction is Building {
  return interaction.stage === "playing" || interaction.stage === "activating";
}

/** What is still unchosen about the play in flight, derived only from the candidates. */
export type PlayNeed =
  | { kind: "ability"; min: number; max: number; abilities: string[] }
  | { kind: "faceDown"; min: number; max: number; options: FaceDownChoice[] }
  | { kind: "zone"; min: number; max: number; zones: ZoneChoice[] }
  | { kind: "x"; min: number; max: number; values: number[] }
  | { kind: "embiggen"; min: number; max: number; values: boolean[] }
  | { kind: "equip"; min: number; max: number; values: boolean[] }
  | { kind: "plague"; min: number; max: number; options: PlagueChoice[] }
  | { kind: "tribute"; min: number; max: number; instanceIds: string[] }
  | { kind: "target"; min: number; max: number; selections: Selection[] }
  | { kind: "mode"; min: number; max: number; options: string[] };

/**
 * What a click comes to: the next interaction, the action to send, if one is ready, and (R849000, R979000) a
 * `notice`, the engine's own words for why a click on your unit or hand card did nothing. A notice sends nothing.
 */
export type ClickResult = { interaction: Interaction; action?: ActionBody; notice?: string };

// Keys. Presentation-only identity for a choice, so a picker can round-trip it through the DOM.

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

// Searching the legal array. Nothing here knows a rule; it all reads `ActionBody`s.

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
 * R384: the activations an Activate control stands for: a named ability's `activate`s, or with none
 * named every `activate` of the card and its `activatePower` (R43's alias, B3.2 rule 10).
 */
export function activationsFor(legal: readonly ActionBody[], instanceId: string, ability?: string): ActivationBody[] {
  return legal.filter(isActivation).filter((body) => {
    if (body.instanceId !== instanceId) return false;
    if (body.type === "activatePower") return ability === undefined;
    return ability === undefined || body.ability === ability;
  });
}

/**
 * How an `attack` names a hero: `hero-<playerId>`, the client's copy of the id
 * `crates/engine/src/damage.rs`'s `target_id()` builds (`@jackioh/shared` exports no helper for it).
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

export function attackTargetTestid(view: PlayerView, targetId: string): string {
  const player = heroPlayerOfTargetId(view, targetId);
  return player === null ? testid.card(targetId) : testid.hero(sideOf(view, player));
}

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
 * The element a `play` is started from: the hand card (a Resource among them, R2250000), else the
 * "Play" on a card in the viewer's graveyard pile (B5 E11).
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
 * R384, R510: every control an activation lights. A body the view places nowhere lights the
 * control its type has always meant: `power` for `activatePower`, `activate-<id>` for `activate`.
 */
export function activationControlTestids(view: PlayerView, body: ActivationBody): string[] {
  const out: string[] = [];
  const powers = view.you.hero.powers ?? [];
  if (view.you.hero.power?.instanceId === body.instanceId) out.push(testid.power);
  else if (powers.some((power) => power.instanceId === body.instanceId)) out.push(testid.powerOf(body.instanceId));
  if (body.type === "activate") {
    const listed = activationsOnField(view, body.instanceId);
    if (listed !== null && listed.length > 0) {
      out.push(testid.activate(body.instanceId));
    }
  }
  if (out.length === 0) out.push(body.type === "activatePower" ? testid.power : testid.activate(body.instanceId));
  return out;
}

// Narrowing a build. `picked` is what the player has said; candidates are what the engine allows.

/** Every choice field a build body can carry, read the same way off a play or an activation. */
type BuildFields = {
  ability?: string;
  zone?: ZoneChoice;
  x?: number;
  embiggen?: boolean;
  equip?: boolean;
  tributes?: string[];
  targets?: Selection[];
  modes?: string[];
  plague?: PlagueSpend;
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
 * A candidate still matches when every field it fixes equals what the player picked. A field the candidate
 * leaves unset is carried through (CARRY_THROUGH), except the payments, which a candidate without them pays none of.
 */
function matches(candidate: BuildBody, picked: Partial<PlayBuild>): boolean {
  const fixed = fieldsOf(candidate);
  if (picked.ability !== undefined && fixed.ability !== undefined && fixed.ability !== picked.ability) {
    return false;
  }
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
  // R1086: Equip or plain, as picked; a play with `equip: false` is a plain play.
  if (picked.equip !== undefined && (fixed.equip === true) !== picked.equip) return false;
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
      const equip = candidate.equip ?? picked.equip;
      if (zone !== undefined) body.zone = zone;
      if (x !== undefined) body.x = x;
      if (embiggen !== undefined) body.embiggen = embiggen;
      if (equip !== undefined) body.equip = equip;
      if (tributes !== undefined) body.tributes = tributes;
      if (targets !== undefined) body.targets = targets;
      if (modes !== undefined) body.modes = modes;
      return body;
    }
    case "activate": {
      const { tributes: _t, targets: _g, modes: _m, ...rest } = candidate;
      const body: ActivateBody = { ...rest };
      if (picked.ability !== undefined) body.ability = picked.ability;
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
 * The next choice the build still needs, or null when the candidates agree on everything. Asked in cost
 * order (X, embiggen and the Plague Counters change what is paid), then the Tribute, the targets, the modes,
 * and the board-driven zone last so a zone click finishes the play.
 */
export function outstandingNeed(interaction: Interaction): PlayNeed | null {
  if (!isBuilding(interaction)) return null;

  // R1751000: an activation on a card with several abilities asks which one first.
  if (interaction.stage === "activating" && interaction.picked.ability === undefined) {
    const cardAbilities = interaction.cardAbilities;
    const remaining = remainingCandidates(interaction);
    const fields = remaining.map(fieldsOf);
    const legalAbilities = [...new Set(fields.flatMap((c) => (c.ability === undefined ? [] : [c.ability])))];
    const hasMultiple = (cardAbilities !== undefined && cardAbilities.length > 1) || legalAbilities.length > 1;
    if (hasMultiple && legalAbilities.length > 0) {
      return { kind: "ability", min: 1, max: 1, abilities: legalAbilities };
    }
  }

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

  // Classic #20's Discard: a target-first order would force a target pick and strand the targetless
  // mode where no click can reach it. A mode every remaining candidate shares is never asked twice.
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

  // R1086: one zone with both a plain and an Equip play asks which — Equip first.
  if (interaction.picked.equip === undefined) {
    const values = [...new Set(fields.map((c) => c.equip === true))];
    if (values.length > 1) {
      values.sort((a, b) => Number(b) - Number(a));
      return { kind: "equip", min: 1, max: 1, values };
    }
  }

  return null;
}

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

function someCandidateFixesZone(interaction: Building, zone: ZoneChoice): boolean {
  return remainingCandidates(interaction).some((c) => {
    const fixed = fieldsOf(c).zone;
    return fixed !== undefined && zoneKey(fixed) === zoneKey(zone);
  });
}

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

// Highlighting: the set of testids the engine has already blessed.

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
  // R821000: a face-down card has no testid of its own, so the zone its back lies in is what is clicked.
  if (isPlaguePlacementPrompt(pending)) {
    for (const option of pending.options) {
      const zone = hiddenOptionZone(view, option);
      if (zone !== null) out.add(testid.zone(zone.side, zone.row, zone.lane));
    }
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
 * Every `data-testid` the board may light up, plus the selected set. `glow` is the green (Hearthstone's "can
 * act") and is narrower than `legal`: `switchPosition`, `offerDraw` and `concede` are clickable but never glow.
 */
export function highlightFor(
  view: PlayerView,
  legal: readonly ActionBody[],
  interaction: Interaction,
): Highlight {
  // Nothing but concede is legal, so a selection in flight is dropped and the board reads as idle.
  // This reads the engine's answer rather than `view.result` or whose turn it is: `legalActions`
  // returns [] once the game is over, and only concede while the other seat holds a prompt (R211).
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
    for (const body of legal) if (body.type === "attack") legalIds.add(testid.card(body.attackerId));
    legalIds.add(testid.card(interaction.attackerId));
    for (const candidate of interaction.candidates.filter(isAttack)) {
      if (candidate.attackerId !== interaction.attackerId) continue;
      const where = attackTargetTestid(view, candidate.targetId);
      legalIds.add(where);
      glow.add(where);
    }
    // MD-D5, R1121: the attacker's `conditionTargets` inside the glow read yellow too — the drag
    // layer paints them while the attack is dragged or selected. Anything outside the glow is a
    // target the modifier would not apply to, so it stays green-or-dark as the engine listed it.
    const attacker = view.you.units.find((unit) => unit?.instanceId === interaction.attackerId);
    const condition = new Set<string>();
    for (const id of attacker?.conditionTargets ?? []) {
      const where = testid.card(id);
      if (glow.has(where)) condition.add(where);
    }
    const kept = withinLegal(glow, legalIds);
    return { legal: legalIds, selected, glow: kept, condition: withinLegal(condition, kept) };
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
        break;
    }
  }

  // With a prompt open `legalActions` offers only that prompt's answers (§10.7), so the board
  // would otherwise go entirely grey; the prompt's own options say which cells may be clicked.
  for (const where of pendingHighlight(view, view.pending)) {
    legalIds.add(where);
    glow.add(where);
  }

  const endTurnListed = legal.some((body) => body.type === "endTurn");
  if (endTurnListed && !canAct && view.pending === null) glow.add(testid.endTurn);

  return { legal: legalIds, selected, glow: withinLegal(glow, legalIds) };
}

// The click reducer.

/**
 * Picks a card up to play it (from the hand, or from the graveyard pile's "Play", B5 E11). R979000: when
 * `legal` offers a hand card no play, the notice is the engine's `CardView.playReason` (a graveyard card's
 * shows in the pile's sheet instead, R1571000). Nothing is sent.
 */
function startPlay(
  legal: readonly ActionBody[],
  interaction: Interaction,
  instanceId: string,
  view: PlayerView,
): ClickResult {
  const candidates = playsFor(legal, instanceId);
  if (candidates.length === 0) {
    const reason = Array.isArray(view.you.hand)
      ? view.you.hand.find((card) => card.instanceId === instanceId)?.playReason
      : undefined;
    return reason === undefined ? { interaction } : { interaction, notice: reason };
  }
  return settle({ stage: "playing", instanceId, candidates, picked: {} });
}

/** R384: an Activate control pressed. Pressing the control of the build in flight puts it back down. */
function startActivation(
  legal: readonly ActionBody[],
  interaction: Interaction,
  target: Extract<ClickTarget, { on: "activate" }>,
  view?: PlayerView,
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
  const listed = view ? activationsOnField(view, target.instanceId) : null;
  const cardAbilities = listed && listed.length > 1 ? listed.map((a) => a.ability) : undefined;
  return settle({
    stage: "activating",
    instanceId: target.instanceId,
    ...(target.ability === undefined ? {} : { ability: target.ability }),
    candidates,
    picked: target.ability === undefined ? {} : { ability: target.ability },
    cardAbilities,
  });
}

/**
 * Picks a unit up to attack with it, or `null` when `legal` offers it no attack. R1200: while a
 * Mayor acts the attack names no target, so the click sends it at once.
 */
function startAttack(legal: readonly ActionBody[], attackerId: string): ClickResult | null {
  const candidates = attacksBy(legal, attackerId);
  if (candidates.length === 0) return null;
  const only = candidates.length === 1 ? candidates[0] : undefined;
  if (only?.targetId === RANDOM_ATTACK_TARGET) return { interaction: IDLE, action: only };
  return { interaction: { stage: "attacking", attackerId, candidates } };
}

/** One click. A click that no candidate can account for returns the interaction unchanged and no action. */
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
      return startPlay(legal, interaction, target.instanceId, view);
    }

    case "graveyard": {
      if (interaction.stage === "playing" && interaction.instanceId === target.instanceId) return { interaction: IDLE };
      return startPlay(legal, interaction, target.instanceId, view);
    }

    case "activate":
      return startActivation(legal, interaction, target, view);

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
        if (action !== undefined) return { interaction: IDLE, action };
        // A click on another attacker switches the attack to it, as dragging it does (`planAttack`).
        return startAttack(legal, target.instanceId) ?? { interaction };
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
      const started = startAttack(legal, target.instanceId);
      if (started !== null) return started;
      // R849000: no attack listed; the notice is the engine's words, `UnitView.attackReason` (on the
      // viewer's own units only). Nothing is sent.
      const reason = view.you.units.find((unit) => unit?.instanceId === target.instanceId)?.attackReason;
      return reason === undefined ? { interaction } : { interaction, notice: reason };
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
 * The listed action a board control sends. `power` is the first `activatePower` listed, kept for a caller
 * with no view; the board's power button builds its activation through `onClickTarget` (`{ on: "activate" }`).
 */
export function onControl(legal: readonly ActionBody[], control: BoardControl): ActionBody | undefined {
  if (control === "power") return legal.find((body) => body.type === "activatePower");
  return legal.find((body) => CONTROL_FOR_TYPE[body.type] === control);
}

// Answering a prompt.

/** The `<pick>:` prefixes the engine puts on an option key (`crates/engine/src/effects/choose.rs`). */
const KEY_PREFIXES = ["instance:", "hero:", "zone:", "mode:", "craft:"] as const;

function withoutPickPrefix(key: string): string {
  for (const prefix of KEY_PREFIXES) if (key.startsWith(prefix)) return key.slice(prefix.length);
  return key;
}

/**
 * A `PendingOption` back to the `Selection` the engine stored for it. FINDING against SPEC §10.6 / §10.8:
 * the view's `PendingOption` flattens the engine's `PromptOption` (`{ key, label, selection }`), so the
 * client has to rebuild the `Selection`.
 */
export function selectionForOption(option: PendingOption): Selection {
  // ME-CRAFT (Meditative #17, R880): a craft preset carries its recipe; nothing else sets it.
  if (option.recipe !== undefined) return { pick: "craft", recipe: option.recipe };
  // R177, R821000: a face-down card's option carries its id to answer with and the zone it lies in; the
  // id is the selection, so it comes before the coordinates of a zone option.
  if (option.instanceId !== undefined) return { pick: "instance", instanceId: option.instanceId };
  if (option.row !== undefined && option.lane !== undefined && option.player !== undefined) {
    return { pick: "zone", player: option.player, row: option.row, lane: option.lane };
  }
  if (option.player !== undefined) return { pick: "hero", player: option.player };
  if (option.key === "none" || option.key.startsWith("none:")) return { pick: "none" };
  // Everything else is a mode option. The engine keys one `mode:<option>`, its stored string, which
  // can differ from its def id: a night market's Radiant lot (R831002) names its card with a suffix.
  if (option.key.startsWith("mode:")) return { pick: "mode", option: withoutPickPrefix(option.key) };
  return { pick: "mode", option: option.defId ?? withoutPickPrefix(option.key) };
}

/**
 * The action that answers the open prompt with the options the player picked. The mulligan is its own
 * `mulligan {keep[]}` action (§10.2): the picked keys *are* the kept cards, put in hand order so the same
 * picks always build the same action (the engine reads `keep` as a set).
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

  // `promptAnswers` enumerates every answer the open prompt would accept, so when the caller hands
  // the array over, the action sent is the engine's own body, option order included. The client's
  // reconstruction is the fallback: no array, or a prompt whose answers ran past the engine's own cap.
  const wanted = targetsKey(selection);
  const listed = legal.find(
    (body) => body.type === "answer" && body.choiceId === pending.choiceId && targetsKey(body.selection) === wanted,
  );
  return listed ?? { type: "answer", choiceId: pending.choiceId, selection };
}

/**
 * R689, R471: the prompt "Place N Plague Counters" opens (`ask_placement`, `crates/engine/src/effects/plague.rs`):
 * one pick among every permanent on the field, answered on the board instead of in a list. The engine's own
 * two sentences are the only mark the view carries.
 */
const PLAGUE_PLACEMENT_PROMPT = /^Place (?:a Plague Counter|\d+ Plague Counters) on a permanent$/;

export function isPlaguePlacementPrompt(pending: PendingView | null): boolean {
  return (
    pending !== null &&
    pending.forYou &&
    pending.kind === "target" &&
    pending.min === 1 &&
    pending.max === 1 &&
    pending.options.length > 0 &&
    pending.options.every((option) => option.instanceId !== undefined) &&
    PLAGUE_PLACEMENT_PROMPT.test(pending.prompt)
  );
}

/** R821000: the zone a face-down card's option names (its definition is hidden, its place is not). */
function hiddenOptionZone(view: PlayerView, option: PendingOption): { side: Side; row: Row; lane: number } | null {
  if (option.instanceId === undefined || option.defId !== undefined) return null;
  if (option.player === undefined || option.row === undefined || option.lane === undefined) return null;
  return { side: sideOf(view, option.player), row: option.row, lane: option.lane };
}

/**
 * A click on the board that answers the placement prompt, or null when it does not. A card is picked
 * by its id and a face-down card by the zone its back lies in.
 */
export function answerPlaguePlacement(
  view: PlayerView,
  legal: readonly ActionBody[],
  pending: PendingView | null,
  target: ClickTarget,
): ActionBody | null {
  if (pending === null || !pending.forYou || !isPlaguePlacementPrompt(pending)) return null;
  const picked = pending.options.find((option) => {
    switch (target.on) {
      case "unit":
      case "backrow":
        return option.instanceId === target.instanceId;
      case "zone": {
        const zone = hiddenOptionZone(view, option);
        return zone !== null && zone.side === target.side && zone.row === target.row && zone.lane === target.lane;
      }
      default:
        return false;
    }
  });
  return picked === undefined ? null : answerAction(pending, [picked.key], view, legal);
}

/** Zone choices in board order, for a picker that wants to draw a grid. */
export function zonesInBoardOrder(zones: readonly ZoneChoice[]): ZoneChoice[] {
  const rows: Row[] = ["units", "backrow"];
  return rows.flatMap((row) =>
    LANES.flatMap((lane) => zones.filter((zone) => zone.row === row && zone.lane === lane)),
  );
}
