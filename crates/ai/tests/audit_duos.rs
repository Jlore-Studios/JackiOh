//! Audit of the Duos AI (`crates/ai/src/duos.rs`, §13.7): the tests that prove the audit's findings.
//! Each asserts what the README's "Duos (§13.7)" section, R1723000 or §13.7 says, and fails today.
//!
//! Helpers are copied from `tests/ai/duos.rs` (round 1 from `duos_scenario`, hands and mana set by hand).

use std::cell::Cell;

use jackioh_ai::{AI_EVAL, AiOptions, NextSwing, duos_decide, duos_passes, evaluate};
use jackioh_engine::config::DUOS_PASS_COST;
use jackioh_engine::duos::{DuosState, duos_reduce, duos_scenario};
use jackioh_engine::testkit::{
    ActionBody, DECK_SIZE, PlaceOnFieldOptions, PlayerId, PlayerState, Position, Row, Value, Zone, ZoneSlot,
    create_rng, def_of, find_instance_mut, new_instance, place_on_field, plays_in, query_cost,
};
use jackioh_engine::wire::{DuosAction, DuosActionBody, DuosDecks, DuosSeat, DuosTeamRules, PerTeam};
use jackioh_engine::{Action, legal_actions, reduce_unpiloted};

const A1: DuosSeat = DuosSeat::A1;
const A2: DuosSeat = DuosSeat::A2;
const B1: DuosSeat = DuosSeat::B1;
const B2: DuosSeat = DuosSeat::B2;

/// `tests/ai/duos.rs`'s deck: every card playable on turn 1.
const CHEAP: [&str; DECK_SIZE as usize] = [
    "core-003", "core-004", "core-005", "core-007", "core-008", "core-010", "core-011", "core-015",
    "core-018", "core-023", "core-031", "core-035", "core-039", "core-041", "core-044", "core-048",
    "core-051", "core-060", "core-062", "core-067", "core-071", "core-075", "core-081", "core-086",
    "core-096",
];
/// Mr. Vanilla, a 1-cost 4/4.
const ONE_DROP: &str = "core-008";
/// Pointmaster, a 2-cost Unit.
const TWO_DROP: &str = "core-020";
/// 4-mana 7/7.
const BIG: &str = "core-025";
/// Hit Job, a 3-cost Spell: "Destroy a Unit." `legal_actions` lists its caster's own Units first.
const HIT_JOB: &str = "core-016";

fn decks() -> DuosDecks {
    let cheap = || CHEAP.iter().map(|id| id.to_string()).collect::<Vec<String>>();
    DuosDecks {
        a1: cheap(),
        a2: cheap(),
        b1: cheap(),
        b2: cheap(),
    }
}

/// Round 1 open: A1 and A2 are the round seats.
fn round_one(seed: &str, teams: Option<PerTeam<DuosTeamRules>>) -> DuosState {
    jackioh_cards::register_all();
    let state = duos_scenario(seed, decks(), teams);
    assert_eq!(state.round, 1, "{seed}: round 1 is open");
    state
}

/// Round 2 open, team B's, on the human rules (the Easy AI team's).
fn round_two(seed: &str) -> DuosState {
    let state = round_one(seed, None);
    let state = apply(&state, A1, end_turn()).expect("A1 ends");
    let state = apply(&state, A2, end_turn()).expect("A2 ends");
    assert_eq!(state.round_seats, vec![B1, B2], "{seed}: round 2 is team B's");
    assert_eq!(state.ended, vec![false, false], "{seed}");
    state
}

fn opts(seed: &str) -> AiOptions<'static> {
    AiOptions::new(create_rng(seed, 0))
}

fn json_of<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serializes")
}

fn end_turn() -> DuosActionBody {
    DuosActionBody::Duel {
        body: ActionBody::EndTurn,
    }
}

thread_local! {
    static NONCES: Cell<u32> = const { Cell::new(0) };
}

/// One action through `duos_reduce`: the state it leaves, or the wrapper's refusal.
fn apply(state: &DuosState, seat: DuosSeat, body: DuosActionBody) -> Result<DuosState, String> {
    let count = NONCES.with(|nonces| {
        nonces.set(nonces.get() + 1);
        nonces.get()
    });
    let action = DuosAction {
        seat,
        nonce: format!("audit-duos-{count}"),
        body,
    };
    let result = duos_reduce(state, &action);
    match result.error {
        Some(error) => Err(error.to_string()),
        None => Ok(result.state),
    }
}

fn side(state: &mut DuosState, seat: DuosSeat) -> &mut PlayerState {
    &mut state.duels[seat.duel()].players[seat.player()]
}

fn set_hand(state: &mut DuosState, seat: DuosSeat, defs: &[&str]) -> Vec<String> {
    side(state, seat).hand.clear();
    let player = seat.player();
    defs.iter()
        .map(|def_id| {
            let game = &mut state.duels[seat.duel()];
            let card = new_instance(game, def_id, player, Zone::Hand { player });
            let id = card.id.clone();
            game.players[player].hand.push(card);
            id
        })
        .collect()
}

fn set_mana(state: &mut DuosState, seat: DuosSeat, current: i32) {
    side(state, seat).mana.current = current;
}

