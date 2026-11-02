//! `reduce(state, action)` and `legal_actions` (SPEC §9.3, §10.2, §10.5). The reducer is pure: it
//! clones the state, applies the action and returns the new state with its events; an illegal action
//! comes back as an error with the state untouched.
//!
//! The action layer only: each rule lives in the module that owns it, which gives both what `legal_actions`
//! offers and what an action is refused for, so a greyed-out button and the reducer's error never disagree
//! (§9.3). An attack can come back with its trap window's prompt open and the combat owed on `state.work`
//! (R113); a Frozen unit's switch is refused (§4.1, R1174000); `turn` owns draws, concede, End turn and the
//! turn cap (§2.2, §2.5, R36); a sandbox action is accepted only by a sandbox game and never listed
//! (R923000–R923003).
//!
//! `reduce` and `begin_game` take no rng: it is rebuilt from `(state.seed, state.rng_cursor)` and the
//! cursor written back (SURFACE §4.1, §6.1), and fused scripts come from the state (SURFACE §6.6), so
//! `reduce` never touches its input.

use std::ops::ControlFlow;

use serde::{Deserialize, Serialize};

use crate::combat::{
    AttackTarget, attack_targets_in, declare_attack, position_after_switch, switch_position,
    why_cannot_switch_in,
};
use crate::config::{
    AUTOPILOT_MAX_ACTIONS, MAX_MULLIGAN_SUBSETS, NONCE_HISTORY, TIMEOUT_ANSWER_CAP, TURN_CAP_PLAYER_TURNS,
};
use crate::game_over::end_game;
use crate::layers::AuraTable;
use crate::modifiers::remove_modifier;
use crate::play_choices::{PlayAction, graveyard_play_actions_for, play_actions_for};
use crate::play_steps::run_play_steps;
use crate::prompts::{AnswerInput, answer_prompt, prompt_answers};
use crate::rng::Rng;
use crate::script::EngineSink;
use crate::setup::{answer_mulligan, begin_setup, mulligan_owed, mulligan_prompt_for, why_mulligan_refused};
use crate::state::{
    AppliedAction, CardInstance, EngineError, GameState, ModifierExpiry, ModifierKind, find_instance,
};
use crate::subsystems::activate::{ActivateAction, abilities_of, activate_ability, activate_actions_for};
use crate::subsystems::ai_policy::play_out_turn;
use crate::subsystems::glitch::reset_match;
use crate::subsystems::hero_power::power_ability_of;
use crate::triggers::{SettleOptions, settle};
use crate::turn::{answer_draw, can_offer_draw, concede, end_turn, has_standing_draw_offer, offer_draw};
use crate::wire::{
    Action, ActionBody, ActionType, GameEvent, GameOverReason, NON_ACTIVE_ACTION_TYPES,
    PROMPT_OPEN_ACTION_TYPES, Phase, PlayerId, Row, Winner, opponent_of,
};
use crate::zones::{active_units_of, card_at, carried_units_of, slots_of, standing_units_of};

/// What `reduce` returns. `error` is `Some` exactly when the action was refused; then `state` is the
/// input, unchanged, and `events` is empty.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReduceResult {
    pub state: GameState,
    pub events: Vec<GameEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// A refusal: the input state, no events and the error's message (SURFACE §4.4.9).
fn refused(state: &GameState, error: &str) -> ReduceResult {
    ReduceResult {
        state: state.clone(),
        events: Vec::new(),
        error: Some(error.to_string()),
    }
}

/// R265: what may be sent while the mulligans are open — a seat's own mulligan, and the actions that
/// end a game or answer for a seat whose clock ran out (R79, R268), exactly as while a prompt is open.
const MULLIGAN_OPEN_ACTION_TYPES: &[ActionType] = &[
    ActionType::Mulligan,
    ActionType::Concede,
    ActionType::SetAutoEndTurn,
    ActionType::Timeout,
    ActionType::DisconnectExpired,
    ActionType::CeilingReached,
];

/// The seat the game waits on first: the open prompt's holder, else the first seat still owing its
/// mulligan (R265), else the active player (always `Some`). For a harness playing both seats; a live
/// table asks each seat's own `legal_actions`, since both may owe a mulligan at once.
pub fn seat_to_act(state: &GameState) -> Option<PlayerId> {
    if let Some(pending) = &state.pending {
        return Some(pending.player_id);
    }
    if let Some(owed) = mulligan_owed(state).first() {
        return Some(*owed);
    }
    Some(state.active)
}

/// §4.2: a target names an enemy unit by instance id, or an enemy hero as `hero-<player>`.
pub fn attack_target_id(target: &AttackTarget) -> String {
    match target {
        AttackTarget::Unit { instance, .. } => instance.id.clone(),
        AttackTarget::Hero { player, .. } => format!("hero-{player}"),
    }
}

