// Trigger a Cry (docs/classic-sets.md B5 E13, R467): run a Unit's Cry again, on the field or out of a
// graveyard, for the player whose card triggered it (Classic #54 Rewind).
//
// The rules, in the order the sequence meets them:
//   - Which card. A Unit whose running face has a Cry: the top of a unit pile (a dormant card under a
//     Stack is not on the field, R13) or a Unit card in a graveyard. Anything else triggers nothing.
//   - Who. The triggering card's controller runs it and makes its choices (R70), whoever controls or
//     owns the Unit — Rewind's Radiant reaches the other player's Units too.
//   - Its choices. A Cry's declared targets and modes (R81) travel in the play that played the Unit,
//     and a triggered Cry has no play, so they are asked as prompts, declaration by declaration, as a
//     cast's are (R70) — modes first when a target declaration hangs on them (R90's `forModes`). A
//     declaration the board cannot satisfy is skipped (§8's conventions), and a Tribute declaration is
//     a play's price, which a triggered Cry never pays: its slots are left empty (`{ pick: "none" }`)
//     so the declarations after it still read their own (R90, R123).
//   - "This". On the field the Unit is the Cry's `self`. Out of a graveyard the Cry runs with no card
//     at all (`self` null, its definition named as R127 names a ceased card's), so an effect aimed at
//     "this" finds nothing, a pool still excludes the definition (§5.1), and the Cry reads X as 0.
//   - Not a play. Nothing is played, cast or announced: no `cardPlayed`, no counters (R55, E4), no
//     Echo, no Combo. The Cry's own effects are ordinary effects.
//   - Pauses. Each question is a prompt this module opened for itself, so its answer comes back here
//     (`prompts.registerPromptAnswerer`, R122), and the Cry that runs once they are answered is the
//     card's own resumable hook (`prompts.runHookResumable`): a prompt inside it parks its own tail on
//     `state.work` ahead of whatever the triggering list still owes (R113). The run record in the
//     prompt's resume is plain JSON, so a paused trigger survives a round trip and replays exactly.
//   - Leaving. A Unit that has left the field since the trigger began (R174), or left the graveyard,
//     by the time its choices are made triggers nothing.

import type { PlayerId, Selection } from "@jackioh/shared";
import { defOf } from "./catalog";
import { cardTypeOf } from "./faces";
import {
  DECLARATION_SLICES_KEY,
  activeTargetDecls,
  declarationSlices,
  declaredModes,
  declaredTargets,
  legalSelectionsFor,
  targetsFollowModes,
} from "./playChoices";
import {
  closePrompt,
  inOfferedOrder,
  openPrompt,
  registerPromptAnswerer,
  resumeAt,
  runHookResumable,
  runResume,
  whyAnswerRefused,
  type AnswerInput,
} from "./prompts";
import type { EngineSink } from "./resolve";
import { scriptOf } from "./scripts";
import { findInstance, type CardInstance, type GameState, type Resume } from "./state";
import { exitMark, leftFieldAfter } from "./stays";
import { RUN_MARKS_KEY, beginWorkCascade, drainWork } from "./work";
import { isBuried } from "./zones";

/** `resume.hook` of a prompt this sequence opened for itself: no card script holds the name. */
export const TRIGGER_CRY_HOOK = "@triggerCry";

/** Where the run record sits inside the prompt's `resume.data`. */
const RUN_KEY = "cry";

/** Where the Unit is when its Cry is triggered. */
export type CryPlace = "field" | "graveyard";

/** A triggered Cry part-way through its questions. All JSON (§9.3). */
type CryRun = {
  instanceId: string;
  controller: PlayerId;
  place: CryPlace;
  /** R174: the field's departures when the trigger began, which a Unit on the field must outlast. */
  since: number;
  targets: Selection[];
  modes: string[];
  /** The next target declaration to ask, and the next mode declaration. */
  declAt: number;
  modeAt: number;
  /** Which kind of question the open prompt is, so its answer is filed where it belongs. */
  awaiting: "mode" | "target" | null;
};

/**
 * Where a card's Cry could be triggered from, or null: a Unit (its running face's type, B2.7) with a
 * Cry on its running face, on top of its unit pile or in a graveyard. A Vanilla card has no text
 * (§6.3), so no Cry. A pure read: the reader a card's target check uses (Classic #54's "a Unit that
 * has a Cry").
 */
