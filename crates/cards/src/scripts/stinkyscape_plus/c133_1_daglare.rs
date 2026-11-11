//! SPEC §8.12 S+ #133.1 Daglare (R101, R1420, R3535000, R3565003): (0) Unit, Mindustrial, Token, Automa-ζ, a card of the held
//! Stinkyscape+ set.
//! 3/2  
//! **Immutable**  
//! **Immune to spells**  
//! **Ranged**  
//! **Flying**  
//! **Rush**  
//! **Can’t Defend**  
//! \~\~\~  
//! 6/4  
//! **Immutable**  
//! **Immune to spells**  
//! **Rush**  
//! **Ranged**  
//! **Flying**  
//! **Reborn**  
//! **Can’t Defend**
//!
//! The fusion of Dagger and Flare (S+ #110, #115), with art and a name of its own (R3565003): both cards'
//! keywords, at the stats the foundation gives it. Every keyword is catalog data the engine reads, so the
//! script is empty on both faces; the Radiant face adds Reborn and doubles the stats. It is a token: no
//! random pool offers it (§5.1), and Great Paperclip Theory III adds it by name (R3565002).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-133-1";

pub fn script() -> CardScripts {
    let base = Script::default();
    // Keywords are catalog data; the Radiant face only adds Reborn and doubles the stats.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #133.1 Daglare — SPEC §8.12 row 133.1.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    /// The two cards it is the fusion of (S+ #110 and #115).
    const INGREDIENTS: [&str; 2] = ["stinkyscapeplus-110", "stinkyscapeplus-115"];
    const FODDER: &str = "stand-in (0) 1/1";
    const WALL: &str = "stand-in (0) 40/60"; // strikes back for 40: only Ranged saves an attacker
    const ATTACK: [i32; 2] = [3, 6];
    const HEALTH: [i32; 2] = [2, 4];

    /// The card on p1's field, p1 to move.
    fn on_field(radiant: bool, enemies: Value) -> Scenario {
        crate::weapon_board("s133-1", json!({ "field": [{ "def": ID, "radiant": radiant }] }), json!({ "field": enemies }))
    }

    /// The card in hand with `fodder` units to pay its Tribute.
    fn in_hand(radiant: bool, fodder: usize) -> Scenario {
        crate::weapon_board(
            "s133-1",
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
                "s133-1",
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
