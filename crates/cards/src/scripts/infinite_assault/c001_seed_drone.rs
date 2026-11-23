//! SPEC §8.13 IA #1 Seed Drone ((1) Unit, Common, 2/2 → 4/4), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cry: Add a random Common Infinite Assault card to your hand."
//!   Radiant: "Cry: Add a random Rare Infinite Assault card to your hand."
//!
//! TG #20 Recruitment Posters' set-named pool, narrowed by rarity: a pool that names the set reaches it
//! while it is held (R1846001, R4577000), this card and the tokens left out (R387, §5.1). One match-rng pick (R60); a full
//! hand burns it (§2.4).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-001";

/// The set every "Infinite Assault card" names (R4577000).
const SET: &str = "Infinite Assault";

fn seed(rarity: &'static str) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "set": SET, "rarity": rarity },
                "count": 1,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: seed("Common"),
        radiant: seed("Rare"),
    }
}

// IA #1 Seed Drone — SPEC §8.13 row 1.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::catalog::def_of;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "stand-in (0) Spell";

    fn board(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "library": crate::filler(2) },
            "p2": { "hand": [FILLER], "library": crate::filler(2) },
        }))
    }

    /// The cards the Cry added: every hand card once the Drone has left it.
    fn added(s: &Scenario) -> Vec<CardDef> {
        s.hand(P1)
            .iter()
            .map(|card| def_of(Some(s.state()), &card.def_id).clone())
            .collect()
    }

    fn adds_one_of(radiant: bool, rarity: Rarity) {
        for seed in 0..12 {
            let mut s = board(radiant, &format!("seed-drone-{seed}"));
            s.play(ID, json!({}));
            let added = added(&s);
            assert_eq!(added.len(), 1, "seed {seed}");
            for def in nonempty(added, "the added card") {
                assert_eq!(def.set, SetName::InfiniteAssault, "{}", def.id);
                assert_eq!(def.rarity, rarity, "{}", def.id);
                assert!(!def.is_token(), "{}", def.id);
            }
        }
    }

    mod base {
        use super::*;

        #[test]
        fn cry_adds_a_random_common_infinite_assault_card() {
            adds_one_of(false, Rarity::Common);
        }

        #[test]
        fn is_a_2_2() {
            let mut s = board(false, "seed-drone");
            s.play(ID, json!({}));
            s.expect_stats(s.unit_at(P1, 1), json!({ "attack": 2, "health": 2 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn cry_adds_a_random_rare_infinite_assault_card() {
            adds_one_of(true, Rarity::Rare);
        }

        #[test]
        fn is_a_4_4() {
            let mut s = board(true, "seed-drone");
            s.play(ID, json!({}));
            s.expect_stats(s.unit_at(P1, 1), json!({ "attack": 4, "health": 4 }));
        }
    }
}
