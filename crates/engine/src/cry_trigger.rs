//! Trigger a Cry (docs/classic-sets.md B5 E13, R467): run a Unit's Cry again, on the field or out of a
//! graveyard, for the player whose card triggered it (Classic #54 Rewind), who makes its choices (R70).
//! A dormant card under a Stack is not on the field (R13). A triggered Cry is no play: its declared
//! choices (R81) are asked as prompts, one the board cannot satisfy skipped (§8); no `cardPlayed`, no
//! counters (R55, E4), no Echo, no Combo. Out of a graveyard "this" finds nothing, a pool still
//! excludes the definition (§5.1), and X reads 0.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::multipliers::{Multiplied, extra_runs};
use crate::play_choices::{
    DECLARATION_SLICES_KEY, active_target_decls, declaration_slices, declared_modes, declared_targets,
    legal_selections_for, targets_follow_modes,
};
use crate::prompts::{
    AnswerInput, HookResumableOptions, OpenPromptArgs, ResumeAtArgs, ResumeOptions, cell_option_label,
    close_prompt, hero_option_label, in_offered_order, open_prompt, resume_at, run_hook_resumable,
    run_resume, why_answer_refused,
};
use crate::random_cast::{
    RANDOM_CHOICES_KEY, prefer_enemies, prefer_friends, random_cast_of, random_picks, with_cast_mode,
};
use crate::script::EngineSink;
use crate::state::{
    CardInstance, CastMode, EngineError, GameState, PromptOption, Resume, WorkItem, find_instance,
};
use crate::state_check::state_check;
use crate::stays::{exit_mark, left_field_after};
use crate::wire::{CardType, PlayerId, PromptKind, Row, Selection, TargetAim, Zone};
use crate::work::{RUN_MARKS_KEY, begin_work_cascade, drain_work, paused, push_work};

/// `resume.hook` of a prompt this sequence opened for itself: no card script holds the name.
pub const TRIGGER_CRY_HOOK: &str = "@triggerCry";

/// Where the run record sits inside the prompt's `resume.data`.
const RUN_KEY: &str = "cry";

/// Where the Unit is when its Cry is triggered.
/// ME-ALTPLAY, R1044: a set Spell resolving in the backrow runs from `Backrow`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum CryPlace {
    Field,
    Graveyard,
    Backrow,
}

/// Which kind of question the open prompt is.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
enum CryAwaiting {
    Mode,
    Target,
}

/// A triggered Cry part-way through its questions. All JSON (§9.3); its field order is the format's.
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
    /// absent, while none is open.
    awaiting: Option<CryAwaiting>,
    /// ME-ALTPLAY, R1045: Echo repeats a set Spell still owes, with fresh prompts each time.
    #[serde(default, skip_serializing_if = "is_zero")]
    repeats: u32,
    /// R1855001: the controller's every choice is random, the Cry's own text's included, across its
    /// pauses too (`trigger_random_cry_of`). Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    random: Option<bool>,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

fn name_of(state: &GameState, def_id: &str) -> String {
    crate::catalog::def_of(Some(state), def_id).name.clone()
}

/// Where a card's Cry could be triggered from, or `None`: a Unit (its running face's type, B2.7) with a
/// Cry on its running face, on top of its unit pile, standing in a backrow zone (R1081000) or in a
/// graveyard, or a face-up Field Spell with one in its backrow zone (R1873003). A Vanilla card has no
/// text (§6.3), so no Cry.
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
    let card_type = crate::faces::card_type_of(state, card);
    // R1873003: a face-up Field Spell on the field runs its Cry as itself (R411). A face-down one reads
    // as the Trap it was set as (`card_type_of`), and out of a graveyard only a Unit's Cry is triggered.
    if card_type == CardType::FieldSpell && matches!(card.zone, Zone::Field { .. }) {
        crate::scripts::script_of(state, card).cry.as_ref()?;
        return Some(CryPlace::Field);
    }
    if card_type != CardType::Unit {
        return None;
    }
    crate::scripts::script_of(state, card).cry.as_ref()?;
    match card.zone {
        Zone::Graveyard { .. } => Some(CryPlace::Graveyard),
        // R1081000: a Unit standing in the backrow is on the field as one in a unit zone is.
        Zone::Field { .. }
            if crate::zones::in_units_row_or_standing(state, card)
                && !crate::zones::is_buried(state, card)
                // R1839000: no effect triggers a Dormant card's Cry.
                && !crate::slumber::is_slumbering(card) =>
        {
            Some(CryPlace::Field)
        }
        _ => None,
    }
}

