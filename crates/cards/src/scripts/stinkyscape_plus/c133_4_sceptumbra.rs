//! SPEC §8.12 S+ #133.4 Sceptumbra (R101, R386, R1420, R3535000, R3540002, R3565003): (3) Unit, Mindustrial, Token, Automa-ζ, a card of the held
//! Stinkyscape+ set.
//! 14/18  
//! **Tribute 2**  
//! **Immutable**  
//! **Immune to spells**  
//! **Ranged**  
//! **Windfury**  
//! **Pierce**  
//! **Armor 3**  
//! **Flying**  
//! **Can’t Defend**  
//! Damage also **Burns 4**  
//! \~\~\~  
//! 28/36  
//! **Tribute 2**  
//! **Immutable**  
//! **Immune to spells**  
//! **Ranged**  
//! **Windfury**  
//! **Pierce**  
//! **Rush**  
//! **Armor 3**  
//! **Flying**  
//! **Can’t Defend**  
//! Damage also **Burns 8**
//!
//! The fusion of Scepter and Antumbra (S+ #113, #118), with art and a name of its own (R3565003). Tribute 2
//! is the one the two share (R101), and the keywords are catalog data; "Damage also Burns N" is the rider
//! (R3540002), N declared as `burns` per face (R386).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-133-4";

/// "Tribute 2" (R101; R3565003: the one Tribute the two fused cards share).
const TRIBUTE: i32 = 2;

/// The printed Burns of each face, which the catalog declares as `burns` (R386); the engine reads the flag
/// through `declared_or`, which moves it only by what a tune moved the declared number.
const BASE_BURNS: i32 = 4;
const RADIANT_BURNS: i32 = 8;

/// R3540002: "Damage also Burns N" as the `burnsOnDamage` flag, with its Tribute (R101).
fn rides(burns: i32) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE),
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

// S+ #133.4 Sceptumbra — SPEC §8.12 row 133.4.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::play_choices::tribute_cost_of;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    /// The two cards it is the fusion of (S+ #113 and #118).
    const INGREDIENTS: [&str; 2] = ["stinkyscapeplus-113", "stinkyscapeplus-118"];
    const FODDER: &str = "stand-in (0) 1/1";
    const WALL: &str = "stand-in (0) 40/60"; // strikes back for 40: only Ranged saves an attacker
    const ARMORED: &str = "stand-in (0) 40/60 Armor 5";
    const ATTACK: [i32; 2] = [14, 28];
    const HEALTH: [i32; 2] = [18, 36];

    /// The card on p1's field, p1 to move.
    fn on_field(radiant: bool, enemies: Value) -> Scenario {
        crate::weapon_board("s133-4", json!({ "field": [{ "def": ID, "radiant": radiant }] }), json!({ "field": enemies }))
    }

    /// The card in hand with `fodder` units to pay its Tribute.
    fn in_hand(radiant: bool, fodder: usize) -> Scenario {
        crate::weapon_board(
            "s133-4",
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
    fn r3565003_tribute_2_is_required() {
        for radiant in [false, true] {
            let mut s = in_hand(radiant, TRIBUTE as usize);
            assert_eq!(tribute_cost_of(s.state(), s.card(ID)), TRIBUTE, "radiant {radiant}");
            s.expect_refused(|s| s.play(ID, json!({ "zone": 5 })));
            if TRIBUTE > 1 {
                let one_short: Vec<String> = (1..TRIBUTE).map(|lane| s.unit_at(P1, lane).id).collect();
                s.expect_refused(|s| s.play(ID, json!({ "zone": 5, "tributes": one_short })));
            }
            let fodder: Vec<String> = (1..=TRIBUTE).map(|lane| s.unit_at(P1, lane).id).collect();
            play(&mut s, TRIBUTE);
            for id in &fodder {
                s.expect_in_zone(id.as_str(), "graveyard");
            }
            assert_eq!(s.unit_at(P1, 1).def_id, ID, "radiant {radiant}");
        }
    }

    #[test]
    fn armor_is_printed() {
        assert_eq!(on_field(false, json!([])).stats(ID).armor, 3);
        assert_eq!(on_field(true, json!([])).stats(ID).armor, 3);
    }

    #[test]
    fn r3565003_pierces_armor_and_burns_4_per_hit() {
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
    fn windfury_attacks_twice_and_no_third() {
        let mut s = on_field(false, json!([WALL]));
        let wall = s.unit_at(P2, 1).id;
        s.attack(ID, wall.as_str());
        s.attack(ID, wall.as_str());
        assert_eq!(s.stats(wall.as_str()).health, 60 - 2 * ATTACK[0]);
        s.expect_refused(|s| s.attack(ID, wall.as_str()));
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
                "s133-4",
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

    mod base {
        use super::*;

        #[test]
        fn cannot_attack_on_its_summon_turn() {
            let mut s = in_hand(false, TRIBUTE as usize);
            play(&mut s, TRIBUTE);
            let wall = s.unit_at(P2, 1).id;
            s.expect_refused(|s| s.attack(ID, wall.as_str()));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn rush_hits_a_unit_not_the_hero() {
            let mut s = in_hand(true, TRIBUTE as usize);
            play(&mut s, TRIBUTE);
            s.expect_refused(|s| s.attack(ID, "hero"));
            let wall = s.unit_at(P2, 1).id;
            s.attack(ID, wall.as_str());
            assert_eq!(s.stats(wall.as_str()).health, 60 - ATTACK[1]);
        }
    }
}