/// The attacker an `attack` action names: any card in this player's unit piles, a dormant one included
/// so §3.2's "not on the field" is `combat`'s refusal (R13), then a Unit a carrier holds or standing in
/// a backrow zone.
fn attacker_of(state: &GameState, player: PlayerId, instance_id: &str) -> Option<CardInstance> {
    // R446: a Unit a carrier holds is one of the player's units too, which `combat` refuses to attack.
    let in_piles = state.players[player]
        .units
        .iter()
        .flatten()
        .flatten()
        .find(|card| card.id == instance_id)
        .cloned();
    if in_piles.is_some() {
        return in_piles;
    }
    let carried = carried_units_of(state, player)
        .into_iter()
        .find(|card| card.id == instance_id)
        .cloned();
    if carried.is_some() {
        return carried;
    }
    // R1081000: and a Unit standing in a backrow zone, which attacks and switches as any unit does.
    standing_units_of(state, player)
        .into_iter()
        .find(|card| card.id == instance_id)
        .cloned()
}

/// The target an `attack` action names, among the enemy's units on the field (a Dormant one too, so
/// "that card is Dormant" is `combat`'s refusal, R1839000) and the enemy hero only, so a friendly
/// target never reaches the validator (§4.2 step 2).
fn attack_target_of(state: &GameState, player: PlayerId, target_id: &str) -> Option<AttackTarget> {
    let enemy = opponent_of(player);
    if target_id == format!("hero-{enemy}") {
        return Some(AttackTarget::Hero { player: enemy });
    }
    let unit = crate::zones::units_on_field_of(state, enemy)
        .into_iter()
        .find(|card| card.id == target_id)
        .cloned()?;
    Some(AttackTarget::Unit { instance: unit })
}

fn attack(
    sink: &mut EngineSink<'_>,
    player: PlayerId,
    attacker_id: &str,
    target_id: &str,
) -> Result<(), EngineError> {
    let Some(attacker) = attacker_of(sink.state, player, attacker_id) else {
        return Err(EngineError::new(format!("no unit {attacker_id} you control")));
    };

    // R1200: while a Mayor acts the attack names the sentinel `"random"` and the engine draws the
    // target; the sentinel with no Mayor, or a named target with one, is refused.
    if target_id == crate::config::RANDOM_ATTACK_TARGET {
        if !crate::random_targets::targets_random(sink.state) {
            return Err(EngineError::new(format!("no target {target_id}")));
        }
        let Some(target) = crate::random_targets::draw_attack_target(sink.state, &mut *sink.rng, &attacker)
        else {
            return Err(EngineError::new(format!("no target {target_id}")));
        };
        return declare_attack(sink, &attacker, &target).map(|_| ());
    }
    if crate::random_targets::targets_random(sink.state) {
        return Err(EngineError::new("targets are drawn at random while a Mayor acts"));
    }

    let Some(target) = attack_target_of(sink.state, player, target_id) else {
        return Err(EngineError::new(format!("no target {target_id}")));
    };

    declare_attack(sink, &attacker, &target).map(|_| ())
}

/// §4.1: the player's own switch, which spends the unit's exertion (R20 is the effect's version).
fn switch_action(sink: &mut EngineSink<'_>, player: PlayerId, instance_id: &str) -> Result<(), EngineError> {
    let Some(unit) = attacker_of(sink.state, player, instance_id) else {
        return Err(EngineError::new(format!("no unit {instance_id} you control")));
    };
    switch_position(sink, &unit, Default::default()).map(|_| ())
}

/// B3.2 rule 10, R384, R752: `activate` and `activatePower`, the alias old logs carry, both go to
/// `activate_ability`, since a Heroic Power's power is one of its Activate abilities. An
/// `activatePower` names no ability: on a Heroic Power it is the rolled power, on any other card its
/// only ability, as for an `activate` naming none.
fn activate_card(
    sink: &mut EngineSink<'_>,
    player: PlayerId,
    action: &ActionBody,
) -> Result<(), EngineError> {
    let (instance_id, mut named, targets, modes, tributes, is_power) = match action {
        ActionBody::Activate {
            instance_id,
            ability,
            targets,
            modes,
            tributes,
        } => (
            instance_id.clone(),
            ability.clone(),
            targets.clone(),
            modes.clone(),
            tributes.clone(),
            false,
        ),
        ActionBody::ActivatePower { instance_id, targets } => {
            (instance_id.clone(), None, targets.clone(), None, None, true)
        }
        _ => return Err(EngineError::new("unknown action")),
    };
    let card = find_instance(sink.state, &instance_id).cloned();
    if is_power && let Some(card) = &card {
        named = power_ability_of(sink.state, card).map(|ability| ability.id.to_string());
    }
    let activation = ActivateAction {
        instance_id,
        ability: named,
        targets,
        modes,
        tributes,
    };
    activate_ability(sink, player, &activation).map(|_| ())
}

