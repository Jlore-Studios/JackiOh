//! Duos co-op's AI seats (#936, B26–B31): `duos_decide` answers a seat's mulligan and prompts with
//! the 1v1 `decide`, plays its own turn with it, makes the passes `duos_passes` names before it ends
//! that turn, owes nothing off its turn, and reads only its own team's knowledge. B32's gate plays
//! whole games, so it runs in `ai_slow_b.rs` (`duos_gate.rs`).
//!
//! States come from the testkit's `duos_scenario` (round 1 open, A1 and A2 to act) or from
//! `duos_create` + `duos_begin` (the four mulligans open). Where a test needs exact costs it then sets
//! a seat's hand and mana by hand, minting the cards with the engine's own `new_instance`, as
//! `decide.rs`'s `mulligan_with` sets a hand's defs. Every deck is `CHEAP`, so no round seat is left
//! with nothing to do (B9) before a test sets its hand. A `DuosActionBody` is compared by its JSON.

use std::cell::Cell;

use jackioh_ai::{AiOptions, decide, duos_decide, duos_passes, starting_life};
use jackioh_engine::config::{
    DUOS_HUMAN_TEAM, DUOS_OWN_HEALTH, DUOS_PASS_COST, DUOS_POOL_HEALTH, DUOS_POOL_HEALTH_HARD, duos_ai_team,
};
use jackioh_engine::duos::{
    DuosState, duos_begin, duos_create, duos_legal_actions, duos_reduce, duos_scenario, duos_view_for,
};
use jackioh_engine::testkit::{
    AI_DIFFICULTY, ActionBody, ActionType, COIN_DEF_ID, DECK_SIZE, Difficulty, GameState, MAX_MANA, PlayerId,
    PlayerState, Row, UNIT_ZONES, Value, Zone, ZoneRef, create_rng, def_of, effective_life, hand_cap_of,
    legal_actions, lock_zone, new_instance, query_cost,
};
use jackioh_engine::wire::{
    DUOS_SEATS, DuosAction, DuosActionBody, DuosCreateOptions, DuosDecks, DuosHandicaps, DuosSeat,
    DuosTeamRules, PerTeam, Team,
};

use super::support::register_cards;

const A1: DuosSeat = DuosSeat::A1;
const A2: DuosSeat = DuosSeat::A2;
const B1: DuosSeat = DuosSeat::B1;
const B2: DuosSeat = DuosSeat::B2;

/// Twenty Core cards, each playable on turn 1 onto an empty board (one-cost Units, cheap Traps and
/// Field cards, cheap Spells), so a dealt hand always holds a play and no seat is ended by B9's rule
/// before a test has set it up.
const CHEAP: [&str; 20] = [
    "core-003", "core-004", "core-005", "core-007", "core-008", "core-010", "core-011", "core-015",
    "core-018", "core-023", "core-039", "core-041", "core-048", "core-060", "core-067", "core-071",
    "core-075", "core-081", "core-086", "core-096",
];
/// Mr. Vanilla, a 1-cost vanilla Unit.
const ONE_DROP: &str = "core-008";
/// Tempo Timmy, a 1-cost Unit (Rush, First Strike).
const OTHER_ONE_DROP: &str = "core-011";
/// Pointmaster, a 2-cost Unit (First Strike).
const TWO_DROP: &str = "core-020";
/// 4-mana 7/7, a 4-cost Unit with no Cry: the card these tests pass.
const BIG: &str = "core-025";
/// Three more 4-cost Units needing no Tribute, for tests that pass several cards.
const OTHER_BIGS: [&str; 3] = ["core-054", "core-089", "core-043"];
/// Reminisce: a 1-cost Spell whose Discover asks its player to choose from their graveyard.
const REMINISCE: &str = "core-072";
/// The most actions one AI turn may take in the B27 turn tests before the test calls it stuck.
const TURN_STEPS: usize = 40;
/// The pass cap of `b28_passes_stop_at_the_teams_pass_cap`'s team.
const SMALL_PASS_CAP: i32 = 2;
/// Spiteful Stab, a 3-cost Spell: the cheaper card the R1127000 tests pass. It deals 2 plus
/// A2's missing health below 30 over 5 (R72), so 4 off a full-health Duos hero.
const STAB: &str = "core-070";
/// Midrange Menace, a 3-cost Unit (Taunt): the cheaper card of the free-pass pair.
const MID: &str = "core-019";

// ---------------------------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------------------------

fn cheap() -> Vec<String> {
    CHEAP.iter().map(|id| id.to_string()).collect()
}

fn decks() -> DuosDecks {
    DuosDecks {
        a1: cheap(),
        a2: cheap(),
        b1: cheap(),
        b2: cheap(),
    }
}

/// Core cards from `core-001` on, `size` of them: every one a non-token card of a shipped set.
fn core_deck(size: i32) -> Vec<String> {
    (1..=size).map(|n| format!("core-{n:03}")).collect()
}

fn create_options(seed: &str, teams: Option<PerTeam<DuosTeamRules>>) -> DuosCreateOptions {
    DuosCreateOptions {
        seed: seed.to_string(),
        decks: decks(),
        catalog: None,
        teams,
        handicaps: None,
        dealt: None,
    }
}

/// A begun Duos game: all four mulligans open, round 0 (B1, B2).
fn mulligans(seed: &str) -> DuosState {
    register_cards();
    let state = duos_begin(&duos_create(&create_options(seed, None))).state;
    assert_eq!(state.round, 0, "{seed}: the mulligans are open");
    state
}

/// Round 1 open: A1 and A2 are the round seats and neither has ended (B2, B4).
fn round_one(seed: &str, teams: Option<PerTeam<DuosTeamRules>>) -> DuosState {
    register_cards();
    let state = duos_scenario(seed, decks(), teams);
    assert_eq!(state.round, 1, "{seed}: round 1 is open");
    assert_eq!(state.round_seats, vec![A1, A2], "{seed}");
    assert_eq!(state.ended, vec![false, false], "{seed}");
    state
}

