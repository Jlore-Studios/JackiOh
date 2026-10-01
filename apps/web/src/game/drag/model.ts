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
// to: its press stays a click.
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

  // Nothing is picked yet, so every candidate is a remaining one.
  let anyZone = false;
  let anyTargets = false;
  for (const candidate of candidates) {
    if (candidate.type !== "play") continue;
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
 * (`activationsFor`), unsettled, with the arrow. Null when none is listed or none aims at anything.
 */
function planActivation(
  view: PlayerView,
  legal: readonly ActionBody[],
  source: Extract<ClickTarget, { on: "activate" }> | Extract<ClickTarget, { on: "unit" } | { on: "backrow" }>,
  instanceId: string,
  ability: string | undefined,
  sourceTestid: string,
): DragPlan | null {
  const candidates = activationsFor(legal, instanceId, ability);
  if (candidates.length === 0 || !aims(candidates)) return null;
  const lifted: Activating = {
    stage: "activating",
    instanceId,
    ...(ability === undefined ? {} : { ability }),
    candidates,
    picked: {},
  };
  return {
    kind: "activate",
    source,
    sourceTestid,
    lifted,
    dropTestids: dropSetFor(view, legal, lifted),
    freeDrop: false,
    arrow: true,
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
  return planActivation(view, legal, source, source.instanceId, undefined, testid.card(source.instanceId));
}

/**
 * hand source: the `play`s naming it, or null if none, or if a play or activation is in flight and
 * the card's testid is in the current glow (it's a declared target; its press stays a click).
 * unit source (`side: "you"`): the `attack`s naming it; with none, its one Activate ability's
 * aimed activations (R384); null if neither, or while a play or activation is in flight.
 * backrow source (`side: "you"`): its one Activate ability's aimed activations, as for a unit.
 * activate source (a control): the activations it stands for, when one of them aims at something.
 * Anything else: null.
 */
export function planDrag(
  view: PlayerView,
  legal: readonly ActionBody[],
  interaction: Interaction,
  source: ClickTarget,
): DragPlan | null {
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
 * r if r.action is set or r.interaction !== plan.lifted, else { interaction: IDLE }.
 * board spot and plan.freeDrop: pickInPlay(plan.lifted, {}), which is an action, or a play still
 * needing a picker. Anything else: { interaction: IDLE } and no action.
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
    return { interaction: IDLE };
  }
  if (spot.at === "board" && plan.freeDrop) return pickInPlay(plan.lifted, {});
  return { interaction: IDLE };
}
