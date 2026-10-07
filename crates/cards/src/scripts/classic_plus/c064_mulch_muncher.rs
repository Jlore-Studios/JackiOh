//! C+ #64 Mulch Muncher (SPEC §8.7 row 64; BUILD M9 row C+ 64). (10) Unit, Rare, 9/9 → 18/18.
//!   Base:    "Rush, Trample / Costs ({discount}) less for each Fruit you've played this game." — discount 1
//!   Radiant: "Rush, Trample, Divine Shield / (the same)"
//!
//! A `cost` hook (Core #100's pattern, R55) reading the per-game count of Fruit-tagged plays
//! (`playedThisGameWithTag`, B5 E4: casts count, R70; a Grape is a Fruit; never reset), floored at 0.
//! R584: the discount is a price for a play, so it holds where a play takes the card from — its
//! player's hand, or a graveyard a permission lets them play it from (E11) — and counts that player's
//! Fruits; anywhere else (a deck, a graveyard, the field, a pool) it costs its printed (10) (R65).
//! The keywords are the faces' printed ones, so both faces run this one script.

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-064";

pub fn script() -> CardScripts {
    // TS `const PRINTED_COST = queryCost(def)`, read once as the module loads.
    let printed_cost = query_cost(&crate::card_def(ID));
    let base = Script {
        cost: Some(cost_hook(move |CostArgs { state, instance }| {
            let at = &instance.zone;
            let for_play = matches!(at, Zone::Hand { .. })
                || (matches!(at, Zone::Graveyard { .. }) && playable_from_graveyard(state, instance));
            if !for_play {
                return printed_cost;
            }
            let discount = param(
                &HookArgs {
                    state,
                    self_: instance,
                    radiant: instance.radiant,
                },
                "discount",
            );
            (printed_cost - discount * played_this_game_with_tag(state, at.player(), Tag::Fruit)).max(0)
        })),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// C+ #64 Mulch Muncher — SPEC §8.7 row 64, BUILD M9 row C+ 64: Rush, Trample; a `cost` hook (R55's
// pattern) that costs (1) less for each Fruit-tagged card its controller has played this game (casts
// included, R70; a Grape counts; the opponent's Fruits don't; never reset), floored at 0 and shown as
// its current cost in hand; out of play it costs (10) (R65, R584); the Trample excess reaches the hero
// (R63); the discount reads through `param()`; radiant 18/18 with Divine Shield too.
#[cfg(test)]
mod tests {
    use jackioh_engine::effects::cast_new;
    use jackioh_engine::testkit::*;

    const MULCH: &str = "classicplus-064";
    const FIG: &str = "core-047"; // Fig of Life: (3) Spell, Fruit. "Heal a target 20."
    const GRAPE: &str = "classicplus-065-1"; // Rotten Grape: (1) Spell, Fruit, Token.
    const FILLER: &str = "core-005";

    /// `_harness.ts` registers every card on import; the engine's testkit cannot, so this does.
    fn scenario(opts: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(opts)
    }

    fn hand_cost(s: &Scenario, player: PlayerId) -> i32 {
        let card = s
            .hand(player)
            .into_iter()
            .find(|held| held.def_id == MULCH)
            .unwrap_or_else(|| panic!("no Mulch Muncher in hand"));
        let view = match s.view(player).you.hand {
            HandView::Cards(cards) => cards,
            HandView::Count { .. } => panic!("own hand is a list"),
        };
        let shown = view.iter().find(|held| held.instance_id == card.id).map(|held| held.cost);
        let charged = effective_cost(s.state(), &card, Default::default());
        assert_eq!(shown, Some(charged), "the view shows the cost the engine charges");
        charged
    }

    fn play_fig(s: &mut Scenario) {
        let active = s.state().active;
        let fig = s
            .hand(active)
            .into_iter()
            .find(|card| card.def_id == FIG)
            .unwrap_or_else(|| panic!("no Fig of Life in hand"));
        s.play(&fig, json!({ "targets": [{ "pick": "hero", "player": active }] }));
    }

    fn with_fruit(radiant: bool, figs: Option<usize>) -> Scenario {
        let mut hand = vec![json!({ "def": MULCH, "radiant": radiant })];
        hand.extend((0..figs.unwrap_or(2)).map(|_| json!(FIG)));
        hand.push(json!(FILLER));
        scenario(json!({
            "p1": { "hand": hand, "mana": 30 },
            "p2": { "hand": [FILLER, FIG] },
        }))
    }

    fn kinds(keywords: &[Keyword]) -> Vec<String> {
        keywords.iter().map(|keyword| keyword.kind().as_str().to_string()).collect()
    }

    #[test]
    fn prints_a_10_9_9_rush_trample_radiant_18_18_with_divine_shield_and_runs_one_script_on_both_faces() {
        crate::register_all();
        let def = crate::card_def(super::ID);
        assert_eq!(def.cost, CardCost::Fixed(10));
        assert_eq!(def.base.attack, Some(9));
        assert_eq!(def.base.health, Some(9));
        assert_eq!(def.base.keywords, vec![Keyword::Rush, Keyword::Trample]);
        assert_eq!(def.radiant.attack, Some(18));
        assert_eq!(def.radiant.health, Some(18));
        assert_eq!(def.radiant.keywords, vec![Keyword::Rush, Keyword::Trample, Keyword::DivineShield]);
        assert!(!def.tags.contains(&Tag::Fruit));
        let scripts = super::script();
        // TS `expect(radiant).toBe(base)`: the one cost hook.
        assert!(std::sync::Arc::ptr_eq(
            scripts.radiant.cost.as_ref().expect("a cost hook"),
            scripts.base.cost.as_ref().expect("a cost hook"),
        ));
    }

    mod base {
        use super::*;

        #[test]
        fn r55_costs_1_less_for_each_fruit_its_controller_has_played_this_game_shown_as_its_cost_in_hand() {
            let mut s = with_fruit(false, None);
            assert_eq!(hand_cost(&s, PlayerId::P1), 10);
            play_fig(&mut s);
            assert_eq!(hand_cost(&s, PlayerId::P1), 9);
            play_fig(&mut s);
            assert_eq!(hand_cost(&s, PlayerId::P1), 8);
        }

        #[test]
        fn r382_a_grape_is_a_fruit_and_counts() {
            let mut s = scenario(json!({ "p1": { "hand": [MULCH, GRAPE, FILLER], "mana": 10 }, "p2": { "hand": [FILLER] } }));
            s.play(GRAPE, json!({}));
            assert_eq!(hand_cost(&s, PlayerId::P1), 9);
        }

        #[test]
        fn r70_a_cast_fruit_counts_as_a_play() {
            let mut s = scenario(json!({ "p1": { "hand": [MULCH, FILLER] }, "p2": { "hand": [FILLER] } }));
            let mut rng = create_rng(&s.state().seed, s.state().rng_cursor);
            let mut events: Vec<GameEvent> = Vec::new();
            {
                let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
                {
                    let mut ctx = make_context(
                        &mut sink,
                        None,
                        HookOptions { controller: Some(PlayerId::P1), ..HookOptions::default() },
                    );
                    apply_effects(&[cast_new(json_as(json!({ "def": FIG, "random": true })))], &mut ctx);
                }
                settle(&mut sink, SettleOptions::default());
            }
            s.state_mut().rng_cursor = rng.cursor();
            assert_eq!(hand_cost(&s, PlayerId::P1), 9);
        }

        #[test]
        fn the_opponents_fruits_dont_count() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [MULCH, FILLER] },
                "p2": { "hand": [FIG, FILLER], "mana": 10 },
            }));
            play_fig(&mut s);
            assert_eq!(hand_cost(&s, PlayerId::P1), 10);
        }

        #[test]
        fn the_per_game_count_never_resets_the_discount_outlives_the_turn_and_the_fruit() {
            let mut s = with_fruit(false, Some(1));
            play_fig(&mut s);
            s.expect_in_zone(FIG, "graveyard");
            s.end_turn().end_turn();
            assert_eq!(s.state().active, PlayerId::P1);
            assert_eq!(hand_cost(&s, PlayerId::P1), 9);
        }

        #[test]
        fn s2_3_the_cost_floors_at_0() {
            let mut s = with_fruit(false, Some(2));
            step_param(s.card_mut(MULCH), "discount", 5);
            play_fig(&mut s);
            assert_eq!(hand_cost(&s, PlayerId::P1), 4);
            play_fig(&mut s);
            assert_eq!(hand_cost(&s, PlayerId::P1), 0);
        }

        #[test]
        fn r386_the_discount_reads_through_param_an_upgrade_makes_each_fruit_2_off_a_degrade_never_takes_it_below_1() {
            let mut up = with_fruit(false, Some(1));
            step_param(up.card_mut(MULCH), "discount", 1);
            play_fig(&mut up);
            assert_eq!(hand_cost(&up, PlayerId::P1), 8);
            let mut down = with_fruit(false, Some(1));
            step_param(down.card_mut(MULCH), "discount", -1);
            play_fig(&mut down);
            assert_eq!(hand_cost(&down, PlayerId::P1), 9);
        }

        #[test]
        fn r584_r65_out_of_play_it_costs_10_in_a_pool_in_a_deck_in_a_graveyard_and_on_the_field_whatever_was_played() {
            let ids: Vec<String> = {
                crate::register_all();
                query(&json_as(json!({ "cost": 10 }))).iter().map(|card| card.id.clone()).collect()
            };
            assert!(ids.contains(&MULCH.to_string()));
            let mut s = scenario(json!({
                "p1": { "hand": [MULCH, FIG, FIG, FILLER], "library": [MULCH], "graveyard": [MULCH], "mana": 30 },
                "p2": { "hand": [FILLER] },
            }));
            play_fig(&mut s);
            play_fig(&mut s);
            assert_eq!(hand_cost(&s, PlayerId::P1), 8);
            let in_deck = s.pile(PlayerId::P1, "library").into_iter().next().expect("a library card");
            let in_grave = s.pile(PlayerId::P1, "graveyard").into_iter().next().expect("a graveyard card");
            assert_eq!(effective_cost(s.state(), &in_deck, Default::default()), 10);
            assert_eq!(effective_cost(s.state(), &in_grave, Default::default()), 10);
            s.play(MULCH, json!({ "zone": 1 }));
            let unit = s.unit(PlayerId::P1, 1).expect("the Muncher on the field");
            assert_eq!(cost_now(s.state(), &unit), 10);
            s.expect_mana(PlayerId::P1, 30 - 3 - 3 - 8);
        }

        #[test]
        fn r63_rush_lets_it_attack_a_unit_at_once_and_its_trample_excess_reaches_the_hero() {
            // One mana left over for the Stockpile, so the turn does not end itself (§2.5) and fatigue p2.
            let mut s = scenario(json!({
                "p1": { "hand": [MULCH, FILLER], "mana": 11 },
                "p2": { "hand": [FILLER], "field": ["core-008"] },
            }));
            s.play(MULCH, json!({ "zone": 1 }));
            let attacker = s.unit(PlayerId::P1, 1).expect("the Muncher");
            let target = s.unit(PlayerId::P2, 1).expect("Mr. Vanilla");
            s.attack(&attacker, &target);
            // 9 into Mr. Vanilla's 4/4: 5 tramples over.
            s.expect_health(PlayerId::P2, 25);
            assert!(s.unit(PlayerId::P2, 1).is_none());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_an_18_18_with_divine_shield_and_its_cost_falls_the_same_way() {
            let mut s = with_fruit(true, Some(1));
            assert_eq!(hand_cost(&s, PlayerId::P1), 10);
            play_fig(&mut s);
            assert_eq!(hand_cost(&s, PlayerId::P1), 9);
            s.play(MULCH, json!({ "zone": 2 }));
            let unit = s.unit(PlayerId::P1, 2).expect("the Muncher");
            let stats = s.stats(&unit);
            assert_eq!(stats.attack, 18);
            assert_eq!(stats.health, 18);
            assert_eq!(
                kinds(&stats.keywords),
                vec!["Rush".to_string(), "Trample".to_string(), "Divine Shield".to_string()]
            );
        }
    }
}
