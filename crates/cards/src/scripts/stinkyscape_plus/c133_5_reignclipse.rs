//! SPEC §8.12 S+ #133.5 Reignclipse (R77, R101, R102, R386, R1420, R3535000, R3540002, R3565003): (4) Unit, Mindustrial, Token, Automa-ζ, a card of the held
//! Stinkyscape+ set.
//! 19/23  
//! **Tribute 3**  
//! **Immutable**  
//! **Immune to spells**  
//! **Ranged**  
//! **Windfury**  
//! **Pierce**  
//! **Cleave**  
//! **Armor 4**  
//! **Flying**  
//! **Can’t Defend**  
//! **Crater**  
//! **Armor 3**  
//! Damage also **Burns 4**  
//! Dealing damage **Burns 5**  
//! \~\~\~  
//! 38/46  
//! **Tribute 3**  
//! **Immutable**  
//! **Immune to spells**  
//! **Ranged**  
//! **Windfury**  
//! **Pierce**  
//! **Rush**  
//! **Cleave**  
//! **Armor 4**  
//! **Flying**  
//! **Can’t Defend**  
//! **Crater**  
//! **Armor 3**  
//! Damage also **Burns 8**  
//! Dealing damage **Burns 10**
//!
//! The fusion of Reign and Eclipse (S+ #114, #119), with art and a name of its own (R3565003). Tribute 3
//! is the one the two share (the owner, 2026-11-09: not the list's 4), and every keyword is catalog data:
//! Armor 4 and Armor 3 stack to 7 (R77), Crater locks its zone on arrival (R783010). The two Burn lines are
//! one rider (R3540002), the sum of the two, as a Fuse sums `burnsOnDamage` (R102): 4 + 5, Radiant 8 + 10.
//! Its declared `burns` is Eclipse's line, and a tune of it moves the sum (R386).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-133-5";

/// "Tribute 3" (R101; R3565003: the one Tribute the two fused cards share).
const TRIBUTE: i32 = 3;

/// R3565003: the Burn of each of the fused cards' lines, Reign's "Damage also Burns 4 (8)" and Eclipse's
/// "Dealing damage Burns 5 (10)" (S+ #114, #119), which one rider sums as a Fuse sums them (R102).
const REIGN_BURNS: [i32; 2] = [4, 8];
const ECLIPSE_BURNS: [i32; 2] = [5, 10];

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
        base: rides(REIGN_BURNS[0] + ECLIPSE_BURNS[0]),
        radiant: rides(REIGN_BURNS[1] + ECLIPSE_BURNS[1]),
    }
}

// S+ #133.5 Reignclipse — SPEC §8.12 row 133.5.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::play_choices::tribute_cost_of;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    /// The two cards it is the fusion of (S+ #114 and #119).
    const INGREDIENTS: [&str; 2] = ["stinkyscapeplus-114", "stinkyscapeplus-119"];
    const FODDER: &str = "stand-in (0) 1/1";
    const WALL: &str = "stand-in (0) 40/60"; // strikes back for 40: only Ranged saves an attacker
    const ARMORED: &str = "stand-in (0) 40/60 Armor 5";
    const ATTACK: [i32; 2] = [19, 38];
    const HEALTH: [i32; 2] = [23, 46];

    /// The card on p1's field, p1 to move.
    fn on_field(radiant: bool, enemies: Value) -> Scenario {
        crate::weapon_board("s133-5", json!({ "field": [{ "def": ID, "radiant": radiant }] }), json!({ "field": enemies }))
    }

    /// The card in hand with `fodder` units to pay its Tribute.
    fn in_hand(radiant: bool, fodder: usize) -> Scenario {
        crate::weapon_board(
            "s133-5",
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
        let at = usize::from(radiant);
        REIGN_BURNS[at] + ECLIPSE_BURNS[at]
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
    fn r3565003_tribute_3_is_required() {
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
    fn r3565003_armor_4_and_3_stack_to_7() {
        for radiant in [false, true] {
            assert_eq!(on_field(radiant, json!([])).stats(ID).armor, 7, "radiant {radiant}");
        }
    }

    #[test]
    fn r3565003_pierces_armor_and_burns_nine_per_hit() {
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
            if ATTACK[usize::from(radiant)] >= HERO_HEALTH {
                // The hit kills the hero, and a rider follows only a target that survived (R3540002).
                assert!(s.state().result.is_some(), "radiant {radiant}");
                continue;
            }
            s.expect_health(P2, HERO_HEALTH - ATTACK[usize::from(radiant)]);
            assert_eq!(query::hero_burn_of(s.state(), P2), burns(radiant), "radiant {radiant}");
        }
    }

    #[test]
    fn r3565003_the_declared_burns_is_eclipses_line_and_moves_the_sum() {
        for radiant in [false, true] {
            let s = on_field(radiant, json!([]));
            let declared = param_value(s.state(), Some(s.card(ID)), "burns", ParamValueOptions::default());
            assert_eq!(declared, ECLIPSE_BURNS[usize::from(radiant)], "radiant {radiant}");
            let mut s = on_field(radiant, json!([WALL]));
            let moved = crate::upgrade_number(&mut s, ID, "burns");
            assert_eq!(moved, ECLIPSE_BURNS[usize::from(radiant)] + 1, "radiant {radiant}");
            let wall = s.unit_at(P2, 1).id;
            s.attack(ID, wall.as_str());
            assert_eq!(burn_of(&s, wall.as_str()), burns(radiant) + 1, "radiant {radiant}");
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
    fn cleave_burns_both_neighbours() {
        for radiant in [false, true] {
            let mut s = on_field(radiant, json!([WALL, WALL, WALL]));
            let lanes = [1, 2, 3].map(|lane| s.unit_at(P2, lane).id);
            s.attack(ID, lanes[1].as_str());
            for id in &lanes {
                assert_eq!(s.stats(id.as_str()).health, 60 - ATTACK[usize::from(radiant)], "radiant {radiant}");
                assert_eq!(burn_of(&s, id.as_str()), burns(radiant), "radiant {radiant}");
            }
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
                "s133-5",
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
    fn r783010_crater_locks_its_zone() {
        for radiant in [false, true] {
            let mut s = in_hand(radiant, TRIBUTE as usize);
            play(&mut s, TRIBUTE);
            assert!(s.state().players[P1].locks.units[0], "radiant {radiant}");
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
