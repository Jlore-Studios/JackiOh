//! SPEC §10.5 "Playing a card": the eight steps, in order, as named steps that other code can run
//! and that a prompt can pause in the middle of (BUILD M3-T2/T3, R81, R90).
//!
//! The pipeline is one table of named steps (`PLAY_STEPS`) driven by `drive`, and every step reads
//! and writes the same plain record — a `PlayRun`, which is JSON. That is what makes the sequence
//! resumable: when a step leaves a prompt open, the rest of the pipeline is owed to `state.work` as
//! a `Resume` naming the step to pick up at plus that record (§9.3 "mid-action choices are state,
//! not callbacks"), and the resolution loop drains it after the answer. Nothing is ever held across
//! a prompt in a local variable or a closure, which is the bug `work.rs` exists to prevent.
//!
//! So a Cry that opens a prompt resumes into the *remaining* steps rather than restarting: the
//! answer re-enters the card's own step, `settle` then runs what the pipeline owed, and steps 6, 7
//! and 8 finish the play. Echo (R30) is the same machinery pointed at itself — step 6 is re-entrant,
//! takes one repeat at a time off `state.echoQueue` and re-runs step 5 with *fresh* prompts, so a
//! prompt inside one repeat leaves the rest waiting in state (§6.1 "Echo X").
//!
//! Three steps keep their own cursor inside the record, so a pause inside one continues where it
//! stopped instead of at the top: step 3 over the `onPlayHook` holders, step 5 over its named parts,
//! step 6 over the echo queue and the repeat's fresh picks.
//!
//! Patch v0.2.0 adds a step between 3 and 4, the announce (B5 E1, R448): the paid-for card moves into
//! its player's resolving zone, `cardAnnounced` goes out, and a window runs in which Counters answer
//! it — traps first, then the other triggers the announce woke (§10.3). A countered play stops there:
//! step 4 never places it, so it is never counted, and step 8 settles. Step 1 is the targeting point
//! of the play's declared targets (B5 E5, E9, R450): an interception (Classic #33) moves a pick, and
//! step 2 pays a costly target's discards, at random (Classic #89, R682). Step 3 may replace the card
//! being played (Classic #23 Devil's Pact, R449), and step 4 counts what the play leaves for E4 (R451).
//!
//! What is *not* here: which choices are legal (that is `play_choices.rs`, R90), what a cost is
//! (`mana.rs`, R65), what a card's text does (the card's script, which is a pure builder the engine
//! applies — CLAUDE.md rule 5) and R43's power mechanics (`subsystems/hero_power.rs`).
//!
//! Port of `packages/engine/src/playSteps.ts`. TS registered three entry points at import time
//! (`registerWorkHandler`, `registerCastDriver`, `registerPromptAnswerer`); Rust's callers name them
//! directly (SURFACE §6.6): `work.rs`'s `"play"` arm calls `run_owed_play`, `resolve::cast_card` calls
//! `cast_through_pipeline`, and `prompts.rs`'s `"play"` answerer calls `answer_play_prompt`.

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};

use crate::animated::animate_on_entry;
use crate::announce::{begin_announce, end_announce, is_announce_live};
use crate::catalog::def_of;
use crate::config::{MIN_CHOSEN_X, QUICKSTRIKER_COMBO_MULTIPLE};
use crate::cost_rules::why_play_banned;
use crate::damage::{DamageArgs, DamageTarget, deal_damage};
use crate::draw::{add_to_hand, draw};
use crate::draw_complete::{CAST_ON_DRAW_KEY, release_draw};
use crate::echo::{
    EXILE_ON_LANDING, ResolvedCard, drop_echo_repeats, echo_repeats_owed, exile_on_landing,
    land_after_resolution, queue_echo_repeats, take_echo_repeat,
};
use crate::faces::card_type_of;
use crate::graveyard_play::{in_own_graveyard, mana_due, spend_plague_tokens, why_graveyard_play_refused};
use crate::mana::{cost_rules_spent_by, is_x_cost, mana_event, modifier_is_live, play_cost, spend_mana};
use crate::modifiers::remove_modifier;
use crate::play_choices::{
    DECLARATION_SLICES_KEY, active_target_decls, chooses_x, declaration_slices, declared_modes,
    declared_targets, default_zone_for, in_declared_order, interceptor_fits_decl, legal_selections_for,
    play_made_radiant, play_uses, plays_on_stack, resolving_face, targeting_decls_of,
    targeting_discards_required, targets_follow_modes, why_choices_refused,
};
use crate::play_counts::record_play;
use crate::prompts::{
    AnswerInput, OpenPromptArgs, RunHookOptions, cell_option_label, close_prompt, hero_option_label,
    in_offered_order, open_prompt, run_hook_resumable, why_answer_refused,
};
use crate::random_cast::{count_chain_cast, prefer_enemies, prefer_friends, random_cast_of, random_picks, with_cast_mode};
use crate::resolve::{CastOptions, HookName, MANA_BEFORE_PLAY_KEY, flag_return_to_hand_at_end_of_turn};
use crate::script::{EngineSink, FlagOrCount, Script, StaticFlags, empty_script};
use crate::state::{
    AnnounceRecord, CardInstance, CastMode, EngineError, GameState, ModifierExpiry, ModifierKind, PlayRecord,
    PromptOption, QueuedTrigger, Resume, WorkItem, find_instance, find_instance_mut, new_instance,
};
use crate::state_check::{sacrifice_together, state_check};
use crate::stays::{exit_mark, left_field_after};
use crate::subsystems::copied_text::{copied_text_of, copies_text, fix_copied_text, text_face_of};
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
    CardCost, CardType, Enchantment, GameEvent, PLAYER_IDS, PlagueSpend, PlayedFrom, PlayerId, PromptKind, Row,
    Selection, TargetAim, TargetDecl, Zone, opponent_of,
};
use crate::work::{begin_work_cascade, drain_work, drop_work, paused, paused_of, push_work};
use crate::zones::{
    PlaceOptions, ZoneSlot, card_at, cease_to_exist, first_free_zone, fresh_face_down_id, is_open,
    lands_face_down, place_on_field, release_zone, remove_from_any_zone, reserve_zone, slots_of,
};

/// TS `Extract<ActionBody, { type: "play" }>`. `play_choices.rs` names the same type (TS declared it in
/// both modules); this re-export keeps the two one type, so the crate root's globs agree.
pub use crate::play_choices::PlayAction;

// ---------------------------------------------------------------------------
// Private copies of small helpers other modules own (fullsend rule 5)
// ---------------------------------------------------------------------------

/// TS `scripts.flagsOf(instance)`: the running face's static flags — the radiant text's once the
/// instance is Radiant (§5.2), none at all for a Vanilla instance (§6.3 Vanilla, R115).
fn flags_of_card(state: &GameState, card: &CardInstance) -> StaticFlags {
    let script: Script = if card.vanilla {
        empty_script()
    } else {
        let entry = crate::scripts::script_of(state, &card.def_id);
        if card.radiant { entry.radiant } else { entry.base }
    };
    script.flags()
}

/// TS `enchantments.hasEnchantment(card, "returnAfterResolve")`.
fn has_return_after_resolve(card: &CardInstance) -> bool {
    card.enchantments
        .iter()
        .flatten()
        .any(|entry| matches!(entry, Enchantment::ReturnAfterResolve { .. }))
}

/// TS `enchantments.hasEnchantment(card, "targetEnemies")`.
fn has_target_enemies(card: &CardInstance) -> bool {
    card.enchantments
        .iter()
        .flatten()
        .any(|entry| matches!(entry, Enchantment::TargetEnemies))
}

/// TS `timesPlayed.countPlay(card)`. R429, §10.5 step 4: one more play of this card, when its script
/// counts them (`timesPlayed.timesPlayedOf`: the plays so far, 0 when absent or not positive).
fn count_play(state: &mut GameState, instance_id: &str) {
    let Some(card) = find_instance(state, instance_id) else {
        return;
    };
    if flags_of_card(state, card).counts_plays != Some(true) {
        return;
    }
    let so_far = card.times_played.filter(|count| *count > 0).unwrap_or(0);
    if let Some(live) = find_instance_mut(state, instance_id) {
        live.times_played = Some(so_far + 1);
    }
}

/// The card as it stands now, by id. TS handed the live object around; Rust reads it again at each
/// point TS read the object, and hands a copy to calls that take the card beside a `&mut` state.
fn snapshot(state: &GameState, instance_id: &str) -> Option<CardInstance> {
    find_instance(state, instance_id).cloned()
}

/// The card as it stands now, or `card` itself when it is no longer anywhere (TS's object kept its
/// last fields after it left every pile).
fn live(state: &GameState, card: &CardInstance) -> CardInstance {
    snapshot(state, &card.id).unwrap_or_else(|| card.clone())
}

/// TS `card.x = value` on the live instance.
fn set_x(state: &mut GameState, instance_id: &str, value: i32) {
    if let Some(card) = find_instance_mut(state, instance_id) {
        card.x = Some(value);
    }
}

/// `Number.parseInt(text, 10)` as `fileSelection` uses it: leading whitespace, an optional sign, then
/// the longest run of digits; `None` (TS `NaN`) when there is none.
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

// ---------------------------------------------------------------------------
// The steps and the run record
// ---------------------------------------------------------------------------

