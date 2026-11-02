//! Delayed effects (SPEC §2.2's turn loop, §10.1, §10.6, R62, R68, R76).
//!
//! "At the start of your next turn …" and "End of turn: …" are not triggers but §2.2's own two points
//! in the turn, in R62's order; `modifiers` stores and picks them (R68's creation order) and `turn`
//! re-enters them. This file is the verb, as a card script may not write state (CLAUDE.md rule 5).
//!
//! WHAT IS STORED IS A `Resume`, NEVER A CLOSURE (§9.3, §10.6; its face, §5.2), so it survives a JSON
//! round-trip and a replay. Its instance is optional: R76 has #50 K-Pop Fanatic's steal fire "even if
//! K-Pop Fanatic died", so a card carries what it needs in `data`, not through `ctx.self`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::effects::destroy::{DestroyArgs, destroy, destroy_all};
use crate::effects::move_::{DiscardHandArgs, discard_hand};
use crate::effects::radiant::{RadiantTarget, set_radiant};
use crate::effects::steal::{StealOtherwise, StealTarget, steal};
use crate::effects::targets::{
    BoardScope, PlayerSpec, TargetSpec, cards_in_scope_aimed, instance_by_id_or_spec, instance_of, player_of,
    stands_since_script_began,
};
use crate::marks::{mark_delayed, sync_marks};
use crate::modifiers::{add_lane_watch, add_play_watch, add_start_of_turn_effect, schedule_delayed};
use crate::prompts::{RESUME_HOOK, SELF_KEY, resume_self};
use crate::random_cast::RANDOM_CHOICES_KEY;
use crate::resolve::{HookOptions, make_context};
use crate::script::{Effect, EffectContext, EngineSink};
use crate::state::{CardInstance, DelayedAt, DelayedEffect, Resume, find_instance, is_turn_of};
use crate::wire::{CardMark, GameEvent, Phase, PlayerId, Row, ZoneName, opponent_of};
use crate::work::RUN_MARKS_KEY;
use crate::zones::{card_at, slots_of, standing_units_of};

/// The `Script` key a delayed continuation lands on at its R62 point unless the card names another,
/// such as `RESUME_HOOK` for an entry in its `resume` step table.
pub const DELAYED_HOOK: &str = "delayed";

/// R350: `DelayAt.player` for the turn that is running, whoever's it is.
pub const THIS_TURN: &str = "turn";

/// R1140: the `ctx.data` key where a hand-watching delayed step finds the cards still watched, in pick order.
pub const HAND_WATCH_KEY: &str = "handWatch";

/// `DelayAt.player`: a `PlayerSpec`, relative to the controller like every other, or `"turn"`
/// (`THIS_TURN`): the player whose turn is running as the effect is made.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum DelayPlayer {
    #[serde(rename = "self")]
    SelfSide,
    #[serde(rename = "enemy")]
    Enemy,
    #[serde(rename = "turn")]
    Turn,
}

impl DelayPlayer {
    fn spec(self) -> Option<PlayerSpec> {
        match self {
            DelayPlayer::SelfSide => Some(PlayerSpec::SelfSide),
            DelayPlayer::Enemy => Some(PlayerSpec::Enemy),
            DelayPlayer::Turn => None,
        }
    }
}

/// When a delayed effect comes due, in the vocabulary a card file writes: §2.2's two points. The
/// player is relative to the controller like every other `PlayerSpec`, or `"turn"`: the player whose
/// turn is running as the effect is made — R350's "at the end of this turn", whoever's turn that is
/// (#90.1 CN-Virus, cast on a draw that may come on either player's turn).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DelayAt {
    /// Only `Phase::Start` or `Phase::End`.
    pub phase: Phase,
    pub player: DelayPlayer,
}

