//! Prompts: `state.pending`, the kinds of SPEC §10.6 — Core's ten and patch v0.2.0's five (B5 E18) —
//! and the serializable continuation that makes answering one re-enter the script that asked
//! (BUILD M3-T3). A prompt may be held by a player other than the asking card's controller (B5 E18,
//! `PROMPT_OWNER_KEY`), and an `answer` prompt keeps its key where no view reaches (R465, `ANSWER_KEY`).
//!
//! §9.3: "Mid-action choices are state, not callbacks." Nothing here ever puts a function in state.
//! A paused sequence is named by two plain records instead:
//!
//!  1. `state.pending.resume` — script id, hook, step name, radiant face, instance and the data
//!     the earlier steps captured. Answering re-enters exactly that step with the selection in
//!     `ctx.targets`, which is where `effects/choose.ts` and its `chosenOptions` read it (§10.6).
//!     Chained prompts are just a step that opens the next one: Private Tutor's three steps, Craft
//!     a Card's two Discovers and radiant Masochism Mask's two picks are all this and nothing more.
//!  2. `state.work` — the *remainder* of an effect list that a prompt interrupted, as a `WorkItem`
//!     naming the same continuation plus the index to continue from, so a script that opens a
//!     prompt in the middle of its list neither restarts nor drops the rest. `work.ts` owns that
//!     queue: this module parks through `work.parkWork` and never touches `state.work` itself, so
//!     one place decides the order a cascade resumes in (R113's cursor) and one place decides what
//!     each owed item means. A card's own continuation is the default work handler, registered at
//!     the bottom of this file the way a card registers a script.
//!
//! Both survive `JSON.parse(JSON.stringify(state))`, which is what makes a prompt identical in live
//! play, in a replay and in a test. Option data lives only in `state.pending.options` and is
//! mirrored nowhere else, so §10.8's `viewFor` has exactly one place to hide from the opponent.
//!
//! This module runs no state check and dispatches no trigger: `triggers.settle` owns the resolution
//! loop of §10.3 and calls in here (R59). Answering does drain `state.work`, because continuing what
//! the prompt interrupted is part of the answer rather than a later reaction to it (R113): §10.6's
//! "`answer` re-invokes the script with the selection" is only true of the whole sequence if the
//! remainder the pause parked runs too, and a caller that answers a prompt without going through
//! `reduce` (the pipeline's own tests, a server that drives the engine directly) would otherwise
//! leave a Spell short of its graveyard. The drain is `work.drainWork`, so the order is R113's and
//! the queue is still the one `work.ts` owns; `settle` drains again and finds nothing left.
//!
//! Port of `packages/engine/src/prompts.ts` (part 3, SURFACE §4, §6.6). Three registries TS filled at
//! module scope to break import cycles go (SURFACE §6.6): the answerer map (`registerPromptAnswerer`)
//! is one `match` on the prompt's `resume.hook` (`answer_owned`), the targeting point's hooks
//! (`registerTargetingHooks`) are direct calls into `targeting_point.rs`, and the default work handler
//! (`registerDefaultWorkHandler`) is `default_work_handler`, which `work.rs`'s dispatcher calls for every
//! hook no engine sequence owns. The resumable runners take the context alone: TS passed the sink
//! beside it, and the context *is* that sink (it derefs to it), so Rust's borrow of it is the one.

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::MAX_PROMPT_ANSWERS;
use crate::resolve::{HookOptions, make_context};
use crate::script::{Effect, EffectContext, EffectPart, EngineSink, Hook, Memo, Script};
use crate::state::{CardInstance, EngineError, GameState, PendingChoice, PromptOption, Resume, WorkItem, find_instance};
use crate::wire::{ActionBody, GameEvent, PlayerId, PromptKind, Row, Selection, TargetAim, ZoneName};
use crate::work::{
    PausedStep, RUN_MARKS_KEY, RunMarks, WorkPlan, begin_work_cascade, card_data, drain_work, park_work, paused_of,
    run_marks_of, script_step_for,
};

/// Where a Death hook's context keeps the snapshot of the unit as it died (R89), so a continuation
/// built from that context — a prompt's answered step, re-entered in a later action — reads the same
/// card the hook did rather than the instance R78 has reset since.
pub const SELF_KEY: &str = "__self";

/// The ten kinds of §10.6. `x`, `embiggen`, `zone`, `tribute` and `direction` are play choices for
/// every Core card (R81) and stay here for later sets. B5 E18 adds five: `number` (a number from a
/// fixed range), `answer` (one option of a multiple-choice problem, whose key never leaves the engine,
/// R465), `cell` (a board cell, either side, either row), `reward` (a completed quest's reward) and
/// `pick` (several cards from a pile under a budget). This module reads the kind for the mulligan,
/// which §2.1 answers with its own action, and for `pick`, whose answers it enumerates its own way.
pub const PROMPT_KINDS: &[PromptKind] = &[
    PromptKind::Discover,
    PromptKind::Target,
    PromptKind::Mode,
    PromptKind::Mulligan,
    PromptKind::Hand,
    PromptKind::Zone,
    PromptKind::Tribute,
    PromptKind::Direction,
    PromptKind::X,
    PromptKind::Embiggen,
    PromptKind::Number,
    PromptKind::Answer,
    PromptKind::Cell,
    PromptKind::Reward,
    PromptKind::Pick,
];

/// The `Script` key holding the step table a prompt answer re-enters (`resume: { picked: … }`).
pub const RESUME_HOOK: &str = "resume";

// Enumerating answers is bounded like `legalActions`'s other combinations (R90): `MAX_PROMPT_ANSWERS`,
// which part 1 moved into `config.rs` (CLAUDE.md rule 9).

/// `state.ts`'s `Resume` is the serializable continuation of §10.6 — "script id + step + captured
/// data", never a closure. Its `hook` names a key of the card's `Script`: a step table (`resume`,
/// so `step` picks the entry) or a hook of its own (`cry`, `delayed`, … , and then `step` only
/// labels the pause). Nothing in this module adds a field to it that a JSON round-trip would lose.
///
/// A plan is that record plus the player whose sequence it is, which is what parking one needs:
/// `work.WorkPlan` under the name the effect side reads it by.
pub type ResumePlan = WorkPlan;

#[derive(Clone, Debug, PartialEq)]
pub struct OpenPromptArgs {
    pub player: PlayerId,
    pub kind: PromptKind,
    /// R656: whether a target prompt is beneficial or harmful, for targeting under targetEnemies.
    pub aim: Option<TargetAim>,
    pub prompt: String,
    pub options: Vec<PromptOption>,
    /// Defaults to one pick (§10.6).
    pub min: Option<i32>,
    pub max: Option<i32>,
    /// B5 E18: a `pick` prompt's budget, which the picked options' `cost`s may not exceed.
    pub budget: Option<i32>,
    /// B5 E18: whose sequence the answer continues, when that is not the player asked — a mode prompt
    /// the other player holds (Classic #8) or their own hand pick (Classic #9). The answered step runs
    /// as this player, the card's controller, so "you draw" is still the asking card's controller's
    /// draw. Absent is the player asked, as for every prompt before E18.
    pub owner: Option<PlayerId>,
    pub resume: Resume,
}

