//! SPEC §10.5 "Playing a card": the eight steps as a table of named steps (`PLAY_STEPS`) that `drive` runs
//! over one JSON record, a `PlayRun` (BUILD M3-T2/T3, R81, R90). A step that leaves a prompt open owes the
//! rest to `state.work` as a `Resume` naming the step plus that record (§9.3), so nothing is held across a
//! prompt in a local or a closure (the bug `work.rs` exists to prevent). Echo (R30, §6.1) is step 6
//! re-running step 5 one repeat at a time off `state.echoQueue`, with fresh prompts. Steps 3, 5 and 6 keep
//! their own cursor in the record, so a pause resumes where it stopped.
//!
//! The announce (B5 E1, R448), between 3 and 4, is a window for Counters (§10.3); step 1 is the targeting
//! point (B5 E5, E9, R450), step 3 may replace the card (R449), step 4 counts it (R451).
//!
//! Not here: legal choices (`play_choices.rs`, R90), costs (`mana.rs`, R65), card text (the script,
//! CLAUDE.md rule 5), R43's powers (`subsystems/hero_power.rs`). Callers name the entry points
//! (SURFACE §6.6): `work.rs`'s `"play"` arm `run_owed_play`, `resolve::cast_card`
//! `cast_through_pipeline`, `prompts.rs`'s `"play"` answerer `answer_play_prompt`.

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};

use crate::animated::animate_on_entry;
use crate::announce::{begin_announce, end_announce, is_announce_live};
use crate::catalog::def_of;
use crate::config::{FENG_SHUI_BRITTLE, MIN_CHOSEN_X, QUICKSTRIKER_COMBO_MULTIPLE};
use crate::cost_rules::why_play_banned;
use crate::damage::{DamageArgs, DamageTarget, deal_damage};
use crate::draw::{add_to_hand, draw};
use crate::draw_complete::{CAST_ON_DRAW_KEY, release_draw};
use crate::echo::{
    EXILE_ON_LANDING, ResolvedCard, add_echo_repeats, drop_echo_repeats, echo_repeats_owed, exile_on_landing,
    land_after_resolution, queue_echo_repeats, take_echo_repeat,
};
use crate::enchantments::{EnchantmentKind, has_enchantment};
use crate::faces::card_type_of;
use crate::graveyard_play::{in_own_graveyard, mana_due, spend_plague_tokens, why_graveyard_play_refused};
use crate::mana::{cost_rules_spent_by, is_x_cost, mana_event, modifier_is_live, play_cost, push_mana_spent};
use crate::modifiers::remove_modifier;
use crate::multipliers::{Multiplied, extra_runs};
use crate::play_choices::{
    DECLARATION_SLICES_KEY, active_target_decls, chooses_x, declaration_slices, declared_modes,
    declared_targets, default_zone_for, equip_host_at, in_declared_order, interceptor_fits_decl,
    legal_selections_for, mode_option_allowed, play_made_radiant, play_uses, plays_on_stack, resolving_face,
    targeting_decls_of, targeting_discards_required, targets_follow_modes, why_choices_refused,
};
use crate::play_counts::record_play;
use crate::prompts::{
    AnswerInput, HookResumableOptions, OpenPromptArgs, cell_option_label, close_prompt, hero_option_label,
    in_offered_order, open_prompt, run_hook_resumable, why_answer_refused,
};
use crate::query::played_earlier;
use crate::random_cast::{
    count_chain_cast, prefer_enemies, prefer_friends, random_cast_of, random_picks, with_cast_mode,
};
use crate::resolve::{
    CastOptions, HookName, MANA_BEFORE_PLAY_KEY, flag_return_to_hand_at_end_of_turn, note_return_price,
};
use crate::script::{EngineSink, FlagOrCount};
use crate::state::{
    AnnounceRecord, CardInstance, CastMode, EngineError, GameState, ModifierExpiry, ModifierKind, PlayRecord,
    PromptOption, QueuedTrigger, Resume, WorkItem, find_instance, find_instance_mut, new_instance,
};
use crate::state_check::{sacrifice_together, state_check};
use crate::stays::{exit_mark, left_field_after};
use crate::subsystems::copied_text::{copied_text_of, copies_text, fix_copied_text, text_face_of};
use crate::subsystems::feng_shui::FengShuiVerdict;
use crate::subsystems::fuse::{FuseArgs, fuse};
use crate::subsystems::glitch::count_system_play;
use crate::targeting::target_aim;
use crate::targeting_point::{
    InterceptArgs, intercept_targeting, pay_targeting_discards, target_answer, why_target_answer_refused,
};
use crate::triggers::{
    OWED_TO_TRAPS, SETTLE_PASS_CAP, SettleOptions, TriggerHolder, dispatch_pending, event_of_queued,
    run_queued_trigger, settle, trigger_holder_for, trigger_holders_with_hook,
};
use crate::wire::{
    CardCost, CardType, CounterKind, FengShuiOutcome, GameEvent, ManaSpentFor, PLAYER_IDS, PlagueSpend,
    PlayedFrom, PlayerId, PromptKind, RevealAt, Row, Selection, TargetAim, TargetDecl, Zone, opponent_of,
};
use crate::work::{begin_work_cascade, drain_work, drop_work, paused, paused_of, push_work};
use crate::zones::{
    PlaceOnFieldOptions, ZoneSlot, card_at, cease_to_exist, first_free_zone, fresh_face_down_id, is_open,
    lands_face_down, live_or, place_on_field, place_standing_on_field, release_zone, remove_from_any_zone,
    reserve_zone,
};

/// The play action; re-exported from `play_choices.rs` so the crate root's globs see one type.
pub use crate::play_choices::PlayAction;

// Private copies of small helpers other modules own (fullsend rule 5)

/// The card as it stands now, by id: a copy for calls that take the card beside a `&mut` state.
fn snapshot(state: &GameState, instance_id: &str) -> Option<CardInstance> {
    find_instance(state, instance_id).cloned()
}

fn set_x(state: &mut GameState, instance_id: &str, value: i32) {
    if let Some(card) = find_instance_mut(state, instance_id) {
        card.x = Some(value);
    }
}

/// Leading whitespace, an optional sign, then the longest run of digits; `None` when there is none.
fn parse_int(text: &str) -> Option<i32> {
    let trimmed = text.trim_start();
    let (negative, rest) = match trimmed.as_bytes().first() {
        Some(b'-') => (true, &trimmed[1..]),
        Some(b'+') => (false, &trimmed[1..]),
        _ => (false, trimmed),
    };
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let value: i64 = digits.parse().ok()?;
    let value = if negative { -value } else { value };
    i32::try_from(value).ok()
}

// The steps and the run record

/// The eight steps of §10.5, in the order that section lists them, and E1's announce between 3 and 4.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum PlayStepName {
    Validate,
    Pay,
    GiftedHook,
    // R70, R452: a cast's choices, once step 3 settled their face (R214), so the announce (E1) names them.
    // A play made its choices at step 1, and this step does nothing for it.
    CastChoices,
    Announce,
    Place,
    Resolve,
    Echo,
    Finish,
    Settle,
}

impl PlayStepName {
    /// The step's name in `Resume.step`.
    pub fn as_str(self) -> &'static str {
        match self {
            PlayStepName::Validate => "validate",
            PlayStepName::Pay => "pay",
            PlayStepName::GiftedHook => "giftedHook",
            PlayStepName::CastChoices => "castChoices",
            PlayStepName::Announce => "announce",
            PlayStepName::Place => "place",
            PlayStepName::Resolve => "resolve",
            PlayStepName::Echo => "echo",
            PlayStepName::Finish => "finish",
            PlayStepName::Settle => "settle",
        }
    }
}

/// The steps in `drive`'s order.
pub const PLAY_STEPS: &[PlayStepName] = &[
    PlayStepName::Validate,
    PlayStepName::Pay,
    PlayStepName::GiftedHook,
    PlayStepName::CastChoices,
    PlayStepName::Announce,
    PlayStepName::Place,
    PlayStepName::Resolve,
    PlayStepName::Echo,
    PlayStepName::Finish,
    PlayStepName::Settle,
];

fn step_index(step: PlayStepName) -> usize {
    PLAY_STEPS
        .iter()
        .position(|named| *named == step)
        .unwrap_or(PLAY_STEPS.len())
}

/// The named parts of step 5, so a pause inside one continues at the next, never at the top.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum ResolvePart {
    Quickstriker,
    ComboDraw,
    Script,
}

/// The named parts of step 5, in order.
pub const RESOLVE_PARTS: &[ResolvePart] = &[
    ResolvePart::Quickstriker,
    ResolvePart::ComboDraw,
    ResolvePart::Script,
];

/// `resume.hook` for an owed play pipeline: the sequence name `work.rs` documents (`"play"`).
pub const PLAY_WORK_KIND: &str = "play";

/// Where the run record sits inside `resume.data`, so the rest of `data` stays the card's own.
pub(crate) const RUN_KEY: &str = "__play";

/// `PlayRun.awaiting`: which bucket the answer to a prompt this pipeline opened fills.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Awaiting {
    EchoTarget,
    EchoMode,
    CastX,
}

/// `PlayRun.source`: E11, R454 — the card is played from its player's graveyard.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum PlaySource {
    Graveyard,
}

/// `PlayRun.repeat`: an Echo repeat's or a cast's choices being asked, and the answers so far.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RepeatRecord {
    pub targets: Vec<Selection>,
    pub modes: Vec<String>,
    pub decl_at: usize,
    pub mode_at: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part_at: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exits_from: Option<u32>,
}

/// Absent is `None`, `null` is `Some(None)` (serde would read both as `None`).
fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

/// A play part-way through its steps. All JSON, as it sits in an owed `WorkItem`'s `resume.data` or a
/// prompt's `resume`, so a paused play survives a JSON round trip and a replay resumes it (§9.3, §10.1).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayRun {
    pub instance_id: String,
    #[serde(default)]
    pub def_id: String,
    pub player: PlayerId,
    /// The mana actually paid, which `cardPlayed` reports and Gifted Program reads (§8 #64).
    #[serde(default)]
    pub cost_paid: i32,
    /// Where a permanent goes; null for a Spell, which resolves instead (§10.5 step 4).
    #[serde(default)]
    pub zone: Option<ZoneSlot>,
    #[serde(default)]
    pub targets: Vec<Selection>,
    #[serde(default)]
    pub modes: Vec<String>,
    #[serde(default)]
    pub tributes: Vec<String>,
    /// B5 E5, R450, R682: the targeting cost step 1 checked, owed whatever an interception did (Classic
    /// #33); step 2 pays it at random, never recomputed off the redirected picks.
    #[serde(default)]
    pub targeting_owed: i32,
    /// Index into `PLAY_STEPS` of the step to run next.
    pub at: usize,
    /// Step 3's cursor: how many `onPlayHook` holders have run.
    #[serde(default)]
    pub hook_at: usize,
    /// Step 3's holders, by id, in R68's order as the step began. The cursor indexes this list, never a
    /// fresh read of the board, which a hook's answer can reshape (R113).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hook_ids: Option<Vec<String>>,
    /// Step 5's cursor into `RESOLVE_PARTS`.
    #[serde(default)]
    pub resolve_at: usize,
    /// Step 6: whether this play has worked out how many repeats it owes yet.
    #[serde(default)]
    pub echo_queued: bool,
    /// Step 6: the repeat being re-resolved and its fresh answers; `partAt` its cursor into
    /// `RESOLVE_PARTS` (#78's Combo draw can pause it), `exitsFrom` the stays its picks were made on, so
    /// a target its Combo draw killed is gone, Reborn body or not (R174, R81, R83).
    #[serde(default)]
    pub repeat: Option<RepeatRecord>,
    /// Set while a prompt this pipeline opened is waiting; says which bucket the answer fills.
    #[serde(default)]
    pub awaiting: Option<Awaiting>,
    /// R70, R81: a cast's own choices are in (asked into `repeat` before the card is placed, R90).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cast_chosen: Option<bool>,
    /// R70: a cast, entered at step 3 for 0, placed by R64, leaving the resolution loop to its caster.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cast: Option<bool>,
    /// The face the card was played with, set at step 4 for `cardResolved` (R34, R57).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
    /// R214: whether Gifted Program makes this play Radiant, as step 1 read it; step 3 applies this, not
    /// the board a Death at step 2 can change (#22). A cast (no step 1, R70) reads the board.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gifted: Option<bool>,
    /// R983: Meditative #40 Feng Shui's judgements of this play, fixed at step 1 as `gifted` is and
    /// applied at step 3 (a cast or a replaced card reads the board there instead).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feng_shui: Option<Vec<FengShuiVerdict>>,
    /// Step 4 placed the card; it is re-entered at its resolution loop, which a trap's question can
    /// pause (§10.3, R17).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placed: Option<bool>,
    /// R90, R102: how many of `targets` each declaration took when step 1 read the play, which a fused
    /// card's Cry splits its ingredients' choices by (`play_choices::DECLARATION_SLICES_KEY`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_slices: Option<Vec<usize>>,
    /// R174: the field's departures when the choices were checked (step 1, or once a caster made them,
    /// R70), so a declared target taken off the field since is gone for step 5, even through Reborn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exits_from: Option<u32>,
    /// R174: the field's departures when step 3 read its holders (`hookIds`), whose stays it runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hooks_from: Option<u32>,
    /// R174: the field's departures once step 4 placed the card. The play follows that stay alone: a
    /// Reborn body in its zone (R83) is not the card played, has no Cry (R1, R118) and is not in play
    /// for step 7 (R61).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placed_from: Option<u32>,
    /// R119: every card acting on the field as the play (or cast, R70) began, with the departures then
    /// (`standingFrom`). One arriving after (a Death at step 2, R210; a Cry's recruit, #98, or summon,
    /// #95; Reborn) or dormant under a Stack pile then (§3.2, R153) answers none of the play's events,
    /// the granted Combo (#38) included (`arrivedDuring`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standing: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standing_from: Option<u32>,
    /// R119: the player's modifiers as the play began: a granted Combo modifier the play installed
    /// answers neither it nor its own Echo repeat (§10.5 step 6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mods_before: Option<Vec<String>>,
    /// R360: step 2's Tribute took an opponent's unit (#55), read as paid, before they died. Step 4
    /// reads it against the placed face, as only #55's base face summons itself for the opponent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enemy_tributed: Option<bool>,
    /// R226, §10.5 step 4, §10.1: the card left its hand (or, for a cast, its resolving zone, R70)
    /// before step 4 could move it, so it is not played; what steps 2 and 3 did stands.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lost: Option<bool>,
    // play pipeline B (E11, E12; R452–R454)
    /// E11, R454: played from its player's graveyard under a permission (`graveyard_play.rs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<PlaySource>,
    /// E11, R454: the Plague Counters paying part of the price (`costPaid` is the whole price).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plague: Option<PlagueSpend>,
    /// R783013: a play by the card's own Flashback, which step 4 spends (`take_from_play_source`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flashback: Option<bool>,
    /// E12, R452: a random cast — every choice its caster would make is made at random.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub random: Option<bool>,
    /// E12, E39, R452: its target picks narrow to enemies when one is legal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_enemies: Option<bool>,
    /// R1850004: a cast whose target picks are its caster's own side alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub friends_only: Option<bool>,
    /// E12, R453: a cast Field Spell, Trap or Field Trap that found no zone: counted as played (R70,
    /// R138), nothing resolves, and step 7 puts it in the graveyard.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fizzled: Option<bool>,
    /// The player's mana before step 2 pays (or as a cast began): the Cry's `ctx.manaBeforePlay` (#22).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mana_before: Option<i32>,
    /// R453: the X its caster chose for a cast X card (the `number` prompt's answer), until it is set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cast_x: Option<i32>,
    // play pipeline A (E1, E5, Devil's Pact)
    /// B5 E1, R448: the announce has gone out and the card waits in the resolving zone; the step is
    /// re-entered at its window, which a response's question can pause.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub announced: Option<bool>,
    /// R448: a Counter canceled the play in its announce window: nothing after it happens but step 8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub countered: Option<bool>,
    /// R449: step 3 replaced the card (#23); the new one chooses as a cast does (R70), placed by R64.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaced: Option<bool>,
    /// R58: the id a cast-on-draw card was drawn under; its held `drawn` is released once the cast has
    /// resolved (`draw_complete.rs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drawn_as: Option<String>,
    /// R1086: the host an Equip play fuses into at step 7 (ME-EQUIP), read at step 1 (D14).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equip_host: Option<String>,
    // B5 E14 (Classic #57 Echo; R399, R546)
    /// The Spell text a copier's play resolves, fixed at step 1 (null: nothing had been played) and
    /// written on the card at the announce (`fix_copied_text`). `None` absent, `Some(None)` null.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub copied: Option<Option<PlayRecord>>,
    // Meditative (ME-TRIG)
    /// R822, R823: the Cry's extra runs owed under a Cry multiplier (Meditative #10), read at step 5's
    /// first run. Absent with none owed (docs/meditative-set.md D14).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_cries: Option<u32>,
    /// MD-B22, R946: an aimed cast's first pick, taken when legal (Meditative #98's Book of Buff).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aim_at: Option<String>,
    // Meditative+ (issue #1180)
    /// R1180004 (Meditative+ #6 Bloodbloom): the health this play pays instead of mana, fixed at step 1
    /// and lost at step 2. Absent where no health price reaches (D9).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health_price: Option<i32>,
    /// R1180004: the Echo the health price gives the Spell it prices, queued with its own at step 4.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health_echo: Option<i32>,
    // Meditative+ (issue #1181)
    /// R1181002 (Meditative+ #22 Mountaintop Garden): a Garden reached this play at step 1, so step 3
    /// plays its Embiggened version. Absent otherwise, and for a cast (R545) or a replaced card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub garden: Option<bool>,
}

