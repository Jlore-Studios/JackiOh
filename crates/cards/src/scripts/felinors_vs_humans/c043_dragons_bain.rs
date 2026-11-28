//! SPEC §8.14 FvH #43 Dragons Bain ((2) Unit, Epic, 3/5 → 6/10), a card of the held Felinors vs Humans set
//! (R5235000).
//!   Base:    "End of turn: Deal {damage} damage to a random enemy." (damage 5)
//!   Radiant: "Pierce\nEnd of turn: Deal {damage} damage to a random enemy, twice." (damage 5)
//!
//! Engine: at the end of its controller's turn (R62), `damage_random_enemies` of count 1 and the declared
//! `damage`: the enemy hero or a standing enemy Unit (R60, R1860004). The Radiant face runs it twice, each
//! hit its own pick, so one enemy may take both and a destroyed one is not picked again. The Radiant face's
//! Pierce is catalog data, read off the source for every hit it deals (R346). Built by S01 (#5251).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-043";

/// "Deal {damage} damage to a random enemy", `hits` times, each its own pick.
fn dragons_bain(hits: usize) -> Script {
    Script {
        end_of_turn: Some(hook(move |ctx| {
            let amount = param(&*ctx, "damage");
            (0..hits)
                .map(|_| damage_random_enemies(json_as(json!({ "count": 1, "amount": amount }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: dragons_bain(1),
        radiant: dragons_bain(2),
    }
}

// FvH #43 Dragons Bain — SPEC §8.14 row 43 (R62, R60, R346).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P2;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    /// Dragons Bain (`radiant` its face) stands on p1's side, the enemy side empty, the enemy hero with
    /// `armor`.
    fn standing(radiant: bool, armor: i32) -> Scenario {
        scenario(json!({
            "p1": { "hand": [FILLER], "field": [{ "def": ID, "radiant": radiant }], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4), "armor": armor },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r62_at_its_controllers_end_of_turn_it_hits_the_only_enemy_for_5() {
            let mut s = standing(false, 0);
            s.end_turn();
            s.expect_health(P2, HERO_HEALTH - 5);
            assert_eq!(crate::hits(s.events()), vec![("hero-p2".to_string(), 5)]);
            // The opponent's end of turn is not its controller's.
            s.end_turn();
            s.expect_health(P2, HERO_HEALTH - 5);
        }

        #[test]
        fn without_pierce_the_enemy_heros_armor_takes_the_hit() {
            let mut s = standing(false, 5);
            s.end_turn();
            s.expect_health(P2, HERO_HEALTH);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r62_at_its_controllers_end_of_turn_it_hits_a_random_enemy_twice() {
            let mut s = standing(true, 0);
            s.end_turn();
            assert_eq!(
                crate::hits(s.events()),
                vec![("hero-p2".to_string(), 5), ("hero-p2".to_string(), 5)],
                "two picks, both the only enemy"
            );
            s.expect_health(P2, HERO_HEALTH - 10);
        }

        #[test]
        fn r346_its_pierce_takes_both_hits_past_the_enemy_heros_armor() {
            let mut s = standing(true, 5);
            s.end_turn();
            s.expect_health(P2, HERO_HEALTH - 10);
        }
    }
}