/// B5 E18: where a prompt held by a player other than the asking card's controller keeps that
/// controller (`OpenPromptArgs.owner`), in its resume data. It belongs to that one prompt: a
/// continuation built from the answered step (`resumeSelf`) drops it, and the next prompt that step
/// opens for the other player writes it again.
pub const PROMPT_OWNER_KEY: &str = "__owner";

/// R465: where an `answer` prompt keeps the id of its correct option — in its resume data, which
/// `viewFor` never sends (§10.8) and the AI's redaction strips (`packages/ai`, R185), so the key never
/// leaves the engine. The answered step reads it (`answerKeyOf`); a continuation built from that step
/// drops it (`resumeSelf`), since it belongs to the one prompt that asked.
pub const ANSWER_KEY: &str = "__answerKey";

/// The controller an answered prompt's step runs as (`PROMPT_OWNER_KEY`), else the player asked.
pub fn prompt_owner_of(pending: &PendingChoice) -> PlayerId {
    match pending.resume.data.get(PROMPT_OWNER_KEY).and_then(Value::as_str) {
        Some("p1") => PlayerId::P1,
        Some("p2") => PlayerId::P2,
        _ => pending.player_id,
    }
}

/// R465: the correct option an `answer` prompt's data holds, or null when it holds none — a state
/// whose key was stripped (the AI's redacted copy, R185), where no answer can be judged right.
pub fn answer_key_of(data: &IndexMap<String, Value>) -> Option<String> {
    data.get(ANSWER_KEY).and_then(Value::as_str).map(str::to_string)
}

/// A card's data with the two one-prompt control keys taken out (`PROMPT_OWNER_KEY`, `ANSWER_KEY`).
pub fn without_prompt_keys(data: &IndexMap<String, Value>) -> IndexMap<String, Value> {
    let mut rest = data.clone();
    rest.shift_remove(PROMPT_OWNER_KEY);
    rest.shift_remove(ANSWER_KEY);
    rest
}

/// The `answer` action of §10.2, without the parts the reducer has already checked.
#[derive(Clone, Debug, PartialEq)]
pub struct AnswerInput {
    pub player_id: PlayerId,
    pub choice_id: String,
    pub selection: Vec<Selection>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResumeOptions {
    /// The answering player for a prompt, the owner for parked work.
    pub controller: Option<PlayerId>,
    pub targets: Option<Vec<Selection>>,
    pub modes: Option<Vec<String>>,
    /// R174, §10.6: when `targets` were picked — an answer's picks, as its prompt offered them.
    pub chosen_from: Option<u32>,
}

/// TS `stays.exitMark(state)`: the field's departures so far (R174). A private copy.
fn exit_mark(state: &GameState) -> u32 {
    state.field_exits.as_ref().map_or(0, |exits| exits.count)
}

/// TS `scripts.scriptOf(instance)`: the face that is running — radiant text once the instance is
/// Radiant (§5.2) — and no script at all on a Vanilla instance (§6.3, R115).
fn face_script(state: &GameState, instance: &CardInstance) -> Script {
    if instance.vanilla {
        return Script::default();
    }
    let entry = crate::scripts::script_of(state, &instance.def_id);
    if instance.radiant { entry.radiant } else { entry.base }
}

// ---------------------------------------------------------------------------
// The stored records
// ---------------------------------------------------------------------------

/// The continuation a `PendingChoice`, `WorkItem`, `QueuedTrigger` or `DelayedEffect` carries, with
/// the defaults a hand-built or older record may be missing filled in, so every reader of a stored
/// resume sees the same shape (`playSteps` reads its own payload out of `data`).
///
/// TS took the holder (`{ resume }`); every holder's `resume` is passed here. A Rust `Resume` always
/// has its fields (serde refuses a record without them), so the defaults are already in place.
pub fn resume_of(resume: &Resume) -> Resume {
    Resume {
        def_id: resume.def_id.clone(),
        hook: resume.hook.clone(),
        step: resume.step.clone(),
        radiant: resume.radiant,
        instance_id: resume.instance_id.clone(),
        data: resume.data.clone(),
    }
}

/// `resumeAt`'s argument (data: a card builds it from TS's literal with `json_as`).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResumeAtArgs {
    pub def_id: String,
    pub step: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hook: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<IndexMap<String, Value>>,
}

/// §10.6: a step names itself, so a card file says `step: "picked"` and nothing more.
pub fn resume_at(args: ResumeAtArgs) -> Resume {
    Resume {
        def_id: args.def_id,
        hook: args.hook.unwrap_or_else(|| RESUME_HOOK.to_string()),
        step: args.step,
        radiant: args.radiant == Some(true),
        instance_id: args.instance_id,
        data: card_data(&args.data.unwrap_or_default()),
    }
}

/// The continuation of the script that is running now: the same card, the same face, and the data
/// this chain has captured so far plus whatever this step adds. A step resolving with no instance
/// (`ctx.self === null`: its card has ceased to exist, R127) still names its definition's script,
/// which the context it was re-entered with carries (`EffectContext.defId`), so the answer to a
/// prompt it opens comes back to the same script rather than to none (R113).
///
/// B5 E14, R546: the script running is named by `ctx.defId` first, where the run set it, and by the
/// instance only where it did not — a copier (Classic #57 Echo) runs the copied Spell's text with
/// itself as `self`, and its continuation must come back to that text, not to the copier's own.
pub fn resume_self(ctx: &EffectContext<'_>, step: &str, data: IndexMap<String, Value>) -> Resume {
    let self_ = ctx.self_.as_ref();
    // B5 E18, R465: the owner and the answer key belong to the prompt that asked, not to the run.
    let mut merged = without_prompt_keys(&card_data(&ctx.data));
    for (key, value) in data {
        merged.insert(key, value);
    }
    let built = resume_at(ResumeAtArgs {
        def_id: ctx
            .def_id
            .clone()
            .or_else(|| self_.map(|card| card.def_id.clone()))
            .unwrap_or_default(),
        step: step.to_string(),
        hook: None,
        radiant: Some(ctx.radiant),
        instance_id: self_.map(|card| card.id.clone()),
        data: Some(merged),
    });
    // R113, §10.6: the answer re-invokes the same script, so the step it re-enters is the same run —
    // it reads the stays the run began with (R174) and counts the units it summoned (R136), whichever
    // action it resumes in. A delayed effect is not the run continued and drops this (`effects/delay`).
    let mut data = built.data.clone();
    data.insert(RUN_MARKS_KEY.to_string(), marks_json(&run_marks(ctx)));
    Resume { data, ..built }
}

