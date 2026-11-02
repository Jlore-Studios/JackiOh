//! Prompts: `state.pending`, the kinds of SPEC §10.6 (B5 E18's included), and the serializable
//! continuation that makes answering one re-enter the script that asked (BUILD M3-T3; SURFACE §4).
//!
//! §9.3: "Mid-action choices are state, not callbacks." A paused sequence is `state.pending.resume`
//! (answering re-enters that step with the selection in `ctx.targets`) and, in `state.work`, the rest
//! of the effect list it interrupted, parked through `work.rs`, which decides the order a cascade
//! resumes in (R113's cursor). Option data lives only in `state.pending.options`, so §10.8's
//! `view_for` has one place to hide from the opponent.
//!
//! This module runs no state check and dispatches no trigger (`triggers::settle` owns §10.3's loop,
//! R59), but answering drains `state.work` (R113): a caller that answers without `reduce` would
//! otherwise leave a Spell short of its graveyard.

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::MAX_PROMPT_ANSWERS;
use crate::resolve::{HookOptions, make_context};
use crate::script::{Effect, EffectContext, EffectPart, EngineSink, Hook, Memo, Script};
use crate::state::{
    CardInstance, EngineError, GameState, PendingChoice, PromptOption, Resume, find_instance,
};
use crate::wire::{ActionBody, GameEvent, PlayerId, PromptKind, Row, Selection, TargetAim, ZoneName};
use crate::work::{
    PausedStep, RUN_MARKS_KEY, RunMarks, WorkPlan, begin_work_cascade, card_data, drain_work, park_work,
    paused_of, run_marks_of, script_step_for,
};

/// Where a Death hook's context keeps the snapshot of the unit as it died (R89), so a continuation
/// built from that context reads the card the hook did, not the instance R78 has reset since.
pub const SELF_KEY: &str = "__self";

/// The seventeen kinds of §10.6. B5 E18 adds `number`, `answer` (whose key never leaves the engine,
/// R465), `cell`, `reward` and `pick`; ME-CRAFT adds `craft` (R880) and ME-MARKET `market` (R1000). This
/// module reads the kind for the mulligan, which §2.1 answers with its own action, and for `pick`,
/// whose answers it enumerates its own way.
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
    PromptKind::Craft,
    PromptKind::Market,
];

/// The `Script` key holding the step table a prompt answer re-enters (`resume: { picked: … }`).
pub const RESUME_HOOK: &str = "resume";

// Answers are bounded like `legal_actions`' other lists (R90): `MAX_PROMPT_ANSWERS` (CLAUDE.md rule 9).

/// `Resume` is §10.6's serializable continuation, never a closure (nothing here adds a field a JSON
/// round-trip would lose); its `hook` names a step table (`resume`, whose entry `step` picks) or a hook
/// of the card's own. A plan adds the player whose sequence it is.
pub type ResumePlan = WorkPlan;

/// `open_prompt`'s argument (data: a test builds it with `json_as`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
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
    /// B5 E18: whose sequence the answer continues, when that is not the player asked (Classic #8's mode
    /// prompt, #9's hand pick). The answered step runs as this player, the card's controller, so "you
    /// draw" is still the asking card's controller's draw. Absent is the player asked.
    pub owner: Option<PlayerId>,
    pub resume: Resume,
}

/// B5 E18: where a prompt held by a player other than the asking card's controller keeps that
/// controller (`OpenPromptArgs.owner`), in its resume data. It belongs to that one prompt: a
/// continuation built from the answered step (`resume_self`) drops it.
pub const PROMPT_OWNER_KEY: &str = "__owner";

/// R465: where an `answer` prompt keeps the id of its correct option: in its resume data, which
/// `view_for` never sends (§10.8) and the AI's redaction strips (R185). Like `PROMPT_OWNER_KEY` it
/// belongs to the one prompt that asked, so `resume_self` drops it.
pub const ANSWER_KEY: &str = "__answerKey";

/// The controller an answered prompt's step runs as (`PROMPT_OWNER_KEY`), else the player asked.
pub fn prompt_owner_of(pending: &PendingChoice) -> PlayerId {
    match pending.resume.data.get(PROMPT_OWNER_KEY).and_then(Value::as_str) {
        Some("p1") => PlayerId::P1,
        Some("p2") => PlayerId::P2,
        _ => pending.player_id,
    }
}

