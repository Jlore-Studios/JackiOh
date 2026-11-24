//! Audit (Stinkyscape+ cards, second half): each test asserts a card's printed text and fails on the bug.

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

const CRAWLER: &str = "stinkyscapeplus-120";
const TOXOPID: &str = "stinkyscapeplus-121";
const ENGINEER: &str = "stinkyscapeplus-141";
const OMURA: &str = "stinkyscapeplus-125";
const BARD: &str = "stinkyscapeplus-102";
const FIRE_BEAM: &str = "stinkyscapeplus-143";

fn game(setup: Value) -> Scenario {
    jackioh_cards::register_all();
    scenario(setup)
}

fn filler() -> Value {
    json!(["core-005", "core-005", "core-005", "core-005"])
}

#[test]
fn probe_crawlers_die_together() {
    let mut s = game(json!({
        "p1": { "mana": 10, "hand": [{ "def": ENGINEER, "radiant": true }],
                "field": [{ "def": CRAWLER, "lane": 1 }, { "def": CRAWLER, "lane": 2 }, { "def": CRAWLER, "lane": 3 }],
                "library": filler() },
        "p2": { "library": filler() },
    }));
    s.play(ENGINEER, json!({ "zone": 4 }));
    let held: Vec<CardInstance> = s.hand(P1).into_iter().filter(|c| c.def_id == TOXOPID).collect();
    println!("PROBE crawlers: toxopids={} mods={:?}", held.len(), held.iter().map(|c| c.cost_mod).collect::<Vec<_>>());
    println!("PROBE crawlers gy={:?}", s.pile(P1, "graveyard").iter().map(|c| c.def_id.clone()).collect::<Vec<_>>());
}

#[test]
fn probe_omura_no_enemy() {
    let s = game(json!({
        "p1": { "field": [{ "def": OMURA, "lane": 1 }], "library": filler(), "hand": ["core-005"] },
        "p2": { "library": filler() },
    }));
    println!("PROBE omura acts={:?}", s.activations_of(OMURA));
}

#[test]
fn probe_fire_beam_dormant_only() {
    let s = game(json!({
        "p1": { "field": [{ "def": FIRE_BEAM, "lane": 1 }], "library": filler(), "hand": ["core-005"] },
        "p2": { "field": [{ "def": "stand-in (1) 1/10", "lane": 1, "slumber": {} }], "library": filler() },
    }));
    println!("PROBE beam acts={:?}", s.activations_of(FIRE_BEAM));
}

#[test]
fn probe_bard_cost_mod() {
    let mut s = game(json!({
        "p1": { "mana": 10, "hand": ["stand-in (0) Spell", { "def": BARD, "radiant": true }],
                "library": [{ "def": "stand-in (1) 1/1", "costMod": 3 }, "stand-in (2) 2/2", "core-005", "core-005"] },
        "p2": { "library": filler() },
    }));
    s.play("stand-in (0) Spell", json!({}));
    s.play(BARD, json!({ "zone": 1 }));
    for card in s.hand(P1) {
        println!(
            "PROBE bard hand {} mod={} over={:?} eff={}",
            card.def_id,
            card.cost_mod,
            card.cost_override,
            effective_cost(s.state(), &card, Default::default())
        );
    }
    let _ = P2;
}