/// R136: the units a run has summoned so far — those of earlier actions, then this action's.
pub fn summoned_so_far(ctx: &EffectContext<'_>) -> Vec<String> {
    let mut out: Vec<String> = ctx.summoned.clone().unwrap_or_default();
    for event in ctx.events.iter().skip(ctx.events_from) {
        if let GameEvent::Summoned { instance_id, .. } = event {
            out.push(instance_id.clone());
        }
    }
    out
}

/// What a continuation of this run carries of it (`work.RunMarks`).
fn run_marks(ctx: &EffectContext<'_>) -> RunMarks {
    let summoned = summoned_so_far(ctx);
    RunMarks {
        exits_from: Some(ctx.exits_from.unwrap_or_else(|| exit_mark(&*ctx.state))),
        summoned: if summoned.is_empty() { None } else { Some(summoned) },
        resolving: if ctx.self_resolving == Some(true) { Some(true) } else { None },
        event_stay: ctx.event_stay.clone(),
    }
}

/// A `RunMarks` as the JSON block `RUN_MARKS_KEY` holds (only its present keys, as TS spreads them).
fn marks_json(marks: &RunMarks) -> Value {
    serde_json::to_value(marks).expect("run marks are plain JSON (§9.3)")
}

// ---------------------------------------------------------------------------
// Opening and closing
// ---------------------------------------------------------------------------

fn clamp(value: i32, low: i32, high: i32) -> i32 {
    low.max(high.min(value))
}

/// Open the one prompt §10.1 allows. Returns the choice, or null when there is nothing to ask:
/// no options (the effect fizzles and the card still resolves, §6.3) or a prompt already open, so a
/// second ask can never overwrite an unanswered one — a script that asks twice chains steps.
pub fn open_prompt(sink: &mut EngineSink<'_>, args: OpenPromptArgs) -> Option<PendingChoice> {
    if sink.state.pending.is_some() {
        return None;
    }
    if args.options.is_empty() {
        return None;
    }
    // B5 E12, R452: a random cast's caster is answered for at once, and a cast that targets enemies
    // offers them alone when it can (`castPromptShape`, below).
    let args = cast_prompt_shape(sink, args)?;
    // B5 E5, R450: a `target` prompt never offers a card its chooser may not target (the targeting point).
    let options: Vec<PromptOption> = crate::targeting_point::targetable_options(sink.state, &args);
    if options.is_empty() {
        return None;
    }

    let max = clamp(args.max.unwrap_or(1), 0, options.len() as i32);
    let owned = args.owner.is_some_and(|owner| owner != args.player);
    let resume = match args.owner {
        Some(owner) if owned => {
            let mut resume = args.resume.clone();
            resume
                .data
                .insert(PROMPT_OWNER_KEY.to_string(), Value::String(owner.as_str().to_string()));
            resume
        }
        _ => args.resume.clone(),
    };
    let pending = PendingChoice {
        id: format!("q{}", sink.state.next_id),
        player_id: args.player,
        kind: args.kind,
        prompt: args.prompt.clone(),
        options,
        min: clamp(args.min.unwrap_or(1), 0, max),
        max,
        budget: args.budget.map(|budget| budget.max(0)),
        resume,
    };

    sink.state.next_id += 1;
    sink.state.pending = Some(pending.clone());
    sink.events.push(GameEvent::PromptOpened {
        player: args.player,
        choice_id: pending.id.clone(),
        kind: args.kind,
    });
    Some(pending)
}

/// Clear the open prompt and say so (§10.10 animates `promptAnswered`).
pub fn close_prompt(sink: &mut EngineSink<'_>) -> Option<PendingChoice> {
    let pending = sink.state.pending.take()?;
    sink.events.push(GameEvent::PromptAnswered {
        player: pending.player_id,
        choice_id: pending.id.clone(),
    });
    Some(pending)
}

// ---------------------------------------------------------------------------
// Answering
// ---------------------------------------------------------------------------

fn same_selection(a: &Selection, b: &Selection) -> bool {
    match (a, b) {
        (Selection::Instance { instance_id: x }, Selection::Instance { instance_id: y }) => x == y,
        (Selection::Hero { player: x }, Selection::Hero { player: y }) => x == y,
        (Selection::Mode { option: x }, Selection::Mode { option: y }) => x == y,
        (
            Selection::Zone {
                player: p,
                row: r,
                lane: l,
            },
            Selection::Zone {
                player: q,
                row: s,
                lane: m,
            },
        ) => p == q && r == s && l == m,
        (Selection::None, Selection::None) => true,
        _ => false,
    }
}

/// §10.6: how a prompt names a hero or a board cell to the player it is for: theirs ("Your") or the
/// other player's ("Enemy"), never by seat id, which no player sees anywhere else. Only that player
/// is sent the options (R81), so "Your" always reads as the reader's own; the client's pickers word a
/// play's own choices the same way (`selectionLabel` in apps/web/src/game/Prompt.tsx).
pub fn hero_option_label(player: PlayerId, chooser: PlayerId) -> String {
    format!("{} hero", if player == chooser { "Your" } else { "Enemy" })
}

/// §10.6: a board cell, named to its chooser as `heroOptionLabel` names a hero ("Enemy Unit lane 3").
pub fn cell_option_label(player: PlayerId, row: Row, lane: i32, chooser: PlayerId) -> String {
    format!(
        "{} {} lane {lane}",
        if player == chooser { "Your" } else { "Enemy" },
        if row == Row::Units { "Unit" } else { "Backrow" },
    )
}

fn name_of(selection: &Selection, chooser: PlayerId) -> String {
    match selection {
        Selection::Instance { instance_id } => instance_id.clone(),
        Selection::Hero { player } => hero_option_label(*player, chooser),
        Selection::Mode { option } => option.clone(),
        Selection::Zone { player, row, lane } => cell_option_label(*player, *row, *lane, chooser),
        Selection::None => "nothing".to_string(),
    }
}