/// The player a `DelayAt` waits for. During setup, which is no player's turn, "turn" is p1's first (§2.1).
fn delay_player(ctx: &EffectContext<'_>, at: &DelayAt) -> PlayerId {
    match at.player.spec() {
        None => ctx.sink.state.active,
        Some(spec) => player_of(ctx, spec),
    }
}

/// `delay`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DelayArgs {
    pub at: DelayAt,
    /// The step the continuation re-enters (§10.6: "script id + step + captured data").
    pub step: String,
    /// The `Script` key that step lives under; `DELAYED_HOOK` unless the card says otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hook: Option<String>,
    /// What the continuation carries across the boundary — the only place it may keep anything.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<IndexMap<String, Value>>,
    /// R174: the card on the field this effect is aimed at, if any. The effect is forgotten the moment
    /// that card leaves the field, so it never lands on a card that left and came back (#50).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watch: Option<String>,
    /// B5 E27, R458: "at the start / end of your *next* turn" rather than the next such boundary — the
    /// boundary of the turn it is made on passes it by (`DelayedEffect.notBefore`), so an end-of-turn
    /// clause made on the controller's own turn waits for the end of their next one (Classic #37
    /// Radiant). R241 does not drop one made on the other player's turn: that player's next turn exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<bool>,
    /// R437: the mark the watched card carries in both views while this effect waits (#50's pending
    /// steal, `{ mark: "steal", color: "purple" }`). Ignored without `watch` or `handWatch`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mark: Option<CardMark>,
    /// R1140 (ME-HANDMARK, Meditative #76 Do or Die): hand cards this effect is aimed at, by instance id.
    /// Each is watched for its stay in that hand: it is dropped as it leaves (played, discarded,
    /// shuffled away, stolen, R174), the effect with it once none is left, and a card that comes back is
    /// a new stay nobody watches. Those of them not in a hand now are left out, and with none left
    /// nothing is scheduled. `mark` marks each (R1141). The step finds the ones still watched under
    /// `HAND_WATCH_KEY`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hand_watch: Option<Vec<String>>,
    /// R1181000 (Meditative+ #4 Supply Drop): "in N turns" — the start (or end) of `at.player`'s Nth turn
    /// from now, counted in that player's own turns started (an extra turn counts, R846), so one made on
    /// the other player's turn counts from their next. The boundary of the turn it is made on passes it
    /// by, as `next` does, and R241 does not drop an end-of-turn one made on the other player's turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turns: Option<i32>,
    /// R1181000, R169: with `turns`, the caption of the countdown badge on the controller's side, shown
    /// in both views while the effect waits (`"Supply Drop: 4 damage to each enemy in 2 turns"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// §6.2's "at the start of your next turn" and "end of turn" as one verb (#39 Recycling Initiative,
