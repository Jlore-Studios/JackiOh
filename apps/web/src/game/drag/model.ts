// Drag to play, as data (docs/polish/7-mobile-ux.md S9, B34, B35).
//
// A drag is two clicks with the pointer held down between them: the press is the click that
// picks the source up, the release is the click that puts it down. So this module adds no rule
// and asks no question about a card. It builds the interaction the source click would build
// (without settling it), asks `highlightFor` where that interaction may land, and on the drop
// hands the landing spot to the same `onClickTarget` a click would reach (CLAUDE.md rule 7).
//
// R384, R510: an activation is dragged the same way. A press on an Activate control (or a Heroic
// Power's button) lifts its activation, and a press on a card you control that has nothing to
// attack with lifts the one ability its control would; both draw the targeting arrow, and the drop
// is the target click that narrows it. An activation with no target to aim at has nothing to drag
// to: its press stays a click, except on a backrow card of yours, which is dragged onto the board
// as a spell with no target is (R658).
//
// R658: a play or an activation that a drop (or a click) has narrowed but not finished, such as a
// Unit placed in its zone whose Cry still wants a target, is lifted again from what it has picked
// so far: a press on the chosen zone, Tribute or target draws the arrow from there to what is left,
// with everything picked kept. Released on nothing, it goes back to the build it was, not to idle.
//
// Pure: no DOM, no React, no state of its own.

import type { ActionBody, PlayerView } from "@jackioh/shared";

import {
  IDLE,
  activationControlTestids,
  activationsFor,
  activationsOnField,
  highlightFor,
  isBuilding,
  onClickTarget,
  outstandingNeed,
  pickInPlay,
  playSourceTestid,
  type ActivationBody,
  type ClickResult,
  type Interaction,
} from "../actions.ts";
import { testid, type ClickTarget } from "../contract.ts";

/** Pointer travel, in CSS px, before a press becomes a drag. Below it the press is a click. */
export const DRAG_THRESHOLD_PX = 8;

export type DragKind = "play" | "attack" | "activate";
export type DragSource = Extract<ClickTarget, { on: "hand" } | { on: "unit" } | { on: "backrow" } | { on: "activate" }>;

type Playing = Extract<Interaction, { stage: "playing" }>;
type Attacking = Extract<Interaction, { stage: "attacking" }>;
type Activating = Extract<Interaction, { stage: "activating" }>;

export type DragPlan = {
  kind: DragKind;
  source: DragSource;
  /** `hand-card-<id>`, `card-<id>`, or the Activate control's (`activate-<id>`, `power`). */
  sourceTestid: string;
  /** What the drag holds while in flight: every candidate for the source, NOT settled. */
  lifted: Playing | Attacking | Activating;
  /** `highlightFor(view, legal, lifted).glow ?? new Set()`: the testids a drop may land on. */
  dropTestids: ReadonlySet<string>;
  /** kind "play" and (dropTestids is empty, or outstandingNeed(lifted) === null): a drop anywhere on the board commits. */
  freeDrop: boolean;
  /** kind "attack" or "activate", or a play where no remaining candidate has a zone and some has targets. Otherwise a card ghost. */
  arrow: boolean;
  /**
   * R658: what a release on nothing (or a cancelled drag) leaves, when it is not IDLE: a build lifted
   * again from its picks goes back to that build, so a missed drop never throws the picks away.
   */
  missed?: Interaction;
};

export type DropSpot =
  | { at: "target"; target: ClickTarget; testid: string }
  | { at: "board" }
  | { at: "outside" };

function dropSetFor(view: PlayerView, legal: readonly ActionBody[], lifted: Interaction): ReadonlySet<string> {
  return highlightFor(view, legal, lifted).glow ?? new Set<string>();
}

