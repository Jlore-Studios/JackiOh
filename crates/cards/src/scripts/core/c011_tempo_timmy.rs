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
    CardScripts { base: Script::default(), radiant: Script::default() }
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
// `playRadiant` below.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;
    use serde_json::json;

    /// HARNESS GAP (see the header): make the hand copy radiant, then play it.
    fn play_radiant(s: &mut Scenario, card: &str) {
        s.card_mut(card).radiant = true;
        s.play(card, json!({}));
    }

    /// TS `expect(script).toEqual({})`: no hook, no declaration and no data field is set.
    fn is_empty_script(sc: &Script) -> bool {
        sc.cost.is_none()
            && sc.cry.is_none()
            && sc.death.is_none()
            && sc.start_of_game.is_none()
            && sc.resume.is_empty()
            && sc.delayed.is_none()
            && sc.set_stat.is_none()
            && sc.start_of_turn.is_none()
            && sc.end_of_turn.is_none()
            && sc.aura.is_none()
            && sc.triggers.is_empty()
            && sc.on_play_hook.is_none()
            && sc.hand_triggers.is_empty()
            && sc.static_flags.is_none()
            && sc.targets.is_empty()
            && sc.modes.is_empty()
            && sc.condition_met.is_none()
            && sc.preview.is_none()
            && sc.activations.is_empty()
            && sc.target_checks.is_empty()
            && sc.cost_aura.is_none()
            && sc.graveyard_play.is_none()
            && sc.targeting_discards.is_none()
            && sc.records_play_as.is_none()
            && sc.draw_limit.is_none()
            && sc.replacements.is_empty()
            && sc.hero_guard.is_none()
            && sc.conditional_keywords.is_none()
            && sc.after_attack.is_none()
            && sc.plague_multiplier.is_none()
            && sc.deck_triggers.is_empty()
            && sc.graveyard_triggers.is_empty()
            && sc.quests.is_none()
            && sc.tribute_when.is_none()
            && sc.would_counter.is_none()
            && sc.start_of_opponent_turn.is_none()
    }

    /// The `kind` of every keyword in a printed keyword list, in order.
    fn kinds(keywords: &[Keyword]) -> Vec<String> {
        serde_json::to_value(keywords)
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|k| k["kind"].as_str().unwrap().to_string())
            .collect()
    }

    mod c11_tempo_timmy {
        use super::*;

        #[test]
        fn is_keywords_only_both_faces_print_their_keywords_and_neither_script_has_a_hook_8() {
            // §8 Conventions: a radiant cell that lists keywords without "Plus" is the COMPLETE radiant
            // list, so Rush is gone and Charge replaces it. Verified against the catalog here because a
            // mismatch would otherwise be papered over by a script that re-granted the keyword.
            let def = crate::card_def(ID);
            assert_eq!(kinds(&def.base.keywords), vec!["Rush", "First Strike"]);
            assert_eq!(kinds(&def.radiant.keywords), vec!["Charge", "First Strike"]);
            let sc = script();
            assert!(is_empty_script(&sc.base));
            assert!(is_empty_script(&sc.radiant));
        }

        mod base {
            use super::*;

            #[test]
            fn attacks_a_unit_on_its_summon_turn_rush_4_1() {
                let mut s = scenario(json!({
                    "p1": { "hand": ["core-011", "core-005"], "library": ["core-005"] },
                    "p2": { "field": [{ "def": "core-008", "damage": 1 }], "library": ["core-005"] }
                }));
                s.play("core-011", json!({}));
                let vanilla = s.card("core-008").id.clone();

                s.attack("core-011", "core-008");

                s.expect_in_zone(&vanilla, "graveyard");
            }

            #[test]
            fn kills_a_3_health_unit_unharmed_first_strike_4_3_step_1() {
                let mut s = scenario(json!({
                    "p1": { "hand": ["core-011", "core-005"], "library": ["core-005"] },
                    // Mr. Vanilla is a 4/4 at 3 health with no First Strike of its own, so §4.3's step 1
                    // decides the exchange: Timmy's 3 lands first, the defender has fallen and "deals
                    // nothing" back.
                    "p2": { "field": [{ "def": "core-008", "damage": 1 }], "library": ["core-005"] }
                }));
                s.play("core-011", json!({}));
                let vanilla = s.card("core-008").id.clone();

                s.attack("core-011", "core-008");

                s.expect_in_zone(&vanilla, "graveyard");
                s.expect_stats("core-011", json!({ "attack": 3, "health": 3, "maxHealth": 3 }));
            }

            #[test]
            fn cannot_hit_the_hero_on_its_summon_turn_rush_not_charge_4_2_step_2() {
                let mut s = scenario(json!({
                    "p1": { "hand": ["core-011", "core-005"], "library": ["core-005"] },
                    "p2": { "field": [{ "def": "core-008", "damage": 1 }], "library": ["core-005"] }
                }));
                s.play("core-011", json!({}));

                s.expect_refused_with(
                    |s| {
                        s.attack("core-011", "hero");
                    },
                    "Rush cannot hit the hero on its summon turn",
                );
                s.expect_health(PlayerId::P2, 30);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn may_hit_the_hero_on_its_summon_turn_charge_4_1() {
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
            fn still_kills_a_3_health_unit_unharmed_first_strike_is_kept_8_conventions() {
                let mut s = scenario(json!({
                    "p1": { "hand": ["core-011", "core-005"], "library": ["core-005"] },
                    "p2": { "field": [{ "def": "core-008", "damage": 1 }], "library": ["core-005"] }
                }));
                play_radiant(&mut s, "core-011");
                let vanilla = s.card("core-008").id.clone();

                s.attack("core-011", "core-008");

                s.expect_in_zone(&vanilla, "graveyard");
                s.expect_stats("core-011", json!({ "health": 6 }));
            }
        }
    }
}