/// #50 K-Pop Fanatic, #78 /fullsend). `owner` is the controller: who the continuation runs as and,
/// through R68's creation `seq`, where it sits among several due at once. Scheduling never fizzles: a
/// card may exile or sacrifice itself in the same list (#39), and the entry stays (R76, R86).
pub fn delay(args: DelayArgs) -> Effect {
    Effect::new("delay", move |ctx| {
        // R174, R76: aimed at the target's stay. One an earlier effect of the list took off the field
        // (#52 bounced, #22 sacrificed) fizzles it now, not what later stands under its id (R78, R83).
        if let Some(watch) = &args.watch
            && !stands_since_script_began(ctx, watch)
        {
            return;
        }
        // R241: "End of turn" is the controller's end of the turn it was made on (§6.2, #39, #78), so one
        // made on the other player's turn (a cast on draw, R70; R155) is not scheduled at all.
        if args.next != Some(true) && args.turns.is_none() && ends_other_players_turn(ctx, &args.at) {
            return;
        }
        let hand_watch: Option<Vec<String>> = args.hand_watch.as_ref().map(|ids| {
            ids.iter()
                .filter(|id| find_instance(ctx.state, id).is_some_and(|card| card.zone.z() == ZoneName::Hand))
                .cloned()
                .collect()
        });
        if hand_watch.as_ref().is_some_and(Vec::is_empty) {
            return;
        }
        let resume = delayed_resume(
            ctx,
            &args.step,
            args.data.clone().unwrap_or_default(),
            args.hook.as_deref().unwrap_or(DELAYED_HOOK),
        );
        let at = DelayedAt {
            phase: args.at.phase,
            player: delay_player(ctx, &args.at),
        };
        // R1181000: the count of `at.player`'s turns started at which it comes due.
        let due_own_turn = args
            .turns
            .map(|turns| ctx.sink.state.players[at.player].turns_started + turns);
        let not_before = if args.next == Some(true) {
            Some(next_turn_mark(ctx))
        } else {
            None
        };
        let controller = ctx.controller;
        let entry = schedule_delayed(ctx, controller, at, resume, args.watch.clone(), not_before);
        let mut countdown = false;
        if let Some(stored) = ctx.state.delayed.iter_mut().find(|due| due.id == entry.id) {
            if let Some(ids) = &hand_watch {
                stored.hand_watch = Some(ids.clone());
            }
            stored.due_own_turn = due_own_turn;
            stored.label = due_own_turn.and(args.label.clone());
            countdown = crate::modifiers::has_countdown(stored);
        }
        // R1181000, R169: a countdown is a badge on the controller's side while it waits.
        if countdown {
            ctx.events.push(GameEvent::ModifierChanged {
                player: controller,
                modifier_id: entry.id.clone(),
                added: true,
            });
        }
        // R437: the watched card carries the mark while it waits; R1141: so does each watched hand card.
        if let Some(mark) = &args.mark {
            match &hand_watch {
                Some(ids) => sync_marks(ctx, &entry.id, mark, ids),
                None => mark_delayed(ctx, &entry, mark),
            }
        }
    })
}

/// R1140: the continuation a due entry re-enters — its stored `Resume`, and for a hand watch the cards
/// still watched as it runs, under `HAND_WATCH_KEY`.
pub fn due_resume(entry: &DelayedEffect) -> Resume {
    let mut resume = entry.resume.clone();
    if let Some(ids) = &entry.hand_watch {
        resume
            .data
            .insert(HAND_WATCH_KEY.to_string(), Value::from(ids.clone()));
    }
    resume
}

/// The continuation a delayed effect stores: this script's step as a run of its own. R127: it re-enters
/// as whatever is left of its card then, dropping a Death hook's snapshot (R89), its run's marks
/// (`RUN_MARKS_KEY`; a watched card goes through `watch`, R174) and random choices (R1855001).
pub(crate) fn delayed_resume(
    ctx: &EffectContext<'_>,
    step: &str,
    data: IndexMap<String, Value>,
    hook: &str,
) -> Resume {
    let mut built = resume_self(ctx, step, data);
    built.data.shift_remove(SELF_KEY);
    built.data.shift_remove(RUN_MARKS_KEY);
    built.data.shift_remove(RANDOM_CHOICES_KEY);
    Resume {
        hook: hook.to_string(),
        ..built
    }
}

/// R458: the first turn a "next turn" clause may run on — any turn after the one it is made on.
fn next_turn_mark(ctx: &EffectContext<'_>) -> i32 {
    ctx.sink.state.turn + 1
}

/// R241: an end-of-turn clause of the controller's own, made on a turn that is not the controller's —
/// the other player's, or setup's, which is no player's turn though `active` names p1 there (§2.1).
fn ends_other_players_turn(ctx: &EffectContext<'_>, at: &DelayAt) -> bool {
    // R350: "this turn" is the turn running, whoever's it is, so there is no other player's turn to miss.
    let Some(spec) = at.player.spec() else {
        return false;
    };
    at.phase == Phase::End
        && player_of(ctx, spec) == ctx.controller
        && !is_turn_of(ctx.sink.state, ctx.controller)
}

