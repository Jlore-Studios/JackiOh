//! I2's Windfury shadow (R636): the oracle reads Windfury from the event stream, so a grant that
//! lands mid-action counts before the declarations that follow it in the same action's events.
//! Found by fuzz seed 359: R44's AI turn summoned a Conjure Rush Token, granted it Windfury among
//! six keywords, and fought with it twice, all inside one outer action — no between-action state
//! ever held the token, so the old oracle read attack number 2 as an extra attack.
//!
//! I6 (hidden information, issue #348) is proved by feeding the oracle a view or an event that names a
//! card the seat may not read (a hidden definition beside any id, a former id, R227, or the seat's
//! own library card it was never shown, R312), and by the exceptions §10.8 and its rulings make: a face-down trap's
//! bare id as a target (R177) and a `stolen` event naming a card its viewer could read where it was
//! taken (R466).
//!
//! Port of `packages/cards/test/invariants.test.ts` (SURFACE §4.1, §8).

use jackioh_engine::testkit::*;

const VANILLA: &str = "core-008";
const BIGOT: &str = "core-002";
const HIT_JOB: &str = "core-016";
const MY_PAWN: &str = "core-096";
const SHEEPISH: &str = "core-041";
const RUSH_TOKEN: &str = "core-t-rush";

use super::scenario;

/// An event from TS's object literal.
fn event(literal: Value) -> GameEvent {
    json_as(literal)
}

/// `g.unit(player, lane)`: the top of the unit pile in that lane, which the setup must have put there.
fn unit_at(g: &Scenario, player: PlayerId, lane: usize, what: &str) -> CardInstance {
    match g.state().players[player]
        .units
        .get(lane - 1)
        .and_then(|pile| pile.as_ref())
        .and_then(|pile| pile.first())
    {
        Some(card) => card.clone(),
        None => panic!("setup: {what}"),
    }
}

/// `g.backrow(player, lane)`: the card in that backrow zone, which the setup must have put there.
fn backrow_at(g: &Scenario, player: PlayerId, lane: usize, what: &str) -> CardInstance {
    match g.state().players[player]
        .backrow
        .get(lane - 1)
        .and_then(|card| card.as_ref())
    {
        Some(card) => card.clone(),
        None => panic!("setup: {what}"),
    }
}

fn no_violations() -> Vec<String> {
    Vec::new()
}

mod i2_a_windfury_granted_inside_one_action_s_events {
    use super::*;

    #[test]
    fn r636_r44_two_declarations_after_a_mid_action_windfury_grant_are_one_legal_windfury_turn() {
        let mut g = scenario(json!({
            "p1": { "field": [{ "def": VANILLA, "lane": 1 }] },
            "p2": { "field": [{ "def": VANILLA, "lane": 1 }] },
        }));
        let unit = unit_at(&g, PlayerId::P1, 1, "p1 should hold a unit in lane 1");
        let prey = unit_at(&g, PlayerId::P2, 1, "p2 should hold a unit in lane 1");
        // The end state holds Windfury, as if the grant below resolved mid-action.
        find_instance_mut(g.state_mut(), &unit.id)
            .expect("the unit")
            .granted_keywords = vec![json_as::<Keyword>(json!({ "kind": "Windfury" }))];

        let mut monitor = create_invariant_monitor(g.state());
        let events = vec![
            event(
                json!({ "type": "keywordGranted", "instanceId": unit.id, "keyword": { "kind": "Windfury" } }),
            ),
            event(
                json!({ "type": "attackDeclared", "attackerId": unit.id, "targetId": prey.id, "forced": false }),
            ),
            event(
                json!({ "type": "attackDeclared", "attackerId": unit.id, "targetId": prey.id, "forced": false }),
            ),
        ];
        assert_eq!(monitor.after(&events, g.state()), no_violations());
    }

    #[test]
    fn r636_two_declarations_with_no_windfury_anywhere_are_still_an_extra_attack() {
        let g = scenario(json!({
            "p1": { "field": [{ "def": VANILLA, "lane": 1 }] },
            "p2": { "field": [{ "def": VANILLA, "lane": 1 }] },
        }));
        let unit = unit_at(&g, PlayerId::P1, 1, "p1 should hold a unit in lane 1");
        let prey = unit_at(&g, PlayerId::P2, 1, "p2 should hold a unit in lane 1");

        let mut monitor = create_invariant_monitor(g.state());
        let events = vec![
            event(
                json!({ "type": "attackDeclared", "attackerId": unit.id, "targetId": prey.id, "forced": false }),
            ),
            event(
                json!({ "type": "attackDeclared", "attackerId": unit.id, "targetId": prey.id, "forced": false }),
            ),
        ];
        let found = monitor.after(&events, g.state());
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("I2 extra attack"), "{}", found[0]);
    }
}