export function cryPlaceOf(state: GameState, card: CardInstance): CryPlace | null {
  if (cardTypeOf(state, card) !== "Unit") return null;
  if (scriptOf(card).cry === undefined) return null;
  if (card.zone.z === "graveyard") return "graveyard";
  if (card.zone.z === "field" && card.zone.row === "units" && !isBuried(state, card)) return "field";
  return null;
}

/**
 * B5 E13: trigger `card`'s Cry for `controller`. Asks the Cry's declared choices first, as prompts
 * (the sequence then continues in the answer), and runs the Cry once they are all made. Nothing
 * happens for a card whose Cry cannot be triggered (`cryPlaceOf`).
 */
export function triggerCryOf(sink: EngineSink, card: CardInstance, controller: PlayerId): void {
  const place = cryPlaceOf(sink.state, card);
  if (place === null) return;
  const run: CryRun = {
    instanceId: card.id,
    controller,
    place,
    since: exitMark(sink.state),
    targets: [],
    modes: [],
    declAt: 0,
    modeAt: 0,
    awaiting: null,
  };
  continueRun(sink, run);
}

/** The Unit still where the trigger found it, or null: on the same stay (R174), or in a graveyard. */
function standing(state: GameState, run: CryRun): CardInstance | null {
  const card = findInstance(state, run.instanceId);
  if (card === undefined || cryPlaceOf(state, card) !== run.place) return null;
  if (run.place === "field" && leftFieldAfter(state, run.since, card.id)) return null;
  return card;
}

function resumeFor(run: CryRun): Resume {
  return { defId: "", hook: TRIGGER_CRY_HOOK, step: "choose", radiant: false, data: { [RUN_KEY]: run } };
}

function labelOf(state: GameState, selection: Selection): string {
  switch (selection.pick) {
    case "instance": {
      const card = findInstance(state, selection.instanceId);
      return card === undefined ? selection.instanceId : defOf(state, card.defId).name;
    }
    case "hero":
      return `${selection.player}'s hero`;
    case "zone":
      return `${selection.player} ${selection.row} ${selection.lane}`;
    case "mode":
      return selection.option;
    default:
      return "nothing";
  }
}

function keyOf(selection: Selection): string {
  switch (selection.pick) {
    case "instance":
      return `instance:${selection.instanceId}`;
    case "hero":
      return `hero:${selection.player}`;
    case "zone":
      return `zone:${selection.player}:${selection.row}:${selection.lane}`;
    case "mode":
      return `mode:${selection.option}`;
    default:
      return "none";
  }
}

/** The mode declarations still to ask; false while one is waiting (R81). */
function askModes(sink: EngineSink, run: CryRun, card: CardInstance): boolean {
  const modes = declaredModes(card);
  const name = defOf(sink.state, card.defId).name;
  for (let at = run.modeAt; at < modes.length; at += 1) {
    run.modeAt = at + 1;
    const decl = modes[at];
    if (decl === undefined || decl.options.length === 0) continue;
    run.awaiting = "mode";
    const opened = openPrompt(sink, {
      player: run.controller,
      kind: decl.kind,
      prompt: `Cry: ${name}`,
      options: decl.options.map((option) => ({ key: `mode:${option}`, label: option, selection: { pick: "mode", option } })),
      resume: resumeFor(run),
    });
    if (opened !== null) return false;
    run.awaiting = null;
  }
  return true;
}

/** The target declarations still to ask; false while one is waiting (R81, R90). */
function askTargets(sink: EngineSink, run: CryRun, card: CardInstance): boolean {
  const decls = activeTargetDecls(declaredTargets(card), run.modes);
  const name = defOf(sink.state, card.defId).name;
  for (let at = run.declAt; at < decls.length; at += 1) {
    run.declAt = at + 1;
    const decl = decls[at];
    if (decl === undefined) continue;
    const options = legalSelectionsFor(sink.state, run.controller, card, decl);
    if (decl.kind === "tribute") {
      // A Tribute is a play's price, and nothing was played: its slots stay, empty (R90, R123).
      const slots = Math.min(decl.min, options.length);
      for (let slot = 0; slot < slots; slot += 1) run.targets.push({ pick: "none" });
      continue;
    }
    if (options.length === 0) continue;
    run.awaiting = "target";
    const opened = openPrompt(sink, {
      player: run.controller,
      kind: decl.kind,
      prompt: `Cry: ${name}`,
      options: options.map((selection) => ({ key: keyOf(selection), label: labelOf(sink.state, selection), selection })),
      min: decl.min,
      max: decl.max,
      resume: resumeFor(run),
    });
    if (opened !== null) return false;
    run.awaiting = null;
  }
  return true;
}