/// A run with every cursor at rest and every optional field absent.
fn blank_run(instance_id: String, def_id: String, player: PlayerId) -> PlayRun {
    PlayRun {
        instance_id,
        def_id,
        player,
        cost_paid: 0,
        zone: None,
        targets: Vec::new(),
        modes: Vec::new(),
        tributes: Vec::new(),
        targeting_owed: 0,
        at: 0,
        hook_at: 0,
        hook_ids: None,
        resolve_at: 0,
        echo_queued: false,
        repeat: None,
        awaiting: None,
        cast_chosen: None,
        cast: None,
        radiant: None,
        gifted: None,
        feng_shui: None,
        placed: None,
        target_slices: None,
        exits_from: None,
        hooks_from: None,
        placed_from: None,
        standing: None,
        standing_from: None,
        mods_before: None,
        enemy_tributed: None,
        lost: None,
        source: None,
        plague: None,
        flashback: None,
        random: None,
        target_enemies: None,
        friends_only: None,
        fizzled: None,
        mana_before: None,
        cast_x: None,
        announced: None,
        countered: None,
        replaced: None,
        drawn_as: None,
        copied: None,
        extra_cries: None,
        aim_at: None,
        equip_host: None,
        health_price: None,
        health_echo: None,
        garden: None,
    }
}

// The owed record

fn resume_for(run: &PlayRun, at: usize) -> Resume {
    let mut record = run.clone();
    record.at = at;
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert(
        RUN_KEY.to_string(),
        serde_json::to_value(&record).expect("a play run is plain JSON (§10.1)"),
    );
    Resume {
        def_id: run.def_id.clone(),
        hook: PLAY_WORK_KIND.to_string(),
        step: PLAY_STEPS
            .get(at)
            .map_or(PlayStepName::Settle.as_str(), |step| step.as_str())
            .to_string(),
        radiant: false,
        instance_id: Some(run.instance_id.clone()),
        data,
    }
}

/// True when this continuation is an owed play pipeline rather than a card's own step.
pub fn is_play_resume(resume: &Resume) -> bool {
    resume.hook == PLAY_WORK_KIND
}

/// The run record a continuation carries, or null when it is not one of ours.
pub fn run_of(resume: &Resume) -> Option<PlayRun> {
    let raw = resume.data.get(RUN_KEY)?;
    if !raw.is_object() {
        return None;
    }
    if !raw.get("instanceId").is_some_and(Value::is_string) || !raw.get("at").is_some_and(Value::is_number) {
        return None;
    }
    serde_json::from_value(raw.clone()).ok()
}

// Step 1 — validate (§10.5 step 1)

fn hand_card<'a>(state: &'a GameState, player: PlayerId, instance_id: &str) -> Option<&'a CardInstance> {
    state.players[player]
        .hand
        .iter()
        .find(|card| card.id == instance_id)
}

/// Where a play takes its card from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlayFrom {
    Hand,
    Graveyard,
}

/// E11, R454: the player's hand (a Resource among its cards, R2250000), or their own graveyard where a
/// permission may let them play it (`graveyard_play.rs`); `None` when the card is in neither.
fn play_source_of(
    state: &GameState,
    player: PlayerId,
    instance_id: &str,
) -> Option<(CardInstance, PlayFrom)> {
    if let Some(in_hand) = hand_card(state, player, instance_id) {
        return Some((in_hand.clone(), PlayFrom::Hand));
    }
    let in_graveyard = state.players[player]
        .graveyard
        .iter()
        .find(|card| card.id == instance_id)?;
    if in_own_graveyard(state, player, in_graveyard) {
        Some((in_graveyard.clone(), PlayFrom::Graveyard))
    } else {
        None
    }
}

pub fn is_permanent(state: &GameState, card: &CardInstance) -> bool {
    card_type_of(state, card) != CardType::Spell
}

/// R119: the board and the player's modifiers as a play or a cast begins, which it answers against.
struct PlayBegins {
    standing: Vec<String>,
    standing_from: u32,
    mods_before: Vec<String>,
}

/// §10.5 step 1. The choice checks are `play_choices.rs`'s (R90), asked once; left here are the cost
/// (`mana.rs`) and the default zone (R64's leftmost free one). X and embiggen are stamped on the
/// instance before the cost is read, where R65 looks for them; `reduce` throws its clone away on a
/// refusal, so a refused play leaves nothing behind (§9.3).
pub fn validate_play(
    sink: &mut EngineSink<'_>,
    player: PlayerId,
    action: &PlayAction,
) -> Result<PlayRun, EngineError> {
    let Some((card, from)) = play_source_of(sink.state, player, &action.instance_id) else {
        return Err(EngineError::new(format!(
            "no card {} in {}'s hand",
            action.instance_id, player
        )));
    };
    // R2533000: a seat that plays only the tutorial's cards plays, sets and casts no other card.
    if crate::state::barred_from_play(sink.state, player, &card) {
        return Err(EngineError::new(format!(
            "{player} plays only Training Grounds cards that are not Legendary or Mythic (R2533000), and {} is not one",
            card.def_id
        )));
    }
    // R454: Plague Counters pay only for a play from the graveyard a permission lets them pay for.
    if from != PlayFrom::Graveyard && action.plague.is_some() {
        return Err(EngineError::new(
            "only a play from your graveyard can spend Plague Counters",
        ));
    }

    // R1200: while a Mayor acts the action carries no targets, so draw each at random before anything
    // is checked or paid; the targeting point, costs and Joro's replacements meet the drawn target
    // (MD-E9). A face-down play (ME-ALTPLAY, R1044) declares nothing, so draws nothing.
    let mut owned = action.clone();
    if crate::random_targets::targets_random(sink.state) && owned.face_down.is_none() {
        let face = crate::play_choices::face_for_action(sink.state, player, &card, &owned);
        let given = owned.targets.clone().unwrap_or_default();
        let modes = owned.modes.clone().unwrap_or_default();
        let decls = crate::play_choices::declared_targets(sink.state, &face);
        let drawn = crate::play_choices::with_drawn_target_picks(
            sink.state,
            &mut *sink.rng,
            player,
            &face,
            &decls,
            &given,
            &modes,
        )?;
        owned.targets = Some(drawn);
    }
    let action: &PlayAction = &owned;
    why_choices_refused(sink.state, player, &card, action)?;

    let chooses = chooses_x(sink.state, &card);
    let embiggens = matches!(
        def_of(Some(&*sink.state), &card.def_id).cost,
        CardCost::Embiggen { .. }
    );
    // R1563004: a card of one price that took Mountaintop Garden's Radiant price carries the choice to
    // §10.5 step 3, which takes it off again (`garden_step`).
    let garden_radiant =
        action.embiggen == Some(true) && crate::mana::garden_offers_radiant(sink.state, &card);
    // R783004, R783005: stamp the face-down form before the price is read. Radiant Paranoia's Echo is
    // read as the Spell reveals (R1045), so none is stamped.
    let face_down_stamp: Option<(RevealAt, i32, Option<i32>)> =
        action.face_down.map(|reveal| (reveal, sink.state.turn, None));
    // R1848003: an X-cost card whose X is set is played for that X, which the play names (`cardPlayed`).
    let fixed_x = card.fixed_x.filter(|_| is_x_cost(sink.state, &card));
    if let Some(stamped) = find_instance_mut(sink.state, &card.id) {
        if chooses {
            stamped.x = Some(action.x.unwrap_or(0));
        } else if let Some(fixed) = fixed_x {
            stamped.x = Some(fixed);
        }
        if embiggens {
            stamped.embiggened = Some(action.embiggen == Some(true));
        } else if garden_radiant {
            stamped.embiggened = Some(true);
        }
        if let Some((reveal, turn, echo)) = face_down_stamp {
            stamped.set_as = Some(crate::state::SetAs {
                reveal,
                set_turn: turn,
                echo,
                revealing: None,
            });
        }
    }
    let card = live_or(sink.state, &card);
    let state: &GameState = sink.state;

    // R65, R454, R455: the price of this play, the player's prices included wherever the card lies.
    let cost = play_cost(state, &card);
    why_play_banned(state, player, &card, cost)?;
    if from == PlayFrom::Graveyard {
        why_graveyard_play_refused(state, player, &card, cost, action.plague.as_ref())?;
    // R1223: the refusal reads the one affordability function, as `legal_actions` does.
    } else {
        // R979000: `credit::why_unaffordable`'s sentence, so the reducer and the view refuse alike.
        crate::credit::why_unaffordable(state, &card, cost)?;
    }

    // Step 1 accepted the zone; a play that named none takes the leftmost free one — or, on a full
    // row, the leftmost one its own Tribute empties (R391).
    let tributes: Vec<String> = action.tributes.clone().unwrap_or_default();
    let mut zone: Option<ZoneSlot> = None;
    if is_permanent(state, &card) {
        zone = match &action.zone {
            None => default_zone_for(state, player, &card, &tributes),
            Some(named) => Some(ZoneSlot {
                player,
                row: named.row,
                lane: named.lane,
            }),
        };
        if zone.is_none() {
            return Err(EngineError::new("no free zone"));
        }
    }

    let modes: Vec<String> = action.modes.clone().unwrap_or_default();
    let declared: Vec<Selection> = action.targets.clone().unwrap_or_default();
    // R221, R90: each declaration's picks are a set, taken in the order it offers them, so a listing
    // `legalActions` never offers resolves as the offered one does. The face is step 5's (R214).
    let face = resolving_face(state, player, &card, cost);
    let targets = in_declared_order(state, player, &face, &declared, &modes, None);

    let mut run = blank_run(card.id.clone(), card.def_id.clone(), player);
    run.cost_paid = cost;
    // R1180004: a health price pays `multiple` times this cost in health, with the Echo it gives.
    if let Some((multiple, echo)) = crate::mana::health_price_rule(state, &card) {
        run.health_price = Some(multiple.saturating_mul(cost.max(0)));
        run.health_echo = echo.filter(|echo| *echo > 0);
    }
    run.zone = zone;
    // R1086: an Equip play's host, off the function the listing and the refusal read.
    if action.equip == Some(true) {
        run.equip_host = zone.and_then(|slot| equip_host_at(state, player, &slot, &tributes));
    }
    // B5 E5, R450, R682: read off the picks as checked, before an interception moves any.
    run.targeting_owed = targeting_discards_required(state, player, &face, &targets, &modes, None);
    run.targets = targets;
    run.modes = modes;
    run.tributes = tributes;
    run.at = 1;
    run.gifted = Some(play_made_radiant(state, player, &card, cost));
    // R1181002: whether a Garden plays the card Embiggened, fixed here as `gifted` is (R214); R1563004:
    // or a card of one price took its Radiant price, whose stamp step 3 takes off.
    run.garden = (crate::mana::garden_reaches(state, &card) || garden_radiant).then_some(true);
    // R983: the Feng Shui verdicts, fixed here as `gifted` is (R214).
    let fixed = crate::subsystems::feng_shui::verdicts(state, player, &card);
    run.feng_shui = (!fixed.is_empty()).then_some(fixed);
    run.target_slices = slices_for(state, player, &card, cost, &run.targets, &run.modes);
    run.exits_from = Some(exit_mark(state));
    let begins = play_begins(state, player);
    run.standing = Some(begins.standing);
    run.standing_from = Some(begins.standing_from);
    run.mods_before = Some(begins.mods_before);
    run.mana_before = Some(state.players[player].mana.current);
    // B5 E14, R546: a copier's copy is fixed as its play is checked, the choices with it.
    if copies_text(&card) {
        run.copied = Some(copied_text_of(state, &card));
    }
    if from == PlayFrom::Graveyard {
        run.source = Some(PlaySource::Graveyard);
        // R783013: a play only the card's own Flashback allows spends it, read now: a Tribute at step 2
        // may take the permission that allowed it off the field.
        if crate::graveyard_play::spends_flashback(state, player, &card, cost, action.plague.as_ref()) {
            run.flashback = Some(true);
        }
    }
    if let Some(plague) = &action.plague {
        run.plague = Some(PlagueSpend {
            from: plague.from.clone(),
            tokens: plague.tokens,
        });
    }
    Ok(run)
}

