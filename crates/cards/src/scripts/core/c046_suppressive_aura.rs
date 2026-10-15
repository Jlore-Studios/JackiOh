//! #46 Suppressive Aura (SPEC §8.2): "Aura: All Units have −1/−1. Paid (4): −2/−2 instead.", radiant
//! "Aura: Enemy Units have −2/−2. Paid (4): −4/−4 instead." The radiant cell restates the whole
//! clause and replaces the base one (§8 Conventions). Neither face prints a keyword.
//!
//! A pure §10.4 layer-5 contribution: `Script.aura` is read on every stat read, never applied once
//! and stored, which is all of "leaving restores them" (§8.2 Engine). Nothing here mutates
//! (CLAUDE.md rule 5), and an aura's `applies` predicate reads instance data only, or the layers recurse.
//!
//! Attack floors at 0, max health does not: a unit at 0 max health dies at the next state check (§4.5
//! step 1), Indestructible too, since no destroy effect is involved (R69).
//!
//! The price rides in the `play` action (R81; §10.6: no Core card opens an `embiggen` prompt), and R65
//! makes `instance.embiggened` the price paid, so the aura reads it off `self`. R78: leaving the
//! field resets it.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-046";

/// The penalty at the price this card was played for (R65): the declared number `debuff` at the base
/// price and `paidDebuff` at the embiggen price (R386) — 1 and 2, 2 and 4 on the Radiant face.
fn penalty(args: &HookArgs<'_>) -> i32 {
    if args.self_.embiggened == Some(true) {
        param(args, "paidDebuff")
    } else {
        param(args, "debuff")
    }
}

/// `-penalty` on both stats.
fn suppression(penalty: i32) -> StatMod {
    StatMod {
        attack: Some(-penalty),
        max_health: Some(-penalty),
        ..StatMod::default()
    }
}

/// "All units": every unit on either side, the controller's own included, at one of the two prices.
/// The mod is negative on both stats, so §10.4 subtracts it from attack (floored at 0 on read) and
/// from max health (not floored, R69).
fn all_units(args: HookArgs<'_>) -> Vec<AuraEntry<'_>> {
    vec![AuraEntry {
        applies: Box::new(|_unit: &CardInstance| true),
        mod_: suppression(penalty(&args)),
    }]
}

/// "Enemy units": enemy of this card's controller, which is control and not ownership (R12), so a
/// stolen or rotated Suppressive Aura suppresses the other board from its new side.
fn enemy_units(args: HookArgs<'_>) -> Vec<AuraEntry<'_>> {
    let controller = args.self_.controller;
    vec![AuraEntry {
        applies: Box::new(move |unit: &CardInstance| unit.controller != controller),
        mod_: suppression(penalty(&args)),
    }]
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            aura: Some(aura_hook(all_units)),
            ..Script::default()
        },
        radiant: Script {
            aura: Some(aura_hook(enemy_units)),
            ..Script::default()
        },
    }
}

// #46 Suppressive Aura — SPEC §8.2, BUILD M4-T4: "Embiggen price chosen with the play (R81);
// −1/−1 to all, 1-health units die, restored on leaving; paid 4 → −2/−2; radiant enemy only −2/−2,
// paid 4 → −4/−4; enough of them together kill an enemy The Rock despite Indestructible (R69)"
//
// Stat assertions go through `expect_stats`, which reads the engine's `unitView`, so §10.4 layer 5
// is already in the number. The props have no script beyond printed keywords, so only the aura
// moves their numbers.
//
// A scenario cannot place a card with `embiggened` set (R81 makes the price part of the play), so
// every paid-4 case plays from hand with `{ embiggen: true }`, which writes it (R65).
#[cfg(test)]
mod tests {
    use super::{ID, script};
    // The harness has no verb for a permanent leaving the field, so that one step uses the engine's mover.
    use jackioh_engine::testkit::*;
    use jackioh_engine::zones::{OffFieldZone, move_to_zone};

    use crate::scenario;

    /// The catalog card this file scripts.
    fn def() -> CardDef {
        crate::register_all();
        crate::card_def(ID)
    }

    const AURA: &str = "core-046";
    /// The radiant face of a card that has to be PLAYED for its embiggen price to exist (R81).
    fn radiant_aura() -> Value {
        json!({ "def": AURA, "radiant": true })
    }
    const SEVEN_SEVEN: &str = "core-025";
    const POINTMASTER: &str = "core-020";
    const DUELIST: &str = "core-045";
    const JILLIAX: &str = "core-056";
    const ROCK: &str = "core-066";

