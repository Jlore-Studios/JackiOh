//! #38 Quickstriker (SPEC §8.2): Field Spell, "Your cards gain 'Combo X: deal X damage to the enemy
//! hero', X = cards you played earlier this turn"; radiant "… deal 2X damage …" (R275, R281).
//!
//! ROUTE: §10.5 step 5 — "Resolve Combo checks, Quickstriker, /fullsend's Combo draw, then the card's
//! own Cry or spell script". Quickstriker is one of the two Combo abilities another permanent grants
//! everything its controller plays, so the play pipeline resolves it (`playSteps.quickstrikerCombo`)
//! for every play and every cast (R70), before the played card's own text, reading this card's
//! static flag off the field. X is `turnLog.cardsPlayed` before the card being played (§8.2's Engine
//! cell), which the pipeline reads at that moment: 0 for the first play of a turn, 1 for the second.
//!
//! It used to be a trigger on `cardPlayed`, which read the count whenever the event was dispatched.
//! A play dispatches at step 4, but a cast's events wait for the loop of the effect that cast it —
//! §2.4's draw — so a chain of two cast-on-draw cards read the count after the chain and dealt 1 and
//! 1 rather than 0 and 1. The flag also gives the card the property §8.2 asks for, "nothing when not
//! on the field", for free: the pipeline reads permanents on the field and nothing else.
//!
//! "Your cards" is the controller's own plays (the pipeline reads the playing player's side), and
//! R119 excludes the play that puts this Field Spell onto the field: the pipeline never counts the
//! card being played as one of its own Quickstrikers.
//!
//! THE RADIANT FACE (R281). Both faces carry the same flag, one grant each; the multiple of X a
//! grant deals is the granting card's face's, `QUICKSTRIKER_COMBO_MULTIPLE` in the engine's config
//! (1, radiant 2), which the pipeline picks off the instance as #84's Armor is picked. 2X is ONE hit,
//! so Armor and the Anti-oneshot cap apply to it once, and a base and a Radiant Quickstriker together
//! deal X and then 2X.
//!
//! THE PREVIEW (R280). "X = cards you played earlier this turn {n}": the X the next card its
//! controller plays would count, which is every play so far this turn (`cardsPlayedThisTurn`, the
//! count `playedEarlier` gives a card still in hand, and the one the pipeline reads at step 5). It
//! reads the controller's plays this turn, which are public, and the same X shows on both faces.
//!
//! THE GLOW (R662). The condition this card prints is its grant's, so the cards that glow are the
//! ones in its controller's hand: while a Quickstriker acts for them and they have played a card this
//! turn, the next play takes the Combo branch (X ≥ 1), and `condition.ts` lights every hand card
//! (`query.grantedComboLive`, the facts `playSteps.quickstrikerCombo` reads). The card itself declares
//! no `conditionMet`: it prints no condition of its own, and its own play never answers its grant (R119).

use jackioh_engine::prelude::*;
use jackioh_engine::query::cards_played_this_turn;

pub const ID: &str = "core-038";

/// R280: the part of the text the value belongs to, on both faces.
const X_LABEL: &str = "X = cards you played earlier this turn";

/// TS `const preview: Script["preview"]`.
fn preview() -> PreviewHook {
    condition_hook(|ctx| {
        vec![PreviewValue {
            label: X_LABEL.to_string(),
            value: cards_played_this_turn(ctx.state, ctx.controller),
            display: None,
            ids: None,
        }]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            quickstriker: Some(FlagOrCount::Flag(true)),
            ..StaticFlags::default()
        }),
        preview: Some(preview()),
        ..Script::default()
    };
    // R281: the same grant and the same X, so the same script; the Radiant face's 2X is the multiple the
    // engine reads off the instance's face (`QUICKSTRIKER_COMBO_MULTIPLE`), as it reads #84's Armor.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// #38 Quickstriker (SPEC §8.2, BUILD M4-T4): "First play deals 0, second 1, third 2 to the enemy