/// R465: the correct option an `answer` prompt's data holds; `None` in the AI's redacted copy (R185).
pub fn answer_key_of(data: &IndexMap<String, Value>) -> Option<String> {
    data.get(ANSWER_KEY).and_then(Value::as_str).map(str::to_string)
}

/// R1184000: the key of a sequence question's Stop option, a `none` pick as `choose_cell`'s Done and
/// a night market's Leave are (R1000). It is listed after the lettered answers and is never the key.
pub const ANSWER_STOP_KEY: &str = "none";

/// R1184001: the answer that stops the question sequence open in `state` (Meditative+ #20), which a
/// timeout gives instead of the AI policy's draw, as it leaves a night market (R1002): it draws
/// nothing and summons the Kenny the right answers so far have earned.
pub fn stop_answer(state: &GameState) -> Option<ActionBody> {
    let pending = state.pending.as_ref()?;
    if pending.kind != PromptKind::Answer || pending.streak.is_none() {
        return None;
    }
    pending
        .options
        .iter()
        .find(|option| option.key == ANSWER_STOP_KEY)
        .map(|option| ActionBody::Answer {
            choice_id: pending.id.clone(),
            selection: vec![option.selection.clone()],
        })
}

/// R1563006: a question of a sequence open in a state (Meditative+ #20 KY's Competition), as a host's
/// sequence clock reads it: who answers it, the right answers so far, and the prompt's id. All public:
/// the opponent's view shows that a prompt is open, and the streak rides the prompt (R1184000).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SequenceQuestion {
    pub player: PlayerId,
    pub streak: i32,
    pub prompt_id: String,
}

/// R1563006: the question of a sequence open in `state`, or `None` with none open. A sequence's first
/// question has a streak of 0, so a host starts its clock there and stops it when none is open.
pub fn sequence_question_of(state: &GameState) -> Option<SequenceQuestion> {
    let pending = state.pending.as_ref()?;
    if pending.kind != PromptKind::Answer {
        return None;
    }
    Some(SequenceQuestion {
        player: pending.player_id,
        streak: pending.streak?,
        prompt_id: pending.id.clone(),
    })
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

// The stored records

/// The continuation a `PendingChoice`, `WorkItem`, `QueuedTrigger` or `DelayedEffect` carries, as every
/// reader of a stored resume sees it; serde refuses a record without its fields, so it is a copy.
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

/// `resume_at`'s argument (data: a card builds it with `json_as`).
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

/// The continuation of the script running now: the same card, face and captured data plus what this
/// step adds. B5 E14, R546: its script is `ctx.def_id`, which a step with no instance still has (R127,
/// R113), and the instance's only where that is unset: a copier (Classic #57 Echo) runs the copied
/// Spell's text as `self`, and its continuation must come back to that text.
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
            .or_else(|| self_.map(|card| crate::subsystems::unit_text::text_def_of(card).0.to_string()))
            .unwrap_or_default(),
        step: step.to_string(),
        hook: None,
        radiant: Some(ctx.radiant),
        instance_id: self_.map(|card| card.id.clone()),
        data: Some(merged),
    });
    // R113, §10.6: the step re-entered is the same run, reading the stays it began with (R174) and the
    // units it summoned (R136) in whichever action it resumes; a delayed effect is not and drops this.
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

/// What a continuation of this run carries of it.
fn run_marks(ctx: &EffectContext<'_>) -> RunMarks {
    let summoned = summoned_so_far(ctx);
    RunMarks {
        exits_from: Some(
            ctx.exits_from
                .unwrap_or_else(|| crate::stays::exit_mark(&*ctx.state)),
        ),
        summoned: if summoned.is_empty() { None } else { Some(summoned) },
        resolving: if ctx.self_resolving == Some(true) {
            Some(true)
        } else {
            None
        },
        event_stay: ctx.event_stay.clone(),
    }
}

