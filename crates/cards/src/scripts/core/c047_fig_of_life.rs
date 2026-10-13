//! #47 Fig of Life (SPEC §8.2): "Heal a target 20", radiant "50" — a radiant cell that changes only
//! a number changes only that number (§8 Conventions), so the two faces are one effect at two sizes.
//!
//! R19 is the whole of the §8.2 Engine cell ("any unit or hero"): `side: "any"` with
//! `of: ["unit", "hero"]`, so either side's units and both heroes are offered.
//!
//! The pick is a DECLARED play-time target: it travels in the play action's `targets` and never
//! pauses resolution (R81), and R90 validates what the play carried. It arrives as `{ of: "chosen" }`.
//! An empty target set fizzles and the spell still counts as played (§8 Conventions).
//!
//! "Heal 20" is §6.3's Heal row, not this card's: a unit loses up to 20 damage and never rises past
//! its max health, while a hero gains 20 with no cap, since §3 gives a hero no maximum.

use jackioh_engine::effects::heal;
use jackioh_engine::prelude::*;

pub const ID: &str = "core-047";

/// The two faces differ only in how much the target is healed: the declared number `heal` (R386), 20
/// and 50 on the Radiant face, read off the face that is running.
fn fig_of_life() -> Script {
    Script {
        // R19: any unit or hero, either side.
        // R656: a heal helps, so a random cast that targets enemies aims this at friends.
        targets: vec![json_as(json!({
            "kind": "target",
            "min": 1,
            "max": 1,
            "filter": { "side": "any", "of": ["unit", "hero"] },
            "aim": "help",
        }))],
        cry: Some(hook(|ctx| {
            vec![heal(json_as(json!({ "target": { "of": "chosen" }, "amount": param(&*ctx, "heal") })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = fig_of_life();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// #47 Fig of Life — SPEC §8.2, BUILD M4-T4: "Heals a unit up to max or the hero without cap (R19);
// radiant 50".
//
// R19 is the §8.2 Engine cell in full ("Fig of Life may target any unit or hero"): the tests heal
// an ally unit, an enemy unit, an own hero and an enemy hero.
//
// §8's "an empty target set fizzles" cannot arise: both heroes are always legal targets. What is
// asserted instead is the §6.3 Heal split: a unit never rises past its max health, while a hero
// has no maximum (§3) and goes straight past 30. The props have no script beyond printed keywords.
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    /// The catalog card this file scripts.
    fn def() -> CardDef {
        crate::register_all();
        crate::card_def(ID)
    }

    const FIG: &str = "core-047";
    /// The radiant face in hand, so the radiant text is the one that resolves (§5.2).
    fn radiant_fig() -> Value {
        json!({ "def": FIG, "radiant": true })
    }
    const SEVEN_SEVEN: &str = "core-025";
    /// #45 Deft Duelist, 4/3
    const DUELIST: &str = "core-045";
    /// A unit that does nothing, kept on the board so a turn never runs out of legal actions (R82).
    const BYSTANDER: &str = SEVEN_SEVEN;

    /// The instance id of the unit in `player`'s lane `lane`, or the setup's message.
    fn unit_id(s: &Scenario, player: PlayerId, lane: i32, setup: &str) -> String {
        match s.unit(player, lane) {
            Some(card) => card.id,
            None => panic!("{setup}"),
        }
    }

    /// The play's declared pick (R81) of one instance.
    fn at_instance(id: &str) -> Value {
        json!({ "targets": [{ "pick": "instance", "instanceId": id }] })
    }

    /// The play's declared pick (R81) of a hero.
    fn at_hero(player: &str) -> Value {
        json!({ "targets": [{ "pick": "hero", "player": player }] })
    }

    /// "#47 Fig of Life"
    mod fig_of_life {
        use super::*;

        /// "R19 base: heals an ally unit, and never past its max health (§6.3)"
        #[test]
        fn r19_base_heals_an_ally_unit_and_never_past_its_max_health() {
            let mut g = scenario(json!({
                "p1": { "hand": [FIG], "field": [{ "def": SEVEN_SEVEN, "lane": 1, "damage": 5 }] },
            }));
            let ally = unit_id(&g, PlayerId::P1, 1, "setup: p1 should hold the 7/7");
            g.expect_stats(&ally, json!({ "health": 2, "maxHealth": 7 }));

            g.play(FIG, at_instance(&ally));

            // 20 healing onto 5 damage takes off 5: healing never raises max health.
            g.expect_stats(&ally, json!({ "health": 7, "maxHealth": 7 }));
            g.expect_events(json!(["cardPlayed", "healed", "enteredGraveyard"]));
            g.expect_in_zone(FIG, "graveyard");
            g.expect_mana(PlayerId::P1, 1);
        }

        /// "R19 base: heals an ENEMY unit — the target set is either side"
        #[test]
        fn r19_base_heals_an_enemy_unit_the_target_set_is_either_side() {
            let mut g = scenario(json!({
                "p1": { "hand": [FIG] },
                "p2": { "field": [{ "def": DUELIST, "lane": 3, "damage": 1 }] },
            }));
            let theirs = unit_id(&g, PlayerId::P2, 3, "setup: p2 should hold Deft Duelist");
            g.expect_stats(&theirs, json!({ "health": 2, "maxHealth": 3 }));

            g.play(FIG, at_instance(&theirs));

            g.expect_stats(&theirs, json!({ "health": 3, "maxHealth": 3 }));
        }

        /// "R19 base: heals your own hero with no cap — 30 becomes 50 (§3, §6.3)"
        #[test]
        fn r19_base_heals_your_own_hero_with_no_cap_30_becomes_50() {
            // Both sides keep a unit on the board: a turn with nothing meaningful left auto-ends (R82),
            // and a cascade of empty turns would add fatigue damage to the number under test.
            let mut g = scenario(json!({
                "p1": { "hand": [FIG], "field": [BYSTANDER] },
                "p2": { "field": [BYSTANDER] },
            }));
            g.expect_health(PlayerId::P1, 30);

            g.play(FIG, at_hero("p1"));

            g.expect_health(PlayerId::P1, 50);
            g.expect_events(json!(["healed"]));
        }

        /// "R19 base: an enemy hero is a legal target too, and is healed the same way"
        #[test]
        fn r19_base_an_enemy_hero_is_a_legal_target_too_and_is_healed_the_same_way() {
            let mut g = scenario(json!({
                "p1": { "hand": [FIG], "field": [BYSTANDER] },
                "p2": { "health": 12, "field": [BYSTANDER] },
            }));

            g.play(FIG, at_hero("p2"));

            g.expect_health(PlayerId::P2, 32);
            g.expect_health(PlayerId::P1, 30);
        }

        /// "radiant: 50 onto a unit is still capped by its max health (§8 Conventions: only the number)"
        #[test]
        fn radiant_50_onto_a_unit_is_still_capped_by_its_max_health() {
            let mut g = scenario(json!({
                "p1": { "hand": [radiant_fig()], "field": [{ "def": SEVEN_SEVEN, "lane": 1, "damage": 6 }] },
            }));
            let ally = unit_id(&g, PlayerId::P1, 1, "setup: p1 should hold the 7/7");
            g.expect_stats(&ally, json!({ "health": 1 }));

            g.play(FIG, at_instance(&ally));

            g.expect_stats(&ally, json!({ "health": 7, "maxHealth": 7 }));
        }

        /// "radiant: 50 onto a hero has no cap — 30 becomes 80"
        #[test]
        fn radiant_50_onto_a_hero_has_no_cap_30_becomes_80() {
            let mut g = scenario(json!({
                "p1": { "hand": [radiant_fig()], "field": [BYSTANDER] },
                "p2": { "field": [BYSTANDER] },
            }));

            g.play(FIG, at_hero("p1"));

            g.expect_health(PlayerId::P1, 80);
        }

        /// "radiant: an undamaged unit is healed for nothing and the spell still resolves"
        #[test]
        fn radiant_an_undamaged_unit_is_healed_for_nothing_and_the_spell_still_resolves() {
            let mut g = scenario(json!({
                "p1": { "hand": [radiant_fig()], "field": [{ "def": SEVEN_SEVEN, "lane": 2 }] },
            }));
            let ally = unit_id(&g, PlayerId::P1, 2, "setup: p1 should hold the 7/7");

            g.play(FIG, at_instance(&ally));

            g.expect_stats(&ally, json!({ "health": 7, "maxHealth": 7 }));
            g.expect_in_zone(FIG, "graveyard");
        }

        /// "R19, R81: both faces declare one play-time target — any unit or hero, either side"
        #[test]
        fn r19_r81_both_faces_declare_one_play_time_target_any_unit_or_hero_either_side() {
            let def = def();
            assert_eq!(def.id, FIG);
            assert_eq!(serde_json::to_value(def.type_).unwrap(), json!("Spell"));
            assert!(serde_json::to_value(&def.tags).unwrap().as_array().unwrap().contains(&json!("Fruit")));
            let scripts = script();
            for face in [&scripts.base, &scripts.radiant] {
                assert_eq!(
                    serde_json::to_value(&face.targets).unwrap(),
                    json!([{
                        "kind": "target",
                        "min": 1,
                        "max": 1,
                        "aim": "help",
                        "filter": { "side": "any", "of": ["unit", "hero"] },
                    }]),
                );
                // A declared target is not a prompt (R81), so nothing here opens one.
                assert!(face.modes.is_empty());
            }
        }

        #[test]
        fn r386_an_upgrade_heals_25_and_a_degrade_15() {
            for (upgrade, heal) in [(true, 25), (false, 15)] {
                let mut s = scenario(json!({
                    "p1": { "hand": [FIG], "health": 5, "field": [BYSTANDER] },
                    "p2": { "field": [BYSTANDER] },
                }));
                let moved = if upgrade {
                    crate::upgrade_number(&mut s, FIG, "heal")
                } else {
                    crate::degrade_number(&mut s, FIG, "heal")
                };
                assert_eq!(moved, heal);
                s.play(FIG, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
                s.expect_health(PlayerId::P1, 5 + heal);
            }
        }
    }
}