/// Why an `answer` is not a legal answer to the open prompt, or null when it is. Only what
/// `options` offered may be picked, each option at most once, and the count must sit inside
/// `min`/`max` (M3-T3: "an `answer` with an option not in `options` errors"; R60: N picks are N
/// different cards).
///
/// TS answered the refusal text or null; here `Err` carries that text verbatim (SURFACE §4.4.9).
pub fn why_answer_refused(pending: &PendingChoice, answer: &AnswerInput) -> Result<(), EngineError> {
    if pending.kind == PromptKind::Mulligan {
        return Err(EngineError::new("a mulligan is answered with the mulligan action (§2.1)"));
    }
    if answer.choice_id != pending.id {
        return Err(EngineError::new(format!("no prompt {} is open", answer.choice_id)));
    }
    if pending.player_id != answer.player_id {
        return Err(EngineError::new("that prompt belongs to the other player"));
    }

    let picks = &answer.selection;
    let count = picks.len() as i32;
    if count < pending.min || count > pending.max {
        return Err(EngineError::new(if pending.min == pending.max {
            format!(
                "that prompt takes exactly {} pick{}, got {count}",
                pending.min,
                if pending.min == 1 { "" } else { "s" },
            )
        } else {
            format!("that prompt takes {} to {} picks, got {count}", pending.min, pending.max)
        }));
    }

    let mut used: IndexSet<usize> = IndexSet::new();
    for pick in picks {
        let matches: Vec<usize> = pending
            .options
            .iter()
            .enumerate()
            .filter(|(_, option)| same_selection(&option.selection, pick))
            .map(|(index, _)| index)
            .collect();
        if matches.is_empty() {
            return Err(EngineError::new(format!(
                "{} is not one of the options offered",
                name_of(pick, pending.player_id)
            )));
        }
        let Some(free) = matches.into_iter().find(|index| !used.contains(index)) else {
            return Err(EngineError::new(format!("{} is picked twice", name_of(pick, pending.player_id))));
        };
        used.insert(free);
    }
    // B5 E18: a `pick` prompt's picks may cost no more than its budget together (Classic #44).
    if let Some(budget) = pending.budget {
        let spent: i32 = used
            .iter()
            .map(|&index| pending.options.get(index).and_then(|option| option.cost).unwrap_or(0))
            .sum();
        if spent > budget {
            return Err(EngineError::new(format!(
                "those picks cost {spent} together, over the budget of {budget}"
            )));
        }
    }
    Ok(())
}

/// How an engine sequence answers a prompt it opened itself: validate, close, file the selection
/// and go on, returning the refusal or null — the shape of `answerPrompt`.
pub type PromptAnswerer = fn(&mut EngineSink<'_>, &AnswerInput) -> Result<(), EngineError>;

/// R122, R113: a prompt an engine sequence opened for itself — the play pipeline's own questions, an
/// Echo repeat's fresh picks and a cast's choices (§10.5 steps 4 and 6) — names that sequence as its
/// `resume.hook`, which no card script holds, so re-entering it as a card's continuation finds
/// nothing to run: the selection and the rest of the play would be lost. TS let the module that owns
/// the sequence register its answerer at module scope (`registerPromptAnswerer`); here the registry
/// is this list of hooks, and `answer_owned` below calls each owner directly (SURFACE §6.6).
fn has_answerer(hook: &str) -> bool {
    matches!(hook, "play" | "plague:placement" | "fuse:onto" | "@triggerCry")
}

/// The answer of the engine sequence a prompt's hook names, or `None` when no engine sequence owns
/// it and the prompt re-enters a card's own script step.
fn answer_owned(sink: &mut EngineSink<'_>, hook: &str, answer: &AnswerInput) -> Option<Result<(), EngineError>> {
    match hook {
        // `playSteps.PLAY_WORK_KIND`: the play pipeline's own questions (§10.5).
        "play" => Some(crate::play_steps::answer_play_prompt(sink, answer)),
        // `effects/plague.PLAGUE_PLACEMENT_HOOK`: B5 E19's placement pick.
        "plague:placement" => Some(crate::effects::plague::answer_placement(sink, answer)),
        // `effects/fuse.FUSE_ONTO_HOOK`: the permanent a Fuse goes onto.
        "fuse:onto" => Some(crate::effects::fuse::answer_fuse_onto(sink, answer)),
        // `cryTrigger.TRIGGER_CRY_HOOK`: a Cry a trigger fires.
        "@triggerCry" => Some(crate::cry_trigger::answer_cry_prompt(sink, answer)),
        _ => None,
    }
}

/// §10.6: "`answer` re-invokes the script with the selection." Validate, close the prompt and
/// re-enter the step its `resume` names with the selection in `ctx.targets`. Returns an error
/// message for the reducer, or null.
///
/// It runs that one step and then continues what the prompt interrupted: the remainder parked in
/// `state.work` is drained in R113's order (innermost first, and a pause inside this step lands
/// ahead of everything older), so one answer finishes one sequence. Draining stops at the next
/// prompt, which parks its own tail, so the rest keeps waiting in state. The state check and the
/// trigger queue stay with `triggers.settle`, which drains again and finds nothing owed (R59).
///
/// A prompt an engine sequence opened for itself goes to that sequence's answerer
/// (`registerPromptAnswerer`), so every caller — the reducer, a server or a test driving the engine
/// directly — answers every prompt through this one entry point (R122).
pub fn answer_prompt(sink: &mut EngineSink<'_>, answer: &AnswerInput) -> Result<(), EngineError> {
    let Some(pending) = sink.state.pending.clone() else {
        return Err(EngineError::new("no prompt is open"));
    };

    if let Some(answered) = answer_owned(sink, &pending.resume.hook, answer) {
        return answered;
    }

    why_answer_refused(&pending, answer)?;
    let picks = in_offered_order(&pending, &answer.selection);
    // B5 E5, R450: the targeting point may refuse picks that cost more than their chooser can pay.
    crate::targeting_point::why_target_answer_refused(sink.state, &pending, &picks)?;

    close_prompt(sink);
    // B5 E5, E9, R450: the targeting point — a cost it asks for first (then its own answer finishes
    // this one, so there is nothing more to do here), and the pick an interception moves.
    let targeted: Vec<Selection> = crate::targeting_point::target_answer(sink, &pending, &picks);
    continue_answer(sink, &pending, &targeted);
    Ok(())
}

/// The rest of an answer once its prompt is closed: re-enter the step the prompt paused with the picks,
/// then drain what it interrupted (R113, R122). `answerPrompt` ends here; so does an answer the
/// targeting point interrupted to ask for a cost (`targetingPoint.ts`, R450), and a caller that answers
/// for a player without the targeting point (a random pick targets nothing).
pub fn continue_answer(sink: &mut EngineSink<'_>, pending: &PendingChoice, picks: &[Selection]) {
    // R113, R122: answering re-enters the step the prompt paused, which is taking that step up again —
    // so the cursor resets, and a pause inside it parks its own tail ahead of everything still owed,
    // not behind it at whatever place the action before this one left the cursor.
    begin_work_cascade(sink);
    let chosen_from = exit_mark(sink.state);
    run_resume(
        sink,
        &resume_of(&pending.resume),
        ResumeOptions {
            // B5 E18: a prompt the other player held continues the asking card's sequence, as its controller.
            controller: Some(prompt_owner_of(pending)),
            targets: Some(picks.to_vec()),
            modes: None,
            // R174, §10.6: the picks are cards as the prompt offered them, on the stays they stand on now —
            // whatever the list that asked did to the board before it asked.
            chosen_from: Some(chosen_from),
        },
    );
    drain_work(sink);
}

// ---- v0.2.0: the targeting point (B5 E5, E9, E35; R450), registered by `targetingPoint.ts` ----
//
// What the targeting point does to a prompt: its options drop what the chooser may not target from a
// prompt as it opens (`targeting_point::targetable_options`), its refusal turns down an answer whose
// picks cost more than the chooser holds (`targeting_point::why_target_answer_refused`), and its answer
// runs the point on a closed prompt's picks (`targeting_point::target_answer`). TS's
// `registerTargetingHooks` put the three in a module-level slot because `targetingPoint.ts` sits above
// this module; Rust calls them directly (SURFACE §6.6).

/// R221: an answer's picks are a set (R60's "N different cards"), taken in the order the prompt offered
/// them. `legalActions` offers each set once, in that order, and `reduce` accepts any listing of it —
/// so the listing must not change what the answer does, or a listing no offered answer makes would
/// mean something else: a radiant #26's Echo repeat makes its two picks Radiant in turn, as #80 Zao
/// Gao discarded its picks in turn before R354 made its discard random.
pub fn in_offered_order(pending: &PendingChoice, selection: &[Selection]) -> Vec<Selection> {
    let mut used: IndexSet<usize> = IndexSet::new();
    let mut placed: Vec<(usize, Selection)> = Vec::with_capacity(selection.len());
    for pick in selection {
        let at = pending
            .options
            .iter()
            .enumerate()
            .position(|(index, option)| !used.contains(&index) && same_selection(&option.selection, pick));
        if let Some(at) = at {
            used.insert(at);
        }
        placed.push((at.unwrap_or(usize::MAX), pick.clone()));
    }
    placed.sort_by(|a, b| a.0.cmp(&b.0));
    placed.into_iter().map(|(_, pick)| pick).collect()
}

/// One `answer` action naming these options of the open prompt.
fn answer_of(pending: &PendingChoice, indices: &[usize]) -> ActionBody {
    ActionBody::Answer {
        choice_id: pending.id.clone(),
        selection: indices
            .iter()
            .filter_map(|&index| pending.options.get(index))
            .map(|option| option.selection.clone())
            .collect(),
    }
}

/// `promptAnswers`' walk: every set of `size` options from `start` on, in offered order. Returns false
/// once the bound is reached.
fn walk_answers(
    pending: &PendingChoice,
    start: usize,
    chosen: &mut Vec<usize>,
    size: usize,
    out: &mut Vec<ActionBody>,
) -> bool {
    if chosen.len() == size {
        out.push(answer_of(pending, chosen));
        return out.len() < MAX_PROMPT_ANSWERS;
    }
    for index in start..pending.options.len() {
        chosen.push(index);
        let more = walk_answers(pending, index + 1, chosen, size, out);
        chosen.pop();
        if !more {
            return false;
        }
    }
    true
}

/// Every answer the open prompt would accept, for `legalActions` and the §10.7 policy that draws
/// from it (M3-T3: "`legalActions` lists every option"). Options are taken in the order they were
/// offered, so the list is the same on every machine; a mulligan has its own action and is not
/// enumerated here.
pub fn prompt_answers(pending: &PendingChoice) -> Vec<ActionBody> {
    if pending.kind == PromptKind::Mulligan {
        return Vec::new();
    }
    if pending.kind == PromptKind::Pick {
        return pick_answers(pending);
    }

    let mut out: Vec<ActionBody> = Vec::new();
    let mut size = pending.min;
    while size <= pending.max {
        // A negative size never fills, so it lists nothing (TS's walk never reaches it either).
        if size >= 0 && !walk_answers(pending, 0, &mut Vec::new(), size as usize, &mut out) {
            break;
        }
        size += 1;
    }
    out
}

/// `pickAnswers`' state: the answers listed so far and the sets already listed.
struct PickWalk<'a> {
    pending: &'a PendingChoice,
    out: Vec<ActionBody>,
    seen: IndexSet<String>,
}