/// Round 2 open, team B's, after A1 and A2 have ended round 1 (B5). Team B plays by `b`.
fn round_two(seed: &str, b: DuosTeamRules) -> DuosState {
    let teams = PerTeam {
        a: DUOS_HUMAN_TEAM,
        b,
    };
    let state = round_one(seed, Some(teams));
    let state = apply(&state, A1, end_turn());
    let state = apply(&state, A2, end_turn());
    assert_eq!(state.round, 2, "{seed}: round 2 is open");
    assert_eq!(state.round_seats, vec![B1, B2], "{seed}");
    assert_eq!(state.ended, vec![false, false], "{seed}");
    state
}

/// Team A on `a`, team B on the human rules.
fn team_a(a: DuosTeamRules) -> Option<PerTeam<DuosTeamRules>> {
    Some(PerTeam {
        a,
        b: DUOS_HUMAN_TEAM,
    })
}

fn opts(seed: &str) -> AiOptions<'static> {
    AiOptions::new(create_rng(seed, 0))
}

/// Whether `seat` owes an action: `duos_decide` answers something.
fn owes(state: &DuosState, seat: DuosSeat) -> bool {
    let mut options = opts(&format!("owes:{seat}"));
    duos_decide(state, seat, &mut options).is_some()
}

/// What the 1v1 `decide` alone answers for `seat` in its own duel.
fn decided_alone(state: &DuosState, seat: DuosSeat, seed: &str) -> ActionBody {
    decide(&state.duels[seat.duel()], seat.player(), &mut opts(seed))
        .unwrap_or_else(|| panic!("{seat}: the 1v1 decide owes nothing"))
        .action
}

fn json_of<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

fn duel(body: ActionBody) -> DuosActionBody {
    DuosActionBody::Duel { body }
}

fn end_turn() -> DuosActionBody {
    duel(ActionBody::EndTurn)
}

fn pass(instance_id: &str) -> DuosActionBody {
    DuosActionBody::Pass {
        instance_id: instance_id.to_string(),
    }
}

fn is_pass(body: &DuosActionBody) -> bool {
    matches!(body, DuosActionBody::Pass { .. })
}

fn is_mulligan(body: &DuosActionBody) -> bool {
    matches!(body, DuosActionBody::Duel { body } if body.action_type() == ActionType::Mulligan)
}

fn is_play_of(action: &ActionBody, id: &str) -> bool {
    matches!(action, ActionBody::Play { instance_id, .. } if instance_id == id)
}

thread_local! {
    /// One count per test thread, so every nonce is fresh.
    static NONCES: Cell<u32> = const { Cell::new(0) };
}

/// One action through `duos_reduce`, panicking with the wrapper's refusal.
fn apply(state: &DuosState, seat: DuosSeat, body: DuosActionBody) -> DuosState {
    let count = NONCES.with(|nonces| {
        nonces.set(nonces.get() + 1);
        nonces.get()
    });
    let shown = json_of(&body);
    let action = DuosAction {
        seat,
        nonce: format!("duos-ai-{count}"),
        body,
    };
    let result = duos_reduce(state, &action);
    if let Some(error) = &result.error {
        panic!("{seat}'s {shown} refused: {error}");
    }
    result.state
}

/// `seat`'s own side of its own duel.
fn side(state: &mut DuosState, seat: DuosSeat) -> &mut PlayerState {
    &mut state.duels[seat.duel()].players[seat.player()]
}

/// Mints a `def_id` card at the right end of `seat`'s hand; returns its instance id.
fn give(state: &mut DuosState, seat: DuosSeat, def_id: &str) -> String {
    let player = seat.player();
    let game = &mut state.duels[seat.duel()];
    let card = new_instance(game, def_id, player, Zone::Hand { player });
    let id = card.id.clone();
    game.players[player].hand.push(card);
    id
}

/// Replaces `seat`'s hand with fresh cards of `defs`, in order; returns their instance ids.
fn set_hand(state: &mut DuosState, seat: DuosSeat, defs: &[&str]) -> Vec<String> {
    side(state, seat).hand.clear();
    defs.iter().map(|def_id| give(state, seat, def_id)).collect()
}

/// Mints a `def_id` card into `seat`'s graveyard.
fn bury(state: &mut DuosState, seat: DuosSeat, def_id: &str) {
    let player = seat.player();
    let game = &mut state.duels[seat.duel()];
    let card = new_instance(game, def_id, player, Zone::Graveyard { player });
    game.players[player].graveyard.push(card);
}

/// Tops `seat`'s hand up with one-drops until it holds `size` cards.
fn fill_hand(state: &mut DuosState, seat: DuosSeat, size: i32) {
    while (side(state, seat).hand.len() as i32) < size {
        give(state, seat, ONE_DROP);
    }
}

fn set_mana(state: &mut DuosState, seat: DuosSeat, current: i32) {
    side(state, seat).mana.current = current;
}

/// Gives `seat` a full refresh next turn: MAX_MANA crystals and the turns started that keep them.
fn full_refresh_next_turn(state: &mut DuosState, seat: DuosSeat) {
    side(state, seat).mana.max = MAX_MANA;
    side(state, seat).turns_started = MAX_MANA;
}

/// Locks every unit zone of `seat`'s (R688: a Lock refuses plays into its zone).
fn lock_units(state: &mut DuosState, seat: DuosSeat) {
    let player = seat.player();
    for lane in 1..=UNIT_ZONES {
        let slot = ZoneRef {
            player,
            row: Row::Units,
            lane,
        };
        lock_zone(&mut state.duels[seat.duel()], slot);
    }
}

fn cost(def_id: &str) -> i32 {
    query_cost(def_of(None, def_id))
}

/// Whether the 1v1 engine offers `player` a play of the instance `id` in `game`.
fn can_play(game: &GameState, player: PlayerId, id: &str) -> bool {
    legal_actions(game, player)
        .iter()
        .any(|action| is_play_of(action, id))
}