/// R384, R752: what `legal_actions` offers for one card acting on the field: every usable "Activate:"
/// ability, a Heroic Power's rolled power among them, once per target it may be dragged to (R81).
fn activation_actions(state: &GameState, player: PlayerId, card: &CardInstance) -> Vec<ActionBody> {
    activate_actions_for(state, player, card)
        .into_iter()
        .map(ActionBody::from)
        .collect()
}

/// §4.1 and R49: whether this unit's own switch is on offer, by the refusal `switch_position` gives,
/// so the list and the reducer cannot disagree.
fn can_switch(auras: &AuraTable<'_>, unit: &CardInstance) -> bool {
    why_cannot_switch_in(auras, unit, position_after_switch(unit), true).is_ok()
}

/// One action applied to the sink's state as `reduce` applies it, before the resolution loop: the
/// reducer's own step, and the autopilot's trials' (R1183003) on their copies.
pub(crate) fn apply_action(sink: &mut EngineSink<'_>, action: &Action) -> Result<(), EngineError> {
    match &action.body {
        ActionBody::Mulligan { keep } => {
            // R265: each seat answers its own mulligan, in either order; the answer is sealed (R266).
            why_mulligan_refused(sink.state, action.player_id, keep)?;
            answer_mulligan(sink, action.player_id, keep);
            Ok(())
        }
        ActionBody::Play { .. } => {
            let play = PlayAction::from_body(&action.body).expect("a play action body is a PlayAction");
            // MD-D28, R1125: judged on the pre-play state, ahead of §10.5 step 1, from the player's
            // own concealed view.
            if crate::subsystems::pareto::watching(sink.state, action.player_id) {
                let optimal =
                    crate::subsystems::pareto::judge_play(sink.state, action.player_id, &play.instance_id);
                let turn = sink.state.turn;
                sink.state.play_judgement = Some(crate::state::PlayJudgement {
                    instance_id: play.instance_id.clone(),
                    optimal,
                    turn,
                });
            }
            run_play_steps(sink, action.player_id, &play).map(|_| ())
        }
        ActionBody::Emote { emote } => {
            // MD-D29, R1127: an emote reaches the board only while a card hears it.
            if !crate::query::emotes_heard(sink.state, action.player_id) {
                return Err(EngineError::new("no card hears emotes"));
            }
            sink.events.push(GameEvent::Emoted {
                player: action.player_id,
                emote: *emote,
            });
            Ok(())
        }
        ActionBody::SwitchPosition { instance_id } => switch_action(sink, action.player_id, instance_id),
        ActionBody::Attack {
            attacker_id,
            target_id,
        } => attack(sink, action.player_id, attacker_id, target_id),
        ActionBody::Activate { .. } | ActionBody::ActivatePower { .. } => {
            // R43, R384: an ability lives on the instance; its module owns its costs, choices and use.
            activate_card(sink, action.player_id, &action.body)
        }
        ActionBody::Answer { choice_id, selection } => {
            // §10.6: a card's continuation re-enters through its script; a prompt an engine sequence
            // opened for itself (an Echo repeat's pick, §10.5 step 6) goes to its answerer (R122).
            let answer = AnswerInput {
                player_id: action.player_id,
                choice_id: choice_id.clone(),
                selection: selection.clone(),
            };
            answer_prompt(sink, &answer).map(|_| ())
        }
        ActionBody::OfferDraw => {
            if !can_offer_draw(sink.state, action.player_id) {
                return Err(EngineError::new("you cannot offer a draw right now"));
            }
            offer_draw(sink, action.player_id);
            Ok(())
        }
        ActionBody::AnswerDraw { accept } => {
            if !has_standing_draw_offer(sink.state, action.player_id) {
                return Err(EngineError::new("there is no draw offer to answer"));
            }
            answer_draw(sink, action.player_id, *accept);
            Ok(())
        }
        ActionBody::Concede => {
            concede(sink, action.player_id);
            Ok(())
        }
        // R923000: a practice-sandbox tool, refused outside a sandbox game (`sandbox.rs` owns the rule).
        ActionBody::Sandbox { op } => crate::sandbox::apply_sandbox(sink, op),
        ActionBody::EndTurn => {
            end_turn(sink);
            Ok(())
        }
        // R1838000: `trade.rs` owns the rule; its refusal is the one `legal_actions` and the view read.
        ActionBody::Trade { instance_id } => crate::trade::apply_trade(sink, action.player_id, instance_id),
        // R1853003: `marketplace.rs` owns the rule; `legal_actions` and the view read its refusals.
        ActionBody::MarketBuy { offer } => {
            crate::marketplace::apply_market_buy(sink, action.player_id, *offer)
        }
        ActionBody::MarketSell { instance_id } => {
            crate::marketplace::apply_market_sell(sink, action.player_id, instance_id)
        }
        ActionBody::SetAutoEndTurn { enabled } => {
            // R345: a preference, not a move, emitting nothing; `end_due_turns` reads it after this
            // reduction too, so turning it back on with nothing left to do ends the turn at once.
            sink.state.players[action.player_id].auto_end_turn = if *enabled { None } else { Some(false) };
            Ok(())
        }
        ActionBody::Timeout => timeout(sink, action),
        ActionBody::DisconnectExpired { player } => {
            end_game(
                sink,
                Winner::from(opponent_of(*player)),
                GameOverReason::Disconnect,
            );
            Ok(())
        }
        ActionBody::CeilingReached => {
            // R79: past the hard wall-clock ceiling the match is a draw.
            end_game(sink, Winner::Draw, GameOverReason::MatchCeiling);
            Ok(())
        }
    }
}