// Verbs: activate and turn (B5 E27 delayed kinds, E28 rest of the game).

/// B5 E27: the `Resume.hook`s of delayed effects the engine resolves itself (`run_engine_delayed`), so
/// no card needs a `delayed` hook of its own; every other entry re-enters its card's step (R126).
pub const DELAYED_DESTROY_HOOK: &str = "@delayedDestroy";
pub const DELAYED_DISCARD_HAND_HOOK: &str = "@delayedDiscardHand";
/// R963, MD-C4 (Meditative #39.5 Jade Beauty's Allure): the hook an Allured Unit's steal waits under.
pub const DELAYED_ALLURE_HOOK: &str = "@delayedAllure";
/// R963: the mark an Allured enemy Unit carries in both views while the steal waits (R437).
pub const ALLURE_MARK: &str = "allure";
pub const ALLURE_MARK_COLOR: &str = "pink";
/// R1850001 (Stinkyscape #110 iKwzki's Duo and Squad): the hook a delayed Make Radiant waits under.
pub const DELAYED_RADIANT_HOOK: &str = "@delayedRadiant";
/// R1850001: the mark a Duo'd or Squad'd permanent carries in both views while it waits (R437).
pub const DUO_MARK: &str = "duo";
pub const DUO_MARK_COLOR: &str = "gold";
/// R2321001: the mark a card carries in both views while a destroy at the end of this turn watches it
/// (R437; TG #15 In a hurry's Rush Tokens, R2198002).
pub const END_OF_TURN_MARK: &str = "endOfTurn";
/// R2321001: that mark's color, the client's ember of a burning fuse.
pub const END_OF_TURN_MARK_COLOR: &str = "orange";

/// The def id and face an engine delayed effect records: the running `ctx.def_id` first (B5 E14, R546).
fn maker_of(ctx: &EffectContext<'_>) -> (String, bool) {
    let def_id = ctx
        .def_id
        .clone()
        .or_else(|| ctx.self_.as_ref().map(|card| card.def_id.clone()))
        .unwrap_or_default();
    (def_id, ctx.radiant)
}

/// `destroyAtNextTurnStart`'s argument: one unit by a `TargetSpec`, or a board scope read when it
/// resolves.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(untagged)]
pub enum DestroyAtNextTurnStartArgs {
    Scope {
        scope: BoardScope,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mark: Option<CardMark>,
    },
    Target {
        target: TargetSpec,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mark: Option<CardMark>,
    },
}

/// B5 E27, R458, R174 (Classic #20 The Power to Punish): the unit `target` names is destroyed at the
/// start of its controller's next turn — a destroy (§6.3: Indestructible ignores it, R46; R59) aimed at
/// that stay on the field (`watch`, R76, R83). `scope` (the Radiant face) names its Units as it resolves,
/// not now; R750: its `mark` goes on every Unit the scope names while it waits (`refresh_scope_marks`).
pub fn destroy_at_next_turn_start(args: DestroyAtNextTurnStartArgs) -> Effect {
    Effect::new("destroyAtNextTurnStart", move |ctx| {
        let (def_id, radiant) = maker_of(ctx);
        let controller = ctx.controller;
        let at = DelayedAt {
            phase: Phase::Start,
            player: controller,
        };
        match &args {
            DestroyAtNextTurnStartArgs::Scope { scope, mark } => {
                let mut data: IndexMap<String, Value> = IndexMap::new();
                data.insert(
                    "scope".to_string(),
                    serde_json::to_value(scope).expect("a board scope is JSON"),
                );
                if let Some(mark) = mark {
                    data.insert(
                        "mark".to_string(),
                        serde_json::to_value(mark).expect("a card mark is JSON"),
                    );
                }
                let resume = Resume {
                    def_id,
                    hook: DELAYED_DESTROY_HOOK.to_string(),
                    step: "scope".to_string(),
                    radiant,
                    instance_id: None,
                    data,
                };
                schedule_delayed(ctx, controller, at, resume, None, None);
            }
            DestroyAtNextTurnStartArgs::Target { target, mark } => {
                let Some(unit) = instance_of(ctx, target) else {
                    return;
                };
                if unit.zone.z() != ZoneName::Field {
                    return;
                }
                let mut data: IndexMap<String, Value> = IndexMap::new();
                data.insert("instanceId".to_string(), Value::String(unit.id.clone()));
                let resume = Resume {
                    def_id,
                    hook: DELAYED_DESTROY_HOOK.to_string(),
                    step: "unit".to_string(),
                    radiant,
                    instance_id: None,
                    data,
                };
                let entry = schedule_delayed(ctx, controller, at, resume, Some(unit.id.clone()), None);
                // R437: the watched unit carries the mark while the destroy waits (Classic #20's red aura).
                if let Some(mark) = mark {
                    mark_delayed(ctx, &entry, mark);
                }
            }
        }
    })
}

