//! Audit (decks area): exploration.

use jackioh_ai::{
    AI_GATE_BUDGET, AiOptions, DEALT_Q, FIXTURE_BANNED_IDS, SHADOW_BAN, SHADOW_UNBANNED, SHADOW_WATCH,
    UNBANNED_PREFER, play_ai_turn,
};
use jackioh_engine::testkit::{
    Action, CatalogQueryArgs, PlayerId, create_rng, hash_state, json, query, reduce, scenario,
};

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

#[test]
fn explore_table_ids() {
    jackioh_cards::register_all();
    let pool: Vec<String> = query(&CatalogQueryArgs::default()).iter().map(|d| d.id.clone()).collect();
    for (name, ids) in [
        ("SHADOW_BAN", SHADOW_BAN.iter().map(|(id, _)| *id).collect::<Vec<_>>()),
        ("SHADOW_WATCH", SHADOW_WATCH.iter().map(|(id, _)| *id).collect()),
        ("SHADOW_UNBANNED", SHADOW_UNBANNED.to_vec()),
        ("FIXTURE_BANNED_IDS", FIXTURE_BANNED_IDS.to_vec()),
        ("DEALT_Q", DEALT_Q.iter().map(|(id, _)| *id).collect()),
        ("UNBANNED_PREFER", UNBANNED_PREFER.to_vec()),
    ] {
        let missing: Vec<&&str> = ids.iter().filter(|id| !pool.iter().any(|p| p == **id)).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        eprintln!("{name}: {} ids, missing from the deck-legal pool {missing:?}, sorted+unique {}", ids.len(), sorted == ids);
    }
}

#[test]
fn explore_play_ai_turn_twice() {
    jackioh_cards::register_all();
    let start = scenario(json!({
        "seed": "audit-turns",
        "active": "p1",
        "turn": 9,
        "p1": { "hand": ["core-025", "core-011"], "mana": 4, "library": ["core-008", "core-011", "core-019"] },
        "p2": { "hand": ["core-025", "core-011"], "mana": 4, "library": ["core-008", "core-011", "core-019"] },
    }))
    .state()
    .clone();
    let mut ai = AiOptions::with_budget(create_rng("audit-turns", 0), AI_GATE_BUDGET);
    let first = play_ai_turn(&start, P1, &mut ai);
    eprintln!("first: {:?}", first.actions.iter().map(|a| (a.nonce.clone(), a.body.clone())).collect::<Vec<_>>());
    let second = play_ai_turn(&first.state, P2, &mut ai);
    eprintln!("second: {:?}", second.actions.iter().map(|a| (a.nonce.clone(), a.body.clone())).collect::<Vec<_>>());
    let mut current = first.state.clone();
    for action in &second.actions {
        let next = reduce(&current, &Action::new(action.body.clone(), P2, action.nonce.clone()));
        eprintln!("{} changed {}", action.nonce, hash_state(&next.state) != hash_state(&current));
        current = next.state;
    }
}