/// R79 and §2.5: `timeout` "answers only the prompts of the player whose clock ran out, using the AI
/// policy, and ends the turn only when that is the active player", as deterministic as any action.
/// The non-active player's clock is a prompt clock: it answers that one prompt of theirs, or nothing.
/// The active player's is the turn clock: every prompt of theirs is answered, then the turn ends; it stops
/// at a prompt the other player holds (that one has its own clock) and once the turn has passed.
fn timeout(sink: &mut EngineSink<'_>, action: &Action) -> Result<(), EngineError> {
    let who = action.player_id;
    let turn = sink.state.turn;
    let turn_clock = who == sink.state.active;

    // R268: while the mulligans are open (R265) the clock that ran out is the mulligan's: it keeps this
    // seat's whole hand, as Hearthstone confirms the hand when its mulligan timer runs out, drawing
    // nothing from the rng and never answering the other seat's.
    if sink.state.pending.is_none() && sink.state.mulligan.is_some() {
        let keep: Option<Vec<String>> = mulligan_prompt_for(sink.state, who)
            .map(|prompt| prompt.options.iter().map(|option| option.key.clone()).collect());
        if let Some(keep) = keep {
            answer_mulligan(sink, who, &keep);
        }
        return Ok(());
    }

    for _step in 0..TIMEOUT_ANSWER_CAP {
        if sink.state.result.is_some() || sink.state.turn != turn {
            return Ok(());
        }

        if let Some(holder) = sink.state.pending.as_ref().map(|pending| pending.player_id) {
            if holder != who {
                return Ok(());
            }
            // R79: the AI policy answers and never concedes (R84), so it draws from the prompt's own
            // answers, not the concede R211 also offers. R1002: a timeout leaves a night market,
            // drawing nothing; R1184001: it stops a question sequence the same way.
            let quit = crate::subsystems::night_market::leave_answer(sink.state)
                .or_else(|| crate::prompts::stop_answer(sink.state));
            let pick = if let Some(quit) = quit {
                quit
            } else {
                let answers: Vec<ActionBody> = legal_actions(sink.state, who)
                    .into_iter()
                    .filter(|body| body.action_type() != ActionType::Concede)
                    .collect();
                let index = sink.rng.int(answers.len() as i32);
                let Some(pick) = answers.get(index.max(0) as usize).cloned() else {
                    return Ok(());
                };
                pick
            };
            apply_action(sink, &Action::new(pick, who, action.nonce.clone()))?;
            if !turn_clock {
                return Ok(());
            }
            // The answer's own resolution loop, so a prompt it leads to is open before the next look.
            settle(sink, SettleOptions::default());
            continue;
        }

        if !turn_clock || sink.state.active != who || sink.state.phase != Phase::Main {
            return Ok(());
        }
        end_turn(sink);
        settle(sink, SettleOptions::default());
    }
    Ok(())
}

/// R44, §8 #96: "an AI plays the rest of their turn with random legal actions", the player locked out
/// while `aiTurn` is set (R152). A question of theirs can still open outside the AI's playout (a Death
/// a trap fires in the other player's answer); it is the AI's to answer, as every prompt of that turn
/// is (§10.7), and the answer finishes the turn the AI owes, which would otherwise stall (R79).
fn answer_for_locked_out(sink: &mut EngineSink<'_>) {
    for _guard in 0..=TURN_CAP_PLAYER_TURNS {
        if sink.state.result.is_some() {
            return;
        }
        let Some(holder) = sink.state.pending.as_ref().map(|pending| pending.player_id) else {
            return;
        };
        if !sink.state.players[holder].ai_turn {
            return;
        }
        if play_out_turn(sink, holder, Default::default()).actions.is_empty() {
            return;
        }
    }
}

/// B5 E10, R456: the main-phase actions "one more action, then your turn ends" counts; an answer is
/// part of the action that asked, and ending the turn uses the rest up.
const TURN_ACTION_TYPES: &[ActionType] = &[
    ActionType::Play,
    ActionType::Attack,
    ActionType::SwitchPosition,
    ActionType::Activate,
    ActionType::ActivatePower,
];

/// The fields of a `turnEnds` modifier this module reads.
struct TurnEndsRider {
    id: String,
    actions_left: i32,
    by_instance_id: Option<String>,
}