/// B5 E13: trigger `card`'s Cry for `controller`: its declared choices as prompts, then the Cry.
/// Nothing happens for a card whose Cry cannot be triggered (`cry_place_of`).
pub fn trigger_cry_of(sink: &mut EngineSink, card: &CardInstance, controller: PlayerId) {
    trigger(sink, card, controller, None);
}

/// R1855001: trigger `card`'s Cry for `controller` with every choice of the controller's drawn at
/// random, as R452's random cast draws its caster's, after a pause as before one
/// (`random_cast::RANDOM_CHOICES_KEY`). The other player's prompts are asked as usual.
pub fn trigger_random_cry_of(sink: &mut EngineSink, card: &CardInstance, controller: PlayerId) {
    trigger(sink, card, controller, Some(true));
}

fn trigger(sink: &mut EngineSink, card: &CardInstance, controller: PlayerId, random: Option<bool>) {
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
        random,
    };
    continue_run(sink, run);
}

/// ME-ALTPLAY, R1044, R1045: resolve a set Spell from the backrow with `repeats` Echo repeats.
/// R703 is read as it reveals: a required declaration the board cannot satisfy fizzles the reveal, as
/// a play gate that does not hold does (R1851001).
pub fn reveal_set_spell(sink: &mut EngineSink, card: &CardInstance, controller: PlayerId, repeats: u32) {
    if !crate::play_choices::play_allowed(sink.state, controller, card) {
        return;
    }
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
        random: None,
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
        // R452: a random cast's caster is never asked; the mode is drawn at random.
        if random_cast_of(sink.state, run.controller).is_some() {
            if let Some(option) = sink.rng.pick(&decl.options).cloned() {
                run.modes.push(option);
            }
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
        // R1200: while a Mayor acts a `target` Cry pick is drawn at random, and nobody is asked.
        if decl.kind == PromptKind::Target && crate::random_targets::targets_random(sink.state) {
            run.targets.extend(crate::random_targets::draw_picks(
                sink.state,
                &mut *sink.rng,
                run.controller,
                &options,
                decl.min,
                decl.max,
                crate::targeting::target_aim(decl),
            ));
            continue;
        }
        // R452: a random cast's caster is never asked; its picks are drawn at random, each aimed by its
        // declaration when the cast targets enemies (R656), as `play_steps::cast_target_options` aims them.
        if let Some(cast) = random_cast_of(sink.state, run.controller) {
            let pool = if cast.friends_only == Some(true) && decl.kind == PromptKind::Target {
                // R1850004: a cast for its caster's own side picks there alone.
                crate::random_cast::friends_only(sink.state, run.controller, &options, Selection::clone)
            } else if cast.target_enemies {
                let required = decl.min.min(options.len() as i32);
                if crate::targeting::target_aim(decl) == TargetAim::Help {
                    prefer_friends(sink.state, run.controller, &options, Selection::clone, required)
                } else {
                    prefer_enemies(sink.state, run.controller, &options, Selection::clone, required)
                }
            } else {
                options
            };
            run.targets
                .extend(random_picks(sink.rng, &pool, decl.min, decl.max));
            continue;
        }
        run.awaiting = Some(CryAwaiting::Target);
        let prompt_options: Vec<PromptOption> = options
            .iter()
            .map(|selection| PromptOption {
                key: crate::prompts::option_key(selection),
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
fn continue_run(sink: &mut EngineSink, run: CryRun) {
    // R1855001: a random Cry's questions are drawn, as a random cast's are (R452).
    let mode = (run.random == Some(true)).then(|| CastMode {
        instance_id: run.instance_id.clone(),
        player: run.controller,
        random: true,
        target_enemies: false,
        casts: 0,
        friends_only: None,
    });
    with_cast_mode(sink, mode, |sink| ask_and_run(sink, run));
}

/// `continue_run`, under whatever mode is in force.
fn ask_and_run(sink: &mut EngineSink, mut run: CryRun) {
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

/// The Cry itself, with the choices made: the card's own resumable hook (R113), then the extra runs
/// its controller's Cry multiplier owes (R823), read as the Cry first runs. A set Spell revealing from
/// the backrow (R1044) is a Spell's resolution, not a Cry, so it runs once (R822); its Echo repeats are
/// its own (R1045).
fn run_cry(sink: &mut EngineSink, run: &CryRun, card: &CardInstance) {
    // R174: the choices were made against the board as it stands now, and the Cry is aimed at that.
    let exits_from = exit_mark(sink.state);
    let extra = if run.place == CryPlace::Backrow {
        0
    } else {
        extra_runs(sink.state, run.controller, Multiplied::CryAndDeath).max(0) as u32
    };
    run_cry_once(sink, run, card, exits_from);
    run_extra_cries(sink, run, extra, exits_from);
}

/// `resume.hook` of the work item that owes a triggered Cry's extra runs (R823): no card script holds
/// the name.
pub const EXTRA_CRY_HOOK: &str = "@extraCry";

/// R823 (Meditative #10 Double Counting): a triggered Cry runs `left` more times, each a whole effect
/// list the state check closes first (R59), reusing its prompts' answers (R467) and aimed at their
/// stays (R174), until the Unit is no longer where the trigger found it. A pause owes the rest as one
/// work item as it pauses (R117), behind the tail of the run that paused (R113).
fn run_extra_cries(sink: &mut EngineSink, run: &CryRun, mut left: u32, exits_from: u32) {
    while left > 0 {
        if sink.state.result.is_some() {
            return;
        }
        if paused(sink) {
            owe_extra_cries(sink, run, left, exits_from);
            return;
        }
        state_check(sink);
        if sink.state.result.is_some() {
            return;
        }
        if paused(sink) {
            owe_extra_cries(sink, run, left, exits_from);
            return;
        }
        let Some(card) = standing(sink.state, run) else {
            return;
        };
        left -= 1;
        run_cry_once(sink, run, &card, exits_from);
    }
}

fn owe_extra_cries(sink: &mut EngineSink, run: &CryRun, left: u32, exits_from: u32) {
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert(
        RUN_KEY.to_string(),
        serde_json::to_value(run).unwrap_or(Value::Null),
    );
    data.insert(EXTRA_LEFT_KEY.to_string(), json!(left));
    data.insert(EXTRA_EXITS_KEY.to_string(), json!(exits_from));
    push_work(
        sink,
        Resume {
            def_id: String::new(),
            hook: EXTRA_CRY_HOOK.to_string(),
            step: "run".to_string(),
            radiant: false,
            instance_id: None,
            data,
        },
        Some(run.controller),
    );
}

/// Where an owed run of extra Cries keeps how many are left, and the stays its choices were made on.
const EXTRA_LEFT_KEY: &str = "left";
const EXTRA_EXITS_KEY: &str = "exitsFrom";

/// `work.rs`'s handler for `EXTRA_CRY_HOOK`: the extra runs a pause interrupted, continued (R113).
pub fn run_owed_extra_cries(sink: &mut EngineSink, item: &WorkItem) {
    let Some(run) = run_of(&item.resume.data) else {
        return;
    };
    let number = |key: &str| {
        item.resume
            .data
            .get(key)
            .and_then(Value::as_u64)
            .and_then(|n| u32::try_from(n).ok())
    };
    let left = number(EXTRA_LEFT_KEY).unwrap_or(0);
    let exits_from = number(EXTRA_EXITS_KEY).unwrap_or_else(|| exit_mark(sink.state));
    run_extra_cries(sink, &run, left, exits_from);
}

/// One run of the Cry, with the choices made, aimed at the stays they were made on (`exits_from`).
fn run_cry_once(sink: &mut EngineSink, run: &CryRun, card: &CardInstance, exits_from: u32) {
    // R90, R102: a fused card's Cry hands each ingredient its own slice of the choices.
    let mut data: IndexMap<String, Value> = IndexMap::new();
    if sink.state.transient_defs.contains_key(&card.def_id) {
        let slices = declaration_slices(sink.state, run.controller, card, &run.targets, &run.modes);
        data.insert(DECLARATION_SLICES_KEY.to_string(), json!(slices));
    }
    // R1855001: and a random Cry's text is random in every continuation of it.
    if run.random == Some(true) {
        data.insert(RANDOM_CHOICES_KEY.to_string(), json!(run.controller));
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
                random: run.random,
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
        random: (raw.get("random").and_then(Value::as_bool) == Some(true)).then_some(true),
    })
}

/// R122: the answer to a prompt whose `resume.hook` is `TRIGGER_CRY_HOOK` (`prompts.rs`, SURFACE §6.6),
/// validated like any answer and filed into the run (picks in offered order, R221); then the next
/// question or the Cry, and the rest drains in R113's order, as `prompts::answer_prompt` does.
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