/// R119: the board and the player's modifiers as a play or a cast begins, which it answers against.
fn play_begins(state: &GameState, player: PlayerId) -> PlayBegins {
    PlayBegins {
        standing: field_card_ids(state),
        standing_from: exit_mark(state),
        mods_before: state.players[player]
            .mods
            .iter()
            .map(|modifier| modifier.id.clone())
            .collect(),
    }
}

/// R90, R102: the split step 1 checked, kept for a fused card, whose Cry hands each ingredient its
/// slice (`subsystems/fuse.rs`); the face is step 5's (R214). `None` for any other card.
fn slices_for(
    state: &GameState,
    player: PlayerId,
    card: &CardInstance,
    cost: i32,
    targets: &[Selection],
    modes: &[String],
) -> Option<Vec<usize>> {
    if !state.transient_defs.contains_key(&card.def_id) {
        return None;
    }
    let face = resolving_face(state, player, card, cost);
    Some(declaration_slices(state, player, &face, targets, modes))
}

// Step 2 — pay (§10.5 step 2)

/// §6.3 Tribute: sacrifice the units this play named, as given: step 1 judged them (#55, Sheep). The
/// set dies together, its Death hooks in R68's order whatever the listing (`sacrifice_together`), so
/// every listing `reduce` accepts is the one play `legalActions` offers.
fn pay_tributes(sink: &mut EngineSink<'_>, run: &PlayRun) {
    if run.tributes.is_empty() {
        return;
    }
    let units: Vec<CardInstance> = run
        .tributes
        .iter()
        .filter_map(|id| find_instance(sink.state, id))
        .filter(|unit| matches!(unit.zone, Zone::Field { .. }))
        .cloned()
        .collect();
    sacrifice_together(sink, &units);
}

/// R360: whether the Tribute this play pays takes a unit the opponent controls, read before it dies.
fn tributes_an_enemy(state: &GameState, run: &PlayRun) -> bool {
    run.tributes.iter().any(|id| {
        find_instance(state, id)
            .is_some_and(|unit| matches!(unit.zone, Zone::Field { .. }) && unit.controller != run.player)
    })
}

/// R360: where #55's base face lands when its Tribute took an opposing unit: the opponent's zone in
/// the named lane if open, else their leftmost open unit zone (R15). With none it stays where named,
/// as a steal with no zone does (R15). A Flying Golem named into the backrow goes to the unit row
/// (R1081000).
fn handed_over_zone(state: &GameState, run: &PlayRun, card: &CardInstance) -> Option<ZoneSlot> {
    if run.enemy_tributed != Some(true) {
        return None;
    }
    let zone = run.zone?;
    if crate::scripts::flags_of(state, card).enemy_tribute_hands_over != Some(true) {
        return None;
    }
    let opponent = opponent_of(run.player);
    let same_lane = ZoneSlot {
        player: opponent,
        row: Row::Units,
        lane: zone.lane,
    };
    if is_open(state, same_lane) {
        Some(same_lane)
    } else {
        first_free_zone(state, opponent, Row::Units)
    }
}

/// §10.5 step 2: "consume the next-spell discount if used". An X-cost card ignores discounts and so
/// spends none (R65); Professor Curvature's is a next-turn modifier, not a one-shot (R48).
fn consume_used_discounts(sink: &mut EngineSink<'_>, run: &PlayRun, card: &CardInstance) {
    let card = live_or(sink.state, card);
    if is_x_cost(sink.state, &card) {
        return;
    }
    let type_ = crate::alt_play::priced_type_of(sink.state, &card);
    let mods = sink.state.players[run.player].mods.clone();
    for modifier in &mods {
        let ModifierKind::CostDiscount {
            only_type,
            once_per_turn,
            ..
        } = &modifier.kind
        else {
            continue;
        };
        // §2.2's Lunar Eclipse discount is "consumed on use" AND expired at cleanup, which no expiry
        // says alone (`used` survives cleanup, keeping #79's `echoNextSpell` alive, R30). So it keeps
        // the "this turn" expiry and `oncePerTurn` consumes it on the first matching play (§8 row 35).
        if modifier.expiry != ModifierExpiry::Used && *once_per_turn != Some(true) {
            continue;
        }
        if !modifier_is_live(sink.state, modifier) {
            continue;
        }
        if let Some(only) = only_type
            && *only != type_
        {
            continue;
        }
        remove_modifier(sink, run.player, &modifier.id);
    }
}

/// R1180004: a play the health price paid for spends it — every `HealthPrice` its player holds, since
/// a newer one replaces an older (`effects::price_next_spell_in_health`).
fn spend_health_prices(sink: &mut EngineSink<'_>, player: PlayerId) {
    let ids: Vec<String> = sink.state.players[player]
        .mods
        .iter()
        .filter(|modifier| matches!(modifier.kind, ModifierKind::HealthPrice { .. }))
        .map(|modifier| modifier.id.clone())
        .collect();
    for id in &ids {
        remove_modifier(sink, player, id);
    }
}

/// §6.2 Echo, R30: what this play's resolution owes, queued once — the card's Echo and R30's grant
/// (`queue_echo_repeats`), and R1180004's, the Echo a health price gave the Spell it priced.
fn queue_play_echo(sink: &mut EngineSink<'_>, run: &PlayRun, card: &CardInstance) {
    queue_echo_repeats(sink, card, run.player);
    if let Some(echo) = run.health_echo {
        add_echo_repeats(sink, card, run.player, echo);
    }
}

fn pay_step(sink: &mut EngineSink<'_>, run: &mut PlayRun) {
    let Some(card) = snapshot(sink.state, &run.instance_id) else {
        return;
    };
    // R455: the "until used" price rules this play met, read before a Tribute changes the board.
    let spent_rules = cost_rules_spent_by(sink.state, &card);

    // R454: Plague Counters pay their part of the price, and the mana the rest.
    // R1223: the rest may be borrowed past current mana, up to the credit limit, and counts as spent.
    // R1180004: a play the health price reaches takes no mana; it loses health instead, below.
    let due = if run.health_price.is_some() {
        0
    } else {
        mana_due(run.cost_paid, run.plague.as_ref())
    };
    let before = sink.state.players[run.player].mana.current;
    let borrowed = crate::credit::pay_mana(sink.state, run.player, due);
    let changed = mana_event(run.player, &sink.state.players[run.player]);
    sink.events.push(changed);
    // MD-D26: the mana actually taken, which answers `manaSpent` while something hears it.
    let taken = before - sink.state.players[run.player].mana.current + borrowed;
    push_mana_spent(sink, run.player, taken, ManaSpentFor::Play);
    if let Some(plague) = run.plague.clone()
        && let Some(holder) = snapshot(sink.state, &plague.from)
    {
        spend_plague_tokens(sink, &holder, plague.tokens);
    }
    // R1180004, R18: the health price is lost, not dealt (no Armor, no cap, no hit answered); a hero
    // at 0 loses at the next state check. A (0) Spell loses nothing and still spends the price.
    if let Some(lost) = run.health_price {
        crate::damage::lose_health(sink, run.player, lost);
        spend_health_prices(sink, run.player);
    }
    // B5 E5, R450, R682: a targeting cost is paid with the price (Classic #89): random hand cards,
    // never the card played or a hand card it picks, step 1's count whatever the interception did.
    if run.targeting_owed > 0 {
        let keep = play_uses(&card, &run.targets);
        pay_targeting_discards(sink, run.player, run.targeting_owed, &keep);
    }
    // R210: the zone step 1 accepted is the play's until step 4. A tributed unit's Death (#22's
    // copies, #86's steals) lands cards by R64 and R15 in that row, so the zone is held like a Reborn
    // zone (R64) and the played card is never left with none.
    if let Some(zone) = run.zone
        && !run.tributes.is_empty()
    {
        reserve_zone(sink.state, zone);
    }
    if tributes_an_enemy(sink.state, run) {
        run.enemy_tributed = Some(true);
    }
    pay_tributes(sink, run);
    consume_used_discounts(sink, run, &card);
    for id in &spent_rules {
        remove_modifier(sink, run.player, id);
    }
}

// Step 3 — the Gifted Program hook (§10.5 step 3)

/// §10.5 step 3: #64 Gifted Program, a static flag read first (R213; step 1 needs the face, R214),
/// then every `onPlayHook` in R68's order, before the card moves. The cursor advances before each
/// hook, so one that opens a prompt never makes a hook run twice.
fn gifted_hook_step(sink: &mut EngineSink<'_>, run: &mut PlayRun) {
    // R213: #64's own rule, which step 1 read too (R214), so the two agree. Once, before the hooks.
    if run.hook_ids.is_none() {
        // R449: a replacement comes first, so what step 3 makes Radiant is the card that is played.
        replace_played_card(sink, run);
        // R983: Feng Shui judges the card that is played, before it is made Radiant.
        feng_shui_step(sink, run);
        gifted_program_step(sink, run);
        garden_step(sink, run);
        // R1845001: the card played takes its hidden Nerfs, after a replacement and with its face set.
        crate::effects::hidden_nerfs::land_hidden_nerfs(sink, &run.instance_id);
        run.hook_ids = Some(
            trigger_holders_with_hook(sink.state, HookName::OnPlayHook, None)
                .into_iter()
                .map(|holder| holder.card.id.clone())
                .collect(),
        );
        run.hooks_from = Some(exit_mark(sink.state));
    }
    let ids = run.hook_ids.clone().unwrap_or_default();
    for at in run.hook_at..ids.len() {
        run.hook_at = at + 1;
        // Each holder is read again as its turn comes: one an earlier hook moved has nothing to run
        // (R153, R174), and one that left the field since the step began is not the holder it was,
        // even back through Reborn (R83).
        let Some((holder_card, holder_controller)) =
            holder_with_on_play_hook(sink.state, ids.get(at).map(String::as_str), run.hooks_from)
                .map(|holder| (holder.card.clone(), holder.controller))
        else {
            continue;
        };
        let mut data: IndexMap<String, Value> = IndexMap::new();
        data.insert("playedId".to_string(), json!(run.instance_id));
        data.insert("playedBy".to_string(), json!(run.player));
        data.insert("costPaid".to_string(), json!(run.cost_paid));
        run_hook_resumable(
            sink,
            &holder_card,
            "onPlayHook",
            HookResumableOptions {
                controller: Some(holder_controller),
                targets: Some(vec![Selection::Instance {
                    instance_id: run.instance_id.clone(),
                }]),
                modes: None,
                data: Some(data),
                exits_from: None,
            },
        );
        if paused(sink) {
            return;
        }
    }
}

/// The `onPlayHook` holder a card is right now, or null when its zone no longer registers one, or
/// when it has left the field since `from` — the stay step 3 began with has ended (R174).
fn holder_with_on_play_hook(state: &GameState, id: Option<&str>, from: Option<u32>) -> Option<TriggerHolder> {
    let card = find_instance(state, id?)?;
    if !matches!(card.zone, Zone::Field { .. }) {
        return None;
    }
    if let Some(from) = from
        && left_field_after(state, from, &card.id)
    {
        return None;
    }
    let holder = trigger_holder_for(state, card)?;
    if holder.script.on_play_hook.is_none() {
        None
    } else {
        Some(holder)
    }
}

/// R983, Meditative #40 Feng Shui: judge the card played, after a replacement (R449) and before it is
/// made Radiant; a cast or a replaced card is judged off the board now (R70), stored for step 7. A
/// punished play takes Brittle as `effects/brittle.rs` gives it: on a copy, written back.
fn feng_shui_step(sink: &mut EngineSink<'_>, run: &mut PlayRun) {
    let Some(card) = snapshot(sink.state, &run.instance_id) else {
        return;
    };
    let verdicts = match run.gifted {
        Some(_) => run.feng_shui.clone().unwrap_or_default(),
        None => crate::subsystems::feng_shui::verdicts(sink.state, run.player, &card),
    };
    if verdicts.is_empty() {
        return;
    }
    run.feng_shui = Some(verdicts.clone());
    for verdict in &verdicts {
        sink.events.push(GameEvent::FengShui {
            instance_id: card.id.clone(),
            source_id: verdict.source_id.clone(),
            player: run.player,
            outcome: verdict.outcome,
        });
    }
    if !verdicts
        .iter()
        .any(|verdict| verdict.outcome == FengShuiOutcome::Negative)
    {
        return;
    }
    let mut copy = card.clone();
    crate::brittle_count::give_brittle_count(sink.state, &mut copy, FENG_SHUI_BRITTLE);
    if let Some(live) = find_instance_mut(sink.state, &card.id) {
        live.brittle = copy.brittle;
    }
    sink.events.push(GameEvent::CounterChanged {
        instance_id: card.id.clone(),
        counter: CounterKind::Brittle,
        value: FENG_SHUI_BRITTLE,
        placed: None,
    });
}

/// §8 #64 Gifted Program, a static flag (`play_choices::gifted_makes_radiant`, R213) so step 1 knows
/// the face before it checks the choices (R214). Setting it is §6.3's Make Radiant: the card's stats
/// and text swap on the next read (§5.2, R74).
fn gifted_program_step(sink: &mut EngineSink<'_>, run: &mut PlayRun) {
    // R2179002: a cast on draw is no card played: not the first (the first is still to come), and it
    // spends no "next card you play".
    if cast_on_draw(run) {
        return;
    }
    let Some(card) = snapshot(sink.state, &run.instance_id) else {
        return;
    };
    let gifted = match run.gifted {
        Some(gifted) => gifted,
        None => play_made_radiant(sink.state, run.player, &card, run.cost_paid),
    };
    // R831005: Prestige's "the next card you play" is spent by this play, Radiant already or not, as
    // `stamp_next_spell` spends Forever&'s.
    let mods = sink.state.players[run.player].mods.clone();
    for modifier in &mods {
        if matches!(modifier.kind, ModifierKind::RadiantNextPlay) && modifier_is_live(sink.state, modifier) {
            remove_modifier(sink, run.player, &modifier.id);
        }
    }
    if !gifted {
        return;
    }
    // R177: reported whether or not the card was Radiant already: the other seat sees redacted events
    // (R97, R33), so a cue only on a change would leak a trap's face. That Gifted Program applies is
    // public anyway.
    let event = {
        let Some(made) = find_instance_mut(sink.state, &card.id) else {
            return;
        };
        made.radiant = true;
        GameEvent::RadiantSet {
            instance_id: made.id.clone(),
            def_id: made.def_id.clone(),
            zone: made.zone.clone(),
        }
    };
    sink.events.push(event);
}