mod i6_hidden_information_in_what_each_seat_is_sent {
    use super::*;

    /// p1 sees a unit and p2 holds a hand card, a library card and a face-down trap.
    struct Board {
        g: Scenario,
        unit: CardInstance,
        secret: CardInstance,
        trap: CardInstance,
    }

    fn board() -> Board {
        let g = scenario(json!({
            "p1": { "field": [{ "def": VANILLA, "lane": 1 }] },
            "p2": { "hand": [BIGOT], "library": [HIT_JOB], "backrow": [{ "def": MY_PAWN, "lane": 1 }] },
        }));
        let unit = unit_at(&g, PlayerId::P1, 1, "p1 should hold a unit in lane 1");
        let secret = match g.state().players.p2.hand.first() {
            Some(card) => card.clone(),
            None => panic!("setup: p2 should hold a card in hand"),
        };
        let trap = backrow_at(&g, PlayerId::P2, 1, "p2 should hold a face-down trap in lane 1");
        Board {
            g,
            unit,
            secret,
            trap,
        }
    }

    fn violations(g: &Scenario, seat: PlayerId, events: &[GameEvent]) -> Vec<String> {
        let mut view = view_for(g.state(), seat);
        view.events.extend(events.iter().cloned());
        hidden_information_violations(g.state(), seat, &view, &legal_actions(g.state(), seat))
    }

    fn drawn(player: &str, instance_id: &str, def_id: &str) -> GameEvent {
        event(json!({ "type": "drawn", "player": player, "instanceId": instance_id, "defId": def_id }))
    }

    fn quoted(text: &str) -> String {
        format!("\"{text}\"")
    }

    #[test]
    fn i6_is_clean_on_a_board_with_a_hidden_hand_library_and_face_down_trap_from_both_seats() {
        // §10.8
        let Board { g, .. } = board();
        assert_eq!(violations(&g, PlayerId::P1, &[]), no_violations());
        assert_eq!(violations(&g, PlayerId::P2, &[]), no_violations());
        assert_eq!(
            create_invariant_monitor(g.state()).hidden(g.state()),
            no_violations()
        );
    }

    #[test]
    fn r97_i6_fires_on_an_event_naming_a_hidden_hand_card_by_id_and_definition() {
        let Board { g, secret, .. } = board();
        let found = violations(&g, PlayerId::P1, &[drawn("p2", &secret.id, &secret.def_id)]);
        assert!(found.iter().all(|message| message.starts_with("I6")), "{found:?}");
        assert!(
            found.iter().any(|message| message.contains(&quoted(&secret.id))),
            "{found:?}"
        );
        assert!(
            found
                .iter()
                .any(|message| message.contains(&quoted(&secret.def_id))),
            "{found:?}"
        );
    }

    #[test]
    fn r97_i6_fires_on_a_hidden_card_s_definition_left_beside_the_sentinel() {
        let Board { g, secret, .. } = board();
        let found = violations(&g, PlayerId::P1, &[drawn("p2", HIDDEN_ID, &secret.def_id)]);
        assert!(
            found
                .iter()
                .any(|message| message.contains(&quoted(&secret.def_id))),
            "{found:?}"
        );
        assert!(
            !found.iter().any(|message| message.contains(&quoted(&secret.id))),
            "{found:?}"
        );
    }

    #[test]
    fn r97_i6_fires_on_a_hidden_definition_beside_an_id_that_vouches_for_nothing_made_up_a_hero_s_or_another_card_s()
     {
        let Board {
            g,
            unit,
            secret,
            trap,
        } = board();
        for instance_id in ["c900", "hero-p2", unit.id.as_str()] {
            for def_id in [secret.def_id.as_str(), trap.def_id.as_str()] {
                let found = violations(&g, PlayerId::P1, &[drawn("p2", instance_id, def_id)]);
                assert!(
                    found.iter().any(|message| message.contains(&quoted(def_id))),
                    "{instance_id} {def_id}: {found:?}"
                );
            }
        }
    }