impl PickWalk<'_> {
    fn cost_of(&self, index: usize) -> i32 {
        self.pending.options.get(index).and_then(|option| option.cost).unwrap_or(0)
    }

    fn fits(&self, spent: i32) -> bool {
        self.pending.budget.is_none_or(|budget| spent <= budget)
    }

    fn emit(&mut self, indices: &[usize]) -> bool {
        let mut sorted = indices.to_vec();
        sorted.sort();
        let key = sorted.iter().map(usize::to_string).collect::<Vec<_>>().join(",");
        if self.seen.insert(key) {
            self.out.push(answer_of(self.pending, &sorted));
        }
        self.out.len() < MAX_PROMPT_ANSWERS
    }

    /// The rest, smallest sets first; costs are never negative, so an over-budget branch is cut.
    fn walk(&mut self, start: usize, chosen: &mut Vec<usize>, spent: i32, size: usize) -> bool {
        if chosen.len() == size {
            return self.emit(chosen);
        }
        for index in start..self.pending.options.len() {
            let cost = spent + self.cost_of(index);
            if !self.fits(cost) {
                continue;
            }
            chosen.push(index);
            let more = self.walk(index + 1, chosen, cost, size);
            chosen.pop();
            if !more {
                return false;
            }
        }
        true
    }
}

/// B5 E18: a `pick` prompt's answers — every set of `min` to `max` options whose costs fit the budget,
/// bounded by `MAX_PROMPT_ANSWERS` like every other prompt's (R90). A pile can be long (a graveyard of
/// thirty cards picked four at a time is 27,405 sets), so a plain walk would cut off every set past the
/// first few options and leave most cards unpickable by `legalActions`, the fuzz suite and the AI. So a
/// cut drops sets, never a card, as R90's play enumeration does: first, for each option in turn, the
/// set that starts at it and takes the options after it (wrapping round) while they fit, so every
/// option the budget allows is in some listed answer at the most picks it can have; then the ordinary
/// walk, smallest sets first, for the rest of the room. Each set is listed once, in offered order (R221).
fn pick_answers(pending: &PendingChoice) -> Vec<ActionBody> {
    let count = pending.options.len();
    let mut walk = PickWalk {
        pending,
        out: Vec::new(),
        seen: IndexSet::new(),
    };

    // Every option, each in the fullest set that starts with it.
    let mut start = 0;
    while start < count && pending.max > 0 {
        if walk.fits(walk.cost_of(start)) {
            let mut set = vec![start];
            let mut spent = walk.cost_of(start);
            let mut step = 1;
            while step < count && (set.len() as i32) < pending.max {
                let next = (start + step) % count;
                if walk.fits(spent + walk.cost_of(next)) {
                    set.push(next);
                    spent += walk.cost_of(next);
                }
                step += 1;
            }
            if (set.len() as i32) >= pending.min && !walk.emit(&set) {
                return walk.out;
            }
        }
        start += 1;
    }

    // The rest, smallest sets first; costs are never negative, so an over-budget branch is cut.
    let mut size = pending.min;
    while size <= pending.max {
        if size >= 0 && !walk.walk(0, &mut Vec::new(), 0, size as usize) {
            break;
        }
        size += 1;
    }
    walk.out
}

