//! #48 5pek Controller (SPEC §8.2): "Switch the position of every unit", radiant "Choose: all enemy
//! units, or all units".
//!
//! The radiant cell restates the whole clause (§8 Conventions), so it replaces the base one: the
//! radiant face switches one side or both, whichever the play named, and "all units" is the base
//! behaviour offered as one of the two options.
//!
//! R81 lists #48: the radiant choice is a DECLARED mode, so it travels in the play action's `modes`
//! and never pauses resolution — no `chooseMode`, no `PendingChoice`, and `chosenOptions(ctx)` reads
//! it back out of the context. The base face declares nothing, because it has nothing to ask.
//!
//! R20 is the §8.2 Engine cell: a position switch from a spell spends no exertion. That is not this
//! card's code either — `effects/position.ts` calls `combat.switchPosition` with
//! `spendExertion: false`, so `exertion.switched` stays false on every unit the effect touches and a
//! unit that has already attacked this turn still flips. §4.1's "one exertion per turn" is about the
//! player's own switch action, which `reduce` handles.
//!
//! §4.1's other ruling belongs to the same helper: "Spikey Pillow cannot be switched to Defense",
//! by an action or by an effect. `combat.switchPosition` refuses a switch to DEF when the unit's
//! script sets `StaticFlags.neverDefense` (#65.1), so an Attack-Position Spikey Pillow is skipped
//! and stays in Attack while everything around it flips, and a hypothetical one already in Defense
//! would still be allowed back to Attack.
//!
//! Every unit on the field switches, in each side's lane order: `switchAllPositions` walks
//! `activeUnitsOf`, so only the top card of a Stack pile is touched (R13) — a dormant card is not
//! on the field.

use jackioh_engine::effects::{chosen_options, switch_all_positions};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-048";

/// §8.2's two radiant options, as the play action spells them (R81).
const ENEMY: &str = "enemy";
const ALL: &str = "all";

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|_ctx| vec![switch_all_positions(json_as(json!({ "side": "both" })))])),
            ..Script::default()
        },
        radiant: Script {
            modes: vec![ModeDecl {
                kind: PromptKind::Mode,
                options: vec![ENEMY.to_string(), ALL.to_string()],
            }],
            cry: Some(hook(|ctx| {
                // R81: the mode arrived with the play, and R90 validated it against this declaration, so the
                // only way it is anything else is that no mode was carried. "All units" is what the card does
                // when it is not narrowed, so that is the reading an unnarrowed play gets.
                let mode = chosen_options(ctx).into_iter().next();
                let side = if mode.as_deref() == Some(ENEMY) { "enemy" } else { "both" };
                vec![switch_all_positions(json_as(json!({ "side": side })))]
            })),
            ..Script::default()
        },
    }
}