/// R963, MD-C4 (Meditative #39.5 Jade Beauty): "Allure every enemy Unit" — each enemy Unit on the field
/// now (a pile's top: never a carried Unit, R446, or one dormant under a Stack, R13) is stolen at the
/// start of the Allurer's next turn, or destroyed with no open unit zone. One entry per Unit, watching
/// its stay (R174) with the `allure` mark (R437); the entries outlive the Allurer (R76).
pub fn allure_enemy_units() -> Effect {
    Effect::new("allureEnemyUnits", move |ctx| {
        let (def_id, radiant) = maker_of(ctx);
        let owner = ctx.controller;
        let at = DelayedAt {
            phase: Phase::Start,
            player: owner,
        };
        let mark = CardMark {
            mark: ALLURE_MARK.to_string(),
            color: ALLURE_MARK_COLOR.to_string(),
        };
        // Each pile's top in lane order, then (R1081000) the Units standing in the backrow.
        let mut units: Vec<CardInstance> = slots_of(opponent_of(owner), Row::Units)
            .into_iter()
            .filter_map(|slot| card_at(ctx.state, slot).cloned())
            .collect();
        units.extend(
            standing_units_of(ctx.state, opponent_of(owner))
                .into_iter()
                .cloned(),
        );
        // R1839000: a Dormant card is marked by nothing, so Allure passes it by.
        units.retain(|unit| !crate::slumber::is_slumbering(unit));
        for unit in units {
            let mut data: IndexMap<String, Value> = IndexMap::new();
            data.insert("instanceId".to_string(), Value::String(unit.id.clone()));
            let resume = Resume {
                def_id: def_id.clone(),
                hook: DELAYED_ALLURE_HOOK.to_string(),
                step: "unit".to_string(),
                radiant,
                instance_id: None,
                data,
            };
            let entry = schedule_delayed(ctx, owner, at, resume, Some(unit.id.clone()), None);
            mark_delayed(ctx, &entry, &mark);
        }
    })
}

/// R1876002 (Stinkyscape #37 Kwzka's Death): "Remove every Allure on the field" — every waiting Allure
/// whose marked card is on the field, whoever made it, is forgotten; its mark comes off at the next
/// sweep (R437). An Allure aimed at a hand card (R1140) stays.
pub fn remove_allures() -> Effect {
    Effect::new("removeAllures", move |ctx| {
        let state = &*ctx.state;
        let withdrawn: Vec<String> = state
            .marks
            .iter()
            .flatten()
            .filter(|record| {
                record.mark == ALLURE_MARK
                    && find_instance(state, &record.instance_id)
                        .is_some_and(|card| card.zone.z() == ZoneName::Field)
            })
            .map(|record| record.delayed_id.clone())
            .collect();
        if withdrawn.is_empty() {
            return;
        }
        ctx.state.delayed.retain(|entry| !withdrawn.contains(&entry.id));
    })
}

