//! The autopilot (Meditative+ #18 Cognitive Offloading, docs/meditative-plus-set.md MP5, R1183002, R1183003):
//! "the AI plays the rest of your turn", played by the engine's §10.7 scorer, never `crates/ai`
//! (CLAUDE.md rule 4: the engine cannot depend on it, and the AI's pick runs on a clock). Core #96 My Pawn's
//! `ai_policy::play_out_turn` is its random cousin.
//!
//! `reduce::end_due_turns` takes `choose`'s action while `due`, until the turn ends, the other player owes
//! an action, or `AUTOPILOT_MAX_ACTIONS` is reached, when the rest is a timeout's (R79). The status ends at
//! cleanup. `choose` is pure in (state, player): every candidate is tried on the player's concealed copy
//! (`scorer::concealed_copy`, R222, R1714000) on the dry run's own seed, so the same state makes the same
//! choice (§9.3). Four steps: (1) a prompt of theirs, the best of the first `AUTOPILOT_ANSWERS_TRIED`
//! answers; (2) a play, in the scorer's order (as M #71's judge scores a play, R1125), the first card with
//! a harmless play; (3) an attack when `Outcome::worth_attacking`; (4) otherwise the turn ends. It never
//! activates an ability, switches a position, emotes, offers or answers a draw or concedes.
//!
//! R1844000 (Stinkyscape #46, #47): the autopilot as an aura, engaged as each of the player's turns begins
//! (`engage_aura`) and read again before every step (`aura_reach`). A count takes R456's actions, never an
//! answer; once spent, the turn is handed back for good (`release_spent_aura`), so a prompt the player's own
//! action opens is theirs.

use crate::config::{AUTOPILOT_ANSWERS_TRIED, SCORER_LOW_HEALTH};
use crate::duos_life::effective_life;
use crate::layers::AuraTable;
use crate::play_choices::PlayAction;
use crate::rng::Rng;
use crate::script::EngineSink;
use crate::state::{Autopilot, GameState, find_instance};
use crate::subsystems::scorer::{
    DRY_RUN_SEED, concealed_copy, dry_run_plays, projected_board_damage, score_instance,
};
use crate::wire::{Action, ActionBody, ActionType, GameEvent, Phase, PlayerId, Winner, opponent_of};

/// R1183002: the autopilot takes `player`'s turn from now to its end: on their own turn only, with no
/// result, once a turn. R1844000: on a turn an aura engaged, the rest of it becomes whole.
pub fn engage(sink: &mut EngineSink<'_>, player: PlayerId) {
    if sink.state.result.is_some() || sink.state.active != player {
        return;
    }
    let turn = sink.state.turn;
    if let Some(status) = sink.state.players[player].autopilot.as_mut()
        && status.turn == turn
    {
        if status.aura_actions.take().is_some() {
            sink.events.push(GameEvent::AutopilotEngaged { player, turn });
        }
        return;
    }
    sink.state.players[player].autopilot = Some(Autopilot {
        turn,
        actions: 0,
        aura_actions: None,
    });
    sink.events.push(GameEvent::AutopilotEngaged { player, turn });
}

/// R1844000: how much of a turn an autopilot aura plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuraReach {
    /// The first this-many R456 actions.
    Actions(i32),
    /// The whole turn.
    Turn,
}

/// R1844000: the reach of the autopilot auras acting on `player`'s field (§3.2): a whole turn when any
/// says so, else the largest count (counts never add up); `None` with neither. The count is printed.
pub fn aura_reach(state: &GameState, player: PlayerId) -> Option<AuraReach> {
    let mut most: Option<i32> = None;
    for card in crate::damage::acting_texts_of(state, player) {
        for text in crate::scripts::texts_of(state, &card) {
            if text.flags.pilots_turn == Some(true) {
                return Some(AuraReach::Turn);
            }
            if let Some(count) = text.flags.pilots_actions.filter(|count| *count > 0) {
                most = Some(most.map_or(count, |held| held.max(count)));
            }
        }
    }
    most.map(AuraReach::Actions)
}

/// R1844000: an autopilot aura takes `player`'s turn as it begins; a card that arrives later in the
/// turn waits for their next.
pub fn engage_aura(sink: &mut EngineSink<'_>, player: PlayerId) {
    if sink.state.result.is_some() || sink.state.active != player || aura_reach(sink.state, player).is_none()
    {
        return;
    }
    let turn = sink.state.turn;
    sink.state.players[player].autopilot = Some(Autopilot {
        turn,
        actions: 0,
        aura_actions: Some(0),
    });
    sink.events.push(GameEvent::AutopilotEngaged { player, turn });
}

