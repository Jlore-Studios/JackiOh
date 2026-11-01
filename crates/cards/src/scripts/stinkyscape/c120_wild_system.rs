//! S #120 Wild System (SPEC §8.10 row S 120; §4.2, §4.5, R53, R1848004, R1848005, R1857001): (3)
//! Spell, Epic.
//!   Base:    "All Units attack each other until only one survives. Gain {mana} mana next turn." — 1.
//!   Radiant: the same text — 2.
//!
//! Engine: `melee` (R1848005): forced attacks (R53) between pairs drawn from the match rng, allies
//! as good a target as enemies, each its own combat and state check, until one Unit or none is left,
//! none may attack another, or `MELEE_MAX_ATTACKS` (20) attacks are made; no hero is a target
//! (ST-S14-9). Then `next_turn_mana` of `mana` on its caster (R1848004). The designer wrote one text
//! on both faces, which R276 refuses, so the Radiant face's declared number is 2 (R1857001): both

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-120";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![
                melee(Default::default()),
                next_turn_mana(json_as(json!({ "amount": param(&*ctx, "mana") }))),
            ]
        })),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S #120 Wild System — SPEC §8.10 row S 120, BUILD M12 row S 120.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario, upgrade_number};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const WILD: &str = "stinkyscape-120";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const GREASER: &str = "meditative-100"; // (2) Unit 7/7.
    const GENN: &str = "classic-086"; // (4) Unit 14/14.
    const STATE_OF_THE_GAME: &str = "classic-041"; // (1) Unit 3/3, Indestructible.
    const FILLER: &str = "core-005";

    fn forced(s: &Scenario) -> Vec<(String, String)> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AttackDeclared {
                    attacker_id,
                    target_id,
                    forced: true,
                    ..
                } => Some((attacker_id.clone(), target_id.clone())),
                _ => None,
            })
            .collect()
    }

    fn units_left(s: &Scenario) -> usize {
        (1..=5)
            .flat_map(|lane| [s.unit(P1, lane), s.unit(P2, lane)])
            .flatten()
            .count()
    }

    fn refresh_label(s: &Scenario) -> Option<String> {
        s.view(P1)
            .you
            .modifiers
            .iter()
            .find(|modifier| modifier.id == "nextTurnMana")
            .map(|modifier| modifier.label.clone())
    }

    fn wild_board(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        crate::register_all();
        scenario(json!({
            "p1": {
                "hand": [{ "def": WILD, "radiant": radiant }, FILLER],
                "field": mine,
                "library": filler(4),
            },
            "p2": { "hand": [FILLER], "field": theirs, "library": filler(4) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r1848005_allies_fight_each_other() {
            let mut s = wild_board(false, json!([VANILLA, GREASER]), json!([]));
            let (small, big) = (s.unit_at(P1, 1), s.unit_at(P1, 2));

            s.play(WILD, json!({}));

            let attacks = forced(&s);
            assert_eq!(attacks.len(), 1, "{attacks:?}");
            let pair = (small.id.clone(), big.id.clone());
            assert!(attacks[0] == pair || attacks[0] == (pair.1.clone(), pair.0.clone()));
            s.expect_in_zone(&small, "graveyard");
            s.expect_stats(&big, json!({ "attack": 7, "health": 3 }));
        }

        #[test]
        fn r1848005_four_units_until_one_or_none_is_left() {
            let mut s = wild_board(false, json!([VANILLA, GENN]), json!([GREASER, VANILLA]));

            s.play(WILD, json!({}));

            assert!(!forced(&s).is_empty());
            assert!(units_left(&s) <= 1, "{} left", units_left(&s));
        }

        #[test]
        fn r1848005_never_more_than_20_attacks() {
            let mut s = wild_board(false, json!([STATE_OF_THE_GAME]), json!([STATE_OF_THE_GAME]));

            s.play(WILD, json!({}));

            assert_eq!(forced(&s).len(), MELEE_MAX_ATTACKS as usize);
            assert_eq!(units_left(&s), 2, "Indestructible, both stand");
        }

        #[test]
        fn no_hero_is_hit_and_a_lone_unit_makes_no_attack() {
            let mut s = wild_board(false, json!([GENN]), json!([]));

            s.play(WILD, json!({}));

            assert!(forced(&s).is_empty());
            for player in [P1, P2] {
                assert_eq!(s.state().players[player].hero.health, HERO_HEALTH);
            }

            let mut s = wild_board(false, json!([VANILLA, VANILLA]), json!([VANILLA, VANILLA]));
            s.play(WILD, json!({}));
            assert!(forced(&s).iter().all(|(_, target)| !target.starts_with("hero")));
            for player in [P1, P2] {
                assert_eq!(s.state().players[player].hero.health, HERO_HEALTH);
            }
        }

        #[test]
        fn r1848004_next_refresh_is_1_more() {
            let mut s = wild_board(false, json!([]), json!([]));

            s.play(WILD, json!({}));

            assert_eq!(refresh_label(&s).as_deref(), Some("Next refresh +1 mana"));
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().active, P1);
            let mana = &s.state().players[P1].mana;
            assert_eq!(mana.current, mana.max + 1);
        }

        #[test]
        fn mana_reads_through_param() {
            let mut s = wild_board(false, json!([]), json!([]));
            assert_eq!(upgrade_number(&mut s, WILD, "mana"), 2);

            s.play(WILD, json!({}));

            assert_eq!(refresh_label(&s).as_deref(), Some("Next refresh +2 mana"));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1857001_2_more() {
            let mut s = wild_board(true, json!([]), json!([]));

            s.play(WILD, json!({}));

            assert_eq!(refresh_label(&s).as_deref(), Some("Next refresh +2 mana"));
        }
    }
}