    #[test]
    fn r227_i6_judges_a_former_id_by_the_card_it_became_a_face_down_trap_s_old_id_is_hidden_like_the_trap() {
        let Board { mut g, trap, .. } = board();
        g.state_mut().applied = vec![AppliedAction {
            nonce: "i6-former".into(),
            events: vec![event(json!({
                "type": "controlChanged", "instanceId": trap.id, "controller": "p2", "row": "backrow", "lane": 1,
                "formerId": "c-former",
            }))],
        }];
        assert_eq!(violations(&g, PlayerId::P1, &[]), no_violations());
        let by_id = violations(&g, PlayerId::P1, &[drawn("p2", "c-former", HIDDEN_ID)]);
        assert!(
            by_id.iter().any(|message| message.contains(&quoted("c-former"))),
            "{by_id:?}"
        );
        let with_def = violations(&g, PlayerId::P1, &[drawn("p2", "c-former", &trap.def_id)]);
        assert!(
            with_def
                .iter()
                .any(|message| message.contains(&quoted(&trap.def_id))),
            "{with_def:?}"
        );
    }

    #[test]
    fn r419_r227_i6_lets_a_card_rollback_recreated_carry_its_old_id_once_it_reads_though_it_was_transformed_away_unseen()
     {
        let Board {
            mut g, unit, trap, ..
        } = board();
        g.state_mut().applied = vec![AppliedAction {
            nonce: "i6-rollback".into(),
            events: vec![
                event(json!({
                    "type": "transformed", "instanceId": "c-old", "fromDefId": trap.def_id, "toDefId": VANILLA,
                    "newInstanceId": unit.id, "hiddenFrom": ["p1"],
                })),
                event(json!({
                    "type": "controlChanged", "instanceId": unit.id, "controller": "p1", "row": "units", "lane": 1,
                    "formerId": "c-old",
                })),
            ],
        }];
        let sent =
            serde_json::to_string(&view_for(g.state(), PlayerId::P1).events).expect("events serialise");
        assert!(sent.contains("c-old"), "{sent}");
        assert_eq!(violations(&g, PlayerId::P1, &[]), no_violations());
    }

    #[test]
    fn r311_r312_i6_reads_the_seat_s_own_library_only_as_it_was_shown_going_in() {
        let mut g = scenario(json!({ "p1": { "library": [SHEEPISH] } }));
        assert!(
            !g.state().players.p1.library.is_empty(),
            "setup: p1 should hold a library card"
        );
        let shown = drawn("p1", HIDDEN_ID, SHEEPISH);
        assert_eq!(
            violations(&g, PlayerId::P1, std::slice::from_ref(&shown)),
            no_violations()
        );

        // A card its owner was never shown (Pocket Chaos #87, Transmogulate #83) is an unknown count.
        g.state_mut().players.p1.library[0].known_as = None;
        assert_eq!(violations(&g, PlayerId::P1, &[]), no_violations());
        let found = violations(&g, PlayerId::P1, &[shown]);
        assert!(
            found.iter().any(|message| message.contains(&quoted(SHEEPISH))),
            "{found:?}"
        );
        let mut listed = view_for(g.state(), PlayerId::P1);
        listed.you.own_library = Some(LibraryView {
            cards: vec![LibraryEntryView {
                def_id: SHEEPISH.into(),
                radiant: false,
                count: 1,
                created: None,
            }],
            unknown: 0,
        });
        let found = hidden_information_violations(
            g.state(),
            PlayerId::P1,
            &listed,
            &legal_actions(g.state(), PlayerId::P1),
        );
        assert!(
            found.iter().any(|message| message.contains(&quoted(SHEEPISH))),
            "{found:?}"
        );
    }

    #[test]
    fn r177_i6_lets_a_face_down_trap_through_legal_actions_as_a_bare_target_and_fires_on_a_hidden_hand_card()
    {
        let Board {
            g,
            unit,
            secret,
            trap,
        } = board();
        let view = view_for(g.state(), PlayerId::P1);
        let offer = |target_id: &str| -> Vec<ActionBody> {
            vec![ActionBody::Attack {
                attacker_id: unit.id.clone(),
                target_id: target_id.to_string(),
            }]
        };
        assert_eq!(
            hidden_information_violations(g.state(), PlayerId::P1, &view, &offer(&trap.id)),
            no_violations()
        );
        let found = hidden_information_violations(g.state(), PlayerId::P1, &view, &offer(&secret.id));
        assert!(
            found.iter().any(|message| message.contains("in legalActions")),
            "{found:?}"
        );
    }

    /// p1's own target prompt offering p2's face-down trap, as #49, #50 and an Echo repeat do (R177).
    fn offer_trap(g: &mut Scenario, trap_id: &str) -> PendingChoice {
        let prompt: PendingChoice = json_as(json!({
            "id": "i6-prompt",
            "playerId": "p1",
            "kind": "target",
            "prompt": "Choose a target",
            "options": [{
                "key": format!("instance:{trap_id}"),
                "label": "a trap",
                "selection": { "pick": "instance", "instanceId": trap_id },
            }],
            "min": 1,
            "max": 1,
            "resume": { "defId": "", "hook": "resume", "step": "none", "radiant": false, "data": {} },
        }));
        g.state_mut().pending = Some(prompt.clone());
        prompt
    }

