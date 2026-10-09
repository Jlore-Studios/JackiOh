//! Trigger a Cry (docs/classic-sets.md B5 E13, R467): run a Unit's Cry again, on the field or out of a
//! graveyard, for the player whose card triggered it (Classic #54 Rewind).
//!
//! The rules, in the order the sequence meets them:
//!   - Which card. A Unit whose running face has a Cry: the top of a unit pile (a dormant card under a
//!     Stack is not on the field, R13) or a Unit card in a graveyard. Anything else triggers nothing.
//!   - Who. The triggering card's controller runs it and makes its choices (R70), whoever controls or
//!     owns the Unit — Rewind's Radiant reaches the other player's Units too.
//!   - Its choices. A Cry's declared targets and modes (R81) travel in the play that played the Unit,
//!     and a triggered Cry has no play, so they are asked as prompts, declaration by declaration, as a
//!     cast's are (R70) — modes first when a target declaration hangs on them (R90's `forModes`). A
//!     declaration the board cannot satisfy is skipped (§8's conventions), and a Tribute declaration is
//!     a play's price, which a triggered Cry never pays: its slots are left empty (`{ pick: "none" }`)
//!     so the declarations after it still read their own (R90, R123).
//!   - "This". On the field the Unit is the Cry's `self`. Out of a graveyard the Cry runs with no card
//!     at all (`self` null, its definition named as R127 names a ceased card's), so an effect aimed at
//!     "this" finds nothing, a pool still excludes the definition (§5.1), and the Cry reads X as 0.
//!   - Not a play. Nothing is played, cast or announced: no `cardPlayed`, no counters (R55, E4), no
//!     Echo, no Combo. The Cry's own effects are ordinary effects.
//!   - Pauses. Each question is a prompt this module opened for itself, so its answer comes back here
//!     (`prompts.rs` matches `TRIGGER_CRY_HOOK` to `answer_cry_prompt`, R122; TS registered it with
//!     `registerPromptAnswerer`, SURFACE §6.6), and the Cry that runs once they are answered is the
//!     card's own resumable hook (`prompts::run_hook_resumable`): a prompt inside it parks its own tail
//!     on `state.work` ahead of whatever the triggering list still owes (R113). The run record in the
//!     prompt's resume is plain JSON, so a paused trigger survives a round trip and replays exactly.
//!   - Leaving. A Unit that has left the field since the trigger began (R174), or left the graveyard,
//!     by the time its choices are made triggers nothing.
//!
//! Port of `packages/engine/src/cryTrigger.ts`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::play_choices::{
    DECLARATION_SLICES_KEY, active_target_decls, declaration_slices, declared_modes, declared_targets,
    legal_selections_for, targets_follow_modes,
};
use crate::prompts::{
    AnswerInput, HookResumableOptions, OpenPromptArgs, ResumeAtArgs, ResumeOptions, cell_option_label,
    close_prompt, hero_option_label, in_offered_order, open_prompt, resume_at, run_hook_resumable,
    run_resume, why_answer_refused,
};
use crate::script::EngineSink;
use crate::state::{CardInstance, EngineError, GameState, PromptOption, Resume, WorkItem, find_instance};
use crate::stays::{exit_mark, left_field_after};
use crate::wire::{CardType, PlayerId, PromptKind, Row, Selection, Zone};
use crate::work::{RUN_MARKS_KEY, begin_work_cascade, drain_work};

/// `resume.hook` of a prompt this sequence opened for itself: no card script holds the name.
pub const TRIGGER_CRY_HOOK: &str = "@triggerCry";

/// Where the run record sits inside the prompt's `resume.data`.
const RUN_KEY: &str = "cry";

/// Where the Unit is when its Cry is triggered. (TS `"field" | "graveyard"`.)
/// ME-ALTPLAY, R1044: a set Spell resolving in the backrow runs from `Backrow`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum CryPlace {
    Field,
    Graveyard,
    Backrow,
}

/// Which kind of question the open prompt is (TS `"mode" | "target" | null`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
enum CryAwaiting {
    Mode,
    Target,
}