/// Classic #23 Devil's Pact, R449: each live `replacePlays` modifier, in creation order, replaces the
/// card by a new instance in the resolving zone, which chooses as a cast does (R70). The old card
/// ceases to exist where it was (R35, R454), its price paid; one already the named card stays.
fn replace_played_card(sink: &mut EngineSink<'_>, run: &mut PlayRun) {
    let mods = sink.state.players[run.player].mods.clone();
    for modifier in &mods {
        let ModifierKind::ReplacePlays { def_id, radiant } = &modifier.kind else {
            continue;
        };
        if !modifier_is_live(sink.state, modifier) {
            continue;
        }
        let Some(old) = snapshot(sink.state, &run.instance_id) else {
            return;
        };
        if old.def_id == *def_id && old.radiant == *radiant {
            continue;
        }
        // R226: a card already taken out of the hand (or the resolving zone) is not played at all. R454: a
        // play from the graveyard is a play as from hand, so its card is replaced there.
        let zone = old.zone.clone();
        if run.source == Some(PlaySource::Graveyard) {
            if !in_own_graveyard(sink.state, run.player, &old) {
                return;
            }
        } else {
            match zone {
                Zone::Hand { player } => {
                    if player != run.player {
                        return;
                    }
                }
                Zone::Resolving { .. } => {}
                _ => return,
            }
        }
        // R177: a card replaced in a hand was never the other player's to read.
        let hidden_from: Vec<PlayerId> = match zone {
            Zone::Hand { player } => vec![opponent_of(player)],
            _ => Vec::new(),
        };

        let mut fresh = new_instance(
            &mut *sink.state,
            def_id,
            run.player,
            Zone::Resolving { player: run.player },
        );
        fresh.radiant = *radiant;
        cease_to_exist(sink.state, &mut old.clone());
        sink.state.players[run.player].resolving.push(fresh.clone());
        sink.events.push(GameEvent::Transformed {
            instance_id: old.id.clone(),
            from_def_id: old.def_id.clone(),
            to_def_id: fresh.def_id.clone(),
            new_instance_id: fresh.id.clone(),
            hidden_from: if hidden_from.is_empty() {
                None
            } else {
                Some(hidden_from)
            },
            creator: None,
        });

        run.instance_id = fresh.id.clone();
        run.def_id = fresh.def_id.clone();
        run.targets = Vec::new();
        run.modes = Vec::new();
        run.replaced = Some(true);
        // R454: the new card waits in the resolving zone, taken from no graveyard.
        run.source = None;
        run.target_slices = None;
        run.cast_chosen = None;
        // R214: whether step 3 makes it Radiant is read for the card that is played, off the board now.
        run.gifted = None;
        // R983: and so are the Feng Shui verdicts.
        run.feng_shui = None;
        // R1181002: a Garden reached the card that was paid for, not this one, which no step 1 checked.
        run.garden = None;
    }
}

/// R1181002: a play a Garden reached at step 1 is stamped Embiggened before the hooks and the
/// announce read it; a card already gone is not played (R226). R1563004: a card of one price has no
/// Embiggened version, so the stamp carrying its Radiant price comes off (`gifted` made it Radiant).
fn garden_step(sink: &mut EngineSink<'_>, run: &PlayRun) {
    if run.garden != Some(true) {
        return;
    }
    let Some(card) = snapshot(sink.state, &run.instance_id) else {
        return;
    };
    let playing = match run.source {
        Some(PlaySource::Graveyard) => in_own_graveyard(sink.state, run.player, &card),
        None => card.zone == (Zone::Hand { player: run.player }),
    };
    let embiggen_card = crate::play_choices::has_embiggen_price(sink.state, &card);
    if playing && let Some(live) = find_instance_mut(sink.state, &card.id) {
        live.embiggened = embiggen_card.then_some(true);
    }
}

// The announce (B5 E1, R448): between §10.5 steps 3 and 4

/// B5 E1: how an announce names what the play declared — a card by its id, a hero as `hero-<player>`.
fn announced_targets(targets: &[Selection]) -> Vec<String> {
    targets
        .iter()
        .filter_map(|selection| match selection {
            Selection::Instance { instance_id } => Some(instance_id.clone()),
            Selection::Hero { player } => Some(format!("hero-{player}")),
            _ => None,
        })
        .collect()
}

/// The zone the announce names: the one step 1 accepted for a play, and for a cast or a replacement
/// the one R64 would give it now (step 4 reads it again, as the window may have changed the row).
fn announced_zone(state: &GameState, run: &PlayRun, card: &CardInstance) -> Option<ZoneSlot> {
    if run.cast != Some(true) && run.replaced != Some(true) {
        return run.zone;
    }
    let type_ = def_of(Some(state), &card.def_id).type_;
    if type_ == CardType::Spell {
        return None;
    }
    first_free_zone(
        state,
        run.player,
        if type_ == CardType::Unit {
            Row::Units
        } else {
            Row::Backrow
        },
    )
}

/// R448: the card leaves its hand (or graveyard, R454) for the resolving zone (a cast's waits there
/// already) and `cardAnnounced` goes out, showing what `cardPlayed` would (a face-down card only its
/// zone, `view_for`). False when the card is no longer where the play takes it from (R226).
fn announce_play(sink: &mut EngineSink<'_>, run: &PlayRun) -> bool {
    let Some(mut card) = snapshot(sink.state, &run.instance_id) else {
        return false;
    };
    if run.cast == Some(true) || run.replaced == Some(true) {
        if !matches!(card.zone, Zone::Resolving { .. }) {
            return false;
        }
    } else {
        // E11, R454: from the hand, or from the graveyard for a play under a permission.
        if !take_from_play_source(sink.state, run, &mut card) {
            return false;
        }
        card.zone = Zone::Resolving { player: run.player };
        sink.state.players[run.player].resolving.push(card.clone());
        // B5 E14, R546: the copy step 1 checked the choices against is the one the card resolves.
        if let Some(copied) = &run.copied {
            fix_copied_text(sink.state, &card.id, Some(copied.clone()));
            card = live_or(sink.state, &card);
        }
    }

    let type_ = card_type_of(sink.state, &card);
    let face_down = type_ == CardType::Trap || type_ == CardType::FieldTrap;
    begin_announce(
        sink.state,
        AnnounceRecord {
            instance_id: card.id.clone(),
            player: run.player,
            face_down: if face_down { Some(true) } else { None },
            countered: None,
        },
    );
    let zone = announced_zone(sink.state, run, &card);
    sink.events.push(GameEvent::CardAnnounced {
        player: run.player,
        instance_id: card.id.clone(),
        def_id: card.def_id.clone(),
        card_type: type_,
        cost_paid: run.cost_paid,
        targets: announced_targets(&run.targets),
        row: zone.map(|zone| zone.row),
        lane: zone.map(|zone| zone.lane),
        face_down: if face_down { Some(true) } else { None },
    });
    true
}

/// Whether a queue entry is a trigger answering this card's announce (not an owed trap dispatch).
fn answers_announce(entry: &QueuedTrigger, instance_id: &str) -> bool {
    if entry.hook == OWED_TO_TRAPS {
        return false;
    }
    matches!(
        event_of_queued(entry),
        Some(GameEvent::CardAnnounced { instance_id: announced, .. }) if announced == instance_id
    )
}

/// B5 E1, R448: the announce window. Traps fire first as responses (`dispatch_pending`); then each
/// other trigger the announce woke, in queue order (R68), each followed by the state check (R59).
/// Triggers on anything else wait for the step-4 loop, or a cast's caster's (R70, R117). A Counter
/// cancels the play: later triggers find no card (`is_announce_live`), traps are not offered it
/// (`traps::standing_event`). Stops at a prompt, the rest owed in state.
fn run_announce_window(sink: &mut EngineSink<'_>, run: &PlayRun) {
    for _ in 0..SETTLE_PASS_CAP {
        dispatch_pending(sink);
        if paused(sink) {
            return;
        }
        let Some(at) = sink
            .state
            .trigger_queue
            .iter()
            .position(|entry| answers_announce(entry, &run.instance_id))
        else {
            return;
        };
        let entry = sink.state.trigger_queue.remove(at);
        if !is_announce_live(sink.state, &run.instance_id) {
            continue;
        }
        run_queued_trigger(sink, &entry);
        if paused(sink) {
            return;
        }
        state_check(sink);
        if paused(sink) {
            return;
        }
    }
    panic!("the announce window did not close in {SETTLE_PASS_CAP} passes (§10.3)");
}

/// B5 E1, R448: a cast's (or replacement's, R449) choices come first, so the announce names them (R70,
/// R90). A paused window is re-entered (`PlayRun.announced`) after the check owed (R59). Then a
/// countered play stops, and one whose card left the resolving zone is lost (R226).
fn announce_step(sink: &mut EngineSink<'_>, run: &mut PlayRun) {
    if run.announced != Some(true) {
        if (run.cast == Some(true) || run.replaced == Some(true))
            && !cast_choices_made(sink, run, PlayStepName::Announce)
        {
            return;
        }
        if !announce_play(sink, run) {
            run.lost = Some(true);
            return;
        }
        run.announced = Some(true);
    } else {
        state_check(sink);
        if paused(sink) {
            return;
        }
    }
    run_announce_window(sink, run);
    if paused(sink) {
        return;
    }
    let record = end_announce(sink.state, &run.instance_id);
    if record.is_some_and(|record| record.countered == Some(true)) {
        run.countered = Some(true);
        return;
    }
    let resolving = find_instance(sink.state, &run.instance_id)
        .is_some_and(|card| matches!(card.zone, Zone::Resolving { .. }));
    if !resolving {
        run.lost = Some(true);
    }
}

// Step 4 — move the card and announce the play (§10.5 step 4)

/// R1870000: the win a card carries (`effects::WIN_ON_PLAY_KEY`), moved to its player's won flag.
fn claim_win_on_play(sink: &mut EngineSink<'_>, player: PlayerId, instance_id: &str) {
    let Some(card) = find_instance_mut(sink.state, instance_id) else {
        return;
    };
    if card.memory.shift_remove(crate::effects::WIN_ON_PLAY_KEY) != Some(Value::Bool(true)) {
        return;
    }
    sink.state.players[player].won_by_effect = Some(true);
}

/// R2179002: whether this run is §2.4's cast on draw (`PlayRun.drawn_as`): a cast and no play, counted
/// and answered as none.
pub(crate) fn cast_on_draw(run: &PlayRun) -> bool {
    run.drawn_as.is_some()
}

fn played_events(sink: &mut EngineSink<'_>, run: &PlayRun, card: &CardInstance, former_id: Option<String>) {
    // R119: what arrived on the field during the play (a Death at step 2) does not answer it, which the
    // step-4 pair names, as step 7's does.
    let arrived = arrived_during(sink.state, run);
    let arrivals = if arrived.is_empty() {
        None
    } else {
        Some(arrived.clone())
    };
    // R174, R212: the stays the play was announced on, so a response the loop hands the pair later —
    // behind the traps that answer what step 2 did — judges the played card from here.
    let exits_from = exit_mark(sink.state);
    sink.events.push(GameEvent::CardPlayed {
        player: run.player,
        instance_id: card.id.clone(),
        def_id: card.def_id.clone(),
        cost_paid: run.cost_paid,
        x: card.x,
        embiggened: card.embiggened,
        former_id: former_id.clone(),
        // R454: a play from the graveyard says so (public), for a client to animate it from there.
        from: run.source.map(|PlaySource::Graveyard| PlayedFrom::Graveyard),
        arrived_during: arrivals.clone(),
        exits_from: Some(exits_from),
        on_draw: cast_on_draw(run).then_some(true),
    });
    // R2250001: a Resource played pays its player its Credits; a cast is no play of theirs.
    if run.cast != Some(true) {
        crate::resources::pay_resource_credits(sink, run.player, card);
    }
    if let Some(zone) = run.zone {
        sink.events.push(GameEvent::Summoned {
            // The side it lands on, which is the player's own unless R360 summoned it for the opponent.
            player: zone.player,
            instance_id: card.id.clone(),
            def_id: card.def_id.clone(),
            row: zone.row,
            lane: zone.lane,
            former_id,
            source_id: None,
            arrived_during: arrivals,
            exits_from: Some(exits_from),
            creator: None,
        });
    }
}

/// §10.5 step 4: the card moves, `cardPlayed` goes out, the play is counted (R55). It ends in the
/// resolution loop so Sheepish resolves before step 5's Cry (R17); the loop holds the check (§4.5,
/// `SettleOptions.holdCheck`), so a card arriving at 0 health is not collected before its Cry (R118).
fn place_step(sink: &mut EngineSink<'_>, run: &mut PlayRun) {
    // Re-entered after a pause: the answer resolved what asked, so §4.5's check is due before anything
    // else moves (R59), and a unit it killed dies before step 5's Cry counts the board (R118, R113).
    let resumed = run.placed == Some(true);
    if !resumed {
        // R70, R90: a cast made its choices before step 4 (`cast_choices_step`), so a cast Unit is never
        // its own Cry's option. A run owed from before that step existed makes them here.
        if run.cast == Some(true) && !cast_choices_made(sink, run, PlayStepName::CastChoices) {
            return;
        }
        run.placed = Some(true);
        if !place_card(sink, run) {
            run.lost = Some(true);
        }
    }
    // R17's step-4 window, holding §4.5's check (R118). A trap that asks pauses with events still owed
    // (`triggers::OWED_TO_TRAPS`) and resolves fully first (§10.3), so the answer comes back to this
    // loop, not step 5: a Sheepish behind it still makes the unit a Sheep before its Cry.
    if run.cast != Some(true) {
        settle(
            sink,
            SettleOptions {
                hold_check: Some(!resumed),
            },
        );
        return;
    }
    // R70: a cast's step 4 is a window too, but inside its caster's loop (§2.4's draw, #95), so only
    // traps fire here (`dispatch_pending`) and the rest waits for that loop (R117). Re-entered after a
    // trap's question, the check owed comes first (R59).
    if resumed {
        state_check(sink);
        if paused(sink) {
            return;
        }
    }
    dispatch_pending(sink);
}

/// Every card acting on the field, both sides: each Stack pile's top and the backrow. A card dormant
/// under a pile is not on the field for effects and registers nothing (§3.2, R13, R153), so one that
/// resumes while a play resolves arrives for R119 as a Reborn body does.
fn field_card_ids(state: &GameState) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for player in PLAYER_IDS {
        let side = &state.players[player];
        for pile in &side.units {
            if let Some(top) = pile.as_ref().and_then(|pile| pile.first()) {
                out.push(top.id.clone());
            }
        }
        for card in side.backrow.iter().flatten() {
            out.push(card.id.clone());
        }
        // R446: a Unit a carrier holds acts on the field too.
        for card in side.carried.iter().flatten().flatten() {
            out.push(card.id.clone());
        }
    }
    out
}

