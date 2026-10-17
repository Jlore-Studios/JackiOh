//! C #81 The Power to Thrive (SPEC §8.6 row 81). (2) Field Spell, Rare.
//!   Base:    "Activate: Choose one: Heal your hero {heal}; draw {draw}; or reduce the Cost of a card in
//!            your hand by ({discount})." — 3, 1, 1
//!   Radiant: the same text — 6, 2, 2
//!   Engine:  "Activate (§6.2, R384), once per turn, the mode declared in the action (R81). "Active" is
//!            Activate. Tunes: heal 3 ↑; draw 1 ↑; discount 1 ↑."
//!
//! One ability, once per turn, its mode carried in the `activate` action. Heal has no cap on a hero (R19).
//! The third mode gained temporary mana until patch v0.3.10 (#831); it now reduces the Cost of a card in
//! your hand, a `costMod` (R65, R78) on the card its controller picks as the activation resolves, and
//! with an empty hand it does nothing (R831009). The three numbers are declared (R386).

use jackioh_engine::effects::{choose_from_hand, draw, heal, set_cost_mod};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-081";

/// One ability, once per turn, its mode carried in the `activate` action (R81). The third mode reduces
/// the Cost of a card in your hand (patch v0.3.10, #831; it gained mana before): the card is chosen as
/// the activation resolves, a prompt the `resume` step answers (R831009).
const MODES: [&str; 3] = ["heal", "draw", "reduce"];

/// The resume step of the third mode's hand pick.
const REDUCE_STEP: &str = "reduce";

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
                if mode.as_deref() == Some("reduce") {
                    vec![choose_from_hand(json_as(json!({
                        "step": REDUCE_STEP,
                        "count": 1,
                        "prompt": "The Power to Thrive: choose a card",
                    })))]
                } else {
                    vec![]
                }
            }),
        }],
        resume: IndexMap::from([(
            REDUCE_STEP,
            hook(|ctx| {
                vec![set_cost_mod(json_as(json!({
                    "target": { "of": "chosen" },
                    "amount": -param(&*ctx, "discount"),
                    "inHandOnly": true,
                })))]
            }),
        )]),
        ..Script::default()
    };

    // The same script: the Radiant face's 6, 2 and 2 are its declared numbers.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C #81 The Power to Thrive — SPEC §8.6 row 81, BUILD M9 Classic row C 81: "Activate, once per turn
// (R384), the mode carried in the `activate` action: heal your hero 3 (no cap, R19), draw 1, or a card of
// your hand costs (1) less (patch v0.3.10); usable the turn it is played; a second activation that turn is
// refused; not a play; radiant: heal 6, draw 2, or (2) less; its tuned numbers (heal, draw, discount) read
// through `param()` (R386)".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const THRIVE: &str = "classic-081";
    const FILLER: &str = "core-005"; // (1) Spell (§2.5).
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).
    const X: &str = "core-020"; // library filler.

    use crate::scenario;

    use crate::js;

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
            .map(js)
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

    /// base
    mod base {
        use super::*;

        /// R384 legalActions offers the ability once per mode, the mode carried in the action
        #[test]
        fn r384_legalactions_offers_the_ability_once_per_mode_the_mode_carried_in_the_action() {
            let s = standing(false, json!({}));
            assert_eq!(offered_modes(&s), ["draw", "heal", "reduce"]);
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

        /// R831009 reduce: the card is chosen from your hand as it resolves, and costs (1) less
        #[test]
        fn r831009_reduce_asks_for_a_card_of_your_hand_as_it_resolves_and_it_costs_1_less() {
            let mut s = standing(false, json!({ "hand": [ANCHOR, FILLER] }));
            let filler = s.card(FILLER).id.clone();
            s.activate(THRIVE, json!({ "modes": ["reduce"] }));
            let pending = s.state().pending.clone().expect("a hand prompt");
            assert_eq!(pending.player_id, P1);
            assert_eq!(pending.options.len(), 2);

            s.answer(json!(filler));

            assert!(s.state().pending.is_none());
            assert_eq!(s.card(&filler).cost_mod, -1);
            assert_eq!(s.card(ANCHOR).cost_mod, 0);
            // No mana gained, as the old third mode did.
            s.expect_mana(P1, 4);
        }

        /// R831009 reduce with an empty hand does nothing and asks nothing
        #[test]
        fn r831009_reduce_with_an_empty_hand_does_nothing() {
            let mut s = standing(false, json!({ "hand": [] }));
            s.activate(THRIVE, json!({ "modes": ["reduce"] }));
            assert!(s.state().pending.is_none());
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
            assert_eq!(offered_modes(&s), ["draw", "heal", "reduce"]);
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

        /// R386 an Upgrade heals 4, draws 2, takes (2) off
        #[test]
        fn r386_an_upgrade_heals_4_draws_2_takes_2_off() {
            type Check = fn(&mut Scenario);
            let cases: [(&str, Check); 3] = [
                ("heal", |s| {
                    s.expect_health(P1, 24);
                }),
                ("draw", |s| assert_eq!(draws_by(s.last_events(), P1), 2)),
                ("reduce", |s| {
                    let anchor = s.card(ANCHOR).id.clone();
                    s.answer(json!(anchor));
                    assert_eq!(s.card(&anchor).cost_mod, -2);
                }),
            ];
            for (mode, check) in cases {
                let mut s = standing(false, json!({}));
                for key in ["heal", "draw", "discount"] {
                    step_param(s.card_mut(THRIVE), key, 1);
                }
                s.activate(THRIVE, json!({ "modes": [mode] }));
                check(&mut s);
            }
        }
    }

    /// radiant
    mod radiant {
        use super::*;

        /// heal 6, draw 2, or (2) off a card in your hand
        #[test]
        fn heal_6_draw_2_or_2_off_a_card_in_your_hand() {
            let mut healed = standing(true, json!({}));
            healed.activate(THRIVE, json!({ "modes": ["heal"] }));
            healed.expect_health(P1, 26);

            let mut drew = standing(true, json!({}));
            drew.activate(THRIVE, json!({ "modes": ["draw"] }));
            assert_eq!(draws_by(drew.last_events(), P1), 2);

            let mut reduced = standing(true, json!({}));
            reduced.activate(THRIVE, json!({ "modes": ["reduce"] }));
            let anchor = reduced.card(ANCHOR).id.clone();
            reduced.answer(json!(anchor));
            assert_eq!(reduced.card(&anchor).cost_mod, -2);
        }

        /// R384 still once per turn
        #[test]
        fn r384_still_once_per_turn() {
            let mut s = standing(true, json!({}));
            s.activate(THRIVE, json!({ "modes": ["draw"] }));
            s.expect_refused_with(|s| s.activate(THRIVE, json!({ "modes": ["heal"] })), "already been used");
        }

        /// R386 a Degrade heals 5
        #[test]
        fn r386_a_degrade_heals_5() {
            let mut s = standing(true, json!({}));
            step_param(s.card_mut(THRIVE), "heal", -1);
            s.activate(THRIVE, json!({ "modes": ["heal"] }));
            s.expect_health(P1, 25);
        }
    }
}
