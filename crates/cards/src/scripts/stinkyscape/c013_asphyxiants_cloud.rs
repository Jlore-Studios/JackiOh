//! S #13 Asphyxiants Cloud (SPEC §8.10 row S 13; §4.4, §6.2, §10.5, R59, R346, R1848002): (2) Spell, Rare.
//!   Base:    "Poisonous / Your next Spell this turn has Poisonous. Deal {damage} damage to all
//!            enemies. Draw {draw|card|cards}." — 1, 1.
//!   Radiant: "Echo 1, Poisonous" and the same text: it resolves a second time.
//!
//! Engine: its own Poisonous is the keyword on its face, read off it while it resolves (R346,
//! R1848002), so each of its hits destroys a Unit it deals 1 or more to. `give_next_spell_keyword`
//! puts a `keywordNextSpell` modifier on its caster for this turn: the next Spell they play face-up
//! takes Poisonous at §10.5 step 4 for that resolution alone, a Unit, Field Spell or Trap played
//! meanwhile leaving it waiting, and the turn's cleanup ending it (R1848002).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-013";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![
                give_next_spell_keyword(json_as(json!({ "keyword": { "kind": "Poisonous" } }))),
                damage_all(json_as(json!({
                    "amount": param(&*ctx, "damage"),
                    "side": "enemy",
                    "heroes": true,
                }))),
                draw(json_as(json!({ "count": param(&*ctx, "draw") }))),
            ]
        })),
        ..Script::default()
    };

    let radiant = Script {
        static_flags: Some(StaticFlags {
            echo: Some(1),
            ..StaticFlags::default()
        }),
        ..base.clone()
    };

    CardScripts { base, radiant }
}

// S #13 Asphyxiants Cloud — SPEC §8.10 row S 13, BUILD M12 row S 13 (R1848002).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario, upgrade_number};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const CLOUD: &str = "stinkyscape-013";
    const GREASER: &str = "meditative-100"; // (2) Unit 7/7.
    const JILLIAX: &str = "core-056"; // (2) Unit 3/2: Rush, Taunt, Lifesteal, Divine Shield.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const WHIRLWIND: &str = "classicplus-021"; // (0) Spell: 1 damage to all Units.
    const FILLER: &str = "core-005";

    fn hero_health(s: &Scenario, player: PlayerId) -> i32 {
        s.state().players[player].hero.health
    }

    fn cloud_board(radiant: bool, more: Value, mine: Value, theirs: Value) -> Scenario {
        crate::register_all();
        let mut hand = vec![json!({ "def": CLOUD, "radiant": radiant })];
        hand.extend(more.as_array().cloned().unwrap_or_default());
        scenario(json!({
            "p1": { "hand": hand, "field": mine, "mana": 10, "library": filler(6) },
            "p2": { "hand": [FILLER], "field": theirs, "library": filler(6) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r1848002_one_poisonous_hit_on_each_enemy_and_a_draw() {
            let mut s = cloud_board(
                false,
                json!([FILLER]),
                json!([GREASER]),
                json!([GREASER, GREASER, JILLIAX]),
            );
            let (first, second, shielded) = (s.unit_at(P2, 1), s.unit_at(P2, 2), s.unit_at(P2, 3));
            let mine = s.unit_at(P1, 1);
            let held = s.hand(P1).len();

            s.play(CLOUD, json!({}));

            s.expect_in_zone(&first, "graveyard");
            s.expect_in_zone(&second, "graveyard");
            s.expect_in_zone(&shielded, "field");
            assert_eq!(s.card(&shielded).damage, 0);
            assert_eq!(hero_health(&s, P2), HERO_HEALTH - 1);
            assert_eq!(s.card(&mine).damage, 0);
            assert_eq!(hero_health(&s, P1), HERO_HEALTH);
            assert_eq!(s.hand(P1).len(), held);
        }

        #[test]
        fn r1848002_the_next_spell_is_poisonous_and_the_one_after_is_not() {
            let mut s = cloud_board(
                false,
                json!([WHIRLWIND, WHIRLWIND, VANILLA, FILLER]),
                json!([{ "def": GREASER, "lane": 1 }]),
                json!([]),
            );
            let greaser = s.unit_at(P1, 1);
            s.play(CLOUD, json!({}));

            let first = s.card(WHIRLWIND).id.clone();
            s.play(&first, json!({}));
            s.expect_in_zone(&greaser, "graveyard");

            s.play(VANILLA, json!({ "zone": 2 }));
            let vanilla = s.unit_at(P1, 2);
            s.play(WHIRLWIND, json!({}));
            s.expect_in_zone(&vanilla, "field");
            s.expect_stats(&vanilla, json!({ "attack": 4, "health": 3 }));
        }

        #[test]
        fn r1848002_a_unit_played_between_leaves_it_waiting() {
            let mut s = cloud_board(
                false,
                json!([VANILLA, WHIRLWIND, FILLER]),
                json!([{ "def": GREASER, "lane": 1 }]),
                json!([]),
            );
            let greaser = s.unit_at(P1, 1);
            s.play(CLOUD, json!({}));

            s.play(VANILLA, json!({ "zone": 2 }));
            s.play(WHIRLWIND, json!({}));

            s.expect_in_zone(&greaser, "graveyard");
        }

        #[test]
        fn r1848002_it_ends_with_the_turn() {
            let mut s = cloud_board(
                false,
                json!([WHIRLWIND, FILLER]),
                json!([{ "def": GREASER, "lane": 1 }]),
                json!([]),
            );
            let greaser = s.unit_at(P1, 1);
            s.play(CLOUD, json!({}));
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().active, P1);

            s.play(WHIRLWIND, json!({}));

            s.expect_in_zone(&greaser, "field");
            s.expect_stats(&greaser, json!({ "attack": 7, "health": 6 }));
        }

        #[test]
        fn damage_and_draw_read_through_param() {
            let mut s = cloud_board(false, json!([FILLER]), json!([]), json!([]));
            assert_eq!(upgrade_number(&mut s, CLOUD, "damage"), 2);
            assert_eq!(upgrade_number(&mut s, CLOUD, "draw"), 2);
            let held = s.hand(P1).len();

            s.play(CLOUD, json!({}));

            assert_eq!(hero_health(&s, P2), HERO_HEALTH - 2);
            assert_eq!(s.hand(P1).len(), held + 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn it_all_happens_twice() {
            let mut s = cloud_board(true, json!([FILLER]), json!([]), json!([{ "def": JILLIAX, "lane": 1 }]));
            let shielded = s.unit_at(P2, 1);
            let held = s.hand(P1).len();

            s.play(CLOUD, json!({}));

            assert_eq!(hero_health(&s, P2), HERO_HEALTH - 2);
            assert_eq!(s.hand(P1).len(), held + 1, "two drawn for the Cloud played");
            s.expect_in_zone(&shielded, "graveyard");
        }
    }
}