/// R1844000: the active player's aura status goes once its count is spent or its aura gone and
/// nothing is open. A status of Cognitive Offloading's (no aura count) is left alone.
pub fn release_spent_aura(state: &mut GameState) {
    let player = state.active;
    let Some(taken) = state.players[player]
        .autopilot
        .and_then(|status| status.aura_actions)
    else {
        return;
    };
    if state.pending.is_some() {
        return;
    }
    let spent = match aura_reach(state, player) {
        None => true,
        Some(AuraReach::Actions(limit)) => taken >= limit,
        Some(AuraReach::Turn) => false,
    };
    if spent {
        state.players[player].autopilot = None;
    }
}

/// Whether the active player's autopilot owes an action now: their main phase with nothing open, or a
/// prompt of theirs open. R1844000: on an aura's turn, only while an aura acts and its count is not
/// spent, or a prompt the last counted action asked is open.
pub fn due(state: &GameState) -> Option<PlayerId> {
    let player = state.active;
    let status = state.players[player].autopilot?;
    if state.result.is_some() || status.turn != state.turn || state.mulligan.is_some() {
        return None;
    }
    if let Some(taken) = status.aura_actions {
        match aura_reach(state, player) {
            None => return None,
            Some(AuraReach::Actions(limit)) if taken >= limit && state.pending.is_none() => return None,
            Some(_) => {}
        }
    }
    match &state.pending {
        Some(pending) => (pending.player_id == player).then_some(player),
        None => (state.phase == Phase::Main).then_some(player),
    }
}

/// R1183003: what one trial left behind, ranked in field order: a won game, then §10.7's three
/// priorities, then the change in the board's balance. A lost game is no outcome at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Outcome {
    won: bool,
    lethal: bool,
    clears: bool,
    heals: bool,
    gain: i32,
}

impl Outcome {
    fn key(self) -> (bool, bool, bool, bool, i32) {
        (self.won, self.lethal, self.clears, self.heals, self.gain)
    }

    /// Strictly better than `other`; ties keep the earlier candidate.
    fn ranks_above(self, other: Outcome) -> bool {
        self.key() > other.key()
    }

    /// A play that does no harm: a priority met, or no part of the balance lost.
    fn harmless(self) -> bool {
        self.won || self.lethal || self.clears || self.heals || self.gain >= 0
    }

    /// An attack worth making: a priority met, or the balance raised.
    fn worth_attacking(self) -> bool {
        self.won || self.lethal || self.clears || self.gain > 0
    }
}

/// R1183003: one side's share of the board — its hero's life and Armor, and every acting Unit's
/// Attack and Health through §10.4's layers.
fn side_value(auras: &AuraTable<'_>, player: PlayerId) -> i32 {
    let state = auras.state();
    let mut value = effective_life(state, player) + crate::damage::hero_armor_of(state, player);
    for unit in crate::zones::active_units_of(state, player) {
        let view = auras.unit_view(unit);
        value += view.attack.max(0) + view.health.max(0);
    }
    value
}

/// The board's balance for `player`: their share less the opponent's.
fn balance(state: &GameState, player: PlayerId) -> i32 {
    let auras = AuraTable::new(state);
    side_value(&auras, player) - side_value(&auras, opponent_of(player))
}

/// R1183003: `body` tried on `base` (a concealed copy) by the reducer's own step and the resolution
/// loop, on the dry run's seed; `None` when it is refused or loses the game.
fn trial(base: &GameState, player: PlayerId, body: &ActionBody) -> Option<Outcome> {
    let enemy = opponent_of(player);
    let enemy_units_before = crate::zones::active_units_of(base, enemy).len();
    let health_before = base.players[player].hero.health;
    let balance_before = balance(base, player);

    let mut copy = base.clone();
    let mut events = Vec::new();
    let mut rng = Rng::new(DRY_RUN_SEED, 0);
    {
        let mut sink = EngineSink::new(&mut copy, &mut events, &mut rng);
        sink.dry_running = true;
        let action = Action::new(body.clone(), player, "autopilot:trial".to_string());
        crate::reduce::apply_action(&mut sink, &action).ok()?;
        crate::triggers::settle(&mut sink, crate::triggers::SettleOptions::default());
    }

    let winner = copy.result.as_ref().map(|result| result.winner);
    if winner.is_some_and(|winner| winner != Winner::from(player)) {
        return None;
    }
    let won = winner == Some(Winner::from(player));
    // R2201000: no lethal against a seat that cannot lose.
    let lethal = !won
        && crate::state::can_lose(&copy, enemy)
        && effective_life(&copy, enemy) <= projected_board_damage(&copy, player);
    let clears = enemy_units_before > 0 && crate::zones::active_units_of(&copy, enemy).is_empty();
    let heals = health_before < SCORER_LOW_HEALTH && copy.players[player].hero.health > health_before;
    let gain = if won {
        0
    } else {
        balance(&copy, player) - balance_before
    };
    Some(Outcome {
        won,
        lethal,
        clears,
        heals,
        gain,
    })
}

