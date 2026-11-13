//! SPEC §8.12 S+ #64 Surge Wall, a card of the Stinkyscape+ set (R1420, R3535000). Built by S08 (#3555).
//! 0/6 embiggen 0/24  
//! **Can’t attack**  
//! **Can’t defend**  
//! **End of turn** deal 1 embiggen 2 damage to all enemies  
//! **Embiggen: GIGA**  
//! \~\~\~  
//! 0/12 embiggen 0/48  
//! **Can’t attack**  
//! **Can’t defend**  
//! **End of turn** deal 2 embiggen 4 damage to all enemies  
//! **Pierce, Armor 2**  
//! **Embiggen: GIGA**
//!
//! The Walls' shared embiggened text (R3555001) plus an end-of-turn hit on every enemy (R62): `damage`
//! unembiggened, `embiggenedDamage` embiggened, both declared. Pierce on the Radiant face is catalog
//! data, read off the source as any hit is (R346).

use jackioh_engine::effects::damage_all;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-064";

fn surge_wall() -> Script {
    Script {
        end_of_turn: Some(hook(|ctx| {
            let key = if ctx.live_self().and_then(|me| me.embiggened) == Some(true) {
                "embiggenedDamage"
            } else {
                "damage"
            };
            vec![damage_all(json_as(json!({
                "side": "enemy",
                "amount": param(&*ctx, key),
                "heroes": true,
            })))]
        })),
        ..super::c065_thorium_wall::embiggened_wall()
    }
}

pub fn script() -> CardScripts {
    // Pierce and Armor 2 on the Radiant face are catalog data.
    CardScripts {
        base: surge_wall(),
        radiant: surge_wall(),
    }
}

// S+ #64 Surge Wall — SPEC §8.12 row S+ 64 (R62, R346, R3555001).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::weapon_board;

    const P2: PlayerId = PlayerId::P2;

    const WALL: &str = "stand-in (1) 1/30";
    const ARMORED: &str = "stand-in (0) 1/9 Armor 5";

    /// The wall standing for p1, enemies standing for p2.
    fn standing(radiant: bool, seed: &str, enemies: Value) -> Scenario {
        weapon_board(
            seed,
            json!({ "field": [{ "def": ID, "radiant": radiant }] }),
            json!({ "field": enemies }),
        )
    }

    /// The wall in p1's hand with mana to play it either way.
    fn holding(radiant: bool, seed: &str) -> Scenario {
        weapon_board(
            seed,
            json!({ "hand": [{ "def": ID, "radiant": radiant }], "mana": 10 }),
            json!({}),
        )
    }

    mod base {
        use super::*;

        #[test]
        fn r62_end_of_turn_deals_1_to_every_enemy_and_the_hero() {
            let mut s = standing(false, "s64-base", json!([WALL, WALL]));
            s.end_turn();
            assert_eq!(s.stats(&s.unit_at(P2, 1).id).health, 30 - 1);
            assert_eq!(s.stats(&s.unit_at(P2, 2).id).health, 30 - 1);
            s.expect_health(P2, 30 - 1);
        }

        #[test]
        fn r3555001_embiggened_it_deals_2() {
            let mut s = holding(false, "s64-embiggened");
            s.play(ID, json!({ "zone": 1, "embiggen": true }));
            assert_eq!(s.card(ID).giga, Some(true));
            s.expect_stats(ID, json!({ "attack": 0, "maxHealth": 24 }));
            s.end_turn();
            s.expect_health(P2, 30 - 2);
        }

        #[test]
        fn cant_attack() {
            let mut s = standing(false, "s64-cant", json!([WALL]));
            let wall = s.unit_at(P2, 1).id;
            s.expect_refused(|s| s.attack(ID, wall.as_str()));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r62_end_of_turn_deals_2_to_every_enemy_and_the_hero() {
            let mut s = standing(true, "s64-radiant", json!([WALL, WALL]));
            s.end_turn();
            assert_eq!(s.stats(&s.unit_at(P2, 1).id).health, 30 - 2);
            assert_eq!(s.stats(&s.unit_at(P2, 2).id).health, 30 - 2);
            s.expect_health(P2, 30 - 2);
        }

        #[test]
        fn r3555001_embiggened_it_deals_4() {
            let mut s = holding(true, "s64-radiant-embiggened");
            s.play(ID, json!({ "zone": 1, "embiggen": true }));
            assert_eq!(s.card(ID).giga, Some(true));
            s.expect_stats(ID, json!({ "attack": 0, "maxHealth": 48 }));
            s.end_turn();
            s.expect_health(P2, 30 - 4);
        }

        #[test]
        fn r346_its_hits_pierce_armor() {
            let mut s = standing(true, "s64-pierce", json!([ARMORED]));
            s.end_turn();
            // Pierce reads off the source (R346): the full 2 past Armor 5.
            assert_eq!(s.stats(&s.unit_at(P2, 1).id).health, 9 - 2);
        }
    }
}