// hero; nothing when not on the field". The Radiant face (R275) grants "Combo X: deal 2X damage to
// the enemy hero", and R281 makes 2X one hit: Armor and the Anti-oneshot cap apply to it once, and a
// base and a Radiant Quickstriker together deal X and then 2X.
// The X its next play would count, its R280 `preview`, is proved in test/preview.test.ts.
//
// R662's yellow glow: the condition is the grant's, so while it acts for its controller and they have
// played a card this turn, their hand cards glow (`condition.ts`), both faces, at the end of this file.
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    const QUICKSTRIKER: &str = "core-038";
    /// Three plays whose own texts never touch a hero's health, so the damage read is Quickstriker's.
    const RAPID_REPLENISH: &str = "core-010"; // 0-cost Spell; Combo 3, so nothing at one play
    const TEMPO_TIMMY: &str = "core-011"; // 1-cost Unit
    const BIG_D_FENDER: &str = "core-001"; // 2-cost Unit
    const SPARE: &str = "core-005";

    const POINTMASTER: &str = "core-020"; // 2-cost Unit, no text
    const ANTI_ONESHOT: &str = "core-073"; // Field Spell: p2's hero takes at most 5 in one instance
    const GOING_LONG: &str = "core-084"; // Field Spell: p2's hero has Armor 2 (paid 2)

    const HERO: i32 = 30;

    /// The harness's import-time `registerAll()`: the engine's testkit cannot name the cards crate.
    fn scn(opts: Value) -> Scenario {
        crate::register_all();
        scenario(opts)
    }

    /// The amounts of every `damage` event on p2's hero so far, in order: one entry per hit.
    fn hits_on_p2(s: &Scenario) -> Vec<i32> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { target_id, amount, .. } if target_id == "hero-p2" => Some(*amount),
                _ => None,
            })
            .collect()
    }

    /// `_glow.ts`'s `glows` (a private copy, fullsend rule 5): `true` when the view carries the key
    /// (which must then be exactly `true`), `false` when absent.
    fn glows(card: Option<&Value>) -> bool {
        let card = card.expect("no card at that place in the view");
        match card.get("conditionActive") {
            None => false,
            Some(value) => {
                assert_eq!(value, &json!(true));
                true
            }
        }
    }

    /// `_glow.ts`'s `handGlows(s, instanceId)`: does this hand card glow in p1's own view right now?
    fn hand_glows(s: &Scenario, instance_id: &str) -> bool {
        let view = serde_json::to_value(s.view("p1")).expect("a view is JSON");
        let hand = view["you"]["hand"]
            .as_array()
            .expect("the viewer's own hand must travel in full (§10.8)");
        glows(hand.iter().find(|card| card["instanceId"] == instance_id))
    }

    mod quickstriker {
        use super::*;

        #[test]
        fn is_sec8_2s_38_a_3_cost_field_spell_whose_radiant_face_deals_2x_r275() {
            crate::register_all();
            let def = crate::card_def(ID);
            assert_eq!(def.index, "38");
            assert_eq!(def.type_, CardType::FieldSpell);
            assert_eq!(def.cost, CardCost::Fixed(3));
            assert!(def.base.text.contains("\"Combo X: Deal X damage to the enemy hero.\""));
            assert!(def.radiant.text.contains("\"Combo X: Deal 2X damage to the enemy hero.\""));
        }

        #[test]
        fn r281_both_faces_carry_the_one_grant_the_multiple_of_x_is_the_granting_faces_read_by_the_engine() {
            // §10.5 step 5 resolves it for every play and cast, off this flag (R70); the pipeline picks the
            // multiple, 1 or 2, off the Quickstriker's own face (`QUICKSTRIKER_COMBO_MULTIPLE`).
            let scripts = script();
            // TS `expect(radiant).toBe(base)`: the same script, so the same flag and the same hook.
            assert_eq!(scripts.radiant.static_flags, scripts.base.static_flags);
            assert!(std::sync::Arc::ptr_eq(
                scripts.base.preview.as_ref().expect("the base face has a preview"),
                scripts.radiant.preview.as_ref().expect("the radiant face has a preview"),
            ));
            assert_eq!(
                serde_json::to_value(&scripts.base.static_flags).expect("flags are JSON"),
                json!({ "quickstriker": true })
            );
        }

        #[test]
        fn base_deals_0_on_the_first_play_1_on_the_second_and_2_on_the_third_combo_x() {
            let mut s = scn(json!({
                "p1": {
                    "backrow": [QUICKSTRIKER],
                    "hand": [RAPID_REPLENISH, TEMPO_TIMMY, BIG_D_FENDER],
                    "library": [SPARE, SPARE, SPARE, SPARE],
                },
                "p2": { "hand": [SPARE], "library": [SPARE] },
            }));

            // "X = cards you played EARLIER this turn", so the first play of the turn is X = 0.
            s.play(RAPID_REPLENISH, json!({})).expect_health("p2", HERO);
            s.play(TEMPO_TIMMY, json!({})).expect_health("p2", HERO - 1);
            s.play(BIG_D_FENDER, json!({})).expect_health("p2", HERO - 1 - 2);
        }

        #[test]
        fn base_counts_each_turn_on_its_own_the_turn_log_resets_at_the_start_of_a_turn() {
            let mut s = scn(json!({
                "p1": {
                    "backrow": [QUICKSTRIKER],
                    "hand": [RAPID_REPLENISH, TEMPO_TIMMY, BIG_D_FENDER],
                    "library": [SPARE, SPARE, SPARE],
                },
                "p2": { "hand": [SPARE, SPARE], "library": [SPARE, SPARE, SPARE] },
            }));

            s.play(RAPID_REPLENISH, json!({}))
                .play(TEMPO_TIMMY, json!({}))
                .expect_health("p2", HERO - 1);

            // Round the turn back to p1. `startTurn` clears `turnLog` (§2.2), so the count starts over.
            s.end_turn().end_turn().play(BIG_D_FENDER, json!({}));

            s.expect_health("p2", HERO - 1);
        }

        #[test]
        fn base_fires_on_your_plays_only_never_on_the_opponents() {
            let mut s = scn(json!({
                "p1": { "backrow": [QUICKSTRIKER], "hand": [SPARE], "library": [SPARE, SPARE] },
                "p2": { "hand": [RAPID_REPLENISH, TEMPO_TIMMY, BIG_D_FENDER], "library": [SPARE, SPARE] },
                "active": "p2",
            }));

            s.play(RAPID_REPLENISH, json!({}))
                .play(TEMPO_TIMMY, json!({}))
                .play(BIG_D_FENDER, json!({}));

            // "Your cards gain …": the controller of Quickstriker is p1, so p2's plays do nothing.
            s.expect_health("p1", HERO).expect_health("p2", HERO);
        }

        #[test]
        fn does_nothing_when_quickstriker_is_not_on_the_field() {
            let mut s = scn(json!({
                "p1": { "hand": [QUICKSTRIKER, RAPID_REPLENISH, TEMPO_TIMMY], "library": [SPARE, SPARE] },
                "p2": { "hand": [SPARE], "library": [SPARE] },
            }));

            // Quickstriker stays in hand: §10.5 step 5 reads the permanents on the field alone.
            s.play(RAPID_REPLENISH, json!({})).play(TEMPO_TIMMY, json!({}));

            s.expect_health("p2", HERO);
        }

        #[test]
        fn r119_does_not_answer_its_own_arrival_the_play_that_puts_it_on_the_field_deals_nothing() {
            let mut s = scn(json!({
                "p1": {
                    "hand": [RAPID_REPLENISH, QUICKSTRIKER, TEMPO_TIMMY],
                    "library": [SPARE, SPARE, SPARE],
                },
                "p2": { "hand": [SPARE], "library": [SPARE] },
            }));

            // §10.5 step 4 places the card and counts the play before step 5 resolves the granted Combos,
            // so Quickstriker is already on the field for its OWN play. R119: it does not answer its own
            // arrival, so this second play of the turn deals 0 rather than 1.
            s.play(RAPID_REPLENISH, json!({}))
                .play(QUICKSTRIKER, json!({}))
                .expect_health("p2", HERO);

            // The arrival still COUNTS as a card played earlier, so the next play deals 2 (R119 excludes
            // the arriving card from answering, not from the count §6.2 reads).
            s.play(TEMPO_TIMMY, json!({})).expect_health("p2", HERO - 2);
        }

        #[test]
        fn r119_holds_for_the_radiant_face_too_it_is_the_same_flag_so_the_same_arrival_is_silent() {
            let mut s = scn(json!({
                "p1": {
                    "hand": [RAPID_REPLENISH, { "def": QUICKSTRIKER, "radiant": true }, TEMPO_TIMMY],
                    "library": [SPARE, SPARE, SPARE],
                },
                "p2": { "hand": [SPARE], "library": [SPARE] },
            }));

            s.play(RAPID_REPLENISH, json!({}))
                .play(QUICKSTRIKER, json!({}))
                .expect_health("p2", HERO);
            // X = 2 (Replenish and the Quickstriker itself), so the Radiant grant deals 2X = 4.
            s.play(TEMPO_TIMMY, json!({})).expect_health("p2", HERO - 4);
        }

        #[test]
        fn r281_the_base_face_is_unchanged_x_one_hit_per_play() {
            let mut s = scn(json!({
                "p1": {
                    "backrow": [QUICKSTRIKER],
                    "hand": [RAPID_REPLENISH, TEMPO_TIMMY, BIG_D_FENDER],
                    "library": [SPARE, SPARE, SPARE, SPARE],
                },
                "p2": { "hand": [SPARE], "library": [SPARE] },
            }));

            s.play(RAPID_REPLENISH, json!({}))
                .play(TEMPO_TIMMY, json!({}))
                .play(BIG_D_FENDER, json!({}));

            assert_eq!(hits_on_p2(&s), vec![1, 2]);
            s.expect_health("p2", HERO - 3);
        }

        #[test]
        fn r281_the_radiant_face_deals_2x_as_one_hit_per_play_0_then_2_then_4() {
            let mut s = scn(json!({
                "p1": {
                    "backrow": [{ "def": QUICKSTRIKER, "radiant": true }],
                    "hand": [RAPID_REPLENISH, TEMPO_TIMMY, BIG_D_FENDER],
                    "library": [SPARE, SPARE, SPARE, SPARE],
                },
                "p2": { "hand": [SPARE], "library": [SPARE] },
            }));

            s.play(RAPID_REPLENISH, json!({})).expect_health("p2", HERO);
            s.play(TEMPO_TIMMY, json!({})).expect_health("p2", HERO - 2);
            s.play(BIG_D_FENDER, json!({})).expect_health("p2", HERO - 2 - 4);
            // One damage event per play, never X hits of 2 or two hits of X.
            assert_eq!(hits_on_p2(&s), vec![2, 4]);
        }

        #[test]
        fn r281_going_longs_hero_armor_2_comes_off_the_radiant_2x_once_x_2_deals_4_minus_2_is_2_not_2_times_2_minus_2_is_0()
        {
            let mut s = scn(json!({
                "p1": {
                    "backrow": [{ "def": QUICKSTRIKER, "radiant": true }],
                    "hand": [RAPID_REPLENISH, TEMPO_TIMMY, BIG_D_FENDER],
                    "library": [SPARE, SPARE, SPARE, SPARE],
                },
                "p2": { "backrow": [GOING_LONG], "hand": [SPARE], "library": [SPARE] },
            }));

            s.play(RAPID_REPLENISH, json!({}));
            // X = 1: 2X = 2, and Armor 2 takes the whole hit (R63: no damage event at all).
            s.play(TEMPO_TIMMY, json!({})).expect_health("p2", HERO);
            // X = 2: 2X = 4 as one instance, less Armor 2 once.
            s.play(BIG_D_FENDER, json!({})).expect_health("p2", HERO - 2);
            assert_eq!(hits_on_p2(&s), vec![2]);
        }

        #[test]
        fn r281_anti_oneshot_armors_cap_clamps_the_radiant_2x_once_x_3_deals_6_capped_to_5_not_3_plus_3() {
            let mut s = scn(json!({
                "p1": {
                    "backrow": [{ "def": QUICKSTRIKER, "radiant": true }],
                    "hand": [RAPID_REPLENISH, TEMPO_TIMMY, BIG_D_FENDER, POINTMASTER],
                    "library": [SPARE, SPARE, SPARE, SPARE],
                    "mana": 10,
                },
                "p2": { "backrow": [ANTI_ONESHOT], "hand": [SPARE], "library": [SPARE] },
            }));

            s.play(RAPID_REPLENISH, json!({}))
                .play(TEMPO_TIMMY, json!({}))
                .play(BIG_D_FENDER, json!({}))
                .play(POINTMASTER, json!({}));

            // X = 1, 2 and 3: 2, 4 and 6, and the cap (5 on the base face of #73) takes the last down to 5.
            assert_eq!(hits_on_p2(&s), vec![2, 4, 5]);
            s.expect_health("p2", HERO - 11);
        }

        #[test]
        fn r281_a_base_and_a_radiant_quickstriker_together_deal_x_and_then_2x_each_its_own_hit() {
            let mut s = scn(json!({
                "p1": {
                    "backrow": [QUICKSTRIKER, { "def": QUICKSTRIKER, "radiant": true }],
                    "hand": [RAPID_REPLENISH, TEMPO_TIMMY, BIG_D_FENDER],
                    "library": [SPARE, SPARE, SPARE, SPARE],
                },
                "p2": { "hand": [SPARE], "library": [SPARE] },
            }));

            s.play(RAPID_REPLENISH, json!({}))
                .play(TEMPO_TIMMY, json!({}))
                .play(BIG_D_FENDER, json!({}));

            // Board order: the base one in lane 1 grants X, the Radiant one in lane 2 grants 2X.
            assert_eq!(hits_on_p2(&s), vec![1, 2, 2, 4]);
            s.expect_health("p2", HERO - 9);
        }

        #[test]
        fn r281_a_radiant_card_fused_from_two_quickstrikers_r102_keeps_both_grants_each_2x() {
            let mut s = scn(json!({
                "p1": {
                    "backrow": [{ "def": QUICKSTRIKER, "radiant": true }],
                    "hand": [QUICKSTRIKER, RAPID_REPLENISH, TEMPO_TIMMY],
                    "library": [SPARE, SPARE, SPARE, SPARE],
                },
                "p2": { "hand": [SPARE], "library": [SPARE] },
            }));
            let kept = s.backrow("p1", 1);
            let ingredient = s.hand("p1").into_iter().find(|card| card.def_id == QUICKSTRIKER);
            let (Some(kept), Some(ingredient)) = (kept, ingredient) else {
                panic!("the two Quickstrikers are not in place");
            };
            let seed = s.state().seed.clone();
            let cursor = s.state().rng_cursor;
            let mut rng = Rng::new(&seed, cursor);
            let mut events: Vec<GameEvent> = Vec::new();
            // #85's path: a Quickstriker fused onto the Radiant one on the field, which the Fuse keeps.
            let fused = {
                let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
                subsystems::fuse::fuse(
                    &mut sink,
                    subsystems::fuse::FuseArgs {
                        ingredients: vec![ingredient],
                        target: Some(kept.clone()),
                        ..Default::default()
                    },
                )
            };
            assert_eq!(fused.map(|card| card.id), Some(kept.id.clone()));

            s.play(RAPID_REPLENISH, json!({})).play(TEMPO_TIMMY, json!({}));

            // X = 1, and the kept card runs its Radiant face: both texts' grants, 2X each.
            assert_eq!(hits_on_p2(&s), vec![2, 2]);
        }
    }

    mod quickstriker_lights_its_controllers_hand_once_its_combo_would_hit_r662 {
        use super::*;

        /// TS's `for (const radiant of [false, true])` first `it`, run for one face.
        fn after_a_play_this_turn_the_next_card_glows(radiant: bool) {
            let face = if radiant { "radiant" } else { "base" };
            let mut s = scn(json!({
                "seed": format!("r662-038-{face}-on"),
                "p1": {
                    "backrow": [{ "def": QUICKSTRIKER, "radiant": radiant }],
                    "hand": [RAPID_REPLENISH, TEMPO_TIMMY, SPARE],
                    "library": [SPARE, SPARE],
                },
                "p2": { "hand": [SPARE], "library": [SPARE] },
            }));
            // X = 0 for the first play of the turn: nothing glows yet.
            let timmy = s.card(TEMPO_TIMMY).id.clone();
            assert!(!hand_glows(&s, &timmy));

            s.play(RAPID_REPLENISH, json!({}));
            let timmy = s.card(TEMPO_TIMMY).id.clone();
            assert!(hand_glows(&s, &timmy));
            s.play(TEMPO_TIMMY, json!({}));
            assert_eq!(hits_on_p2(&s), vec![if radiant { 2 } else { 1 }]);
        }

        /// TS's `for (const radiant of [false, true])` second `it`, run for one face.
        fn the_quickstriker_in_hand_does_not_light_itself_or_the_hand(radiant: bool) {
            let face = if radiant { "radiant" } else { "base" };
            let mut s = scn(json!({
                "seed": format!("r662-038-{face}-hand"),
                "p1": {
                    "hand": [{ "def": QUICKSTRIKER, "radiant": radiant }, RAPID_REPLENISH, TEMPO_TIMMY],
                    "library": [SPARE],
                },
            }));
            s.play(RAPID_REPLENISH, json!({}));
            let quickstriker = s.card(QUICKSTRIKER).id.clone();
            assert!(!hand_glows(&s, &quickstriker));
            let timmy = s.card(TEMPO_TIMMY).id.clone();
            assert!(!hand_glows(&s, &timmy));
        }

        #[test]
        fn r662_base_after_a_play_this_turn_the_next_card_glows_and_playing_it_deals_the_combo_damage() {
            after_a_play_this_turn_the_next_card_glows(false);
        }

        #[test]
        fn r662_base_the_quickstriker_in_hand_does_not_light_itself_or_the_hand_r119() {
            the_quickstriker_in_hand_does_not_light_itself_or_the_hand(false);
        }

        #[test]
        fn r662_radiant_after_a_play_this_turn_the_next_card_glows_and_playing_it_deals_the_combo_damage() {
            after_a_play_this_turn_the_next_card_glows(true);
        }

        #[test]
        fn r662_radiant_the_quickstriker_in_hand_does_not_light_itself_or_the_hand_r119() {
            the_quickstriker_in_hand_does_not_light_itself_or_the_hand(true);
        }

        #[test]
        fn r662_the_opponents_quickstriker_lights_nothing_in_your_hand() {
            let mut s = scn(json!({
                "seed": "r662-038-theirs",
                "p1": { "hand": [RAPID_REPLENISH, TEMPO_TIMMY], "library": [SPARE] },
                "p2": { "backrow": [QUICKSTRIKER] },
            }));
            s.play(RAPID_REPLENISH, json!({}));
            let timmy = s.card(TEMPO_TIMMY).id.clone();
            assert!(!hand_glows(&s, &timmy));
        }
    }
}
