//! Stinkyscape #151 Call to Chaos (Scape Edition)'s entry builders (`subsystems/call_to_chaos_scape.rs`;
//! the table, its first entry laying an Anomaly board, is the cards crate's data): the entries that summon or
//! add the set's cards, against `fixtures::call_to_chaos_stinkyscape`: the Corvettes, the Drone Battleship's
//! pile (R1842000), the Interceptor in every zone (R1882007), the Starship Pieces (R1878000), the Weapons.

use jackioh_engine::config::{
    CHAOS_SCAPE_CORVETTES, CHAOS_SCAPE_HAND_PIECES, CHAOS_SCAPE_WEAPONS, CHAOS_STINKYSCAPE_COST, UNIT_ZONES,
};
use jackioh_engine::subsystems::call_to_chaos_scape::{
    scape_battleship, scape_corvettes, scape_interceptors, scape_pieces, scape_weapons,
};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::P1;

use crate::rules::fixtures::call_to_chaos_stinkyscape::{
    CORVETTE, INTERCEPTOR, MELTDOWN, PIECES, PROPAGATION, SHIELD, WEAPONS, stinkyscape_game as game,
};
use crate::rules::fixtures::harness::{put, slot};

/// Run `effect` as p1's, from a resolving fixture card, then the state check.
fn run(state: &mut GameState, effect: Effect) -> Vec<GameEvent> {
    let me = new_instance(&mut *state, CORVETTE, P1, Zone::Resolving { player: P1 });
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(&mut *state, &mut events, &mut rng);
        let options = HookOptions {
            controller: Some(P1),
            ..HookOptions::default()
        };
        {
            let mut ctx = make_context(&mut sink, Some(&me), options);
            apply_effects(&[effect], &mut ctx);
        }
        state_check(&mut sink);
    }
    state.rng_cursor = rng.cursor();
    events
}

/// p1's unit zones, each pile's definitions top first, as the state holds a pile (§3.2).
fn piles(state: &GameState) -> Vec<Vec<String>> {
    state
        .players
        .p1
        .units
        .iter()
        .map(|pile| pile.iter().flatten().map(|card| card.def_id.clone()).collect())
        .collect()
}

#[test]
fn the_corvettes_entry_summons_three_drone_corvettes() {
    let mut state = game("scape-corvettes");
    run(&mut state, scape_corvettes());
    let corvettes = piles(&state).concat();
    assert_eq!(
        corvettes,
        vec![CORVETTE.to_string(); CHAOS_SCAPE_CORVETTES as usize]
    );
}

#[test]
fn r1842000_the_battleship_entry_summons_its_three_layers_as_one_pile_the_shield_on_top() {
    let mut state = game("scape-battleship");
    run(&mut state, scape_battleship());
    let piles: Vec<Vec<String>> = piles(&state)
        .into_iter()
        .filter(|pile| !pile.is_empty())
        .collect();
    assert_eq!(
        piles,
        vec![vec![
            SHIELD.to_string(),
            PROPAGATION.to_string(),
            MELTDOWN.to_string()
        ]]
    );
}

#[test]
fn r1882007_the_interceptors_entry_tops_every_unit_zone_a_unit_standing_in_one_included() {
    let mut state = game("scape-interceptors");
    put(&mut state, CORVETTE, slot(P1, Row::Units, 2), json!({}));
    run(&mut state, scape_interceptors());
    let piles = piles(&state);
    assert_eq!(piles.len(), UNIT_ZONES as usize);
    for (lane, pile) in piles.iter().enumerate() {
        assert_eq!(
            pile.first().map(String::as_str),
            Some(INTERCEPTOR),
            "lane {}",
            lane + 1
        );
    }
    assert_eq!(piles[1], vec![INTERCEPTOR.to_string(), CORVETTE.to_string()]);
}

#[test]
fn r1878000_the_pieces_entry_adds_four_starship_pieces_at_zero() {
    let mut state = game("scape-pieces");
    run(&mut state, scape_pieces());
    let hand = &state.players.p1.hand;
    assert_eq!(hand.len(), CHAOS_SCAPE_HAND_PIECES as usize);
    for card in hand {
        assert!(PIECES.contains(&card.def_id.as_str()), "{}", card.def_id);
        assert_eq!(card.cost_override, Some(CHAOS_STINKYSCAPE_COST));
    }
}

#[test]
fn the_weapons_entry_adds_two_radiant_weapons_at_zero() {
    let mut state = game("scape-weapons");
    run(&mut state, scape_weapons());
    let hand = &state.players.p1.hand;
    assert_eq!(hand.len(), CHAOS_SCAPE_WEAPONS as usize);
    for card in hand {
        assert!(WEAPONS.contains(&card.def_id.as_str()), "{}", card.def_id);
        assert!(card.radiant);
        assert_eq!(card.cost_override, Some(CHAOS_STINKYSCAPE_COST));
    }
}