/// B5 E10, R456: the "your turn ends" rider on `player` for this turn, or `None`: the first `turnEnds`
/// modifier whose `thisTurn` expiry, if any, is this turn. A private copy of `modifiers::turn_ends_of`
/// (fullsend rule 5).
fn turn_ends_rider(state: &GameState, player: PlayerId) -> Option<TurnEndsRider> {
    for modifier in &state.players[player].mods {
        let ModifierKind::TurnEnds {
            actions_left,
            by_instance_id,
        } = &modifier.kind
        else {
            continue;
        };
        if let ModifierExpiry::ThisTurn { turn } = modifier.expiry
            && turn != state.turn
        {
            continue;
        }
        return Some(TurnEndsRider {
            id: modifier.id.clone(),
            actions_left: *actions_left,
            by_instance_id: by_instance_id.clone(),
        });
    }
    None
}

/// B5 E10, R456: the rider an action counts against: the acting player's own, with actions left, as it
/// stood before the action, so one the action puts in place (Classic+ #26 Radiant drawn by it) counts
/// from the next action on.
fn turn_action_counted(state: &GameState, action: &Action) -> Option<String> {
    if !TURN_ACTION_TYPES.contains(&action.action_type()) {
        return None;
    }
    if action.player_id != state.active {
        return None;
    }
    let rider = turn_ends_rider(state, action.player_id)?;
    if rider.actions_left <= 0 {
        None
    } else {
        Some(rider.id)
    }
}

/// R456: spend one of the actions a rider leaves, once the action it counted was accepted.
fn count_turn_action(state: &mut GameState, player: PlayerId, rider_id: &str) {
    let Some(rider) = turn_ends_rider(state, player) else {
        return;
    };
    if rider.id != rider_id || rider.actions_left <= 0 {
        return;
    }
    for modifier in state.players[player].mods.iter_mut() {
        if modifier.id != rider.id {
            continue;
        }
        if let ModifierKind::TurnEnds { actions_left, .. } = &mut modifier.kind {
            *actions_left -= 1;
            return;
        }
    }
}

/// B5 E10, R456: the rider of an active turn an effect cut short, once it has no actions left, no
/// prompt is open and the turn is in its main phase (one set at a turn's start, a cast on draw's,
/// waits for it).
fn turn_cut_due(state: &GameState) -> Option<TurnEndsRider> {
    if state.result.is_some() || state.pending.is_some() || state.phase != Phase::Main {
        return None;
    }
    let rider = turn_ends_rider(state, state.active)?;
    if rider.actions_left <= 0 {
        Some(rider)
    } else {
        None
    }
}

/// R82, R456: end every turn due to end once the action has resolved: one an effect cut short, and one
/// with nothing but ending it left to do unless its player turned that off (R345); either may start another
/// such turn (a Tommy Tempo drawn at its start), hence the loop. R1844000: first an aura's spent count hands
/// the turn back; R1183002: then a turn the autopilot owes is played, unless `drive_autopilot` is off (in a
/// search's world, and in the autopilot's own steps so none plays a turn twice).
fn end_due_turns(sink: &mut EngineSink<'_>, drive_autopilot: bool) {
    for _guard in 0..=TURN_CAP_PLAYER_TURNS {
        crate::subsystems::autopilot::release_spent_aura(sink.state);
        if drive_autopilot && let Some(player) = crate::subsystems::autopilot::due(sink.state) {
            run_autopilot(sink, player);
            continue;
        }
        if let Some(cut) = turn_cut_due(sink.state) {
            let player = sink.state.active;
            remove_modifier(sink, player, &cut.id);
            sink.events.push(GameEvent::TurnCutShort {
                player,
                by_instance_id: cut.by_instance_id,
            });
            end_turn(sink);
            settle(sink, SettleOptions::default());
            continue;
        }
        if !auto_end_due(sink.state) {
            return;
        }
        let (player, turn) = (sink.state.active, sink.state.turn);
        sink.events.push(GameEvent::TurnAutoEnded { player, turn });
        end_turn(sink);
        settle(sink, SettleOptions::default());
    }
}

/// R1183002, R1183003: the autopilot plays `player`'s turn while it is due, one action at a time
/// through `autopilot_step`; at `AUTOPILOT_MAX_ACTIONS`, or if a chosen action is refused, the rest is
/// a timeout's (R79). Returns once the turn has ended, the game is over, the other player owes an
/// answer or an aura's count is spent (R1844000).
fn run_autopilot(sink: &mut EngineSink<'_>, player: PlayerId) {
    let turn = sink.state.turn;
    while sink.state.turn == turn && crate::subsystems::autopilot::due(sink.state) == Some(player) {
        let Some(status) = sink.state.players[player].autopilot else {
            return;
        };
        let nonce = format!("autopilot:{player}:t{turn}:a{}", status.actions);
        let chosen = if status.actions < AUTOPILOT_MAX_ACTIONS {
            crate::subsystems::autopilot::choose(sink.state, player)
        } else {
            None
        };
        let Some(body) = chosen else {
            let _ = timeout(sink, &Action::new(ActionBody::Timeout, player, nonce));
            return;
        };
        let kind = body.action_type();
        if let Some(status) = sink.state.players[player].autopilot.as_mut() {
            status.actions += 1;
            // R1844000: an aura's count takes R456's actions; an answer is part of the one that asked.
            if let Some(taken) = status.aura_actions.as_mut()
                && TURN_ACTION_TYPES.contains(&kind)
            {
                *taken += 1;
            }
        }
        if autopilot_step(sink, &Action::new(body, player, nonce.clone())).is_err() {
            let _ = timeout(sink, &Action::new(ActionBody::Timeout, player, nonce));
            return;
        }
    }
}