/// `duoRadiant`'s argument: the permanent to Duo with (the first chosen one by default).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DuoRadiantArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
}

/// R1850001: the delayed Make Radiant of one permanent at the start of `owner`'s next turn, watching its
/// stay (R174) with the public `duo` mark (R437). A face-up Radiant card is passed by; a face-down one
/// is marked whatever its face, which the other seat may not read (R177).
fn schedule_radiant(ctx: &mut EffectContext<'_>, card: &CardInstance) {
    if card.radiant && !crate::preview::is_face_down(ctx.state, card) {
        return;
    }
    let (def_id, radiant) = maker_of(ctx);
    let owner = ctx.controller;
    let at = DelayedAt {
        phase: Phase::Start,
        player: owner,
    };
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert("instanceId".to_string(), Value::String(card.id.clone()));
    let resume = Resume {
        def_id,
        hook: DELAYED_RADIANT_HOOK.to_string(),
        step: "permanent".to_string(),
        radiant,
        instance_id: None,
        data,
    };
    let entry = schedule_delayed(ctx, owner, at, resume, Some(card.id.clone()), None);
    let mark = CardMark {
        mark: DUO_MARK.to_string(),
        color: DUO_MARK_COLOR.to_string(),
    };
    mark_delayed(ctx, &entry, &mark);
}

/// R1850001 (Stinkyscape #110 iKwzki): "Duo with a permanent and make it Radiant next turn" —
/// `schedule_radiant` for the permanent `target` names on the field, either side, landing even after
/// its maker has left (R76).
pub fn duo_radiant(args: DuoRadiantArgs) -> Effect {
    Effect::new("duoRadiant", move |ctx| {
        let Some(card) = instance_by_id_or_spec(ctx, None, args.target.as_ref()) else {
            return;
        };
        if !crate::zones::acts_on_field(ctx.state, &card) {
            return;
        }
        schedule_radiant(ctx, &card);
    })
}

/// R1850001 (#110's Radiant face): "Squad with all your permanents and make them Radiant next turn" —
/// `schedule_radiant` for each of the controller's permanents on the field now, but a Dormant one.
pub fn squad_radiant() -> Effect {
    Effect::new("squadRadiant", move |ctx| {
        let owner = ctx.controller;
        let cards: Vec<CardInstance> = slots_of(owner, Row::Units)
            .into_iter()
            .chain(slots_of(owner, Row::Backrow))
            .filter_map(|slot| card_at(ctx.state, slot).cloned())
            .filter(|card| !crate::slumber::is_slumbering(card))
            .collect();
        for card in cards {
            schedule_radiant(ctx, &card);
        }
    })
}

/// `discardHandAtTurnEnd`'s `turn`: the end of this turn, or of the controller's next one.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum DiscardHandTurn {
    This,
    Next,
}

impl DiscardHandTurn {
    /// The literal, which the stored continuation names as its step.
    fn as_str(self) -> &'static str {
        match self {
            DiscardHandTurn::This => "this",
            DiscardHandTurn::Next => "next",
        }
    }
}

/// `discardHandAtTurnEnd`'s argument.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DiscardHandAtTurnEndArgs {
    pub turn: DiscardHandTurn,
}

/// B5 E27, R458 (Classic #37 Last Hurrah): the controller discards their whole hand at the end of
/// `this` turn, whoever's (R350), or of their `next` turn (Radiant). A discard (§6.3), so "whenever you
/// discard" sees each card.
pub fn discard_hand_at_turn_end(args: DiscardHandAtTurnEndArgs) -> Effect {
    Effect::new("discardHandAtTurnEnd", move |ctx| {
        let (def_id, radiant) = maker_of(ctx);
        let resume = Resume {
            def_id,
            hook: DELAYED_DISCARD_HAND_HOOK.to_string(),
            step: args.turn.as_str().to_string(),
            radiant,
            instance_id: None,
            data: IndexMap::new(),
        };
        let controller = ctx.controller;
        if args.turn == DiscardHandTurn::This {
            let at = DelayedAt {
                phase: Phase::End,
                player: ctx.sink.state.active,
            };
            schedule_delayed(ctx, controller, at, resume, None, None);
            return;
        }
        let at = DelayedAt {
            phase: Phase::End,
            player: controller,
        };
        let not_before = next_turn_mark(ctx);
        schedule_delayed(ctx, controller, at, resume, None, Some(not_before));
    })
}