// ---------------------------------------------------------------------------
// Re-entering a script
// ---------------------------------------------------------------------------

fn face_of(state: &GameState, def_id: &str, radiant: bool) -> Script {
    let scripts = crate::scripts::script_of(state, def_id);
    if radiant { scripts.radiant } else { scripts.base }
}

/// The hook a continuation names: a step out of a table (`resume: { picked: … }`), a hook of the
/// card's own (`cry`, `delayed`), or an event trigger by its id (`work.scriptStepFor`). Nothing
/// registered is not an error — the answer just closed the prompt (§10.6) — so this returns
/// undefined rather than throwing.
fn hook_for(script: &Script, resume: &Resume) -> Option<Hook> {
    script_step_for(script, resume)
}

/// R89: the card a continuation re-enters as, when the step was paused by a Death hook. R78 has
/// reset the instance on the board by then (or a Reborn body stands under the same id), so the hook
/// reads the snapshot taken as the unit died, which the Death pass puts in its context's data
/// (`stateCheck.runDeathPass`) and every continuation built from that context carries on.
fn self_snapshot_of(data: &IndexMap<String, Value>) -> Option<CardInstance> {
    let raw = data.get(SELF_KEY)?;
    let card = raw.as_object()?;
    let named = card.get("id").is_some_and(Value::is_string) && card.get("defId").is_some_and(Value::is_string);
    if !named {
        return None;
    }
    serde_json::from_value::<CardInstance>(raw.clone()).ok()
}

/// How an effect list ended: whole (`done`); stopped by a prompt with what was left of it parked on
/// `state.work` (`parked`); stopped by a prompt at its very last effect, so nothing was left to park
/// (`asked`); or cut short because the game ended inside it (`over`, R216).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ListStatus {
    Done,
    Parked,
    Asked,
    Over,
}

/// One list of the walk: a composed list's part is a list inside the list that holds it.
struct Frame {
    effects: Vec<Effect>,
    at: usize,
}

/// What a resumed part at `level` hands its rebuild (`PausedStep.memo`, level by level).
fn memo_at(step: &PausedStep, level: usize) -> Memo {
    step.memo.as_ref().and_then(|memo| memo.get(level)).cloned()
}

/// Apply an effect list so that a prompt in the middle of it pauses the list instead of being
/// stepped over: the effects after the one that opened the prompt are parked as a work item naming
/// this same continuation and where to continue from (§9.3, R113). Returns true when the whole list
/// ran.
///
/// A hook is a pure builder (CLAUDE.md rule 5), so re-entering it and skipping the effects that
/// already ran continues the sequence exactly; the alternative — holding the remaining `Effect[]`
/// in state — would be holding closures, which §9.3 forbids.
///
/// A composed list (a fused hook, R102) is a list of parts, each built when the walk reaches it
/// (`Effect.expand`), so an ingredient's list reads the board the ones before it left. The walk is a
/// stack of lists, and a pause parks ONE item for all of it: the parts it stood inside and the place
/// in the innermost one (`PausedStep.part`, `from`), so the continuation finishes that part and then
/// goes on with every part after it, level by level. Parking once per level instead would owe a
/// Death pass's remainder twice (`stateCheck.runDeathPass` continues the pass after its hook).
///
/// TS took `(sink, ctx, …)`; the context is that sink (it derefs to it), so it is the one argument.
pub fn apply_resumable(
    ctx: &mut EffectContext<'_>,
    plan: &ResumePlan,
    effects: Vec<Effect>,
    paused: Option<PausedStep>,
) -> bool {
    run_resumable_list(ctx, plan, effects, paused) == ListStatus::Done
}

/// `applyResumable`, saying how the list ended (`ListStatus`).
pub fn run_resumable_list(
    ctx: &mut EffectContext<'_>,
    plan: &ResumePlan,
    effects: Vec<Effect>,
    paused: Option<PausedStep>,
) -> ListStatus {
    let mut stack: Vec<Frame> = Vec::new();
    let mut memos: Vec<Memo> = Vec::new();
    let mut list = effects;
    // A resumed walk builds again the parts the pause stood inside, and only those: each part was
    // built as the walk reached it, and the ones before it have run.
    if let Some(step) = &paused {
        for (level, &at) in step.part.iter().flatten().enumerate() {
            let part = list.get(at).and_then(|effect| effect.expand.clone());
            stack.push(Frame { effects: list, at });
            let built = match part {
                None => EffectPart::default(),
                Some(expand) => expand(ctx, &memo_at(step, level)),
            };
            memos.push(built.memo);
            list = built.effects;
        }
    }
    let from = paused.as_ref().map_or(0, |step| step.from);
    stack.push(Frame { effects: list, at: from });

    loop {
        let Some(top) = stack.last() else {
            return ListStatus::Done;
        };
        if top.at >= top.effects.len() {
            // A part is done: the list that holds it goes on after it.
            stack.pop();
            memos.pop();
            let Some(parent) = stack.last_mut() else {
                return ListStatus::Done;
            };
            parent.at += 1;
            continue;
        }
        // R216: the game ended inside this list (a state check a draw's cast ran), so the rest of it
        // never resolves, and nothing is parked for a game that is over.
        if ctx.state.result.is_some() {
            return ListStatus::Over;
        }

        let effect = top.effects[top.at].clone();
        if let Some(expand) = &effect.expand {
            let built = expand(ctx, &None);
            memos.push(built.memo);
            stack.push(Frame {
                effects: built.effects,
                at: 0,
            });
            continue;
        }

        // TS compared the open prompt by identity; a prompt's id is minted from `nextId` as it opens,
        // so a prompt this effect opened is one whose id differs from the one open before it.
        let before = ctx.state.pending.as_ref().map(|pending| pending.id.clone());
        (effect.apply)(ctx);
        let last = stack.len() - 1;
        stack[last].at += 1;
        let top_at = stack[last].at;
        let opened = match &ctx.state.pending {
            None => false,
            Some(pending) => Some(&pending.id) != before.as_ref(),
        };
        if !opened {
            continue;
        }

        let left = stack.iter().enumerate().any(|(level, frame)| {
            if level == last {
                frame.at < frame.effects.len()
            } else {
                frame.at + 1 < frame.effects.len()
            }
        });
        if !left {
            return ListStatus::Asked;
        }
        let marks = run_marks(ctx);
        let nested = stack.len() > 1;
        let step = PausedStep {
            from: top_at,
            targets: ctx.targets.clone(),
            modes: ctx.modes.clone(),
            part: if nested {
                Some(stack[..last].iter().map(|frame| frame.at).collect())
            } else {
                None
            },
            memo: if nested {
                Some(memos.iter().map(|memo| memo.clone().unwrap_or(Value::Null)).collect())
            } else {
                None
            },
            exits_from: marks.exits_from,
            chosen_from: ctx.chosen_from,
            summoned: marks.summoned,
            resolving: marks.resolving,
            event_stay: marks.event_stay,
        };
        park_work(ctx, plan, step);
        return ListStatus::Parked;
    }
}

