//! #78 /fullsend (SPEC §8.3, R62, R65, R364, §2.2, §2.3, §10.5 step 5, §10.1).
//!
//! Base: "Refresh 3 mana. Your cards cost (1) less this turn. End of turn: Exile your hand." Radiant adds
//! "gain 'Combo: Draw 1'" to the discount. §8's Engine cell: "Turn-scoped player modifiers plus an
//! end-of-turn delayed exile": `refreshMana` (§6.3 Refresh, R364: spent mana back, never past max); a
//! `costDiscount` for this turn, flat before Curvature (R65: "your CARDS", so no `onlyType`, and an X-cost
//! card still costs exactly X); on Radiant a PLAYER-scoped `comboDraw` rider read once per card played
//! (§10.5 step 5); and a delayed effect at the controller's end phase that exiles the hand, after the
//! end-of-turn trap window (R68 order) and before cleanup, so the modifiers are still live (R62). Its
//! `resume` step is the one registration (R126); `radiant` persists in every zone (R78), so the Radiant
//! step runs even with no instance left (R127). R662: the Radiant rider lights every hand card once a
//! card was played this turn; the base face grants a discount, not a condition (cost is on the faces, R280).

use jackioh_engine::effects::{add_player_modifier, delay, exile_hand, refresh_mana};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-078";

// The numbers are declared (R386): `refresh`, §6.3 Refresh's 3 (R364: spent mana given back, never
// past max); `discount`, "Your cards cost (1) less this turn" on both faces; and `comboDraw`, the
// Radiant face's "gain 'Combo: Draw 1'" — one card per play.

/// The step name the delayed effect carries; it labels the pause.
const EXILE_STEP: &str = "exileHand";

/// R62: the end-of-turn delayed effect. It runs after the trap window and before cleanup, and it
/// exiles whatever the hand holds then — including cards drawn by the Combo rider this turn.
fn exile_the_hand(_ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    vec![exile_hand(json_as(json!({ "player": "self" })))]
}

/// The Radiant face's rider: each card played this turn draws 1 (§10.5 step 5).
fn combo_draw(ctx: &EffectContext<'_>) -> Effect {
    add_player_modifier(json_as(json!({
        "player": "self",
        "mod": {
            "kind": "comboDraw",
            "amount": param(ctx, "comboDraw"),
            "expiry": { "until": "thisTurn", "turn": ctx.state.turn }
        }
    })))
}

/// The two faces differ only in whether the turn's cards gain the Combo draw.
fn fullsend(with_combo_draw: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut effects = vec![
                refresh_mana(json_as(json!({ "amount": param(&*ctx, "refresh") }))),
                add_player_modifier(json_as(json!({
                    "player": "self",
                    // "your cards", so no `onlyType`; R65 applies it as a flat discount before Curvature.
                    "mod": {
                        "kind": "costDiscount",
                        "amount": param(&*ctx, "discount"),
                        "expiry": { "until": "thisTurn", "turn": ctx.state.turn }
                    }
                }))),
            ];
            if with_combo_draw {
                effects.push(combo_draw(ctx));
            }
            // R62: `delay` takes a PlayerSpec, so "my own end of turn" is "self" (§6.3, §2.2).
            effects.push(delay(json_as(json!({
                "at": { "phase": "end", "player": "self" },
                "step": EXILE_STEP,
                "hook": RESUME_HOOK
            }))));
            effects
        })),
        // The one registration (R126): the step table the `delay` above names.
        resume: IndexMap::from([(EXILE_STEP, hook(exile_the_hand))]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: fullsend(false),
        radiant: fullsend(true),
    }
}

