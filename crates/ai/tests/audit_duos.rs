//! Audit of the Duos AI (`crates/ai/src/duos.rs`). Exploratory, temporary.

use std::cell::Cell;

use jackioh_ai::{
    AI_EVAL, AiOptions, NextSwing, SearchBudget, decide, duos_decide, duos_decision, duos_passes,
    duos_pool_loss, evaluate, face_threat,
};
use jackioh_engine::config::{DUOS_HUMAN_TEAM, DUOS_PASS_COST};
use jackioh_engine::duos::{DuosState, duos_legal_actions, duos_reduce, duos_scenario};
use jackioh_engine::testkit::{
    ActionBody, DECK_SIZE, PlaceOnFieldOptions, PlayerId, PlayerState, Position, Row, Value, Zone, ZoneSlot,
    create_rng, def_of, effective_life, find_instance_mut, new_instance, place_on_field, plays_in,
    query_cost,
};
use jackioh_engine::wire::{DuosAction, DuosActionBody, DuosDecks, DuosSeat, DuosTeamRules, PerTeam};
use jackioh_engine::{Action, legal_actions, reduce_unpiloted};

const A1: DuosSeat = DuosSeat::A1;
const A2: DuosSeat = DuosSeat::A2;
const B1: DuosSeat = DuosSeat::B1;
const B2: DuosSeat = DuosSeat::B2;

const CHEAP: [&str; DECK_SIZE as usize] = [
    "core-003", "core-004", "core-005", "core-007", "core-008", "core-010", "core-011", "core-015",
    "core-018", "core-023", "core-031", "core-035", "core-039", "core-041", "core-044", "core-048",
    "core-051", "core-060", "core-062", "core-067", "core-071", "core-075", "core-081", "core-086",
    "core-096",
];
const ONE_DROP: &str = "core-008";
const BIG: &str = "core-025";

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

fn round_one(seed: &str, teams: Option<PerTeam<DuosTeamRules>>) -> DuosState {
    jackioh_cards::register_all();
    let state = duos_scenario(seed, decks(), teams);
    assert_eq!(state.round, 1, "{seed}: round 1 is open");
    state
}

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
    state
}

fn opts(seed: &str) -> AiOptions<'static> {
    AiOptions::new(create_rng(seed, 0))
}

fn json_of<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serializes")
}

fn duel(body: ActionBody) -> DuosActionBody {
    DuosActionBody::Duel { body }
}

fn end_turn() -> DuosActionBody {
    duel(ActionBody::EndTurn)
}

thread_local! {
    static NONCES: Cell<u32> = const { Cell::new(0) };
}

fn try_apply(state: &DuosState, seat: DuosSeat, body: DuosActionBody) -> Result<DuosState, String> {
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

fn apply(state: &DuosState, seat: DuosSeat, body: DuosActionBody) -> DuosState {
    let shown = json_of(&body);
    try_apply(state, seat, body).unwrap_or_else(|error| panic!("{seat}'s {shown} refused: {error}"))
}

fn side(state: &mut DuosState, seat: DuosSeat) -> &mut PlayerState {
    &mut state.duels[seat.duel()].players[seat.player()]
}

fn give(state: &mut DuosState, seat: DuosSeat, def_id: &str) -> String {
    let player = seat.player();
    let game = &mut state.duels[seat.duel()];
    let card = new_instance(game, def_id, player, Zone::Hand { player });
    let id = card.id.clone();
    game.players[player].hand.push(card);
    id
}

fn set_hand(state: &mut DuosState, seat: DuosSeat, defs: &[&str]) -> Vec<String> {
    side(state, seat).hand.clear();
    defs.iter().map(|def_id| give(state, seat, def_id)).collect()
}

fn set_mana(state: &mut DuosState, seat: DuosSeat, current: i32) {
    side(state, seat).mana.current = current;
}

fn cost(def_id: &str) -> i32 {
    query_cost(def_of(None, def_id))
}

fn field_unit(state: &mut DuosState, seat: DuosSeat, def_id: &str, lane: i32) -> String {
    let player = seat.player();
    let game = &mut state.duels[seat.duel()];
    let row = Row::Units;
    let mut unit = new_instance(game, def_id, player, Zone::Field { player, row, lane });
    let slot = ZoneSlot { player, row, lane };
    let placed = place_on_field(game, &mut unit, slot, PlaceOnFieldOptions::default());
    assert!(placed, "{seat}'s unit zone {lane} takes the unit");
    find_instance_mut(game, &unit.id)
        .expect("the placed unit")
        .position = Some(Position::Atk);
    unit.id
}

/// EXPLORE: the order `legal_actions` lists a targeted spell's plays in, and each one's score.
#[test]
fn explore_target_order() {
    let mut state = round_one("audit-duos-explore", None);
    set_hand(&mut state, A1, &[BIG]);
    set_mana(&mut state, A1, DUOS_PASS_COST);
    for spell in ["core-016", "core-035", "core-044", "core-063", "core-047", "core-034", "core-036"] {
        let mut s = state.clone();
        let own = field_unit(&mut s, A2, ONE_DROP, 1);
        let foe = field_unit(&mut s, B2, BIG, 2);
        let ids = set_hand(&mut s, A2, &[spell]);
        set_mana(&mut s, A2, 4);
        let duel1 = &s.duels[1];
        println!("== {spell} ({}) own={own} foe={foe}", def_of(None, spell).name);
        let base = evaluate(duel1, PlayerId::P1, NextSwing::Enemy, &AI_EVAL);
        println!("   standing value {base:.2}");
        for (at, action) in legal_actions(duel1, PlayerId::P1).into_iter().enumerate() {
            if !matches!(&action, ActionBody::Play { instance_id, .. } if *instance_id == ids[0]) {
                continue;
            }
            let applied = reduce_unpiloted(duel1, &Action::new(action.clone(), PlayerId::P1, format!("x{at}")));
            let value = evaluate(&applied.state, PlayerId::P1, NextSwing::Enemy, &AI_EVAL);
            println!("   {at}: {} -> {:.2} (err {:?})", json_of(&action), value, applied.error.map(|e| e.to_string()));
        }
        println!("   duos_passes(A1) = {:?}", duos_passes(&s, A1));
    }
    let _ = (plays_in(&state.duels[0], PlayerId::P1, "x"), effective_life(&state.duels[0], PlayerId::P1));
    let _ = (face_threat(&state.duels[0], PlayerId::P1), duos_pool_loss(&state, A1));
    let _ = (duos_legal_actions(&state, A1), None::<SearchBudget>, round_two as fn(&str, DuosTeamRules) -> DuosState);
    let _ = (decide(&state.duels[0], PlayerId::P1, &mut opts("x")), duos_decide(&state, A1, &mut opts("x")), duos_decision(&state, A1, &mut opts("x")));
    let _ = (B1, B2, try_apply as fn(&DuosState, DuosSeat, DuosActionBody) -> Result<DuosState, String>);
}