/// The best of `candidates` on `base` and its outcome; ties keep the earlier.
fn best_of(base: &GameState, player: PlayerId, candidates: &[ActionBody]) -> Option<(ActionBody, Outcome)> {
    let mut best: Option<(ActionBody, Outcome)> = None;
    for body in candidates {
        let Some(outcome) = trial(base, player, body) else {
            continue;
        };
        if best.as_ref().is_none_or(|(_, held)| outcome.ranks_above(*held)) {
            best = Some((body.clone(), outcome));
        }
    }
    best
}

/// R1183003: the autopilot's next action for `player` (the header's four steps), or `None` when
/// `player` has no action at all.
pub fn choose(state: &GameState, player: PlayerId) -> Option<ActionBody> {
    let actions = crate::reduce::legal_actions(state, player);
    let of_type = |kind: ActionType| -> Vec<ActionBody> {
        actions
            .iter()
            .filter(|action| action.action_type() == kind)
            .cloned()
            .collect()
    };

    if state
        .pending
        .as_ref()
        .is_some_and(|pending| pending.player_id == player)
    {
        let answers: Vec<ActionBody> = of_type(ActionType::Answer)
            .into_iter()
            .take(AUTOPILOT_ANSWERS_TRIED)
            .collect();
        let base = concealed_copy(state, player);
        return best_of(&base, player, &answers)
            .map(|(body, _)| body)
            .or_else(|| answers.first().cloned());
    }

    let plays = of_type(ActionType::Play);
    if !plays.is_empty()
        && let Some(play) = best_play(state, player, &plays)
    {
        return Some(play);
    }

    let attacks = of_type(ActionType::Attack);
    if !attacks.is_empty() {
        let base = concealed_copy(state, player);
        if let Some((attack, outcome)) = best_of(&base, player, &attacks)
            && outcome.worth_attacking()
        {
            return Some(attack);
        }
    }

    of_type(ActionType::EndTurn).into_iter().next()
}

/// R1183003's step 2: the playable cards in the scorer's order, each at its best harmless play.
fn best_play(state: &GameState, player: PlayerId, plays: &[ActionBody]) -> Option<ActionBody> {
    let mut cards: Vec<String> = Vec::new();
    for play in plays {
        if let ActionBody::Play { instance_id, .. } = play
            && !cards.contains(instance_id)
        {
            cards.push(instance_id.clone());
        }
    }
    // R1183003: each card scored as its face states it (no dry-run copy): the tries below are its
    // dry run, so the ranking costs no copy of the board per card.
    let mut ranked: Vec<(String, f64)> = cards
        .into_iter()
        .filter_map(|id| {
            let scored = score_instance(state, player, &id, None)?;
            Some((id, scored.score))
        })
        .collect();
    // Best score first; a stable sort keeps `legal_actions`' order between equals.
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let base = concealed_copy(state, player);
    for (id, _) in ranked {
        let Some(card) = find_instance(state, &id) else {
            continue;
        };
        // The scorer's plays of this card that `legal_actions` lists, else its first listed play.
        let mut tried: Vec<ActionBody> = dry_run_plays(state, player, card)
            .into_iter()
            .map(ActionBody::from)
            .filter(|body| plays.contains(body))
            .collect();
        if tried.is_empty()
            && let Some(first) = plays
                .iter()
                .find(|body| PlayAction::from_body(body).is_some_and(|play| play.instance_id == id))
        {
            tried.push(first.clone());
        }
        if let Some((play, outcome)) = best_of(&base, player, &tried)
            && outcome.harmless()
        {
            return Some(play);
        }
    }
    None
}
