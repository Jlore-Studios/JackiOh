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

fn dump(s: &Scenario, who: &str, card: &CardInstance) {
    let c = s.card(card).clone();
    let v = s.stats(&c);
    eprintln!("{who}: def={} zone={:?} atk={} hp={} kws={:?} slumber={:?} radiant={}", c.def_id, c.zone, v.attack, v.health, v.keywords, c.slumber, c.radiant);
}

#[test]
fn probe_tnack_copies_shockwave() {
    const TNACK: &str = "stinkyscape-020";
    const SHOCK: &str = "stinkyscape-111";
    let mut s = scenario_with_cards(json!({
        "p1": { "hand": [TNACK, FILLER], "field": [{ "def": SHOCK, "lane": 1 }], "library": filler(4), "mana": 10 },
        "p2": { "hand": [FILLER], "library": filler(4) },
    }));
    let shock = s.unit_at(P1, 1);
    dump(&s, "shock", &shock);
    s.play(TNACK, json!({ "zone": 3, "targets": pick::instance(&shock) }));
    let tnack = s.unit_at(P1, 3);
    dump(&s, "tnack", &tnack);
    eprintln!("tnack memory: {:?}", s.card(&tnack).memory);
}

#[test]
fn probe_tnack_copies_starship() {
    const TNACK: &str = "stinkyscape-020";
    const AP: &str = "stinkyscape-100";
    const HIT_JOB: &str = "core-016";
    let mut s = scenario_with_cards(json!({
        "p1": { "hand": [HIT_JOB, TNACK, FILLER], "field": [{ "def": AP, "lane": 2 }], "library": filler(4), "mana": 10 },
        "p2": { "hand": [FILLER], "library": filler(4) },
    }));
    for player in [P1, P2] { s.state_mut().players[player].auto_end_turn = Some(false); }
    let piece = s.unit_at(P1, 2);
    s.play(HIT_JOB, json!({ "targets": pick::instance(&piece) }));
    let ship = jackioh_engine::prelude::dormant_starship_of(s.state(), P1).expect("ship").clone();
    dump(&s, "ship dormant", &ship);
    s.state_mut().players[P1].mana.current = 10;
    eprintln!("tnack targets before launch: {:?}", s.play_targets(TNACK));
    s.activate(&ship, json!({ "ability": jackioh_engine::prelude::STARSHIP_LAUNCH_ABILITY }));
    dump(&s, "ship launched", &ship);
    s.state_mut().players[P1].mana.current = 10;
    eprintln!("tnack targets after launch: {:?}", s.play_targets(TNACK));
    s.play(TNACK, json!({ "zone": 3, "targets": pick::instance(&ship) }));
    let tnack = s.unit_at(P1, 3);
    dump(&s, "tnack", &tnack);
    eprintln!("tnack memory: {:?}", s.card(&tnack).memory);
    eprintln!("tnack activations: {:?}", s.activations_of(&tnack));
    eprintln!("tnack attacks: {:?}", s.attacks_of(&tnack).len());
    s.end_turn();
    s.end_turn();
    dump(&s, "tnack next turn", &tnack);
    eprintln!("tnack attacks next turn: {:?}", s.attacks_of(&tnack).len());
    eprintln!("tnack activations: {:?}", s.activations_of(&tnack));
}

#[test]
fn probe_kwzka_dormant() {
    const KWZKA: &str = "stinkyscape-037";
    const SMITH: &str = "stinkyscape-017";
    let mut s = scenario_with_cards(json!({
        "p1": { "hand": [SMITH, FILLER, FILLER], "library": filler(6), "mana": 10 },
        "p2": { "hand": [FILLER, FILLER], "field": [{ "def": KWZKA, "lane": 1 }], "library": filler(6) },
    }));
    for player in [P1, P2] { s.state_mut().players[player].auto_end_turn = Some(false); }
    s.play(SMITH, json!({ "zone": 2 }));
    let smith = s.card(SMITH).clone();
    dump(&s, "smith", &smith);
    eprintln!("delayed: {:?}", s.state().delayed.len());
    eprintln!("events: {:?}", s.last_events().iter().map(|e| e.event_type().as_str().to_string()).collect::<Vec<_>>());
    s.end_turn();
    dump(&s, "smith after p2 start", &smith);
    eprintln!("smith controller: {:?}", s.card(&smith).controller);
    eprintln!("events: {:?}", s.last_events().iter().map(|e| e.event_type().as_str().to_string()).collect::<Vec<_>>());
}

#[test]
fn probe_anomaly_drones() {
    const ANOMALY: &str = "stinkyscape-t-anomaly-found";
    for (i, radiant) in [(0, false), (1, false), (2, false), (3, false), (4, true), (5, true), (6, true), (7, false), (8, false), (9, true)] {
        let mut s = scenario_with_cards(json!({
            "seed": format!("anomaly-{i}"),
            "p1": { "hand": [{ "def": ANOMALY, "radiant": radiant }, FILLER], "library": filler(6), "mana": 10 },
            "p2": { "hand": [FILLER], "field": ["stand-in (1) 4/4"], "library": filler(6) },
        }));
        s.play(ANOMALY, json!({}));
        let mut guard = 0;
        while s.state().pending.is_some() && guard < 20 {
            let first = s.state().pending.as_ref().unwrap().options[0].key.clone();
            s.answer(json!(first));
            guard += 1;
        }
        let code = s.events_of("boardReplaced").len();
        let mut out = vec![];
        for row in [Row::Units, Row::Backrow] {
            for lane in 1..=5 {
                let card = if row == Row::Units { s.unit(P1, lane) } else { s.backrow(P1, lane) };
                if let Some(card) = card {
                    let d = &card.def_id;
                    if d.contains("drone") || d.contains("111") || d.contains("layer") {
                        let v = s.stats(&card);
                        out.push(format!("{}{}:{:?} mem={:?}", if card.radiant { "R " } else { "" }, d.replace("stinkyscape-t-", ""), v.keywords, card.memory.keys().collect::<Vec<_>>()));
                    }
                }
            }
        }
        eprintln!("seed {i} radiant={radiant} replaced={code}: {out:?}");
    }
}