/// The duel play of the instance `id` that `duos_legal_actions` offers `seat`.
fn duel_play_of(state: &DuosState, seat: DuosSeat, id: &str) -> Option<ActionBody> {
    duos_legal_actions(state, seat)
        .into_iter()
        .find_map(|legal| match legal {
            DuosActionBody::Duel { body } if is_play_of(&body, id) => Some(body),
            _ => None,
        })
}

/// `duos_decide`'s answer for `seat`, applied; panics if it owes nothing.
fn ai_answers(state: &DuosState, seat: DuosSeat, seed: &str) -> DuosState {
    let body = duos_decide(state, seat, &mut opts(&format!("{seed}:{seat}")))
        .unwrap_or_else(|| panic!("{seed}: {seat} owes an action"));
    apply(state, seat, body)
}

/// Round 1 with A1 holding exactly `hand` at `a1_mana` and A2 (its dealt hand kept) at `a2_mana`.
fn passing(
    seed: &str,
    teams: Option<PerTeam<DuosTeamRules>>,
    hand: &[&str],
    a1_mana: i32,
    a2_mana: i32,
) -> (DuosState, Vec<String>) {
    let mut state = round_one(seed, teams);
    let ids = set_hand(&mut state, A1, hand);
    set_mana(&mut state, A1, a1_mana);
    set_mana(&mut state, A2, a2_mana);
    (state, ids)
}

/// Round 1 with a real choice for A1: three cheap Units it can play, and BIG, which it cannot
/// afford and A2 can.
fn choice_state(seed: &str) -> DuosState {
    let mut state = round_one(seed, None);
    set_hand(&mut state, A1, &[ONE_DROP, OTHER_ONE_DROP, TWO_DROP, BIG]);
    set_mana(&mut state, A1, cost(TWO_DROP));
    set_mana(&mut state, A2, cost(BIG));
    state
}

/// Round 1 with A1 having played Reminisce, so it holds Reminisce's prompt.
fn prompt_state(seed: &str) -> DuosState {
    let mut state = round_one(seed, None);
    let reminisce = set_hand(&mut state, A1, &[REMINISCE]).remove(0);
    bury(&mut state, A1, ONE_DROP);
    set_mana(&mut state, A1, cost(REMINISCE));
    let play = duel_play_of(&state, A1, &reminisce).expect("Reminisce is legal");
    let state = apply(&state, A1, duel(play));
    let holder = state.duels[0].pending.as_ref().map(|pending| pending.player_id);
    assert_eq!(holder, Some(PlayerId::P1), "Reminisce asks A1 to choose");
    state
}

/// Gives the first hand card of `seat`'s that is not The Coin (whose deal is public, R244) the def
/// `def_id` instead: a change only `seat`'s own team can see.
fn swap_hidden_hand_card(state: &mut DuosState, seat: DuosSeat, def_id: &str) {
    let card = side(state, seat)
        .hand
        .iter_mut()
        .find(|card| card.def_id != COIN_DEF_ID && card.def_id != def_id)
        .unwrap_or_else(|| panic!("{seat} holds a hidden hand card"));
    card.def_id = def_id.to_string();
}

/// Plays `seat`'s turn with `duos_decide` until it owes nothing, at most TURN_STEPS actions; returns
/// the state, every action taken as JSON, and whether the seat stopped owing.
fn play_turn(mut state: DuosState, seat: DuosSeat, seed: &str) -> (DuosState, Vec<Value>, bool) {
    let mut options = opts(seed);
    let mut taken: Vec<Value> = Vec::new();
    for _ in 0..TURN_STEPS {
        let Some(body) = duos_decide(&state, seat, &mut options) else {
            return (state, taken, true);
        };
        assert!(!matches!(body, DuosActionBody::Concede), "never concedes");
        taken.push(json_of(&body));
        state = apply(&state, seat, body);
    }
    (state, taken, false)
}

// ---------------------------------------------------------------------------------------------
// B26: the mulligan and prompts go to the 1v1 decide; a seat that owes nothing gets None
// ---------------------------------------------------------------------------------------------

#[test]
fn b26_each_seat_answers_its_own_open_mulligan_with_the_1v1_decide() {
    let state = mulligans("duos-ai-mulligan");
    for seat in DUOS_SEATS {
        let seed = format!("duos-ai-mulligan:{seat}");
        let expected = decided_alone(&state, seat, &seed);
        assert_eq!(expected.action_type(), ActionType::Mulligan, "{seat}");
        let got = duos_decide(&state, seat, &mut opts(&seed));
        assert_eq!(json_of(&got), json_of(&Some(duel(expected))), "{seat}");
        // The wrapper takes it.
        apply(&state, seat, got.expect("an answer"));
    }
}

#[test]
fn b26_a_seat_owes_nothing_once_its_mulligan_is_in_nor_while_the_other_duels_are_open() {
    let seed = "duos-ai-mulligan-wait";
    let state = mulligans(seed);
    let state = ai_answers(&state, A1, seed);
    assert!(!owes(&state, A1), "A1 has answered its mulligan");
    assert!(owes(&state, B1), "B1's mulligan is still open");

    // Duel 0's mulligans are both in and duel 1's are not, so nobody may play yet (B2).
    let state = ai_answers(&state, B1, seed);
    assert_eq!(state.round, 0);
    assert!(!owes(&state, A1), "A1 waits for duel 1's mulligans");
    assert!(!owes(&state, B1), "B1 waits for duel 1's mulligans");
    assert!(owes(&state, A2), "A2's mulligan is open");
    assert!(owes(&state, B2), "B2's mulligan is open");

    // Once all four are in, round 1 is team A's.
    let state = ai_answers(&state, A2, seed);
    let state = ai_answers(&state, B2, seed);
    assert_eq!(state.round, 1, "all four mulligans resolved");
    assert!(owes(&state, A1));
    assert!(owes(&state, A2));
    assert!(!owes(&state, B1), "B1 owes nothing in team A's round");
    assert!(!owes(&state, B2), "B2 owes nothing in team A's round");
}