/// R119: the permanents on the field now that were not there as the play began, or left and stand
/// there again (a Reborn body, R83), either side. The played card is its own case
/// (`traps::is_own_arrival`, #33's own check).
fn arrived_during(state: &GameState, run: &PlayRun) -> Vec<String> {
    let Some(standing) = run.standing.as_ref() else {
        return Vec::new();
    };
    let standing: IndexSet<&String> = standing.iter().collect();
    let from = run.standing_from;
    field_card_ids(state)
        .into_iter()
        .filter(|id| {
            *id != run.instance_id
                && (!standing.contains(id) || from.is_some_and(|from| left_field_after(state, from, id)))
        })
        .collect()
}

/// Step 4's placement and announcement, once; false when the card is gone (`PlayRun.lost`).
fn place_card(sink: &mut EngineSink<'_>, run: &mut PlayRun) -> bool {
    // R210: step 2 held the named zone for this play; it is released here, whatever happens next.
    if let Some(zone) = run.zone {
        release_zone(sink.state, zone);
    }
    let Some(mut card) = snapshot(sink.state, &run.instance_id) else {
        return false;
    };

    // R448, R70, R226: the card waits in the resolving zone from its announce. One that left it stays
    // where it went (§10.1), unplayed: pulling it out of exile would undo a move §6.3 makes final. Not
    // `move_to_zone`: that resets the instance (R78), and a play does not.
    if !matches!(card.zone, Zone::Resolving { .. }) {
        return false;
    }
    remove_from_any_zone(sink.state, &mut card);
    if run.cast == Some(true) || run.replaced == Some(true) {
        // R70, R449: a permanent takes the leftmost empty, unlocked zone of its row (R64).
        let type_ = card_type_of(sink.state, &card);
        run.zone = if type_ == CardType::Spell {
            None
        } else {
            first_free_zone(
                sink.state,
                run.player,
                if type_ == CardType::Unit {
                    Row::Units
                } else {
                    Row::Backrow
                },
            )
        };
    } else {
        // R360: #55's base face, paid for with an opposing unit, is summoned for the opponent: still
        // this player's play and card (§3.2), in the opponent's zone and control (`place_on_field`).
        if let Some(theirs) = handed_over_zone(sink.state, run, &card) {
            run.zone = Some(theirs);
        }
    }

    // R227: a Trap or Field Trap set face-down takes a fresh id before anything names it on the field,
    // so no player can link the face-down card to an id they saw while it was public (R177). The run
    // follows the card, and the events that place it carry the id it had (`formerId`).
    let mut former_id: Option<String> = None;
    if let Some(zone) = run.zone
        && lands_face_down(sink.state, &card, zone.row)
    {
        former_id = Some(fresh_face_down_id(sink.state, &mut card));
        run.instance_id = card.id.clone();
        // MD-D28, R1125: the judgement names the play as the action named it; it follows the card to
        // its fresh id, so the trigger reads it off the play's `cardResolved`.
        if sink
            .state
            .play_judgement
            .as_ref()
            .is_some_and(|judgement| Some(&judgement.instance_id) == former_id.as_ref())
            && let Some(judgement) = sink.state.play_judgement.as_mut()
        {
            judgement.instance_id = card.id.clone();
        }
    }

    // §3.2/§6.2 Stack: a Stack card's placement builds the pile (R13); others need the zone empty. R210
    // keeps the zone the play's, so a refusal is a broken invariant; then step 7 lands the card in its
    // graveyard as R138 does a permanent with no zone, never in no pile (§10.1).
    let base = if run.cast != Some(true) {
        run.zone
            .as_ref()
            .filter(|zone| crate::zones::stack_base_at(sink.state, run.player, zone))
            .and_then(|zone| card_at(sink.state, *zone).cloned())
    } else {
        None
    };
    let landing_buffs = base.as_ref().and_then(|b| {
        crate::scripts::flags_of(sink.state, b)
            .stack_base_buffs
            .map(|n| crate::params::declared_or(sink.state, b, "buffs", n))
    });

    // R1852007: whether the play lands on a Dormant Starship, read before it lands.
    let onto_starship = run
        .zone
        .and_then(|zone| card_at(sink.state, zone))
        .is_some_and(crate::starship::exiles_stacked_play);
    let placed = match run.zone {
        Some(zone) => {
            // R1086: an Equip play lands on top of its host, as a Stack card lands on its pile.
            let stack = plays_on_stack(sink.state, &card) || base.is_some() || run.equip_host.is_some();
            let options = PlaceOnFieldOptions { stack: Some(stack) };
            if zone.row == Row::Backrow {
                // R1081000: a Unit played into the backrow was vetted as Flying in hand
                // (`play_choices::flies`) or carried, so it stands whatever its keywords read now.
                place_standing_on_field(sink.state, &mut card, zone, options)
            } else {
                place_on_field(sink.state, &mut card, zone, options)
            }
        }
        None => false,
    };
    if placed {
        let turn = sink.state.turn;
        if let Some(on_field) = find_instance_mut(sink.state, &card.id) {
            on_field.summoned_turn = Some(turn);
        }
        run.placed_from = Some(exit_mark(sink.state));
    } else {
        run.zone = None;
        card.zone = Zone::Resolving { player: run.player };
        sink.state.players[run.player].resolving.push(card.clone());
    }
    let card = live_or(sink.state, &card);
    // R453: a cast Field Spell, Trap or Field Trap with no zone fizzles; a cast permanent on the field
    // has no landing for "then exile it" to change (Classic #56's rider is a Spell's).
    if run.cast == Some(true) {
        settle_cast_placement(sink.state, run, &card);
    }

    // R119: a run owed from before the marks were kept reads the board the play was announced on.
    if run.standing.is_none() {
        run.standing = Some(field_card_ids(sink.state));
        run.standing_from = Some(exit_mark(sink.state));
    }

    // R2179002: a cast on draw is a cast and no play, so none of a play's counts and records takes it.
    if !cast_on_draw(run) {
        {
            let side = &mut sink.state.players[run.player];
            side.turn_log.played_ids.push(card.id.clone());
            side.turn_log.cards_played += 1;
            // R213: what this play paid, which the next play's Gifted Program check counts (R56, R70).
            let mut costs = side.turn_log.costs_paid.clone().unwrap_or_default();
            costs.push(run.cost_paid);
            side.turn_log.costs_paid = Some(costs);
        }
        sink.state.counters.played += 1;
        // R429: a card that counts its own plays counts this one here, with every other count of it.
        crate::times_played::count_play(sink.state, &card);
    }
    let card = live_or(sink.state, &card);
    // R673: a play of a "… in the System" card, by either player, raises the match's Glitch odds.
    if run.cast != Some(true) {
        count_system_play(sink.state, &card);
    }
    run.radiant = Some(card.radiant);
    // B5 E4, R451: the per-turn types, the per-game tags and the "last" records.
    if !cast_on_draw(run) {
        record_play(sink.state, run.player, &card);
    }

    played_events(sink, run, &card, former_id);
    // R1870000: a card carrying the win wins as it is played, a cast too (R70); the closing state
    // check holds the flag, a hero at 0 losing first (R850).
    claim_win_on_play(sink, run.player, &card.id);
    // R1852007: a Stack card played onto a Dormant Starship is counted, then exiled before it resolves
    // (its Cry lost, R17); the Starship tops its zone again, still Dormant.
    if placed && onto_starship {
        if let Some(on_top) = find_instance(sink.state, &card.id).cloned() {
            crate::zones::exile_to_pile(sink, on_top);
        }
        run.echo_queued = true;
        return true;
    }
    // R783010: a Crater Unit Locks the zone it was played into, before its Cry.
    crate::crater::lock_on_arrival(sink, &card.id);
    if placed
        && let Some(times) = landing_buffs
        && times > 0
    {
        let mut ctx = crate::resolve::make_context(
            sink,
            base.as_ref(),
            crate::resolve::HookOptions {
                controller: Some(run.player),
                ..Default::default()
            },
        );
        crate::resolve::apply_effects(
            &[crate::effects::upgrade(crate::effects::TuneArgs {
                instance_id: Some(card.id.clone()),
                times: Some(times),
                ..Default::default()
            })],
            &mut ctx,
        );
    }
    // B3.1 rule 4 (R383): an Animated Field Spell, or an "Animated on your turn" card on its controller's
    // turn, animates as it enters the field.
    animate_on_entry(sink, &card);
    // §6.2 Echo, R30: the Spell gains its Echo as it is played, so the grant is taken here, not at
    // step 6 after its text ran (a Spell moving Twinspell away, #87's swap, would take nothing). The
    // repeats still resolve at step 6.
    run.echo_queued = true;
    let card = live_or(sink.state, &card);
    queue_play_echo(sink, run, &card);
    // E39 (Classic+ #14): "the next Spell you play gains …" is gained as it is played, as Echo is (R178).
    let card = live_or(sink.state, &card);
    stamp_next_spell(sink, &card, run.player);
    // R1848002: "your next Spell this turn has Poisonous" is had as it is played, as Forever&'s is.
    let card = live_or(sink.state, &card);
    give_next_spell_keywords(sink, &card, run.player);
    true
}

/// E11, R454, §10.5 step 4: take the played card out of its hand or graveyard (E1's announce too);
/// false, nothing moved, when it is gone (R226). The removal's change is made on `card`, the
/// caller's copy it goes on to place.
pub fn take_from_play_source(state: &mut GameState, run: &PlayRun, card: &mut CardInstance) -> bool {
    if run.source == Some(PlaySource::Graveyard) {
        if !in_own_graveyard(state, run.player, card) {
            return false;
        }
        remove_from_any_zone(state, card);
        // R155: leaving the graveyard spends the landing §5.1's return belongs to (§10.5 step 7).
        card.return_to_hand_at_end_of_turn = None;
        // R783013: a play by its Flashback spends it as the card leaves the graveyard.
        if run.flashback == Some(true) {
            card.flashback_spent = Some(true);
        }
        return true;
    }
    let side = &mut state.players[run.player];
    let Some(at) = side.hand.iter().position(|held| held.id == card.id) else {
        return false;
    };
    side.hand.remove(at);
    // R1140: a card played out of a hand has ended its stay there.
    crate::zones::forget_hand_watch(state, &card.id);
    true
}

/// R453: a cast permanent's placement settled — fizzled when it found no zone, the Spell rider dropped.
fn settle_cast_placement(state: &mut GameState, run: &mut PlayRun, card: &CardInstance) {
    if run.zone.is_some() {
        if let Some(placed) = find_instance_mut(state, &card.id) {
            placed.memory.shift_remove(EXILE_ON_LANDING);
        }
        return;
    }
    let type_ = card_type_of(state, card);
    if type_ != CardType::Spell && type_ != CardType::Unit {
        run.fizzled = Some(true);
    }
}

/// E39, R455 (Classic+ #14 Forever&): each live `enchantNextSpell` modifier gives its enchantment to
/// the Spell played and is spent; a cast takes it (R70), a countered play leaves it waiting.
fn stamp_next_spell(sink: &mut EngineSink<'_>, card: &CardInstance, player: PlayerId) {
    if card_type_of(sink.state, card) != CardType::Spell {
        return;
    }
    let mods = sink.state.players[player].mods.clone();
    for modifier in &mods {
        let ModifierKind::EnchantNextSpell { enchantment } = &modifier.kind else {
            continue;
        };
        if !modifier_is_live(sink.state, modifier) {
            continue;
        }
        if let Some(stamped) = find_instance_mut(sink.state, &card.id) {
            let mut list = stamped.enchantments.clone().unwrap_or_default();
            list.push(enchantment.clone());
            stamped.enchantments = Some(list);
        }
        remove_modifier(sink, player, &modifier.id);
    }
}

/// R1848002 (Stinkyscape #13): each live `keywordNextSpell` modifier grants its keyword to the Spell
/// played face-up and is spent (§4.4 step 7, R346); R78's reset as it lands ends it. A cast takes it
/// (R70); a face-down Spell (a Trap till it resolves, R783005) or a countered play leaves it waiting.
fn give_next_spell_keywords(sink: &mut EngineSink<'_>, card: &CardInstance, player: PlayerId) {
    if card_type_of(sink.state, card) != CardType::Spell || card.set_as.is_some() {
        return;
    }
    let mods = sink.state.players[player].mods.clone();
    for modifier in &mods {
        let ModifierKind::KeywordNextSpell { keyword } = &modifier.kind else {
            continue;
        };
        if !modifier_is_live(sink.state, modifier) {
            continue;
        }
        let given = find_instance_mut(sink.state, &card.id).is_some_and(|stamped| {
            let had = stamped
                .granted_keywords
                .iter()
                .any(|held| held.kind() == keyword.kind());
            if !had {
                stamped.granted_keywords.push(keyword.clone());
            }
            !had
        });
        if given {
            sink.events.push(GameEvent::KeywordGranted {
                instance_id: card.id.clone(),
                keyword: keyword.clone(),
                lost: None,
            });
        }
        remove_modifier(sink, player, &modifier.id);
    }
}

// Step 5 — resolve the card (§10.5 step 5)

/// The card as step 4 left it, or `None` when it is no longer there to resolve (transformed,
/// countered, ceased to exist, R11): the Cry is lost (R17).
fn still_resolving(state: &GameState, run: &PlayRun) -> Option<CardInstance> {
    let card = find_instance(state, &run.instance_id)?;
    if card.def_id != run.def_id {
        return None;
    }
    match card.zone {
        Zone::Resolving { .. } => Some(card.clone()),
        Zone::Field { .. } => {
            // R174, R118: the stay step 4 put it on. One it left since is gone for the play; a Reborn
            // body in its zone is a new arrival whose "Cry does not fire" (§4.5 step 4, R1).
            if run
                .placed_from
                .is_some_and(|from| left_field_after(state, from, &card.id))
            {
                None
            } else {
                Some(card.clone())
            }
        }
        _ => None,
    }
}

/// #38 Quickstriker's grants from this player's side: one multiple of X per grant, its granter's face's
/// (`QUICKSTRIKER_COMBO_MULTIPLE`, R281; a fused card carries both texts, R102). Never the played card
/// nor one that arrived during the play (`arrived_during`, R119), nor a face-down card (R1487000). A
/// carried Unit grants as any permanent does (R446, R1824000).
fn quickstriker_grants(state: &GameState, run: &PlayRun, played: &CardInstance) -> Vec<i32> {
    let arrived: IndexSet<String> = arrived_during(state, run).into_iter().collect();
    let mut multiples: Vec<i32> = Vec::new();
    for held in crate::zones::acting_on(state, run.player, crate::zones::StandingAt::ItsLane) {
        // R1487000: a face-down card's text sleeps until it is revealed or face-up.
        if held.id == played.id || arrived.contains(&held.id) || crate::preview::lasting_text_off(state, held)
        {
            continue;
        }
        let grants = match crate::scripts::flags_of(state, held).quickstriker {
            Some(FlagOrCount::Flag(true)) => 1,
            Some(FlagOrCount::Count(count)) => count.max(0),
            _ => 0,
        };
        let multiple = QUICKSTRIKER_COMBO_MULTIPLE.on(held.radiant);
        for _ in 0..grants {
            multiples.push(multiple);
        }
    }
    multiples
}

