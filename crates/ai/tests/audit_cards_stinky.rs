//! Audit (Stinkyscape cards, S #1 to #177 and the set's tokens).

use jackioh_engine::testkit::*;

#[allow(dead_code)]
const P1: PlayerId = PlayerId::P1;
#[allow(dead_code)]
const P2: PlayerId = PlayerId::P2;

const FILLER: &str = "core-005";

fn scenario_with_cards(opts: serde_json::Value) -> Scenario {
    jackioh_cards::register_all();
    scenario(opts)
}

fn filler(n: usize) -> Vec<&'static str> {
    vec![FILLER; n]
}

fn units_of(s: &Scenario, player: PlayerId) -> Vec<String> {
    (1..=5).filter_map(|lane| s.unit(player, lane)).map(|c| format!("{}", c.def_id)).collect()
}

#[test]
fn probe_inner_fire_negative() {
    const INNER_FIRE: &str = "stinkyscape-010-1";
    const PILLOW: &str = "core-065-1";
    let mut s = scenario_with_cards(json!({
        "p1": { "hand": [INNER_FIRE, FILLER], "field": [{ "def": "stand-in (1) 1/5", "lane": 1 }, { "def": PILLOW, "lane": 3 }],
                "library": filler(4), "mana": 10 },
        "p2": { "hand": [FILLER], "library": filler(4) },
    }));
    let unit = s.unit_at(P1, 1);
    eprintln!("before: {:?}", (s.stats(&unit).attack, s.stats(&unit).health));
    s.play(INNER_FIRE, pick::targets(pick::instance(&unit)));
    eprintln!("after: {:?}", (s.stats(&unit).attack, s.stats(&unit).health));
}

#[test]
fn probe_two_sacked() {
    const SACKED: &str = "stinkyscape-135";
    let mut s = scenario_with_cards(json!({
        "p1": { "hand": ["stand-in (1) 4/4", FILLER], "library": filler(4), "mana": 10 },
        "p2": { "hand": [FILLER], "backrow": [{ "def": SACKED, "lane": 1 }, { "def": SACKED, "lane": 2 }], "library": filler(4) },
    }));
    s.play("stand-in (1) 4/4", json!({ "zone": 2 }));
    eprintln!("p1 units: {:?}", units_of(&s, P1));
    eprintln!("p2 backrow: {:?}", (1..=5).filter_map(|l| s.backrow(P2, l)).map(|c| (c.def_id, c.face_up)).collect::<Vec<_>>());
    eprintln!("held: {:?}", s.state().held.iter().flatten().map(|h| h.card.def_id.clone()).collect::<Vec<_>>());
    eprintln!("events: {:?}", s.last_events().iter().map(|e| e.event_type().as_str().to_string()).collect::<Vec<_>>());
}

#[test]
fn probe_radiant_sacked_full_board() {
    const SACKED: &str = "stinkyscape-135";
    let v = "stand-in (1) 2/2";
    let mut s = scenario_with_cards(json!({
        "p1": { "hand": ["stand-in (1) 4/4", FILLER], "library": filler(4), "mana": 10 },
        "p2": { "hand": [FILLER], "field": [v, v, v, v, v], "backrow": [{ "def": SACKED, "radiant": true, "lane": 1 }], "library": filler(4) },
    }));
    s.play("stand-in (1) 4/4", json!({ "zone": 2 }));
    eprintln!("p1 units: {:?}", units_of(&s, P1));
    eprintln!("p2 units: {:?}", units_of(&s, P2));
    eprintln!("p2 backrow: {:?}", (1..=5).filter_map(|l| s.backrow(P2, l)).map(|c| (c.def_id, c.face_up)).collect::<Vec<_>>());
    eprintln!("held: {:?}", s.state().held.iter().flatten().map(|h| h.card.def_id.clone()).collect::<Vec<_>>());
    eprintln!("4/4 zone: {:?}", s.card("stand-in (1) 4/4").zone);
    eprintln!("events: {:?}", s.last_events().iter().map(|e| e.event_type().as_str().to_string()).collect::<Vec<_>>());
}

#[test]
fn probe_zzoth_replicators() {
    const ZZOTH: &str = "stinkyscape-070";
    const REPLICATORS: &str = "stinkyscapeplus-093";
    let mut s = scenario_with_cards(json!({
        "p1": { "hand": [ZZOTH, FILLER], "graveyard": [REPLICATORS, "stand-in (1) 4/4"], "library": filler(4), "mana": 10 },
        "p2": { "hand": [FILLER], "library": filler(4) },
    }));
    s.play(ZZOTH, json!({ "zone": 3 }));
    eprintln!("p1 units: {:?}", units_of(&s, P1));
    eprintln!("pending: {:?}", s.state().pending.is_some());
}
