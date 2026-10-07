//! #47 Fig of Life (SPEC §8.2): "Heal a target 20", radiant "50" — a radiant cell that changes only
//! a number changes only that number (§8 Conventions), so the two faces are one effect at two sizes.
//!
//! R19 is the whole of the §8.2 Engine cell ("any unit or hero"): the declaration below says
//! `of: ["unit", "hero"]` with `side: "any"`, so the picker offers both heroes and every unit on
//! either side, and healing the opponent's board is legal if the player wants it.
//!
//! The pick is a DECLARED play-time target, so it travels in the play action's `targets` and never
//! pauses resolution (R81); `legalActions` builds the picker from the declaration without running
//! this script, and R90 validates what the play carried. It arrives as `{ of: "chosen" }`. An empty
//! target set — no units and, impossibly, no heroes — fizzles and the spell still counts as played
//! (§8 Conventions).
//!
//! What "heal 20" means is §6.3's Heal row, not this card's: a unit loses up to 20 damage and never
//! rises past its max health (healing never raises max health), while a hero simply gains 20 health
//! with no cap, because §3 gives a hero no maximum — a 30-health hero reaches 50. `effects/heal.ts`
//! is that split, so this file only names the amount.

use jackioh_engine::effects::heal;
use jackioh_engine::prelude::*;

pub const ID: &str = "core-047";

/// The two faces differ only in how much the target is healed.
fn fig_of_life(amount: i32) -> Script {
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
        cry: Some(hook(move |_ctx| {
            vec![heal(json_as(json!({ "target": { "of": "chosen" }, "amount": amount })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: fig_of_life(20),
        radiant: fig_of_life(50),
    }
}

// #47 Fig of Life — SPEC §8.2, BUILD M4-T4: "Heals a unit up to max or the hero without cap (R19);
// radiant 50".
//
// R19 is the §8.2 Engine cell in full: "Fig of Life may target any unit or hero", so the
// declaration says `side: "any"` with `of: ["unit", "hero"]` and the tests below heal an ally unit,
// an enemy unit, an own hero and an enemy hero.
//
// §8's "an empty target set fizzles" cannot arise for this card: both heroes are always legal
// targets, so there is always something to pick. What is asserted instead is the §6.3 Heal split —
// a unit never rises past its max health because healing only removes damage, while a hero has no
// maximum (§3) and goes straight past 30.
//
// The props are cards with no script beyond printed keywords, so nothing but the heal moves a
// number: #25 4-mana 7/7 (7/7, Armor 7) and #20 Pointmaster (7/2, First Strike).
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    /// TS's harness registered the real catalog and every script on import (`registerAll()`); the
    /// Rust testkit cannot name this crate, so the scenario builder registers first.
    fn scenario(opts: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(opts)
    }

    /// TS's `def` (`cardDef("core-047")`): the catalog card this file scripts.
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
    }
}