    #[test]
    fn r177_i6_lets_p1_s_own_prompt_offer_a_face_down_trap_by_its_bare_id_and_still_fires_on_any_other_mention()
     {
        let Board { mut g, trap, .. } = board();
        offer_trap(&mut g, &trap.id);
        assert_eq!(violations(&g, PlayerId::P1, &[]), no_violations());
        // The id outside the option, or the definition anywhere, is a leak the prompt does not excuse.
        let found = violations(&g, PlayerId::P1, &[drawn("p2", &trap.id, &trap.def_id)]);
        assert!(
            found.iter().any(|message| message.contains(&quoted(&trap.id))),
            "{found:?}"
        );
        assert!(
            found
                .iter()
                .any(|message| message.contains(&quoted(&trap.def_id))),
            "{found:?}"
        );
    }

    #[test]
    fn r177_i6_fires_on_a_prompt_option_that_carries_a_face_down_trap_s_definition() {
        let Board { mut g, trap, .. } = board();
        offer_trap(&mut g, &trap.id);
        let mut leaky = view_for(g.state(), PlayerId::P1);
        match leaky.pending.as_mut() {
            Some(PendingView::ForYou(pending)) => {
                for option in pending.options.iter_mut() {
                    option.def_id = Some(trap.def_id.clone());
                    option.label = "a trap".into();
                }
            }
            _ => panic!("setup: p1 should hold the prompt"),
        }
        let found = hidden_information_violations(
            g.state(),
            PlayerId::P1,
            &leaky,
            &legal_actions(g.state(), PlayerId::P1),
        );
        assert!(
            found
                .iter()
                .any(|message| message.contains(&quoted(&trap.def_id))),
            "{found:?}"
        );
    }

    #[test]
    fn r97_i6_fires_on_an_event_naming_a_face_down_trap_dormant_under_a_backrow_top() {
        // B5 E21
        let g = scenario(json!({
            "p2": {
                "backrow": [
                    { "def": SHEEPISH, "lane": 1 },
                    { "def": MY_PAWN, "lane": 1, "stack": true },
                ],
            },
        }));
        let dormant = match g
            .state()
            .players
            .p2
            .backrow_piles
            .as_ref()
            .and_then(|piles| piles.first())
            .and_then(|pile| pile.first())
        {
            Some(card) => card.clone(),
            None => panic!("setup: p2 should hold a dormant trap under lane 1's top"),
        };
        assert_eq!(violations(&g, PlayerId::P1, &[]), no_violations());
        let found = violations(&g, PlayerId::P1, &[drawn("p2", &dormant.id, &dormant.def_id)]);
        assert!(
            found.iter().any(|message| message.contains(&quoted(&dormant.id))),
            "{found:?}"
        );
        assert!(
            found
                .iter()
                .any(|message| message.contains(&quoted(&dormant.def_id))),
            "{found:?}"
        );
    }

    #[test]
    fn r466_i6_lets_a_stolen_event_name_a_card_its_viewer_could_read_where_it_was_taken() {
        let Board { mut g, secret, .. } = board();
        g.state_mut().applied = vec![AppliedAction {
            nonce: "i6-stolen".into(),
            events: vec![event(json!({
                "type": "stolen", "instanceId": secret.id, "defId": secret.def_id, "from": "p1", "to": "p2",
                "zone": "hand", "readableFrom": ["p1"],
            }))],
        }];
        let sent =
            serde_json::to_string(&view_for(g.state(), PlayerId::P1).events).expect("events serialise");
        assert!(sent.contains(&secret.id), "{sent}");
        assert_eq!(violations(&g, PlayerId::P1, &[]), no_violations());
    }

    /// p2 sets My Pawn face-down and holds a card, p1 ends the turn and plays Glitch, once per seed until
    /// it draws `outcome` (R676): the cards it takes off the board or out of the match go unseen.
    fn glitched(outcome: GlitchOutcome) -> Scenario {
        for n in 0..300 {
            let mut g = scenario(json!({
                "seed": format!("i6-glitch-{outcome}-{n}"),
                "active": "p2",
                "turn": 8,
                "p1": { "hand": [GLITCH_DEF_ID], "mana": 0 },
                "p2": { "hand": [MY_PAWN, HIT_JOB], "library": [HIT_JOB], "mana": 5 },
                "glitchBoards": [[], []],
            }));
            g.play(MY_PAWN, json!({}));
            g.end_turn();
            g.play(GLITCH_DEF_ID, json!({}));
            let drew = g
                .last_events()
                .iter()
                .any(|e| matches!(e, GameEvent::Glitched { outcome: got, .. } if *got == outcome));
            if drew {
                return g;
            }
        }
        panic!("no seed draws {outcome}");
    }