/** Ask what is still to ask, then run the Cry; stops at a prompt, whose answer comes back here. */
function continueRun(sink: EngineSink, run: CryRun): void {
  const card = standing(sink.state, run);
  if (card === null) return;
  const asked = targetsFollowModes(declaredTargets(card))
    ? askModes(sink, run, card) && askTargets(sink, run, card)
    : askTargets(sink, run, card) && askModes(sink, run, card);
  if (!asked) return;
  runCry(sink, run, card);
}

/** The Cry itself, with the choices made: the card's own resumable hook (R113). */
function runCry(sink: EngineSink, run: CryRun, card: CardInstance): void {
  // R174: the choices were made against the board as it stands now, and the Cry is aimed at that.
  const exitsFrom = exitMark(sink.state);
  // R90, R102: a fused card's Cry hands each ingredient its own slice of the choices.
  const data =
    sink.state.transientDefs[card.defId] === undefined
      ? {}
      : { [DECLARATION_SLICES_KEY]: declarationSlices(sink.state, run.controller, card, run.targets, run.modes) };
  if (run.place === "field") {
    runHookResumable(sink, card, "cry", {
      controller: run.controller,
      targets: run.targets,
      modes: run.modes,
      data,
      exitsFrom,
    });
    return;
  }
  // Out of a graveyard "this" finds nothing: the Cry runs as its definition's, with no card (R127).
  const resume = resumeAt({
    defId: card.defId,
    hook: "cry",
    step: "",
    radiant: card.radiant,
    catalogVersion: card.catalogVersion ?? sink.state.catalogVersion,
    data,
  });
  runResume(sink, { ...resume, data: { ...resume.data, [RUN_MARKS_KEY]: { exitsFrom } } }, {
    controller: run.controller,
    targets: run.targets,
    modes: run.modes,
  });
}

function runOf(data: Record<string, unknown>): CryRun | null {
  const raw: unknown = data[RUN_KEY];
  if (raw === null || typeof raw !== "object") return null;
  const run = raw as Partial<CryRun>;
  if (typeof run.instanceId !== "string" || (run.controller !== "p1" && run.controller !== "p2")) return null;
  if (run.place !== "field" && run.place !== "graveyard") return null;
  return {
    instanceId: run.instanceId,
    controller: run.controller,
    place: run.place,
    since: typeof run.since === "number" ? run.since : 0,
    targets: Array.isArray(run.targets) ? [...run.targets] : [],
    modes: Array.isArray(run.modes) ? run.modes.filter((mode): mode is string => typeof mode === "string") : [],
    declAt: typeof run.declAt === "number" ? run.declAt : 0,
    modeAt: typeof run.modeAt === "number" ? run.modeAt : 0,
    awaiting: run.awaiting === "mode" || run.awaiting === "target" ? run.awaiting : null,
  };
}

/**
 * R122: the answer to one of this sequence's questions. Validated like any answer, filed into the
 * run — a mode prompt's pick into its modes, a target prompt's picks, in offered order (R221), into
 * its targets — and the sequence goes on: the next question, or the Cry. Then the rest of what the
 * question interrupted drains in R113's order, as `prompts.answerPrompt` does for a card's own step.
 */
function answerCryPrompt(sink: EngineSink, answer: AnswerInput): string | null {
  const pending = sink.state.pending;
  if (pending === null) return "no prompt is open";
  const refused = whyAnswerRefused(pending, answer);
  if (refused !== null) return refused;
  const run = runOf(pending.resume.data);
  closePrompt(sink);
  beginWorkCascade(sink);
  if (run !== null) {
    const picks = inOfferedOrder(pending, answer.selection);
    for (const pick of picks) {
      if (run.awaiting === "mode" && pick.pick === "mode") run.modes.push(pick.option);
      else run.targets.push(pick);
    }
    run.awaiting = null;
    continueRun(sink, run);
  }
  drainWork(sink);
  return null;
}

registerPromptAnswerer(TRIGGER_CRY_HOOK, answerCryPrompt);