// #48 5pek Controller — SPEC §8.2, BUILD M4-T4: "Every unit switches, exertion untouched (R20),
// Spikey Pillow stays ATK; radiant enemy-only mode".
//
// R20 is asserted twice: once as "no unit's `exertion.switched` moved" and once as "a unit that has
// already attacked this turn still flips", which is the thing a switch costing exertion would make
// impossible (§4.1: one exertion per turn, one attack OR one switch).
//
// R81 is asserted as an absence: the radiant choice arrives in the play action's `modes`, so no
// `PendingChoice` is ever opened and `state.pending` stays null.
//
// The props are units with no script beyond printed keywords: #25 4-mana 7/7, #20 Pointmaster,
// #45 Deft Duelist, #11 Tempo Timmy. The exception is #65.1 Spikey Pillow, whose §4.1 ruling is the
// point of its case and whose `StaticFlags.neverDefense` lives in ITS card file.
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    /// TS's `def` (`cardDef("core-048")`): the catalog card this file scripts.
    fn def() -> CardDef {
        crate::register_all();
        crate::card_def(ID)
    }

    const CONTROLLER: &str = "core-048";
    /// The radiant face in hand, so the modal text is the one that resolves (§5.2).
    fn radiant_controller() -> Value {
        json!({ "def": CONTROLLER, "radiant": true })
    }
    const SEVEN_SEVEN: &str = "core-025";
    const POINTMASTER: &str = "core-020";
    const DUELIST: &str = "core-045";
    const TIMMY: &str = "core-011";
    const SPIKEY: &str = "core-065-1";

    fn position_at(g: &Scenario, player: PlayerId, lane: i32) -> Option<Position> {
        g.unit(player, lane).and_then(|unit| unit.position)
    }

    fn exertion_at(g: &Scenario, player: PlayerId, lane: i32) -> Option<Exertion> {
        g.unit(player, lane).map(|unit| unit.exertion)
    }

    /// "#48 5pek Controller"
    mod c5pek_controller {
        use super::*;

        /// "base: every unit on both sides switches position, in either direction"
        #[test]
        fn base_every_unit_on_both_sides_switches_position_in_either_direction() {
            let mut g = scenario(json!({
                "p1": {
                    "hand": [CONTROLLER],
                    "field": [
                        { "def": SEVEN_SEVEN, "lane": 1 },
                        { "def": POINTMASTER, "lane": 2, "position": "DEF" },
                    ],
                },
                "p2": {
                    "field": [
                        { "def": DUELIST, "lane": 1 },
                        { "def": TIMMY, "lane": 2, "position": "DEF" },
                    ],
                },
            }));
            assert_eq!(position_at(&g, PlayerId::P1, 1), Some(Position::Atk));
            assert_eq!(position_at(&g, PlayerId::P1, 2), Some(Position::Def));

            g.play(CONTROLLER, json!({}));

            assert_eq!(position_at(&g, PlayerId::P1, 1), Some(Position::Def));
            assert_eq!(position_at(&g, PlayerId::P1, 2), Some(Position::Atk));
            assert_eq!(position_at(&g, PlayerId::P2, 1), Some(Position::Def));
            assert_eq!(position_at(&g, PlayerId::P2, 2), Some(Position::Atk));
            g.expect_events(json!(["cardPlayed", "positionSwitched"]));
            g.expect_mana(PlayerId::P1, 4);
        }

        /// "R20 base: the switch spends no exertion — nothing's `switched` flag moves"
        #[test]
        fn r20_base_the_switch_spends_no_exertion_nothings_switched_flag_moves() {
            let mut g = scenario(json!({
                "p1": { "hand": [CONTROLLER], "field": [{ "def": SEVEN_SEVEN, "lane": 1 }] },
                "p2": { "field": [{ "def": DUELIST, "lane": 1 }, { "def": TIMMY, "lane": 2, "position": "DEF" }] },
            }));

            g.play(CONTROLLER, json!({}));

            for (player, lane) in [(PlayerId::P1, 1), (PlayerId::P2, 1), (PlayerId::P2, 2)] {
                let exertion = exertion_at(&g, player, lane);
                assert_eq!(
                    exertion.as_ref().map(|e| e.switched),
                    Some(false),
                    "{player} lane {lane} spent a switch",
                );
                assert_eq!(exertion.as_ref().map(|e| e.attacked), Some(false));
            }
        }

        /// "R20 base: a unit that already attacked this turn still switches"
        #[test]
        fn r20_base_a_unit_that_already_attacked_this_turn_still_switches() {
            let mut g = scenario(json!({
                "p1": { "hand": [CONTROLLER], "field": [{ "def": SEVEN_SEVEN, "lane": 1 }] },
                "p2": { "library": [TIMMY] },
            }));
            let Some(attacker) = g.unit(PlayerId::P1, 1).map(|unit| unit.id) else {
                panic!("setup: p1 should hold the 7/7");
            };

            g.attack(&attacker, "hero");
            g.expect_health(PlayerId::P2, 23);
            assert!(g.card(&attacker).exertion.attacked);

            g.play(CONTROLLER, json!({}));

            // §4.1 would refuse this as a player action; R20 says the effect does not care.
            assert_eq!(g.card(&attacker).position, Some(Position::Def));
            assert!(!g.card(&attacker).exertion.switched);
        }

        /// "§4.1 base: Spikey Pillow stays in Attack while everything around it flips"
        #[test]
        fn s4_1_base_spikey_pillow_stays_in_attack_while_everything_around_it_flips() {
            let mut g = scenario(json!({
                "p1": {
                    "hand": [CONTROLLER],
                    "field": [{ "def": SPIKEY, "lane": 1 }, { "def": SEVEN_SEVEN, "lane": 2 }],
                },
                "p2": { "field": [{ "def": DUELIST, "lane": 1 }] },
            }));

            g.play(CONTROLLER, json!({}));

            // `StaticFlags.neverDefense` (#65.1's own card file) makes `combat.switchPosition` refuse it,
            // and `switchAllPositions` walks on: one unit refusing never stops the rest.
            assert_eq!(position_at(&g, PlayerId::P1, 1), Some(Position::Atk));
            assert_eq!(exertion_at(&g, PlayerId::P1, 1).map(|e| e.switched), Some(false));
            assert_eq!(position_at(&g, PlayerId::P1, 2), Some(Position::Def));
            assert_eq!(position_at(&g, PlayerId::P2, 1), Some(Position::Def));
        }

        /// "radiant, mode \"enemy\": only the opponent's units switch (R81, no prompt)"
        #[test]
        fn r81_radiant_mode_enemy_only_the_opponents_units_switch_no_prompt() {
            let mut g = scenario(json!({
                "p1": { "hand": [radiant_controller()], "field": [{ "def": SEVEN_SEVEN, "lane": 1 }] },
                "p2": { "field": [{ "def": DUELIST, "lane": 1 }, { "def": TIMMY, "lane": 2, "position": "DEF" }] },
            }));

            g.play(CONTROLLER, json!({ "modes": ["enemy"] }));

            assert_eq!(position_at(&g, PlayerId::P1, 1), Some(Position::Atk));
            assert_eq!(position_at(&g, PlayerId::P2, 1), Some(Position::Def));
            assert_eq!(position_at(&g, PlayerId::P2, 2), Some(Position::Atk));
            // R81: a declared mode travels with the play, so resolution never paused.
            assert!(g.state().pending.is_none());
            assert!(!g.events().iter().any(|event| event.event_type() == GameEventType::PromptOpened));
        }

        /// "radiant, mode \"all\": both sides switch, exactly as the base face does"
        #[test]
        fn radiant_mode_all_both_sides_switch_exactly_as_the_base_face_does() {
            let mut g = scenario(json!({
                "p1": { "hand": [radiant_controller()], "field": [{ "def": SEVEN_SEVEN, "lane": 1 }] },
                "p2": { "field": [{ "def": DUELIST, "lane": 1 }] },
            }));

            g.play(CONTROLLER, json!({ "modes": ["all"] }));

            assert_eq!(position_at(&g, PlayerId::P1, 1), Some(Position::Def));
            assert_eq!(position_at(&g, PlayerId::P2, 1), Some(Position::Def));
            assert_eq!(exertion_at(&g, PlayerId::P1, 1).map(|e| e.switched), Some(false));
        }

        /// "R20 radiant: the enemy-only switch spends no enemy exertion either"
        #[test]
        fn r20_radiant_the_enemy_only_switch_spends_no_enemy_exertion_either() {
            let mut g = scenario(json!({
                "p1": { "hand": [radiant_controller()], "field": [{ "def": SEVEN_SEVEN, "lane": 1 }] },
                "p2": { "field": [{ "def": DUELIST, "lane": 1 }] },
            }));

            g.play(CONTROLLER, json!({ "modes": ["enemy"] }));

            assert_eq!(exertion_at(&g, PlayerId::P2, 1).map(|e| e.switched), Some(false));
        }

        /// "R81: only the radiant face declares a mode, and it declares no targets"
        #[test]
        fn r81_only_the_radiant_face_declares_a_mode_and_it_declares_no_targets() {
            let def = def();
            assert_eq!(def.id, CONTROLLER);
            assert_eq!(serde_json::to_value(def.type_).unwrap(), json!("Spell"));
            assert_eq!(serde_json::to_value(&def.cost).unwrap(), json!(0));
            let scripts = script();
            assert!(scripts.base.modes.is_empty());
            assert_eq!(
                serde_json::to_value(&scripts.radiant.modes).unwrap(),
                json!([{ "kind": "mode", "options": ["enemy", "all"] }]),
            );
            assert!(scripts.base.targets.is_empty());
            assert!(scripts.radiant.targets.is_empty());
        }
    }
}