    /// The instance ids the log's raw events pair with a definition, which no pile holds once a Glitch has taken them.
    fn ids_naming(g: &Scenario, def_id: &str) -> Vec<String> {
        g.state()
            .applied
            .iter()
            .flat_map(|entry| entry.events.iter())
            .filter_map(|event| {
                let value = serde_json::to_value(event).expect("events serialise");
                match (
                    value.get("instanceId").and_then(Value::as_str),
                    value.get("defId").and_then(Value::as_str),
                ) {
                    (Some(instance_id), Some(named)) if named == def_id => Some(instance_id.to_string()),
                    _ => None,
                }
            })
            .collect()
    }

    /// What an engine that left the log's events unredacted would send p1.
    fn unredacted(g: &Scenario) -> PlayerView {
        let mut view = view_for(g.state(), PlayerId::P1);
        view.events = g
            .state()
            .applied
            .iter()
            .flat_map(|entry| entry.events.iter().cloned())
            .collect();
        view
    }

    #[test]
    fn r678_r764_i6_fires_on_a_face_down_trap_a_boards_glitch_took_named_openly_in_the_events_before_it() {
        let g = glitched(GlitchOutcome::Boards);
        let found = hidden_information_violations(
            g.state(),
            PlayerId::P1,
            &unredacted(&g),
            &legal_actions(g.state(), PlayerId::P1),
        );
        let ids = ids_naming(&g, MY_PAWN);
        assert!(
            found.iter().any(
                |message| message.starts_with("I6") && ids.iter().any(|id| message.contains(&quoted(id)))
            ),
            "{found:?}"
        );
        // The engine's own view is clean (R764), and so is the monitor over both seats.
        assert_eq!(
            hidden_information_violations(
                g.state(),
                PlayerId::P1,
                &view_for(g.state(), PlayerId::P1),
                &legal_actions(g.state(), PlayerId::P1)
            ),
            no_violations()
        );
        assert_eq!(
            create_invariant_monitor(g.state()).hidden(g.state()),
            no_violations()
        );
    }

    #[test]
    fn r676_r764_i6_fires_on_a_card_the_other_seat_drew_in_a_game_a_glitch_s_reset_took_named_openly_after_it()
     {
        let g = glitched(GlitchOutcome::Reset);
        let found = hidden_information_violations(
            g.state(),
            PlayerId::P1,
            &unredacted(&g),
            &legal_actions(g.state(), PlayerId::P1),
        );
        let ids = ids_naming(&g, HIT_JOB);
        assert!(
            found.iter().any(
                |message| message.starts_with("I6") && ids.iter().any(|id| message.contains(&quoted(id)))
            ),
            "{found:?}"
        );
        assert_eq!(
            hidden_information_violations(
                g.state(),
                PlayerId::P1,
                &view_for(g.state(), PlayerId::P1),
                &legal_actions(g.state(), PlayerId::P1)
            ),
            no_violations()
        );
        assert_eq!(
            create_invariant_monitor(g.state()).hidden(g.state()),
            no_violations()
        );
    }

    #[test]
    fn r11_i6_lets_a_token_that_ceased_to_exist_be_named_openly_as_it_was_public_when_it_went() {
        let mut g = scenario(json!({ "p2": { "hand": [RUSH_TOKEN, RUSH_TOKEN] } }));
        let gone = match g.state().players.p2.hand.first() {
            Some(card) => card.clone(),
            None => panic!("setup: p2 should hold the tokens"),
        };
        g.state_mut().applied = vec![AppliedAction {
            nonce: "i6-token".into(),
            events: vec![event(json!({
                "type": "drawn", "player": "p2", "instanceId": gone.id, "defId": gone.def_id, "turnDraw": 1,
            }))],
        }];
        cease_to_exist(g.state_mut(), &mut gone.clone());
        let mut view = view_for(g.state(), PlayerId::P1);
        view.events = g
            .state()
            .applied
            .iter()
            .flat_map(|entry| entry.events.iter().cloned())
            .collect();
        let sent = serde_json::to_string(&view.events).expect("events serialise");
        assert!(sent.contains(&gone.id), "{sent}");
        assert_eq!(
            hidden_information_violations(
                g.state(),
                PlayerId::P1,
                &view,
                &legal_actions(g.state(), PlayerId::P1)
            ),
            no_violations()
        );
    }
}