/// A triggered Cry part-way through its questions. All JSON (§9.3); the field order is TS's literal's.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct CryRun {
    instance_id: String,
    controller: PlayerId,
    place: CryPlace,
    /// R174: the field's departures when the trigger began, which a Unit on the field must outlast.
    since: u32,
    targets: Vec<Selection>,
    modes: Vec<String>,
    /// The next target declaration to ask, and the next mode declaration.
    decl_at: usize,
    mode_at: usize,
    /// Which kind of question the open prompt is, so its answer is filed where it belongs. `null`, not
    /// absent, while none is open, as TS writes it.
    awaiting: Option<CryAwaiting>,
    /// ME-ALTPLAY, R1045: Echo repeats a set Spell still owes, with fresh prompts each time.
    #[serde(default, skip_serializing_if = "is_zero")]
    repeats: u32,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// The name a card's definition prints.
fn name_of(state: &GameState, def_id: &str) -> String {
    crate::catalog::def_of(Some(state), def_id).name.clone()
}

/// Where a card's Cry could be triggered from, or `None`: a Unit (its running face's type, B2.7) with a
/// Cry on its running face, on top of its unit pile or in a graveyard. A Vanilla card has no text
/// (§6.3), so no Cry. A pure read: the reader a card's target check uses (Classic #54's "a Unit that
/// has a Cry").
pub fn cry_place_of(state: &GameState, card: &CardInstance) -> Option<CryPlace> {
    // ME-ALTPLAY, R1044: a set Spell whose text is resolving runs from the backrow first.
    if matches!(
        card.zone,
        Zone::Field {
            row: Row::Backrow,
            ..
        }
    ) && card.set_as.as_ref().and_then(|set| set.revealing) == Some(true)
    {
        crate::scripts::script_of(state, card).cry.as_ref()?;
        return Some(CryPlace::Backrow);
    }
    if crate::faces::card_type_of(state, card) != CardType::Unit {
        return None;
    }
    crate::scripts::script_of(state, card).cry.as_ref()?;
    match card.zone {
        Zone::Graveyard { .. } => Some(CryPlace::Graveyard),
        Zone::Field { row: Row::Units, .. } if !crate::zones::is_buried(state, card) => Some(CryPlace::Field),
        _ => None,
    }
}

/// B5 E13: trigger `card`'s Cry for `controller`. Asks the Cry's declared choices first, as prompts
/// (the sequence then continues in the answer), and runs the Cry once they are all made. Nothing
/// happens for a card whose Cry cannot be triggered (`cry_place_of`).
pub fn trigger_cry_of(sink: &mut EngineSink, card: &CardInstance, controller: PlayerId) {
    let Some(place) = cry_place_of(sink.state, card) else {
        return;
    };
    let run = CryRun {
        instance_id: card.id.clone(),
        controller,
        place,
        since: exit_mark(sink.state),
        targets: Vec::new(),
        modes: Vec::new(),
        decl_at: 0,
        mode_at: 0,
        awaiting: None,
        repeats: 0,
    };
    continue_run(sink, run);
}

/// ME-ALTPLAY, R1044, R1045: resolve a set Spell from the backrow with `repeats` Echo repeats.
/// R703 is read as it reveals: a required declaration the board cannot satisfy fizzles the reveal.
pub fn reveal_set_spell(sink: &mut EngineSink, card: &CardInstance, controller: PlayerId, repeats: u32) {
    for decl in crate::play_choices::declared_targets(sink.state, card) {
        let offered = crate::play_choices::legal_selections_for(sink.state, controller, card, &decl);
        if crate::play_choices::unmet_requirement(&decl, offered.len()) {
            return;
        }
    }
    let run = CryRun {
        instance_id: card.id.clone(),
        controller,
        place: CryPlace::Backrow,
        since: exit_mark(sink.state),
        targets: Vec::new(),
        modes: Vec::new(),
        decl_at: 0,
        mode_at: 0,
        awaiting: None,
        repeats,
    };
    continue_run(sink, run);
}