/// R1183002: one autopilot action as `reduce` takes a player's (applied, counted against R456's rider,
/// settled, due turns ended) but with no second state copy or nonce in the log: it is part of the
/// action that handed the turn over, and its events are that one's. A refused one changes nothing.
fn autopilot_step(sink: &mut EngineSink<'_>, action: &Action) -> Result<(), EngineError> {
    let (saved, events_at, rng) = (sink.state.clone(), sink.events.len(), sink.rng.clone());
    let counted = turn_action_counted(sink.state, action);
    if let Err(error) = apply_action(sink, action) {
        *sink.state = saved;
        sink.events.truncate(events_at);
        *sink.rng = rng;
        return Err(error);
    }
    if let Some(rider_id) = counted {
        count_turn_action(sink.state, action.player_id, &rider_id);
    }
    settle(sink, SettleOptions::default());
    answer_for_locked_out(sink);
    end_due_turns(sink, false);
    Ok(())
}

/// §2.5, R82: when nothing but ending the turn is left, the turn ends by itself, unless the active
/// player turned that off (R345). The walk stops at the first other action: this runs after every
/// action, the AI's simulated ones included, and listing every play cost a third of a gate game (#188).
fn auto_end_due(state: &GameState) -> bool {
    if state.result.is_some() || state.pending.is_some() || state.phase != Phase::Main {
        return false;
    }
    let player = state.active;
    if state.players[player].auto_end_turn == Some(false) {
        return false;
    }
    let walk = each_legal_action(state, player, &mut |action| match action.action_type() {
        ActionType::EndTurn | ActionType::Concede | ActionType::OfferDraw | ActionType::Emote => {
            ControlFlow::Continue(())
        }
        _ => ControlFlow::Break(()),
    });
    walk.is_continue()
}

fn remember_nonce(state: &mut GameState, nonce: &str, events: &[GameEvent]) {
    state.applied.push(AppliedAction {
        nonce: nonce.to_string(),
        events: events.to_vec(),
    });
    if state.applied.len() > NONCE_HISTORY {
        let excess = state.applied.len() - NONCE_HISTORY;
        state.applied.drain(0..excess);
    }
}

/// §9.3, §10.2: apply one action. A refused action comes back with the input state, no events and
/// its message; a replayed nonce comes back with the events it produced the first time.
pub fn reduce(state: &GameState, action: &Action) -> ReduceResult {
    reduce_driving(state, action, true)
}

/// `reduce` for a search's own world (§9, `crates/ai`'s simulated steps): a turn the autopilot owes
/// (R1183002) is left owed, not played: driven, one simulated End turn whose next draw casts Cognitive
/// Offloading would play that whole turn by §10.7's scorer inside one node (#1337). A game, and every
/// replay of it, goes through `reduce`.
pub fn reduce_unpiloted(state: &GameState, action: &Action) -> ReduceResult {
    reduce_driving(state, action, false)
}

/// `reduce` and `reduce_unpiloted`: `drive_autopilot` is whether a turn the autopilot owes is played.
/// R2201001: the step runs under its game's pool hold.
fn reduce_driving(state: &GameState, action: &Action, drive_autopilot: bool) -> ReduceResult {
    crate::catalog::with_pool_hold(state, || reduce_held(state, action, drive_autopilot))
}

