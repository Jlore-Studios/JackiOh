//! The autopilot (Meditative+ #18 Cognitive Offloading, docs/meditative-plus-set.md MP5, R1183002,
//! R1183003): "the AI plays the rest of your turn", played by the engine's §10.7 scorer, never
//! `crates/ai` (CLAUDE.md rule 4: the engine cannot depend on it, and the AI's own pick runs on a
//! budget, a wall clock and a determinization rng, so the server, the browser and a replay could
//! disagree). Core #96 My Pawn's `ai_policy::play_out_turn` is its random cousin.
//!
//! The `autopilot` verb sets `PlayerState.autopilot` on its caster for the turn running, and only on
//! the caster's own turn (R1183002). `reduce` drives it (`reduce::end_due_turns`): once an action has
//! settled, while the active player's autopilot is due — their main phase with nothing open, or a
//! prompt of theirs open — it takes `choose`'s action, one at a time, each through the reducer's own
//! step, until the turn ends, an action of the other player's is owed (a prompt of theirs: their
//! answer brings the autopilot back), or `AUTOPILOT_MAX_ACTIONS` is reached, when the rest of the
//! turn is a timeout's (R79). The status ends at that turn's cleanup.
//!
//! `choose` (R1183003) is a pure function of the state and the player: every candidate is tried on
//! the player's concealed copy of the board (`scorer::concealed_copy`, R222: a card hidden from
//! them is a stand-in with no text), drawing from the dry run's own seed, so the match's rng is
//! never touched, the choice never reads what the player may not, and the same state always makes
//! the same choice (§9.3). In order:
//!
//! 1. A prompt of theirs: the first `AUTOPILOT_ANSWERS_TRIED` answers `legal_actions` lists, the one
//!    whose result ranks highest (`Outcome`), ties to the earlier.
//! 2. A play: their playable cards in the scorer's order (`scorer::score_instance`, the card as its
//!    face states it, as M #71's judge scores a play, R1125), each tried at the plays the scorer's dry
//!    run narrows it to (`scorer::dry_run_plays`) — those tries are the dry run; the first card with
//!    a play that does no harm — it wins, sets up lethal, clears the enemy board, heals a hero below
//!    `SCORER_LOW_HEALTH` or lowers no part of the board's balance — is played at its best such play.
//! 3. An attack: every attack `legal_actions` lists, the best; taken when it wins, sets up lethal,
//!    clears the enemy board or raises the balance.
//! 4. Otherwise the turn ends.
//!
//! It never activates an ability, switches a position, emotes, offers or answers a draw or concedes.

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

/// R1183002: the autopilot takes `player`'s turn from now to its end — on `player`'s own turn only,
/// with no game result, and once a turn (a second engagement that turn changes nothing). Emits
/// `autopilotEngaged`.
pub fn engage(sink: &mut EngineSink<'_>, player: PlayerId) {
    if sink.state.result.is_some() || sink.state.active != player {
        return;
    }
    let turn = sink.state.turn;
    if sink.state.players[player]
        .autopilot
        .is_some_and(|status| status.turn == turn)
    {
        return;
    }
    sink.state.players[player].autopilot = Some(Autopilot { turn, actions: 0 });
    sink.events.push(GameEvent::AutopilotEngaged { player, turn });
}

/// Whether the active player's autopilot owes an action now: theirs for this turn, no result, the
/// mulligans done, and either their main phase with nothing open or a prompt of theirs open.
pub fn due(state: &GameState) -> Option<PlayerId> {
    let player = state.active;
    let status = state.players[player].autopilot?;
    if state.result.is_some() || status.turn != state.turn || state.mulligan.is_some() {
        return None;
    }
    match &state.pending {
        Some(pending) => (pending.player_id == player).then_some(player),
        None => (state.phase == Phase::Main).then_some(player),
    }
}

/// R1183003: what one trial left behind, ranked in field order (`ranks_above`): a won game, then
/// §10.7's three priorities (lethal on the board, the enemy board cleared, a low hero healed), then
/// the change in the board's balance (`balance`). A lost game is no outcome at all.
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
    let lethal = !won && effective_life(&copy, enemy) <= projected_board_damage(&copy, player);
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
    // The scorer's reading of each card as its face states it (`score_instance` with no dry-run
    // copy): the tries below are the dry run, one per play, so a card is played only after its own
    // plays were tried, and the ranking costs no copy of the board per card (R1183003).
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