fn cost(def_id: &str) -> i32 {
    query_cost(def_of(None, def_id))
}

/// A fresh `def_id` in Attack Position in `seat`'s unit zone `lane`; its instance id.
fn field_unit(state: &mut DuosState, seat: DuosSeat, def_id: &str, lane: i32) -> String {
    let player = seat.player();
    let game = &mut state.duels[seat.duel()];
    let row = Row::Units;
    let mut unit = new_instance(game, def_id, player, Zone::Field { player, row, lane });
    let slot = ZoneSlot { player, row, lane };
    assert!(place_on_field(game, &mut unit, slot, PlaceOnFieldOptions::default()));
    find_instance_mut(game, &unit.id).expect("the placed unit").position = Some(Position::Atk);
    unit.id
}

/// A2's standing value in its duel, and the best `evaluate` any one legal `Play` of A2's reaches there
/// with the play that reaches it: the teammate's real best single follow-up.
fn a2_best(state: &DuosState) -> (f64, f64, Value) {
    let duel = &state.duels[A2.duel()];
    let standing = evaluate(duel, PlayerId::P1, NextSwing::Enemy, &AI_EVAL);
    let mut best = (f64::NEG_INFINITY, Value::Null);
    for (at, action) in legal_actions(duel, PlayerId::P1).into_iter().enumerate() {
        if !matches!(action, ActionBody::Play { .. }) {
            continue;
        }
        let applied = reduce_unpiloted(duel, &Action::new(action.clone(), PlayerId::P1, format!("best{at}")));
        if applied.error.is_some() {
            continue;
        }
        let value = evaluate(&applied.state, PlayerId::P1, NextSwing::Enemy, &AI_EVAL);
        if value > best.0 {
            best = (value, json_of(&action));
        }
    }
    (standing, best.0, best.1)
}

// Finding 1: `pass_gain`'s "after the teammate's own best play" trial replays a nonce

/// Round 1: A1 holds only TWO_DROP at pass mana (it cannot play it), A2 holds `a2_hand` with the mana
/// for BIG and TWO_DROP both, and A2's unit zones 1 to `filled` hold a Unit each.
fn after_state(filled: i32, a2_hand: &[&str]) -> (DuosState, String) {
    let mut state = round_one("audit-duos-after", None);
    let two = set_hand(&mut state, A1, &[TWO_DROP]).remove(0);
    set_mana(&mut state, A1, DUOS_PASS_COST);
    set_hand(&mut state, A2, a2_hand);
    set_mana(&mut state, A2, cost(BIG) + cost(TWO_DROP));
    for lane in 1..=filled {
        field_unit(&mut state, A2, ONE_DROP, lane);
    }
    assert!(plays_in(&state.duels[0], PlayerId::P1, &two).is_empty(), "A1 cannot play it");
    (state, two)
}

/// README, Duos: "the teammate's best follow-up with the card, also tried after the teammate's own best
/// play" (R1723000: "the best of the card's own plays made after the teammate's own best play, since the
/// mana that play leaves may pay for the card too"). A2 can play BIG and then the passed TWO_DROP.
///
/// With three of A2's unit zones full, BIG takes the fourth and the passed card has one play left, into
/// the fifth. `best_step` numbers its trial nonces from 0 (`duos-pass-score:{at}`), and the state
/// `mate_baseline` hands on already holds the baseline play's nonce (`duos-pass-score:0`), so that one
/// play is answered as a replay: unchanged state, no error, scored as "the card stays in hand". The pass
/// that is named with two zones full is dropped with three, and the teammate's hand ORDER flips it.
#[test]
fn r1723000_the_passed_card_is_tried_after_the_teammates_best_play_whichever_play_that_was() {
    // PREMISE: with a second zone left for it after BIG, the pass is named.
    let (roomy, two) = after_state(2, &[BIG]);
    assert_eq!(duos_passes(&roomy, A1), vec![two.clone()], "PREMISE: worth passing");

    // The same trade with one zone left after BIG: the card's only play after BIG is trial 0.
    let (tight, two) = after_state(3, &[BIG]);
    let (first, _) = after_state(3, &[ONE_DROP, BIG]);
    let (second, _) = after_state(3, &[BIG, ONE_DROP]);
    // BIG is the baseline's play 1 here, and the card's one play after it (trial 0) is tried.
    assert_eq!(duos_passes(&first, A1), vec![two.clone()], "PREMISE: named when BIG is listed second");
    // BIG is the baseline's play 0: the same cards in the other order.
    assert_eq!(
        duos_passes(&second, A1),
        duos_passes(&first, A1),
        "the order of the teammate's hand changes no trade"
    );
    assert_eq!(duos_passes(&tight, A1), vec![two], "A2 plays BIG, then the passed card");
}

// Finding 2: the baseline is the teammate's FIRST listed play of each card, the score any of them