fn reduce_held(state: &GameState, action: &Action, drive_autopilot: bool) -> ReduceResult {
    if let Some(previous) = state.applied.iter().find(|entry| entry.nonce == action.nonce) {
        return ReduceResult {
            state: state.clone(),
            events: previous.events.clone(),
            error: None,
        };
    }

    if state.result.is_some() {
        return refused(state, "the game is over");
    }

    let action_type = action.action_type();

    // A prompt blocks every action but its own answer and the ones that end a game (§9.3, R79).
    if let Some(pending) = &state.pending {
        let allowed = PROMPT_OPEN_ACTION_TYPES.contains(&action_type);
        if !allowed {
            return refused(state, "a prompt is open: answer it first");
        }
        let is_answer = action_type == ActionType::Answer || action_type == ActionType::Mulligan;
        if is_answer && pending.player_id != action.player_id {
            return refused(state, "that prompt belongs to the other player");
        }
    }

    // R265: while the mulligans are open both seats owe one, so neither is "not on turn" — setup is no
    // player's turn (§2.1) — and nothing but a mulligan and the actions that end a game moves.
    let mulligan_open = state.pending.is_none() && state.mulligan.is_some();
    if mulligan_open && !MULLIGAN_OPEN_ACTION_TYPES.contains(&action_type) {
        return refused(state, "the mulligan is open: answer it first");
    }

    let non_active = action.player_id != state.active;
    if non_active
        && !mulligan_open
        && !NON_ACTIVE_ACTION_TYPES.contains(&action_type)
        && state.pending.as_ref().map(|pending| pending.player_id) != Some(action.player_id)
    {
        return refused(state, "it is not your turn");
    }

    let mut next = state.clone();
    // R179: the state's fused scripts, composed on entry when it holds none (it came through JSON); a
    // Fuse composes the ones it mints as it mints them.
    crate::scripts::sync_fused_scripts(&mut next);
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&next.seed, next.rng_cursor);
    {
        let mut sink = EngineSink::new(&mut next, &mut events, &mut rng);

        let counted = turn_action_counted(sink.state, action);
        if let Err(error) = apply_action(&mut sink, action) {
            return refused(state, &error.message);
        }
        if let Some(rider_id) = counted {
            count_turn_action(sink.state, action.player_id, &rider_id);
        }

        // §10.3: the resolution loop finishes the action and stops where a prompt is waiting.
        settle(&mut sink, SettleOptions::default());
        answer_for_locked_out(&mut sink);
        end_due_turns(&mut sink, drive_autopilot);
        // R676: a Glitch's reset goes once the action that drew it has settled.
        if sink.state.reset_owed == Some(true) {
            reset_match(&mut sink);
        }
    }

    next.rng_cursor = rng.cursor();
    remember_nonce(&mut next, &action.nonce, &events);
    ReduceResult {
        state: next,
        events,
        error: None,
    }
}

/// Start the game: shuffle, deal and open both mulligans (§2.1, R265).
pub fn begin_game(state: &GameState) -> ReduceResult {
    // R2201001: setup runs under its game's pool hold, as every step does.
    crate::catalog::with_pool_hold(state, || begin_held(state))
}

fn begin_held(state: &GameState) -> ReduceResult {
    let mut next = state.clone();
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&next.seed, next.rng_cursor);
    {
        let mut sink = EngineSink::new(&mut next, &mut events, &mut rng);
        begin_setup(&mut sink);
    }
    next.rng_cursor = rng.cursor();
    ReduceResult {
        state: next,
        events,
        error: None,
    }
}

fn mulligan_subsets(ids: &[String]) -> Vec<Vec<String>> {
    // `2 ** ids.length`, which past 63 cards is past any cap anyway.
    let total: u64 = 1u64.checked_shl(ids.len() as u32).unwrap_or(u64::MAX);
    if ids.len() >= 64 || total > MAX_MULLIGAN_SUBSETS as u64 {
        return vec![ids.to_vec(), Vec::new()];
    }
    let mut out: Vec<Vec<String>> = Vec::new();
    for mask in 0..total {
        out.push(
            ids.iter()
                .enumerate()
                .filter(|(i, _)| mask & (1u64 << i) != 0)
                .map(|(_, id)| id.clone())
                .collect(),
        );
    }
    out
}

/// Every action that would not error, for the client's greying-out and for the AI policy (§10.2), each
/// kind from the module that refuses it (a play's R81 choice kinds crossed and bounded, R90), never
/// from a second copy of the rule here.
pub fn legal_actions(state: &GameState, player: PlayerId) -> Vec<ActionBody> {
    let mut out: Vec<ActionBody> = Vec::new();
    let _ = each_legal_action(state, player, &mut |action| {
        out.push(action);
        ControlFlow::Continue(())
    });
    out
}