/// ME-ALTPLAY, R1045: the work hook a paused reveal's repeats wait behind (R113).
pub const SET_REPEAT_HOOK: &str = "@setRepeat";

/// ME-ALTPLAY: run an owed Echo repeat of a set Spell.
pub fn run_owed_repeat(sink: &mut EngineSink, item: &WorkItem) {
    if let Some(run) = run_of(&item.resume.data) {
        continue_run(sink, run);
    }
}

/// The Unit still where the trigger found it, or `None`: on the same stay (R174), or in a graveyard.
fn standing(state: &GameState, run: &CryRun) -> Option<CardInstance> {
    let card = find_instance(state, &run.instance_id)?;
    if cry_place_of(state, card) != Some(run.place) {
        return None;
    }
    if run.place == CryPlace::Field && left_field_after(state, run.since, &card.id) {
        return None;
    }
    Some(card.clone())
}

fn resume_for(run: &CryRun) -> Resume {
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert(
        RUN_KEY.to_string(),
        serde_json::to_value(run).unwrap_or(Value::Null),
    );
    Resume {
        def_id: String::new(),
        hook: TRIGGER_CRY_HOOK.to_string(),
        step: "choose".to_string(),
        radiant: false,
        instance_id: None,
        data,
    }
}

fn label_of(state: &GameState, selection: &Selection, chooser: PlayerId) -> String {
    match selection {
        Selection::Instance { instance_id } => match find_instance(state, instance_id) {
            None => instance_id.clone(),
            Some(card) => name_of(state, &card.def_id),
        },
        Selection::Hero { player } => hero_option_label(*player, chooser),
        Selection::Zone { player, row, lane } => cell_option_label(*player, *row, *lane, chooser),
        Selection::Mode { option } => option.clone(),
        Selection::Craft { recipe } => recipe.name(),
        Selection::None => "nothing".to_string(),
    }
}

