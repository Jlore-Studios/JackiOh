//! SPEC §8.13 IA #61 Meltdown Charge ((3) Spell, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Deal {damage} damage to every Unit and both heroes. Repeat once for each mana you have left." (damage 2)
//!   Radiant: "Deal {damage} damage to every enemy. Repeat once for each mana you have left." (damage 2)
//!
//! Rounds are 1 plus the mana its controller holds as it resolves, after its price: the count is
//! read once and the mana is not spent (R4587001). The rounds are one list, with no state check
//! between them (R59). The base face sweeps every Unit and both heroes (`damageAll` over any side
//! with `heroes`); the Radiant face sweeps the enemy side alone.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-061";

fn meltdown(enemy_only: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            // R4587001: 1 plus the mana held as it resolves, read once, never spent.
            let rounds = 1 + unspent_mana_of(ctx.state, ctx.controller);
            (0..rounds)
                .map(|_| {
                    let mut sweep = json!({ "amount": param(&*ctx, "damage"), "heroes": true });
                    if enemy_only {
                        sweep["side"] = json!("enemy");
                    }
                    damage_all(json_as(sweep))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: meltdown(false),
        radiant: meltdown(true),
    }
}

// IA #61 Meltdown Charge — SPEC §8.13 row 61 (R4587001).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const WALL: &str = "stand-in (1) 5/5";

    fn board(radiant: bool, mana: i32) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": [WALL],
                "mana": mana,
            },
            "p2": { "field": [WALL] },
        }))
    }

    fn unit_health(s: &Scenario, player: PlayerId) -> i32 {
        s.stats(&s.unit_at(player, 1)).health
    }

    mod base {
        use super::*;

        #[test]
        fn r4587001_one_round_and_one_more_for_each_mana_left() {
            // 4 mana, 3 paid: 1 left, so 2 rounds of 2.
            let mut s = board(false, 4);
            s.play(ID, json!({}));
            s.expect_health(P1, HERO_HEALTH - 4);
            s.expect_health(P2, HERO_HEALTH - 4);
            assert_eq!(unit_health(&s, P1), 1);
            assert_eq!(unit_health(&s, P2), 1);
        }

        #[test]
        fn with_no_mana_left_it_hits_once() {
            let mut s = board(false, 3);
            s.play(ID, json!({}));
            s.expect_health(P1, HERO_HEALTH - 2);
            s.expect_health(P2, HERO_HEALTH - 2);
            assert_eq!(unit_health(&s, P1), 3);
            assert_eq!(unit_health(&s, P2), 3);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4587001_your_side_is_spared() {
            let mut s = board(true, 4);
            s.play(ID, json!({}));
            s.expect_health(P1, HERO_HEALTH);
            s.expect_health(P2, HERO_HEALTH - 4);
            assert_eq!(unit_health(&s, P1), 5);
            assert_eq!(unit_health(&s, P2), 1);
        }
    }
}