/// A `RunMarks` as the JSON block `RUN_MARKS_KEY` holds (only its present keys).
fn marks_json(marks: &RunMarks) -> Value {
    serde_json::to_value(marks).expect("run marks are plain JSON (§9.3)")
}

// Opening and closing

fn clamp(value: i32, low: i32, high: i32) -> i32 {
    low.max(high.min(value))
}

/// Open the one prompt §10.1 allows, or `None` with no options (the effect fizzles and the card still
/// resolves, §6.3) or a prompt already open: a second ask never overwrites one, so a script chains steps.
pub fn open_prompt(sink: &mut EngineSink<'_>, args: OpenPromptArgs) -> Option<PendingChoice> {
    if sink.state.pending.is_some() {
        return None;
    }
    if args.options.is_empty() {
        return None;
    }
    // B5 E12, R452: a random cast's caster is answered for at once (`cast_prompt_shape`).
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
            resume.data.insert(
                PROMPT_OWNER_KEY.to_string(),
                Value::String(owner.as_str().to_string()),
            );
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
        craft_mode: None,
        craft_offer: None,
        streak: None,
        resume,
    };

    sink.state.next_id += 1;
    sink.state.pending = Some(pending.clone());
    sink.events.push(GameEvent::PromptOpened {
        player: args.player,
        choice_id: Some(pending.id.clone()),
        kind: Some(args.kind),
    });
    Some(pending)
}

/// Clear the open prompt and say so (§10.10 animates `promptAnswered`).
pub fn close_prompt(sink: &mut EngineSink<'_>) -> Option<PendingChoice> {
    let pending = sink.state.pending.take()?;
    sink.events.push(GameEvent::PromptAnswered {
        player: pending.player_id,
        choice_id: Some(pending.id.clone()),
    });
    Some(pending)
}

// Answering

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
        (Selection::Craft { recipe: x }, Selection::Craft { recipe: y }) => x == y,
        (Selection::None, Selection::None) => true,
        _ => false,
    }
}

/// §10.6, §10.8: an option's key, what the client sends back to answer it, built from the selection
/// (unique among the options), never from a label: two cards of one name are two options (R177).
pub fn option_key(selection: &Selection) -> String {
    match selection {
        Selection::Instance { instance_id } => format!("instance:{instance_id}"),
        Selection::Hero { player } => format!("hero:{player}"),
        Selection::Zone { player, row, lane } => format!("zone:{player}:{row}:{lane}"),
        Selection::Mode { option } => format!("mode:{option}"),
        // ME-CRAFT (R880): a recipe names its definition, as its prompt's option key does.
        Selection::Craft { recipe } => crate::subsystems::craft::craft_id(recipe),
        Selection::None => "none".to_string(),
    }
}

/// §10.6: a prompt names a hero to the player it is for as "Your" or "Enemy", never by seat id. Only
/// that player is sent the options (R81), so "Your" is the reader's own; the client's pickers word a
/// play's own choices the same way (`selectionLabel` in apps/web/src/game/Prompt.tsx).
pub fn hero_option_label(player: PlayerId, chooser: PlayerId) -> String {
    format!("{} hero", if player == chooser { "Your" } else { "Enemy" })
}

/// §10.6: a board cell, named to its chooser as `hero_option_label` names a hero ("Enemy Unit lane 3").
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
        Selection::Craft { recipe } => recipe.name(),
        Selection::None => "nothing".to_string(),
    }
}

