//! #11 Tempo Timmy (SPEC §8.1): 3/3 → 6/6 Unit, Human, cost 1. Base "Rush, First Strike", radiant
//! "Charge, First Strike". §8's Engine cell is "Keywords only", so there is nothing to script.
//!
//! The radiant cell lists keywords without "Plus", so it is the radiant form's COMPLETE keyword list
//! (§8 Conventions, "Reading a Radiant cell"): Rush is gone and Charge replaces it — §4.1's two
//! sickness lifts, Rush for unit targets only and Charge for units and the hero. Both faces' keyword
//! lists are printed in `catalog.json` (verified: base `[Rush, First Strike]`, radiant
//! `[Charge, First Strike]`) and §10.4 layer 1 reads them off the running face, so re-granting them
//! here would double them up. An empty Script is the whole card.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-011";

pub fn script() -> CardScripts {
    let base = Script::default();
    let radiant = Script::default();
    CardScripts { base, radiant }
}

// #11 Tempo Timmy (SPEC §8.1, BUILD M4-T4 row 11): "Attacks a unit on summon turn, not the hero;
// kills a 3-health unit unharmed; radiant may hit the hero".
//
// The card scripts nothing, so every clause here is a keyword behaving: §4.1's two sickness lifts
// (Rush for unit targets only, Charge for units and the hero) and §4.3 step 1's First Strike.
//
// HARNESS GAP: `SideSetup.hand` is `string[]`, so a RADIANT card cannot be seeded in a hand, and a
// radiant Cry or a radiant summoning-sick body can only be reached by playing one. Until `hand`
// takes `{ def, radiant }` the radiant tests set the flag on the hand instance themselves; see
// `play_radiant` below.
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    /// HARNESS GAP (see the header): make the hand copy radiant, then play it.
    fn play_radiant<'a>(s: &'a mut Scenario, card: &str) -> &'a mut Scenario {
        let id = s.card(card).id.clone();
        match find_instance_mut(s.state_mut(), &id) {
            Some(instance) => instance.radiant = true,
            None => panic!("no instance {id}"),
        }
        s.play(card, json!({}))
    }

    /// TS `expect(script).toEqual({})`: a face with no hook, no declaration and no flag. The
    /// destructuring names every field, so a field added to `Script` must be added here too.
    fn is_empty_script(script: &Script) -> bool {
        let Script {
            cost,
            cry,
            death,
            start_of_game,
            enters_hand,
            resume,
            delayed,
            set_stat,
            start_of_turn,
            end_of_turn,
            aura,
            triggers,
            on_play_hook,
            hand_triggers,
            static_flags,
            targets,
            modes,
            condition_met,
            preview,
            activations,
            target_checks,
            cost_aura,
            graveyard_play,
            targeting_discards,
            records_play_as,
            draw_limit,
            replacements,
            hero_guard,
            conditional_keywords,
            after_attack,
            plague_multiplier,
            deck_triggers,
            graveyard_triggers,
            quests,
            tribute_when,
            would_counter,
            start_of_opponent_turn,
            face_down_play,
        } = script;
        cost.is_none()
            && cry.is_none()
            && death.is_none()
            && start_of_game.is_none()
            && enters_hand.is_none()
            && resume.is_empty()
            && delayed.is_none()
            && set_stat.is_none()
            && start_of_turn.is_none()
            && end_of_turn.is_none()
            && aura.is_none()
            && triggers.is_empty()
            && on_play_hook.is_none()
            && hand_triggers.is_empty()
            && static_flags.is_none()
            && targets.is_empty()
            && modes.is_empty()
            && condition_met.is_none()
            && preview.is_none()
            && activations.is_empty()
            && target_checks.is_empty()
            && cost_aura.is_none()
            && graveyard_play.is_none()
            && targeting_discards.is_none()
            && records_play_as.is_none()
            && draw_limit.is_none()
            && replacements.is_empty()
            && hero_guard.is_none()
            && conditional_keywords.is_none()
            && after_attack.is_none()
            && plague_multiplier.is_none()
            && deck_triggers.is_empty()
            && graveyard_triggers.is_empty()
            && quests.is_none()
            && tribute_when.is_none()
            && would_counter.is_none()
            && start_of_opponent_turn.is_none()
            && face_down_play.is_none()
    }

    fn kinds(keywords: &[Keyword]) -> Vec<&'static str> {
        keywords.iter().map(|keyword| keyword.kind().as_str()).collect()
    }

    mod n11_tempo_timmy {
        use super::*;

        #[test]
        fn is_keywords_only_both_faces_print_their_keywords_and_neither_script_has_a_hook_s8() {
            crate::register_all();
            // §8 Conventions: a radiant cell that lists keywords without "Plus" is the COMPLETE radiant
            // list, so Rush is gone and Charge replaces it. Verified against the catalog here because a
            // mismatch would otherwise be papered over by a script that re-granted the keyword.
            let def = crate::card_def(ID);
            assert_eq!(kinds(&def.base.keywords), vec!["Rush", "First Strike"]);
            assert_eq!(kinds(&def.radiant.keywords), vec!["Charge", "First Strike"]);
            let scripts = script();
            assert!(is_empty_script(&scripts.base));
            assert!(is_empty_script(&scripts.radiant));
        }

        mod base {
            use super::*;

            #[test]
            fn attacks_a_unit_on_its_summon_turn_rush_s4_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": ["core-011", "core-005"], "library": ["core-005"] },
                    "p2": { "field": [{ "def": "core-008", "damage": 1 }], "library": ["core-005"] }
                }));
                s.play("core-011", json!({}));
                let vanilla = s.card("core-008").clone();

                s.attack("core-011", "core-008");

                s.expect_in_zone(&vanilla, "graveyard");
            }

            #[test]
            fn kills_a_3_health_unit_unharmed_first_strike_s4_3_step_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": ["core-011", "core-005"], "library": ["core-005"] },
                    // Mr. Vanilla is a 4/4 at 3 health with no First Strike of its own, so §4.3's step 1
                    // decides the exchange: Timmy's 3 lands first, the defender has fallen and "deals
                    // nothing" back.
                    "p2": { "field": [{ "def": "core-008", "damage": 1 }], "library": ["core-005"] }
                }));
                s.play("core-011", json!({}));
                let vanilla = s.card("core-008").clone();

                s.attack("core-011", "core-008");

                s.expect_in_zone(&vanilla, "graveyard");
                s.expect_stats("core-011", json!({ "attack": 3, "health": 3, "maxHealth": 3 }));
            }

            #[test]
            fn cannot_hit_the_hero_on_its_summon_turn_rush_not_charge_s4_2_step_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": ["core-011", "core-005"], "library": ["core-005"] },
                    "p2": { "field": [{ "def": "core-008", "damage": 1 }], "library": ["core-005"] }
                }));
                s.play("core-011", json!({}));

                s.expect_refused_with(
                    |s| s.attack("core-011", "hero"),
                    "Rush cannot hit the hero on its summon turn",
                );
                s.expect_health(PlayerId::P2, 30);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn may_hit_the_hero_on_its_summon_turn_charge_s4_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": ["core-011", "core-005"], "library": ["core-005"] },
                    "p2": { "field": [{ "def": "core-008", "damage": 1 }], "library": ["core-005"] }
                }));
                play_radiant(&mut s, "core-011");

                s.expect_stats("core-011", json!({ "attack": 6, "health": 6, "maxHealth": 6 }));
                s.attack("core-011", "hero");

                s.expect_health(PlayerId::P2, 24);
            }

            #[test]
            fn still_kills_a_3_health_unit_unharmed_first_strike_is_kept_s8_conventions() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": ["core-011", "core-005"], "library": ["core-005"] },
                    "p2": { "field": [{ "def": "core-008", "damage": 1 }], "library": ["core-005"] }
                }));
                play_radiant(&mut s, "core-011");
                let vanilla = s.card("core-008").clone();

                s.attack("core-011", "core-008");

                s.expect_in_zone(&vanilla, "graveyard");
                s.expect_stats("core-011", json!({ "health": 6 }));
            }
        }
    }
}
