//! SPEC §8.12 S+ #133.2 Macizon (R386, R1420, R3535000, R3540002, R3565003): (1) Unit, Mindustrial, Token, Automa-ζ, a card of the held
//! Stinkyscape+ set.
//! 5/3  
//! **Immutable**  
//! **Immune to spells**  
//! **Ranged**  
//! **Pierce**  
//! **Flying**  
//! **Rush**  
//! **Can’t Defend**  
//! Damage also **Burns 1**  
//! \~\~\~  
//! 10/6  
//! **Immutable**  
//! **Immune to spells**  
//! **Ranged**  
//! **Rush**  
//! **Pierce**  
//! **Flying**  
//! **Reborn**  
//! **Can’t Defend**  
//! Damage also **Burns 2**
//!
//! The fusion of Mace and Horizon (S+ #111, #116), with art and a name of its own (R3565003). Its
//! keywords are catalog data; "Damage also Burns N" is the rider (R3540002), N declared as `burns` per face
//! (R386), as Mace's is.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-133-2";

/// The printed Burns of each face, which the catalog declares as `burns` (R386); the engine reads the flag
/// through `declared_or`, which moves it only by what a tune moved the declared number.
const BASE_BURNS: i32 = 1;
const RADIANT_BURNS: i32 = 2;

/// R3540002: "Damage also Burns N" as the `burnsOnDamage` flag.
fn rides(burns: i32) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            burns_on_damage: Some(burns),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: rides(BASE_BURNS),
        radiant: rides(RADIANT_BURNS),
    }
}