/// Why an `answer` is not a legal answer to the open prompt, as `Err`'s text (SURFACE §4.4.9). Only what
/// `options` offered may be picked, each at most once, and the count must sit inside `min`/`max`
/// (M3-T3; R60: N picks are N different cards).
pub fn why_answer_refused(pending: &PendingChoice, answer: &AnswerInput) -> Result<(), EngineError> {
    if pending.kind == PromptKind::Mulligan {
        return Err(EngineError::new(
            "a mulligan is answered with the mulligan action (§2.1)",
        ));
    }
    if answer.choice_id != pending.id {
        return Err(EngineError::new(format!(
            "no prompt {} is open",
            answer.choice_id
        )));
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
            format!(
                "that prompt takes {} to {} picks, got {count}",
                pending.min, pending.max
            )
        }));
    }

    // ME-CRAFT (R880): a `craft` prompt's answer is a recipe, not one of the options, so it is
    // validated against the prompt's budget instead of the membership and spend checks below.
    if pending.kind == PromptKind::Craft {
        let Some(Selection::Craft { recipe }) = picks.first() else {
            return Err(EngineError::new("that prompt takes a crafted recipe"));
        };
        // R1182000, R1182002: and in the frame the prompt's mode names, if any; R1563007: of the blocks
        // its offer holds, if it has one.
        return match crate::subsystems::craft::validate_answer(
            recipe,
            pending.budget.unwrap_or(-1),
            pending.craft_mode,
            pending.craft_offer.as_ref(),
        ) {
            Ok(()) => Ok(()),
            Err(reasons) => Err(EngineError::new(format!(
                "that recipe cannot be crafted: {}",
                reasons.join("; ")
            ))),
        };
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
            return Err(EngineError::new(format!(
                "{} is picked twice",
                name_of(pick, pending.player_id)
            )));
        };
        used.insert(free);
    }
    // B5 E18: a `pick` prompt's picks may cost no more than its budget together (Classic #44).
    if let Some(budget) = pending.budget {
        let spent: i32 = used
            .iter()
            .map(|&index| {
                pending
                    .options
                    .get(index)
                    .and_then(|option| option.cost)
                    .unwrap_or(0)
            })
            .sum();
        if spent > budget {
            return Err(EngineError::new(format!(
                "those picks cost {spent} together, over the budget of {budget}"
            )));
        }
    }
    Ok(())
}

/// R122, R113: a prompt an engine sequence opened for itself (the play pipeline's own questions, an
/// Echo repeat's fresh picks and a cast's choices, §10.5 steps 4 and 6) names that sequence as its
/// `resume.hook`, which no card script holds, so re-entering it as a card's continuation would lose the
/// selection and the rest of the play; `answer_owned` calls each owner directly (SURFACE §6.6).
fn has_answerer(hook: &str) -> bool {
    matches!(
        hook,
        "play" | "plague:placement" | "fuse:onto" | "@triggerCry" | "@attackSummon"
    )
}

/// The answer of the engine sequence a prompt's hook names, or `None` when no engine sequence owns
/// it and the prompt re-enters a card's own script step.
fn answer_owned(
    sink: &mut EngineSink<'_>,
    hook: &str,
    answer: &AnswerInput,
) -> Option<Result<(), EngineError>> {
    match hook {
        // `play_steps::PLAY_WORK_KIND`: the play pipeline's own questions (§10.5).
        "play" => Some(crate::play_steps::answer_play_prompt(sink, answer)),
        // `PLAGUE_PLACEMENT_HOOK`: B5 E19's placement pick.
        "plague:placement" => Some(crate::effects::plague::answer_placement(sink, answer)),
        // `FUSE_ONTO_HOOK`: the permanent a Fuse goes onto.
        "fuse:onto" => Some(crate::effects::fuse::answer_fuse_onto(sink, answer)),
        // `TRIGGER_CRY_HOOK`: a Cry a trigger fires.
        "@triggerCry" => Some(crate::cry_trigger::answer_cry_prompt(sink, answer)),
        _ => None,
    }
}

/// §10.6: "`answer` re-invokes the script with the selection." Validate, close the prompt, re-enter the step
/// its `resume` names with the selection in `ctx.targets`, then drain what the prompt interrupted in R113's
/// order (a pause inside this step lands ahead of everything older) up to the next prompt, so one answer
/// finishes one sequence; the state check and triggers stay with `triggers::settle` (R59). A prompt an engine
/// sequence opened goes to its answerer, so every caller answers every prompt through this one entry point
/// (R122).
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
    // B5 E5, E9, R450: a cost the targeting point asks for first (its own answer then finishes this
    // one), and the pick an interception moves.
    let targeted: Vec<Selection> = crate::targeting_point::target_answer(sink, &pending, &picks);
    // R1180005: a `target` prompt a resolving Spell's own script opened targets what it picks.
    if pending.kind == PromptKind::Target
        && let Some(asking) = pending.resume.instance_id.as_deref()
    {
        crate::targeted::note_spell_targets(sink.state, asking, &targeted);
    }
    continue_answer(sink, &pending, &targeted);
    Ok(())
}

