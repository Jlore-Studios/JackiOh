//! C #10 Exile (SPEC §8.6 row 10). Trap, cost 2, Common.
//!   Base:    "Activates when your opponent plays a card that costs ({threshold}) or less: Counter and
//!             exile it."
//!   Radiant: "Activates when your opponent plays a card that costs ({threshold}) or less: Counter and
//!             exile it. Then exile random enemy permanents that together cost up to ({threshold})
//!             minus its cost."
//!
//! Counter (§6.3, B5 E1, R448), in §10.5's announce window (`cardAnnounced`), before the card moves:
//! the countered card never resolves or enters the field and goes to exile, not the graveyard. It is
//! treated as never played. "Costs" is the cost paid (the announce's `costPaid`), as #60 Bear Honeypot
//! reads it (R56), so a card cast for free (R70) always qualifies. A card set face-down is announced to
//! the opponent by its zone and cost only (§10.5 step 3a), so this trap reads nothing it may not; the
//! exile then shows the card (exile is public). The condition lives in `when` (R99, R61), so a play
//! that costs more, or its controller's own play, leaves it set. A Trap is consumed when it fires.
//!
//! Radiant: a budget of the threshold ((3) on the Radiant face, tuned with it) minus the countered
//! card's cost paid, as the designer's "until the difference in cost is made up (but never exceeded)"
//! reads. Then, one at a time: pick a random enemy permanent (the top of a unit pile or a backrow card,
//! face-down ones included) whose cost now is no more than the budget left — R396: R65's cost where it
//! stands, an X card at the X it was played for (0 with none chosen), read by `costNow` — exile it and
//! take its cost off the budget; stop when the budget is 0 or nothing fits. A (0) Cost permanent always
//! fits while the budget is above 0. The picks are drawn from the match rng as the clause resolves and
//! kept (`forEachCard`), so a pause could never re-roll them (R113). The name is also a rules word,
//! which the reference proof never reads as this card unless `refs` lists it (R381).

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-010";

/// The enemy permanents on the field: the top of each unit pile, then each backrow card (§3.2, R13).
/// (TS `const ENEMY_PERMANENTS: BoardScope`; built on use, since a `BoardScope` is not a `const`.)
fn enemy_permanents() -> BoardScope {
    json_as(json!({ "side": "enemy", "rows": ["units", "backrow"] }))
}

/// TS `Extract<GameEvent, { type: "cardAnnounced" }>`: the fields of the announce this card reads.
struct Announced {
    instance_id: String,
    cost_paid: i32,
}

/// "When your opponent plays a card that costs ({threshold}) or less": the cost paid (R56, R70).
fn cheap_play(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<Announced> {
    let GameEvent::CardAnnounced { player, instance_id, cost_paid, .. } = event else {
        return None;
    };
    if *player == ctx.controller {
        return None;
    }
    if *cost_paid <= param(ctx, "threshold") {
        Some(Announced { instance_id: instance_id.clone(), cost_paid: *cost_paid })
    } else {
        None
    }
}

/// The Radiant face's picks, drawn as the clause resolves: random enemy permanents, each costing no
/// more than the budget left (R396), until the budget is spent or nothing fits.
fn budget_picks(ctx: &mut EffectContext<'_>, budget: i32) -> Vec<CardInstance> {
    let mut picked: Vec<CardInstance> = Vec::new();
    let mut left = budget;
    while left > 0 {
        let fits: Vec<CardInstance> = cards_in_scope(ctx, &enemy_permanents())
            .into_iter()
            .filter(|card| !picked.iter().any(|taken| taken.id == card.id) && cost_now(&ctx.state, card) <= left)
            .collect();
        let Some(card) = ctx.rng.pick(&fits).cloned() else {
            break;
        };
        left -= cost_now(&ctx.state, &card);
        picked.push(card);
    }
    picked
}

fn exile_trap(radiant_face: bool) -> TriggerDef {
    TriggerDef::new("exile", &[GameEventType::CardAnnounced], move |ctx, event| {
        let Some(played) = cheap_play(ctx, event) else {
            return vec![];
        };
        let countered = counter_play(json_as(json!({
            "to": "exile",
            "target": { "of": "instance", "instanceId": played.instance_id },
        })));
        if !radiant_face {
            return vec![countered];
        }
        let budget = param(&*ctx, "threshold") - played.cost_paid;
        vec![
            countered,
            for_each_card(ForEachCardArgs {
                // `forEachCard`'s `cards` answers ids (part 6's port of TS's `CardInstance | string`).
                cards: Arc::new(move |at: &mut EffectContext<'_>| {
                    budget_picks(at, budget).into_iter().map(|card| card.id).collect()
                }),
                each: Arc::new(|instance_id: &str| {
                    exile(json_as(json!({ "target": { "of": "instance", "instanceId": instance_id } })))
                }),
            }),
        ]
    })
    .with_when(|ctx, event| cheap_play(ctx, event).is_some())
}