/// Round 1: A1 holds only TWO_DROP at pass mana, A2 holds only Hit Job at exactly its mana, B2 has BIG
/// on the field, and with `own_unit` A2 has a Unit of its own, which `legal_actions` lists as Hit Job's
/// first target.
fn baseline_state(own_unit: bool) -> (DuosState, String) {
    let mut state = round_one("audit-duos-baseline", None);
    let two = set_hand(&mut state, A1, &[TWO_DROP]).remove(0);
    set_mana(&mut state, A1, DUOS_PASS_COST);
    set_hand(&mut state, A2, &[HIT_JOB]);
    set_mana(&mut state, A2, cost(HIT_JOB));
    if own_unit {
        field_unit(&mut state, A2, ONE_DROP, 1);
    }
    field_unit(&mut state, B2, BIG, 2);
    (state, two)
}

/// README, Duos: "An immediate pass is named only when it is worth it (R1723000): the teammate's best
/// follow-up with the card ... beats its best without it by more than the card's `hand_card_value` in
/// the seat's hand, `DUOS_PASS_MARGIN_PER_COST` per crystal of `pass_cost` and `DUOS_PASS_MIN_GAIN`."
/// R1723000: before it the seat passed "even one the teammate could not use better than a play it
/// already had".
///
/// A2's best play is Hit Job on B2's BIG, with or without the passed card, and it takes all of A2's
/// mana: the card adds to A2's best follow-up exactly what it adds lying in A2's hand. `mate_baseline`
/// scores only Hit Job's FIRST listed play, onto A2's own Unit, while `pass_score` also scores its
/// second, onto BIG, with the card in hand, so the gap between Hit Job's two targets is booked as the
/// pass's gain and A1 pays a crystal to move a card A2 has no mana for.
#[test]
fn r1723000_a_pass_is_weighed_against_the_teammates_best_play_not_its_first_listed_one() {
    for own_unit in [false, true] {
        let (state, two) = baseline_state(own_unit);
        let (standing, without, play) = a2_best(&state);
        let passed = apply(&state, A1, DuosActionBody::Pass { instance_id: two }).expect("the pass is legal");
        let (standing_with, with, play_with) = a2_best(&passed);
        // PREMISE: A2's best play is the same Hit Job with and without the card, and the card adds to it
        // only what it adds to A2's standing value from its hand.
        assert_eq!(play, play_with, "PREMISE ({own_unit}): the same best play");
        assert!(
            ((with - without) - (standing_with - standing)).abs() < 1e-9,
            "PREMISE ({own_unit}): the card adds its hand value and no more"
        );
        assert_eq!(
            duos_passes(&state, A1),
            Vec::<String>::new(),
            "own unit {own_unit}: A2's best with the card is its best without it, plus the card in hand"
        );
    }
}

// Finding 3 (the host's, shown here on the engine): a move decided on an older state is played on a newer one

/// §13.7: a seat passes "a hand card its teammate can play this turn: the teammate is a round seat that
/// has not ended", and "when the search would end the turn, the seat first makes its passes and then
/// ends it". `crates/shared/src/host/duos.rs` asks both AI seats for their move on one state
/// (`drive_ai` arms both pauses at once; its own tests' `asked` asserts it) and `on_ai_move` plays each
/// answer on the state as it stands then, deciding again only when the move is REFUSED. Both of these
/// stale moves are legal, so neither is decided again.
#[test]
fn s13_7_two_ai_seats_asked_on_one_state_do_not_waste_the_pass_between_them() {
    let seed = "audit-duos-stale";
    let mut state = round_two(seed);
    let two = set_hand(&mut state, B1, &[TWO_DROP]).remove(0);
    set_mana(&mut state, B1, DUOS_PASS_COST);
    set_hand(&mut state, B2, &[BIG]);
    set_mana(&mut state, B2, cost(BIG) - 1);

    // Both searches run on this one state, as the host asks them.
    let b1 = duos_decide(&state, B1, &mut opts(seed)).expect("B1 owes a move");
    let b2 = duos_decide(&state, B2, &mut opts(seed)).expect("B2 owes a move");
    assert_eq!(b1, DuosActionBody::Pass { instance_id: two }, "PREMISE: B1 passes B2 the card B2 can play");
    assert_eq!(b2, end_turn(), "PREMISE: B2, holding nothing it can play, ends its turn");

    // B1's answer lands first; B2's, decided before the pass, is still legal, so the host plays it.
    let after_pass = apply(&state, B1, b1.clone()).expect("the pass is legal");
    assert!(apply(&after_pass, B2, b2.clone()).is_ok(), "PREMISE: the stale End turn is not refused");
    let fresh = duos_decide(&after_pass, B2, &mut opts(seed));
    assert_eq!(
        json_of(&Some(b2.clone())),
        json_of(&fresh),
        "B2's move decided before the pass is not the move it owes after it: it now plays the card"
    );

    // Or B2's lands first; B1's pass, decided while B2 could still play the card, is still legal.
    let after_end = apply(&state, B2, b2).expect("the End turn is legal");
    assert!(apply(&after_end, B1, b1.clone()).is_ok(), "PREMISE: the stale pass is not refused");
    assert_eq!(
        json_of(&Some(b1)),
        json_of(&duos_decide(&after_end, B1, &mut opts(seed))),
        "B1's pass decided before B2 ended is not the move it owes after it: B2 has ended"
    );
}