/// The rest of an answer once its prompt is closed: re-enter the paused step with the picks, then drain
/// what it interrupted (R113, R122). An answer the targeting point interrupted for a cost (R450) ends
/// here too, as does a caller answering without the targeting point (a random pick targets nothing).
pub fn continue_answer(sink: &mut EngineSink<'_>, pending: &PendingChoice, picks: &[Selection]) {
    // R113, R122: re-entering the paused step takes it up again, so the cursor resets and a pause inside
    // it parks its own tail ahead of everything still owed, not where the last action left the cursor.
    begin_work_cascade(sink);
    let chosen_from = crate::stays::exit_mark(sink.state);
    run_resume(
        sink,
        &resume_of(&pending.resume),
        ResumeOptions {
            // B5 E18: a prompt the other player held continues the asking card's sequence, as its controller.
            controller: Some(prompt_owner_of(pending)),
            targets: Some(picks.to_vec()),
            modes: None,
            // R174, §10.6: the picks as the prompt offered them, on the stays they stand on now.
            chosen_from: Some(chosen_from),
        },
    );
    drain_work(sink);
}

// The targeting point (B5 E5, E9, E35; R450): `targeting_point.rs`'s options, refusal and answer, which
// `open_prompt` and `answer_prompt` call directly.

/// R221: an answer's picks are a set (R60's "N different cards"), taken in the order the prompt offered
/// them: `legal_actions` offers each set once and `reduce` accepts any listing of it, so the listing must
/// not change what the answer does (a radiant #26's Echo repeat makes its two picks Radiant in turn;
/// R354 made #80 Zao Gao's discard random).
pub fn in_offered_order(pending: &PendingChoice, selection: &[Selection]) -> Vec<Selection> {
    let mut used: IndexSet<usize> = IndexSet::new();
    let mut placed: Vec<(usize, Selection)> = Vec::with_capacity(selection.len());
    for pick in selection {
        let at =
            pending.options.iter().enumerate().position(|(index, option)| {
                !used.contains(&index) && same_selection(&option.selection, pick)
            });
        if let Some(at) = at {
            used.insert(at);
        }
        placed.push((at.unwrap_or(usize::MAX), pick.clone()));
    }
    placed.sort_by_key(|a| a.0);
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

/// `prompt_answers`' walk: every set of `size` options from `start` on, in offered order. Returns false
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

/// Every answer the open prompt would accept, for `legal_actions` and the §10.7 policy that draws
/// from it (M3-T3). Options are taken in offered order, so the list is the same on every machine; a
/// mulligan has its own action and is not enumerated here.
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
        // A negative size never fills, so it lists nothing.
        if size >= 0 && !walk_answers(pending, 0, &mut Vec::new(), size as usize, &mut out) {
            break;
        }
        size += 1;
    }
    out
}

/// `pick_answers`' state: the answers listed so far and the sets already listed.
struct PickWalk<'a> {
    pending: &'a PendingChoice,
    out: Vec<ActionBody>,
    seen: IndexSet<String>,
}

impl PickWalk<'_> {
    fn cost_of(&self, index: usize) -> i32 {
        self.pending
            .options
            .get(index)
            .and_then(|option| option.cost)
            .unwrap_or(0)
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

/// B5 E18: a `pick` prompt's answers: every set of `min` to `max` options whose costs fit the budget, bounded
/// by `MAX_PROMPT_ANSWERS` (R90). A pile can be long and a plain walk would leave most cards unpickable, so a
/// cut drops sets, never a card: first, for each option, the fullest set that starts at it (wrapping round),
/// then the ordinary walk, smallest sets first. Each set is listed once, in offered order (R221).
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

    // The rest, smallest sets first.
    let mut size = pending.min;
    while size <= pending.max {
        if size >= 0 && !walk.walk(0, &mut Vec::new(), 0, size as usize) {
            break;
        }
        size += 1;
    }
    walk.out
}

// Re-entering a script