fn scope_in(data: &IndexMap<String, Value>) -> Option<BoardScope> {
    match data.get("scope") {
        Some(raw @ Value::Object(_)) => serde_json::from_value::<BoardScope>(raw.clone()).ok(),
        _ => None,
    }
}

/// R750: the mark a delayed destroy of a scope puts on the Units it names, if it was made with one.
fn mark_in(data: &IndexMap<String, Value>) -> Option<CardMark> {
    let Some(Value::Object(raw)) = data.get("mark") else {
        return None;
    };
    match (raw.get("mark"), raw.get("color")) {
        (Some(Value::String(mark)), Some(Value::String(color))) => Some(CardMark {
            mark: mark.clone(),
            color: color.clone(),
        }),
        _ => None,
    }
}

/// R750, R437: every waiting delayed destroy of a scope made with a mark marks the Units its scope names
/// now, as `run_engine_delayed` will read it. Called from `triggers::collect_events`, before
/// `marks::sweep_marks` drops the marks of an effect that is gone.
pub fn refresh_scope_marks(sink: &mut EngineSink<'_>) {
    // Read off the list first: neither the context nor `sync_marks` touches `state.delayed`.
    let waiting: Vec<(String, PlayerId, BoardScope, CardMark)> = sink
        .state
        .delayed
        .iter()
        .filter(|entry| entry.resume.hook == DELAYED_DESTROY_HOOK)
        .filter_map(|entry| {
            let scope = scope_in(&entry.resume.data)?;
            let mark = mark_in(&entry.resume.data)?;
            Some((entry.id.clone(), entry.owner, scope, mark))
        })
        .collect();
    for (id, owner, scope, mark) in waiting {
        let ids: Vec<String> = {
            let ctx = make_context(
                sink,
                None,
                HookOptions {
                    controller: Some(owner),
                    ..HookOptions::default()
                },
            );
            // MD-B1, R940: a harmful walk — an immune card in a tribal scope is passed by.
            cards_in_scope_aimed(&ctx, &scope, crate::wire::TargetAim::Harm)
                .iter()
                .map(|card| card.id.clone())
                .collect()
        };
        sync_marks(sink, &id, &mark, &ids);
    }
}

/// Run one of the engine's own delayed kinds, as its owner. Returns false for an entry that is a card's
/// step, which the caller re-enters through the card's script instead (R126).
pub fn run_engine_delayed(sink: &mut EngineSink<'_>, effect: &DelayedEffect) -> bool {
    let mut ctx = make_context(
        sink,
        None,
        HookOptions {
            controller: Some(effect.owner),
            ..HookOptions::default()
        },
    );
    let data = &effect.resume.data;
    match effect.resume.hook.as_str() {
        DELAYED_DESTROY_HOOK => {
            if let Some(scope) = scope_in(data) {
                (destroy_all(scope).apply)(&mut ctx);
            } else if let Some(instance_id) = data.get("instanceId").and_then(Value::as_str) {
                let target = TargetSpec::Instance {
                    instance_id: instance_id.to_string(),
                };
                (destroy(DestroyArgs { target }).apply)(&mut ctx);
            }
            true
        }
        DELAYED_DISCARD_HAND_HOOK => {
            let discard = discard_hand(DiscardHandArgs {
                player: Some(PlayerSpec::SelfSide),
            });
            (discard.apply)(&mut ctx);
            true
        }
        DELAYED_RADIANT_HOOK => {
            if let Some(instance_id) = data.get("instanceId").and_then(Value::as_str) {
                let target = RadiantTarget {
                    instance_id: Some(instance_id.to_string()),
                    target: None,
                };
                (set_radiant(target).apply)(&mut ctx);
            }
            true
        }
        DELAYED_ALLURE_HOOK => {
            if let Some(instance_id) = data.get("instanceId").and_then(Value::as_str) {
                let target = StealTarget {
                    instance_id: Some(instance_id.to_string()),
                    otherwise: Some(StealOtherwise::Destroy),
                    ..StealTarget::default()
                };
                (steal(target).apply)(&mut ctx);
            }
            true
        }
        _ => false,
    }
}

