//! C+ #38 Solarius (SPEC §8.7 row 38): (2) Unit, Epic, 3/2 → 6/4.
//!   Base:    "Spell Damage +2. Death: Shuffle a Solarius Prime into your deck." (no Cry, balance patch 1)
//!   Radiant: "Spell Damage +5. Death: Shuffle a Radiant Solarius Prime into your deck."
//! Spell Damage is the catalog's numbered keyword, which §4.4 step 0 (`damage.ts`) reads off the field;
//! the Death shuffles a fresh C+ #38.1 in at a random position, R80's cap turning it away.

use jackioh_engine::effects::shuffle_into;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-038";

const SOLARIUS_PRIME: &str = "classicplus-038-1";

fn solarius(radiant: bool) -> Script {
    Script {
        death: Some(hook(move |_ctx| {
            vec![shuffle_into(json_as(json!({ "defId": SOLARIUS_PRIME, "count": 1, "radiant": radiant })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: solarius(false),
        radiant: solarius(true),
    }
}

// C+ #38 Solarius — SPEC §8.7 row 38, BUILD M9 Classic+ row C+ 38: "Spell Damage +2 while on the
// field: each hit of a Spell you play or cast gains 2 (§4.4 step 0), every hit of a multi-hit Spell,
// never a Field Spell's, a Trap's, a Unit's or an activation's hit, never the opponent's Spells; two
// sources add; no Cry on either face (balance patch 1); Death shuffles a Solarius Prime (C+ #38.1)
// into your deck at a random position (R80's cap), shown in your library list; radiant Spell Damage +5,
// the Solarius Prime Radiant".
//
// Spell Damage is a numbered keyword (§6.1), so B3.4's X change tunes it rather than a param (R482).
// A Trap's hit is proved in C+ #22 Blood Moon's test, the one Trap of these sets whose text deals
// damage from itself.
#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SOLARIUS: &str = "classicplus-038";
    const PRIME: &str = "classicplus-038-1";
    const BONE_STORM: &str = "classicplus-036-1";
    const LUNAR_ECLIPSE: &str = "core-035"; // (1) Spell: deal 3 damage to a target.
    const ECHOES: &str = "core-040"; // Field Spell: start of turn, damage to the enemy hero = your exile count.
    const SORCERER: &str = "core-068"; // Unit, Cry: deal 4 damage to a target.
    const HEROIC: &str = "core-098"; // Field Spell, its "burn" power: 2 damage to each opposing hero.
    const MENACE: &str = "core-019"; // 9/9 Taunt.
    const FILLER: &str = "core-005";

    fn at_hero() -> Value {
        json!({ "targets": [{ "pick": "hero", "player": "p2" }] })
    }

    use crate::scenario;

    fn events_json(s: &Scenario) -> Vec<Value> {
        s.events().iter().map(|event| serde_json::to_value(event).expect("an event is JSON")).collect()
    }

    fn own_library_cards(s: &Scenario) -> Vec<Value> {
        let view = serde_json::to_value(s.view(P1)).expect("a view is JSON");
        view["you"]["ownLibrary"]["cards"].as_array().cloned().unwrap_or_default()
    }

    fn def_ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.def_id.clone()).collect()
    }

    /// TS `power.memory[subsystems.POWER_KEY] = power` on the live Heroic Power card.
    fn set_power(s: &mut Scenario, id: &str, power: &str) {
        let live = find_instance_mut(s.state_mut(), id).expect("the Heroic Power card");
        live.memory.insert(subsystems::POWER_KEY.to_string(), json!(power));
    }

    #[test]
    fn is_tagged_catalyst_and_both_faces_name_its_prime_spaced_solarius_prime_patch_v0_2_y() {
        let def = crate::card_def(SOLARIUS);
        assert_eq!(def.tags, vec![Tag::Catalyst]);
        assert!(def.base.text.contains("Death: Shuffle a Solarius Prime into your deck."));
        assert!(def.radiant.text.contains("Death: Shuffle a Radiant Solarius Prime into your deck."));
        assert!(def.refs.unwrap_or_default().iter().any(|id| id == PRIME));
    }

    mod base {
        use super::*;

        #[test]
        fn prints_spell_damage_2_a_keyword_not_a_param_and_declares_nothing() {
            let s = scenario(json!({ "p1": { "field": [SOLARIUS] } }));
            assert!(s.stats(SOLARIUS).keywords.contains(&Keyword::SpellDamage { n: 2 }));
        }

        #[test]
        fn s4_4_step_0_a_spell_you_play_deals_2_more_per_hit() {
            let mut s = scenario(json!({ "p1": { "hand": [LUNAR_ECLIPSE, FILLER], "field": [SOLARIUS] }, "p2": { "hand": [FILLER] } }));
            s.play(LUNAR_ECLIPSE, at_hero());
            s.expect_health(P2, HERO_HEALTH - 5);
        }

        #[test]
        fn every_hit_of_a_multi_hit_spell_gains_2() {
            let mut s = scenario(json!({
                "p1": { "hand": [BONE_STORM, FILLER], "field": [SOLARIUS] },
                "p2": { "hand": [FILLER], "field": [MENACE, MENACE] },
            }));
            s.play(BONE_STORM, json!({}));
            s.expect_health(P2, HERO_HEALTH - 3);
            for lane in [1, 2] {
                let unit = s.unit(P2, lane).expect("a Menace");
                assert_eq!(s.stats(&unit).health, 6);
            }
        }

        #[test]
        fn two_sources_add_two_solarius_make_4() {
            let mut s = scenario(json!({ "p1": { "hand": [LUNAR_ECLIPSE, FILLER], "field": [SOLARIUS, SOLARIUS] }, "p2": { "hand": [FILLER] } }));
            s.play(LUNAR_ECLIPSE, at_hero());
            s.expect_health(P2, HERO_HEALTH - 7);
        }

        #[test]
        fn never_the_opponents_spells() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [FILLER], "field": [SOLARIUS] },
                "p2": { "hand": [LUNAR_ECLIPSE, FILLER] },
            }));
            s.play(LUNAR_ECLIPSE, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
            s.expect_health(P1, HERO_HEALTH - 3);
        }

        #[test]
        fn never_a_units_hit_a_crys_damage_is_not_raised() {
            let mut s = scenario(json!({ "p1": { "hand": [SORCERER, FILLER], "field": [SOLARIUS] }, "p2": { "hand": [FILLER] } }));
            s.play(SORCERER, at_hero());
            s.expect_health(P2, HERO_HEALTH - 4);
        }

        #[test]
        fn never_a_units_combat_hit() {
            let mut s = scenario(json!({ "p1": { "hand": [FILLER], "field": [SOLARIUS] }, "p2": { "hand": [FILLER] } }));
            s.attack(SOLARIUS, "hero");
            s.expect_health(P2, HERO_HEALTH - 3);
        }

        #[test]
        fn never_a_field_spells_hit_echoes_of_the_forgotten_deals_its_exile_count_unraised() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [SOLARIUS], "backrow": [ECHOES], "exile": [FILLER, FILLER], "library": [FILLER, FILLER] },
                "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
            }));
            s.end_turn();
            s.end_turn(); // p1's start of turn: Echoes deals 2.
            s.expect_health(P2, HERO_HEALTH - 2);
        }

        #[test]
        fn never_an_activations_hit_heroic_powers_burn_deals_2() {
            let mut s = scenario(json!({ "p1": { "hand": [FILLER], "field": [SOLARIUS], "backrow": [HEROIC], "mana": 4 }, "p2": { "hand": [FILLER] } }));
            let power = s.backrow(P1, 1).expect("Heroic Power in the backrow").id;
            set_power(&mut s, &power, "burn");
            s.activate(&power, json!({}));
            s.expect_health(P2, HERO_HEALTH - 2);
        }

        #[test]
        fn stops_raising_once_it_has_left_the_field() {
            let mut s = scenario(json!({
                "p1": { "hand": [LUNAR_ECLIPSE, LUNAR_ECLIPSE, FILLER], "field": [SOLARIUS] },
                "p2": { "hand": [FILLER] },
            }));
            let hand = s.hand(P1);
            let (first, second) = (hand[0].clone(), hand[1].clone());
            let solarius = s.card(SOLARIUS).id.clone();
            s.play(&first, json!({ "targets": [{ "pick": "instance", "instanceId": solarius }] }));
            s.expect_in_zone(SOLARIUS, "graveyard");
            s.play(&second, at_hero());
            s.expect_health(P2, HERO_HEALTH - 3);
        }

        #[test]
        fn no_cry_on_either_face_playing_draws_nothing() {
            let mut s = scenario(json!({ "p1": { "hand": [SOLARIUS, FILLER], "library": [LUNAR_ECLIPSE, FILLER] }, "p2": { "hand": [FILLER] } }));
            s.play(SOLARIUS, json!({}));
            assert_eq!(def_ids(&s.hand(P1)), vec![FILLER]);
        }

        #[test]
        fn r1_summoned_not_played_it_draws_nothing_a_recruit() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "backrow": [HEROIC], "library": [SOLARIUS, FILLER], "mana": 4 },
                "p2": { "hand": [FILLER] },
            }));
            let power = s.backrow(P1, 1).expect("Heroic Power in the backrow").id;
            set_power(&mut s, &power, "recruit");
            s.activate(&power, json!({}));
            assert_eq!(s.unit(P1, 1).map(|card| card.def_id), Some(SOLARIUS.to_string()));
            assert_eq!(def_ids(&s.hand(P1)), vec![FILLER]);
        }

        #[test]
        fn death_shuffles_a_base_solarius_prime_into_your_deck_at_a_random_position_shown_in_your_list() {
            let library: Vec<&str> = (0..12).map(|_| FILLER).collect();
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [SOLARIUS], "library": library },
                "p2": { "hand": [FILLER], "field": [MENACE] },
            }));
            s.attack(SOLARIUS, MENACE);
            s.expect_in_zone(SOLARIUS, "graveyard");
            let library = s.pile(P1, "library");
            let primes: Vec<&CardInstance> = library.iter().filter(|card| card.def_id == PRIME).collect();
            assert_eq!(primes.len(), 1);
            assert!(!primes[0].radiant);
            assert_eq!(library.len(), 13);
            assert!(own_library_cards(&s).contains(&json!({ "defId": PRIME, "radiant": false, "count": 1 })));
            assert!(events_json(&s).iter().any(|event| event["type"] == "shuffledIn" && event["defId"] == PRIME));
        }

        #[test]
        fn the_solarius_prime_goes_in_at_a_position_the_rng_picks_not_always_the_top() {
            let mut positions: BTreeSet<i64> = BTreeSet::new();
            for seed in ["sol-a", "sol-b", "sol-c", "sol-d", "sol-e", "sol-f"] {
                let library: Vec<&str> = (0..12).map(|_| FILLER).collect();
                let mut s = scenario(json!({
                    "seed": seed,
                    "p1": { "hand": [FILLER], "field": [SOLARIUS], "library": library },
                    "p2": { "hand": [FILLER], "field": [MENACE] },
                }));
                s.attack(SOLARIUS, MENACE);
                let at = s.pile(P1, "library").iter().position(|card| card.def_id == PRIME).map_or(-1, |at| at as i64);
                positions.insert(at);
            }
            assert!(positions.len() > 1);
        }

        #[test]
        fn r80_a_full_deck_turns_the_solarius_prime_away() {
            let library: Vec<&str> = (0..LIBRARY_CAP).map(|_| FILLER).collect();
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [SOLARIUS], "library": library },
                "p2": { "hand": [FILLER], "field": [MENACE] },
            }));
            s.attack(SOLARIUS, MENACE);
            assert!(!s.pile(P1, "library").iter().any(|card| card.def_id == PRIME));
            assert!(events_json(&s).iter().any(|event| event["type"] == "libraryOverflow" && event["defId"] == PRIME));
        }

        #[test]
        fn r386_its_spell_damage_is_a_numbered_keyword_b3_4s_x_change_tunes_one_upgrade_step_makes_it_3() {
            let mut s = scenario(json!({ "p1": { "hand": [LUNAR_ECLIPSE, FILLER], "field": [SOLARIUS] }, "p2": { "hand": [FILLER] } }));
            let id = s.card(SOLARIUS).id.clone();
            find_instance_mut(s.state_mut(), &id).expect("Solarius on the field").tuning =
                Some(json_as(json!({ "x": { "Spell Damage": 1 } })));
            assert!(s.stats(SOLARIUS).keywords.contains(&Keyword::SpellDamage { n: 3 }));
            s.play(LUNAR_ECLIPSE, at_hero());
            s.expect_health(P2, HERO_HEALTH - 6);
        }

        #[test]
        fn r386_no_draw_to_tune_a_draw_tuning_still_draws_nothing() {
            let mut s = scenario(json!({ "p1": { "hand": [SOLARIUS, FILLER], "library": [LUNAR_ECLIPSE, FILLER, FILLER] }, "p2": { "hand": [FILLER] } }));
            step_param(s.card_mut(SOLARIUS), "draw", 1);
            s.play(SOLARIUS, json!({}));
            assert_eq!(s.hand(P1).len(), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn spell_damage_5_a_3_damage_spell_deals_8() {
            let mut s = scenario(json!({
                "p1": { "hand": [LUNAR_ECLIPSE, FILLER], "field": [{ "def": SOLARIUS, "radiant": true }] },
                "p2": { "hand": [FILLER] },
            }));
            s.play(LUNAR_ECLIPSE, at_hero());
            s.expect_health(P2, HERO_HEALTH - 8);
        }

        #[test]
        fn no_cry_on_the_radiant_face_either_playing_draws_nothing() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": SOLARIUS, "radiant": true }, FILLER], "library": [LUNAR_ECLIPSE, FILLER, FILLER] },
                "p2": { "hand": [FILLER] },
            }));
            s.play(SOLARIUS, json!({}));
            assert_eq!(s.hand(P1).len(), 1);
        }

        #[test]
        fn death_shuffles_a_radiant_solarius_prime() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [{ "def": SOLARIUS, "radiant": true }], "library": [FILLER, FILLER] },
                "p2": { "hand": [FILLER], "field": [{ "def": MENACE, "radiant": true }] },
            }));
            s.attack(SOLARIUS, MENACE);
            s.expect_in_zone(SOLARIUS, "graveyard");
            let primes: Vec<bool> =
                s.pile(P1, "library").into_iter().filter(|card| card.def_id == PRIME).map(|card| card.radiant).collect();
            assert_eq!(primes, vec![true]);
            assert!(own_library_cards(&s).contains(&json!({ "defId": PRIME, "radiant": true, "count": 1 })));
        }
    }
}