pub fn script() -> CardScripts {
    let base = Script { triggers: vec![exile_trap(false)], ..Script::default() };

    let radiant = Script { triggers: vec![exile_trap(true)], ..Script::default() };
    CardScripts { base, radiant }
}

// C #10 Exile — SPEC §8.6 row 10, BUILD M9 Classic row C 10: "Face-down (R33); fires on the opponent's
// play of a card whose cost paid is (2) or less (R56), so a free cast always qualifies (R70), in the
// announce window before the card moves (§10.5); counters it: it never resolves or enters the field,
// no Cry, not counted as played by the turn's or the game's counts, Combo, Quickstriker or Ceaseless
// Void, its mana and Tributes stay spent, and it goes to exile, not the graveyard; a (3)+ Cost play
// and your own plays leave it set; a face-down set is announced to the opponent by its zone and cost
// only (§10.5 step 3a), so their `cardAnnounced` names no card and the exile then shows the card;
// radiant: (3) or less, then exile random enemy permanents one at a time, each with a cost (R65 on the
// field; an X card its X, 0 with none chosen, R396) no more than the budget left, budget = 3 minus the
// countered card's cost paid, until the budget is 0 or nothing fits, a (0) Cost permanent always
// fitting while the budget is above 0; its name is a rules word, so "exile" in other texts is no
// reference to it (R381); its tuned number (threshold) reads through `param()` (R386)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    /// Card tests run on the real catalog and scripts (TS: the vitest globalSetup's `registerAll`).
    fn scenario(setup: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(setup)
    }

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const EXILE: &str = "classic-010";
    const TIMMY: &str = "core-011"; // (1) Unit 3/3.
    const TOKEN_MAN: &str = "core-015"; // (1) Unit, "Cry: Summon a Rush Token."
    const POINTMASTER: &str = "core-020"; // (2) Unit 7/1.
    const MENACE: &str = "core-019"; // (3) Unit 9/9.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const STOCKPILE: &str = "core-005"; // (1) Spell.
    const BEAR: &str = "core-060"; // (1) Trap.
    const HINDER: &str = "core-021"; // (0) Spell, cast on draw.
    const SLIME: &str = "classic-027"; // (0) Unit 1/1.
    const CHALICE: &str = "classic-087"; // (X) Field Spell, Plague Chalice.
    const ROCK: &str = "core-066"; // (4) Unit 10/10, Tribute 1, Indestructible.

    fn armed(radiant_face: bool, lane: i32) -> Value {
        json!({ "def": EXILE, "radiant": radiant_face, "faceUp": false, "lane": lane })
    }

    /// TS `{ ...base, ...extra }` on a setup object: `extra`'s keys replace `base`'s.
    fn merged(base: Value, extra: Value) -> Value {
        let mut out = base;
        if let (Some(map), Value::Object(extra)) = (out.as_object_mut(), extra) {
            for (key, value) in extra {
                map.insert(key, value);
            }
        }
        out
    }

    fn setup(p2: Value, radiant_face: bool, p1: Value) -> Scenario {
        scenario(json!({
            "active": "p2",
            "p1": merged(json!({ "hand": [VANILLA], "backrow": [armed(radiant_face, 2)], "library": [VANILLA, VANILLA] }), p1),
            "p2": merged(json!({ "hand": [TIMMY, POINTMASTER, STOCKPILE], "library": [VANILLA, VANILLA, VANILLA] }), p2),
        }))
    }

    fn count(events: &[GameEvent], kind: GameEventType) -> usize {
        events.iter().filter(|event| event.event_type() == kind).count()
    }

    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).unwrap()
    }

    fn defs(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.def_id.clone()).collect()
    }

    #[test]
    fn is_a_trap_on_both_faces_whose_condition_lives_in_when_r99() {
        assert_eq!(crate::card_def(EXILE).type_, CardType::Trap);
        let CardScripts { base, radiant } = script();
        assert!(base.triggers.first().is_some_and(|trigger| trigger.when.is_some()));
        assert!(radiant.triggers.first().is_some_and(|trigger| trigger.when.is_some()));
    }

    #[test]
    fn r381_its_name_is_a_rules_word_no_other_card_s_text_refers_to_it_unless_its_refs_list_it() {
        let referring: Vec<&CardDef> = crate::CATALOG
            .values()
            .filter(|card| card.refs.clone().unwrap_or_default().iter().any(|id| id == EXILE))
            .collect();
        assert!(referring.is_empty());
    }

    mod base {
        use super::*;

        #[test]
        fn r33_it_is_face_down_and_the_opponent_s_view_never_names_it() {
            let s = setup(json!({}), false, json!({}));
            assert_eq!(s.card(EXILE).face_up, Some(false));
            assert!(!serde_json::to_string(&s.view(P2)).unwrap().contains(EXILE));
        }

        #[test]
        fn r448_counters_the_opponent_s_1_cost_unit_before_it_enters_the_field_and_exiles_it() {
            let mut s = setup(json!({}), false, json!({}));
            let timmy = s.card(TIMMY).clone();

            s.play(&timmy, json!({ "zone": 1 }));

            s.expect_in_zone(&timmy, "exile");
            assert!(s.unit(P2, 1).is_none());
            assert_eq!(count(s.last_events(), GameEventType::CardPlayed), 0);
            assert_eq!(count(s.last_events(), GameEventType::Summoned), 0);
            s.expect_events(json!(["cardAnnounced", "trapFired", "countered", "exiled"]));
            assert!(!s.pile(P2, "graveyard").iter().any(|card| card.id == timmy.id));
        }

        #[test]
        fn r448_no_cry_no_count_and_the_mana_stays_spent() {
            let mut s = setup(json!({ "hand": [TOKEN_MAN, VANILLA] }), false, json!({}));
            let played = s.state().players.p2.turn_log.cards_played;
            let game = s.state().counters.played;

            s.play(TOKEN_MAN, json!({ "zone": 1 }));

            assert!(s.unit(P2, 2).is_none());
            assert_eq!(count(s.last_events(), GameEventType::Summoned), 0);
            assert_eq!(s.state().players.p2.turn_log.cards_played, played);
            assert_eq!(s.state().counters.played, game);
            s.expect_mana(P2, 3);
        }

        #[test]
        fn r56_a_spell_costing_1_is_countered_and_exiled_too() {
            let mut s = setup(json!({}), false, json!({}));
            let spell = s.card(STOCKPILE).clone();

            s.play(&spell, json!({}));

            s.expect_in_zone(&spell, "exile");
            assert_eq!(count(s.last_events(), GameEventType::Drawn), 0);
        }

        #[test]
        fn r70_a_free_cast_always_qualifies_a_cast_on_draw_card_is_countered_and_exiled() {
            let mut s = scenario(json!({
                "p1": { "hand": [VANILLA], "backrow": [armed(false, 2)], "library": [VANILLA, VANILLA] },
                "p2": { "hand": [VANILLA], "library": [{ "def": HINDER, "radiant": true }, VANILLA, VANILLA] },
            }));
            let hinder = s.card(HINDER).clone();

            s.end_turn();

            s.expect_in_zone(&hinder, "exile");
            assert_eq!(s.state().players.p1.mana.next_turn_mod, 0);
        }

        #[test]
        fn r56_a_2_cost_play_is_countered_and_exiled_too() {
            let mut s = setup(json!({}), false, json!({}));

            s.play(POINTMASTER, json!({ "zone": 1 }));

            s.expect_in_zone(POINTMASTER, "exile");
            s.expect_events(json!(["cardAnnounced", "trapFired", "countered", "exiled"]));
        }

        #[test]
        fn r56_a_3_cost_play_leaves_it_set() {
            let mut s = setup(json!({ "hand": [MENACE, VANILLA] }), false, json!({}));

            s.play(MENACE, json!({ "zone": 1 }));

            s.expect_in_zone(MENACE, "field");
            assert_eq!(count(s.events(), GameEventType::Countered), 0);
            assert_eq!(s.backrow(P1, 2).and_then(|card| card.face_up), Some(false));
        }

        #[test]
        fn its_controller_s_own_plays_leave_it_set() {
            let mut s = scenario(json!({
                "p1": { "hand": [TIMMY, VANILLA], "backrow": [armed(false, 2)] },
                "p2": { "hand": [VANILLA] },
            }));

            s.play(TIMMY, json!({ "zone": 1 }));

            s.expect_in_zone(TIMMY, "field");
            assert_eq!(s.backrow(P1, 2).and_then(|card| card.face_up), Some(false));
        }

        #[test]
        fn sec5_1_the_trap_is_consumed_when_it_fires() {
            let mut s = setup(json!({}), false, json!({}));
            let trap = s.card(EXILE).clone();

            s.play(TIMMY, json!({ "zone": 1 }));

            s.expect_in_zone(&trap, "graveyard");
        }

        #[test]
        fn r227_a_face_down_set_is_announced_to_the_other_player_by_zone_and_cost_only_the_exile_then_shows_it() {
            let mut s = setup(json!({ "hand": [BEAR, VANILLA] }), false, json!({}));
            let bear = s.card(BEAR).clone();

            s.play(&bear, json!({ "zone": 3 }));

            let seen = js(&s.view(P1))["events"].as_array().cloned().unwrap_or_default();
            let announced = seen.iter().find(|event| event["type"] == "cardAnnounced");
            assert!(announced.is_some());
            assert!(!announced.map(Value::to_string).unwrap_or_default().contains(BEAR));
            let exiled = seen.iter().find(|event| event["type"] == "exiled");
            assert_eq!(exiled.map(|event| event["defId"].clone()), Some(json!(BEAR)));
            assert_eq!(defs(&s.pile(P2, "exile")), [BEAR]);
        }

        #[test]
        fn r386_an_upgrade_of_its_threshold_reaches_a_3_cost_play() {
            let mut s = setup(json!({ "hand": [MENACE, VANILLA] }), false, json!({}));
            step_param(s.card_mut(EXILE), "threshold", 1);

            s.play(MENACE, json!({ "zone": 1 }));

            s.expect_in_zone(MENACE, "exile");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn counters_and_exiles_a_3_cost_play() {
            let mut s = setup(json!({ "hand": [MENACE, VANILLA] }), true, json!({}));

            s.play(MENACE, json!({ "zone": 1 }));

            s.expect_in_zone(MENACE, "exile");
        }

        #[test]
        fn a_4_cost_play_leaves_it_set() {
            let mut s = setup(json!({ "hand": ["core-043", VANILLA] }), true, json!({}));

            s.play("core-043", json!({ "zone": 1 }));

            assert_eq!(count(s.events(), GameEventType::Countered), 0);
        }

        #[test]
        fn then_exiles_enemy_permanents_within_the_budget_3_minus_the_cost_paid() {
            // A (1) Cost play leaves a budget of 2: the (2) Cost Pointmaster fits, the (3) Cost Menace never.
            let mut s = setup(json!({ "field": [POINTMASTER, MENACE] }), true, json!({}));
            let (Some(point), Some(menace)) = (s.unit(P2, 1), s.unit(P2, 2)) else {
                panic!("fixture");
            };

            s.play(TIMMY, json!({ "zone": 3 }));

            s.expect_in_zone(&point, "exile");
            s.expect_in_zone(&menace, "field");
        }

        #[test]
        fn never_exceeds_the_budget_the_exiled_permanents_costs_total_at_most_the_budget() {
            let mut s = setup(json!({ "field": [VANILLA, VANILLA, POINTMASTER, MENACE] }), true, json!({}));

            s.play(TIMMY, json!({ "zone": 5 }));

            let exiled: Vec<CardInstance> =
                s.pile(P2, "exile").into_iter().filter(|card| card.def_id != TIMMY).collect();
            let total: i32 = exiled.iter().map(|card| if card.def_id == POINTMASTER { 2 } else { 1 }).sum();
            assert!(total > 0);
            assert!(total <= 2);
            s.expect_in_zone(MENACE, "field");
        }

        #[test]
        fn a_0_cost_permanent_always_fits_while_the_budget_is_above_0() {
            // A (2) Cost play leaves 1; the Slime costs 0 and fits, and then nothing else does.
            let mut s = setup(json!({ "field": [SLIME, MENACE] }), true, json!({}));
            let slime = s.card(SLIME).clone();

            s.play(POINTMASTER, json!({ "zone": 3 }));

            s.expect_in_zone(&slime, "exile");
            s.expect_in_zone(MENACE, "field");
        }

        #[test]
        fn a_3_cost_play_leaves_no_budget_only_the_countered_card_goes() {
            let mut s = setup(json!({ "hand": [MENACE, VANILLA], "field": [SLIME] }), true, json!({}));

            s.play(MENACE, json!({ "zone": 2 }));

            s.expect_in_zone(SLIME, "field");
        }

        #[test]
        fn reaches_the_enemy_backrow_a_face_down_trap_included_and_exile_shows_it() {
            let mut s = setup(
                json!({ "backrow": [{ "def": BEAR, "faceUp": false, "lane": 1 }], "field": [MENACE] }),
                true,
                json!({}),
            );
            let bear = s.card(BEAR).clone();

            s.play(TIMMY, json!({ "zone": 2 }));

            s.expect_in_zone(&bear, "exile");
            assert!(js(&s.view(P1))["opponent"]["exile"].to_string().contains(BEAR));
        }

        #[test]
        fn r13_a_card_dormant_under_a_stack_pile_is_not_on_the_field_only_the_top_of_the_pile_is_weighed() {
            // Budget 2: the dormant (1) Cost Vanilla would fit, the (3) Cost Menace on top of it never does.
            let mut s = setup(
                json!({ "field": [{ "def": VANILLA, "lane": 1 }, { "def": MENACE, "stack": true }] }),
                true,
                json!({}),
            );
            let dormant = s.card(VANILLA).clone();

            s.play(TIMMY, json!({ "zone": 2 }));

            s.expect_in_zone(TIMMY, "exile");
            s.expect_in_zone(&dormant, "field");
            assert_eq!(s.unit(P2, 1).map(|card| card.def_id), Some(MENACE.to_string()));
            assert_eq!(defs(&s.pile(P2, "exile")), [TIMMY]);
        }

        #[test]
        fn never_exiles_its_controller_s_own_permanents() {
            let mut s = setup(json!({ "field": [MENACE] }), true, json!({ "field": [SLIME, VANILLA] }));
            let mine = [s.unit(P1, 1), s.unit(P1, 2)];

            s.play(TIMMY, json!({ "zone": 2 }));

            for card in mine {
                let Some(card) = card else {
                    panic!("fixture");
                };
                s.expect_in_zone(&card, "field");
            }
        }

        #[test]
        fn r396_an_x_card_on_the_field_costs_the_x_it_was_played_for_and_0_with_none_chosen() {
            let mut s = setup(
                json!({ "backrow": [{ "def": CHALICE, "faceUp": true, "lane": 1 }], "field": [MENACE] }),
                true,
                json!({}),
            );
            let chalice = s.card(CHALICE).id.clone();
            if let Some(live) = find_instance_mut(s.state_mut(), &chalice) {
                live.x = Some(3);
            }

            // Budget 2: a Plague Chalice played for 3 does not fit.
            s.play(TIMMY, json!({ "zone": 2 }));
            s.expect_in_zone(chalice.as_str(), "field");

            let mut t = setup(
                json!({ "backrow": [{ "def": CHALICE, "faceUp": true, "lane": 1 }], "field": [MENACE] }),
                true,
                json!({}),
            );
            let none = t.card(CHALICE).id.clone();
            if let Some(live) = find_instance_mut(t.state_mut(), &none) {
                live.x = None;
            }
            t.play(TIMMY, json!({ "zone": 2 }));
            t.expect_in_zone(none.as_str(), "exile");
        }

        #[test]
        fn r448_a_countered_tribute_card_s_tribute_stays_spent_an_upgrade_to_4_reaches_the_rock() {
            let mut s = setup(json!({ "hand": [ROCK, VANILLA], "field": [{ "def": TIMMY, "lane": 1 }] }), true, json!({}));
            step_param(s.card_mut(EXILE), "threshold", 1);
            let Some(timmy) = s.unit(P2, 1) else {
                panic!("fixture");
            };

            s.play(ROCK, json!({ "zone": 2, "tributes": [timmy.id] }));

            s.expect_in_zone(ROCK, "exile");
            s.expect_in_zone(&timmy, "graveyard");
            s.expect_mana(P2, 0);
            assert!(s.unit(P2, 2).is_none());
        }

        #[test]
        fn r386_a_degrade_of_its_threshold_makes_it_answer_2_or_less_with_a_budget_of_2_minus_the_cost() {
            let mut s = setup(json!({ "hand": [MENACE, TIMMY], "field": [POINTMASTER] }), true, json!({}));
            step_param(s.card_mut(EXILE), "threshold", -1);

            s.play(MENACE, json!({ "zone": 2 }));
            assert_eq!(count(s.events(), GameEventType::Countered), 0);
            s.play(TIMMY, json!({ "zone": 3 }));

            s.expect_in_zone(TIMMY, "exile");
            // Budget 2 − 1 = 1: the (2) Cost Pointmaster no longer fits.
            s.expect_in_zone(POINTMASTER, "field");
        }
    }
}