/// #38 Quickstriker's granted Combo, read off the field now, not off a `cardPlayed` trigger: a cast's
/// events wait for its caster's loop (R70). R281: each grant is ONE damage instance of X times its
/// multiple (§4.4), in board order; a `quickstrikerDamage` modifier is one more grant of X.
fn quickstriker_combo(sink: &mut EngineSink<'_>, run: &PlayRun, card: &CardInstance) {
    let amount = played_earlier(sink.state, run.player, &run.instance_id);
    if amount <= 0 {
        return;
    }
    let riders = sink.state.players[run.player]
        .mods
        .iter()
        .filter(|modifier| {
            matches!(modifier.kind, ModifierKind::QuickstrikerDamage)
                && modifier_is_live(sink.state, modifier)
                && in_place_before(run, &modifier.id)
        })
        .count();
    let mut multiples = quickstriker_grants(sink.state, run, card);
    multiples.extend(std::iter::repeat_n(QUICKSTRIKER_COMBO_MULTIPLE.base, riders));
    for multiple in multiples {
        let source = live_or(sink.state, card);
        deal_damage(
            sink,
            DamageArgs {
                source: Some(source),
                target: DamageTarget::Hero {
                    player: opponent_of(run.player),
                },
                amount: amount * multiple,
                flags: None,
            },
        );
    }
}

/// #78 /fullsend's granted "Combo: draw 1" (a Combo with no X is Combo 1, §6.2). The riders are one
/// `draw` of their total ("draw N" is N draws, §2.4, R58), so a pause owes the rest to the answer
/// (R113) and a cast that ends the game ends the rest (R216).
fn combo_draw_step(sink: &mut EngineSink<'_>, run: &PlayRun) {
    if played_earlier(sink.state, run.player, &run.instance_id) < 1 {
        return;
    }
    let mut draws = 0;
    for modifier in &sink.state.players[run.player].mods {
        if let ModifierKind::ComboDraw { amount } = modifier.kind
            && modifier_is_live(sink.state, modifier)
            && in_place_before(run, &modifier.id)
        {
            draws += amount.max(0);
        }
    }
    if draws > 0 {
        draw(sink, run.player, draws);
    }
}

/// R119: whether a modifier was in place as the play began (`PlayRun.modsBefore`); one the play
/// installed (met again by its own Echo repeat) does not answer it.
fn in_place_before(run: &PlayRun, id: &str) -> bool {
    run.mods_before
        .as_ref()
        .is_none_or(|before| before.iter().any(|known| known == id))
}

/// R174: the targets step 5 hands the script. Step 1 checks targets and Tribute apart (R90), so a
/// unit may be both (#55 crafted onto #68); step 2 sacrificed it, so its slot fizzles (§8
/// Conventions), Reborn body or not. The slot is kept as `none` so later declarations read their own.
fn standing_targets(run: &PlayRun) -> Vec<Selection> {
    if run.tributes.is_empty() {
        return run.targets.clone();
    }
    run.targets
        .iter()
        .map(|selection| match selection {
            Selection::Instance { instance_id } if run.tributes.contains(instance_id) => Selection::None,
            other => other.clone(),
        })
        .collect()
}

/// What the card's Cry is handed in its data: a fused card's declaration slices (R90, R102), and the
/// player's mana as the play began (`EffectContext.mana_before_play`, Classic #22).
fn cry_data(run: &PlayRun) -> IndexMap<String, Value> {
    let mut data: IndexMap<String, Value> = IndexMap::new();
    if let Some(slices) = &run.target_slices {
        data.insert(DECLARATION_SLICES_KEY.to_string(), json!(slices));
    }
    if let Some(mana) = run.mana_before {
        data.insert(MANA_BEFORE_PLAY_KEY.to_string(), json!(mana));
    }
    data
}

/// §10.5 step 5: the two granted Combos (Quickstriker, /fullsend's draw), then the card's own script.
/// A card's own Combo check is in its script (`query::played_earlier`).
fn resolve_step(sink: &mut EngineSink<'_>, run: &mut PlayRun) {
    if run.cast == Some(true) && !cast_choices_made(sink, run, PlayStepName::Resolve) {
        return;
    }
    // R453: a cast that found no zone for its Field Spell or Trap resolves nothing.
    if run.fizzled == Some(true) {
        return;
    }
    for (at, part) in RESOLVE_PARTS.iter().enumerate().skip(run.resolve_at) {
        run.resolve_at = at + 1;
        let Some(card) = still_resolving(sink.state, run) else {
            return;
        };

        match part {
            ResolvePart::Quickstriker => quickstriker_combo(sink, run, &card),
            ResolvePart::ComboDraw => combo_draw_step(sink, run),
            ResolvePart::Script => {
                // B5 E14, R399: a copier resolves the copied Spell's script, itself as "this".
                let face = text_face_of(sink.state, &card);
                // R822, R823: a permanent's Cry runs again under its player's Cry multiplier, read now; a
                // Spell's resolution is no Cry, though it runs from the same hook.
                if is_permanent(sink.state, &card)
                    && crate::scripts::script_of(sink.state, &face).cry.is_some()
                {
                    let extra = extra_runs(sink.state, run.player, Multiplied::CryAndDeath);
                    if extra > 0 {
                        run.extra_cries = Some(extra as u32);
                    }
                }
                run_script_cry(sink, run, &face);
            }
        }

        if paused(sink) {
            return;
        }
    }
    resolve_extra_cries(sink, run);
}

/// Step 5's run of the card's own Cry or spell script, with the play's choices.
fn run_script_cry(sink: &mut EngineSink<'_>, run: &PlayRun, face: &CardInstance) {
    // R1180005: a Spell's declared targets, or a cast's choices, are what it targets as it resolves.
    crate::targeted::note_spell_targets(sink.state, &run.instance_id, &run.targets);
    run_hook_resumable(
        sink,
        face,
        "cry",
        HookResumableOptions {
            controller: Some(run.player),
            targets: Some(standing_targets(run)),
            modes: Some(run.modes.clone()),
            data: Some(cry_data(run)),
            // R174: the choices are aimed at the stays step 1 checked them on (a cast's, once made).
            exits_from: run.exits_from,
        },
    );
}

/// R822, R823: a permanent's extra Cry runs on the play's choices (R174), each closed by the check
/// (R59), until the card leaves its stay (R118). A prompt parks the play here behind that run's tail
/// (`extra_cries`, R113, R117). An Echo repeat (step 6) is a new resolution, never multiplied.
fn resolve_extra_cries(sink: &mut EngineSink<'_>, run: &mut PlayRun) {
    while let Some(left) = run.extra_cries {
        state_check(sink);
        if paused(sink) {
            return;
        }
        let Some(card) = still_resolving(sink.state, run) else {
            run.extra_cries = None;
            return;
        };
        run.extra_cries = if left > 1 { Some(left - 1) } else { None };
        let face = text_face_of(sink.state, &card);
        run_script_cry(sink, run, &face);
        if paused(sink) {
            return;
        }
    }
}

/// R70, R81: the caster picks a cast's targets and modes as prompts, as an Echo repeat does (§10.6),
/// for the face step 3 settled (R214), before the card is placed (R90), so a cast Unit is never its
/// own Cry's target. Choices its caller named are kept. False while a prompt waits.
fn cast_choices_made(sink: &mut EngineSink<'_>, run: &mut PlayRun, step: PlayStepName) -> bool {
    if run.cast_chosen == Some(true) {
        return true;
    }
    let Some(card) = still_resolving(sink.state, run) else {
        return true;
    };
    // R453: an X card's X first, so its targets and modes are asked of the card as it will resolve.
    if !cast_x_chosen(sink, run, &card, step) {
        return false;
    }
    let card = live_or(sink.state, &card);
    if run.repeat.is_none() {
        // B5 E14, R399: a copier's choices are its copied text's.
        let face = text_face_of(sink.state, &card);
        let declares =
            !declared_targets(sink.state, &face).is_empty() || !declared_modes(sink.state, &face).is_empty();
        if !declares || !run.targets.is_empty() || !run.modes.is_empty() {
            run.cast_chosen = Some(true);
            return true;
        }
        run.repeat = Some(RepeatRecord::default());
    }
    if !ask_repeat_choices(sink, run, &card, step) {
        return false;
    }
    let chosen = run.repeat.take();
    run.cast_chosen = Some(true);
    let Some(chosen) = chosen else {
        return true;
    };
    run.targets = chosen.targets;
    run.modes = chosen.modes;
    // R174: the cast's choices were made against the board as its caster answered.
    run.exits_from = Some(exit_mark(sink.state));
    // A fused card's Cry splits the choices by its ingredients' declarations (R90, R102).
    let card = live_or(sink.state, &card);
    if let Some(slices) = slices_for(
        sink.state,
        run.player,
        &card,
        run.cost_paid,
        &run.targets,
        &run.modes,
    ) {
        run.target_slices = Some(slices);
    }
    true
}

/// R453: a cast X card's X, the caster's first choice (R70, R81), a `number` prompt from MIN_CHOSEN_X
/// (R348) to their mana, stored on the instance for the script and R396 (a random cast's is set in
/// `cast_through_pipeline`). False while the prompt waits.
fn cast_x_chosen(
    sink: &mut EngineSink<'_>,
    run: &mut PlayRun,
    card: &CardInstance,
    step: PlayStepName,
) -> bool {
    if !chooses_x(sink.state, card) || card.x.is_some() {
        return true;
    }
    let most = MIN_CHOSEN_X.max(sink.state.players[run.player].mana.current);
    if let Some(cast_x) = run.cast_x {
        set_x(sink.state, &card.id, MIN_CHOSEN_X.max(most.min(cast_x)));
        return true;
    }
    if run.random == Some(true) || random_cast_of(sink.state, run.player).is_some() {
        set_x(sink.state, &card.id, most);
        return true;
    }
    let values: Vec<String> = (MIN_CHOSEN_X..=most).map(|value| value.to_string()).collect();
    run.awaiting = Some(Awaiting::CastX);
    let name = def_of(Some(&*sink.state), &card.def_id).name.clone();
    let opened = open_prompt(
        sink,
        OpenPromptArgs {
            player: run.player,
            kind: PromptKind::Number,
            aim: None,
            prompt: format!("{}: choose X", ask_label(step, &name, None)),
            options: values
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
                .collect(),
            min: None,
            max: None,
            budget: None,
            owner: None,
            resume: resume_for(run, step_index(step)),
        },
    );
    if opened.is_some() {
        return false;
    }
    run.awaiting = None;
    set_x(sink.state, &card.id, most);
    true
}

// Step 6 — Echo (§10.5 step 6, R30)

fn label_of(selection: &Selection, chooser: PlayerId) -> String {
    match selection {
        Selection::Instance { instance_id } => instance_id.clone(),
        Selection::Hero { player } => hero_option_label(*player, chooser),
        Selection::Zone { player, row, lane } => cell_option_label(*player, *row, *lane, chooser),
        Selection::Mode { option } => option.clone(),
        Selection::Craft { recipe } => recipe.name(),
        Selection::None => "nothing".to_string(),
    }
}

/// R81: play choices travel in the `play` action once, so a repeat asks again (§10.6), each prompt
/// carrying the run to `answer_play_prompt`. True once all are answered; one the board cannot
/// satisfy is skipped and fizzles (R90, §8's conventions).
fn ask_repeat_choices(
    sink: &mut EngineSink<'_>,
    run: &mut PlayRun,
    played: &CardInstance,
    step: PlayStepName,
) -> bool {
    if run.repeat.is_none() {
        return false;
    }
    // B5 E14, R399: a copier asks for the choices its copied text declares.
    let card = text_face_of(sink.state, played);
    // A declaration for some modes only (`forModes`, #24) waits on its mode, so modes are asked first.
    if targets_follow_modes(&declared_targets(sink.state, &card)) {
        return ask_repeat_modes(sink, run, &card, step) && ask_repeat_targets(sink, run, &card, step);
    }
    ask_repeat_targets(sink, run, &card, step) && ask_repeat_modes(sink, run, &card, step)
}

/// What a prompt the pipeline opens calls itself: an Echo repeat's picks, or a cast's (R70).
fn ask_label(step: PlayStepName, name: &str, run: Option<&PlayRun>) -> String {
    if step == PlayStepName::Echo {
        return format!("Echo: {name}");
    }
    // R449: a replacement is played, not cast, and asks its choices as a cast does.
    let replacement = run.is_some_and(|run| run.cast != Some(true) && run.replaced == Some(true));
    if replacement {
        format!("Play: {name}")
    } else {
        format!("Cast: {name}")
    }
}

/// R70, R452: a cast's own choices, once step 3 settled their face (R214) and before the card moves
/// (R90), so E1's announce names them. A play made its choices at step 1.
fn cast_choices_step(sink: &mut EngineSink<'_>, run: &mut PlayRun) {
    if run.cast != Some(true) {
        return;
    }
    cast_choices_made(sink, run, PlayStepName::CastChoices);
}

/// A declaration's options for an Echo repeat's or a cast's pick (R81, R70), narrowed to enemies when
/// the run targets enemies and one is legal (R452), or to friends for a beneficial aim (R656).
fn cast_target_options(
    state: &GameState,
    run: &PlayRun,
    card: &CardInstance,
    decl: &TargetDecl,
) -> Vec<Selection> {
    let options = legal_selections_for(state, run.player, card, decl);
    // R1850004: a cast for its caster's own side picks its targets there alone, or passes the
    // declaration; a pick from a hand, a pile or a zone is no target and stays as it is.
    if run.friends_only == Some(true) && decl.kind == PromptKind::Target {
        return crate::random_cast::friends_only(state, run.player, &options, |selection: &Selection| {
            selection.clone()
        });
    }
    if run.target_enemies != Some(true) {
        return options;
    }
    let required = decl.min.min(options.len() as i32);
    if target_aim(decl) == TargetAim::Help {
        prefer_friends(
            state,
            run.player,
            &options,
            |selection: &Selection| selection.clone(),
            required,
        )
    } else {
        prefer_enemies(
            state,
            run.player,
            &options,
            |selection: &Selection| selection.clone(),
            required,
        )
    }
}

