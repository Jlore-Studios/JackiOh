//! C #9 Income Tax (SPEC §8.6 row 9, §6.3 Steal, §10.6; R12, R33, R58, R61, R78, R97, R99, R317,
//! R521). Trap, cost 2, Legendary.
//!   Base:    "Activates when the cards your opponent has drawn in a turn reach {draws}: They keep one
//!            card of their choice and give you the rest of their hand."
//!   Radiant: the same, then "The cards you get cost ({discount}) less."
//!
//! The condition (R99, a trap's `when`) is the engine's draw count per player per turn (§10.1, B5 E4),
//! on either turn, riding each `drawn` event as `turnDraw`: it counts the start-of-turn draw and a burned
//! or cast-on-draw card, not a draw a limit stops or an empty deck (R521). A card cast on it goes first
//! (R58). The opponent keeps one hand card (R177); `giveFromHand` (B5 E16) gives you the rest as yours
//! (R12, §3.2), your hand cap burning the overflow (R317), unread by them (R97); one card or none asks
//! nothing (R61). The Radiant `costMod` (R78) is read at firing and carried to the answer: the trap is
//! in the graveyard by then.

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-009";

/// Where the Radiant face's discount travels from the firing to the answer (§10.6).
const DISCOUNT: &str = "discount";

fn taxes(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    matches!(
        event,
        GameEvent::Drawn { player, turn_draw, .. }
            if *player == opponent_of(ctx.controller) && *turn_draw == Some(param(ctx, "draws"))
    )
}

fn tax(radiant: bool) -> TriggerDef {
    TriggerDef::new("income-tax", &[GameEventType::Drawn], move |ctx, _event| {
        if zone_count(ctx.state, opponent_of(ctx.controller), OffFieldZone::Hand) <= 1 {
            return vec![];
        }
        let mut data = json!({});
        if radiant {
            data[DISCOUNT] = json!(param(&*ctx, "discount"));
        }
        vec![choose_from_hand(json_as(json!({
            "of": "enemy",
            "by": "enemy",
            "count": 1,
            "step": "keep",
            "prompt": "Income Tax: keep one card; your opponent takes the rest",
            "data": data,
        })))]
    })
    .with_when(taxes)
}

fn discount_of(ctx: &EffectContext<'_>) -> i32 {
    // Only an integer counts, else 0 (§4.4.10).
    match ctx.data.get(DISCOUNT) {
        Some(Value::Number(n)) => n
            .as_i64()
            .map(|value| value as i32)
            .or_else(|| n.as_f64().filter(|value| value.fract() == 0.0).map(|value| value as i32))
            .unwrap_or(0),
        _ => 0,
    }
}

