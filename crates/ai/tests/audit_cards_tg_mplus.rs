//! Audit (Training Grounds and Meditative+ cards): exploratory probes.

use jackioh_engine::prelude::*;
use jackioh_engine::testkit::*;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;
const FILLER: &str = "core-005";

fn game(opts: serde_json::Value) -> Scenario {
    jackioh_cards::register_all();
    scenario(opts)
}

fn hp(s: &Scenario, p: PlayerId) -> i32 {
    s.state().players[p].hero.health
}

fn dump_hand(s: &Scenario, p: PlayerId) -> Vec<(String, bool, Option<i32>, i32)> {
    s.hand(p)
        .iter()
        .map(|c| (c.def_id.clone(), c.radiant, c.cost_override, c.cost_mod))
        .collect()
}

/// Probe 1: Dogpile answers a Spell that exiles itself (Legacy: True Strike).
#[test]
fn probe_dogpile_after_true_strike() {
    const DOGPILE: &str = "meditativeplus-017";
    const STRIKE: &str = "traininggrounds-058";
    let mut s = game(json!({
        "seed": "p1",
        "p1": { "hand": [STRIKE, DOGPILE], "library": [FILLER, FILLER, FILLER], "mana": 10 },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
    }));
    let before = hp(&s, P2);
    s.play(STRIKE, pick::targets(pick::hero("p2")));
    println!("P1 hp loss on p2: {}", before - hp(&s, P2));
    println!("hand: {:?}", dump_hand(&s, P1));
    assert_eq!(before - hp(&s, P2), 6, "True Strike 4 + Dogpile 2");
}

/// Probe 2: Wondrous Wand draws Cognitive Offloading.
#[test]
fn probe_wand_draws_offloading() {
    const WAND: &str = "meditativeplus-003";
    const OFFLOADING: &str = "meditativeplus-018";
    const A: &str = "stand-in (9) 1/1";
    const B: &str = "stand-in (8) 2/2";
    let mut s = game(json!({
        "seed": "p2",
        "p1": { "hand": [{ "def": WAND, "radiant": true }], "library": [OFFLOADING, A, B, FILLER, FILLER, FILLER, FILLER], "mana": 1 },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
    }));
    s.play(WAND, json!({}));
    println!("active {:?} turn {} phase {:?}", s.state().active, s.state().turn, s.state().phase);
    println!("p1 hand: {:?}", dump_hand(&s, P1));
    println!("p1 field: {:?}", active_units_of(s.state(), P1).iter().map(|c| c.def_id.clone()).collect::<Vec<_>>());
    for e in s.events() {
        let t = e.event_type().as_str().to_string();
        if t.contains("ost") || t.contains("rawn") || t.contains("layed") || t.contains("utopilot") {
            println!("  ev {}", serde_json::to_string(e).unwrap());
        }
    }
}

/// Probe 3: Radiant Bloodbloom gives KY's Competition Echo 1.
#[test]
fn probe_bloodbloom_echo_kys_competition() {
    const BLOOM: &str = "meditativeplus-006";
    const KY: &str = "meditativeplus-020";
    let mut s = game(json!({
        "seed": "p3",
        "p1": { "hand": [{ "def": BLOOM, "radiant": true }, KY, FILLER], "library": [FILLER, FILLER, FILLER], "mana": 10 },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
    }));
    s.play(BLOOM, json!({}));
    s.play(KY, json!({}));
    let mut asked = 0;
    while let Some(p) = s.state().pending.clone() {
        asked += 1;
        if asked > 80 {
            break;
        }
        let key = answer_key_of(&p.resume.data).expect("key");
        // two right answers then Stop each sequence
        let streak = p.streak.unwrap_or(0);
        if streak < 2 {
            s.answer(json!(format!("mode:{key}")));
        } else {
            let wrong = ["A", "B", "C", "D"].into_iter().find(|l| *l != key).unwrap();
            s.answer(json!(format!("mode:{wrong}")));
        }
    }
    let kennies: Vec<(i32, i32)> = active_units_of(s.state(), P1)
        .into_iter()
        .map(|c| {
            let v = unit_view(s.state(), c);
            (v.attack, v.health)
        })
        .collect();
    println!("asked {asked} kennies {kennies:?} hp {}", hp(&s, P1));
    assert_eq!(kennies, vec![(2, 2), (2, 2)], "Echo 1: the Spell resolves twice");
}