/// The eight steps of §10.5, in the order that section lists them, and E1's announce between 3 and 4.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum PlayStepName {
    Validate,
    Pay,
    GiftedHook,
    // R70, R452: a cast's choices, made once step 3 has settled the face they answer (R214) and before
    // the card moves — so an announce that follows (E1, between steps 3 and 4) names them. A play made
    // its choices at step 1, and this step does nothing for it.
    CastChoices,
    Announce,
    Place,
    Resolve,
    Echo,
    Finish,
    Settle,
}

impl PlayStepName {
    /// The step's name as TS wrote it (`Resume.step`).
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

/// The eight steps of §10.5, in the order that section lists them, and E1's announce between 3 and 4.
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

/// `PLAY_STEPS.indexOf(step)`.
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
pub const RESOLVE_PARTS: &[ResolvePart] = &[ResolvePart::Quickstriker, ResolvePart::ComboDraw, ResolvePart::Script];

/// `resume.hook` for an owed play pipeline: the sequence name `work.rs` documents (`"play"`).
pub const PLAY_WORK_KIND: &str = "play";

/// Where the run record sits inside `resume.data`, so the rest of `data` stays the card's own.
const RUN_KEY: &str = "__play";

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

/// `PlayRun.repeat` (TS `NonNullable<PlayRun["repeat"]>`): the repeat being re-resolved, or a cast's
/// own choices being asked, with the fresh answers collected so far.
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

/// TS `x?: T | null`: absent is `None`, `null` is `Some(None)` (serde would read both as `None`).
fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

/// A play part-way through its steps. All JSON: it is stored inside an owed `WorkItem`'s
/// `resume.data` and inside the `resume` of a prompt the pipeline opened itself, so a paused play
/// survives `JSON.parse(JSON.stringify(state))` and a replay resumes it exactly (§9.3, §10.1).
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
    /// B5 E5, R450, R682: the targeting cost step 1 checked, owed whatever the step-1 interception did
    /// (Classic #33: the targeting happened). Step 2 pays this count at random — never recomputed off
    /// the redirected picks, whose costs nobody owes.
    #[serde(default)]
    pub targeting_owed: i32,
    /// Index into `PLAY_STEPS` of the step to run next.
    pub at: usize,
    /// Step 3's cursor: how many `onPlayHook` holders have run.
    #[serde(default)]
    pub hook_at: usize,
    /// Step 3's holders, by id, in R68's order as the step began. The cursor indexes this list, never a
    /// fresh read of the board, which a hook's own answer can reshape (a hook that bounced its own card
    /// dropped the next one, R113).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hook_ids: Option<Vec<String>>,
    /// Step 5's cursor into `RESOLVE_PARTS`.
    #[serde(default)]
    pub resolve_at: usize,
    /// Step 6: whether this play has worked out how many repeats it owes yet.
    #[serde(default)]
    pub echo_queued: bool,
    /// Step 6: the repeat being re-resolved, with the fresh answers collected so far, and `partAt`,
    /// its cursor into `RESOLVE_PARTS` once the answers are in — a repeat is step 5 again, granted
    /// Combo parts included, and #78's Combo draw can pause it before its script runs. `exitsFrom` is
    /// the field's departures once the answers were in (R174): the repeat's script is aimed at the
    /// stays its fresh picks were made on, so a target the repeat's own Combo draw killed is gone for
    /// it, Reborn body or not, as a play's declared target is gone for its Cry (R81, R83).
    #[serde(default)]
    pub repeat: Option<RepeatRecord>,
    /// Set while a prompt this pipeline opened is waiting; says which bucket the answer fills.
    #[serde(default)]
    pub awaiting: Option<Awaiting>,
    /// R70, R81: a cast's own choices are made. A play carries its targets and modes in the action, and
    /// a cast has none, so step 4 asks the caster for them before it places the card (R90: a play's
    /// choices are made with the card still in hand), as prompts, into `repeat` (the same record an
    /// Echo repeat's fresh picks fill), and sets this once they are in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cast_chosen: Option<bool>,
    /// R70: a cast rather than a play from hand — the same steps, entered at step 3 for 0, placed by
    /// R64 at step 4 wherever the card is, and leaving the resolution loop to the effect that cast it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cast: Option<bool>,
    /// The face the card was played with, set at step 4 for `cardResolved` (R34, R57).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
    /// R214: whether Gifted Program makes this play Radiant, as step 1 read it to know the face the
    /// play's choices answer. Step 3 applies this answer rather than asking the board again, which a
    /// Tribute's Death at step 2 can have changed (#22's copies of the Gifted Program it ate). A cast has
    /// no step 1 (R70), so its step 3 reads the board.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gifted: Option<bool>,
    /// Step 4 has placed the card and announced the play; what is left of it is its resolution loop,
    /// which a trap's question can pause (§10.3, R17), so the step is re-entered at that loop.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placed: Option<bool>,
    /// R90, R102: how many of `targets` each declaration took when step 1 read the play, which a fused
    /// card's Cry splits its ingredients' choices by (`play_choices::DECLARATION_SLICES_KEY`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_slices: Option<Vec<usize>>,
    /// R174: the field's departures when the play's choices were checked — at step 1 for a play, and
    /// for a cast once its caster has made them (R70) — so a declared target a Tribute (step 2) or a
    /// trap answering the play (step 4) took off the field is gone for step 5, even back through Reborn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exits_from: Option<u32>,
    /// R174: the field's departures when step 3 read its holders (`hookIds`), whose stays it runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hooks_from: Option<u32>,
    /// R174: the field's departures once step 4 had put the card on the field. The play follows that
    /// stay and no other: a card that has left it since — a trap at step 4 killed it, its own Cry did —
    /// is no longer the card being played even when Reborn has put a new body in its zone (R83), so
    /// that body has no Cry to resolve (R1, R118) and is not in play for step 7 (R61).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placed_from: Option<u32>,
    /// R119: every card acting on the field as the play began — at step 1 for a play, as the cast began
    /// for a cast (R70) — by id, with the field's departures then (`standingFrom`). A permanent that
    /// arrives on the field after that, whatever puts it there — a tributed unit's Death at step 2
    /// (#22's copies, R210), the Cry recruiting it (#98), summoning it (#95) or bringing a body back
    /// through Reborn, a trap answering the play summoning it — does not answer the play, as the played
    /// card itself does not: not its `cardPlayed` and `summoned` at step 4, not step 5's granted Combo
    /// (#38) on the first resolution or an Echo repeat, and not its `cardResolved` at step 7
    /// (`arrivedDuring`). Nor does a card that lay dormant under a Stack pile as the play began and
    /// resumed as its top while the play resolved: it registered nothing then (§3.2, R153).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standing: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standing_from: Option<u32>,
    /// R119: the playing player's modifiers as the play began, by id. A granted Combo that is a
    /// modifier (#78's `comboDraw`, a `quickstrikerDamage`) answers the play only when it was already
    /// in place then, so one the play itself installed — /fullsend's own rider, met again by the Echo
    /// repeat of the same play (§10.5 step 6) — does not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mods_before: Option<Vec<String>>,
    /// R360: the Tribute step 2 paid took at least one of the opponent's units (#55's "if opposing
    /// Units are used"), read as it was paid, before those units died. Step 4 reads it against the
    /// face the card is placed with, since only #55's base face summons itself for the opponent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enemy_tributed: Option<bool>,
    /// R226, §10.5 step 4, §10.1: the card left its owner's hand before step 4 could move it — a
    /// Tribute's Death at step 2, or an `onPlayHook` at step 3, had it discarded — or, for a cast, left
    /// the resolving zone it waits in (R70) — so it is not played: no placement, no `cardPlayed`, no
    /// resolution. What steps 2 and 3 did stands, and step 8 settles it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lost: Option<bool>,
    // ---- play pipeline B (E11, E12; R452–R454) ----
    /// E11, R454: the card is played from its player's graveyard under a permission (`graveyard_play.rs`)
    /// rather than from the hand. Absent for a play from hand and for a cast.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<PlaySource>,
    /// E11, R454: the Plague Counters paying part of the price (`costPaid` is the whole price).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plague: Option<PlagueSpend>,
    /// E12, R452: a random cast — every choice its caster would make is made at random.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub random: Option<bool>,
    /// E12, E39, R452: its target picks narrow to enemies when one is legal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_enemies: Option<bool>,
    /// E12, R453: a cast Field Spell, Trap or Field Trap that found no zone. It still counts as played
    /// (R70, R138), nothing of its text resolves, and step 7 puts it in its owner's graveyard.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fizzled: Option<bool>,
    /// The player's current mana as the play began — at step 1, before step 2 pays — or as a cast began,
    /// which the card's Cry reads as `ctx.manaBeforePlay` (Classic #22 Mid Runner).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mana_before: Option<i32>,
    /// R453: the X its caster chose for a cast X card (the `number` prompt's answer), until it is set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cast_x: Option<i32>,
    // ---- v0.2.0 play pipeline A (E1, E5, Devil's Pact) ----
    /// B5 E1, R448: the announce has gone out and the card waits in the resolving zone; what is left of
    /// the step is its window, which a response's question can pause, so the step is re-entered there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub announced: Option<bool>,
    /// R448: a Counter cancelled the play in its announce window: nothing after it happens but step 8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub countered: Option<bool>,
    /// R449: step 3 replaced the card played by a new card (Classic #23 Devil's Pact). The new card makes
    /// its choices as a cast does (R70), before the announce, and takes its zone at step 4 by R64.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaced: Option<bool>,
    /// R58: the id the card was drawn under, for a cast-on-draw cast. The draw is complete once this
    /// cast has resolved, and its held `drawn` is released then (`draw_complete.rs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drawn_as: Option<String>,
    // ---- B5 E14 (Classic #57 Echo; R399, R546) ----
    /// The Spell text a copier's play resolves, fixed at step 1 as its choices are checked against it
    /// (null: nothing had been played), and written on the card as the announce moves it into the
    /// resolving zone (`subsystems::copied_text::fix_copied_text`). Absent for every other card.
    /// (TS `copied?: PlayRecord | null`: `None` absent, `Some(None)` null.)
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present")]
    pub copied: Option<Option<PlayRecord>>,
}