// S+ #133.2 Macizon — SPEC §8.12 row 133.2.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    /// The two cards it is the fusion of (S+ #111 and #116).
    const INGREDIENTS: [&str; 2] = ["stinkyscapeplus-111", "stinkyscapeplus-116"];
    const FODDER: &str = "stand-in (0) 1/1";
    const WALL: &str = "stand-in (0) 40/60"; // strikes back for 40: only Ranged saves an attacker
    const ARMORED: &str = "stand-in (0) 40/60 Armor 5";
    const ATTACK: [i32; 2] = [5, 10];
    const HEALTH: [i32; 2] = [3, 6];

    /// The card on p1's field, p1 to move.
    fn on_field(radiant: bool, enemies: Value) -> Scenario {
        crate::weapon_board("s133-2", json!({ "field": [{ "def": ID, "radiant": radiant }] }), json!({ "field": enemies }))
    }

    /// The card in hand with `fodder` units to pay its Tribute.
    fn in_hand(radiant: bool, fodder: usize) -> Scenario {
        crate::weapon_board(
            "s133-2",
            json!({ "hand": [{ "def": ID, "radiant": radiant }], "field": vec![FODDER; fodder] }),
            json!({ "field": [WALL] }),
        )
    }

    /// Plays it into lane 1, paying the units in lanes 1..=`tribute`.
    fn play(s: &mut Scenario, tribute: i32) {
        let tributes: Vec<String> = (1..=tribute).map(|lane| s.unit_at(P1, lane).id).collect();
        let opts = if tributes.is_empty() { json!({ "zone": 1 }) } else { json!({ "zone": 1, "tributes": tributes }) };
        s.play(ID, opts);
    }

    fn burn_of(s: &Scenario, id: &str) -> i32 {
        query::burn_on(s.card(id))
    }

    fn burns(radiant: bool) -> i32 {
        if radiant { RADIANT_BURNS } else { BASE_BURNS }
    }

    #[test]
    fn ranged_takes_no_strike_back() {
        for radiant in [false, true] {
            let mut s = on_field(radiant, json!([WALL]));
            let wall = s.unit_at(P2, 1).id;
            s.attack(ID, wall.as_str());
            assert_eq!(s.stats(ID).health, HEALTH[usize::from(radiant)], "radiant {radiant}");
            assert_eq!(s.stats(wall.as_str()).health, 60 - ATTACK[usize::from(radiant)], "radiant {radiant}");
        }
    }

    #[test]
    fn r3565003_pierces_armor_and_burns_1_per_hit() {
        for radiant in [false, true] {
            let mut s = on_field(radiant, json!([ARMORED]));
            let wall = s.unit_at(P2, 1).id;
            s.attack(ID, wall.as_str());
            assert_eq!(s.stats(wall.as_str()).health, 60 - ATTACK[usize::from(radiant)], "radiant {radiant}");
            assert_eq!(burn_of(&s, wall.as_str()), burns(radiant), "radiant {radiant}");
        }
    }

    #[test]
    fn r3565003_burns_the_hero_too() {
        for radiant in [false, true] {
            let mut s = on_field(radiant, json!([]));
            s.attack(ID, "hero");
            s.expect_health(P2, HERO_HEALTH - ATTACK[usize::from(radiant)]);
            assert_eq!(query::hero_burn_of(s.state(), P2), burns(radiant), "radiant {radiant}");
        }
    }

    #[test]
    fn declared_burns_match_the_script() {
        for radiant in [false, true] {
            let s = on_field(radiant, json!([]));
            let declared = param_value(s.state(), Some(s.card(ID)), "burns", ParamValueOptions::default());
            assert_eq!(declared, burns(radiant), "radiant {radiant}");
        }
    }

    #[test]
    fn r386_an_upgrade_adds_more_burn() {
        for radiant in [false, true] {
            let mut s = on_field(radiant, json!([WALL]));
            let moved = crate::upgrade_number(&mut s, ID, "burns");
            assert!(moved > burns(radiant), "radiant {radiant}");
            let wall = s.unit_at(P2, 1).id;
            s.attack(ID, wall.as_str());
            assert_eq!(burn_of(&s, wall.as_str()), moved, "radiant {radiant}");
        }
    }

    #[test]
    fn r3565003_carries_both_fused_cards_keywords() {
        crate::register_all();
        let fused = crate::card_def(ID);
        for ingredient in INGREDIENTS {
            let card = crate::card_def(ingredient);
            for (radiant, face, of) in [(false, &fused.base, &card.base), (true, &fused.radiant, &card.radiant)] {
                for keyword in &of.keywords {
                    assert!(face.keywords.contains(keyword), "radiant {radiant}: {ID} lacks {keyword:?} of {ingredient}");
                }
            }
        }
    }

    #[test]
    fn immune_to_spells_no_spell_reaches_it() {
        for radiant in [false, true] {
            let s = crate::weapon_board(
                "s133-2",
                json!({ "field": [{ "def": ID, "radiant": radiant }], "hand": ["stand-in (0) Spell"] }),
                json!({}),
            );
            let card = s.unit_at(P1, 1);
            let spell = s.card("stand-in (0) Spell").clone();
            assert!(immune_to_spells(s.state(), &card), "radiant {radiant}");
            assert!(spell_cannot_reach(s.state(), Some(&spell), &card), "radiant {radiant}");
        }
    }

    #[test]
    fn flying_only_ranged_or_flying_attackers() {
        for radiant in [false, true] {
            let mut s = on_field(radiant, json!(["stand-in (0) 3/9", "stand-in (0) 9/9 Ranged"]));
            s.end_turn();
            let (grounded, ranged) = (s.unit_at(P2, 1).id, s.unit_at(P2, 2).id);
            s.expect_refused(|s| s.attack(grounded.as_str(), ID));
            s.attack(ranged.as_str(), ID);
            let left = s.unit(P1, 1).map_or(0, |unit| s.stats(&unit).health);
            assert!(left < HEALTH[usize::from(radiant)], "radiant {radiant}: the Ranged attacker reached it");
        }
    }

    #[test]
    fn cant_defend() {
        for radiant in [false, true] {
            let mut s = on_field(radiant, json!([WALL]));
            s.expect_refused(|s| s.switch_position(ID));
        }
    }

    #[test]
    fn rush_hits_a_unit_not_the_hero_on_both_faces() {
        for radiant in [false, true] {
            let mut s = in_hand(radiant, 0);
            play(&mut s, 0);
            s.expect_refused(|s| s.attack(ID, "hero"));
            let wall = s.unit_at(P2, 1).id;
            s.attack(ID, wall.as_str());
            assert_eq!(s.stats(wall.as_str()).health, 60 - ATTACK[usize::from(radiant)], "radiant {radiant}");
            assert_eq!(s.stats(ID).health, HEALTH[usize::from(radiant)], "radiant {radiant}: Ranged takes no strike back");
        }
    }
}