    /// The instance id of the unit in `player`'s lane `lane`, or the setup's message.
    fn unit_id(s: &Scenario, player: PlayerId, lane: i32, setup: &str) -> String {
        match s.unit(player, lane) {
            Some(card) => card.id,
            None => panic!("{setup}"),
        }
    }

    /// "#46 Suppressive Aura"
    mod suppressive_aura {
        use super::*;

        /// "base: every unit on both sides is −1/−1 while it is in play (§10.4 layer 5)"
        #[test]
        fn base_every_unit_on_both_sides_is_minus_1_minus_1_while_it_is_in_play() {
            let mut g = scenario(json!({
                "p1": { "hand": [AURA], "field": [{ "def": SEVEN_SEVEN, "lane": 1 }] },
                "p2": { "field": [{ "def": SEVEN_SEVEN, "lane": 1 }] },
            }));
            let mine = g.unit(PlayerId::P1, 1);
            let theirs = g.unit(PlayerId::P2, 1);
            assert!(mine.is_some());
            assert!(theirs.is_some());
            let (Some(mine), Some(theirs)) = (mine, theirs) else {
                return;
            };

            g.expect_stats(&mine, json!({ "attack": 7, "maxHealth": 7, "health": 7 }));
            g.expect_stats(&theirs, json!({ "attack": 7, "maxHealth": 7, "health": 7 }));

            g.play(AURA, json!({}));

            // "All Units": the controller's own board is suppressed too.
            g.expect_stats(&mine, json!({ "attack": 6, "maxHealth": 6, "health": 6 }));
            g.expect_stats(&theirs, json!({ "attack": 6, "maxHealth": 6, "health": 6 }));
            g.expect_mana(PlayerId::P1, 2);
        }

        /// "base: a 1-health unit dies at the state check, with no destroy effect involved (§4.5)"
        #[test]
        fn base_a_1_health_unit_dies_at_the_state_check_with_no_destroy_effect_involved() {
            let mut g = scenario(json!({
                "p1": { "hand": [AURA] },
                "p2": { "field": [{ "def": POINTMASTER, "lane": 2 }, { "def": SEVEN_SEVEN, "lane": 3 }] },
            }));
            let doomed = unit_id(&g, PlayerId::P2, 2, "setup: p2 should hold both units");
            let survivor = unit_id(&g, PlayerId::P2, 3, "setup: p2 should hold both units");

            g.play(AURA, json!({}));

            // §10.4: max health can fall to 0, and §4.5 step 1 collects the unit at the next state check.
            g.expect_in_zone(&doomed, "graveyard");
            g.expect_events(json!(["cardPlayed", "destroyed", "enteredGraveyard"]));
            g.expect_stats(&survivor, json!({ "attack": 6, "maxHealth": 6 }));
        }

        /// "base: leaving the field restores the survivors (§8.2 Engine)"
        #[test]
        fn base_leaving_the_field_restores_the_survivors() {
            let mut g = scenario(json!({
                "p1": { "hand": [AURA], "field": [{ "def": SEVEN_SEVEN, "lane": 1 }] },
                "p2": { "field": [{ "def": DUELIST, "lane": 1 }] },
            }));
            g.play(AURA, json!({}));
            let mine = unit_id(&g, PlayerId::P1, 1, "setup: both units should still be there");
            let theirs = unit_id(&g, PlayerId::P2, 1, "setup: both units should still be there");
            g.expect_stats(&mine, json!({ "attack": 6, "maxHealth": 6 }));
            g.expect_stats(&theirs, json!({ "attack": 3, "maxHealth": 2 }));

            let Some(mut aura) = g.backrow(PlayerId::P1, 1) else {
                panic!("the Field Spell should be in p1's backrow");
            };
            let _ = move_to_zone(g.state_mut(), &mut aura, OffFieldZone::Graveyard, Default::default());

            // The aura is a layer computed on every read, so nothing had to be undone.
            g.expect_stats(&mine, json!({ "attack": 7, "maxHealth": 7, "health": 7 }));
            g.expect_stats(&theirs, json!({ "attack": 4, "maxHealth": 3, "health": 3 }));
        }