/// The repeat's target declarations, each offered in turn; false while one is waiting (R81).
fn ask_repeat_targets(
    sink: &mut EngineSink<'_>,
    run: &mut PlayRun,
    card: &CardInstance,
    step: PlayStepName,
) -> bool {
    let name = def_of(Some(&*sink.state), &card.def_id).name.clone();
    let chosen_modes: Vec<String> = run
        .repeat
        .as_ref()
        .map(|repeat| repeat.modes.clone())
        .unwrap_or_default();
    let targets = active_target_decls(&declared_targets(sink.state, card), &chosen_modes);
    let start = run.repeat.as_ref().map_or(0, |repeat| repeat.decl_at);
    for (at, decl) in targets.iter().enumerate().skip(start) {
        if let Some(repeat) = run.repeat.as_mut() {
            repeat.decl_at = at + 1;
        }
        let options = cast_target_options(sink.state, run, card, decl);
        if options.is_empty() {
            continue;
        }
        // MD-B22, R946: an aimed cast takes the named legal pick for its first declaration — R1180005:
        // a hero named by its prompt key as much as a card by its id.
        if at == 0
            && run.cast == Some(true)
            && let Some(aimed) = run.aim_at.clone()
            && let Some(pick) = options
                .iter()
                .find(|option| crate::targeted::aims_at_pick(option, &aimed))
        {
            if let Some(repeat) = run.repeat.as_mut() {
                repeat.targets.push(pick.clone());
            }
            continue;
        }
        // R1200: while a Mayor acts a `target` repeat is drawn at random, and nobody is asked.
        if decl.kind == PromptKind::Target && crate::random_targets::targets_random(sink.state) {
            let picks = crate::random_targets::draw_picks(
                sink.state,
                &mut *sink.rng,
                run.player,
                &options,
                decl.min,
                decl.max,
                crate::targeting::target_aim(decl),
            );
            if let Some(repeat) = run.repeat.as_mut() {
                repeat.targets.extend(picks);
            }
            continue;
        }
        // R452: a random cast makes this pick itself, at random, and asks nobody.
        if run.random == Some(true) || random_cast_of(sink.state, run.player).is_some() {
            let picks = random_picks(sink.rng, &options, decl.min, decl.max);
            if let Some(repeat) = run.repeat.as_mut() {
                repeat.targets.extend(picks);
            }
            continue;
        }
        run.awaiting = Some(Awaiting::EchoTarget);
        let prompt = ask_label(step, &name, Some(&*run));
        let offered: Vec<PromptOption> = options
            .iter()
            .map(|selection| PromptOption {
                key: crate::prompts::option_key(selection),
                label: label_of(selection, run.player),
                selection: selection.clone(),
                cost: None,
                radiant: None,
            })
            .collect();
        let opened = open_prompt(
            sink,
            OpenPromptArgs {
                player: run.player,
                kind: decl.kind,
                aim: decl.aim,
                prompt,
                options: offered,
                min: Some(decl.min),
                max: Some(decl.max),
                budget: None,
                owner: None,
                resume: resume_for(run, step_index(step)),
            },
        );
        if opened.is_some() {
            return false;
        }
        run.awaiting = None;
    }
    true
}

/// The repeat's mode declarations, each offered in turn; false while one is waiting (R81).
fn ask_repeat_modes(
    sink: &mut EngineSink<'_>,
    run: &mut PlayRun,
    card: &CardInstance,
    step: PlayStepName,
) -> bool {
    let name = def_of(Some(&*sink.state), &card.def_id).name.clone();
    let modes = declared_modes(sink.state, card);
    let start = run.repeat.as_ref().map_or(0, |repeat| repeat.mode_at);
    for (at, decl) in modes.iter().enumerate().skip(start) {
        if let Some(repeat) = run.repeat.as_mut() {
            repeat.mode_at = at + 1;
        }
        // R831008: only the options the card's check allows now; with none the declaration is
        // skipped, as one the board cannot satisfy is (R90).
        let allowed: Vec<String> = decl
            .options
            .iter()
            .filter(|option| mode_option_allowed(sink.state, run.player, card, option))
            .cloned()
            .collect();
        if allowed.is_empty() {
            continue;
        }
        // R452: a random cast makes this choice itself, at random.
        if run.random == Some(true) || random_cast_of(sink.state, run.player).is_some() {
            let option = sink.rng.pick(&allowed).cloned();
            if let Some(option) = option
                && let Some(repeat) = run.repeat.as_mut()
            {
                repeat.modes.push(option);
            }
            continue;
        }
        run.awaiting = Some(Awaiting::EchoMode);
        let opened = open_prompt(
            sink,
            OpenPromptArgs {
                player: run.player,
                kind: decl.kind,
                aim: None,
                prompt: ask_label(step, &name, Some(&*run)),
                options: allowed
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
                    .collect(),
                min: None,
                max: None,
                budget: None,
                owner: None,
                resume: resume_for(run, step_index(step)),
            },
        );
        if opened.is_some() {
            return false;
        }
        run.awaiting = None;
    }
    true
}

/// §10.5 step 6: `state.echoQueue`'s repeats, one at a time (§6.1), resumed from the queue and the
/// record. Then §4.5's check (R59), so step 7's `cardResolved` (#60, #85) meets a board without the
/// units the card killed; here, not in step 7, as a Death hook that asks pauses this re-entrant step.
fn echo_step(sink: &mut EngineSink<'_>, run: &mut PlayRun) {
    resolve_echo_repeats(sink, run);
    if paused(sink) {
        return;
    }
    state_check(sink);
}

fn resolve_echo_repeats(sink: &mut EngineSink<'_>, run: &mut PlayRun) {
    let opening = still_resolving(sink.state, run);
    let Some(opening) = opening.filter(|_| run.fizzled != Some(true)) else {
        drop_echo_repeats(sink.state, &run.instance_id);
        return;
    };
    if !run.echo_queued {
        // Once per resolution: `queue_echo_repeats` consumes R30's grant (`echo.rs`).
        run.echo_queued = true;
        queue_play_echo(sink, run, &opening);
    }

    loop {
        let Some(card) = still_resolving(sink.state, run) else {
            drop_echo_repeats(sink.state, &run.instance_id);
            return;
        };

        if run.repeat.is_none() {
            if echo_repeats_owed(sink.state, &run.instance_id) <= 0 {
                return;
            }
            // §4.5: the state check runs after every whole script (R59), so the last resolution's deaths
            // and a hero at 0 settle before the repeat asks anything. A Death hook that asks pauses the
            // repeats here; they wait in state.
            state_check(sink);
            if paused(sink) {
                return;
            }
            if still_resolving(sink.state, run).is_none() {
                drop_echo_repeats(sink.state, &run.instance_id);
                return;
            }
            if !take_echo_repeat(sink.state, &run.instance_id) {
                return;
            }
            run.repeat = Some(RepeatRecord::default());
        }

        let card = live_or(sink.state, &card);
        if !ask_repeat_choices(sink, run, &card, PlayStepName::Echo) {
            return;
        }
        if !resolve_repeat(sink, run) {
            return;
        }
    }
}

/// One Echo repeat: step 5 again, whole, its place kept in `RESOLVE_PARTS` so a paused Combo draw
/// resumes at the next part; the record goes as the script starts, its tail parked by
/// `run_hook_resumable` (R113). False when a prompt (or the game's end) stopped it.
fn resolve_repeat(sink: &mut EngineSink<'_>, run: &mut PlayRun) -> bool {
    let Some(repeat) = run.repeat.as_mut() else {
        return true;
    };
    // R174: the stays the repeat's fresh picks were made on, taken once, as its answers are all in.
    if repeat.exits_from.is_none() {
        repeat.exits_from = Some(exit_mark(sink.state));
    }
    let start = repeat.part_at.unwrap_or(0);
    for (at, part) in RESOLVE_PARTS.iter().enumerate().skip(start) {
        if let Some(repeat) = run.repeat.as_mut() {
            repeat.part_at = Some(at + 1);
        }
        let Some(card) = still_resolving(sink.state, run) else {
            break;
        };
        match part {
            ResolvePart::Quickstriker => quickstriker_combo(sink, run, &card),
            ResolvePart::ComboDraw => combo_draw_step(sink, run),
            ResolvePart::Script => {
                let repeat = run.repeat.take().unwrap_or_default();
                // R1180005: an Echo repeat's fresh picks are targeted too.
                crate::targeted::note_spell_targets(sink.state, &run.instance_id, &repeat.targets);
                // B5 E14, R399: an Echo repeat of a copier runs its copied text again.
                let face = text_face_of(sink.state, &card);
                let data = run.mana_before.map(|mana| {
                    let mut data: IndexMap<String, Value> = IndexMap::new();
                    data.insert(MANA_BEFORE_PLAY_KEY.to_string(), json!(mana));
                    data
                });
                run_hook_resumable(
                    sink,
                    &face,
                    "cry",
                    HookResumableOptions {
                        controller: Some(run.player),
                        targets: Some(repeat.targets),
                        modes: Some(repeat.modes),
                        data,
                        exits_from: repeat.exits_from,
                    },
                );
            }
        }
        if paused(sink) {
            return false;
        }
    }
    run.repeat = None;
    true
}

// Step 7 — where a Spell lands (§10.5 step 7)

/// §10.5 step 7 (`echo.rs`'s, as a cast lands the same way, R70): the card lands and `cardResolved`
/// goes out after every Echo repeat (R17). R155: §5.1's return flag is written after the landing, as
/// "reached the graveyard" is a condition (a no-op for a permanent or a self-exiled Spell, #39).
/// R429, R766: so is the climb a price-keeping return (#31) carries, which the landing wiped.
fn finish_step(sink: &mut EngineSink<'_>, run: &mut PlayRun) {
    // E39, R410, R455 (Classic+ #14 Forever&): a Spell with the return comes back once it has landed.
    let resolving = find_instance(sink.state, &run.instance_id)
        .filter(|card| matches!(card.zone, Zone::Resolving { .. }));
    let landing = resolving.is_some();
    // R429, R766 (issue #572): the climb its own returns gave it, read before the landing wipes it
    // (R78); a discount or surcharge it was played at stays behind with the rest of its `costMod`.
    let played_at = resolving.map_or(0, crate::query::return_price_of);
    let arrived = arrived_during(sink.state, run);
    // R1180005: the enemies the Spell targeted, taken off it before the landing resets it (R78).
    let targeted = crate::targeted::take_spell_targets(sink.state, &run.instance_id);
    land_after_resolution(
        sink,
        &ResolvedCard {
            instance_id: run.instance_id.clone(),
            def_id: run.def_id.clone(),
            player: run.player,
            // Step 4's `cardPlayed` number: the mana step 2 charged, with R65's X and embiggen prices;
            // a cast carries 0 (R70).
            cost_paid: run.cost_paid,
            radiant: run.radiant.unwrap_or(false),
            // R174, R61: whether the card is still in play is asked of the stay step 4 put it on.
            placed_from: run.placed_from,
            arrived_during: Some(arrived),
            targeted,
            on_draw: cast_on_draw(run),
        },
    );
    if landing {
        return_after_resolving(sink, run);
    }
    flag_return_to_hand_at_end_of_turn(sink.state, &run.instance_id);
    note_return_price(sink.state, &run.instance_id, played_at);
    // R983: a punished play's hit, once it has resolved (`cardResolved` is out) — from the judge
    // wherever it is now, through §4.4 on the playing player's hero.
    for verdict in run.feng_shui.clone().unwrap_or_default() {
        if verdict.outcome != FengShuiOutcome::Negative {
            continue;
        }
        if sink.state.result.is_some() {
            break;
        }
        deal_damage(
            sink,
            DamageArgs {
                source: find_instance(sink.state, &verdict.source_id).cloned(),
                target: DamageTarget::Hero { player: run.player },
                amount: verdict.damage,
                flags: None,
            },
        );
    }
    // R1086: an Equip card fuses into its host once the play has resolved, not as a trigger.
    fuse_equip(sink, run);
}

/// R1086 (ME-EQUIP): fuse the Equip card on top into its host, which keeps its zone, damage,
/// exertion and arrival turn; the Equip card ceases to exist with no Death. If either left the field
/// first, nothing fuses.
fn fuse_equip(sink: &mut EngineSink<'_>, run: &PlayRun) {
    let Some(host_id) = run.equip_host.clone() else {
        return;
    };
    let top_id = run.instance_id.clone();
    // R174: each judged from its stay: the host from step 1's, the top card from step 4's.
    if run
        .exits_from
        .is_some_and(|from| left_field_after(sink.state, from, &host_id))
    {
        return;
    }
    if run
        .placed_from
        .is_some_and(|from| left_field_after(sink.state, from, &top_id))
    {
        return;
    }
    let (Some(host), Some(top)) = (
        find_instance(sink.state, &host_id).cloned(),
        find_instance(sink.state, &top_id).cloned(),
    ) else {
        return;
    };
    if !matches!(host.zone, Zone::Field { .. }) || !matches!(top.zone, Zone::Field { .. }) {
        return;
    }
    fuse(
        sink,
        FuseArgs {
            ingredients: vec![top],
            target: Some(host),
            ..FuseArgs::default()
        },
    );
}

/// E39, R410, R455: a resolved Spell with a `returnAfterResolve` enchantment, just landed, goes back
/// to hand from the graveyard or exile it went to, the hand cap burning it as always (§2.4). A
/// countered or discarded Spell never gets here; a fizzled cast resolved nothing.
fn return_after_resolving(sink: &mut EngineSink<'_>, run: &PlayRun) {
    if run.fizzled == Some(true) {
        return;
    }
    let Some(mut card) = snapshot(sink.state, &run.instance_id) else {
        return;
    };
    if !matches!(card.zone, Zone::Graveyard { .. } | Zone::Exile { .. }) {
        return;
    }
    if card_type_of(sink.state, &card) != CardType::Spell
        || !has_enchantment(&card, EnchantmentKind::ReturnAfterResolve)
    {
        return;
    }
    add_to_hand(sink, &mut card);
}

// The driver