/// Re-enter the continuation a `Resume` names and apply what its step returns: the answered step of
/// a prompt, a delayed effect at its R62 point, a queued trigger, or a parked tail. Returns true
/// when the step ran to the end, false when a prompt paused it again.
///
/// The face is the one the pause recorded (§5.2), not whatever the instance is now, so a chain of
/// steps runs the text that started it. An instance that has ceased to exist resumes with
/// `ctx.self === null`, which is why a step carries what it needs in `data`.
pub fn run_resume(sink: &mut EngineSink<'_>, resume: &Resume, options: ResumeOptions) -> bool {
    let paused = paused_of(&resume.data);
    // R113: an answered step (`resumeSelf`) and a parked tail (`PausedStep`) are the run continued.
    let run = run_marks_of(&resume.data);
    let exits_from = paused
        .as_ref()
        .and_then(|step| step.exits_from)
        .or_else(|| run.as_ref().and_then(|marks| marks.exits_from));
    // The picks a tail carries were chosen when its list's were; fresh picks, when the answer made them.
    let chosen_from = if options.targets.is_none() {
        paused.as_ref().and_then(|step| step.chosen_from)
    } else {
        options.chosen_from
    };
    let summoned = paused
        .as_ref()
        .and_then(|step| step.summoned.clone())
        .or_else(|| run.as_ref().and_then(|marks| marks.summoned.clone()));
    // R98: the run began with its card in the resolving zone, and the card is its self only while it
    // is still there — a Spell its own list returned to a hand before it asked resumes with none.
    let resolving = paused.as_ref().is_some_and(|step| step.resolving == Some(true))
        || run.as_ref().is_some_and(|marks| marks.resolving == Some(true));
    // R174, R212: a queued trigger's continuation still judges its event's cards from the event.
    let event_stay = paused
        .as_ref()
        .and_then(|step| step.event_stay.clone())
        .or_else(|| run.as_ref().and_then(|marks| marks.event_stay.clone()));
    let data = card_data(&resume.data);
    let found: Option<CardInstance> = resume
        .instance_id
        .as_deref()
        .and_then(|id| find_instance(sink.state, id))
        .cloned();
    let instance: Option<CardInstance> = match self_snapshot_of(&data) {
        Some(snapshot) => Some(snapshot),
        None => {
            let still_resolving = found.as_ref().is_some_and(|card| card.zone.z() == ZoneName::Resolving);
            if resolving && !still_resolving { None } else { found }
        }
    };

    let Some(hook) = hook_for(&face_of(sink.state, &resume.def_id, resume.radiant), resume) else {
        return true;
    };

    let targets = options
        .targets
        .clone()
        .or_else(|| paused.as_ref().map(|step| step.targets.clone()))
        .unwrap_or_default();
    let modes = options
        .modes
        .clone()
        .or_else(|| paused.as_ref().map(|step| step.modes.clone()))
        .unwrap_or_default();
    let mut ctx = make_context(
        sink,
        instance.as_ref(),
        HookOptions {
            controller: options.controller.or(instance.as_ref().map(|card| card.controller)),
            targets: Some(targets),
            modes: Some(modes),
            data: Some(data.clone()),
        },
    );
    ctx.radiant = resume.radiant;
    // R127: the script this continuation named, which a step with no instance still asks again in.
    ctx.def_id = Some(resume.def_id.clone());
    // R174, R113: a paused list is the same run continued, so it keeps the mark it began with.
    if let Some(exits_from) = exits_from {
        ctx.exits_from = Some(exits_from);
    }
    if let Some(chosen_from) = chosen_from {
        ctx.chosen_from = Some(chosen_from);
    }
    // R136: and it reads the units its head summoned, in whichever action that happened.
    if let Some(summoned) = summoned.filter(|summoned| !summoned.is_empty()) {
        ctx.summoned = Some(summoned);
    }
    // R98: and it stays the resolving card's run across a further pause, card or no card.
    if resolving {
        ctx.self_resolving = Some(true);
    }
    if let Some(event_stay) = event_stay {
        ctx.event_stay = Some(event_stay);
    }

    let plan: ResumePlan = WorkPlan::new(
        Resume {
            data,
            ..resume.clone()
        },
        ctx.controller,
    );
    // A composed list (a fused hook, R102) continues in the part it stood in, then the rest.
    let effects = hook(&mut ctx);
    apply_resumable(&mut ctx, &plan, effects, paused)
}

/// TS `{ id; defId; controller; radiant }`: the card a hook is run for, which a caller may name
/// without holding the instance (an activation run, `subsystems/activate`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HookInstance {
    pub id: String,
    pub def_id: String,
    pub controller: PlayerId,
    pub radiant: bool,
}

impl From<&CardInstance> for HookInstance {
    fn from(card: &CardInstance) -> HookInstance {
        HookInstance {
            id: card.id.clone(),
            def_id: card.def_id.clone(),
            controller: card.controller,
            radiant: card.radiant,
        }
    }
}

/// `runHookResumable`'s options.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HookResumableOptions {
    pub controller: Option<PlayerId>,
    pub targets: Option<Vec<Selection>>,
    pub modes: Option<Vec<String>>,
    pub data: Option<IndexMap<String, Value>>,
    /// R174: the stays the hook's choices were made against, when they were made before the hook
    /// runs — a play's declared targets, checked at §10.5 step 1 — so a card taken off the field
    /// between the choice and the hook (a Tribute at step 2, a trap at step 4) is gone for it.
    pub exits_from: Option<u32>,
}