fn key_of(selection: &Selection) -> String {
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

/// The mode declarations still to ask; false while one is waiting (R81).
fn ask_modes(sink: &mut EngineSink, run: &mut CryRun, card: &CardInstance) -> bool {
    let modes = declared_modes(sink.state, card);
    let name = name_of(sink.state, &card.def_id);
    let title = if run.place == CryPlace::Backrow {
        format!("Reveal: {name}")
    } else {
        format!("Cry: {name}")
    };
    for (at, decl) in modes.iter().enumerate().skip(run.mode_at) {
        run.mode_at = at + 1;
        if decl.options.is_empty() {
            continue;
        }
        run.awaiting = Some(CryAwaiting::Mode);
        let options: Vec<PromptOption> = decl
            .options
            .iter()
            .map(|option| PromptOption {
                key: format!("mode:{option}"),
                label: option.clone(),
                selection: Selection::Mode {
                    option: option.clone(),
                },
                cost: None,
                radiant: None,
            })
            .collect();
        let opened = open_prompt(
            sink,
            OpenPromptArgs {
                player: run.controller,
                kind: decl.kind,
                aim: None,
                prompt: title.clone(),
                options,
                min: None,
                max: None,
                budget: None,
                owner: None,
                resume: resume_for(run),
            },
        );
        if opened.is_some() {
            return false;
        }
        run.awaiting = None;
    }
    true
}

/// The target declarations still to ask; false while one is waiting (R81, R90).
fn ask_targets(sink: &mut EngineSink, run: &mut CryRun, card: &CardInstance) -> bool {
    let decls = active_target_decls(&declared_targets(sink.state, card), &run.modes);
    let name = name_of(sink.state, &card.def_id);
    let title = if run.place == CryPlace::Backrow {
        format!("Reveal: {name}")
    } else {
        format!("Cry: {name}")
    };
    for (at, decl) in decls.iter().enumerate().skip(run.decl_at) {
        run.decl_at = at + 1;
        let options = legal_selections_for(sink.state, run.controller, card, decl);
        if decl.kind == PromptKind::Tribute {
            // A Tribute is a play's price, and nothing was played: its slots stay, empty (R90, R123).
            let slots = (decl.min.max(0) as usize).min(options.len());
            for _ in 0..slots {
                run.targets.push(Selection::None);
            }
            continue;
        }
        if options.is_empty() {
            continue;
        }
        run.awaiting = Some(CryAwaiting::Target);
        let prompt_options: Vec<PromptOption> = options
            .iter()
            .map(|selection| PromptOption {
                key: key_of(selection),
                label: label_of(sink.state, selection, run.controller),
                selection: selection.clone(),
                cost: None,
                radiant: None,
            })
            .collect();
        let opened = open_prompt(
            sink,
            OpenPromptArgs {
                player: run.controller,
                kind: decl.kind,
                aim: None,
                prompt: title.clone(),
                options: prompt_options,
                min: Some(decl.min),
                max: Some(decl.max),
                budget: None,
                owner: None,
                resume: resume_for(run),
            },
        );
        if opened.is_some() {
            return false;
        }
        run.awaiting = None;
    }
    true
}

/// `ask_modes` or `ask_targets`: one kind of declaration still to ask; false while one is waiting.
type Asker = fn(&mut EngineSink, &mut CryRun, &CardInstance) -> bool;

/// Ask what is still to ask, then run the Cry; stops at a prompt, whose answer comes back here.
fn continue_run(sink: &mut EngineSink, mut run: CryRun) {
    let Some(card) = standing(sink.state, &run) else {
        return;
    };
    // The order is the point: the second asks only once the first has nothing left to ask.
    let (first, second): (Asker, Asker) = if targets_follow_modes(&declared_targets(sink.state, &card)) {
        (ask_modes, ask_targets)
    } else {
        (ask_targets, ask_modes)
    };
    let asked = first(sink, &mut run, &card) && second(sink, &mut run, &card);
    if !asked {
        return;
    }
    run_cry(sink, &run, &card);
}

/// The Cry itself, with the choices made: the card's own resumable hook (R113).
fn run_cry(sink: &mut EngineSink, run: &CryRun, card: &CardInstance) {
    // R174: the choices were made against the board as it stands now, and the Cry is aimed at that.
    let exits_from = exit_mark(sink.state);
    // R90, R102: a fused card's Cry hands each ingredient its own slice of the choices.
    let mut data: IndexMap<String, Value> = IndexMap::new();
    if sink.state.transient_defs.contains_key(&card.def_id) {
        let slices = declaration_slices(sink.state, run.controller, card, &run.targets, &run.modes);
        data.insert(DECLARATION_SLICES_KEY.to_string(), json!(slices));
    }
    if run.place == CryPlace::Field || run.place == CryPlace::Backrow {
        let ran = run_hook_resumable(
            sink,
            card,
            "cry",
            HookResumableOptions {
                controller: Some(run.controller),
                targets: Some(run.targets.clone()),
                modes: Some(run.modes.clone()),
                data: Some(data),
                exits_from: Some(exits_from),
            },
        );
        // ME-ALTPLAY, R1045: Echo repeats with fresh prompts, after the hook's parked tail (R113).
        if run.place == CryPlace::Backrow && run.repeats > 0 {
            let next = CryRun {
                instance_id: run.instance_id.clone(),
                controller: run.controller,
                place: CryPlace::Backrow,
                since: exit_mark(sink.state),
                targets: Vec::new(),
                modes: Vec::new(),
                decl_at: 0,
                mode_at: 0,
                awaiting: None,
                repeats: run.repeats - 1,
            };
            if !ran {
                let mut repeat_data: IndexMap<String, Value> = IndexMap::new();
                repeat_data.insert(
                    RUN_KEY.to_string(),
                    serde_json::to_value(&next).unwrap_or(Value::Null),
                );
                let resume = Resume {
                    def_id: String::new(),
                    hook: SET_REPEAT_HOOK.to_string(),
                    step: "repeat".to_string(),
                    radiant: false,
                    instance_id: None,
                    data: repeat_data,
                };
                crate::work::owe(sink, resume);
            } else {
                continue_run(sink, next);
            }
        }
        return;
    }
    // Out of a graveyard "this" finds nothing: the Cry runs as its definition's, with no card (R127).
    let mut resume = resume_at(ResumeAtArgs {
        def_id: card.def_id.clone(),
        step: String::new(),
        hook: Some("cry".to_string()),
        radiant: Some(card.radiant),
        instance_id: None,
        data: Some(data),
    });
    resume
        .data
        .insert(RUN_MARKS_KEY.to_string(), json!({ "exitsFrom": exits_from }));
    run_resume(
        sink,
        &resume,
        ResumeOptions {
            controller: Some(run.controller),
            targets: Some(run.targets.clone()),
            modes: Some(run.modes.clone()),
            ..Default::default()
        },
    );
}

/// The run record a prompt's data carries, re-read defensively: it came back through JSON (§10.1,
/// SURFACE §4.4.10), so nothing about the payload is assumed.
fn run_of(data: &IndexMap<String, Value>) -> Option<CryRun> {
    let raw = data.get(RUN_KEY)?.as_object()?;
    let instance_id = raw.get("instanceId")?.as_str()?.to_string();
    let controller = match raw.get("controller").and_then(Value::as_str) {
        Some("p1") => PlayerId::P1,
        Some("p2") => PlayerId::P2,
        _ => return None,
    };
    let place = match raw.get("place").and_then(Value::as_str) {
        Some("field") => CryPlace::Field,
        Some("graveyard") => CryPlace::Graveyard,
        Some("backrow") => CryPlace::Backrow,
        _ => return None,
    };
    let number = |key: &str| raw.get(key).and_then(Value::as_f64);
    Some(CryRun {
        instance_id,
        controller,
        place,
        since: number("since").map_or(0, |n| n.max(0.0) as u32),
        targets: raw
            .get("targets")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| serde_json::from_value::<Selection>(item.clone()).ok())
                    .collect()
            })
            .unwrap_or_default(),
        modes: raw
            .get("modes")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        decl_at: number("declAt").map_or(0, |n| n.max(0.0) as usize),
        mode_at: number("modeAt").map_or(0, |n| n.max(0.0) as usize),
        awaiting: match raw.get("awaiting").and_then(Value::as_str) {
            Some("mode") => Some(CryAwaiting::Mode),
            Some("target") => Some(CryAwaiting::Target),
            _ => None,
        },
        repeats: number("repeats").map_or(0, |n| n.max(0.0) as u32),
    })
}