function planPlay(
  view: PlayerView,
  legal: readonly ActionBody[],
  interaction: Interaction,
  source: Extract<ClickTarget, { on: "hand" }>,
): DragPlan | null {
  const candidates = legal.filter((body) => body.type === "play" && body.instanceId === source.instanceId);
  if (candidates.length === 0) return null;

  const sourceTestid = testid.handCard(source.instanceId);
  // A hand card the play (or activation) in flight names as a declared target (R81) is picked by
  // clicking it, so its press stays a click rather than lifting it as a play of its own.
  if (isBuilding(interaction)) {
    const glow = highlightFor(view, legal, interaction).glow;
    if (glow !== undefined && glow.has(sourceTestid)) return null;
  }

  const lifted: Playing = { stage: "playing", instanceId: source.instanceId, candidates, picked: {} };
  const dropTestids = dropSetFor(view, legal, lifted);
  const freeDrop = dropTestids.size === 0 || outstandingNeed(lifted) === null;

  // Nothing is picked yet, so every candidate is a remaining one. ME-ALTPLAY: a free drop casts
  // face-up, so the set plays never make a drop free nor draw its arrow.
  let anyZone = false;
  let anyTargets = false;
  for (const candidate of candidates) {
    if (candidate.type !== "play" || candidate.faceDown !== undefined) continue;
    if (candidate.zone !== undefined) anyZone = true;
    if (candidate.targets !== undefined && candidate.targets.length > 0) anyTargets = true;
  }

  return {
    kind: "play",
    source,
    sourceTestid,
    lifted,
    dropTestids,
    freeDrop,
    arrow: !anyZone && anyTargets,
  };
}

function planAttack(
  view: PlayerView,
  legal: readonly ActionBody[],
  interaction: Interaction,
  source: Extract<ClickTarget, { on: "unit" }>,
): DragPlan | null {
  if (source.side !== "you") return null;
  // While a play or an activation is in flight a press on your own unit is a tribute or target click.
  if (isBuilding(interaction)) return null;
  const candidates = legal.filter((body) => body.type === "attack" && body.attackerId === source.instanceId);
  // R384: a unit with nothing to attack with lifts its Activate ability instead, when it has one.
  if (candidates.length === 0) return planCardActivation(view, legal, source);

  const lifted: Attacking = { stage: "attacking", attackerId: source.instanceId, candidates };
  return {
    kind: "attack",
    source,
    sourceTestid: testid.card(source.instanceId),
    lifted,
    dropTestids: dropSetFor(view, legal, lifted),
    freeDrop: false,
    arrow: true,
  };
}

/** R384: some candidate aims at something, so there is a target to drag the activation to. */
function aims(candidates: readonly ActivationBody[]): boolean {
  return candidates.some((body) => (body.targets?.length ?? 0) > 0);
}

/**
 * R384: an activation lifted from its control: every listed activation the control stands for
 * (`activationsFor`), unsettled, with the arrow. Null when none is listed, or none aims at anything
 * and `unaimed` is not set. R658: with `unaimed` (a backrow card of yours), an activation that aims
 * at nothing is lifted as a card ghost and dropped anywhere on the board, as a spell with no target is.
 */
function planActivation(
  view: PlayerView,
  legal: readonly ActionBody[],
  source: Extract<ClickTarget, { on: "activate" }> | Extract<ClickTarget, { on: "unit" } | { on: "backrow" }>,
  instanceId: string,
  ability: string | undefined,
  sourceTestid: string,
  unaimed = false,
): DragPlan | null {
  const candidates = activationsFor(legal, instanceId, ability);
  if (candidates.length === 0) return null;
  const aimed = aims(candidates);
  if (!aimed && !unaimed) return null;
  const lifted: Activating = {
    stage: "activating",
    instanceId,
    ...(ability === undefined ? {} : { ability }),
    candidates,
    picked: {},
  };
  const dropTestids = dropSetFor(view, legal, lifted);
  return {
    kind: "activate",
    source,
    sourceTestid,
    lifted,
    dropTestids,
    freeDrop: !aimed && (dropTestids.size === 0 || outstandingNeed(lifted) === null),
    arrow: aimed,
  };
}

/**
 * R384: a card you control pressed with nothing to attack with: the activation its one Activate
 * control would build. A card listing several abilities is dragged from the control of the one meant.
 */
function planCardActivation(
  view: PlayerView,
  legal: readonly ActionBody[],
  source: Extract<ClickTarget, { on: "unit" } | { on: "backrow" }>,
): DragPlan | null {
  const listed = activationsOnField(view, source.instanceId);
  if (listed !== null && listed.length > 1) return null;
  const unaimed = source.on === "backrow";
  return planActivation(view, legal, source, source.instanceId, undefined, testid.card(source.instanceId), unaimed);
}

/**
 * R658: a build in flight lifted again from one of its picks. `pressed` is the testid pressed; it
 * must be a pick the board shows as selected (the chosen zone, a Tribute, a declared target, or the
 * card whose ability is built), not the hand card of the play, whose press lifts it afresh. Null
 * when nothing is picked yet or nothing is left to aim at on the board (a picker's choice is next).
 */