pub fn script() -> CardScripts {
    let base = Script {
        triggers: vec![tax(false)],
        resume: IndexMap::from([(
            "keep",
            hook(|_ctx| vec![give_from_hand(json_as(json!({ "from": "enemy", "cards": "unchosen" })))]),
        )]),
        ..Script::default()
    };

    let radiant = Script {
        triggers: vec![tax(true)],
        resume: IndexMap::from([(
            "keep",
            hook(|ctx| {
                vec![give_from_hand(json_as(json!({
                    "from": "enemy",
                    "cards": "unchosen",
                    "costMod": -discount_of(ctx),
                })))]
            }),
        )]),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// C #9 Income Tax — SPEC §8.6 row 9, BUILD M9 Classic row C 9: fires on the opponent's second draw of a
// turn, on either turn, after a cast-on-draw card is cast (R58); a draw a limit stops does not count
// (§2.4); the rest of their hand becomes yours (R12), overflow burned (R317); radiant: (1) less, kept in
// every zone (R78); tuned numbers read through `param()` (R386). Here p2 sets the trap, p1 draws.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TAX: &str = "classic-009";
    const PALANTIR: &str = "classic-004"; // (1) Field Spell: "Aura: Your opponent can't draw more than 1 card each turn."
    const STOCKPILE: &str = "core-005"; // (1) Spell: "Draw 2. Heal your hero 2."
    const JELLY_BEAN: &str = "core-027"; // (1) Spell: "Cast on draw: Make a random card in your hand Radiant. Lose 5 health."
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const MENACE: &str = "core-019"; // (3) Unit 9/9
    const SEVEN: &str = "core-025"; // (4) Unit 7/7
    const FELINORS: &str = "core-012"; // (2) Unit 3/4
    const TIMMY: &str = "core-011"; // (1) Unit 3/3

    fn must<T>(value: Option<T>, what: &str) -> T {
        match value {
            Some(value) => value,
            None => panic!("the scenario has no {what}"),
        }
    }

    fn open(s: &Scenario) -> PendingChoice {
        must(s.state().pending.clone(), "open prompt")
    }

    use crate::js;

    fn tax_board(opts: Value) -> Scenario {
        let radiant_face = opts.get("radiantFace").and_then(Value::as_bool) == Some(true);
        let mut backrow = vec![json!({ "def": TAX, "faceUp": false, "radiant": radiant_face })];
        if let Some(Value::Array(more)) = opts.get("p2Backrow") {
            backrow.extend(more.iter().cloned());
        }
        scenario(json!({
            "p1": {
                "hand": opts.get("p1Hand").cloned().unwrap_or(json!([STOCKPILE, MENACE])),
                "library": opts.get("p1Library").cloned().unwrap_or(json!([FELINORS, SEVEN, TIMMY])),
            },
            "p2": {
                "hand": opts.get("p2Hand").cloned().unwrap_or(json!([VANILLA])),
                "backrow": backrow,
                "library": [VANILLA, VANILLA],
            },
        }))
    }

    fn fired(s: &Scenario) -> bool {
        s.events().iter().any(|event| event.event_type() == GameEventType::TrapFired)
    }

    fn hand_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.hand(player).into_iter().map(|card| card.def_id).collect()
    }

    fn drawn_by(events: &[GameEvent], who: PlayerId) -> usize {
        events
            .iter()
            .filter(|event| matches!(event, GameEvent::Drawn { player, .. } if *player == who))
            .count()
    }

    fn count(events: &[GameEvent], kind: GameEventType) -> usize {
        events.iter().filter(|event| event.event_type() == kind).count()
    }

    fn backrow_def(s: &Scenario, player: PlayerId, lane: i32) -> Option<String> {
        s.backrow(player, lane).map(|card| card.def_id)
    }

    fn answer_with(s: &mut Scenario, card: &str) {
        let id = s.card(card).id.clone();
        s.answer(json!(id));
    }

    #[test]
    fn declares_its_two_numbers_r386_trigger_draw_2_never_below_2_radiant_discount_1() {
        assert_eq!(
            js(&crate::card_def(TAX).params),
            json!([
                { "key": "draws", "base": 2, "radiant": 2, "better": "down", "step": 1, "min": 2 },
                { "key": "discount", "base": 1, "radiant": 1, "better": "up", "step": 1, "min": 1 },
            ])
        );
        let CardScripts { base, radiant } = script();
        let ons = |face: &Script| face.triggers.iter().map(|trigger| trigger.on.clone()).collect::<Vec<_>>();
        assert_eq!(ons(&base), vec![vec![GameEventType::Drawn]]);
        assert_eq!(ons(&radiant), vec![vec![GameEventType::Drawn]]);
    }

    mod base {
        use super::*;

        #[test]
        fn r33_it_sits_face_down_the_opponent_reads_a_face_down_card_and_nothing_more() {
            let s = tax_board(json!({}));
            assert_eq!(js(&s.view(P1))["opponent"]["backrow"][0], json!({ "faceDown": true, "cost": 2 }));
        }

        #[test]
        fn fires_when_the_opponent_s_second_draw_of_a_turn_is_complete_they_pick_one_card_to_keep() {
            let mut s = tax_board(json!({}));
            s.play(STOCKPILE, json!({}));
            assert!(fired(&s));
            let keep = open(&s);
            assert_eq!(keep.player_id, P1);
            assert_eq!(keep.kind, PromptKind::Hand);
            assert_eq!(keep.min, 1);
            assert_eq!(keep.max, 1);
            let offered: Vec<String> = keep
                .options
                .iter()
                .map(|option| match &option.selection {
                    Selection::Instance { instance_id } => s.card(instance_id.as_str()).def_id.clone(),
                    _ => "?".to_string(),
                })
                .collect();
            assert_eq!(offered, [MENACE, FELINORS, SEVEN]);
        }

        #[test]
        fn r12_the_opponent_keeps_one_card_and_every_other_card_moves_to_your_hand_as_yours() {
            let mut s = tax_board(json!({}));
            s.play(STOCKPILE, json!({}));
            let menace = s.card(MENACE).clone();
            let felinors = s.hand(P1).into_iter().find(|card| card.def_id == FELINORS);
            let seven = s.hand(P1).into_iter().find(|card| card.def_id == SEVEN);
            s.answer(json!(menace.id));
            assert_eq!(hand_defs(&s, P1), [MENACE]);
            assert_eq!(hand_defs(&s, P2), [VANILLA, FELINORS, SEVEN]);
            for card in [felinors, seven] {
                let card = must(card, "a moved card");
                let moved = s.card(&card);
                assert_eq!(moved.owner, P2);
                assert_eq!(moved.controller, P2);
            }
            s.expect_in_zone(TAX, "graveyard");
        }

        #[test]
        fn r521_one_draw_is_not_two_a_single_draw_leaves_it_set() {
            let mut s = tax_board(json!({ "p1Library": [FELINORS] }));
            s.start_turn();
            assert!(!fired(&s));
            assert_eq!(backrow_def(&s, P2, 1), Some(TAX.to_string()));
        }

        #[test]
        fn on_their_own_turn_the_start_of_turn_draw_is_the_first_so_any_extra_draw_sets_it_off() {
            let mut s = tax_board(json!({ "p1Library": [TIMMY, FELINORS, SEVEN] }));
            s.start_turn();
            assert!(!fired(&s));
            s.play(STOCKPILE, json!({}));
            assert!(fired(&s));
            assert_eq!(open(&s).player_id, P1);
        }

        #[test]
        fn on_your_turn_too_their_second_draw_of_your_turn_sets_it_off_c_38_jackiestan_auctioneer_draws_for_them() {
            // p1's Radiant Auctioneer animates on p2's 2nd play and from then on draws p1 a card on each
            // play, so p2's 3rd and 4th plays are p1's 1st and 2nd draws of this, p2's, turn.
            const RECYCLING: &str = "core-039"; // (0) Spell
            let mut s = scenario(json!({
                "active": "p2",
                "p1": {
                    "hand": [VANILLA, MENACE],
                    "backrow": [{ "def": "classic-038", "radiant": true, "faceUp": false }],
                    "library": [FELINORS, SEVEN, TIMMY],
                },
                "p2": {
                    "hand": [RECYCLING, RECYCLING, RECYCLING, RECYCLING, VANILLA],
                    "backrow": [{ "def": TAX, "faceUp": false }],
                    "library": [VANILLA, VANILLA],
                },
            }));
            for _play in 0..3 {
                let filler = must(s.hand(P2).into_iter().find(|card| card.def_id == RECYCLING), "a filler");
                s.play(&filler, json!({}));
            }
            assert_eq!(drawn_by(s.events(), P1), 1);
            // The Auctioneer's own firing is a trap's too; the Tax is still set.
            assert_eq!(backrow_def(&s, P2, 1), Some(TAX.to_string()));
            let filler = must(s.hand(P2).into_iter().find(|card| card.def_id == RECYCLING), "a filler");
            s.play(&filler, json!({}));
            assert_eq!(s.state().active, P2);
            let keep = open(&s);
            assert_eq!(keep.player_id, P1);
            assert_eq!(keep.resume.def_id, TAX);
            answer_with(&mut s, MENACE);
            s.expect_in_zone(TAX, "graveyard");
            assert_eq!(hand_defs(&s, P1), [MENACE]);
        }

        #[test]
        fn a_count_is_per_turn_a_draw_last_turn_does_not_add_to_this_turn_s() {
            // p1 draws one with Anti-oneshot Armor's Cry, then two turn changes later draws at the start
            // of their turn: the first of that turn, not the second of the game.
            let mut s = scenario(json!({
                "p1": { "hand": ["core-073", MENACE], "library": [FELINORS, SEVEN, TIMMY, VANILLA] },
                "p2": { "hand": [VANILLA, VANILLA], "backrow": [{ "def": TAX, "faceUp": false }], "library": [VANILLA, VANILLA] },
            }));
            s.play("core-073", json!({}));
            assert_eq!(drawn_by(s.events(), P1), 1);
            s.end_turn(); // p2's turn
            s.end_turn(); // p1's turn starts: one draw (their first this turn)
            assert_eq!(s.state().active, P1);
            assert_eq!(drawn_by(s.events(), P1), 2);
            assert!(!fired(&s));
            assert_eq!(backrow_def(&s, P2, 1), Some(TAX.to_string()));
        }

        #[test]
        fn your_own_draws_never_set_it_off() {
            let mut s = scenario(json!({
                "p1": { "hand": [MENACE], "library": [VANILLA, VANILLA] },
                "p2": { "hand": [STOCKPILE, VANILLA], "backrow": [{ "def": TAX, "faceUp": false }], "library": [FELINORS, SEVEN, TIMMY] },
                "active": "p2",
            }));
            s.play(STOCKPILE, json!({}));
            assert_eq!(drawn_by(s.events(), P2), 2);
            assert!(!fired(&s));
            assert_eq!(backrow_def(&s, P2, 1), Some(TAX.to_string()));
        }

        #[test]
        fn r58_a_cast_on_draw_card_that_draw_finds_is_cast_first_then_the_trap_fires() {
            let mut s = tax_board(json!({ "p1Library": [FELINORS, JELLY_BEAN, SEVEN] }));
            s.play(STOCKPILE, json!({}));
            let cast_at = s
                .events()
                .iter()
                .position(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if def_id == JELLY_BEAN));
            let trap_at = s.events().iter().position(|event| event.event_type() == GameEventType::TrapFired);
            let cast_at = must(cast_at, "cast of the Jelly Bean");
            let trap_at = must(trap_at, "trapFired");
            assert!(cast_at < trap_at);
            s.expect_health(P1, 27); // 30 − 5 + 2
        }

        #[test]
        fn sec2_4_a_draw_a_limit_stops_does_not_happen_and_does_not_count_under_palantir_s_limit_of_1_it_never_fires() {
            let mut s = tax_board(json!({ "p2Backrow": [PALANTIR] }));
            s.start_turn(); // the one draw p1 may make this turn
            s.play(STOCKPILE, json!({})); // both draws stopped
            assert!(!fired(&s));
            assert_eq!(backrow_def(&s, P2, 1), Some(TAX.to_string()));
            assert_eq!(count(s.events(), GameEventType::DrawLimited), 2);
        }

        #[test]
        fn r177_the_keep_prompt_s_options_are_the_opponent_s_own_hand_and_your_view_names_none_of_them() {
            let mut s = tax_board(json!({}));
            s.play(STOCKPILE, json!({}));
            assert_eq!(js(&s.view(P2))["pending"], json!({ "forYou": false, "pendingFor": "p1" }));
            let yours = serde_json::to_string(&s.view(P2)).unwrap();
            for card in s.hand(P1) {
                assert!(!yours.contains(&format!("\"{}\"", card.id)));
                assert!(!yours.contains(&card.def_id));
            }
        }

        #[test]
        fn r317_your_hand_cap_burns_the_overflow_into_your_graveyard_both_players_seeing_which() {
            let nine = vec![VANILLA; 9];
            let mut s = tax_board(json!({ "p2Hand": nine }));
            s.play(STOCKPILE, json!({}));
            answer_with(&mut s, MENACE);
            assert_eq!(s.hand(P2).len(), 10);
            let burned: Vec<GameEvent> = s
                .events()
                .iter()
                .filter(|event| event.event_type() == GameEventType::Burned)
                .cloned()
                .collect();
            assert_eq!(burned.len(), 1);
            let burned_card = must(burned.first().cloned(), "a burned card");
            let GameEvent::Burned { instance_id, .. } = burned_card else {
                panic!("not a burn");
            };
            assert!(s.pile(P2, "graveyard").iter().any(|card| card.id == instance_id));
            assert_eq!(s.card(instance_id.as_str()).owner, P2);
            for viewer in [P1, P2] {
                let seen: Vec<GameEvent> = s
                    .view(viewer)
                    .events
                    .into_iter()
                    .filter(|event| event.event_type() == GameEventType::Burned)
                    .collect();
                assert_eq!(seen, burned);
            }
        }

        #[test]
        fn r521_a_card_burned_on_a_full_hand_was_drawn_and_counts() {
            // p1 holds ten after the Stockpile leaves and draws one: the first draw fills the hand, the second burns.
            let mut hand = vec![STOCKPILE, TIMMY];
            hand.extend(vec![MENACE; 8]);
            let mut s = tax_board(json!({ "p1Hand": hand, "p1Library": [FELINORS, SEVEN] }));
            s.play(STOCKPILE, json!({}));
            assert_eq!(count(s.events(), GameEventType::Burned), 1);
            assert!(fired(&s));
            assert_eq!(open(&s).player_id, P1);
        }

        #[test]
        fn r521_a_card_cast_on_draw_was_drawn_and_counts() {
            let mut s = tax_board(json!({ "p1Library": [JELLY_BEAN, FELINORS, SEVEN] }));
            s.play(STOCKPILE, json!({}));
            // The Jelly Bean was cast on the first draw; the Felinors is the second.
            assert!(s
                .events()
                .iter()
                .any(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if def_id == JELLY_BEAN)));
            assert!(fired(&s));
        }

        #[test]
        fn r521_a_draw_from_an_empty_deck_counts_toward_the_number_the_next_real_draw_is_the_2nd_and_fires_it() {
            // Their start-of-turn draw finds an empty deck (fatigue, draw 1); Unstable Clone Machine then
            // shuffles three Mr. Vanillas in, and Stockpile's first draw is their 2nd of the turn.
            let mut s = scenario(json!({
                "p1": { "hand": [VANILLA, STOCKPILE, MENACE], "backrow": ["core-033"], "library": [] },
                "p2": { "hand": [VANILLA], "backrow": [{ "def": TAX, "faceUp": false }], "library": [VANILLA, VANILLA] },
            }));
            s.start_turn();
            assert_eq!(count(s.events(), GameEventType::Fatigue), 1);
            assert!(!fired(&s));
            s.play(VANILLA, json!({}));
            assert_eq!(s.pile(P1, "library").len(), 3);
            s.play(STOCKPILE, json!({}));
            let drawn: Vec<Option<i32>> = s
                .events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Drawn { turn_draw, .. } => Some(*turn_draw),
                    _ => None,
                })
                .collect();
            assert_eq!(drawn, vec![Some(2), Some(3)]);
            assert!(fired(&s));
            assert_eq!(open(&s).player_id, P1);
        }

        #[test]
        fn r521_a_draw_from_an_empty_deck_draws_no_card_and_fires_nothing() {
            let mut s = tax_board(json!({ "p1Library": [] }));
            s.play(STOCKPILE, json!({}));
            assert_eq!(count(s.events(), GameEventType::Fatigue), 2);
            assert!(!fired(&s));
            assert_eq!(backrow_def(&s, P2, 1), Some(TAX.to_string()));
        }

        #[test]
        fn an_opponent_holding_one_card_keeps_it_and_you_get_nothing_no_prompt() {
            // The first draw is cast on draw (so it never reaches the hand), the second is the only card left.
            let mut s = tax_board(json!({ "p1Hand": [STOCKPILE], "p1Library": [JELLY_BEAN, FELINORS] }));
            s.play(STOCKPILE, json!({}));
            assert!(fired(&s));
            assert!(s.state().pending.is_none());
            assert_eq!(hand_defs(&s, P1), [FELINORS]);
            assert_eq!(hand_defs(&s, P2), [VANILLA]);
        }

        #[test]
        fn r97_once_in_your_hand_the_moved_cards_are_named_in_none_of_the_opponent_s_view_but_the_steal_itself() {
            // The `stolen` event is read by a player who could read the card where it was taken from or
            // where it is now (R97 extended to E2), so the opponent reads the steal of their own hand
            // cards; every other event and their view of your hand name none of them.
            let mut s = tax_board(json!({}));
            s.play(STOCKPILE, json!({}));
            let moved: Vec<CardInstance> = [SEVEN, FELINORS]
                .iter()
                .map(|def_id| must(s.hand(P1).into_iter().find(|card| card.def_id == *def_id), def_id))
                .collect();
            answer_with(&mut s, MENACE);
            let their_view = js(&s.view(P1));
            assert_eq!(their_view["opponent"]["hand"], json!({ "count": 3 }));
            let mut not_steals = their_view.clone();
            not_steals["events"] = Value::Array(
                their_view["events"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|event| event["type"] != "stolen")
                    .collect(),
            );
            let not_steals = not_steals.to_string();
            for card in &moved {
                assert!(!not_steals.contains(&format!("\"{}\"", card.id)));
                assert!(!not_steals.contains(&card.def_id));
            }
            for viewer in [P1, P2] {
                let mut steals: Vec<String> = s
                    .view(viewer)
                    .events
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::Stolen { instance_id, .. } => Some(instance_id.clone()),
                        _ => None,
                    })
                    .collect();
                steals.sort();
                let mut ids: Vec<String> = moved.iter().map(|card| card.id.clone()).collect();
                ids.sort();
                assert_eq!(steals, ids);
            }
        }

        #[test]
        fn sec9_3_the_keep_prompt_survives_a_json_round_trip_and_resumes_through_reduce() {
            let mut s = tax_board(json!({}));
            s.play(STOCKPILE, json!({}));
            let revived: GameState = serde_json::from_value(js(s.state())).unwrap();
            assert_eq!(&revived, s.state());
            let pending = must(revived.pending.clone(), "revived prompt");
            let seven = s.card(SEVEN).id.clone();
            let action: Action = json_as(json!({
                "type": "answer",
                "playerId": "p1",
                "choiceId": pending.id,
                "selection": [{ "pick": "instance", "instanceId": seven }],
                "nonce": "income-tax-round-trip",
            }));
            let result = reduce(&revived, &action);
            assert!(result.error.is_none());
            assert!(result.state.pending.is_none());
            assert!(result.state.work.is_empty());
        }

        #[test]
        fn the_base_face_gives_no_discount() {
            let mut s = tax_board(json!({}));
            s.play(STOCKPILE, json!({}));
            let felinors = must(s.hand(P1).into_iter().find(|card| card.def_id == FELINORS), "Felinors");
            answer_with(&mut s, MENACE);
            assert_eq!(s.card(&felinors).cost_mod, 0);
        }

        #[test]
        fn r386_a_degrade_of_the_trigger_draw_makes_it_the_3rd_draw() {
            let mut s = tax_board(json!({ "p1Library": [FELINORS, SEVEN, TIMMY, VANILLA] }));
            step_param(s.card_mut(TAX), "draws", 1);
            s.start_turn(); // 1
            s.play(STOCKPILE, json!({})); // 2, 3
            assert!(fired(&s));
            let mut t = tax_board(json!({ "p1Library": [FELINORS, SEVEN] }));
            step_param(t.card_mut(TAX), "draws", 1);
            t.play(STOCKPILE, json!({})); // 1, 2
            assert!(!fired(&t));
        }

        #[test]
        fn r386_never_below_2_an_upgrade_of_the_trigger_draw_leaves_it_at_the_2nd() {
            let mut s = tax_board(json!({}));
            step_param(s.card_mut(TAX), "draws", -1);
            s.start_turn(); // 1 — not yet
            assert!(!fired(&s));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_cards_you_get_cost_1_less_and_keep_that_in_every_zone_r78() {
            let mut s = tax_board(json!({ "radiantFace": true }));
            s.play(STOCKPILE, json!({}));
            let moved: Vec<CardInstance> = [FELINORS, SEVEN]
                .iter()
                .map(|def_id| must(s.hand(P1).into_iter().find(|card| card.def_id == *def_id), def_id))
                .collect();
            answer_with(&mut s, MENACE);
            for card in &moved {
                let live = s.card(card);
                assert_eq!(live.cost_mod, -1);
                assert_eq!(live.owner, P2);
            }
            let felinors = must(moved.first().cloned(), "Felinors");
            let seven = must(moved.get(1).cloned(), "7/7");
            assert_eq!(effective_cost(s.state(), s.card(&felinors), Default::default()), 1);
            assert_eq!(effective_cost(s.state(), s.card(&seven), Default::default()), 3);
        }

        #[test]
        fn the_kept_card_is_not_discounted() {
            let mut s = tax_board(json!({ "radiantFace": true }));
            s.play(STOCKPILE, json!({}));
            answer_with(&mut s, MENACE);
            assert_eq!(s.card(MENACE).cost_mod, 0);
        }

        #[test]
        fn r386_an_upgrade_of_the_discount_makes_them_cost_2_less() {
            let mut s = tax_board(json!({ "radiantFace": true }));
            step_param(s.card_mut(TAX), "discount", 1);
            s.play(STOCKPILE, json!({}));
            let seven = must(s.hand(P1).into_iter().find(|card| card.def_id == SEVEN), "7/7");
            answer_with(&mut s, MENACE);
            assert_eq!(s.card(&seven).cost_mod, -2);
        }

        #[test]
        fn the_same_condition_a_single_draw_leaves_it_set() {
            let mut s = tax_board(json!({ "radiantFace": true, "p1Library": [FELINORS] }));
            s.start_turn();
            assert!(!fired(&s));
        }
    }
}