/// Run one of a card's own hooks the resumable way: the drop-in for `resolve.runHook` on any path
/// whose effects may open a prompt (a Cry, a trigger, an activate). Returns true when the whole
/// hook ran.
pub fn run_hook_resumable(
    sink: &mut EngineSink<'_>,
    instance: impl Into<HookInstance>,
    hook_name: &str,
    options: HookResumableOptions,
) -> bool {
    let instance: HookInstance = instance.into();
    let mut resume = resume_at(ResumeAtArgs {
        def_id: instance.def_id.clone(),
        step: String::new(),
        hook: Some(hook_name.to_string()),
        radiant: Some(instance.radiant),
        instance_id: Some(instance.id.clone()),
        data: Some(options.data.unwrap_or_default()),
    });
    if let Some(exits_from) = options.exits_from {
        let marks = RunMarks {
            exits_from: Some(exits_from),
            summoned: None,
            resolving: None,
            event_stay: None,
        };
        resume.data.insert(RUN_MARKS_KEY.to_string(), marks_json(&marks));
    }
    run_resume(
        sink,
        &resume,
        ResumeOptions {
            controller: Some(options.controller.unwrap_or(instance.controller)),
            targets: options.targets,
            modes: options.modes,
            chosen_from: None,
        },
    )
}

/// R151, §9.3: a card's start-of-game clause (§6.2, R43), run as the card arrives somewhere it can be
/// looked at — a hand, a library, the field — and at §2.1's start of the game. It is an effect list
/// like any other (§10.9), and a Choose is one of the primitives it may return, so it runs the
/// resumable way: a question in it pauses the effects after it, which are parked on `state.work` and
/// finished by the answer (R113, R122), rather than applying over the open prompt — and a second
/// question in the same list is asked in its turn instead of being dropped, since `openPrompt` never
/// overwrites one that is open. Returns true when the whole clause ran; false when it paused, which a
/// caller running a sequence of its own (setup, a draw loop) owes its remainder behind.
pub fn run_start_of_game(sink: &mut EngineSink<'_>, card: &CardInstance, controller: PlayerId) -> bool {
    // A Vanilla card carries no text (§6.3), and a card without the clause has nothing to run.
    if face_script(sink.state, card).start_of_game.is_none() {
        return true;
    }
    run_hook_resumable(
        sink,
        card,
        "startOfGame",
        HookResumableOptions {
            controller: Some(controller),
            ..HookResumableOptions::default()
        },
    )
}

/// A card's own continuation, registered at module scope the way a card registers a script and the
/// way `playSteps` registers the `"play"` sequence: `work.ts` sends it every owed item no engine
/// sequence claims, and the step it names is re-entered with the data the pause captured.
///
/// Without this, `applyResumable`'s parked tail — whose `hook` is the card's own (`cry`, `death`, a
/// trigger's), which no engine sequence ever claims — has nobody to run it, and R113's "must never
/// be dropped in silence" turns every such pause into a raise. So the registration is not a
/// convenience: it is the other half of `work.ts`'s refusal to drop a sequence.
///
/// TS registered this closure with `registerDefaultWorkHandler`; `work.rs`'s dispatcher calls it for
/// every hook no engine sequence owns (SURFACE §6.6).
pub fn default_work_handler(sink: &mut EngineSink<'_>, item: &WorkItem) {
    run_resume(
        sink,
        &item.resume,
        ResumeOptions {
            controller: Some(item.owner),
            ..ResumeOptions::default()
        },
    );
}

// ---------------------------------------------------------------------------
// Prompts inside a cast (B5 E12, R452) — play pipeline B's random-answer mode
// ---------------------------------------------------------------------------

/// R452: what becomes of a prompt opened while a random cast, or a cast that targets enemies, is being
/// driven (`randomCast.ts`). A prompt for the random cast's caster is answered at once, uniformly among
/// the answers `promptAnswers` would list, and nothing opens (null): "a random cast makes every choice
/// at random … so nothing pauses". A prompt a cast that targets enemies opens through its own text, or
/// one a random cast that does answers, offers the enemies among its target options when there is one
/// (`randomCast.preferEnemies`). Anything else is asked as it stands: the other player's prompts are
/// theirs, and an engine sequence's own question (`registerPromptAnswerer`) makes its random picks
/// itself (`playSteps`).
fn cast_prompt_shape(sink: &mut EngineSink<'_>, args: OpenPromptArgs) -> Option<OpenPromptArgs> {
    if has_answerer(&args.resume.hook) {
        return Some(args);
    }
    let Some(mode) =
        crate::random_cast::cast_mode_for_prompt(sink.state, args.player, args.resume.instance_id.as_deref())
    else {
        return Some(args);
    };
    let max = clamp(args.max.unwrap_or(1), 0, args.options.len() as i32);
    let required = clamp(args.min.unwrap_or(1), 0, max);
    let options: Vec<PromptOption> = if mode.target_enemies {
        if args.aim == Some(TargetAim::Help) {
            crate::random_cast::prefer_friends(
                sink.state,
                args.player,
                &args.options,
                |option: &PromptOption| option.selection.clone(),
                required,
            )
        } else {
            crate::random_cast::prefer_enemies(
                sink.state,
                args.player,
                &args.options,
                |option: &PromptOption| option.selection.clone(),
                required,
            )
        }
    } else {
        args.options.clone()
    };
    let shaped = OpenPromptArgs { options, ..args };
    if !mode.random {
        return Some(shaped);
    }
    answer_at_random(sink, &shaped);
    None
}

/// R452: answer a prompt that never opens — one of the answers `promptAnswers` lists for it, drawn
/// uniformly from the match rng — and re-enter the step it names with that selection, exactly as
/// `answerPrompt` does, inside the effect that asked: the step runs first and the rest of that effect's
/// list after it, the order a parked tail would have kept (R113). Nothing is emitted for the prompt,
/// since none was open. A prompt with no answer at all resolves into nothing.
fn answer_at_random(sink: &mut EngineSink<'_>, args: &OpenPromptArgs) {
    let max = clamp(args.max.unwrap_or(1), 0, args.options.len() as i32);
    let probe = PendingChoice {
        id: String::new(),
        player_id: args.player,
        kind: args.kind,
        prompt: args.prompt.clone(),
        options: args.options.clone(),
        min: clamp(args.min.unwrap_or(1), 0, max),
        max,
        budget: None,
        resume: args.resume.clone(),
    };
    let answers = prompt_answers(&probe);
    if answers.is_empty() {
        return;
    }
    let drawn = sink.rng.int(answers.len() as i32);
    let Some(ActionBody::Answer { selection, .. }) = usize::try_from(drawn).ok().and_then(|at| answers.get(at)) else {
        return;
    };
    let targets = in_offered_order(&probe, selection);
    let chosen_from = exit_mark(sink.state);
    run_resume(
        sink,
        &resume_of(&probe.resume),
        ResumeOptions {
            controller: Some(args.player),
            targets: Some(targets),
            modes: None,
            chosen_from: Some(chosen_from),
        },
    );
}
