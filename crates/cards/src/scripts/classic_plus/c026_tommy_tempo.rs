//! C+ #26 Tommy Tempo (SPEC §8.7 row 26): Taunt, cast on draw; that cast ends your turn (Radiant: after
//! {actions} more action, R415). No open zone: to the hand uncast (R560). Played from hand: a plain Unit.

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-026";

fn cast_on_draw(ctx: &EffectContext<'_>) -> bool {
    ctx.self_.as_ref().is_some_and(|me| is_cast_on_draw(&*ctx.state, me))
}

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(|ctx| {
            if cast_on_draw(ctx) {
                vec![end_turn(json_as(json!({})))]
            } else {
                Vec::new()
            }
        })),
        ..Script::default()
    };

    let radiant = Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(|ctx| {
            if cast_on_draw(ctx) {
                vec![end_turn_after_actions(json_as(json!({ "actions": param(&*ctx, "actions") })))]
            } else {
                Vec::new()
            }
        })),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C+ #26 Tommy Tempo — SPEC §8.7 row 26, BUILD M9 Classic+ row C+ 26: "Taunt; cast on draw (R70: free,
// counts as played) into your leftmost open unit zone, then, once the rest of the drawing effect's
// list resolves (a "Draw 2" still draws its second card), your turn ends as if you had pressed End
// turn, every end-of-turn step running; drawn at the start of your turn it ends that turn before your
// main phase; drawn on the opponent's turn only the summon happens; with no open zone it goes to your
// hand uncast as R58's cap sends one (burned when the hand is full); played from a hand it is a plain
// Unit and ends nothing; radiant you may take one more main-phase action (a play, an attack, a
// position switch, an activation, or ending the turn yourself) and the turn ends once it resolves
// (R415); the action count reads through `param()`".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TOMMY: &str = "classicplus-026";
    const STOCKPILE: &str = "core-005"; // Draw 2. Heal your hero 2.
    const MENACE: &str = "core-019"; // 9/9 Taunt; end of turn: heal this to full.
    const FILLER: &str = "core-008"; // Mr. Vanilla, a (1) 4/4: a plain card to play or hold
    const PANTHER: &str = "core-032"; // Prem Panther: after it attacks and survives, draw 2 for each Unit it destroyed
    const MOTHS: &str = "core-009"; // Moths to the Flame: start of turn, every enemy Unit attacks this
    const NOSE: &str = "classic-015"; // Nose Hunter: "Activate: Discard a random card. …"
    const SCARAB: &str = "core-007"; // Jewelosco Scarab: Cry: Discover a (2) Cost card (a prompt)
    const DECK: [&str; 6] = [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER];

    /// TS `{ ...defaults, ...overrides }` on a side setup: every key of `overrides` replaces the default's.
    fn spread(defaults: Value, overrides: &Value) -> Value {
        let mut out = defaults;
        if let (Some(into), Some(from)) = (out.as_object_mut(), overrides.as_object()) {
            for (key, value) in from {
                into.insert(key.clone(), value.clone());
            }
        }
        out
    }

    /// p2 is active; p1's library has Tommy on top, so p2 ending the turn makes p1 draw it at its start.
    fn drawn_at_start(radiant_face: bool, p1: Value) -> Scenario {
        crate::register_all();
        let mut library = vec![json!({ "def": TOMMY, "radiant": radiant_face })];
        library.extend(DECK.iter().map(|card| json!(card)));
        scenario(json!({
            "active": "p2",
            "p1": spread(json!({ "hand": [FILLER], "library": library }), &p1),
            "p2": { "hand": [FILLER], "library": DECK },
        }))
    }

    fn tommy_on_field(s: &Scenario) -> Option<CardInstance> {
        (1..=5).find_map(|lane| s.unit(P1, lane).filter(|unit| unit.def_id == TOMMY))
    }

    /// TS `s.unit(p, lane) ?? ""`: the unit's id, or a reference that names nothing.
    fn unit_or_blank(s: &Scenario, player: PlayerId, lane: i32) -> String {
        s.unit(player, lane).map(|unit| unit.id).unwrap_or_default()
    }

    /// TS `s.hand(p)[0] ?? FILLER`.
    fn first_in_hand(s: &Scenario, player: PlayerId) -> String {
        s.hand(player).first().map_or_else(|| FILLER.to_string(), |card| card.id.clone())
    }

    fn cut_short(s: &Scenario) -> bool {
        s.events().iter().any(|event| event.event_type() == GameEventType::TurnCutShort)
    }

    fn has_turn_ends(s: &Scenario, player: PlayerId) -> bool {
        s.state().players[player]
            .mods
            .iter()
            .any(|modifier| matches!(modifier.kind, ModifierKind::TurnEnds { .. }))
    }

    /// The `cardPlayed` event for `id`, if any.
    fn played_event<'a>(s: &'a Scenario, id: Option<&String>) -> Option<&'a GameEvent> {
        s.events().iter().find(|event| {
            matches!(event, GameEvent::CardPlayed { instance_id, .. } if Some(instance_id) == id)
        })
    }

    /// JS `findIndex`: the first position of an event that matches, or −1.
    fn find_index(log: &[GameEvent], matches: impl Fn(&GameEvent) -> bool) -> i64 {
        log.iter()
            .position(matches)
            .map_or(-1, |at| i64::try_from(at).unwrap_or(i64::MAX))
    }

    /// TS `toMatchObject`: every key of `pattern` is in `actual` with a matching value.
    fn matches_object(actual: &Value, pattern: &Value) -> bool {
        match (actual, pattern) {
            (Value::Object(have), Value::Object(want)) => want
                .iter()
                .all(|(key, value)| have.get(key).is_some_and(|got| matches_object(got, value))),
            (Value::Array(have), Value::Array(want)) => {
                have.len() == want.len() && have.iter().zip(want).all(|(got, value)| matches_object(got, value))
            }
            _ => actual == pattern,
        }
    }

    fn round_trip(state: &GameState) -> GameState {
        serde_json::from_value(serde_json::to_value(state).expect("JSON")).expect("a state")
    }

    mod c_n26_tommy_tempo {
        use super::*;

        #[test]
        fn is_a_3_9_9_taunt_human_unit_that_casts_itself_on_draw_on_both_faces() {
            let def = crate::card_def(ID);
            assert_eq!(def.cost, CardCost::Fixed(3));
            assert_eq!(def.base.keywords, vec![Keyword::Taunt]);
            let scripts = script();
            assert_eq!(scripts.base.static_flags.as_ref().and_then(|flags| flags.cast_on_draw), Some(true));
            assert_eq!(scripts.radiant.static_flags.as_ref().and_then(|flags| flags.cast_on_draw), Some(true));
        }

        mod base {
            use super::*;

            #[test]
            fn r70_drawn_at_the_start_of_your_turn_it_is_cast_for_free_into_the_leftmost_open_zone_and_that_turn_ends_before_its_main_phase(
            ) {
                let mut s = drawn_at_start(false, json!({ "field": [{ "def": MENACE, "lane": 1 }] }));
                s.end_turn();
                let tommy = tommy_on_field(&s);
                assert!(tommy.is_some());
                let tommy_id = tommy.as_ref().map(|unit| unit.id.clone());
                assert_eq!(s.unit(P1, 2).map(|unit| unit.id), tommy_id);
                let played = played_event(&s, tommy_id.as_ref());
                assert!(matches!(played, Some(GameEvent::CardPlayed { cost_paid: 0, .. })));
                let cut = s.events().iter().find(|event| event.event_type() == GameEventType::TurnCutShort);
                assert!(matches_object(
                    &serde_json::to_value(cut).expect("JSON"),
                    &json!({ "player": "p1", "byInstanceId": tommy_id }),
                ));
                // p1 never reached a main phase: the turn ended and p2's began.
                assert_eq!(s.state().active, P2);
                let tommy = tommy_id.unwrap_or_default();
                assert!(s.stats(&tommy).keywords.iter().any(|keyword| keyword.kind() == KeywordKind::Taunt));
            }

            #[test]
            fn s6_3_end_the_turn_a_draw_2_draws_its_second_card_and_heals_then_the_turn_ends_with_every_end_of_turn_step() {
                crate::register_all();
                let mut library = vec![json!(TOMMY), json!(FILLER)];
                library.extend(DECK.iter().map(|card| json!(card)));
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [STOCKPILE, FILLER],
                        "library": library,
                        "field": [{ "def": MENACE, "lane": 1, "damage": 4 }],
                        "health": 20,
                    },
                    "p2": { "hand": [FILLER], "library": DECK },
                }));
                let menace = s.unit(P1, 1).expect("setup");
                let second = s.state().players[P1].library.get(1).map(|card| card.id.clone());
                s.play(STOCKPILE, json!({}));
                let log = s.last_events().to_vec();
                let cut = find_index(&log, |event| event.event_type() == GameEventType::TurnCutShort);
                // The second draw and the heal resolve before the turn is cut short.
                assert_eq!(s.card(second.clone().unwrap_or_default()).zone.z(), ZoneName::Hand);
                assert!(
                    cut > find_index(&log, |event| matches!(
                        event,
                        GameEvent::Drawn { instance_id, .. } if Some(instance_id) == second.as_ref()
                    ))
                );
                assert!(
                    cut > find_index(&log, |event| matches!(
                        event,
                        GameEvent::Healed { target_id, .. } if target_id == "hero-p1"
                    ))
                );
                // Every end-of-turn step ran: the Menace healed to full at the end of p1's turn.
                assert_eq!(s.card(&menace).damage, 0);
                assert_eq!(s.state().active, P2);
                s.expect_health(P1, 22);
            }

            #[test]
            fn r415_drawn_on_the_opponent_s_turn_only_the_summon_happens() {
                // p2's Moths to the Flame makes p1's Prem Panther attack it at p2's start of turn; the Panther kills
                // it and survives, so p1 draws 2 on p2's turn — Tommy Tempo first.
                crate::register_all();
                let mut library = vec![json!(TOMMY)];
                library.extend(DECK.iter().map(|card| json!(card)));
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "library": library, "field": [{ "def": PANTHER, "lane": 1 }] },
                    "p2": { "hand": [FILLER, FILLER], "library": DECK, "field": [{ "def": MOTHS, "lane": 3, "damage": 10 }] },
                }));
                s.end_turn();
                let tommy = tommy_on_field(&s);
                assert!(tommy.is_some());
                let tommy_id = tommy.map(|unit| unit.id);
                assert!(matches!(
                    played_event(&s, tommy_id.as_ref()),
                    Some(GameEvent::CardPlayed { cost_paid: 0, .. })
                ));
                // No rider on anybody, nothing cut short, and p2's turn goes on: p2 still plays.
                assert!(!cut_short(&s));
                assert!(!has_turn_ends(&s, P1));
                assert!(!has_turn_ends(&s, P2));
                assert_eq!(s.state().active, P2);
                let card = first_in_hand(&s, P2);
                s.play(card, json!({}));
                assert_eq!(s.state().active, P2);
            }

            #[test]
            fn r560_with_no_open_unit_zone_it_goes_to_the_hand_uncast_and_ends_nothing() {
                let mut s = drawn_at_start(false, json!({ "field": [MENACE, MENACE, MENACE, MENACE, MENACE] }));
                s.end_turn();
                let tommy = s.hand(P1).into_iter().find(|card| card.def_id == TOMMY);
                assert!(tommy.is_some());
                let tommy_id = tommy.map(|card| card.id);
                assert!(played_event(&s, tommy_id.as_ref()).is_none());
                assert!(!cut_short(&s));
                assert_eq!(s.state().active, P1);
                assert_eq!(s.state().phase, Phase::Main);
            }

            #[test]
            fn r560_r317_with_no_open_unit_zone_and_a_full_hand_it_burns() {
                let hand = [FILLER; 10];
                let mut s = drawn_at_start(
                    false,
                    json!({ "field": [MENACE, MENACE, MENACE, MENACE, MENACE], "hand": hand }),
                );
                s.end_turn();
                let burned = s.events().iter().find(|event| event.event_type() == GameEventType::Burned);
                assert!(matches!(burned, Some(GameEvent::Burned { def_id, .. }) if def_id == TOMMY));
                assert!(!cut_short(&s));
            }

            #[test]
            fn played_from_a_hand_it_is_a_plain_unit_and_ends_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [TOMMY, FILLER], "library": DECK },
                    "p2": { "hand": [FILLER], "library": DECK },
                }));
                s.play(TOMMY, json!({}));
                assert!(tommy_on_field(&s).is_some());
                assert!(!cut_short(&s));
                assert!(!has_turn_ends(&s, P1));
                s.play(FILLER, json!({}));
                assert_eq!(s.state().active, P1);
            }

            #[test]
            fn s9_3_the_draw_that_casts_it_and_the_turn_it_cuts_short_replay_from_a_json_copy_to_the_same_hash() {
                let s = drawn_at_start(false, json!({}));
                let action = Action::new(ActionBody::EndTurn, P2, "tommy-replay");
                let thawed = round_trip(s.state());
                let live = reduce(s.state(), &action);
                assert!(live.error.is_none());
                assert!(live.events.iter().any(|event| event.event_type() == GameEventType::TurnCutShort));
                assert_eq!(live.state.active, P2);
                assert_eq!(hash_state(&reduce(&thawed, &action).state), hash_state(&live.state));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r415_drawn_at_the_start_of_your_turn_you_take_one_more_action_and_the_turn_ends_once_it_resolves() {
                let mut s = drawn_at_start(true, json!({}));
                s.end_turn();
                assert_eq!(s.state().active, P1);
                let tommy = tommy_on_field(&s).map(|unit| unit.id).unwrap_or_default();
                s.expect_stats(&tommy, json!({ "attack": 18, "health": 18 }));
                s.play(FILLER, json!({}));
                assert!(cut_short(&s));
                assert_eq!(s.state().active, P2);
            }

            #[test]
            fn r415_drawn_on_the_opponent_s_turn_only_the_summon_happens_no_action_is_counted() {
                crate::register_all();
                let mut library = vec![json!({ "def": TOMMY, "radiant": true })];
                library.extend(DECK.iter().map(|card| json!(card)));
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "library": library, "field": [{ "def": PANTHER, "lane": 1 }] },
                    "p2": { "hand": [FILLER, FILLER], "library": DECK, "field": [{ "def": MOTHS, "lane": 3, "damage": 10 }] },
                }));
                s.end_turn();
                let tommy = tommy_on_field(&s).map(|unit| unit.id).unwrap_or_default();
                s.expect_stats(&tommy, json!({ "attack": 18, "health": 18 }));
                assert!(!has_turn_ends(&s, P1));
                assert!(!has_turn_ends(&s, P2));
                let card = first_in_hand(&s, P2);
                s.play(card, json!({}));
                assert_eq!(s.state().active, P2);
            }

            #[test]
            fn r113_r415_the_one_action_asks_a_question_the_turn_ends_once_it_is_answered_after_a_json_round_trip() {
                let mut s = drawn_at_start(true, json!({ "hand": [SCARAB, FILLER] }));
                s.end_turn();
                s.play(SCARAB, json!({}));
                let pending = s.state().pending.clone();
                assert_eq!(pending.as_ref().map(|prompt| prompt.player_id), Some(P1));
                assert_eq!(s.state().active, P1);
                let thawed = round_trip(s.state());
                assert_eq!(&thawed, s.state());
                let pending = pending.expect("no Discover");
                let choice = pending.options.first().map(|option| option.selection.clone()).expect("no Discover");
                let action = Action::new(
                    ActionBody::Answer { choice_id: pending.id.clone(), selection: vec![choice] },
                    P1,
                    "tommy-answer",
                );
                let live = reduce(s.state(), &action);
                let frozen = reduce(&thawed, &action);
                assert!(live.error.is_none());
                assert_eq!(live.state.active, P2);
                assert!(live.events.iter().any(|event| event.event_type() == GameEventType::TurnCutShort));
                assert_eq!(hash_state(&frozen.state), hash_state(&live.state));
            }

            #[test]
            fn r415_an_attack_is_the_action() {
                let mut s = drawn_at_start(true, json!({ "field": [{ "def": MENACE, "lane": 1 }] }));
                s.end_turn();
                let attacker = unit_or_blank(&s, P1, 1);
                s.attack(attacker, "hero");
                assert_eq!(s.state().active, P2);
            }

            #[test]
            fn r415_a_position_switch_is_the_action() {
                let mut s = drawn_at_start(true, json!({ "field": [{ "def": MENACE, "lane": 1 }] }));
                s.end_turn();
                let unit = unit_or_blank(&s, P1, 1);
                s.switch_position(unit);
                assert_eq!(s.state().active, P2);
            }

            #[test]
            fn r415_an_activation_is_the_action() {
                let mut s = drawn_at_start(true, json!({ "field": [{ "def": NOSE, "lane": 1 }], "hand": [FILLER, FILLER] }));
                s.end_turn();
                let unit = unit_or_blank(&s, P1, 1);
                s.activate(unit, json!({}));
                assert!(s.events().iter().any(|event| event.event_type() == GameEventType::Activated));
                assert_eq!(s.state().active, P2);
            }

            #[test]
            fn r415_ending_the_turn_yourself_uses_it() {
                let mut s = drawn_at_start(true, json!({}));
                s.end_turn();
                s.end_turn();
                assert_eq!(s.state().active, P2);
                assert_eq!(
                    s.events().iter().filter(|event| event.event_type() == GameEventType::TurnCutShort).count(),
                    0
                );
            }

            #[test]
            fn r386_the_action_count_reads_through_param_an_upgrade_leaves_two_actions() {
                let mut s = drawn_at_start(true, json!({ "hand": [FILLER, FILLER] }));
                let tommy = s.state().players[P1].library.first().map(|card| card.id.clone()).expect("setup");
                step_param(find_instance_mut(s.state_mut(), &tommy).expect("setup"), "actions", 1);
                s.end_turn();
                s.play(FILLER, json!({}));
                assert_eq!(s.state().active, P1);
                s.play(FILLER, json!({}));
                assert_eq!(s.state().active, P2);
            }

            #[test]
            fn r113_the_rider_is_state_the_turn_waiting_on_its_last_action_survives_json() {
                let mut s = drawn_at_start(true, json!({}));
                s.end_turn();
                let rider = s.state().players[P1]
                    .mods
                    .iter()
                    .find(|modifier| matches!(modifier.kind, ModifierKind::TurnEnds { .. }));
                assert!(matches!(rider.map(|modifier| &modifier.kind), Some(ModifierKind::TurnEnds { actions_left: 1, .. })));
                assert_eq!(&round_trip(s.state()), s.state());
            }
        }
    }
}