#[test]
fn b26_the_seat_answers_the_prompt_it_holds_with_the_1v1_decide() {
    let seed = "duos-ai-prompt";
    let state = prompt_state(seed);
    let expected = decided_alone(&state, A1, seed);
    let got = duos_decide(&state, A1, &mut opts(seed));
    assert_eq!(json_of(&got), json_of(&Some(duel(expected))));
    // The wrapper takes it.
    apply(&state, A1, got.expect("an answer"));
}

#[test]
fn b26_an_ended_seat_still_answers_the_prompt_it_holds() {
    let seed = "duos-ai-prompt-ended";
    let mut state = prompt_state(seed);
    // B6: a seat that has ended its turn may still answer a prompt it holds.
    state.ended[0] = true;
    let expected = decided_alone(&state, A1, seed);
    let got = duos_decide(&state, A1, &mut opts(seed));
    assert_eq!(json_of(&got), json_of(&Some(duel(expected))));
}

#[test]
fn b26_the_enemy_team_owes_nothing_in_team_as_round() {
    let state = round_one("duos-ai-enemy-round", None);
    assert!(!owes(&state, B1), "B1 owes nothing in round 1");
    assert!(!owes(&state, B2), "B2 owes nothing in round 1");
}

#[test]
fn b26_a_seat_that_has_ended_owes_nothing_while_its_teammate_plays_on() {
    let seed = "duos-ai-ended";
    let state = round_one(seed, None);
    let state = apply(&state, A1, end_turn());
    assert!(state.ended[0], "A1 has ended");
    assert_eq!(state.round, 1, "A2 has not, so round 1 goes on");
    // Duel 0's endTurn waits for A2's (B5), so the 1v1 decide alone would still play it.
    assert!(decide(&state.duels[0], PlayerId::P1, &mut opts(seed)).is_some());

    assert!(!owes(&state, A1), "A1 has ended");
    assert!(owes(&state, A2), "A2 still owes its turn");
}

#[test]
fn b26_a_finished_game_owes_nothing() {
    let state = round_one("duos-ai-over", None);
    let state = apply(&state, A1, DuosActionBody::Concede);
    let state = apply(&state, A2, DuosActionBody::Concede);
    assert!(state.result.is_some(), "team A conceded (B11)");
    for seat in DUOS_SEATS {
        assert!(!owes(&state, seat), "{seat} owes nothing once the game is over");
    }
}

// ---------------------------------------------------------------------------------------------
// B27: its own turn is the 1v1 decide's, and its passes come before the turn's end
// ---------------------------------------------------------------------------------------------

#[test]
fn b27_while_decide_still_plays_the_seat_plays_that_move_and_passes_nothing_yet() {
    let seed = "duos-ai-plays";
    let mut state = round_one(seed, None);
    set_hand(&mut state, A1, &[ONE_DROP, OTHER_ONE_DROP, BIG]);
    set_mana(&mut state, A1, cost(ONE_DROP));
    set_mana(&mut state, A2, cost(BIG));
    // BIG is a card A1 would pass (B28), but A1 still has plays.
    let expected = decided_alone(&state, A1, seed);
    assert_ne!(expected, ActionBody::EndTurn, "the 1v1 AI plays a one-drop");

    let got = duos_decide(&state, A1, &mut opts(seed));
    assert!(!got.as_ref().is_some_and(is_pass), "no pass yet");
    assert_eq!(json_of(&got), json_of(&Some(duel(expected))));
}

#[test]
fn b27_when_decide_would_end_the_turn_the_seat_makes_its_pass_first_then_ends() {
    let seed = "duos-ai-pass-then-end";
    let mut state = round_one(seed, None);
    let big = set_hand(&mut state, A1, &[BIG]).remove(0);
    set_mana(&mut state, A1, DUOS_PASS_COST);
    set_mana(&mut state, A2, cost(BIG));
    // A2 keeps a play of its own, so round 1 stays open after A1 ends.
    give(&mut state, A2, ONE_DROP);
    let would = decided_alone(&state, A1, seed);
    assert_eq!(would, ActionBody::EndTurn, "A1 can play nothing");

    let (state, taken, finished) = play_turn(state, A1, seed);
    assert!(finished, "{taken:?}");
    let first = json_of(&pass(&big));
    assert_eq!(taken.first(), Some(&first), "the pass comes first: {taken:?}");
    let end = json_of(&end_turn());
    assert!(
        taken[1..].iter().all(|body| *body == end),
        "then only the turn's end: {taken:?}"
    );
    assert!(state.ended[0], "A1 has ended its turn");
    assert_eq!(state.round, 1, "A2 has not");
    let hand = &state.duels[1].players.p1.hand;
    assert!(
        hand.iter().any(|card| card.def_id == BIG),
        "A2 holds the passed card"
    );
}