        /// "R81, R65 base paid 4: the price chosen with the play makes it −2/−2"
        #[test]
        fn r81_r65_base_paid_4_the_price_chosen_with_the_play_makes_it_minus_2_minus_2() {
            let mut g = scenario(json!({
                "p1": { "hand": [AURA], "field": [{ "def": SEVEN_SEVEN, "lane": 1 }] },
                "p2": { "field": [{ "def": JILLIAX, "lane": 1 }, { "def": DUELIST, "lane": 2 }] },
            }));
            let mine = unit_id(&g, PlayerId::P1, 1, "setup: the units should be there");
            let doomed = unit_id(&g, PlayerId::P2, 1, "setup: the units should be there");
            let survivor = unit_id(&g, PlayerId::P2, 2, "setup: the units should be there");

            // The price travels in the play action, never as a prompt (§10.6: no Core card asks).
            g.play(AURA, json!({ "embiggen": true }));

            g.expect_mana(PlayerId::P1, 0);
            g.expect_stats(&mine, json!({ "attack": 5, "maxHealth": 5 }));
            // A 3/2 is at 0 max health now, and the 4/3 is a 2/1.
            g.expect_in_zone(&doomed, "graveyard");
            g.expect_stats(&survivor, json!({ "attack": 2, "maxHealth": 1 }));
            assert!(g.state().pending.is_none());
        }

        /// "base: playing it for 2 leaves the instance unembiggened (R65's 'chosen price')"
        #[test]
        fn r65_base_playing_it_for_2_leaves_the_instance_unembiggened() {
            let mut g = scenario(json!({ "p1": { "hand": [AURA] } }));
            g.play(AURA, json!({}));
            assert_eq!(g.backrow(PlayerId::P1, 1).and_then(|card| card.embiggened), Some(false));
        }

        /// "radiant: enemy units are −2/−2 and the controller's own are untouched"
        #[test]
        fn radiant_enemy_units_are_minus_2_minus_2_and_the_controllers_own_are_untouched() {
            let mut g = scenario(json!({
                "p1": { "hand": [radiant_aura()], "field": [{ "def": SEVEN_SEVEN, "lane": 1 }] },
                "p2": { "field": [{ "def": SEVEN_SEVEN, "lane": 1 }] },
            }));
            let mine = unit_id(&g, PlayerId::P1, 1, "setup: both units should be there");
            let theirs = unit_id(&g, PlayerId::P2, 1, "setup: both units should be there");

            g.play(AURA, json!({}));

            g.expect_stats(&mine, json!({ "attack": 7, "maxHealth": 7, "health": 7 }));
            g.expect_stats(&theirs, json!({ "attack": 5, "maxHealth": 5, "health": 5 }));
        }

        /// "R81 radiant paid 4: enemy units are −4/−4"
        #[test]
        fn r81_radiant_paid_4_enemy_units_are_minus_4_minus_4() {
            let mut g = scenario(json!({
                "p1": { "hand": [radiant_aura()], "field": [{ "def": SEVEN_SEVEN, "lane": 1 }] },
                "p2": { "field": [{ "def": SEVEN_SEVEN, "lane": 1 }] },
            }));
            let theirs = unit_id(&g, PlayerId::P2, 1, "setup: the enemy unit should be there");

            g.play(AURA, json!({ "embiggen": true }));

            g.expect_mana(PlayerId::P1, 0);
            g.expect_stats(&theirs, json!({ "attack": 3, "maxHealth": 3, "health": 3 }));
            g.expect_stats(SEVEN_SEVEN, json!({ "attack": 7, "maxHealth": 7 }));
        }

        /// "R69 radiant paid 4 on top of three more radiant auras: an enemy The Rock at 0 max health dies despite Indestructible"
        #[test]
        fn r69_radiant_paid_4_on_top_of_three_more_radiant_auras_an_enemy_the_rock_at_0_max_health_dies() {
            let mut g = scenario(json!({
                "p1": {
                    "hand": [radiant_aura()],
                    "field": [{ "def": ROCK, "lane": 1 }],
                    // Three placed radiant auras at their base price: −6/−6 on the enemy between them.
                    "backrow": [radiant_aura(), radiant_aura(), radiant_aura()],
                },
                "p2": { "field": [{ "def": ROCK, "lane": 1 }] },
            }));
            let mine = unit_id(&g, PlayerId::P1, 1, "setup: both Rocks should be there");
            let doomed = unit_id(&g, PlayerId::P2, 1, "setup: both Rocks should be there");
            g.expect_stats(&doomed, json!({ "attack": 4, "maxHealth": 4 }));

            g.play(AURA, json!({ "embiggen": true }));

            // R69: no destroy effect is involved, so Indestructible does not save a unit at 0 max health.
            g.expect_in_zone(&doomed, "graveyard");
            g.expect_events(json!(["destroyed"]));
            let death = g.last_events().iter().find_map(|event| match event {
                GameEvent::Destroyed {
                    instance_id,
                    max_health,
                    killer_id,
                    ..
                } if *instance_id == doomed => Some((*max_health, killer_id.clone())),
                _ => None,
            });
            assert_eq!(death, Some((0, None)));
            // Enemy units only: the auras' own side keeps its 10/10 Rock.
            g.expect_stats(&mine, json!({ "attack": 10, "maxHealth": 10, "health": 10 }));
        }