function planContinue(
  view: PlayerView,
  legal: readonly ActionBody[],
  interaction: Interaction,
  pressed: string,
): DragPlan | null {
  if (!isBuilding(interaction)) return null;
  if (Object.keys(interaction.picked).length === 0) return null;
  if (interaction.stage === "playing" && pressed === playSourceTestid(view, interaction.instanceId)) return null;
  const highlight = highlightFor(view, legal, interaction);
  if (!highlight.selected.has(pressed)) return null;
  // What is already picked still glows (every remaining candidate fixes it), but is not a place to go.
  const dropTestids = new Set([...(highlight.glow ?? [])].filter((id) => !highlight.selected.has(id)));
  if (dropTestids.size === 0 || outstandingNeed(interaction) === null) return null;
  const source: DragSource =
    interaction.stage === "playing"
      ? { on: "hand", instanceId: interaction.instanceId }
      : {
          on: "activate",
          instanceId: interaction.instanceId,
          ...(interaction.ability === undefined ? {} : { ability: interaction.ability }),
        };
  return {
    kind: interaction.stage === "playing" ? "play" : "activate",
    source,
    sourceTestid: pressed,
    lifted: interaction,
    dropTestids,
    freeDrop: false,
    arrow: true,
    missed: interaction,
  };
}

/**
 * hand source: the `play`s naming it, or null if none, or if a play or activation is in flight and
 * the card's testid is in the current glow (it's a declared target; its press stays a click).
 * unit source (`side: "you"`): the `attack`s naming it; with none, its one Activate ability's
 * aimed activations (R384); null if neither, or while a play or activation is in flight.
 * backrow source (`side: "you"`): its one Activate ability's activations, aimed (the arrow) or not
 * (a ghost dropped on the board, R658).
 * activate source (a control): the activations it stands for, when one of them aims at something.
 * R658: with `pressed` (the testid pressed) one of the picks of a build in flight, the build itself,
 * lifted again with its picks kept; this comes before all of the above.
 * Anything else: null.
 */
export function planDrag(
  view: PlayerView,
  legal: readonly ActionBody[],
  interaction: Interaction,
  source: ClickTarget,
  pressed?: string,
): DragPlan | null {
  if (pressed !== undefined) {
    const continued = planContinue(view, legal, interaction, pressed);
    if (continued !== null) return continued;
  }
  if (source.on === "hand") return planPlay(view, legal, interaction, source);
  if (source.on === "unit") return planAttack(view, legal, interaction, source);
  if (source.on === "backrow") {
    if (source.side !== "you" || isBuilding(interaction)) return null;
    return planCardActivation(view, legal, source);
  }
  if (source.on === "activate") {
    // The arrow starts from the control pressed: the card's own, or a Heroic Power's on the panel.
    const first = activationsFor(legal, source.instanceId, source.ability)[0];
    const control = first === undefined ? undefined : activationControlTestids(view, first)[0];
    const sourceTestid = control ?? testid.activate(source.instanceId, source.ability);
    return planActivation(view, legal, source, source.instanceId, source.ability, sourceTestid);
  }
  return null;
}

/**
 * target spot in dropTestids: r = onClickTarget(view, legal, plan.lifted, spot.target); return
 * r if r.action is set or r.interaction !== plan.lifted, else { interaction: plan.missed ?? IDLE }.
 * board spot and plan.freeDrop: pickInPlay(plan.lifted, {}), which is an action, or a play still
 * needing a picker. Anything else: { interaction: plan.missed ?? IDLE } and no action.
 */
export function resolveDrop(
  view: PlayerView,
  legal: readonly ActionBody[],
  plan: DragPlan,
  spot: DropSpot,
): ClickResult {
  if (spot.at === "target" && plan.dropTestids.has(spot.testid)) {
    const result = onClickTarget(view, legal, plan.lifted, spot.target);
    if (result.action !== undefined || result.interaction !== plan.lifted) return result;
    return { interaction: plan.missed ?? IDLE };
  }
  // ME-ALTPLAY: a free drop casts face-up; setting asks its timing through the picker.
  if (spot.at === "board" && plan.freeDrop)
    return pickInPlay(plan.lifted, plan.kind === "play" ? { faceDown: "up" } : {});
  return { interaction: plan.missed ?? IDLE };
}