/// Probe 4: Radiant Bloodbloom + Legacy Reminisce.
#[test]
fn probe_bloodbloom_echo_reminisce() {
    const BLOOM: &str = "meditativeplus-006";
    const REM: &str = "traininggrounds-055";
    const A: &str = "stand-in (3) 9/9";
    const B: &str = "stand-in (4) 7/7";
    const C: &str = "stand-in (2) 2/2";
    let mut s = game(json!({
        "seed": "p4",
        "p1": { "hand": [{ "def": BLOOM, "radiant": true }, REM, FILLER], "graveyard": [A, B, C], "library": [FILLER, FILLER, FILLER], "mana": 10 },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
    }));
    s.play(BLOOM, json!({}));
    s.play(REM, json!({}));
    let mut n = 0;
    while let Some(p) = s.state().pending.clone() {
        n += 1;
        if n > 5 {
            break;
        }
        println!("prompt {} options {:?}", p.prompt, p.options);
        let first = serde_json::to_value(&p.options[0]).unwrap();
        s.answer(first);
    }
    println!("prompts {n} hand {:?}", dump_hand(&s, P1));
    println!("exile {:?}", s.pile(P1, "exile").iter().map(|c| c.def_id.clone()).collect::<Vec<_>>());
    println!("gy {:?}", s.pile(P1, "graveyard").iter().map(|c| c.def_id.clone()).collect::<Vec<_>>());
}

/// Probe 5: Radiant Bloodbloom + In a hurry (Echo).
#[test]
fn probe_bloodbloom_echo_in_a_hurry() {
    const BLOOM: &str = "meditativeplus-006";
    const HURRY: &str = "traininggrounds-015";
    let mut s = game(json!({
        "seed": "p5",
        "p1": { "hand": [{ "def": BLOOM, "radiant": true }, HURRY, FILLER], "library": [FILLER, FILLER, FILLER], "mana": 10, "health": 30 },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
    }));
    s.play(BLOOM, json!({}));
    s.play(HURRY, json!({}));
    println!("units {}", active_units_of(s.state(), P1).len());
    s.end_turn();
    println!("units after end {}", active_units_of(s.state(), P1).len());
    assert_eq!(active_units_of(s.state(), P1).len(), 0);
}

/// Probe 6: Cognitive Offloading played from the hand under a Radiant Bloodbloom (Echo 1).
#[test]
fn probe_offloading_from_hand_with_echo() {
    const BLOOM: &str = "meditativeplus-006";
    const OFFLOADING: &str = "meditativeplus-018";
    let mut s = game(json!({
        "seed": "p6",
        "p1": { "hand": [{ "def": BLOOM, "radiant": true }, OFFLOADING, "stand-in (1) 4/4"], "library": [FILLER, FILLER, FILLER, FILLER, FILLER], "mana": 10 },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
    }));
    s.play(BLOOM, json!({}));
    s.play(OFFLOADING, json!({}));
    println!("active {:?} turn {} hp {} hand {:?}", s.state().active, s.state().turn, hp(&s, P1), dump_hand(&s, P1));
}

/// Probe 7: Spirit Lash by Flashback under a Garden.
#[test]
fn probe_spirit_lash_flashback_under_garden() {
    const LASH: &str = "meditativeplus-005";
    const GARDEN: &str = "meditativeplus-022";
    let s = game(json!({
        "seed": "p7",
        "p1": { "hand": [FILLER], "graveyard": [LASH], "backrow": [{ "def": GARDEN, "faceUp": true }], "library": [FILLER, FILLER, FILLER], "mana": 10 },
        "p2": { "hand": [FILLER], "field": ["stand-in (1) 4/4"], "library": [FILLER, FILLER, FILLER] },
    }));
    for a in s.offered(P1) {
        println!("offered {}", serde_json::to_string(&a).unwrap());
    }
}