// #78 /fullsend — SPEC §8.3, R62, R65, R364, §2.3, §10.5 step 5.
//
// BUILD M4-T4: "Refresh 3 mana (R364); −1 cost this turn; hand exiled at end of turn; radiant also
// draws 1 for each card played this turn".
//
// The discount is read off the hand's own cost (`viewFor`, §10.8) and confirmed by paying; R65's X-cost
// clause by playing an X card for exactly X. R62's place for the exile is proved by the event order.
// R662's yellow glow (Radiant lights the hand, base lights nothing) is tested at the end of this file.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FULLSEND: &str = "core-078";

    const COST_0: &str = "core-010"; // Rapid Replenish — a discount floors at 0 (R65)
    const COST_3: &str = "core-070"; // Spiteful Stab (Flood costs (4) since patch v0.2.0)
    const COST_4: &str = "core-025"; // 4-mana 7/7
    const X_CARD: &str = "core-074"; // Adaptive UI, printed cost X

    /// R81: #74 Adaptive UI declares one target with its play, so the `play` action must carry it. The
    /// enemy hero is always a legal pick, and the X damage lands off to one side.
    fn at_enemy_hero() -> Value {
        json!([{ "pick": "hero", "player": "p2" }])
    }

    const LIBRARY: [&str; 4] = ["core-008", "core-008", "core-008", "core-008"];

    use crate::js;

    use crate::matches_object;

    /// The position, or -1 when there is none.
    fn index_of(found: Option<usize>) -> i64 {
        found.map_or(-1, |index| index as i64)
    }

    /// The cost a card in the viewer's own hand shows now (§10.8, R65).
    fn hand_cost(s: &Scenario, def_id: &str) -> i32 {
        let HandView::Cards(hand) = s.view(P1).you.hand else {
            panic!("§10.8: the viewer's own hand is a list of cards");
        };
        match hand.iter().find(|entry| entry.def_id == def_id) {
            Some(card) => card.cost,
            None => panic!("{def_id} is not in p1's hand"),
        }
    }

    /// No `mods()` accessor, so the test reads `state` — B1.7 covers `src` only.
    fn mods_of(s: &Scenario, kind: &str) -> Vec<Value> {
        s.state()
            .players
            .p1
            .mods
            .iter()
            .map(js)
            .filter(|modifier| modifier["kind"] == kind)
            .collect()
    }

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": FULLSEND, "radiant": radiant }, COST_0, COST_3, COST_4, X_CARD],
                "library": LIBRARY
            },
            // R82: the opponent keeps something to do, so `endTurn()` does not cascade.
            "p2": { "hand": ["core-005"], "field": ["core-019"], "library": LIBRARY }
        }))
    }

    #[test]
    fn r386_an_upgrade_refreshes_4_and_a_degrade_2() {
        for (upgrade, refresh) in [(true, 4), (false, 2)] {
            crate::register_all();
            let mut s = board(false);
            let moved = if upgrade {
                crate::upgrade_number(&mut s, FULLSEND, "refresh")
            } else {
                crate::degrade_number(&mut s, FULLSEND, "refresh")
            };
            assert_eq!(moved, refresh);
            s.play(FULLSEND, json!({}));
            // 4 paid of 4, then the refresh, never past max (R364).
            s.expect_mana(P1, refresh);
        }
    }

    #[test]
    fn r386_an_upgrade_makes_the_discount_2_and_the_radiant_combo_draw_2_and_a_degrade_finds_both_at_1() {
        crate::register_all();
        let mut s = board(true);
        assert!(!crate::can_degrade_number(&s, FULLSEND, "discount"));
        assert!(!crate::can_degrade_number(&s, FULLSEND, "comboDraw"));
        assert_eq!(crate::upgrade_number(&mut s, FULLSEND, "discount"), 2);
        assert_eq!(crate::upgrade_number(&mut s, FULLSEND, "comboDraw"), 2);
        s.play(FULLSEND, json!({}));
        assert_eq!(hand_cost(&s, COST_4), 2);
        assert_eq!(mods_of(&s, "comboDraw")[0]["amount"], json!(2));
    }

    mod n78_fullsend_base {
        use super::*;

        #[test]
        fn r364_refreshes_3_mana_paying_4_of_4_leaves_0_and_3_come_back() {
            crate::register_all();
            let mut s = board(false);

            s.expect_mana(P1, 4);
            s.play(FULLSEND, json!({}));

            s.expect_mana(P1, 3);
            s.expect_events(json!(["cardPlayed", "manaChanged"]));
        }

        #[test]
        fn r364_a_refresh_never_goes_past_max_temporary_mana_above_it_is_not_topped_up() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [FULLSEND, COST_0], "library": LIBRARY, "mana": 6 },
                "p2": { "hand": ["core-005"], "field": ["core-019"], "library": LIBRARY }
            }));

            // 6 of max 4, pay 4: 2 left, and the Refresh stops at max 4 — a gain of 3 would have made 5.
            s.play(FULLSEND, json!({}));

            s.expect_mana(P1, 4);
        }

        #[test]
        fn r65_this_turn_your_cards_cost_1_less_flooring_at_0() {
            crate::register_all();
            let mut s = board(false);

            s.play(FULLSEND, json!({}));

            assert_eq!(hand_cost(&s, COST_4), 3);
            assert_eq!(hand_cost(&s, COST_3), 2);
            assert_eq!(hand_cost(&s, COST_0), 0);
            assert_eq!(mods_of(&s, "costDiscount").len(), 1);
            let discount = mods_of(&s, "costDiscount").into_iter().next().expect("a costDiscount modifier");
            assert!(
                matches_object(
                    &discount,
                    &json!({
                        "kind": "costDiscount",
                        "amount": 1,
                        "expiry": { "until": "thisTurn", "turn": 9 }
                    })
                ),
                "{discount}"
            );
        }

        #[test]
        fn the_discount_is_really_charged_a_cost_4_card_is_paid_at_3() {
            crate::register_all();
            let mut s = board(false);
            s.play(FULLSEND, json!({}));
            s.expect_mana(P1, 3);

            s.play(COST_4, json!({}));

            s.expect_mana(P1, 0);
        }

        #[test]
        fn r65_an_x_cost_card_costs_exactly_x_the_discount_does_not_cheapen_it() {
            crate::register_all();
            let mut s = board(false);
            s.play(FULLSEND, json!({}));
            s.expect_mana(P1, 3);

            s.play(X_CARD, json!({ "x": 2, "targets": at_enemy_hero() }));

            // Exactly 2, not 1: "costMod and discounts don't change it" (R65).
            s.expect_mana(P1, 1);
        }

        #[test]
        fn patch_v0_1_1_the_base_face_installs_no_combo_draw_1_so_a_play_afterwards_draws_nothing() {
            crate::register_all();
            let mut s = board(false);
            s.play(FULLSEND, json!({}));
            assert_eq!(mods_of(&s, "comboDraw").len(), 0);
            let library_before = s.pile(P1, "library").len();

            s.play(COST_0, json!({}));

            assert_eq!(s.pile(P1, "library").len(), library_before);
        }

        #[test]
        fn r62_the_hand_is_exiled_at_end_of_turn_before_cleanup() {
            crate::register_all();
            let mut s = board(false);
            s.play(FULLSEND, json!({}));
            let left: Vec<String> = s.pile(P1, "hand").iter().map(|card| card.def_id.clone()).collect();
            assert!(!left.is_empty());

            s.end_turn();

            assert_eq!(s.pile(P1, "hand").len(), 0);
            let mut exiled: Vec<String> = s.pile(P1, "exile").iter().map(|card| card.def_id.clone()).collect();
            exiled.sort();
            let mut expected = left.clone();
            expected.sort();
            assert_eq!(exiled, expected);

            // R62 puts the exile between the trap window and cleanup. `turnEnded` is emitted at the TOP of
            // the window, because its traps read it (#18 Bread and Butter answers `event.unspentMana`), so
            // the exile follows it and cleanup follows the exile: the `modifierChanged` that retires
            // /fullsend's own "this turn" discount.
            let types: Vec<GameEventType> = s.events().iter().map(GameEvent::event_type).collect();
            let window_opened = index_of(types.iter().position(|kind| *kind == GameEventType::TurnEnded));
            let last_exile = index_of(types.iter().rposition(|kind| *kind == GameEventType::Exiled));
            let cleanup_at = index_of(
                s.events()
                    .iter()
                    .position(|event| matches!(event, GameEvent::ModifierChanged { added: false, .. })),
            );

            assert!(window_opened >= 0);
            assert!(last_exile > window_opened);
            assert!(cleanup_at > last_exile);
            s.expect_events(json!(["cardPlayed", "turnEnded", "exiled", "turnStarted"]));
        }

        #[test]
        fn s2_2_the_this_turn_discount_is_gone_after_cleanup() {
            crate::register_all();
            let mut s = board(false);
            s.play(FULLSEND, json!({}));

            s.end_turn();

            assert_eq!(mods_of(&s, "costDiscount").len(), 0);
        }
    }

    /// R169, BUILD M5-T4 ("badge list equals the view's modifiers"): /fullsend's riders are badges, on
    /// both seats, from the moment the spell resolves until the cleanup that retires them.
    mod r169_n78_fullsend_visible_to_the_player_while_active_s10_8 {
        use super::*;

        #[test]
        fn shows_the_discount_as_a_badge_the_moment_the_spell_resolves() {
            crate::register_all();
            let mut s = board(false);

            s.play(FULLSEND, json!({}));

            let badges = s.view(P1).you.modifiers;
            assert_eq!(
                badges.iter().map(|modifier| modifier.label.as_str()).collect::<Vec<_>>(),
                vec!["Your cards cost 1 less"]
            );
            // The ids are the engine's, so a `modifierChanged` animation lands on the badge it names.
            assert_eq!(
                badges.iter().map(|modifier| modifier.id.clone()).collect::<Vec<_>>(),
                s.state().players.p1.mods.iter().map(|modifier| modifier.id.clone()).collect::<Vec<_>>()
            );
        }

        #[test]
        fn the_badges_go_at_cleanup_with_the_modifiers_they_stand_for_s2_2() {
            crate::register_all();
            let mut s = board(true);
            s.play(FULLSEND, json!({}));
            assert_eq!(s.view(P1).you.modifiers.len(), 2);

            s.end_turn();

            assert_eq!(s.view(P1).you.modifiers, Vec::<ModifierView>::new());
        }

        #[test]
        fn the_opponent_sees_them_too_playing_a_spell_is_public_s10_5_step_4() {
            crate::register_all();
            let mut s = board(true);

            s.play(FULLSEND, json!({}));

            assert_eq!(
                s.view(P2)
                    .opponent
                    .modifiers
                    .iter()
                    .map(|modifier| modifier.label.clone())
                    .collect::<Vec<_>>(),
                vec!["Your cards cost 1 less", "Your cards gain \"Combo: draw 1\""]
            );
        }
    }

    mod n78_fullsend_radiant {
        use super::*;

        #[test]
        fn r364_radiant_refreshes_3_mana_and_its_cards_still_cost_only_1_less_patch_v0_1_1() {
            crate::register_all();
            let mut s = board(true);

            s.play(FULLSEND, json!({}));

            s.expect_mana(P1, 3);
            assert_eq!(hand_cost(&s, COST_4), 3);
            assert_eq!(hand_cost(&s, COST_3), 2);
            assert_eq!(hand_cost(&s, COST_0), 0);
        }

        #[test]
        fn installs_the_combo_draw_1_rider_as_a_turn_scoped_player_modifier_s10_5_step_5() {
            crate::register_all();
            let mut s = board(true);

            s.play(FULLSEND, json!({}));

            assert_eq!(mods_of(&s, "comboDraw").len(), 1);
            let rider = mods_of(&s, "comboDraw").into_iter().next().expect("a comboDraw modifier");
            assert!(
                matches_object(
                    &rider,
                    &json!({
                        "kind": "comboDraw",
                        "amount": 1,
                        "expiry": { "until": "thisTurn", "turn": 9 }
                    })
                ),
                "{rider}"
            );
        }

        #[test]
        fn each_card_played_this_turn_draws_1_s10_5_step_5() {
            crate::register_all();
            let mut s = board(true);
            s.play(FULLSEND, json!({}));
            let library_before = s.pile(P1, "library").len();
            let hand_before = s.pile(P1, "hand").len();

            s.play(COST_0, json!({}));

            // One draw for the play: the library is one shorter, and the hand is the played card plus the
            // drawn one.
            assert_eq!(s.pile(P1, "library").len(), library_before - 1);
            assert_eq!(s.pile(P1, "hand").len(), hand_before - 1 + 1);
        }

        #[test]
        fn r65_radiant_does_not_cheapen_an_x_cost_card_either() {
            crate::register_all();
            let mut s = board(true);
            s.play(FULLSEND, json!({}));

            s.play(X_CARD, json!({ "x": 2, "targets": at_enemy_hero() }));

            s.expect_mana(P1, 1);
        }

        #[test]
        fn r62_radiant_still_exiles_the_hand_at_end_of_turn_and_both_riders_are_gone_after_cleanup() {
            crate::register_all();
            let mut s = board(true);
            s.play(FULLSEND, json!({}));
            let left: Vec<String> = s.pile(P1, "hand").iter().map(|card| card.def_id.clone()).collect();

            s.end_turn();

            assert_eq!(s.pile(P1, "hand").len(), 0);
            let mut exiled: Vec<String> = s.pile(P1, "exile").iter().map(|card| card.def_id.clone()).collect();
            exiled.sort();
            let mut expected = left.clone();
            expected.sort();
            assert_eq!(exiled, expected);
            assert_eq!(mods_of(&s, "costDiscount").len(), 0);
            assert_eq!(mods_of(&s, "comboDraw").len(), 0);
        }
    }

    mod r662_n78_fullsend_s_radiant_combo_draw_lights_the_hand {
        use super::*;

        #[test]
        fn r662_radiant_once_it_resolves_the_hand_glows_and_the_next_play_draws_1() {
            crate::register_all();
            let mut s = board(true);
            let cheap = s.card(COST_0).id.clone();
            assert!(!hand_glows(&s, &cheap, P1));

            s.play(FULLSEND, json!({}));
            let cheap = s.card(COST_0).id.clone();
            assert!(hand_glows(&s, &cheap, P1));
            let before = s.hand(P1).len();
            s.play(COST_0, json!({}));
            // Rapid Replenish leaves the hand and the Combo draw brings one in.
            assert_eq!(s.hand(P1).len(), before);
        }

        #[test]
        fn r662_base_the_discount_is_no_condition_so_nothing_glows_and_the_next_play_draws_nothing() {
            crate::register_all();
            let mut s = board(false);
            s.play(FULLSEND, json!({}));
            let cheap = s.card(COST_0).id.clone();
            assert!(!hand_glows(&s, &cheap, P1));
            let before = s.hand(P1).len();
            s.play(COST_0, json!({}));
            assert_eq!(s.hand(P1).len(), before - 1);
        }
    }
}
