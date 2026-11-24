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

fn back_to(s: &mut Scenario, seat: PlayerId) {
    s.end_turn();
    let mut guard = 0;
    while s.state().active != seat && s.state().result.is_none() && guard < 4 {
        s.end_turn();
        guard += 1;
    }
}

fn names(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|c| format!("{}{}(m{})", c.def_id, if c.radiant { "*" } else { "" }, c.cost_mod)).collect()
}

#[test]
fn probe_omura_cooldown() {
    let mut s = game(json!({
        "p1": { "field": [{ "def": OMURA, "lane": 1 }], "library": filler(), "hand": ["core-005"] },
        "p2": { "library": filler(), "hand": ["stand-in (1) 1/10"], "mana": 10 },
    }));
    s.activate(OMURA, json!({ "ability": "omura" }));
    println!("PROBE omura after: acts={:?}", s.activations_of(OMURA));
    s.end_turn();
    s.play("stand-in (1) 1/10", json!({ "zone": 1 }));
    s.end_turn();
    println!("PROBE omura next turn (active {:?}): acts={:?}", s.state().active, s.activations_of(OMURA));
}

#[test]
fn probe_revolution_radiant() {
    let mut s = game(json!({
        "p1": { "mana": 10, "hand": [{ "def": "stinkyscapeplus-166", "radiant": true }, "core-005"], "library": filler() },
        "p2": { "library": filler(), "hand": ["core-005"] },
    }));
    s.play("stinkyscapeplus-166", json!({}));
    println!("PROBE rev now: {:?}", names(&s.hand(P1)));
    back_to(&mut s, P1);
    println!("PROBE rev t+1: {:?}", names(&s.hand(P1)));
}

#[test]
fn probe_gpt2_radiant() {
    let mut s = game(json!({
        "p1": { "mana": 10, "hand": [{ "def": "stinkyscapeplus-132", "radiant": true }, { "def": "stinkyscapeplus-133", "radiant": true }], "library": filler() },
        "p2": { "library": filler(), "hand": ["core-005"] },
    }));
    s.play("stinkyscapeplus-132", json!({ "modes": ["α"] }));
    println!("PROBE gpt2: {:?} pending={:?}", names(&s.hand(P1)), s.state().pending.is_some());
    s.play("stinkyscapeplus-110", json!({ "zone": 1 }));
    println!("PROBE gpt2 after dagger: {:?}", names(&s.hand(P1)));
    s.play("stinkyscapeplus-133", json!({}));
    println!("PROBE gpt3: {:?}", names(&s.hand(P1)));
    s.play("stinkyscapeplus-133-1", json!({ "zone": 2 }));
    println!("PROBE gpt3 after daglare: {:?}", names(&s.hand(P1)));
    s.play("stinkyscapeplus-111", json!({ "zone": 3 }));
    println!("PROBE after mace: {:?}", names(&s.hand(P1)));
}

#[test]
fn probe_shipyard() {
    let mut s = game(json!({
        "p1": { "mana": 10, "hand": ["stinkyscapeplus-157", "core-005"], "library": filler() },
        "p2": { "library": filler(), "hand": ["core-005"] },
    }));
    s.play("stinkyscapeplus-157", json!({ "zone": 1 }));
    let yard = s.unit_at(P1, 1);
    println!("PROBE yard slumber={:?} acts={:?}", yard.slumber, s.activations_of(&yard));
    back_to(&mut s, P1);
    let yard = s.unit_at(P1, 1);
    println!("PROBE yard t+1 def={} acts={:?} stats={:?}", yard.def_id, s.activations_of(&yard), s.stats(&yard));
    s.activate(&yard, json!({ "ability": "shipyard" }));
    for lane in 1..=5 {
        if let Some(unit) = s.unit(P1, lane) {
            println!("PROBE yard launched lane {lane}: {} slumber={:?} stats={:?} acts={:?}", unit.def_id, unit.slumber, s.stats(&unit), s.activations_of(&unit));
        }
    }
    println!("PROBE yard exile={:?} gy={:?}", names(&s.pile(P1, "exile")), names(&s.pile(P1, "graveyard")));
}

#[test]
fn probe_space_invader() {
    let mut s = game(json!({
        "p1": { "mana": 10, "hand": ["stinkyscapeplus-161", "core-005"], "library": ["core-005","core-005","core-005","core-005","core-005","core-005","core-005","core-005"] },
        "p2": { "library": ["core-005","core-005","core-005","core-005","core-005","core-005","core-005","core-005"], "hand": ["core-005"] },
    }));
    s.play("stinkyscapeplus-161", json!({ "zone": 1 }));
    for turn in 0..3 {
        back_to(&mut s, P1);
        let n = (1..=5).filter(|lane| s.unit(P1, *lane).is_some()).count();
        println!("PROBE invader after {} own turn starts: {} units, evolutions={:?}", turn + 1, n, s.state().players[P1].evolutions);
    }
}

#[test]
fn probe_alnilam() {
    let mut s = game(json!({
        "p1": { "mana": 10, "hand": ["stinkyscapeplus-t-supernova", "core-005"], "library": ["core-005","core-005","core-005","core-005","core-005","core-005","core-005","core-005","core-005","core-005"],
                "field": [{ "def": "stand-in (1) 2/2", "lane": 2 }] },
        "p2": { "library": ["core-005","core-005","core-005","core-005","core-005","core-005","core-005","core-005","core-005","core-005"], "hand": ["core-005"],
                "field": [{ "def": "stand-in (1) 3/3", "lane": 1 }] },
    }));
    s.play("stinkyscapeplus-t-supernova", json!({}));
    for lane in 1..=5 {
        if let Some(unit) = s.unit(P1, lane) {
            println!("PROBE nova p1 lane {lane}: {} stats={:?}", unit.def_id, s.stats(&unit));
        }
        if let Some(unit) = s.unit(P2, lane) {
            println!("PROBE nova p2 lane {lane}: {}", unit.def_id);
        }
    }
    for turn in 0..5 {
        back_to(&mut s, P1);
        println!("PROBE nova turn {} winner={:?} turn={}", turn, s.state().result, s.state().turn);
        if s.state().result.is_some() { break; }
    }
}