        /// "radiant: an ally at 0 max health would die too, but the radiant aura never touches allies"
        #[test]
        fn radiant_an_ally_at_0_max_health_would_die_too_but_the_radiant_aura_never_touches_allies() {
            let mut g = scenario(json!({
                "p1": { "hand": [radiant_aura()], "field": [{ "def": POINTMASTER, "lane": 1 }] },
                "p2": { "field": [{ "def": POINTMASTER, "lane": 1 }] },
            }));
            let mine = unit_id(&g, PlayerId::P1, 1, "setup: both units should be there");
            let doomed = unit_id(&g, PlayerId::P2, 1, "setup: both units should be there");

            g.play(AURA, json!({}));

            g.expect_in_zone(&doomed, "graveyard");
            g.expect_in_zone(&mine, "field");
            g.expect_stats(&mine, json!({ "attack": 7, "maxHealth": 1 }));
        }

        /// "both faces are a §10.4 layer-5 aura and ask for nothing at play time (R81, §10.6)"
        #[test]
        fn r81_both_faces_are_a_layer_5_aura_and_ask_for_nothing_at_play_time() {
            let def = def();
            assert_eq!(def.id, AURA);
            assert_eq!(serde_json::to_value(def.type_).unwrap(), json!("Field Spell"));
            assert_eq!(serde_json::to_value(def.cost).unwrap(), json!({ "base": 2, "embiggen": 4 }));
            let scripts = script();
            for face in [&scripts.base, &scripts.radiant] {
                assert!(face.aura.is_some());
                // The price is not a declared choice and not a prompt: it rides in the play action (R81).
                assert!(face.targets.is_empty());
                assert!(face.modes.is_empty());
                assert!(face.cry.is_none());
            }
        }

        #[test]
        fn r386_an_upgrade_makes_the_base_price_2_2_and_a_radiant_degrade_1_1() {
            for (radiant, upgrade, debuff) in [(false, true, 2), (true, false, 1)] {
                let mut g = scenario(json!({
                    "p1": { "hand": [{ "def": AURA, "radiant": radiant }] },
                    "p2": { "field": [{ "def": SEVEN_SEVEN, "lane": 1 }] },
                }));
                let moved = if upgrade {
                    crate::upgrade_number(&mut g, AURA, "debuff")
                } else {
                    crate::degrade_number(&mut g, AURA, "debuff")
                };
                assert_eq!(moved, debuff);
                g.play(AURA, json!({}));
                let theirs = unit_id(&g, PlayerId::P2, 1, "p2's 7/7");
                g.expect_stats(theirs.as_str(), json!({ "attack": 7 - debuff, "maxHealth": 7 - debuff }));
            }
        }

        #[test]
        fn r386_an_upgrade_makes_the_paid_price_3_3_and_a_degrade_1_1() {
            for (upgrade, debuff) in [(true, 3), (false, 1)] {
                let mut g = scenario(json!({
                    "p1": { "hand": [AURA] },
                    "p2": { "field": [{ "def": SEVEN_SEVEN, "lane": 1 }] },
                }));
                let moved = if upgrade {
                    crate::upgrade_number(&mut g, AURA, "paidDebuff")
                } else {
                    crate::degrade_number(&mut g, AURA, "paidDebuff")
                };
                assert_eq!(moved, debuff);
                g.play(AURA, json!({ "embiggen": true }));
                let theirs = unit_id(&g, PlayerId::P2, 1, "p2's 7/7");
                g.expect_stats(theirs.as_str(), json!({ "attack": 7 - debuff, "maxHealth": 7 - debuff }));
            }
        }
    }
}
