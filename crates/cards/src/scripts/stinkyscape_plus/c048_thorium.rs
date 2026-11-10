//! SPEC §8.12 S+ #48 Thorium (§6.1 Brittle X, R385, R441; §6.3 Buff, Give Brittle; R81, R1420,
//! R3535000, R3547000): (1) Spell, Epic, Mindustrial, Mineral, a card of the held Stinkyscape+ set.
//!   Base:    "Give a Unit +{buff}/+{buff} and Brittle {brittle2}." — 5, 1.
//!   Radiant: "Give a Unit +{buff2}/+{buff2} and Brittle {brittle2}." — 10, 1.
//!
//! Engine: a declared target, any Unit on either side (R81, R3547000), then in the text's order a
//! `buff` of attack and health by the same number and `give_brittle`, which sets the Unit's count to
//! N from now, whatever it had (R385: "Give Brittle N", never `gain_brittle`'s "+N"). The count's
//! first tick waits a turn cycle (`BRITTLE_FIRST_TICK_TURNS`), and at 0 the Unit is destroyed. The
//! faces differ in the stats number alone, `buff` and `buff2`, each tuned on its own face;
//! `brittle2` is the one count both read.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-048";

/// R81, R3547000: any Unit on either side, picked with the play.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
}

/// Both faces run the same two verbs; `key` names the stats number the face prints.
fn thorium(key: &'static str) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            let n = param(&*ctx, key);
            vec![
                buff(json_as(json!({ "target": { "of": "chosen" }, "attack": n, "health": n }))),
                give_brittle(json_as(json!({ "target": { "of": "chosen" }, "n": param(&*ctx, "brittle2") }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: thorium("buff"),
        radiant: thorium("buff2"),
    }
}

// S+ #48 Thorium — SPEC §8.12 row S+ 48 (R81, R385, R3547000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{scenario, upgrade_number};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const THORIUM: &str = "stinkyscapeplus-048";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const BRITTLE: &str = "stand-in (1) 4/4 Brittle 3";
    const FILLER: &str = "core-005";

    fn board(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": THORIUM, "radiant": radiant }, FILLER], "field": mine },
            "p2": { "hand": [FILLER], "field": theirs },
        }))
    }

    fn on(card: &CardInstance) -> Value {
        json!({ "targets": pick::instance(card) })
    }

    fn brittle(s: &Scenario, card: &CardInstance) -> Option<i32> {
        s.card(card).brittle.map(|counter| counter.count)
    }

    fn offered(s: &Scenario) -> Vec<Selection> {
        let thorium = s.card(THORIUM).id.clone();
        legal_actions(s.state(), P1)
            .into_iter()
            .flat_map(|action| match action {
                ActionBody::Play { instance_id, targets, .. } if instance_id == thorium => targets.unwrap_or_default(),
                _ => Vec::new(),
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r3547000_it_offers_any_unit_on_either_side_and_no_hero() {
            let s = board(false, json!([VANILLA]), json!([VANILLA]));
            let offered = offered(&s);
            for unit in [s.unit_at(P1, 1), s.unit_at(P2, 1)] {
                assert!(offered.contains(&Selection::Instance { instance_id: unit.id }));
            }
            assert!(!offered.iter().any(|pick| matches!(pick, Selection::Hero { .. })));
        }

        #[test]
        fn stats_up_5_5_and_brittle_1() {
            let mut s = board(false, json!([VANILLA]), json!([]));
            let unit = s.unit_at(P1, 1);
            assert_eq!(brittle(&s, &unit), None);
            s.play(THORIUM, on(&unit));
            s.expect_stats(&unit, json!({ "attack": 9, "health": 9 }));
            assert_eq!(s.stats(&unit).max_health, 9);
            assert_eq!(brittle(&s, &unit), Some(1));
            s.expect_in_zone(&unit, "field");
        }

        #[test]
        fn an_enemy_unit_takes_both() {
            let mut s = board(false, json!([]), json!([VANILLA]));
            let theirs = s.unit_at(P2, 1);
            s.play(THORIUM, on(&theirs));
            s.expect_stats(&theirs, json!({ "attack": 9, "health": 9 }));
            assert_eq!(brittle(&s, &theirs), Some(1));
        }

        #[test]
        fn it_gives_brittle_one_and_never_adds_it_to_a_count_in_force() {
            let mut s = board(false, json!([BRITTLE]), json!([]));
            let unit = s.unit_at(P1, 1);
            assert_eq!(brittle(&s, &unit), Some(3), "the printed Brittle 3 is running");
            s.play(THORIUM, on(&unit));
            assert_eq!(brittle(&s, &unit), Some(1), "R385: Give Brittle N sets the count to N");
        }

        #[test]
        fn the_numbers_read_through_param() {
            let mut s = board(false, json!([VANILLA]), json!([]));
            assert_eq!(upgrade_number(&mut s, THORIUM, "buff"), 6);
            assert_eq!(upgrade_number(&mut s, THORIUM, "brittle2"), 2);
            let unit = s.unit_at(P1, 1);
            s.play(THORIUM, on(&unit));
            s.expect_stats(&unit, json!({ "attack": 10, "health": 10 }));
            assert_eq!(brittle(&s, &unit), Some(2));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn stats_up_10_10_and_brittle_1() {
            let mut s = board(true, json!([VANILLA]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(THORIUM, on(&unit));
            s.expect_stats(&unit, json!({ "attack": 14, "health": 14 }));
            assert_eq!(s.stats(&unit).max_health, 14);
            assert_eq!(brittle(&s, &unit), Some(1));
        }

        #[test]
        fn the_numbers_read_through_param_on_their_own_face() {
            let mut s = board(true, json!([VANILLA]), json!([]));
            assert_eq!(upgrade_number(&mut s, THORIUM, "buff2"), 11);
            assert_eq!(upgrade_number(&mut s, THORIUM, "brittle2"), 2);
            let unit = s.unit_at(P1, 1);
            s.play(THORIUM, on(&unit));
            s.expect_stats(&unit, json!({ "attack": 15, "health": 15 }));
            assert_eq!(brittle(&s, &unit), Some(2));
        }
    }
}