/// A run with every cursor at rest and every optional field absent, for `validate_play` and
/// `cast_through_pipeline` to fill (TS wrote each as one object literal).
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
        random: None,
        target_enemies: None,
        fizzled: None,
        mana_before: None,
        cast_x: None,
        announced: None,
        countered: None,
        replaced: None,
        drawn_as: None,
        copied: None,
    }
}

// ---------------------------------------------------------------------------
// The owed record
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Step 1 — validate (§10.5 step 1)
// ---------------------------------------------------------------------------

fn hand_card<'a>(state: &'a GameState, player: PlayerId, instance_id: &str) -> Option<&'a CardInstance> {
    state.players[player].hand.iter().find(|card| card.id == instance_id)
}

/// Where a play takes its card from (`playSourceOf`'s `from`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlayFrom {
    Hand,
    Graveyard,
}

/// E11, R454: where a play takes its card from — the player's hand, or their own graveyard, where a
/// permission may let them play it (`graveyard_play.rs`). Undefined when the card is in neither.
fn play_source_of(state: &GameState, player: PlayerId, instance_id: &str) -> Option<(CardInstance, PlayFrom)> {
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

/// R119: the board and the player's modifiers as a play or a cast begins, which it answers against
/// (TS `Pick<PlayRun, "standing" | "standingFrom" | "modsBefore">`).
struct PlayBegins {
    standing: Vec<String>,
    standing_from: u32,
    mods_before: Vec<String>,
}

/// §10.5 step 1: "legal zone, cost ≤ current mana after all modifiers, Tribute available, X or
/// embiggen chosen, targets legal — every choice the play carried is checked against what the card
/// declared and what the board allows (R90)". Every one of those checks is `play_choices.rs`'s, which
/// is why this asks once and restates nothing; what is left here is the cost, which `mana.rs` owns,
/// and the default zone a play that named none takes (R64's leftmost free zone).
///
/// X and embiggen are stamped on the instance before the cost is read, because that is where R65's
/// calculation looks for them. `reduce` clones the state and throws the clone away on a refusal, so
/// a refused play leaves nothing behind (§9.3).
pub fn validate_play(sink: &mut EngineSink<'_>, player: PlayerId, action: &PlayAction) -> Result<PlayRun, EngineError> {
    let Some((card, from)) = play_source_of(sink.state, player, &action.instance_id) else {
        return Err(EngineError::new(format!(
            "no card {} in {}'s hand",
            action.instance_id, player
        )));
    };
    // R454: Plague Counters pay only for a play from the graveyard a permission lets them pay for.
    if from == PlayFrom::Hand && action.plague.is_some() {
        return Err(EngineError::new(
            "only a play from your graveyard can spend Plague Counters",
        ));
    }

    why_choices_refused(sink.state, player, &card, action)?;

    let chooses = chooses_x(sink.state, &card);
    let embiggens = matches!(def_of(Some(&*sink.state), &card.def_id).cost, CardCost::Embiggen { .. });
    if let Some(stamped) = find_instance_mut(sink.state, &card.id) {
        if chooses {
            stamped.x = Some(action.x.unwrap_or(0));
        }
        if embiggens {
            stamped.embiggened = Some(action.embiggen == Some(true));
        }
    }
    let card = live(sink.state, &card);
    let state: &GameState = sink.state;

    // R65, R454, R455: the price of this play, the player's prices included wherever the card lies.
    let cost = play_cost(state, &card);
    why_play_banned(state, player, &card, cost)?;
    if from == PlayFrom::Graveyard {
        why_graveyard_play_refused(state, player, &card, cost, action.plague.as_ref())?;
    } else if cost > state.players[player].mana.current {
        return Err(EngineError::new(format!(
            "{} costs {cost}, more than your mana",
            def_of(Some(state), &card.def_id).name
        )));
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
    run.zone = zone;
    // B5 E5, R450, R682: read against the picks as checked, before the step-1 interception moves
    // any of them — a cost the targeting owes whatever answers it.
    run.targeting_owed = targeting_discards_required(state, player, &face, &targets, &modes, None);
    run.targets = targets;
    run.modes = modes;
    run.tributes = tributes;
    run.at = 1;
    run.gifted = Some(play_made_radiant(state, player, &card, cost));
    run.target_slices = slices_for(state, player, &card, cost, &run.targets, &run.modes);
    run.exits_from = Some(exit_mark(state));
    let begins = play_begins(state, player);
    run.standing = Some(begins.standing);
    run.standing_from = Some(begins.standing_from);
    run.mods_before = Some(begins.mods_before);
    run.mana_before = Some(state.players[player].mana.current);
    // B5 E14, R546: a copier's copy is fixed as its play is checked, the choices with it.
    if copies_text(state, &card) {
        run.copied = Some(copied_text_of(state, &card));
    }
    if from == PlayFrom::Graveyard {
        run.source = Some(PlaySource::Graveyard);
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

/// R90, R102: the split step 1 just checked, kept for a fused card, whose Cry hands each ingredient
/// its own slice (`subsystems/fuse.rs`) and must hand it the slice the play was checked with. The
/// face is the one step 5 will resolve (R214). Nothing is kept for any other card, which reads the
/// flat list itself (TS `{}`: `None`).
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

// ---------------------------------------------------------------------------
// Step 2 — pay (§10.5 step 2)
// ---------------------------------------------------------------------------

/// §6.3 Tribute: sacrifice the units this play named. How many they must be worth, whether an enemy
/// unit counts (#55) and that a Sheep Token counts 2 were all settled in step 1, so this is the
/// payment and nothing else — which is why it sacrifices what it was given rather than judging it
/// again. The set is one payment and dies together, its Death hooks in R68's order whatever order the
/// play listed it in (`state_check::sacrifice_together`), so `legalActions`, which offers each set once,
/// and `reduce`, which accepts any listing of it, mean the same play.
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

/// R360: where #55's base face lands when its Tribute took an opposing unit — "summon for your
/// opponent". The opponent's zone in the lane the player named when it is open, else the leftmost
/// open zone of their unit row, which is R15's placement for a card changing sides. With no open zone
/// there it is placed where the player named, on their own side, as a steal that finds no zone leaves
/// the card where it is (R15).
fn handed_over_zone(state: &GameState, run: &PlayRun, card: &CardInstance) -> Option<ZoneSlot> {
    if run.enemy_tributed != Some(true) {
        return None;
    }
    let zone = run.zone?;
    if zone.row != Row::Units {
        return None;
    }
    if flags_of_card(state, card).enemy_tribute_hands_over != Some(true) {
        return None;
    }
    let opponent = opponent_of(run.player);
    let same_lane = ZoneSlot {
        player: opponent,
        row: Row::Units,
        lane: zone.lane,
    };
    if is_open(state, &same_lane) {
        Some(same_lane)
    } else {
        first_free_zone(state, opponent, Row::Units)
    }
}

/// §10.5 step 2: "consume the next-spell discount if used". A one-shot discount (Lunar Eclipse) is
/// `{ until: "used" }`, so the play it applied to spends it; an X-cost card ignores discounts and so
/// spends none (R65), and Professor Curvature's is not a one-shot but a next-turn modifier (R48).
fn consume_used_discounts(sink: &mut EngineSink<'_>, run: &PlayRun, card: &CardInstance) {
    let card = live(sink.state, card);
    if is_x_cost(sink.state, &card) {
        return;
    }
    let type_ = card_type_of(sink.state, &card);
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
        // §2.2 names the Lunar Eclipse discount as both "consumed on use" AND expired at cleanup, so
        // it cannot be expressed by the expiry alone: `{ until: "used" }` survives cleanup (that is
        // what keeps #79 Twinspell's `echoNextSpell` alive, R30), while `{ until: "thisTurn" }` is
        // never consumed here. `oncePerTurn` is the flag for exactly that pair — the card keeps the
        // "this turn" expiry and this consumes it on the first matching play (§8 row 35).
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

fn pay_step(sink: &mut EngineSink<'_>, run: &mut PlayRun) {
    let Some(card) = snapshot(sink.state, &run.instance_id) else {
        return;
    };
    // R455: the "until used" price rules this play's price met, read before a Tribute can change the
    // board they are read against.
    let spent_rules = cost_rules_spent_by(sink.state, &card);

    // R454: Plague Counters pay their part of the price, and the mana the rest.
    let due = mana_due(run.cost_paid, run.plague.as_ref());
    spend_mana(&mut sink.state.players[run.player], due);
    let changed = mana_event(run.player, &sink.state.players[run.player]);
    sink.events.push(changed);
    if let Some(plague) = run.plague.clone()
        && let Some(holder) = snapshot(sink.state, &plague.from)
    {
        spend_plague_tokens(sink, &holder, plague.tokens);
    }
    // B5 E5, R450, R682: a targeting cost is part of the price, paid with it (Classic #89) — random
    // cards from the hand, drawn at pay time. Never the card being played or a hand card it picks.
    // The count is step 1's, owed whatever the interception did to the picks.
    if run.targeting_owed > 0 {
        let keep = play_uses(&card, &run.targets);
        pay_targeting_discards(sink, run.player, run.targeting_owed, &keep);
    }
    // R210: the zone step 1 accepted is the play's until step 4 puts the card in it. A Tribute is
    // paid here, and a tributed unit's Death — #3 radiant's summon, #22's copies, #86's steals — lands
    // cards by R64 and R15 in the very row the play is going to; held like a Reborn zone (R64), the
    // named zone is closed to them, so they take the next one and the played card is never left with
    // no zone at all.
    if let Some(zone) = run.zone
        && !run.tributes.is_empty()
    {
        reserve_zone(sink.state, &zone);
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

// ---------------------------------------------------------------------------
// Step 3 — the Gifted Program hook (§10.5 step 3)
// ---------------------------------------------------------------------------

/// §10.5 step 3: "the Gifted Program hook may set `radiant` now". #64 itself is a static flag the
/// engine reads first (`gifted_program_step`, R213), because step 1 has to know the face the play will
/// resolve with before it reads the play's choices (R214). Then every `onPlayHook` on the board runs
/// in R68's order, before the card is moved and before anything resolves, with the played card as its
/// selection and the cost paid in `data` — no Core card has one now, and the engine's fixtures keep
/// the hook honest. The cursor is advanced before each hook runs, so a hook that opens a prompt
/// continues with the hooks after it instead of running any of them twice.
fn gifted_hook_step(sink: &mut EngineSink<'_>, run: &mut PlayRun) {
    // R213: #64 Gifted Program's own rule, which step 1 has already read to know the face the play's
    // choices answer (R214), so the two cannot disagree. Once, before the hooks: the cursor is 0 only
    // on the first entry.
    if run.hook_ids.is_none() {
        // R449: a replacement comes first, so what step 3 makes Radiant is the card that is played.
        replace_played_card(sink, run);
        gifted_program_step(sink, run);
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
        // Each holder is read again as its turn comes: one an earlier hook took off the field, or out of
        // the zone that registers the hook, has nothing to run (R153, R174) — and one that has left the
        // field since the step began is not the holder it was, even back through Reborn: a new arrival
        // that did not stand there as the play reached step 3 (R83), as a hook queued before a death
        // does not fire for the body that came back.
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
            RunHookOptions {
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

/// §8 #64 Gifted Program: "the first card costing 1 or less you play each turn becomes Radiant as it
/// is played" (2 or less on its radiant face). The card is a static flag the engine reads here —
/// `play_choices::gifted_makes_radiant` counts the player's plays this turn (R213) — so step 1 can know
/// the face this play will resolve with before it checks the play's choices (R214), as a hook that
/// only ran now could not tell it. Setting the flag is §6.3's Make Radiant: in hand the card's stats
/// and text swap on the next read (§5.2, R74), and the event says so.
fn gifted_program_step(sink: &mut EngineSink<'_>, run: &mut PlayRun) {
    let Some(card) = snapshot(sink.state, &run.instance_id) else {
        return;
    };
    let gifted = match run.gifted {
        Some(gifted) => gifted,
        None => play_made_radiant(sink.state, run.player, &card, run.cost_paid),
    };
    if !gifted {
        return;
    }
    // R177: reported whether or not the card was Radiant already. A face-down trap stays hidden from
    // the other seat, whose stream keeps its events redacted but present (R97, R33), so a cue only for
    // a card that changed would tell that seat the trap's face in hand. Whether Gifted Program applies
    // is public — the cost paid and the Field Spell are — so the cue says nothing more than that.
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

/// Classic #23 Devil's Pact, R449: "this turn, each card you play is replaced by a Book of Flame". A live
/// `replacePlays` modifier of the playing player replaces the card played, here at §10.5 step 3, by a
/// new instance of its definition (Radiant per the modifier) that is played in its place: it waits in
/// the resolving zone, makes its own choices as a cast does (R70) at the announce, and takes its zone
/// by R64 at step 4. The old card ceases to exist (R35), in its hand, in the graveyard a play under a
/// permission takes it from (R454) or in the resolving zone a cast waited in, and the price paid was
/// the old card's (its mana, Tribute and targeting cost stay paid).
/// Several modifiers replace in turn, in creation order, each re-reading the card; a card that already
/// is the named card on the named face is not replaced by itself.
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
        cease_to_exist(sink.state, &old);
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
    }
}

// ---------------------------------------------------------------------------
// The announce (B5 E1, R448): between §10.5 steps 3 and 4
// ---------------------------------------------------------------------------

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

/// R448: the card leaves its player's hand (or graveyard, R454) for the resolving zone (a cast's, or a replacement's,
/// waits there already), its announce opens, and `cardAnnounced` goes out — what `cardPlayed` would
/// show: a card to be set face-down shows the other player only its zone (`view_for`). False when the
/// card is no longer where the play takes it from (R226): the play is lost.
fn announce_play(sink: &mut EngineSink<'_>, run: &mut PlayRun) -> bool {
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
        // B5 E14, R546: the copy step 1 checked the choices against is the one the card resolves.
        // (Written on the card before it joins the resolving zone; the copy reads only the card and
        // `state.lastSpell`, so the order TS used, push then fix, changes nothing.)
        if let Some(copied) = &run.copied {
            fix_copied_text(sink.state, &mut card, Some(copied.clone()));
        }
        sink.state.players[run.player].resolving.push(card.clone());
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

/// B5 E1, R448: the announce window. Traps see every event so far first, the announce among them, and
/// fire at once as responses (`triggers::dispatch_pending`); then every other trigger the announce woke
/// resolves, one at a time in queue order (R68), each followed by the state check (R59). Triggers that
/// answer anything else wait for the step-4 loop, or for a cast the loop of the effect that cast it
/// (R70, R117), exactly as they did before the announce existed. A Counter among them cancels the play:
/// the triggers still to come on it find no card (`is_announce_live`) and are dropped, and a trap on it is
/// not offered it (`traps::standing_event`). Stops at a prompt, leaving the rest owed in state.
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

/// B5 E1, R448: the step between §10.5 steps 3 and 4. A cast's choices, and a replacement's (R449), are
/// made first, so the announce names them (R70, R90). Then the card is announced and its window runs;
/// a question in it pauses the step, which is re-entered at its window (`PlayRun.announced`) after the
/// check the answer is owed (R59). Once the window is over the announce closes: a countered play stops
/// here (`PlayRun.countered`), and one whose card something else took out of the resolving zone is lost
/// (R226).
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

// ---------------------------------------------------------------------------
// Step 4 — move the card and announce the play (§10.5 step 4)
// ---------------------------------------------------------------------------

fn played_events(sink: &mut EngineSink<'_>, run: &PlayRun, card: &CardInstance, former_id: Option<String>) {
    // R119: what has already arrived on the field during the play — a tributed unit's Death at step 2
    // (#22's copies of a Sheepish) — does not answer it, which the step-4 pair names, as step 7's does.
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
        // R454: a play from the graveyard says so, for a client to animate it from there. Public: the
        // graveyard is.
        from: if run.source == Some(PlaySource::Graveyard) {
            Some(PlayedFrom::Graveyard)
        } else {
            None
        },
        arrived_during: arrivals.clone(),
        exits_from: Some(exits_from),
    });
    if let Some(zone) = run.zone {
        sink.events.push(GameEvent::Summoned {
            // The side it lands on, which is the player's own unless R360 summoned it for the opponent.
            player: zone.player,
            instance_id: card.id.clone(),
            def_id: card.def_id.clone(),
            row: zone.row,
            lane: zone.lane,
            former_id,
            arrived_during: arrivals,
            exits_from: Some(exits_from),
        });
    }
}

/// §10.5 step 4: the card leaves the hand for the field (Units, Field Spells, Traps) or for the
/// resolving state (Spells); `cardPlayed` goes out; `turnLog.cardsPlayed` and the game's `played`
/// counter go up (R55); "Sheepish fires here for Units".
///
/// That last clause is why this step ends in the resolution loop: the trap answers the play event
/// and resolves before step 5 runs the Cry, which is R17's "fires before the Cry (Cry lost)". It
/// also means the play's own events are never left owed while a prompt is open. The loop holds the
/// state check until something in it has resolved (§4.5, `SettleOptions.holdCheck`), so a card that
/// arrives at 0 or less health is not collected before its own Cry (R118).
fn place_step(sink: &mut EngineSink<'_>, run: &mut PlayRun) {
    // Re-entered after a pause: the answer has resolved what asked — a trap, or a trigger whose
    // answered step and tail ran in the answer's own drain — so §4.5's check is due before anything
    // else moves (R59), as the loop runs it after a trigger it resolves itself. The unit that
    // trigger killed has died before step 5's Cry counts the board (R118, R113).
    let resumed = run.placed == Some(true);
    if !resumed {
        // R70, R90: a cast has made its choices by now (`cast_choices_step`), before step 4 puts it on the
        // field, as a play makes them at step 1 with the card still in hand — so a cast Unit is never one
        // of its own Cry's options. A run owed from before that step existed makes them here.
        if run.cast == Some(true) && !cast_choices_made(sink, run, PlayStepName::CastChoices) {
            return;
        }
        run.placed = Some(true);
        if !place_card(sink, run) {
            run.lost = Some(true);
        }
    }
    // R17's step-4 window. The play has not resolved yet, so the loop holds §4.5's check until
    // something in it has (R118).
    //
    // A trap that asks here pauses the loop with its events still owed — the other traps that answer
    // the play (`triggers::OWED_TO_TRAPS`), the events after the one it answered, the triggers they
    // queue — and a trap is a response that resolves to completion before the play goes on (§10.3,
    // R118). So the step owes itself, and the answer brings it back to this loop rather than on to
    // step 5: a Sheepish owed the play's `cardPlayed` behind a trap that asked still turns the unit
    // into a Sheep before its Cry (R17).
    if run.cast != Some(true) {
        settle(sink, SettleOptions { hold_check: Some(!resumed) });
        return;
    }
    // R70: a cast is a play, so its step 4 is a window too, and a Sheepish answering a cast Unit turns
    // it into a Sheep before its Cry (R17). A cast runs inside another effect (§2.4's draw, #95), whose
    // own loop is running around it (`cast_through_pipeline`), so the window is the traps' part of that
    // loop and no more (`triggers::dispatch_pending`): every event so far reaches the traps, which fire
    // at once as responses — an earlier cast of the same chain included — while the other triggers
    // they wake, and the work owed around the cast, wait for the effect's own loop (R117). A cast
    // re-entered here after a trap's question meets the check the answer is owed first (R59).
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

/// R119: the permanents on the field now that were not there when step 4 announced the play, or have
/// left the field since and stand there again (a Reborn body, R83) — what arrived on the field while
/// the play resolved, on either side and whatever put it there. The played card is its own case
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

/// Step 4's placement and announcement, once. False when the card is no longer the hand's to move,
/// or for a cast the resolving zone's (`PlayRun.lost`), which ends the play.
fn place_card(sink: &mut EngineSink<'_>, run: &mut PlayRun) -> bool {
    // R210: step 2 held the named zone for this play; it is released here, whatever happens next.
    if let Some(zone) = run.zone {
        release_zone(sink.state, &zone);
    }
    let Some(mut card) = snapshot(sink.state, &run.instance_id) else {
        return false;
    };

    // R448, R70, R226: the card waits in the resolving zone from its announce — a played card moved
    // there out of its owner's hand, a cast's or a replacement's was made there. One that has left it
    // by now is where that move put it, in one zone (§10.1), and is not played: pulling it back out of
    // exile would undo a move §6.3 makes final. Not `move_to_zone`: that resets the instance (R78), and a
    // play does not.
    if !matches!(card.zone, Zone::Resolving { .. }) {
        return false;
    }
    remove_from_any_zone(sink.state, &card);
    if run.cast == Some(true) || run.replaced == Some(true) {
        // R70, R449: a permanent takes the leftmost empty, unlocked zone of its row (R64), as a play that
        // names none does.
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
        // R360: #55's base face, paid for with an opposing unit, is summoned for the opponent. It is
        // still this player's play (`cardPlayed`), and its owner does not change (§3.2); the zone, and
        // so its controller, is the opponent's (`place_on_field`).
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
    }

    // §3.2/§6.2 Stack: step 1 already accepted an occupied unit zone for a Stack card, so the
    // placement is the one that builds the pile — the arriving card goes on top and the card beneath
    // stops acting (R13). Every other card needs the zone empty, which is what `stack: false` keeps
    // `place_on_field` insisting on. R210 keeps the zone the play's, so a refusal is a broken
    // invariant rather than a game rule; should one ever happen, the card waits in `resolving` and
    // step 7 lands it in its graveyard, as R138 has a permanent with no zone do, rather than being
    // left in no pile at all (§10.1).
    let placed = match run.zone {
        Some(zone) => {
            let stack = plays_on_stack(sink.state, &card);
            place_on_field(sink.state, &card, &zone, PlaceOptions { stack: Some(stack) })
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
    let card = live(sink.state, &card);
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
    count_play(sink.state, &card.id);
    let card = live(sink.state, &card);
    // R673: a play of a "… in the System" card, by either player, raises the match's Glitch odds.
    if run.cast != Some(true) {
        count_system_play(sink.state, &card);
    }
    run.radiant = Some(card.radiant);
    // B5 E4, R451: the per-turn types, the per-game tags and the "last" records.
    record_play(sink.state, run.player, &card);

    played_events(sink, run, &card, former_id);
    // B3.1 rule 4 (R383): an Animated Field Spell, or an "Animated on your turn" card on its controller's
    // turn, animates as it enters the field.
    animate_on_entry(sink, &card);
    // §6.2 Echo, R30: the Spell GAINS its Echo as it is played — "the next Spell you play gains Echo
    // +1" — so the grant is taken here, from the player who played it, and not at step 6 after the
    // Spell's own text has run. Taken later, a Spell that moves Twinspell to the other side (#87's
    // board swap) or out of play took nothing, and Twinspell stayed for the other player's next one.
    // The repeats still resolve at step 6, which only takes what is queued.
    run.echo_queued = true;
    let card = live(sink.state, &card);
    queue_echo_repeats(sink, &card, run.player);
    // E39 (Classic+ #14): "the next Spell you play gains …" is gained as it is played, as Echo is (R178).
    let card = live(sink.state, &card);
    stamp_next_spell(sink, &card, run.player);
    true
}

/// E11, R454, §10.5 step 4: take the played card out of the pile the play takes it from — its owner's
/// hand, or, for a play from the graveyard, its player's graveyard. False, and nothing moved, when it is
/// no longer there (R226): a Tribute's Death at step 2 or a step-3 hook has moved it, and it is not
/// played. Exported so a step that moves the card earlier (E1's announce) takes it the same way.
///
/// `card` is the caller's copy of the card it goes on to place: TS's `removeFromAnyZone` wrote the one
/// change a removal makes to the instance onto that same object, so it is made on the copy here.
pub fn take_from_play_source(state: &mut GameState, run: &PlayRun, card: &mut CardInstance) -> bool {
    if run.source == Some(PlaySource::Graveyard) {
        if !in_own_graveyard(state, run.player, card) {
            return false;
        }
        remove_from_any_zone(state, card);
        // R155: §5.1's end-of-turn return belongs to the Spell its own play landed in the graveyard
        // (§10.5 step 7). A card that leaves the graveyard has spent that landing (`zones`' removal).
        card.return_to_hand_at_end_of_turn = None;
        return true;
    }
    let side = &mut state.players[run.player];
    let Some(at) = side.hand.iter().position(|held| held.id == card.id) else {
        return false;
    };
    side.hand.remove(at);
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

/// E39, R455 (Classic+ #14 Forever&): every live `enchantNextSpell` modifier of the player gives its
/// enchantment to the Spell being played and is spent — "the next Spell you play gains …". A cast is a
/// play (R70), so a cast Spell takes it; a countered play never reaches step 4 and leaves it waiting.
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

// ---------------------------------------------------------------------------
// Step 5 — resolve the card (§10.5 step 5)
// ---------------------------------------------------------------------------

/// §6.2 Combo X: "cards you played earlier this turn", so the card being played does not count, and
/// nor does a card its own step 5 casts — the count is the one at play time (`query::played_earlier`,
/// asked by the card's id, whose body is copied here: fullsend rule 5).
fn played_earlier_this_turn(state: &GameState, run: &PlayRun) -> i32 {
    let log = &state.players[run.player].turn_log;
    let id = &run.instance_id;
    if find_instance(state, id).is_some_and(|card| matches!(card.zone, Zone::Hand { .. })) {
        return log.cards_played;
    }
    match log.played_ids.iter().rposition(|played| played == id) {
        Some(at) => at as i32,
        None => log.cards_played,
    }
}

/// The card as step 4 left it, or null when it is no longer there to resolve: Sheepish transformed
/// it, a trap countered it, or it has ceased to exist (R11's `{ z: "gone" }`). R17: the Cry is lost.
fn still_resolving(state: &GameState, run: &PlayRun) -> Option<CardInstance> {
    let card = find_instance(state, &run.instance_id)?;
    if card.def_id != run.def_id {
        return None;
    }
    match card.zone {
        Zone::Resolving { .. } => Some(card.clone()),
        Zone::Field { .. } => {
            // R174, R118: the stay step 4 put it on. One it has left since — a trap answering the play killed
            // it at step 4 — is gone for the play, and a Reborn body in its zone is a new arrival whose "Cry
            // does not fire" (§4.5 step 4, R1).
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

/// #38 Quickstriker's lasting effect (`staticFlags.quickstriker`) as it is granted from this player's
/// side of the field: one entry per grant, each the multiple of X that grant deals as one hit — its
/// granting card's own face's (`QUICKSTRIKER_COMBO_MULTIPLE`, R281), so a base and a Radiant
/// Quickstriker give `[1, 2]`, and a card fused from two carries both texts at its one face (R102).
/// The played card is never one of them: a permanent does not answer its own arrival (R119), and a
/// Quickstriker being played is on the field by step 5. Nor is one that arrived on the field during
/// the play (`arrived_during`): a copy a tributed Cube's Death summoned at step 2, one #95's first
/// resolution summoned, met again by the Echo repeat of that same play (R119).
fn quickstriker_grants(state: &GameState, run: &PlayRun, played: &CardInstance) -> Vec<i32> {
    let arrived: IndexSet<String> = arrived_during(state, run).into_iter().collect();
    let mut multiples: Vec<i32> = Vec::new();
    for row in [Row::Units, Row::Backrow] {
        for slot in slots_of(run.player, row) {
            let Some(held) = card_at(state, &slot) else {
                continue;
            };
            if held.id == played.id || arrived.contains(&held.id) {
                continue;
            }
            let grants = match flags_of_card(state, &held).quickstriker {
                Some(FlagOrCount::Flag(true)) => 1,
                Some(FlagOrCount::Count(count)) => count.max(0),
                _ => 0,
            };
            let multiple = QUICKSTRIKER_COMBO_MULTIPLE.on(held.radiant);
            for _ in 0..grants {
                multiples.push(multiple);
            }
        }
    }
    multiples
}

/// #38 Quickstriker: "your cards gain 'Combo X: deal X damage to the enemy hero', X = cards you
/// played earlier this turn" (radiant 2X) — one of the two granted Combo parts §10.5 step 5 resolves
/// before the card's own script. It is the Field Spell's lasting effect, so it is read off the
/// permanents on the field now (one hit per grant) rather than from a trigger on `cardPlayed`, which
/// popped whenever that event was dispatched: a cast's events wait for the loop of the effect that
/// cast it (R70), so a cast-on-draw chain of two read the count after the chain, 1 and 1, instead of
/// 0 and 1. R281: each grant's hit is X times its face's multiple, dealt as ONE damage instance
/// (§4.4), so Armor and the Anti-oneshot cap apply to a Radiant one's 2X once; the grants go in board
/// order, each its own hit. A `quickstrikerDamage` modifier counts as one more grant of X.
fn quickstriker_combo(sink: &mut EngineSink<'_>, run: &PlayRun, card: &CardInstance) {
    let amount = played_earlier_this_turn(sink.state, run);
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
        let source = live(sink.state, card);
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

/// #78 /fullsend: "this turn your cards gain 'Combo: draw 1'" — a Combo with no X is Combo 1 (§6.2).
/// Each live rider is its own draw, and "draw N" is N separate draws (§2.4, R58), so the riders are
/// one `draw` of their total: a draw that pauses owes the rest to the answer (R113), and one whose
/// cast ended the game ends the rest with it (R216) — looping over the riders drew on over both.
fn combo_draw_step(sink: &mut EngineSink<'_>, run: &PlayRun) {
    if played_earlier_this_turn(sink.state, run) < 1 {
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

/// R119: whether a modifier was in place as the play began (`PlayRun.modsBefore`). One the play
/// installed itself — /fullsend's "Combo: draw 1", which an Echo repeat of the same /fullsend would
/// otherwise meet — does not answer it.
fn in_place_before(run: &PlayRun, id: &str) -> bool {
    run.mods_before
        .as_ref()
        .is_none_or(|before| before.iter().any(|known| known == id))
}

/// R174: the targets step 5 hands the card's script. §10.5 step 1 checks a play's targets and its
/// Tribute each on its own (R90), so one play may name a unit both as a target and as a Tribute —
/// #55 Lava Golem's enemy tribute, crafted onto #68 Twisted Sorcerer's "deal 4 damage to a target".
/// Step 2 sacrifices it before anything resolves, and an effect aimed at a card on the field is aimed
/// at that stay: the unit has left the field, so its slot names nothing and the effect fizzles (§8
/// Conventions), even when Reborn has put a new body in its zone. The slot is kept, as `none`, so the
/// declarations after it still read their own (R90).
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

/// §10.5 step 5: "Resolve Combo checks, Quickstriker, /fullsend's Combo draw, then the card's own
/// Cry or spell script (targets already chosen)". An ordinary card's own Combo check is part of its
/// own script, which reads `query::played_earlier`; what the engine owes is the two Combo abilities
/// another permanent grants everything you play, and they come first.
fn resolve_step(sink: &mut EngineSink<'_>, run: &mut PlayRun) {
    if run.cast == Some(true) && !cast_choices_made(sink, run, PlayStepName::Resolve) {
        return;
    }
    // R453: a cast that found no zone for its Field Spell or Trap resolves nothing.
    if run.fizzled == Some(true) {
        return;
    }
    for at in run.resolve_at..RESOLVE_PARTS.len() {
        run.resolve_at = at + 1;
        let Some(card) = still_resolving(sink.state, run) else {
            return;
        };

        match RESOLVE_PARTS[at] {
            ResolvePart::Quickstriker => quickstriker_combo(sink, run, &card),
            ResolvePart::ComboDraw => combo_draw_step(sink, run),
            ResolvePart::Script => {
                // B5 E14, R399: a copier resolves the copied Spell's script, itself as "this".
                let face = text_face_of(sink.state, &card);
                run_hook_resumable(
                    sink,
                    &face,
                    "cry",
                    RunHookOptions {
                        controller: Some(run.player),
                        targets: Some(standing_targets(run)),
                        modes: Some(run.modes.clone()),
                        data: Some(cry_data(run)),
                        // R174: the choices are aimed at the stays step 1 checked them on (a cast's, once made).
                        exits_from: run.exits_from,
                    },
                );
            }
        }

        if paused(sink) {
            return;
        }
    }
}

/// R70: "the caster picks its targets and modes", and R81: a choice made during resolution — a cast's
/// among them — opens a `PendingChoice`. So a cast of a card that declares targets or modes asks its
/// caster for them, declaration by declaration, the way an Echo repeat asks for its fresh picks
/// (§10.6) — and for the face step 5 resolves, since step 3 has already made it Radiant if it is
/// going to be (R214). It asks as step 4 begins, before the card is placed (`place_step`): a play's
/// choices are checked at step 1 with the card still in hand (R90), so a cast Unit asked after its
/// placement was offered as a target of its own Cry, which no play of it ever is. A cast whose caller
/// named its choices (none in Core) keeps them. Returns false while a prompt is waiting.
///
/// TS's `step` defaulted to "resolve"; every Rust caller names it.
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
    let card = live(sink.state, &card);
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
    let card = live(sink.state, &card);
    if let Some(slices) = slices_for(sink.state, run.player, &card, run.cost_paid, &run.targets, &run.modes) {
        run.target_slices = Some(slices);
    }
    true
}

/// R453: a cast X card's X, which its caster chooses (R70, R81) as the cast's first choice — a `number`
/// prompt over MIN_CHOSEN_X up to their current mana, at least MIN_CHOSEN_X (R348), each option the
/// number as a mode (`{ pick: "mode", option: "3" }`). A cast pays nothing, so the X costs nothing; it
/// is stored on the instance, where the card's script and R396 read it. A random cast's X was set as the
/// cast began: the caster's current mana (`cast_through_pipeline`). False while the prompt waits.
fn cast_x_chosen(sink: &mut EngineSink<'_>, run: &mut PlayRun, card: &CardInstance, step: PlayStepName) -> bool {
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

// ---------------------------------------------------------------------------
// Step 6 — Echo (§10.5 step 6, R30)
// ---------------------------------------------------------------------------

/// An option's key, built from the selection and never from its label (§10.6, R177): what the client sends back.
fn key_of(selection: &Selection) -> String {
    match selection {
        Selection::Instance { instance_id } => format!("instance:{instance_id}"),
        Selection::Hero { player } => format!("hero:{player}"),
        Selection::Zone { player, row, lane } => format!("zone:{player}:{row}:{lane}"),
        Selection::Mode { option } => format!("mode:{option}"),
        Selection::None => "none".to_string(),
    }
}

fn label_of(selection: &Selection, chooser: PlayerId) -> String {
    match selection {
        Selection::Instance { instance_id } => instance_id.clone(),
        Selection::Hero { player } => hero_option_label(*player, chooser),
        Selection::Zone { player, row, lane } => cell_option_label(*player, *row, *lane, chooser),
        Selection::Mode { option } => option.clone(),
        Selection::None => "nothing".to_string(),
    }
}

/// R81: a card's play choices travel in the `play` action *once*. A repeat therefore asks again, as
/// prompts — §10.6's "an Echo repeat of Glowy Jelly Bean reopens its hand pick" — so each
/// declaration the card made is offered in turn and the answers collect in the repeat record. The
/// prompt carries the run record, so answering it re-enters this pipeline (`answer_play_prompt`, which
/// `prompts::answer_prompt` hands it to).
///
/// Returns true when every declaration has its answer and the repeat can resolve. A declaration the
/// board cannot satisfy is skipped rather than refused: the effect fizzles (R90, §8's conventions).
///
/// TS's `step` defaulted to "echo"; every Rust caller names it.
fn ask_repeat_choices(sink: &mut EngineSink<'_>, run: &mut PlayRun, played: &CardInstance, step: PlayStepName) -> bool {
    if run.repeat.is_none() {
        return false;
    }
    // B5 E14, R399: a copier asks for the choices its copied text declares.
    let card = text_face_of(sink.state, played);
    // A declaration that belongs to some modes only (`forModes`, #24) cannot be asked before the
    // mode it depends on, so such a card's repeat asks its modes first.
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

/// R70, R452: a cast's own choices, made as a step of their own once step 3 has settled the face they
/// answer (R214) and before the card moves (R90) — so whatever announces the cast next (E1's window,
/// between steps 3 and 4) announces them. A play made its choices at step 1: nothing to do.
fn cast_choices_step(sink: &mut EngineSink<'_>, run: &mut PlayRun) {
    if run.cast != Some(true) {
        return;
    }
    cast_choices_made(sink, run, PlayStepName::CastChoices);
}

/// The options one declaration offers a pick the pipeline makes itself — an Echo repeat's or a cast's
/// (R81, R70) — narrowed to enemies when the run targets enemies and one is legal (R452), or to friendly
/// targets when the declaration is beneficial (aim "help", R656).
fn cast_target_options(state: &GameState, run: &PlayRun, card: &CardInstance, decl: &TargetDecl) -> Vec<Selection> {
    let options = legal_selections_for(state, run.player, card, decl);
    if run.target_enemies != Some(true) {
        return options;
    }
    let required = decl.min.min(options.len() as i32);
    if target_aim(decl) == TargetAim::Help {
        prefer_friends(state, run.player, &options, |selection: &Selection| selection.clone(), required)
    } else {
        prefer_enemies(state, run.player, &options, |selection: &Selection| selection.clone(), required)
    }
}

/// The repeat's target declarations, each offered in turn; false while one is waiting (R81).
fn ask_repeat_targets(sink: &mut EngineSink<'_>, run: &mut PlayRun, card: &CardInstance, step: PlayStepName) -> bool {
    let name = def_of(Some(&*sink.state), &card.def_id).name.clone();
    let chosen_modes: Vec<String> = run
        .repeat
        .as_ref()
        .map(|repeat| repeat.modes.clone())
        .unwrap_or_default();
    let targets = active_target_decls(&declared_targets(sink.state, card), &chosen_modes);
    let start = run.repeat.as_ref().map_or(0, |repeat| repeat.decl_at);
    for at in start..targets.len() {
        if let Some(repeat) = run.repeat.as_mut() {
            repeat.decl_at = at + 1;
        }
        let decl = &targets[at];
        let options = cast_target_options(sink.state, run, card, decl);
        if options.is_empty() {
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
                key: key_of(selection),
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
fn ask_repeat_modes(sink: &mut EngineSink<'_>, run: &mut PlayRun, card: &CardInstance, step: PlayStepName) -> bool {
    let name = def_of(Some(&*sink.state), &card.def_id).name.clone();
    let modes = declared_modes(sink.state, card);
    let start = run.repeat.as_ref().map_or(0, |repeat| repeat.mode_at);
    for at in start..modes.len() {
        if let Some(repeat) = run.repeat.as_mut() {
            repeat.mode_at = at + 1;
        }
        let decl = &modes[at];
        if decl.options.is_empty() {
            continue;
        }
        // R452: a random cast makes this choice itself, at random.
        if run.random == Some(true) || random_cast_of(sink.state, run.player).is_some() {
            let option = sink.rng.pick(&decl.options).cloned();
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
                options: decl
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

/// §10.5 step 6: "Echo: repeat step 5 with fresh prompts N times". The repeats outstanding live in
/// `state.echoQueue` and resolve one at a time, so a prompt inside one pauses the rest (§6.1). This
/// is the step an owed pipeline re-enters, and it picks up from whatever the queue and the current
/// repeat record say, which is why it can be entered any number of times.
///
/// Then §4.5's check, once the last resolution is over and before step 7 (R59): the check runs after
/// "a card's whole Cry, spell, trap or triggered script", and step 7's `cardResolved` is what #60 Bear
/// Honeypot and #85 Unlicensed Experimentation answer, so they must meet the board the card left —
/// without the units its Cry or spell has killed. It sits here rather than in step 7 because this
/// step is re-entered at itself: a Death hook that asks pauses the play here, and the answer brings
/// it back through a check that finds nothing left to do, and on to step 7.
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
        queue_echo_repeats(sink, &opening, run.player);
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
            // §4.5: the resolution before this repeat was a whole spell script, and the state check runs
            // after every one (R59) — so its deaths, their Death hooks and a hero at 0 are settled before
            // the repeat asks anything. A dead unit is not offered again, and a game the first resolution
            // won ends there. A Death hook that asks pauses the repeats here; they wait in state.
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

        let card = live(sink.state, &card);
        if !ask_repeat_choices(sink, run, &card, PlayStepName::Echo) {
            return;
        }
        if !resolve_repeat(sink, run) {
            return;
        }
    }
}

/// One Echo repeat, once its fresh answers are in: step 5 again, whole — #38 Quickstriker's and #78
/// /fullsend's granted Combo parts, then the card's own script (§10.5 step 6: "repeat step 5"). The
/// repeat record keeps its place in `RESOLVE_PARTS`, so a Combo draw that pauses resumes at the next
/// part rather than drawing again; the record is let go as the script starts, as before, because the
/// script's own tail is parked by `run_hook_resumable` and resumes ahead of this step (R113).
///
/// Returns false when a prompt (or the end of the game) stopped the repeat.
fn resolve_repeat(sink: &mut EngineSink<'_>, run: &mut PlayRun) -> bool {
    let Some(repeat) = run.repeat.as_mut() else {
        return true;
    };
    // R174: the stays the repeat's fresh picks were made on, taken once, as its answers are all in.
    if repeat.exits_from.is_none() {
        repeat.exits_from = Some(exit_mark(sink.state));
    }
    let start = repeat.part_at.unwrap_or(0);
    for at in start..RESOLVE_PARTS.len() {
        if let Some(repeat) = run.repeat.as_mut() {
            repeat.part_at = Some(at + 1);
        }
        let Some(card) = still_resolving(sink.state, run) else {
            break;
        };
        match RESOLVE_PARTS[at] {
            ResolvePart::Quickstriker => quickstriker_combo(sink, run, &card),
            ResolvePart::ComboDraw => combo_draw_step(sink, run),
            ResolvePart::Script => {
                let repeat = run.repeat.take().unwrap_or_default();
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
                    RunHookOptions {
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

// ---------------------------------------------------------------------------
// Step 7 — where a Spell lands (§10.5 step 7)
// ---------------------------------------------------------------------------

/// §10.5 step 7, which `echo.rs` owns because a cast lands the same way (R70): the card reaches the
/// graveyard and `cardResolved` goes out — once per play, here, after step 6 has drained every Echo
/// repeat, which is the moment R17 gives Bear Honeypot, Unstable Clone Machine and Unlicensed
/// Experimentation.
///
/// R155: this is also where §5.1's `returnToHandAtEndOfTurn` is written, because this is the step
/// §5.1 describes — the Spell has just reached the graveyard, and only a card that got there this
/// way returns from it at the end of the turn. `resolve::flag_return_to_hand_at_end_of_turn` holds the
/// three conditions; it runs after the landing because "reached the graveyard" is one of them, and it
/// is a no-op for everything else the step lands — a permanent, and a Spell that exiled itself (#39).
fn finish_step(sink: &mut EngineSink<'_>, run: &mut PlayRun) {
    // E39, R410, R455 (Classic+ #14 Forever&): a Spell with the return comes back once it has landed.
    let landing = find_instance(sink.state, &run.instance_id)
        .is_some_and(|card| matches!(card.zone, Zone::Resolving { .. }));
    let arrived = arrived_during(sink.state, run);
    land_after_resolution(
        sink,
        &ResolvedCard {
            instance_id: run.instance_id.clone(),
            def_id: run.def_id.clone(),
            player: run.player,
            // The same number step 4's `cardPlayed` reported: `run.costPaid` is the mana step 2 actually
            // charged, after every modifier and with R65's X and embiggen prices in it. A cast carries 0
            // (R70), which `cast_through_pipeline` puts on the run it drives.
            cost_paid: run.cost_paid,
            radiant: run.radiant.unwrap_or(false),
            // R174, R61: whether the card is still in play is asked of the stay step 4 put it on.
            placed_from: run.placed_from,
            arrived_during: Some(arrived),
        },
    );
    if landing {
        return_after_resolving(sink, run);
    }
    flag_return_to_hand_at_end_of_turn(sink.state, &run.instance_id);
}

/// E39, R410, R455 (Classic+ #14 Forever&: "After this resolves, return it to hand"): a Spell
/// carrying a `returnAfterResolve` enchantment that has resolved — played or cast — and that step 7 has
/// just landed goes back to its owner's hand from the graveyard or the exile pile it went to (its own
/// "exile this", a cast's "then exile it", a "would go to a graveyard" replacement), the hand cap
/// burning it as always (§2.4). A countered Spell never reaches step 7, a discarded one was never
/// played, and a fizzled cast resolved nothing.
fn return_after_resolving(sink: &mut EngineSink<'_>, run: &PlayRun) {
    if run.fizzled == Some(true) {
        return;
    }
    let Some(card) = snapshot(sink.state, &run.instance_id) else {
        return;
    };
    if !matches!(card.zone, Zone::Graveyard { .. } | Zone::Exile { .. }) {
        return;
    }
    if card_type_of(sink.state, &card) != CardType::Spell || !has_return_after_resolve(&card) {
        return;
    }
    add_to_hand(sink, &card);
}

// ---------------------------------------------------------------------------
// The driver
// ---------------------------------------------------------------------------

/// One step's body.
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

/// Run the steps from `run.at` on, and stop the moment one leaves a prompt open.
///
/// The rest of the pipeline is owed to `state.work` *only when a step actually pauses*, never
/// before — R117, which this file used to get wrong. Two reasons, and the first is that bug:
///
///  - While `drive` is on the stack the steps are the driver's, so nothing else may run them.
///    Step 4 ends in the resolution loop (`place_step`), and `settle` drains `state.work` before it
///    pops a trigger — so a pipeline that had parked itself in advance was drained by its own
///    nested `settle`, which ran steps 5 to 8 (the Cry included) and then handed control back to a
///    loop that ran them again. Owing the rest only at a pause keeps one owner per step: the
///    driver while it is running, `state.work` once it has stopped (R1: a Cry fires exactly once).
///  - It is also the order R113 wants, which R117 says follows from the rule above rather than being
///    separate. `work.rs` parks at `state.workCursor`, which the pausing scope has just advanced past
///    its own item, so the tail of a Cry's effect list — parked by the step that was running — lands
///    in front of this one and resumes first: steps 6, 7 and 8 come *after* the Cry rather than
///    inside it. Parking in advance put the pipeline ahead of that tail.
///
/// Returns true when the pipeline is finished with (a game that ended under it included).
fn drive(sink: &mut EngineSink<'_>, run: &mut PlayRun) -> bool {
    // R452: a random cast, or one that targets enemies, is in force while its steps run, however often
    // they are driven — the first time, and each time an answer brings the owed rest back.
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
    if run.cast != Some(true) || (run.random != Some(true) && run.target_enemies != Some(true)) {
        return None;
    }
    Some(CastMode {
        instance_id: run.instance_id.clone(),
        player: run.player,
        random: run.random == Some(true),
        target_enemies: run.target_enemies == Some(true),
        casts: 1,
    })
}

/// `drive`'s loop over the steps from `run.at`; true when the pipeline is finished with.
fn drive_steps(sink: &mut EngineSink<'_>, run: &mut PlayRun) -> bool {
    for at in run.at..STEP_TABLE.len() {
        let step = &STEP_TABLE[at];

        // Where a pause would pick up. A `repeats` step is re-entered at itself, because it holds its
        // own place inside the record — and the record is read when the pause is parked, after the step
        // has moved that cursor, never before.
        let next = if step.repeats { at } else { at + 1 };
        // A card lost at step 4 is not played (R226): nothing resolves, and step 8 settles what steps 2
        // and 3 did (`PlayRun.lost`).
        // R448: a countered play is the same — nothing after its announce but step 8.
        if (run.lost == Some(true) || run.countered == Some(true)) && step.name != PlayStepName::Settle {
            continue;
        }
        (step.run)(sink, run);

        if sink.state.result.is_some() {
            return true;
        }
        if sink.state.pending.is_some() {
            // Nothing to owe when the last step is the one that paused, and nothing to owe for a prompt
            // this pipeline opened itself: that prompt's own `resume` already carries the record, so a
            // work item would be a second copy of the same continuation (§10.5 step 6's fresh picks).
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
        repeat.modes.extend(selection.iter().filter_map(|pick| match pick {
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

/// R70: a cast is a play, "free … with cost paid 0", so it runs this pipeline from step 3 — step 1
/// has nothing to validate, since the effect chose the card, and step 2 pays nothing. Everything else
/// is a play's: #64 Gifted Program's hook at step 3, the placement, counters, `cardPlayed` and the
/// Echo gained as it is played at step 4 (R178), #38 Quickstriker's and #78 /fullsend's granted Combo
/// parts before the card's own script at step 5, the repeats with fresh prompts at step 6 and the
/// landing and `cardResolved` at step 7. A step that asks parks the rest of the cast as an ordinary
/// `"play"` item at the moment it pauses (R113, R117), exactly as a play does.
///
/// `resolve::cast_card` is the entry point; TS reached this through a driver it registered, because
/// `resolve.ts` sat under `prompts.ts` and could not import the pipeline itself. Rust calls it directly.
pub fn cast_through_pipeline(sink: &mut EngineSink<'_>, instance: &CardInstance, options: &CastOptions) {
    // The card an effect casts may be in no pile yet — drawn off the library (§2.4) or made from the
    // catalog (#95) — and the pipeline finds its card by id, so it waits in the resolving zone from
    // the start, as a card being played does (§10.5 step 4, R98).
    let player = instance.controller;
    let mut card = instance.clone();
    let from_graveyard =
        find_instance(sink.state, &card.id).is_some_and(|held| matches!(held.zone, Zone::Graveyard { .. }));
    remove_from_any_zone(sink.state, &card);
    // R155: a card that leaves the graveyard has spent the landing §5.1's end-of-turn return belongs to
    // (the one change TS's `removeFromAnyZone` wrote onto the instance it was handed).
    if from_graveyard {
        card.return_to_hand_at_end_of_turn = None;
    }
    card.zone = Zone::Resolving { player };
    // B5 E14, R546: a copier cast resolves the Spell that is last as the cast begins. (Written before the
    // card joins the resolving zone, which the copy does not read.)
    fix_copied_text(sink.state, &mut card, None);
    sink.state.players[player].resolving.push(card.clone());

    // R452: a cast made while a random cast of its caster's resolves is random too, and counts in that
    // cast's chain; a card that targets enemies (E39's enchantment) is cast so, whoever casts it.
    let random = options.random == Some(true) || random_cast_of(sink.state, player).is_some();
    let target_enemies = options.target_enemies == Some(true) || has_target_enemies(&card);
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
    // R453: "then exile it" (Classic #56) — step 7 lands the resolved Spell in exile (R178's mark).
    // (`afterward` has one value, "exile".)
    if options.afterward.is_some()
        && let Some(resolving) = find_instance_mut(sink.state, &card.id)
    {
        exile_on_landing(resolving);
    }

    let targets: Vec<Selection> = options.targets.clone().unwrap_or_default();
    let modes: Vec<String> = options.modes.clone().unwrap_or_default();
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
    run.mana_before = Some(sink.state.players[player].mana.current);
    // A cast starts past the pay step, so it owes no targeting cost (as before: casts never paid one).
    run.targeting_owed = 0;
    drive(sink, &mut run);
}

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

/// §10.5, all eight steps: validate the play, then run steps 2 to 8, pausing wherever a prompt
/// opens. Returns the refusal, or `Ok` once the play has run (or has owed itself the rest).
pub fn run_play_steps(sink: &mut EngineSink<'_>, player: PlayerId, action: &PlayAction) -> Result<(), EngineError> {
    let mut run = validate_play(sink, player, action)?;
    intercept_declared_targets(sink, &mut run);
    drive(sink, &mut run);
    Ok(())
}

/// B5 E5, E9, R450: the targeting point of a play's declared targets, at §10.5 step 1 once the play is
/// legal. An interceptor in the targeted player's hand (Classic #33) answers the first declared
/// `target` pick naming one of that player's units — a `by: "spell"` one only when the played card
/// is a Spell (R651) — if it would itself be a legal pick of that declaration, and the pick moves to
/// it. The costs the picks carry were checked at step 1 and are paid at step 2, whatever the
/// interception did (the targeting happened).
fn intercept_declared_targets(sink: &mut EngineSink<'_>, run: &mut PlayRun) {
    if run.targets.is_empty() {
        return;
    }
    let Some(card) = snapshot(sink.state, &run.instance_id) else {
        return;
    };
    // R214: the face whose declarations the picks answer.
    let face = resolving_face(sink.state, run.player, &card, run.cost_paid);
    let decls: Vec<Option<TargetDecl>> = targeting_decls_of(sink.state, run.player, &face, &run.targets, &run.modes, None);
    // With no pick a targeting one, the interception leaves every pick as it is (TS's call returned
    // the picks unchanged), so it is not asked.
    if !decls.iter().any(Option::is_some) {
        return;
    }
    // R651: a `by: "spell"` interceptor (Classic #33 Joro) answers only a Spell's declared target.
    let source = card_type_of(sink.state, &card);
    let player = run.player;
    // `accepts` reads the board: nothing moves before the first pick it accepts (the interception
    // stops there), so a copy of the board as it stands now reads as TS's live state did, and leaves
    // the sink free for the interception's own writes.
    let board: GameState = sink.state.clone();
    let targeting = |index: usize| decls.get(index).is_some_and(Option::is_some);
    let accepts = |interceptor: &CardInstance, index: usize| match decls.get(index) {
        Some(Some(decl)) => interceptor_fits_decl(&board, player, &face, decl, interceptor),
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

/// Answer a prompt this pipeline opened itself (§10.5 step 6's fresh picks, a cast's choices at step
/// 4): validate the answer the same way, close the prompt, file the selection and drive on. Such a
/// prompt names the `"play"` sequence as its hook, which no card script holds, so `prompts.rs`'s
/// answerer match hands it here: every caller answers through that one entry point, the reducer and a
/// caller driving the engine directly alike, and none of them drops the selection and the rest of the
/// play (R122, R113).
///
/// Returns the refusal, or `Ok`.
pub fn answer_play_prompt(sink: &mut EngineSink<'_>, answer: &AnswerInput) -> Result<(), EngineError> {
    let Some(pending) = sink.state.pending.clone() else {
        return Err(EngineError::new("no prompt is open"));
    };

    why_answer_refused(&pending, answer)?;

    // TS read the run through `prompts.resumeOf(pending)`, which only fills fields a JSON round trip
    // may have dropped; Rust's `Resume` always has them.
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
        is_play_resume(&item.resume) && run_of(&item.resume).is_some_and(|owed| owed.instance_id == instance_id)
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
    // R122: the action that answers finishes what the prompt interrupted — the draw chain a cast's
    // own question stopped (§2.4), the steps a trap's play owes — before the resolution loop moves, as
    // `prompts::answer_prompt` does; otherwise the traps answered the cast's events before the draw
    // repeated into the card beneath it.
    drain_work(sink);
}