fn face_of(state: &GameState, def_id: &str, radiant: bool) -> crate::scripts::ScriptRef {
    crate::scripts::face_ref(state, def_id, radiant)
}

/// The hook a continuation names: a step out of a table (`resume: { picked: … }`), a hook of the
/// card's own (`cry`, `delayed`), or an event trigger by its id. Nothing registered is not an error
/// (the answer just closed the prompt, §10.6), so this answers `None`.
fn hook_for(script: &Script, resume: &Resume) -> Option<Hook> {
    script_step_for(script, resume)
}

/// R89: the card a continuation re-enters as, when the step was paused by a Death hook. R78 has
/// reset the instance on the board by then (or a Reborn body stands under the same id), so the hook
/// reads the snapshot taken as the unit died, which the Death pass puts in its context's data
/// (`stateCheck.runDeathPass`) and every continuation built from that context carries on.
pub(crate) fn self_snapshot_of(data: &IndexMap<String, Value>) -> Option<CardInstance> {
    let raw = data.get(SELF_KEY)?;
    let card = raw.as_object()?;
    let named =
        card.get("id").is_some_and(Value::is_string) && card.get("defId").is_some_and(Value::is_string);
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

/// Apply an effect list so that a prompt in the middle of it pauses the list instead of being stepped over:
/// the effects after the one that opened the prompt are parked as a work item naming this same continuation
/// and where to continue from (§9.3, R113). Returns true when the whole list ran. A hook is a pure builder
/// (CLAUDE.md rule 5), so re-entering it and skipping what already ran continues exactly; holding the
/// remaining effects in state would hold closures, which §9.3 forbids. A composed list (a fused hook, R102)
/// builds each part as the walk reaches it (`Effect.expand`), so an ingredient's list reads the board the
/// ones before it left. A pause parks ONE item for the whole stack (`PausedStep.part`, `from`): parking once
/// per level would owe a Death pass's remainder twice.
pub fn apply_resumable(
    ctx: &mut EffectContext<'_>,
    plan: &ResumePlan,
    effects: Vec<Effect>,
    paused: Option<PausedStep>,
) -> bool {
    run_resumable_list(ctx, plan, effects, paused) == ListStatus::Done
}

/// `apply_resumable`, saying how the list ended (`ListStatus`).
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
    stack.push(Frame {
        effects: list,
        at: from,
    });

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

        // A prompt's id is minted from `next_id` as it opens, so a prompt this effect opened is one
        // whose id differs from the one open before it.
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
                Some(
                    memos
                        .iter()
                        .map(|memo| memo.clone().unwrap_or(Value::Null))
                        .collect(),
                )
            } else {
                None
            },
            exits_from: marks.exits_from,
            chosen_from: ctx.chosen_from,
            summoned: marks.summoned,
            resolving: marks.resolving,
            event_stay: marks.event_stay,
        };
        park_work(ctx, plan, &step);
        return ListStatus::Parked;
    }
}

/// Re-enter the continuation a `Resume` names and apply what its step returns: the answered step of a prompt,
/// a delayed effect at its R62 point, a queued trigger, or a parked tail. Returns true when the step ran to
/// the end, false when a prompt paused it again. The face is the one the pause recorded (§5.2), so a chain of
/// steps runs the text that started it; an instance that has ceased to exist resumes with no `self`, so a
/// step carries what it needs in `data`.
pub fn run_resume(sink: &mut EngineSink<'_>, resume: &Resume, options: ResumeOptions) -> bool {
    // R1855001: a run whose caster's choices are random is random in every continuation of it too.
    let mode = crate::random_cast::random_choices_of(resume);
    crate::random_cast::with_cast_mode(sink, mode, |sink| run_resume_now(sink, resume, options))
}

/// `run_resume`, under whatever mode is in force.
fn run_resume_now(sink: &mut EngineSink<'_>, resume: &Resume, options: ResumeOptions) -> bool {
    let paused = paused_of(&resume.data);
    // R113: an answered step (`resume_self`) and a parked tail (`PausedStep`) are the run continued.
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
            let still_resolving = found
                .as_ref()
                .is_some_and(|card| card.zone.z() == ZoneName::Resolving);
            if resolving && !still_resolving {
                None
            } else {
                found
            }
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
            controller: options
                .controller
                .or(instance.as_ref().map(|card| card.controller)),
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

/// The card a hook is run for, which a caller may name without holding the instance (an activation).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HookInstance {
    pub id: String,
    pub def_id: String,
    pub controller: PlayerId,
    pub radiant: bool,
}