/// `forRestOfGame`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ForRestOfGameArgs {
    pub step: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hook: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<IndexMap<String, Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    /// R1861002: "at the end of each of your turns" instead of the start (Stinkyscape #39).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at_end: Option<bool>,
}

/// B5 E28, R458 (Classic+ #52): "For the rest of the game: at the start of your turn, …" — the card's
/// `step`, re-entered at each of `player`'s turn starts from the next on, in R62's delayed stage in
/// creation order; with `atEnd` (R1861002) at each end, this one's too if made before its end-of-turn
/// stage. The player's effect, no `self` (R127); `label` is its badge (R169).
pub fn for_rest_of_game(args: ForRestOfGameArgs) -> Effect {
    Effect::new("forRestOfGame", move |ctx| {
        let mut resume = delayed_resume(
            ctx,
            &args.step,
            args.data.clone().unwrap_or_default(),
            args.hook.as_deref().unwrap_or(DELAYED_HOOK),
        );
        resume.instance_id = None;
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        add_start_of_turn_effect(ctx, player, resume, &args.label, args.at_end == Some(true));
    })
}

/// `watchLaneThisTurn`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct WatchLaneArgs {
    pub lane: i32,
    pub step: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hook: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<IndexMap<String, Value>>,
}

/// MD-B22, R946 (Meditative #98, Radiant face): "after you put a card into that lane, …" — a
/// `thisTurn` watcher re-entering the card's `step` (under `hook`, `RESUME_HOOK` by default) after a
/// resolved play or a summon puts one of the controller's cards into one of their zones in `lane`.
pub fn watch_lane_this_turn(args: WatchLaneArgs) -> Effect {
    Effect::new("watchLaneThisTurn", move |ctx| {
        let mut resume = delayed_resume(
            ctx,
            &args.step,
            args.data.clone().unwrap_or_default(),
            args.hook.as_deref().unwrap_or(RESUME_HOOK),
        );
        resume.instance_id = None;
        let player = ctx.controller;
        add_lane_watch(ctx, player, args.lane, resume, &args.label);
    })
}

/// `watchFirstPlays`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct WatchFirstPlaysArgs {
    pub def_ids: Vec<String>,
    pub step: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hook: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<IndexMap<String, Value>>,
}

/// R1863000 (Stinkyscape #44 Great Replacement Theory): "for the rest of the game, the first time you
/// play each …" — a `never` watcher as `watch_lane_this_turn`'s, after the controller's first resolved
/// play (a cast too, R70) of each of `def_ids`, with its `defId` in the data.
pub fn watch_first_plays(args: WatchFirstPlaysArgs) -> Effect {
    Effect::new("watchFirstPlays", move |ctx| {
        let mut resume = delayed_resume(
            ctx,
            &args.step,
            args.data.clone().unwrap_or_default(),
            args.hook.as_deref().unwrap_or(RESUME_HOOK),
        );
        resume.instance_id = None;
        let player = ctx.controller;
        add_play_watch(ctx, player, args.def_ids.clone(), resume, &args.label);
    })
}