/// `legal_actions` one action at a time, in the same order: the one place the list is made, so a
/// caller that only asks whether some action exists (`auto_end_due`) stops computing at the first:
/// a `Break` from `visit` ends the walk there.
pub(crate) fn each_legal_action(
    state: &GameState,
    player: PlayerId,
    visit: &mut dyn FnMut(ActionBody) -> ControlFlow<()>,
) -> ControlFlow<()> {
    if state.result.is_some() {
        return ControlFlow::Continue(());
    }

    if let Some(pending) = &state.pending {
        // R211: `reduce` accepts a concede from either seat while a prompt is open (§2.5, BUILD M1-T3),
        // so both are offered it; §10.7's policy never takes it (R84), drawing from the answers alone.
        if pending.player_id != player {
            return visit(ActionBody::Concede);
        }
        for answer in prompt_answers(pending) {
            visit(answer)?;
        }
        return visit(ActionBody::Concede);
    }

    if state.mulligan.is_some() {
        // R265: both mulligans are open at once. A seat that still owes one is offered its answers and
        // concede (R211); a seat that has answered waits for the other, with concede alone.
        let keys: Option<Vec<String>> = mulligan_prompt_for(state, player)
            .map(|mulligan| mulligan.options.iter().map(|option| option.key.clone()).collect());
        let Some(keys) = keys else {
            return visit(ActionBody::Concede);
        };
        for keep in mulligan_subsets(&keys) {
            visit(ActionBody::Mulligan { keep })?;
        }
        return visit(ActionBody::Concede);
    }

    if has_standing_draw_offer(state, player) {
        visit(ActionBody::AnswerDraw { accept: true })?;
        visit(ActionBody::AnswerDraw { accept: false })?;
    }

    // MD-D29, R1127: either seat may emote while a card hears it, never with a prompt or mulligan
    // open; otherwise `reduce` refuses it.
    if crate::query::emotes_heard(state, player) {
        for emote in crate::wire::EMOTE_IDS.iter() {
            visit(ActionBody::Emote { emote: *emote })?;
        }
    }

    if state.active != player || state.phase != Phase::Main {
        return visit(ActionBody::Concede);
    }

    let side = &state.players[player];
    for card in &side.hand {
        for play in play_actions_for(state, player, card) {
            visit(ActionBody::from(play))?;
        }
    }
    // B5 E11, R454: a graveyard card a permission on their field lets them play, as a `play` action.
    for card in &side.graveyard {
        for play in graveyard_play_actions_for(state, player, card) {
            visit(ActionBody::from(play))?;
        }
    }

    // R1200: while a Mayor acts each attack is listed once per attacker with no target — the
    // reducer draws it — so the client asks for none.
    let random = crate::random_targets::targets_random(state);
    // #892: one aura table for every attacker's targets and switch.
    let auras = AuraTable::new(state);
    for unit in active_units_of(state, player) {
        if random {
            if !attack_targets_in(&auras, unit).is_empty() {
                visit(ActionBody::Attack {
                    attacker_id: unit.id.clone(),
                    target_id: crate::config::RANDOM_ATTACK_TARGET.to_string(),
                })?;
            }
        } else {
            for target in attack_targets_in(&auras, unit) {
                visit(ActionBody::Attack {
                    attacker_id: unit.id.clone(),
                    target_id: attack_target_id(&target),
                })?;
            }
        }
        if can_switch(&auras, unit) {
            visit(ActionBody::SwitchPosition {
                instance_id: unit.id.clone(),
            })?;
        }
    }

    // R43, R384: a power or an ability is the instance's, so every card the player has acting on the
    // field is asked — the top of each unit pile, then the backrow, lane by lane.
    for row in [Row::Units, Row::Backrow] {
        for zone_ref in slots_of(player, row) {
            if let Some(card) = card_at(state, zone_ref) {
                for activation in activation_actions(state, player, card) {
                    visit(activation)?;
                }
            }
        }
    }

    // MD-D9: an ability either player may use is offered off the opponent's cards too; a card with none
    // is skipped before its choices are built, so a shipped card lists nothing new here.
    for row in [Row::Units, Row::Backrow] {
        for zone_ref in slots_of(opponent_of(player), row) {
            let Some(card) = card_at(state, zone_ref) else {
                continue;
            };
            let open = abilities_of(state, card)
                .iter()
                .any(|decl| decl.cost.is_some_and(|cost| cost.either_player == Some(true)));
            if !open {
                continue;
            }
            for activation in activation_actions(state, player, card) {
                visit(activation)?;
            }
        }
    }

    // R1838000: a Trade of each Tradable hand card, in hand order, while `trade.why_trade_refused`
    // passes it — the refusal the reducer gives and the view's `tradeReason` reads.
    for card in &side.hand {
        if crate::trade::why_trade_refused(state, player, &card.id).is_ok() {
            visit(ActionBody::Trade {
                instance_id: card.id.clone(),
            })?;
        }
    }
    // R1853003: a buy of each lot offered to the player, in offer order (R2250002), then a sale of each
    // hand card that sells from there (R1861000), in hand order, while `marketplace.rs`'s refusals (the
    // view's `reason`) pass them. A closed Marketplace lists neither.
    if let Some(market) = &state.marketplace {
        let offered = crate::marketplace::offered_to(market, player).count() as i32;
        for offer in 0..offered {
            if crate::marketplace::why_market_buy_refused(state, player, offer).is_ok() {
                visit(ActionBody::MarketBuy { offer })?;
            }
        }
        let hand_sales = side
            .hand
            .iter()
            .filter(|card| crate::marketplace::sells_from_hand(state, card));
        for card in hand_sales {
            if crate::marketplace::why_market_sell_refused(state, player, &card.id).is_ok() {
                visit(ActionBody::MarketSell {
                    instance_id: card.id.clone(),
                })?;
            }
        }
    }

    if can_offer_draw(state, player) {
        visit(ActionBody::OfferDraw)?;
    }

    visit(ActionBody::EndTurn)?;
    visit(ActionBody::Concede)
}