impl From<&CardInstance> for HookInstance {
    fn from(card: &CardInstance) -> HookInstance {
        // R1850000: a copier's hooks are the copied text's, and their continuations name it.
        let (def_id, radiant) = crate::subsystems::unit_text::text_def_of(card);
        HookInstance {
            id: card.id.clone(),
            def_id: def_id.to_string(),
            controller: card.controller,
            radiant,
        }
    }
}

/// `run_hook_resumable`'s options.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
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

/// Run one of a card's own hooks the resumable way: the drop-in for `resolve::run_hook` on any path
/// whose effects may open a prompt (a Cry, a trigger, an activate). Returns true when it all ran.
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

/// R151, §9.3: a card's start-of-game clause (§6.2, R43), run as the card arrives somewhere it can be looked
/// at (a hand, a library, the field) and at §2.1's start of the game. It may Choose (§10.9), so it runs the
/// resumable way: a question pauses the effects after it (R113, R122), and a second one is asked in its turn
/// instead of dropped. Returns false when it paused, which a caller running a sequence of its own (setup, a
/// draw loop) owes its remainder behind.
pub fn run_start_of_game(sink: &mut EngineSink<'_>, card: &CardInstance, controller: PlayerId) -> bool {
    // A Vanilla card carries no text (§6.3), and a card without the clause has nothing to run.
    if crate::scripts::script_of(sink.state, card)
        .start_of_game
        .is_none()
    {
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

/// R781004: a card's `fusedInto` hook, run on the card a Fuse keeps once the text that carries it is
/// fused into it (`subsystems::fuse::fuse`). Resumable like the start-of-game clause beside it.
pub fn run_fused_into(sink: &mut EngineSink<'_>, card: &CardInstance, controller: PlayerId) -> bool {
    if crate::scripts::script_of(sink.state, card).fused_into.is_none() {
        return true;
    }
    run_hook_resumable(
        sink,
        card,
        "fusedInto",
        HookResumableOptions {
            controller: Some(controller),
            ..HookResumableOptions::default()
        },
    )
}

/// R1849001: a Unit's `evolve` hook ("Evolve: X", Stinkyscape #91), run when an effect evolves it
/// (`effects::evolve`). Resumable like the `fusedInto` hook above. Returns true when the whole hook ran.
pub fn run_evolve(sink: &mut EngineSink<'_>, card: &CardInstance, controller: PlayerId) -> bool {
    if crate::scripts::script_of(sink.state, card).evolve.is_none() {
        return true;
    }
    run_hook_resumable(
        sink,
        card,
        "evolve",
        HookResumableOptions {
            controller: Some(controller),
            ..HookResumableOptions::default()
        },
    )
}

/// MD-B18, R925: a card's "When this enters your hand" clause, run as the card arrives in a hand,
/// after R151's start-of-game clause (`draw::run_arrival_hooks`), and resumable like it.
pub fn run_enters_hand(sink: &mut EngineSink<'_>, card: &CardInstance, controller: PlayerId) -> bool {
    if crate::scripts::script_of(sink.state, card).enters_hand.is_none() {
        return true;
    }
    run_hook_resumable(
        sink,
        card,
        "entersHand",
        HookResumableOptions {
            controller: Some(controller),
            ..HookResumableOptions::default()
        },
    )
}

// Prompts inside a cast (B5 E12, R452) — play pipeline B's random-answer mode

/// R452: what becomes of a prompt opened while a random cast, or a cast that targets enemies, is being
/// driven. A prompt for the random cast's caster is answered at once, uniformly among `prompt_answers`, and
/// nothing opens. A prompt a cast that targets enemies opens (or a random one that does answers) offers the
/// enemies among its target options when there is one. Anything else is asked as it stands: the other
/// player's prompts are theirs, and an engine sequence's own question makes its random picks itself
/// (`play_steps`).
fn cast_prompt_shape(sink: &mut EngineSink<'_>, args: OpenPromptArgs) -> Option<OpenPromptArgs> {
    if has_answerer(&args.resume.hook) {
        return Some(args);
    }
    // R1200: while a Mayor acts a `target` prompt is answered at once at random, for whichever player
    // it opened for, meeting no targeting point (as R452's picks). R1201: the opener's Lucky rolls again.
    if args.kind == PromptKind::Target && crate::random_targets::targets_random(sink.state) {
        let shaped = OpenPromptArgs {
            options: crate::targeting_point::targetable_options(sink.state, &args),
            ..args
        };
        if shaped.options.is_empty() {
            return None;
        }
        let luck = crate::random_targets::mayor_luck(sink.state, shaped.player);
        answer_at_random(sink, &shaped, luck);
        return None;
    }
    let Some(mode) =
        crate::random_cast::cast_mode_for_prompt(sink.state, args.player, args.resume.instance_id.as_deref())
    else {
        return Some(args);
    };
    let max = clamp(args.max.unwrap_or(1), 0, args.options.len() as i32);
    let required = clamp(args.min.unwrap_or(1), 0, max);
    let options: Vec<PromptOption> = if mode.friends_only && args.kind == PromptKind::Target {
        // R1850004: a cast for its caster's own side offers its targets there alone; with nothing
        // there the question is passed, as a declaration with no legal option is.
        let friends = crate::random_cast::friends_only(
            sink.state,
            args.player,
            &args.options,
            |option: &PromptOption| option.selection.clone(),
        );
        if friends.is_empty() {
            return None;
        }
        friends
    } else if mode.target_enemies {
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
    answer_at_random(sink, &shaped, 0);
    None
}

/// R452: answer a prompt that never opens with one of `prompt_answers`' answers, drawn uniformly from the
/// match rng, and re-enter its step as `answer_prompt` does, inside the effect that asked: the step runs
/// first and the rest of that effect's list after it, the order a parked tail would have kept (R113). Nothing
/// is emitted, since no prompt was open. R1201: `luck` rolls again and keeps the answer with more picks on
/// the prompt's aim side (Mayor Medinamogger's Radiant face).
fn answer_at_random(sink: &mut EngineSink<'_>, args: &OpenPromptArgs, luck: i32) {
    let max = clamp(args.max.unwrap_or(1), 0, args.options.len() as i32);
    let probe = PendingChoice {
        id: String::new(),
        player_id: args.player,
        kind: args.kind,
        prompt: args.prompt.clone(),
        options: args.options.clone(),
        min: clamp(args.min.unwrap_or(1), 0, max),
        max,
        // B5 E18: the budget `open_prompt` gives the prompt, so a random answer is one a player could pick.
        budget: args.budget.map(|budget| budget.max(0)),
        craft_mode: None,
        craft_offer: None,
        streak: None,
        resume: args.resume.clone(),
    };
    let answers = prompt_answers(&probe);
    if answers.is_empty() {
        return;
    }
    let state: &GameState = sink.state;
    let sided = |at: i32| -> usize {
        usize::try_from(at)
            .ok()
            .and_then(|at| answers.get(at))
            .map(|answer| match answer {
                ActionBody::Answer { selection, .. } => selection
                    .iter()
                    .filter(|pick| {
                        if args.aim == Some(TargetAim::Help) {
                            crate::random_cast::is_friendly_pick(state, args.player, pick)
                        } else {
                            crate::random_cast::is_enemy_pick(state, args.player, pick)
                        }
                    })
                    .count(),
                _ => 0,
            })
            .unwrap_or(0)
    };
    let drawn = sink.rng.lucky(
        luck,
        |rng| rng.int(answers.len() as i32),
        |first, second| {
            if sided(second) > sided(first) {
                second
            } else {
                first
            }
        },
    );
    let Some(ActionBody::Answer { selection, .. }) =
        usize::try_from(drawn).ok().and_then(|at| answers.get(at))
    else {
        return;
    };
    let targets = in_offered_order(&probe, selection);
    let chosen_from = crate::stays::exit_mark(sink.state);
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