#[test]
fn b27_a_whole_ai_turn_plays_then_passes_then_ends() {
    let state = round_one("duos-ai-whole-turn", None);
    let (state, taken, finished) = play_turn(state, A1, "duos-ai-whole-turn");
    assert!(finished, "A1's turn ends within {TURN_STEPS} actions: {taken:?}");
    assert!(state.ended[0], "A1 has ended its turn");
    assert!(!state.ended[1], "A2 has not");
    assert_eq!(state.round, 1);
    // Once the first pass is made, only passes and the turn's end follow.
    let end = json_of(&end_turn());
    if let Some(first) = taken.iter().position(|body| body["kind"] == "pass") {
        let tail = &taken[first..];
        assert!(
            tail.iter().all(|body| body["kind"] == "pass" || *body == end),
            "{taken:?}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// B28: the cards duos_passes names
// ---------------------------------------------------------------------------------------------

#[test]
fn b28_a_card_the_seat_cannot_afford_and_its_teammate_can_is_passed() {
    let seed = "duos-ai-pass";
    let (state, ids) = passing(seed, None, &[BIG], DUOS_PASS_COST, cost(BIG));
    let playable = can_play(&state.duels[0], PlayerId::P1, &ids[0]);
    assert!(!playable, "A1 cannot afford it");
    let free_zone = state.duels[1].players.p1.units.iter().any(Option::is_none);
    assert!(free_zone, "A2 has a unit zone free");
    assert_eq!(duos_passes(&state, A1), ids);
    let got = duos_decide(&state, A1, &mut opts(seed));
    assert_eq!(json_of(&got), json_of(&Some(pass(&ids[0]))));
}

#[test]
fn b28_a_card_the_seat_can_afford_but_has_no_zone_for_is_passed() {
    let (mut state, ids) = passing("duos-ai-pass-no-zone", None, &[BIG], cost(BIG), cost(BIG));
    lock_units(&mut state, A1);
    let playable = can_play(&state.duels[0], PlayerId::P1, &ids[0]);
    assert!(!playable, "the locks close every unit zone of A1's");
    assert_eq!(duos_passes(&state, A1), ids);
}

#[test]
fn b28_a_card_the_seat_can_play_itself_is_not_passed() {
    let hand = [ONE_DROP, BIG];
    let (state, ids) = passing("duos-ai-keep-playable", None, &hand, DUOS_PASS_COST, cost(BIG));
    assert!(can_play(&state.duels[0], PlayerId::P1, &ids[0]));
    assert!(!duos_passes(&state, A1).contains(&ids[0]));
}

#[test]
fn b28_nothing_is_passed_to_a_teammate_that_has_ended() {
    let seed = "duos-ai-teammate-ended";
    let state = round_one(seed, None);
    let mut state = apply(&state, A2, end_turn());
    assert!(state.ended[1], "A2 has ended");
    assert_eq!(state.round, 1, "A1 has not");
    set_hand(&mut state, A1, &[BIG]);
    set_mana(&mut state, A1, DUOS_PASS_COST);
    set_mana(&mut state, A2, cost(BIG));
    assert!(duos_passes(&state, A1).is_empty());
    let got = duos_decide(&state, A1, &mut opts(seed));
    assert_eq!(json_of(&got), json_of(&Some(end_turn())));
}

#[test]
fn b28_nothing_is_passed_to_a_teammate_whose_hand_is_full() {
    let (mut state, _) = passing("duos-ai-teammate-full", None, &[BIG], DUOS_PASS_COST, cost(BIG));
    let cap = hand_cap_of(&state.duels[1], PlayerId::P1);
    fill_hand(&mut state, A2, cap);
    assert!(duos_passes(&state, A1).is_empty());
}

#[test]
fn b28_nothing_is_passed_to_a_teammate_that_cannot_afford_it() {
    let seed = "duos-ai-teammate-poor";
    let (state, _) = passing(seed, None, &[BIG], DUOS_PASS_COST, cost(BIG) - 1);
    assert!(duos_passes(&state, A1).is_empty());
    let got = duos_decide(&state, A1, &mut opts(seed));
    assert_eq!(json_of(&got), json_of(&Some(end_turn())));
}

#[test]
fn b28_nothing_is_passed_to_a_teammate_with_no_zone_for_it() {
    let seed = "duos-ai-teammate-no-zone";
    let (mut state, _) = passing(seed, None, &[BIG], DUOS_PASS_COST, cost(BIG));
    lock_units(&mut state, A2);
    let probe = give(&mut state, A2, ONE_DROP);
    let playable = can_play(&state.duels[1], PlayerId::P1, &probe);
    assert!(!playable, "the locks close every unit zone of A2's");
    assert!(duos_passes(&state, A1).is_empty());
}

#[test]
fn b28_a_seat_without_the_mana_for_a_paid_pass_passes_nothing() {
    let seed = "duos-ai-no-mana";
    let (state, _) = passing(seed, None, &[BIG], DUOS_PASS_COST - 1, cost(BIG));
    assert!(duos_passes(&state, A1).is_empty());
    let got = duos_decide(&state, A1, &mut opts(seed));
    assert_eq!(json_of(&got), json_of(&Some(end_turn())));
}

#[test]
fn b28_off_its_own_turn_a_seat_passes_nothing() {
    let mut state = round_one("duos-ai-off-turn", None);
    // Round 1 is team A's: neither B1 nor B2 is a round seat.
    set_hand(&mut state, B1, &[BIG]);
    set_mana(&mut state, B1, DUOS_PASS_COST);
    set_mana(&mut state, B2, cost(BIG));
    assert!(duos_passes(&state, B1).is_empty());
}

#[test]
fn b28_passes_stop_at_the_teams_pass_cap() {
    let rules = DuosTeamRules {
        pass_cap: SMALL_PASS_CAP,
        ..DUOS_HUMAN_TEAM
    };
    let mut hand = vec![ONE_DROP, BIG];
    hand.extend(OTHER_BIGS);
    for def_id in &hand[1..] {
        assert_eq!(cost(def_id), cost(BIG), "{def_id}");
    }
    // A1 can afford its one-drop (so it always has a play left) and every pass, but none of the
    // four 4-costs, all of which A2 can afford.
    let seed = "duos-ai-pass-cap";
    let (state, ids) = passing(seed, team_a(rules), &hand, cost(BIG) - 1, cost(BIG));
    let passable = &ids[1..];

    let first = duos_passes(&state, A1);
    assert_eq!(first.len(), SMALL_PASS_CAP as usize, "{first:?}");
    assert!(first.iter().all(|id| passable.contains(id)), "{first:?}");
    let state = apply(&state, A1, pass(&first[0]));

    let second = duos_passes(&state, A1);
    assert_eq!(second.len(), 1, "one pass left: {second:?}");
    assert!(passable.contains(&second[0]), "{second:?}");
    assert_ne!(second[0], first[0]);
    let state = apply(&state, A1, pass(&second[0]));

    assert!(!state.ended[0], "A1 can still play its one-drop");
    assert!(duos_passes(&state, A1).is_empty(), "the pass cap is reached");
}

#[test]
fn b28_never_more_passes_than_the_teammates_free_hand_slots() {
    let mut hand = vec![BIG];
    hand.extend(OTHER_BIGS);
    let (mut state, ids) = passing("duos-ai-free-slots", None, &hand, cost(BIG) - 1, cost(BIG));
    let cap = hand_cap_of(&state.duels[1], PlayerId::P1);
    fill_hand(&mut state, A2, cap - 1);
    let passes = duos_passes(&state, A1);
    assert_eq!(passes.len(), 1, "one free slot: {passes:?}");
    assert!(ids.contains(&passes[0]));
}

#[test]
fn b28_with_free_passes_a_card_the_teammate_can_afford_next_turn_is_passed() {
    let free = DuosTeamRules {
        pass_cost: 0,
        ..DUOS_HUMAN_TEAM
    };
    assert!(cost(BIG) <= MAX_MANA);
    let mut state = round_one("duos-ai-free-next-turn", team_a(free));
    let ids = set_hand(&mut state, A1, &[BIG]);
    // A1: turn 1's crystal, spent; BIG is out of its reach this turn and next.
    set_mana(&mut state, A1, 0);
    assert!(side(&mut state, A1).mana.max + 1 < cost(BIG));
    // A2: no mana now, and a full refresh next turn.
    set_mana(&mut state, A2, 0);
    full_refresh_next_turn(&mut state, A2);
    assert_eq!(duos_passes(&state, A1), ids);
}

#[test]
fn b28_with_paid_passes_a_card_the_teammate_can_afford_only_next_turn_is_kept() {
    let mut state = round_one("duos-ai-paid-next-turn", None);
    set_hand(&mut state, A1, &[BIG]);
    set_mana(&mut state, A1, DUOS_PASS_COST);
    set_mana(&mut state, A2, 0);
    full_refresh_next_turn(&mut state, A2);
    assert!(duos_passes(&state, A1).is_empty());
}

// ---------------------------------------------------------------------------------------------
// B29: only the team's knowledge
// ---------------------------------------------------------------------------------------------

/// The same decision and passes for team A from `base` and `variant`, which differ only in a card
/// team A cannot see.
fn assert_team_a_decides_alike(base: &DuosState, variant: &DuosState, seed: &str) {
    for seat in [A1, A2] {
        let (seen, seen_too) = (duos_view_for(base, seat), duos_view_for(variant, seat));
        assert_eq!(json_of(&seen), json_of(&seen_too), "{seat} sees no difference");
        let (mut first, mut second) = (opts(seed), opts(seed));
        let from_base = duos_decide(base, seat, &mut first);
        let from_variant = duos_decide(variant, seat, &mut second);
        assert!(from_base.is_some(), "{seat} owes its turn");
        assert_eq!(json_of(&from_base), json_of(&from_variant), "{seat}");
        assert_eq!(first.rng.cursor(), second.rng.cursor(), "{seat}");
        assert_eq!(duos_passes(base, seat), duos_passes(variant, seat), "{seat}");
    }
}

#[test]
fn b29_a_hidden_card_in_the_opponents_hand_does_not_change_the_decision() {
    let seed = "duos-ai-hidden-across";
    let base = choice_state(seed);
    let mut variant = choice_state(seed);
    swap_hidden_hand_card(&mut variant, B1, OTHER_BIGS[2]);
    let hand = |state: &DuosState| json_of(&state.duels[0].players.p2.hand);
    assert_ne!(hand(&base), hand(&variant), "B1's hands differ");
    assert_team_a_decides_alike(&base, &variant, seed);
}

#[test]
fn b29_a_hidden_card_in_the_teammates_opponents_hand_does_not_change_the_decision() {
    let seed = "duos-ai-hidden-diagonal";
    let base = choice_state(seed);
    let mut variant = choice_state(seed);
    swap_hidden_hand_card(&mut variant, B2, BIG);
    let hand = |state: &DuosState| json_of(&state.duels[1].players.p2.hand);
    assert_ne!(hand(&base), hand(&variant), "B2's hands differ");
    assert_team_a_decides_alike(&base, &variant, seed);
}

// ---------------------------------------------------------------------------------------------
// B30: the AI tiers
// ---------------------------------------------------------------------------------------------

#[test]
fn b30_the_easy_ai_team_plays_the_human_teams_rules() {
    assert_eq!(duos_ai_team(Difficulty::Easy), DUOS_HUMAN_TEAM);
}

#[test]
fn b30_medium_passes_free_and_hard_passes_free_with_the_hard_pool() {
    let medium = DuosTeamRules {
        pass_cost: 0,
        ..DUOS_HUMAN_TEAM
    };
    assert_eq!(duos_ai_team(Difficulty::Medium), medium);
    let hard = duos_ai_team(Difficulty::Hard);
    assert_eq!(hard.pass_cost, 0);
    assert_eq!(hard.pool_health, DUOS_POOL_HEALTH_HARD);
    let expected = DuosTeamRules {
        pass_cost: 0,
        pool_health: DUOS_POOL_HEALTH_HARD,
        ..DUOS_HUMAN_TEAM
    };
    assert_eq!(hard, expected);
}

#[test]
fn b30_a_medium_or_hard_ai_seat_with_no_mana_left_still_passes() {
    for difficulty in [Difficulty::Medium, Difficulty::Hard] {
        let seed = format!("duos-ai-free-pass:{difficulty}");
        let mut state = round_two(&seed, duos_ai_team(difficulty));
        let ids = set_hand(&mut state, B1, &[BIG]);
        set_mana(&mut state, B1, 0);
        set_mana(&mut state, B2, cost(BIG));
        assert_eq!(duos_passes(&state, B1), ids, "{difficulty}");
        let got = duos_decide(&state, B1, &mut opts(&seed));
        assert_eq!(json_of(&got), json_of(&Some(pass(&ids[0]))), "{difficulty}");
    }
}

#[test]
fn b30_a_free_passing_ai_seat_keeps_a_card_its_teammate_cannot_afford_now_or_next_turn() {
    let seed = "duos-ai-free-pass-poor";
    let mut state = round_two(seed, duos_ai_team(Difficulty::Medium));
    set_hand(&mut state, B1, &[BIG]);
    set_mana(&mut state, B1, 0);
    // B2 is on its first turn: no mana now (The Coin would not make four), two crystals next turn.
    set_mana(&mut state, B2, 0);
    assert!(side(&mut state, B2).mana.max + 1 < cost(BIG));
    assert!(duos_passes(&state, B1).is_empty());
}

#[test]
fn b30_an_easy_ai_seat_with_no_mana_left_passes_nothing() {
    let seed = "duos-ai-easy-no-pass";
    let mut state = round_two(seed, duos_ai_team(Difficulty::Easy));
    set_hand(&mut state, B1, &[BIG]);
    set_mana(&mut state, B1, 0);
    set_mana(&mut state, B2, cost(BIG));
    assert!(duos_passes(&state, B1).is_empty());
    let got = duos_decide(&state, B1, &mut opts(seed));
    assert_eq!(json_of(&got), json_of(&Some(end_turn())));
}

#[test]
fn b30_hard_ai_seats_keep_their_handicap_and_their_pool_and_answer_their_mulligans() {
    register_cards();
    let hard = AI_DIFFICULTY[Difficulty::Hard];
    let decks = DuosDecks {
        a1: core_deck(DECK_SIZE),
        a2: core_deck(DECK_SIZE),
        b1: core_deck(hard.deck_size),
        b2: core_deck(hard.deck_size),
    };
    let teams = PerTeam {
        a: DUOS_HUMAN_TEAM,
        b: duos_ai_team(Difficulty::Hard),
    };
    let handicaps = DuosHandicaps {
        a1: None,
        a2: None,
        b1: Some(hard),
        b2: Some(hard),
    };
    let options = DuosCreateOptions {
        seed: "duos-ai-hard".to_string(),
        decks,
        catalog: None,
        teams: Some(teams),
        handicaps: Some(handicaps),
        dealt: None,
    };
    let state = duos_begin(&duos_create(&options)).state;
    for seat in [B1, B2] {
        let ai = &state.duels[seat.duel()].players[seat.player()];
        assert_eq!(ai.handicap, Some(hard), "{seat}");
        let pool = ai.duos.as_ref().map(|life| (life.pool, life.pool_cap));
        let full = (DUOS_POOL_HEALTH_HARD, DUOS_POOL_HEALTH_HARD);
        assert_eq!(pool, Some(full), "{seat}");
        let body = duos_decide(&state, seat, &mut opts(&format!("duos-ai-hard:{seat}")))
            .unwrap_or_else(|| panic!("{seat} owes its mulligan"));
        assert!(is_mulligan(&body), "{seat}: {}", json_of(&body));
        apply(&state, seat, body);
    }
}

// ---------------------------------------------------------------------------------------------
// B31: deterministic
// ---------------------------------------------------------------------------------------------

#[test]
fn b31_the_same_state_and_options_give_the_same_action_twice() {
    let mulligan = mulligans("duos-ai-same-mulligan");
    let choice = choice_state("duos-ai-same-choice");
    let dealt = round_one("duos-ai-same-dealt", None);
    let cases = [
        (&mulligan, A1),
        (&mulligan, B2),
        (&choice, A1),
        (&choice, A2),
        (&dealt, A1),
    ];
    for (at, (state, seat)) in cases.into_iter().enumerate() {
        let seed = format!("duos-ai-same:{at}");
        let (mut first, mut second) = (opts(&seed), opts(&seed));
        let once = duos_decide(state, seat, &mut first);
        let twice = duos_decide(state, seat, &mut second);
        assert!(once.is_some(), "case {at}: {seat} owes an action");
        assert_eq!(json_of(&once), json_of(&twice), "case {at}: {seat}");
        assert_eq!(first.rng.cursor(), second.rng.cursor(), "case {at}: {seat}");
        let passes = duos_passes(state, seat);
        assert_eq!(passes, duos_passes(state, seat), "case {at}: {seat}");
    }
}

// ---------------------------------------------------------------------------------------------
// R964000: a seat's life is its own HP and its team's pool
// ---------------------------------------------------------------------------------------------

#[test]
fn r964000_every_seat_starts_at_its_own_hp_and_its_teams_pool() {
    let hard = PerTeam {
        a: DUOS_HUMAN_TEAM,
        b: duos_ai_team(Difficulty::Hard),
    };
    let cases = [
        (round_one("r964000", None), DUOS_POOL_HEALTH),
        (round_one("r964000-hard", Some(hard)), DUOS_POOL_HEALTH_HARD),
    ];
    for (state, team_b_pool) in &cases {
        for seat in DUOS_SEATS {
            let pool = if seat.team() == Team::B {
                *team_b_pool
            } else {
                DUOS_POOL_HEALTH
            };
            let duel = &state.duels[seat.duel()];
            assert_eq!(
                starting_life(duel, seat.player()),
                DUOS_OWN_HEALTH + pool,
                "{seat}"
            );
            assert_eq!(
                effective_life(duel, seat.player()),
                DUOS_OWN_HEALTH + pool,
                "{seat}"
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------
// R1127000: the Duos AI passes the card that gives its teammate the best next play
// ---------------------------------------------------------------------------------------------

/// Round 1 with A1 holding BIG (4-cost) and the Stab (3-cost) at pass mana, so it can afford
/// neither, A2 at BIG mana with an empty hand, and B2 at an effective life of 3: the Stab deals 4
/// and wins outright, while the 7/7 only threatens. A2's hand is cleared so its dealt cards cannot
/// win on their own and wash the two candidates' scores out.
fn winning_pass_state(seed: &str) -> (DuosState, String, String) {
    let mut state = round_one(seed, None);
    let ids = set_hand(&mut state, A1, &[BIG, STAB]);
    set_mana(&mut state, A1, DUOS_PASS_COST);
    set_mana(&mut state, A2, cost(BIG));
    state.duels[1].players[PlayerId::P1].hand.clear();
    state.duels[1].players[PlayerId::P2].hero.health = 2;
    state.duels[1].players[PlayerId::P2]
        .duos
        .as_mut()
        .expect("a duel holds Duos life")
        .pool = 1;
    (state, ids[0].clone(), ids[1].clone())
}

#[test]
fn r1127000_passes_the_cheaper_card_that_wins_the_teammate_the_game_first() {
    let seed = "duos-ai-best-pass";
    let (state, big, stab) = winning_pass_state(seed);
    assert_eq!(
        effective_life(&state.duels[1], PlayerId::P2),
        3,
        "B2's effective life"
    );
    assert!(
        !can_play(&state.duels[0], PlayerId::P1, &big),
        "A1 cannot play BIG"
    );
    assert!(
        !can_play(&state.duels[0], PlayerId::P1, &stab),
        "A1 cannot play the Stab"
    );
    // One pass is paid for, and it is the cheaper, better card: this fails on the old cost-first
    // order, which names BIG.
    assert_eq!(duos_passes(&state, A1), vec![stab.clone()]);
    let got = duos_decide(&state, A1, &mut opts(seed));
    assert_eq!(json_of(&got), json_of(&Some(pass(&stab))));
}

#[test]
fn r1127000_hidden_opponent_cards_leave_the_pass_order_unchanged() {
    let seed = "duos-ai-best-pass-hidden";
    let base = winning_pass_state(seed).0;
    let mut across = winning_pass_state(seed).0;
    swap_hidden_hand_card(&mut across, B1, OTHER_BIGS[2]);
    let mut diagonal = winning_pass_state(seed).0;
    swap_hidden_hand_card(&mut diagonal, B2, BIG);
    for variant in [&across, &diagonal] {
        for seat in [A1, A2] {
            let (seen, seen_too) = (duos_view_for(&base, seat), duos_view_for(variant, seat));
            assert_eq!(json_of(&seen), json_of(&seen_too), "{seat} sees no difference");
            let (mut first, mut second) = (opts(seed), opts(seed));
            let from_base = duos_decide(&base, seat, &mut first);
            let from_variant = duos_decide(variant, seat, &mut second);
            assert!(from_base.is_some(), "{seat} owes its turn");
            assert_eq!(json_of(&from_base), json_of(&from_variant), "{seat}");
            assert_eq!(first.rng.cursor(), second.rng.cursor(), "{seat}");
            assert_eq!(duos_passes(&base, seat), duos_passes(variant, seat), "{seat}");
        }
    }
}

#[test]
fn r1127000_pass_limits_still_cap_a_scored_list() {
    let rules = DuosTeamRules {
        pass_cap: SMALL_PASS_CAP,
        ..DUOS_HUMAN_TEAM
    };
    let mut hand = vec![ONE_DROP, BIG];
    hand.extend(OTHER_BIGS);
    let seed = "duos-ai-best-pass-cap";
    // `cost` reads the catalog before `passing` registers it.
    register_cards();
    let (state, ids) = passing(seed, team_a(rules), &hand, cost(BIG) - 1, cost(BIG));
    assert!(
        can_play(&state.duels[0], PlayerId::P1, &ids[0]),
        "A1 keeps its one-drop"
    );
    // Scoring picks the order, the cap still truncates: two of the four 4-costs.
    let passes = duos_passes(&state, A1);
    assert_eq!(passes.len(), SMALL_PASS_CAP as usize, "{passes:?}");
    assert!(passes.iter().all(|id| ids[1..].contains(id)), "{passes:?}");
}

#[test]
fn r1127000_free_pass_next_turn_cards_keep_cost_order() {
    let free = DuosTeamRules {
        pass_cost: 0,
        ..DUOS_HUMAN_TEAM
    };
    let mut state = round_one("duos-ai-best-pass-later", team_a(free));
    let ids = set_hand(&mut state, A1, &[MID, BIG]);
    // No mana now on either seat, and a full refresh next turn for A2: both cards are affordable
    // next turn only.
    set_mana(&mut state, A1, 0);
    set_mana(&mut state, A2, 0);
    full_refresh_next_turn(&mut state, A2);
    assert_eq!(duos_passes(&state, A1), vec![ids[1].clone(), ids[0].clone()]);
}

#[test]
fn r1127000_the_passed_card_is_scored_under_the_teammates_next_id_not_the_one_it_held() {
    let seed = "duos-ai-best-pass-ids";
    let (mut state, first, second) = winning_pass_state(seed);
    // Both duels number their cards from the same start, so a card minted now in A2's duel takes the
    // id the first card of A1's hand holds. The Stab takes that id, and in A2's duel it names a
    // one-drop of A2's own.
    let (big, stab) = (second, first);
    let hand = &mut state.duels[0].players[PlayerId::P1].hand;
    hand[0].id = big.clone();
    hand[1].id = stab.clone();
    let one_drop = give(&mut state, A2, ONE_DROP);
    assert_eq!(one_drop, stab, "the Stab's id names A2's one-drop");
    // Scored under that id, the Stab's plays would play the one-drop and leave the Stab in hand, and
    // BIG, whose id names nothing of A2's, would outscore it.
    assert_eq!(duos_passes(&state, A1), vec![stab.clone()], "not {big}");
}

#[test]
fn r1127000_a_passed_fused_card_is_scored_with_its_match_made_definitions_along() {
    let seed = "duos-ai-best-pass-fused";
    let (mut state, big, stab) = winning_pass_state(seed);
    // A Fuse of a crafted card and BIG, minted in A1's duel alone (R77, R102): its definitions live
    // only in duel 0's `transient_defs`, and it costs what BIG costs.
    let crafted = "craft:#duos-ai";
    let fused = format!("t-1:{crafted}+{BIG}");
    let mut crafted_def = def_of(None, BIG).clone();
    crafted_def.id = crafted.to_string();
    let mut fused_def = def_of(None, BIG).clone();
    fused_def.id = fused.clone();
    state.duels[0]
        .transient_defs
        .insert(crafted.to_string(), crafted_def);
    state.duels[0].transient_defs.insert(fused.clone(), fused_def);
    let fusion = give(&mut state, A1, &fused);
    assert!(
        !can_play(&state.duels[0], PlayerId::P1, &fusion),
        "A1 cannot play the fusion"
    );
    // Scored without its definition, the fusion would break the scoring and every candidate would
    // fall back to cost order, BIG first.
    assert_eq!(duos_passes(&state, A1), vec![stab.clone()], "not {big}");
}