/// R122: the answer to one of this sequence's questions. Validated like any answer, filed into the
/// run — a mode prompt's pick into its modes, a target prompt's picks, in offered order (R221), into
/// its targets — and the sequence goes on: the next question, or the Cry. Then the rest of what the
/// question interrupted drains in R113's order, as `prompts::answer_prompt` does for a card's own step.
///
/// `prompts.rs` calls this for a prompt whose `resume.hook` is `TRIGGER_CRY_HOOK` (TS registered it
/// with `registerPromptAnswerer`; SURFACE §6.6 matches the key instead).
pub fn answer_cry_prompt(sink: &mut EngineSink, answer: &AnswerInput) -> Result<(), EngineError> {
    let Some(pending) = sink.state.pending.clone() else {
        return Err(EngineError::new("no prompt is open"));
    };
    why_answer_refused(&pending, answer)?;
    let run = run_of(&pending.resume.data);
    close_prompt(sink);
    begin_work_cascade(sink);
    if let Some(mut run) = run {
        let picks = in_offered_order(&pending, &answer.selection);
        for pick in picks {
            match pick {
                Selection::Mode { option } if run.awaiting == Some(CryAwaiting::Mode) => {
                    run.modes.push(option)
                }
                other => run.targets.push(other),
            }
        }
        run.awaiting = None;
        continue_run(sink, run);
    }
    drain_work(sink);
    Ok(())
}
