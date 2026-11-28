//! SPEC §8.14 FvH #83 Maomao ((1) Unit, Human, Felinor, CN, Rare, 2/1 → 4/2), a card of the held Felinors
//! vs Humans set. Built by S10 (#5260).
//!   Base:    "Damage dealt is Toxic. / Cry: Add a random Potion to your hand."
//!   Radiant: "Damage dealt is Toxic. / Cry: Add a random Radiant Potion to your hand."
//!
//! `damage_is_toxic` (Stinkyscape+ #153 Noxious Rounds's shape, R3540003): each hit it deals becomes that
//! much Toxic on its target after Armor and deals nothing, so it never kills in combat by itself. Cry
//! (R1): `add_random_from_catalog` of one card of the Potion pool (R1179003: the non-token Potions of the
//! sets that ship, so FvH #85 and #86 join only once the set ships, R5235000), Radiant on the Radiant
//! face; a full hand burns it (R4). Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-083";

/// The one Potion its Cry adds.
const POTIONS_ADDED: i32 = 1;

fn maomao(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            damage_is_toxic: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(move |_ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "tags": ["Potion"] },
                "count": POTIONS_ADDED,
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: maomao(false),
        radiant: maomao(true),
    }
}

// FvH #83 Maomao — SPEC §8.14 row 83 (R1, R1179003, R3540003).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::query::toxic_on;
    use jackioh_engine::testkit::*;

    use crate::{hits, weapon_board};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// An enemy that takes the hit and lives, striking nothing back.
    const TARGET: &str = "stand-in (1) 0/30";

    /// The printed attack, base and Radiant.
    const ATTACK: i32 = 2;
    const RADIANT_ATTACK: i32 = 4;

    /// p1 plays Maomao from its hand: the cards its Cry added.
    fn added(radiant: bool) -> Vec<CardInstance> {
        let mut s = weapon_board(
            "fvh083-cry",
            json!({ "hand": [{ "def": ID, "radiant": radiant }], "mana": 10 }),
            json!({}),
        );
        s.play(ID, json!({}));
        s.hand(P1)
    }

    /// R1179003: each card added is a Potion of the pool, Radiant as the face says.
    fn adds_a_potion(radiant: bool) {
        let added = added(radiant);
        assert_eq!(added.len(), 1);
        for card in nonempty(added, "the Potion added") {
            let def = def_of(None, &card.def_id);
            assert!(def.tags.contains(&Tag::Potion), "{} is no Potion", card.def_id);
            assert!(!def.token);
            assert_eq!(card.radiant, radiant);
        }
    }

    /// R3540003: its attack deals nothing and puts its attack's worth of Toxic on the target.
    fn its_damage_is_toxic(radiant: bool, attack: i32) {
        let mut s = weapon_board(
            "fvh083-toxic",
            json!({ "field": [{ "def": ID, "radiant": radiant }] }),
            json!({ "field": [TARGET] }),
        );
        let target = s.unit_at(P2, 1).id;
        s.attack(ID, target.as_str());
        assert!(hits(s.last_events()).is_empty());
        assert_eq!(toxic_on(s.card(target.as_str())), attack);
        s.expect_stats(target.as_str(), json!({ "health": 30 }));
    }

    mod base {
        use super::*;

        #[test]
        fn r1179003_its_cry_adds_a_random_potion_to_your_hand() {
            adds_a_potion(false);
        }

        #[test]
        fn r3540003_the_damage_it_deals_is_toxic() {
            its_damage_is_toxic(false, ATTACK);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1179003_its_cry_adds_a_random_radiant_potion_to_your_hand() {
            adds_a_potion(true);
        }

        #[test]
        fn r3540003_the_damage_it_deals_is_toxic() {
            its_damage_is_toxic(true, RADIANT_ATTACK);
        }
    }
}
