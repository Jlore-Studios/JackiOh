//! C #81 The Power to Thrive (SPEC §8.6 row 81). (2) Field Spell, Rare.
//!   Base:    "Activate: Choose one: Heal your hero {heal}; draw {draw}; or gain {mana} mana." — 3, 1, 1
//!   Radiant: the same text — 6, 2, 2
//!   Engine:  "Activate (§6.2, R384), once per turn, the mode declared in the action (R81); the mana is
//!            temporary (§2.3) and may exceed 4. "Active" is Activate. Tunes: heal 3 ↑; draw 1 ↑; mana 1 ↑."
//!
//! One ability, once per turn, its mode carried in the `activate` action. Heal has no cap on a hero (R19).
//! The three numbers are declared (R386).

use jackioh_engine::effects::{draw, gain_mana, heal};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-081";

const MODES: [&str; 3] = ["heal", "draw", "mana"];

pub fn script() -> CardScripts {
    let base = Script {
        activations: vec![ActivationDecl {
            id: "thrive".into(),
            label: "Choose one".into(),
            uses: ActivationUses::Count(1),
            cost: None,
            targets: vec![],
            modes: vec![ModeDecl {
                kind: PromptKind::Mode,
                options: MODES.iter().map(|mode| mode.to_string()).collect(),
            }],
            can_activate: None,
            has: None,
            run: hook(|ctx| {
                let mode = ctx.modes.first().cloned();
                if mode.as_deref() == Some("heal") {
                    return vec![heal(json_as(json!({ "target": { "of": "selfHero" }, "amount": param(&*ctx, "heal") })))];
                }
                if mode.as_deref() == Some("draw") {
                    return vec![draw(json_as(json!({ "count": param(&*ctx, "draw") })))];
                }
                if mode.as_deref() == Some("mana") {
                    vec![gain_mana(json_as(json!({ "amount": param(&*ctx, "mana") })))]
                } else {
                    vec![]
                }
            }),
        }],
        ..Script::default()
    };

    // The same script: the Radiant face's 6, 2 and 2 are its declared numbers.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C #81 The Power to Thrive — SPEC §8.6 row 81, BUILD M9 Classic row C 81: "Activate, once per turn
// (R384), the mode carried in the `activate` action: heal your hero 3 (no cap, R19), draw 1, or gain 1 mana
// this turn; usable the turn it is played; a second activation that turn is refused; not a play; radiant:
// heal 6, draw 2, or 2 mana; its tuned numbers (heal, draw, mana) read through `param()` (R386)".
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;
    use std::sync::Arc;

    const P1: PlayerId = PlayerId::P1;

    const THRIVE: &str = "classic-081";
    const FILLER: &str = "core-005"; // (1) Spell (§2.5).
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).
    const X: &str = "core-020"; // library filler.

    /// The harness, after the catalog and every card script are registered (TS's harness did it on import).
    fn scenario(opts: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(opts)
    }

    fn def() -> CardDef {
        crate::register_all();
        crate::card_def(ID)
    }

    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialisable")
    }

    fn must<T>(value: Option<T>, what: &str) -> T {
        match value {
            Some(found) => found,
            None => panic!("missing: {what}"),
        }
    }

    fn lib(n: usize) -> Vec<&'static str> {
        vec![X; n]
    }

    fn draws_by(events: &[GameEvent], player: PlayerId) -> usize {
        events
            .iter()
            .filter(|event| matches!(event, GameEvent::Drawn { player: p, .. } if *p == player))
            .count()
    }

    fn offered_modes(s: &Scenario) -> Vec<String> {
        let thrive = s.card(THRIVE).id.clone();
        let mut modes: Vec<String> = legal_actions(s.state(), P1)
            .iter()
            .map(|action| js(action))
            .filter(|action| action["type"] == json!("activate") && action["instanceId"] == json!(thrive))
            .flat_map(|action| {
                action["modes"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|mode| mode.as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .collect();
        modes.sort();
        modes
    }

    /// TS `standing(radiantFace = false, extra = {})`; `extra` holds the optional `health`, `library`
    /// (a count) and `hand`.
    fn standing(radiant_face: bool, extra: Value) -> Scenario {
        let hand = extra["hand"].as_array().cloned().unwrap_or_else(|| vec![json!(ANCHOR)]);
        let library = extra["library"].as_u64().map(|n| n as usize).unwrap_or(4);
        let health = extra["health"].as_i64().unwrap_or(20);
        scenario(json!({
            "p1": { "hand": hand, "backrow": [{ "def": THRIVE, "radiant": radiant_face }], "library": lib(library), "health": health },
            "p2": { "hand": [ANCHOR], "library": lib(4) }
        }))
    }

    /// TS `stepParam(s.card(ref), key, delta)`: the live card, found again by id.
    fn step(s: &mut Scenario, card: &str, key: &str, delta: i32) {
        let id = s.card(card).id.clone();
        step_param(must(find_instance_mut(s.state_mut(), &id), "the card to tune"), key, delta);
    }

    /// is a Field Spell with one once-a-turn ability of three modes, its three numbers, one script on both faces
    #[test]
    fn is_a_field_spell_with_one_once_a_turn_ability_of_three_modes_its_three_numbers_one_script_on_both_faces() {
        assert_eq!(def().id, THRIVE);
        assert_eq!(js(&def().type_), json!("Field Spell"));
        assert_eq!(
            js(&def().params),
            json!([
                { "key": "heal", "base": 3, "radiant": 6, "better": "up", "step": 1, "min": 1 },
                { "key": "draw", "base": 1, "radiant": 2, "better": "up", "step": 1, "min": 1 },
                { "key": "mana", "base": 1, "radiant": 2, "better": "up", "step": 1, "min": 1 }
            ])
        );
        let scripts = script();
        let abilities: Vec<Value> = scripts
            .base
            .activations
            .iter()
            .map(|ability| json!([js(&ability.uses), js(&ability.modes)]))
            .collect();
        assert_eq!(
            json!(abilities),
            json!([[1, [{ "kind": "mode", "options": ["heal", "draw", "mana"] }]]])
        );
        assert_eq!(scripts.radiant.activations.len(), scripts.base.activations.len());
        assert!(Arc::ptr_eq(&scripts.base.activations[0].run, &scripts.radiant.activations[0].run));
    }

    /// base
    mod base {
        use super::*;

        /// R384 legalActions offers the ability once per mode, the mode carried in the action
        #[test]
        fn r384_legalactions_offers_the_ability_once_per_mode_the_mode_carried_in_the_action() {
            let s = standing(false, json!({}));
            assert_eq!(offered_modes(&s), ["draw", "heal", "mana"]);
        }

        /// heal: your hero 3
        #[test]
        fn heal_your_hero_3() {
            let mut s = standing(false, json!({}));
            s.activate(THRIVE, json!({ "modes": ["heal"] }));
            s.expect_health(P1, 23);
            s.expect_events(json!(["activated", "healed"]));
        }

        /// R19 heal has no cap on a hero: at 30 it goes to 33
        #[test]
        fn r19_heal_has_no_cap_on_a_hero_at_30_it_goes_to_33() {
            let mut s = standing(false, json!({ "health": 30 }));
            s.activate(THRIVE, json!({ "modes": ["heal"] }));
            s.expect_health(P1, 33);
        }

        /// draw: 1
        #[test]
        fn draw_1() {
            let mut s = standing(false, json!({}));
            s.activate(THRIVE, json!({ "modes": ["draw"] }));
            assert_eq!(draws_by(s.last_events(), P1), 1);
        }

        /// §2.3 mana: 1 this turn, above the maximum of 4, and gone at the next refresh
        #[test]
        fn s2_3_mana_1_this_turn_above_the_maximum_of_4_and_gone_at_the_next_refresh() {
            let mut s = standing(false, json!({ "hand": [ANCHOR, FILLER] }));
            s.activate(THRIVE, json!({ "modes": ["mana"] }));
            s.expect_mana(P1, 5);

            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().active, P1);
            s.expect_mana(P1, 4);
        }

        /// R384 it is usable the turn it is played
        #[test]
        fn r384_it_is_usable_the_turn_it_is_played() {
            let mut s = scenario(json!({ "p1": { "hand": [THRIVE, ANCHOR], "health": 20 }, "p2": { "hand": [ANCHOR] } }));
            s.play(THRIVE, json!({}));

            s.activate(THRIVE, json!({ "modes": ["heal"] }));

            s.expect_health(P1, 23);
        }

        /// R384 a second activation that turn is refused, whatever its mode; the next turn it is usable again
        #[test]
        fn r384_a_second_activation_that_turn_is_refused_whatever_its_mode_the_next_turn_it_is_usable_again() {
            let mut s = standing(false, json!({ "hand": [ANCHOR, FILLER] }));
            s.activate(THRIVE, json!({ "modes": ["heal"] }));

            assert!(offered_modes(&s).is_empty());
            s.expect_refused_with(|s| s.activate(THRIVE, json!({ "modes": ["draw"] })), "already been used");

            s.end_turn();
            s.end_turn();
            assert_eq!(offered_modes(&s), ["draw", "heal", "mana"]);
        }

        /// R81 a mode it does not offer is refused
        #[test]
        fn r81_a_mode_it_does_not_offer_is_refused() {
            let mut s = standing(false, json!({}));
            s.expect_refused(|s| s.activate(THRIVE, json!({ "modes": ["steal"] })));
            s.expect_refused(|s| s.activate(THRIVE, json!({ "modes": [] })));
        }

        /// R384 not a play: nothing counts it
        #[test]
        fn r384_not_a_play_nothing_counts_it() {
            let mut s = standing(false, json!({}));
            let before = s.state().players.p1.turn_log.cards_played;

            s.activate(THRIVE, json!({ "modes": ["draw"] }));

            assert!(!s.events().iter().any(|event| matches!(event, GameEvent::CardPlayed { .. })));
            assert_eq!(s.state().players.p1.turn_log.cards_played, before);
        }

        /// §2.4 a full hand burns the draw
        #[test]
        fn s2_4_a_full_hand_burns_the_draw() {
            let mut s = standing(false, json!({ "hand": vec![ANCHOR; 10] }));
            s.activate(THRIVE, json!({ "modes": ["draw"] }));
            assert_eq!(
                s.last_events().iter().filter(|event| matches!(event, GameEvent::Burned { .. })).count(),
                1
            );
        }

        /// R386 an Upgrade heals 4, draws 2, gives 2 mana
        #[test]
        fn r386_an_upgrade_heals_4_draws_2_gives_2_mana() {
            let cases: [(&str, fn(&mut Scenario)); 3] = [
                ("heal", |s| {
                    s.expect_health(P1, 24);
                }),
                ("draw", |s| assert_eq!(draws_by(s.last_events(), P1), 2)),
                ("mana", |s| {
                    s.expect_mana(P1, 6);
                }),
            ];
            for (mode, check) in cases {
                let mut s = standing(false, json!({}));
                for key in ["heal", "draw", "mana"] {
                    step(&mut s, THRIVE, key, 1);
                }
                s.activate(THRIVE, json!({ "modes": [mode] }));
                check(&mut s);
            }
        }
    }

    /// radiant
    mod radiant {
        use super::*;

        /// heal 6, draw 2, or 2 mana
        #[test]
        fn heal_6_draw_2_or_2_mana() {
            let mut healed = standing(true, json!({}));
            healed.activate(THRIVE, json!({ "modes": ["heal"] }));
            healed.expect_health(P1, 26);

            let mut drew = standing(true, json!({}));
            drew.activate(THRIVE, json!({ "modes": ["draw"] }));
            assert_eq!(draws_by(drew.last_events(), P1), 2);

            let mut mana = standing(true, json!({}));
            mana.activate(THRIVE, json!({ "modes": ["mana"] }));
            mana.expect_mana(P1, 6);
        }

        /// R384 still once per turn
        #[test]
        fn r384_still_once_per_turn() {
            let mut s = standing(true, json!({}));
            s.activate(THRIVE, json!({ "modes": ["mana"] }));
            s.expect_refused_with(|s| s.activate(THRIVE, json!({ "modes": ["mana"] })), "already been used");
        }

        /// R386 a Degrade heals 5
        #[test]
        fn r386_a_degrade_heals_5() {
            let mut s = standing(true, json!({}));
            step(&mut s, THRIVE, "heal", -1);
            s.activate(THRIVE, json!({ "modes": ["heal"] }));
            s.expect_health(P1, 25);
        }
    }
}
