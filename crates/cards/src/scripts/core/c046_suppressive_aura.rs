//! #46 Suppressive Aura (SPEC §8.2): "Aura: All Units have −1/−1. Paid (4): −2/−2 instead.", radiant
//! "Aura: Enemy Units have −2/−2. Paid (4): −4/−4 instead." (patch v0.1.1 halved the paid-2 numbers
//! and cut the paid-4 ones from −5/−5 and −10/−10).
//!
//! Reading the radiant cell (§8 Conventions): it restates the whole clause, so it replaces the base
//! one — the radiant face touches enemy units only and carries its own two numbers. Neither face
//! prints a keyword, so the keyword list stays empty on both.
//!
//! This is a pure §10.4 layer-5 contribution and nothing else. `Script.aura` is read on every stat
//! read (`layers.unitView`), never applied once and stored, which is the whole of "leaving restores
//! them" in the §8.2 Engine cell: when the Field Spell leaves the backrow `layers.auraSources` stops
//! finding it and the next read of a survivor is its unmodified self again. Nothing in this file
//! mutates anything (CLAUDE.md rule 5); an aura's `applies` predicate also reads instance data only
//! and never calls back into `unitView`, or the layers would recurse.
//!
//! Attack floors at 0 but max health does not: §10.4 layer 5 lets it fall to 0 or less, and §4.5
//! step 1 collects such a unit at the next state check — R69 spells out that this reaches an
//! Indestructible unit too, because no destroy effect is involved: it dies, fires Death, may Reborn
//! and counts toward Ceaseless Void's destroyed counter. So radiant paid 4 (−4/−4) kills an
//! Indestructible unit with 4 health or less while no `destroy` verb appears anywhere below.
//!
//! The price is not a choice this card asks for. R81 lists #46: zone, X, embiggen, Tribute and the
//! declared targets and modes all travel in the `play` action, and §10.6 adds that no Core card ever
//! opens an `embiggen` prompt. `reduce.playCard` writes the answer to `instance.embiggened` and R65
//! makes that the cost actually paid, so the aura simply reads the flag off `self` — no `targets`
//! and no `modes` declaration belongs here.
//!
//! R78: `embiggened` is one of the fields leaving the field resets, so a Suppressive Aura that is
//! bounced and replayed for 2 is a −1/−1 aura again, which is what "the chosen embiggen price" means.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-046";

/// One face's two prices (TS `{ paid2, paid4 }`): the penalty at the base price, then at the
/// embiggen price (R65).
struct Penalties {
    paid2: i32,
    paid4: i32,
}

/// The two prices of §8.2's cell: the base price, then the embiggen price (R65).
const BASE_PENALTIES: Penalties = Penalties { paid2: 1, paid4: 2 };
const RADIANT_PENALTIES: Penalties = Penalties { paid2: 2, paid4: 4 };

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
    let penalty = if args.self_.embiggened == Some(true) {
        BASE_PENALTIES.paid4
    } else {
        BASE_PENALTIES.paid2
    };
    vec![AuraEntry {
        applies: Box::new(|_unit: &CardInstance| true),
        mod_: suppression(penalty),
    }]
}

/// "Enemy units": enemy of this card's controller, which is control and not ownership (R12), so a
/// stolen or rotated Suppressive Aura suppresses the other board from its new side.
fn enemy_units(args: HookArgs<'_>) -> Vec<AuraEntry<'_>> {
    let penalty = if args.self_.embiggened == Some(true) {
        RADIANT_PENALTIES.paid4
    } else {
        RADIANT_PENALTIES.paid2
    };
    let controller = args.self_.controller;
    vec![AuraEntry {
        applies: Box::new(move |unit: &CardInstance| unit.controller != controller),
        mod_: suppression(penalty),
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
// (patch v0.1.1 cut the numbers from −2/−2 and −5/−5, radiant −4/−4 and −10/−10).
//
// Every stat assertion goes through `expectStats`, which reads the engine's `unitView`, so the
// number already has §10.4 layer 5 in it — the aura is never asserted by inspecting the script.
//
// The props, chosen for having no script of their own beyond printed keywords, so nothing but the
// aura moves their numbers:
//   #25 4-mana 7/7 (7/7, Armor 7)   — the survivor, and the unit whose restoration is asserted.
//   #20 Pointmaster (7/1, First Strike) — the 1-health unit §8.2's Engine cell kills.
//   #45 Deft Duelist (4/3, Charge)  — survives −1/−1 and −2/−2.
//   #56 Jilliax (3/2, Rush, Taunt, Lifesteal, Divine Shield) — survives −1/−1, dies to −2/−2.
//   #66 The Rock (10/10, Indestructible) — R69's unit, killed only once its max health reaches 0.
//
// The embiggen price cannot be set on a card the scenario builder PLACES — no setup entry carries
// `embiggened` — which is as it should be: R81 makes the price part of the play. So every paid-4
// case here plays the card out of hand with `{ embiggen: true }` and checks the mana as well, and
// `reduce.playCard` is what writes the answer to `instance.embiggened` (R65).
#[cfg(test)]
mod tests {
    use super::{ID, script};
    // The harness has no verb for "a permanent leaves the field" and none of #46's own text removes
    // one, so the engine's own mover is used for that one step (see the "restored on leaving" test).
    use jackioh_engine::testkit::*;
    use jackioh_engine::zones::{OffFieldZone, move_to_zone};

    /// TS's harness registered the real catalog and every script on import (`registerAll()`); the
    /// Rust testkit cannot name this crate, so the scenario builder registers first.
    fn scenario(opts: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(opts)
    }

    /// TS's `def` (`cardDef("core-046")`): the catalog card this file scripts.
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

            // R69: no destroy effect is involved, so Indestructible does not save a unit at 0 max health;
            // it is collected like any other, which is why this is a death and not a survival.
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
            assert_eq!(serde_json::to_value(&def.cost).unwrap(), json!({ "base": 2, "embiggen": 4 }));
            let scripts = script();
            for face in [&scripts.base, &scripts.radiant] {
                assert!(face.aura.is_some());
                // The price is not a declared choice and not a prompt: it rides in the play action (R81).
                assert!(face.targets.is_empty());
                assert!(face.modes.is_empty());
                assert!(face.cry.is_none());
            }
        }
    }
}