type StepRun = fn(&mut EngineSink<'_>, &mut PlayRun);

struct Step {
    name: PlayStepName,
    run: StepRun,
    /// A step that is re-entered at itself, because it holds its own place in state.
    repeats: bool,
}

/// Step 1 runs before the table is driven (`validate_play`), so its entry does nothing.
fn validate_step(_sink: &mut EngineSink<'_>, _run: &mut PlayRun) {}

/// §10.5 step 8. A cast settles nothing of its own: it runs inside another effect, whose loop
/// takes its events, and §2.4's chain runs the state check a cast on draw is owed (R59).
fn settle_step(sink: &mut EngineSink<'_>, run: &mut PlayRun) {
    if run.cast != Some(true) {
        settle(sink, SettleOptions::default());
    }
}

/// Steps 2 to 8 of §10.5; step 1 runs before any of them and cannot pause.
const STEP_TABLE: &[Step] = &[
    Step {
        name: PlayStepName::Validate,
        run: validate_step,
        repeats: false,
    },
    Step {
        name: PlayStepName::Pay,
        run: pay_step,
        repeats: false,
    },
    Step {
        name: PlayStepName::GiftedHook,
        run: gifted_hook_step,
        repeats: true,
    },
    Step {
        name: PlayStepName::CastChoices,
        run: cast_choices_step,
        repeats: true,
    },
    Step {
        name: PlayStepName::Announce,
        run: announce_step,
        repeats: true,
    },
    Step {
        name: PlayStepName::Place,
        run: place_step,
        repeats: true,
    },
    Step {
        name: PlayStepName::Resolve,
        run: resolve_step,
        repeats: true,
    },
    Step {
        name: PlayStepName::Echo,
        run: echo_step,
        repeats: true,
    },
    Step {
        name: PlayStepName::Finish,
        run: finish_step,
        repeats: false,
    },
    Step {
        name: PlayStepName::Settle,
        run: settle_step,
        repeats: false,
    },
];

/// Run the steps from `run.at` on, and stop the moment one leaves a prompt open. The rest is owed to
/// `state.work` only when a step pauses, never before (R117): step 4's nested `settle` drains it, so a
/// pipeline parked in advance would run its own steps 5 to 8 twice (R1: a Cry fires once). That is also
/// R113's order: `work.rs` parks at `state.workCursor`, so a Cry's parked tail resumes before steps 6 to
/// 8. True when the pipeline is finished with (a game that ended under it included).
fn drive(sink: &mut EngineSink<'_>, run: &mut PlayRun) -> bool {
    // R452: a random or enemy-targeting cast is in force each time its steps are driven.
    let mode = cast_mode_of(run);
    let finished = with_cast_mode(sink, mode, |sink| drive_steps(sink, run));
    // R58: a cast-on-draw cast has resolved, so the draw that cast it is complete.
    if finished && let Some(drawn_as) = &run.drawn_as {
        release_draw(sink.state, drawn_as);
    }
    finished
}

/// R452: the mode a run's steps run under, or null for a play and an ordinary cast.
fn cast_mode_of(run: &PlayRun) -> Option<CastMode> {
    if run.cast != Some(true)
        || (run.random != Some(true) && run.target_enemies != Some(true) && run.friends_only != Some(true))
    {
        return None;
    }
    Some(CastMode {
        instance_id: run.instance_id.clone(),
        player: run.player,
        random: run.random == Some(true),
        target_enemies: run.target_enemies == Some(true),
        casts: 1,
        friends_only: run.friends_only,
    })
}

/// `drive`'s loop over the steps from `run.at`; true when the pipeline is finished with.
fn drive_steps(sink: &mut EngineSink<'_>, run: &mut PlayRun) -> bool {
    for (at, step) in STEP_TABLE.iter().enumerate().skip(run.at) {
        // Where a pause would pick up: a `repeats` step is re-entered at itself, its cursor read from the
        // record as the pause is parked, after the step moved it.
        let next = if step.repeats { at } else { at + 1 };
        // A lost card is not played (R226), and a countered play (R448) has nothing after its announce:
        // only step 8 settles what was done.
        if (run.lost == Some(true) || run.countered == Some(true)) && step.name != PlayStepName::Settle {
            continue;
        }
        (step.run)(sink, run);

        if sink.state.result.is_some() {
            return true;
        }
        if sink.state.pending.is_some() {
            // Nothing to owe after the last step, nor for a prompt this pipeline opened: its `resume`
            // already carries the record (§10.5 step 6's fresh picks), so a work item would copy it.
            if run.awaiting.is_none() && next < STEP_TABLE.len() {
                push_work(sink, resume_for(run, next), Some(run.player));
            }
            return false;
        }
    }

    true
}

/// File an answered selection in the bucket the pause was waiting on (§10.5 step 6).
fn file_selection(run: &mut PlayRun, selection: &[Selection]) {
    let awaiting = run.awaiting.take();
    if awaiting == Some(Awaiting::CastX) {
        // R453: the X a cast's caster chose, applied to the card as the step goes on (`cast_x_chosen`).
        let picked = selection.iter().find_map(|pick| match pick {
            Selection::Mode { option } => Some(option),
            _ => None,
        });
        if let Some(value) = picked.and_then(|option| parse_int(option)) {
            run.cast_x = Some(value);
        }
        return;
    }
    let Some(repeat) = run.repeat.as_mut() else {
        return;
    };
    if awaiting == Some(Awaiting::EchoTarget) {
        repeat.targets.extend(selection.iter().cloned());
    }
    if awaiting == Some(Awaiting::EchoMode) {
        repeat
            .modes
            .extend(selection.iter().filter_map(|pick| match pick {
                Selection::Mode { option } => Some(option.clone()),
                _ => None,
            }));
    }
}

/// The selection an answered prompt brought back, wherever the continuation carries it (§10.6).
fn selection_in(data: &IndexMap<String, Value>) -> Option<Vec<Selection>> {
    if let Some(parked) = paused_of(data)
        && !parked.targets.is_empty()
    {
        return Some(parked.targets);
    }
    for key in ["selection", "targets"] {
        if let Some(Value::Array(raw)) = data.get(key)
            && !raw.is_empty()
        {
            return serde_json::from_value(Value::Array(raw.clone())).ok();
        }
    }
    None
}

/// `work.rs`'s handler for the `"play"` sequence: the owed pipeline, continued where it stopped. An
/// answer to a prompt the pipeline opened itself does not come back this way: it goes to
/// `answer_play_prompt`, which `prompts::answer_prompt` hands it to (R122).
pub fn run_owed_play(sink: &mut EngineSink<'_>, item: &WorkItem) {
    let Some(mut run) = run_of(&item.resume) else {
        return;
    };

    // One continuation per play: whichever of them runs first drops the others, so a play that is
    // both owed and named by an answered prompt does not run its tail twice.
    let item_id = item.id.clone();
    let instance_id = run.instance_id.clone();
    drop_work(sink.state, |other: &WorkItem| {
        other.id != item_id
            && is_play_resume(&other.resume)
            && run_of(&other.resume).is_some_and(|owed| owed.instance_id == instance_id)
    });

    if let Some(selection) = selection_in(&item.resume.data) {
        file_selection(&mut run, &selection);
    }
    drive(sink, &mut run);
}

/// R70: a cast is a play, "free … with cost paid 0", so it runs this pipeline from step 3 (the effect
/// chose the card; step 2 pays nothing); the rest is a play's, the Echo gained at step 4 (R178)
/// included. A step that asks parks the rest as an ordinary `"play"` item as it pauses (R113, R117).
/// Called by `resolve::cast_card`.
pub fn cast_through_pipeline(sink: &mut EngineSink<'_>, instance: &CardInstance, options: CastOptions) {
    // A cast card may be in no pile yet (drawn, §2.4, or made, #95) and the pipeline finds it by id,
    // so it waits in the resolving zone from the start (§10.5 step 4, R98).
    let player = instance.controller;
    // R2533000: an effect that casts a card out of a seat's hand casts none that seat may not play;
    // the card stays where it is and the cast does nothing.
    if let Some(Zone::Hand { player: holder }) =
        find_instance(sink.state, &instance.id).map(|held| held.zone.clone())
        && crate::state::barred_from_play(sink.state, holder, instance)
    {
        return;
    }
    let mut card = instance.clone();
    let from_graveyard =
        find_instance(sink.state, &card.id).is_some_and(|held| matches!(held.zone, Zone::Graveyard { .. }));
    remove_from_any_zone(sink.state, &mut card);
    // R155: a card that leaves the graveyard has spent the landing §5.1's end-of-turn return belongs to.
    if from_graveyard {
        card.return_to_hand_at_end_of_turn = None;
    }
    card.zone = Zone::Resolving { player };
    sink.state.players[player].resolving.push(card.clone());
    // B5 E14, R546: a copier cast resolves the Spell that is last as the cast begins.
    fix_copied_text(sink.state, &card.id, None);
    let mut card = live_or(sink.state, &card);

    // R452: a cast made while a random cast of its caster's resolves is random too, and counts in that
    // cast's chain; a card that targets enemies (E39's enchantment) is cast so, whoever casts it.
    let random = options.random == Some(true) || random_cast_of(sink.state, player).is_some();
    let target_enemies =
        options.target_enemies == Some(true) || has_enchantment(&card, EnchantmentKind::TargetEnemies);
    if random {
        count_chain_cast(sink.state, player);
    }
    // R452, R453: a random cast's X is the caster's current mana, at least MIN_CHOSEN_X (R348); any
    // other cast's caster chooses it as the cast's first choice (`cast_x_chosen`).
    if random && chooses_x(sink.state, &card) && card.x.is_none() {
        let x = MIN_CHOSEN_X.max(sink.state.players[player].mana.current);
        set_x(sink.state, &card.id, x);
        card.x = Some(x);
    }
    // R453: "then exile it" (Classic #56, `afterward`'s one value): step 7 exiles it (R178's mark).
    if options.afterward.is_some()
        && let Some(resolving) = find_instance_mut(sink.state, &card.id)
    {
        exile_on_landing(resolving);
    }

    let targets: Vec<Selection> = options.targets.unwrap_or_default();
    let modes: Vec<String> = options.modes.unwrap_or_default();
    let drawn_as: Option<String> = options
        .data
        .as_ref()
        .and_then(|data| data.get(CAST_ON_DRAW_KEY))
        .and_then(Value::as_str)
        .map(str::to_string);

    let mut run = blank_run(card.id.clone(), card.def_id.clone(), player);
    run.drawn_as = drawn_as;
    run.cost_paid = 0;
    run.zone = None;
    run.target_slices = slices_for(sink.state, player, &card, 0, &targets, &modes);
    run.targets = targets;
    run.modes = modes;
    run.tributes = Vec::new();
    run.at = step_index(PlayStepName::GiftedHook);
    run.hook_at = 0;
    run.resolve_at = 0;
    run.echo_queued = false;
    run.repeat = None;
    run.awaiting = None;
    run.cast = Some(true);
    run.exits_from = Some(exit_mark(sink.state));
    let begins = play_begins(sink.state, player);
    run.standing = Some(begins.standing);
    run.standing_from = Some(begins.standing_from);
    run.mods_before = Some(begins.mods_before);
    if random {
        run.random = Some(true);
    }
    if target_enemies {
        run.target_enemies = Some(true);
    }
    if options.friends_only == Some(true) {
        run.friends_only = Some(true);
    }
    // MD-B22, R946: an aimed cast names its first pick.
    run.aim_at = options.aim_at.clone();
    run.mana_before = Some(sink.state.players[player].mana.current);
    // A cast starts past the pay step, so it owes no targeting cost.
    run.targeting_owed = 0;
    drive(sink, &mut run);
}

// Entry points

/// §10.5, all eight steps: validate the play, then run steps 2 to 8, pausing wherever a prompt
/// opens. Returns the refusal, or `Ok` once the play has run (or has owed itself the rest).
pub fn run_play_steps(
    sink: &mut EngineSink<'_>,
    player: PlayerId,
    action: &PlayAction,
) -> Result<(), EngineError> {
    let mut run = validate_play(sink, player, action)?;
    intercept_declared_targets(sink, &mut run);
    drive(sink, &mut run);
    Ok(())
}

/// B5 E5, E9, R450: the targeting point of a play's declared targets, at §10.5 step 1. An interceptor
/// in the targeted player's hand (Classic #33) takes the first `target` pick naming one of their
/// units when it is itself a legal pick (a `by: "spell"` one only for a Spell, R651). The picks'
/// costs, checked at step 1, are paid at step 2 whatever it did.
fn intercept_declared_targets(sink: &mut EngineSink<'_>, run: &mut PlayRun) {
    if run.targets.is_empty() {
        return;
    }
    let Some(card) = snapshot(sink.state, &run.instance_id) else {
        return;
    };
    // R214: the face whose declarations the picks answer.
    let face = resolving_face(sink.state, run.player, &card, run.cost_paid);
    let decls: Vec<Option<TargetDecl>> =
        targeting_decls_of(sink.state, run.player, &face, &run.targets, &run.modes, None);
    // R651: a `by: "spell"` interceptor (Classic #33 Joro) answers only a Spell's declared target.
    let source = card_type_of(sink.state, &card);
    let player = run.player;
    // `accepts` is handed the state, since the sink holds its borrow.
    let targeting = |index: usize| decls.get(index).is_some_and(Option::is_some);
    let accepts = |state: &GameState, interceptor: &CardInstance, index: usize| match decls.get(index) {
        Some(Some(decl)) => interceptor_fits_decl(state, player, &face, decl, interceptor),
        _ => false,
    };
    run.targets = intercept_targeting(
        sink,
        InterceptArgs {
            chooser: player,
            picks: run.targets.clone(),
            targeting: Some(&targeting),
            accepts: Some(&accepts),
            what: None,
            source: Some(source),
        },
    );
}

/// Answer a prompt this pipeline opened (§10.5 step 6's picks, a cast's choices) and drive on. Its
/// hook is the `"play"` sequence, which no script holds, so `prompts.rs`'s answerer hands it here and
/// no caller drops the rest of the play (R122, R113). Returns the refusal, or `Ok`.
pub fn answer_play_prompt(sink: &mut EngineSink<'_>, answer: &AnswerInput) -> Result<(), EngineError> {
    let Some(pending) = sink.state.pending.clone() else {
        return Err(EngineError::new("no prompt is open"));
    };

    why_answer_refused(&pending, answer)?;

    let Some(mut run) = run_of(&pending.resume) else {
        return Err(EngineError::new("that prompt is not a play's own"));
    };
    let picks = in_offered_order(&pending, &answer.selection);
    // B5 E5, R450: picks that cost more discards than the chooser holds are no answer.
    why_target_answer_refused(sink.state, &pending, &picks)?;

    close_prompt(sink);

    // The answered prompt is this run's, so any tail owed for it earlier would repeat this step.
    let instance_id = run.instance_id.clone();
    drop_work(sink.state, |item: &WorkItem| {
        is_play_resume(&item.resume)
            && run_of(&item.resume).is_some_and(|owed| owed.instance_id == instance_id)
    });
    // B5 E5, E9, R450: the targeting point of an Echo repeat's or a cast's fresh pick — a random
    // discard cost it pays first, and an interception.
    let targeted = target_answer(sink, &pending, &picks);
    continue_play_answer(sink, &mut run, &targeted);
    Ok(())
}

/// File an answered pick and drive the play on, finishing what the prompt interrupted (R113, R122).
fn continue_play_answer(sink: &mut EngineSink<'_>, run: &mut PlayRun, picks: &[Selection]) {
    file_selection(run, picks);
    // R113: taking the paused step up again resets the cursor, as `prompts::answer_prompt` does.
    begin_work_cascade(sink);
    drive(sink, run);
    // R122: the answering action finishes what the prompt interrupted (a cast's draw chain, §2.4)
    // before the resolution loop moves, as `prompts::answer_prompt` does, or traps would answer the
    // cast's events before the draw repeated into the card beneath.
    drain_work(sink);
}