#[test]
fn probe_stimmy_trade() {
    use jackioh_engine::prelude::INFLATION_CHANCE_KEY;
    const STIMMY: &str = "stinkyscape-153";
    const UNION: &str = "stinkyscape-165";
    let mut s = scenario_with_cards(json!({
        "p1": { "hand": [STIMMY, FILLER], "backrow": [{ "def": UNION, "faceUp": true }], "library": filler(6), "mana": 10 },
        "p2": { "hand": [FILLER], "library": filler(6) },
    }));
    let id = s.card(STIMMY).id.clone();
    s.card_mut(STIMMY).memory.insert(INFLATION_CHANCE_KEY.to_string(), json!(40));
    eprintln!("before: zone={:?} mem={:?}", s.card(id.as_str()).zone, s.card(id.as_str()).memory);
    s.trade(id.as_str());
    eprintln!("after trade: zone={:?} mem={:?}", s.card(id.as_str()).zone, s.card(id.as_str()).memory);
}

#[test]
fn probe_tnack_keenus() {
    const TNACK: &str = "stinkyscape-020";
    const KEENUS: &str = "meditative-065";
    let mut s = scenario_with_cards(json!({
        "p1": { "hand": [KEENUS, TNACK, FILLER], "library": filler(4), "mana": 10 },
        "p2": { "hand": [FILLER], "library": filler(4) },
    }));
    eprintln!("keenus plays: {:?}", s.plays_of(KEENUS).len());
    s.play(KEENUS, json!({ "zone": 1 }));
    let keenus = s.unit_at(P1, 1);
    dump(&s, "keenus", &keenus);
    s.state_mut().players[P1].mana.current = 10;
    s.play(TNACK, json!({ "zone": 3, "targets": pick::instance(&keenus) }));
    let tnack = s.unit_at(P1, 3);
    dump(&s, "tnack", &tnack);
    eprintln!("tnack memory: {:?}", s.card(&tnack).memory);
    eprintln!("tnack view text: {:?}", s.view(P1).you.units.iter().flatten().map(|u| format!("{:?}", u)).collect::<Vec<_>>().join(" || ").chars().take(1500).collect::<String>());
}

#[test]
fn probe_ticketed_radiant_brittle() {
    const TICKETED: &str = "stinkyscape-009";
    const IKWZKI: &str = "stinkyscape-110";
    const HANGER: &str = "stinkyscape-168";
    let mut s = scenario_with_cards(json!({
        "p1": { "hand": [TICKETED, IKWZKI, FILLER, FILLER, FILLER], "backrow": [{ "def": HANGER, "faceUp": true, "lane": 2 }], "library": filler(12), "mana": 10 },
        "p2": { "hand": [FILLER, FILLER, FILLER], "library": filler(12) },
    }));
    for player in [P1, P2] { s.state_mut().players[player].auto_end_turn = Some(false); }
    let hanger = s.backrow(P1, 2).unwrap();
    s.play(TICKETED, pick::targets(pick::instance(&hanger)));
    let ticketed = s.card(TICKETED).clone();
    eprintln!("ticketed zone={:?} brittle={:?} radiant={}", s.card(&ticketed).zone, s.card(&ticketed).brittle, s.card(&ticketed).radiant);
    s.play(IKWZKI, json!({ "zone": 1, "targets": pick::instance(&ticketed) }));
    for round in 0..5 {
        s.end_turn();
        s.end_turn();
        let t = s.card(&ticketed).clone();
        eprintln!("round {round} turn {}: ticketed zone={:?} brittle={:?} radiant={} | view brittle {:?}", s.state().turn, t.zone, t.brittle, t.radiant,
            s.view(P1).you.backrow.iter().flatten().map(|b| format!("{:?}", b).chars().take(0).collect::<String>()).count());
    }
}

#[test]
fn probe_sacked_equip() {
    const SACKED: &str = "stinkyscape-135";
    const AMP: &str = "meditative-065";
    for radiant in [false, true] {
        let mut s = scenario_with_cards(json!({
            "p1": { "hand": [AMP, FILLER], "field": [{ "def": "stand-in (1) 4/4", "lane": 1 }], "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "backrow": [{ "def": SACKED, "radiant": radiant, "lane": 1 }], "library": filler(4) },
        }));
        s.play(AMP, json!({ "zone": 1, "equip": true }));
        eprintln!("radiant={radiant} p1 units: {:?}", units_of(&s, P1));
        eprintln!("p2 units: {:?}", units_of(&s, P2));
        eprintln!("p2 backrow: {:?}", (1..=5).filter_map(|l| s.backrow(P2, l)).map(|c| (c.def_id, c.face_up)).collect::<Vec<_>>());
        eprintln!("held: {:?}", s.state().held.iter().flatten().map(|h| h.card.def_id.clone()).collect::<Vec<_>>());
        eprintln!("events: {:?}", s.last_events().iter().map(|e| e.event_type().as_str().to_string()).collect::<Vec<_>>());
    }
}
