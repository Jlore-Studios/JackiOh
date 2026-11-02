//! Invariants I2 (R44, R386, R636, #874), I6 hidden information (SPEC §10.8, issue #348, R177, R227,
//! R312, R466), and I8 (issues #1192, #1574). Test paths: SURFACE §4.1, §8.

use jackioh_engine::testkit::*;

const VANILLA: &str = "core-008";
const BIGOT: &str = "core-002";
const HIT_JOB: &str = "core-016";
const MY_PAWN: &str = "core-096";
const SHEEPISH: &str = "core-041";
const RUSH_TOKEN: &str = "core-t-rush";
const APPROPRIATIONS: &str = "classicplus-040";

use super::{craft_fused, fixture, in_hand, scenario};

fn event(literal: Value) -> GameEvent {
    json_as(literal)
}

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
        let unit = g.unit_at(PlayerId::P1, 1);
        let prey = g.unit_at(PlayerId::P2, 1);
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
        let unit = g.unit_at(PlayerId::P1, 1);
        let prey = g.unit_at(PlayerId::P2, 1);

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

    /// p1's unit with `gained` in its Buff record (R386), the end state of a Buff that resolved
    /// mid-action, and a fresh monitor; then one Buff event moving `change` and two declarations.
    fn buffed_twice_declared(gained: &[&str], change: Value) -> Vec<String> {
        let mut g = scenario(json!({
            "p1": { "field": [{ "def": VANILLA, "lane": 1 }] },
            "p2": { "field": [{ "def": VANILLA, "lane": 1 }] },
        }));
        let unit = g.unit_at(PlayerId::P1, 1);
        let prey = g.unit_at(PlayerId::P2, 1);
        let add_keywords: Vec<Value> = gained.iter().map(|kind| json!({ "kind": kind })).collect();
        find_instance_mut(g.state_mut(), &unit.id)
            .expect("the unit")
            .tuning = Some(json_as(json!({ "addKeywords": add_keywords })));

        let mut monitor = create_invariant_monitor(g.state());
        let events = vec![
            event(json!({ "type": "upgraded", "instanceId": unit.id, "defId": VANILLA, "change": change })),
            event(
                json!({ "type": "attackDeclared", "attackerId": unit.id, "targetId": prey.id, "forced": false }),
            ),
            event(
                json!({ "type": "attackDeclared", "attackerId": unit.id, "targetId": prey.id, "forced": false }),
            ),
        ];
        monitor.after(&events, g.state())
    }

    /// #874: My Pawn canceled an attack (R44), the random rest of the turn cast CN Peptides on the
    /// attacker, whose Buffs gave it Windfury, and it attacked again, all inside the one outer action. A
    /// unit that gains Windfury after one attack may attack once more (R636), and a Buff's keyword row is
    /// a gain like a grant.
    #[test]
    fn r636_r386_two_declarations_after_a_mid_action_buff_into_windfury_are_one_legal_windfury_turn() {
        let change = json!({ "kind": "keyword", "keyword": { "kind": "Windfury" }, "added": true });
        assert_eq!(buffed_twice_declared(&["Windfury"], change), no_violations());
    }

    /// R1160: a multiplied Buff reports the keywords it added beyond the first as `also`.
    #[test]
    fn r636_r1160_a_multiplied_buff_that_adds_windfury_among_its_also_keywords_counts_too() {
        let change = json!({
            "kind": "keyword",
            "keyword": { "kind": "Ranged" },
            "added": true,
            "also": [{ "kind": "Windfury" }],
        });
        assert_eq!(
            buffed_twice_declared(&["Ranged", "Windfury"], change),
            no_violations()
        );
    }

    #[test]
    fn r636_r386_a_buff_into_another_keyword_leaves_the_second_declaration_an_extra_attack() {
        let change = json!({ "kind": "keyword", "keyword": { "kind": "Ranged" }, "added": true });
        let found = buffed_twice_declared(&["Ranged"], change);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("I2 extra attack"), "{}", found[0]);
    }
}

/// I2 and a unit that enters inside one action already holding Windfury: the autopilot (R1183002)
/// (Rampaging Rhino), #1216.
mod i2_a_windfury_unit_that_enters_inside_one_action {
    use super::*;

    const RHINO: &str = "meditative-002";

    fn entered_and_declared_twice(unit: &CardInstance) -> Vec<GameEvent> {
        let declared = json!({ "type": "attackDeclared", "attackerId": unit.id, "targetId": "hero-p2", "forced": false });
        vec![
            event(json!({
                "type": "cardPlayed", "player": "p1", "instanceId": unit.id, "defId": unit.def_id, "costPaid": 0,
            })),
            event(json!({
                "type": "summoned", "player": "p1", "instanceId": unit.id, "defId": unit.def_id,
                "row": "units", "lane": 1,
            })),
            event(declared.clone()),
            event(declared),
        ]
    }

    /// p1's hand holds a Vanilla, Buffed into `gained` there when given (R386).
    fn hand_card(gained: Option<&str>) -> (Scenario, CardInstance) {
        let mut g = scenario(json!({ "p1": { "hand": [VANILLA] } }));
        let card = g.state().players.p1.hand[0].clone();
        if let Some(kind) = gained {
            find_instance_mut(g.state_mut(), &card.id)
                .expect("the hand card")
                .tuning = Some(json_as(json!({ "addKeywords": [{ "kind": kind }] })));
        }
        (g, card)
    }

    #[test]
    fn r636_r1183002_a_windfury_card_from_the_hand_that_enters_and_declares_twice_in_one_action_is_one_legal_windfury_turn()
     {
        let (mut g, card) = hand_card(Some("Windfury"));
        let mut monitor = create_invariant_monitor(g.state());
        assert_eq!(
            monitor.before(g.state(), PlayerId::P1, &ActionBody::EndTurn),
            no_violations()
        );
        g.state_mut().players.p1.hand.clear();
        let events = entered_and_declared_twice(&card);
        assert_eq!(monitor.after(&events, g.state()), no_violations());
    }

    #[test]
    fn r636_a_windfury_unit_made_inside_the_action_is_read_where_the_action_left_it() {
        let mut g = scenario(json!({ "p1": { "field": [{ "def": RHINO, "lane": 1 }] } }));
        let rhino = g.unit_at(PlayerId::P1, 1);
        let turn = g.state().turn;
        find_instance_mut(g.state_mut(), &rhino.id)
            .expect("the Rhino")
            .summoned_turn = Some(turn);
        let mut monitor = create_invariant_monitor(g.state());
        let events = entered_and_declared_twice(&rhino);
        assert_eq!(monitor.after(&events, g.state()), no_violations());
    }

    #[test]
    fn r636_a_unit_with_no_windfury_that_enters_and_declares_twice_in_one_action_is_still_an_extra_attack() {
        let (g, card) = hand_card(None);
        let mut monitor = create_invariant_monitor(g.state());
        assert_eq!(
            monitor.before(g.state(), PlayerId::P1, &ActionBody::EndTurn),
            no_violations()
        );
        let found = monitor.after(&entered_and_declared_twice(&card), g.state());
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("I2 extra attack"), "{}", found[0]);
    }
}

mod i6_hidden_information_in_what_each_seat_is_sent {
    use super::*;

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
        let unit = g.unit_at(PlayerId::P1, 1);
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
            serde_json::to_string(&view_for(g.state(), PlayerId::P1).events).expect("events serialize");
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

    /// p1's own craft prompt offering one recipe (R880): the option key names the recipe's
    /// definition, which p1 reads with the offered recipe (§10.8).
    fn offer_craft(g: &mut Scenario, recipe: &CraftRecipe) {
        let prompt: PendingChoice = json_as(json!({
            "id": "i6-craft-prompt",
            "playerId": "p1",
            "kind": "craft",
            "prompt": "Choose a recipe",
            "options": [{
                "key": option_key(&Selection::Craft { recipe: recipe.clone() }),
                "label": recipe.name(),
                "selection": { "pick": "craft", "recipe": recipe },
            }],
            "min": 1,
            "max": 1,
            "resume": { "defId": "", "hook": "resume", "step": "none", "radiant": false, "data": {} },
        }));
        g.state_mut().pending = Some(prompt);
    }

    #[test]
    fn r880_i6_lets_p1_s_own_craft_prompt_name_the_definition_of_p2_s_hidden_hand_card() {
        // Fuzz seed 1708 (#1216): p2's hand holds a card crafted from the same recipe p1's
        // prompt offers, so the option key and the hidden card share one `craft:` definition.
        let Board { mut g, secret, .. } = board();
        let recipe: CraftRecipe = json_as(json!({
            "cost": 1, "type": "Unit", "adjective": "Pure", "noun": "Closure",
            "attack": 2, "health": 3, "keywords": [], "echo": 0, "hats": [],
        }));
        let def = jackioh_engine::subsystems::craft::craft_id(&recipe);
        assert!(def.starts_with("craft:"), "{def}");
        g.state_mut().players.p2.hand[0].def_id = def.clone();
        offer_craft(&mut g, &recipe);
        assert_eq!(violations(&g, PlayerId::P1, &[]), no_violations());
        let found = violations(&g, PlayerId::P1, &[drawn("p2", &secret.id, &secret.def_id)]);
        assert!(
            found.iter().any(|message| message.contains(&quoted(&secret.id))),
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
            serde_json::to_string(&view_for(g.state(), PlayerId::P1).events).expect("events serialize");
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

    fn ids_naming(g: &Scenario, def_id: &str) -> Vec<String> {
        g.state()
            .applied
            .iter()
            .flat_map(|entry| entry.events.iter())
            .filter_map(|event| {
                let value = serde_json::to_value(event).expect("events serialize");
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
        let sent = serde_json::to_string(&view.events).expect("events serialize");
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

    #[test]
    fn r316_i6_lets_a_card_a_full_library_refused_before_it_existed_be_named_openly_when_it_copies_nothing() {
        // Fuzz seed 5967, handicapped (#562): Classic+ #40 Appropriations' Education into a full deck.
        // Its random Books are never created (R80) and copy no card, so both seats read them (R316):
        // they were never in any pile. A Book is no token, so R11's rule above does not cover them.
        let library: Vec<&str> = (0..LIBRARY_CAP).map(|_| VANILLA).collect();
        let mut g = scenario(json!({ "p1": { "hand": [APPROPRIATIONS], "library": library } }));
        g.play(APPROPRIATIONS, json!({ "x": 2, "modes": ["Education"] }));
        let refused: Vec<String> = g
            .last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::LibraryOverflow {
                    instance_id,
                    outcome: LibraryOverflowOutcome::NotCreated,
                    copy_of: None,
                    ..
                } => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        // 2X − 1 Books at X = 2 (#1704).
        assert_eq!(refused.len(), 3);
        for seat in [PlayerId::P1, PlayerId::P2] {
            let sent = serde_json::to_string(&view_for(g.state(), seat).events).expect("events serialize");
            for id in &refused {
                assert!(sent.contains(id.as_str()), "{seat}: {sent}");
            }
        }
        assert_eq!(
            create_invariant_monitor(g.state()).hidden(g.state()),
            no_violations()
        );
    }
}

/// R1062: I1 and I4 stay clean as Fiery Waraxe animates again and attacks the hero.
mod r1062_fiery_waraxe_reanimated {
    use super::*;

    #[test]
    fn r1062_i1_and_i4_stay_clean_as_fiery_waraxe_animates_again_and_attacks_the_hero() {
        let mut g = scenario(json!({
            "p1": { "hand": ["meditative-066", "core-005"] },
            "p2": { "hand": ["core-005"] },
        }));
        let mut monitor = create_invariant_monitor(g.state());
        g.play("meditative-066", json!({ "zone": 1 }));
        assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());
        g.end_turn();
        assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());
        g.end_turn();
        assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());
        let waraxe = g.unit(PlayerId::P1, 1).expect("the waraxe").id.clone();
        let attack = ActionBody::Attack {
            attacker_id: waraxe.clone(),
            target_id: "hero-p2".to_string(),
        };
        assert_eq!(monitor.before(g.state(), PlayerId::P1, &attack), no_violations());
        g.attack(&waraxe, "hero");
        assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());
    }

    /// The animation entered nothing (R1062), so I4 reads the keyword as the animation had it, off the
    /// change the later event names, and not off the state the action left.
    #[test]
    fn r1062_i4_reads_animated_on_your_turn_as_the_animation_had_it_though_a_nerf_later_in_the_action_takes_it()
     {
        let nerfed = |announced: bool| {
            let mut g = scenario(json!({
                "p1": { "hand": ["meditative-066", "core-005"] },
                "p2": { "hand": ["core-005"] },
            }));
            let mut monitor = create_invariant_monitor(g.state());
            g.play("meditative-066", json!({ "zone": 1 }));
            assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());
            g.end_turn();
            assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());
            g.end_turn();
            let waraxe = g.unit(PlayerId::P1, 1).expect("the waraxe animated").clone();
            find_instance_mut(g.state_mut(), &waraxe.id)
                .expect("the waraxe")
                .tuning
                .get_or_insert_default()
                .remove_keywords = Some(vec![KeywordKind::AnimatedOnYourTurn]);
            let mut events = g.last_events().to_vec();
            if announced {
                events.push(event(json!({
                    "type": "degraded", "instanceId": waraxe.id, "defId": waraxe.def_id,
                    "change": { "kind": "keyword", "keyword": { "kind": "Animated on your turn" }, "added": false },
                })));
            }
            monitor.after(&events, g.state())
        };
        assert_eq!(nerfed(true), no_violations());
        assert!(
            nerfed(false)
                .iter()
                .any(|found| found.starts_with("I4 entry mismatch")),
            "an animation the state alone shows without the keyword is still an entry"
        );
    }

    /// #1216: a face-down Field Trap fused with Fiery Waraxe, set on an earlier turn, fires on its
    /// controller's turn and animates as its firing's last step, face-up for the first time: an entry
    /// (R383), as the engine's `summonedTurn` says, not the start-of-turn animation R1062 makes none.
    #[test]
    fn r1062_r383_a_card_that_fires_as_a_trap_on_its_controllers_turn_enters_as_it_animates() {
        let mut g = scenario(json!({ "p1": { "backrow": [{ "def": "meditative-066", "lane": 1 }] } }));
        let card = backrow_at(&g, PlayerId::P1, 1, "p1 should hold the card in backrow zone 1");
        let set_on = g.state().turn;
        find_instance_mut(g.state_mut(), &card.id)
            .expect("the card")
            .summoned_turn = Some(set_on);
        let mut monitor = create_invariant_monitor(g.state());
        let set = event(json!({
            "type": "summoned", "player": "p1", "instanceId": card.id, "defId": card.def_id,
            "row": "backrow", "lane": 1,
        }));
        assert_eq!(monitor.after(&[set], g.state()), no_violations());

        let now = set_on + 2;
        g.state_mut().turn = now;
        find_instance_mut(g.state_mut(), &card.id)
            .expect("the card")
            .summoned_turn = Some(now);
        let events = vec![
            event(json!({ "type": "turnStarted", "player": "p1", "turn": now })),
            event(json!({
                "type": "trapFired", "instanceId": card.id, "defId": card.def_id, "controller": "p1",
                "row": "backrow", "lane": 1,
            })),
            event(json!({
                "type": "animated", "player": "p1", "instanceId": card.id, "defId": card.def_id,
                "backrowLane": 1, "unitLane": 1,
            })),
        ];
        assert_eq!(monitor.after(&events, g.state()), no_violations());
    }
}

/// R1126: Pareto Optimality's attack animates The Cane on the other player's turn, which is an entry
/// (R383); only its controller's start-of-turn animation enters nothing (R1062).
mod r1126_the_canes_pareto_animation {
    use super::*;

    const PARETO: &str = "meditative-071";
    const BAD: &str = "core-t-felinor";

    #[test]
    fn r1126_r1062_the_cane_animating_on_the_other_players_turn_enters_and_i4_stays_clean() {
        let mut g = scenario(json!({
            // p1's (0) Spell, never played, keeps its turn from ending by itself (R82) once the Cane, which
            // may not switch to Defense (R2179000), is all it has on the field.
            "p1": { "mana": 10, "hand": [PARETO, "core-010"], "library": [VANILLA, VANILLA, VANILLA] },
            "p2": { "mana": 10, "hand": [VANILLA, BAD], "field": ["core-007"], "library": [VANILLA, VANILLA] },
        }));
        let mut monitor = create_invariant_monitor(g.state());
        // The Cry summons the Cane on p1's turn: it enters, animated (R383).
        g.play(PARETO, json!({}));
        assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());
        let cane = g
            .card(PARETO)
            .memory
            .get("cane")
            .and_then(|found| found.as_str().map(str::to_string))
            .expect("Pareto remembers its Cane");
        g.end_turn();
        assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());
        g.play(BAD, json!({}));
        assert!(
            g.last_events().iter().any(|event| matches!(
                event,
                GameEvent::Animated { instance_id, .. } if *instance_id == cane
            )),
            "the Cane animated for Pareto's attack"
        );
        assert_eq!(g.card(&cane).summoned_turn, Some(g.state().turn));
        assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());
        // At p1's next start of turn it animates on its own side, no entry (R1062).
        g.end_turn();
        assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());
    }
}

/// R97, R419, R11: Core #85's trap fuses a played Unit onto p2's Rush Token, which keeps the token's
/// id under a fused (non-token) definition; C+ #35 Rollback then puts the token back as it was, a
/// token again; it dies attacking, a token ceasing to exist in the open. I6 judges a card in no pile
/// by what it left as, so the events that name it after it went read to both seats, as R97 says.
mod r97_r419_a_fused_token_rolled_back_to_itself_dies_in_the_open {
    use super::*;

    const ROLLBACK: &str = "classicplus-035";
    const FUSING_TRAP: &str = "core-085";
    const MENACE: &str = "core-019";

    #[test]
    fn r97_r419_i6_stays_clean_as_a_rolled_back_token_ceases_to_exist() {
        let mut g = scenario(json!({
            "p1": { "mana": 10, "hand": [VANILLA], "field": [MENACE], "library": [VANILLA, VANILLA, VANILLA] },
            "p2": {
                "mana": 10,
                "hand": [ROLLBACK],
                "field": [RUSH_TOKEN],
                "backrow": [{ "def": FUSING_TRAP, "lane": 1 }],
                "library": [VANILLA, VANILLA, VANILLA],
            },
        }));
        let mut monitor = create_invariant_monitor(g.state());
        let token = g.unit(PlayerId::P2, 1).expect("p2's Rush Token").id;
        // Turn 11 begins on p1's side, and its snapshot holds the token as a token (R419). The
        // monitor reads both turn ends: I9 holds each `turnEnded` to the turn the shadow has under way.
        g.end_turn();
        assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());
        g.end_turn();
        assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());
        g.play(VANILLA, json!({ "zone": 2 }));
        assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());
        assert!(
            g.card(&token).def_id.starts_with("t-"),
            "the token wears the fused definition"
        );
        g.end_turn();
        assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());
        g.play(ROLLBACK, json!({ "modes": ["1"] }));
        assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());
        assert_eq!(g.card(&token).def_id, RUSH_TOKEN);
        // It attacks the Taunt Menace and dies: a token, it ceases to exist (R11).
        g.attack(token.as_str(), MENACE);
        assert!(
            find_instance(g.state(), &token).is_none(),
            "the token ceased to exist"
        );
        assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());
        assert_eq!(monitor.hidden(g.state()), no_violations());
    }
}

mod i8_an_attack_or_a_switch_reduce_accepts_that_the_list_leaves_out {
    use super::*;

    fn board() -> (Scenario, CardInstance, CardInstance) {
        let g = scenario(json!({
            "p1": { "field": [{ "def": VANILLA, "lane": 1 }] },
            "p2": { "field": [{ "def": VANILLA, "lane": 1 }] },
        }));
        let unit = g.unit_at(PlayerId::P1, 1);
        let prey = g.unit_at(PlayerId::P2, 1);
        (g, unit, prey)
    }

    fn without(g: &Scenario, left_out: &ActionBody) -> Vec<ActionBody> {
        let legal = legal_actions(g.state(), PlayerId::P1);
        assert!(legal.contains(left_out), "the list should hold {left_out:?}");
        legal.into_iter().filter(|body| body != left_out).collect()
    }

    #[test]
    fn i8_is_clean_when_the_list_holds_every_attack_and_switch() {
        let (g, _, _) = board();
        let legal = legal_actions(g.state(), PlayerId::P1);
        assert_eq!(
            unlisted_accepted(g.state(), PlayerId::P1, &legal),
            no_violations()
        );
        assert_eq!(
            create_invariant_monitor(g.state()).unlisted(g.state()),
            no_violations()
        );
    }

    #[test]
    fn i8_fires_on_a_switch_reduce_accepts_that_the_list_leaves_out() {
        let (g, unit, _) = board();
        let left_out = ActionBody::SwitchPosition {
            instance_id: unit.id.clone(),
        };
        let found = unlisted_accepted(g.state(), PlayerId::P1, &without(&g, &left_out));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].starts_with("I8 unlisted switch accepted"),
            "{}",
            found[0]
        );
        assert!(found[0].contains(&unit.id), "{}", found[0]);
        assert!(found[0].contains(VANILLA), "{}", found[0]);
        assert!(found[0].contains("§4.1"), "{}", found[0]);
    }

    #[test]
    fn i8_fires_on_an_attack_reduce_accepts_that_the_list_leaves_out() {
        let (g, unit, prey) = board();
        let left_out = ActionBody::Attack {
            attacker_id: unit.id.clone(),
            target_id: prey.id.clone(),
        };
        let found = unlisted_accepted(g.state(), PlayerId::P1, &without(&g, &left_out));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].starts_with("I8 unlisted attack accepted"),
            "{}",
            found[0]
        );
        assert!(found[0].contains(&unit.id), "{}", found[0]);
        assert!(found[0].contains(&prey.id), "{}", found[0]);
        assert!(found[0].contains(VANILLA), "{}", found[0]);
        assert!(found[0].contains("§4.2"), "{}", found[0]);
    }
}

/// I8 (#1574): an activation `reduce` accepts that the list leaves out, and a malformed answer.
mod i8_an_activation_reduce_accepts_that_the_list_leaves_out {
    use super::*;

    const PING: &str = "classicplus-076-1"; // C+ #76.1 Brother Ping: Activate (once a turn), deal 1 to any target
    const SCARAB: &str = "core-007"; // #7 Jewelosco Scarab: Cry: Discover, a prompt of one pick
    const DIVIDEND: &str = "core-024"; // #24 Efficiency Dividend: an X-cost Spell of three modes

    fn board() -> (Scenario, CardInstance) {
        let g = scenario(json!({
            "p1": { "hand": [VANILLA], "field": [PING], "library": [VANILLA, VANILLA] },
            "p2": { "hand": [VANILLA], "field": [VANILLA], "library": [VANILLA, VANILLA] },
        }));
        let ping = g.card(PING).clone();
        (g, ping)
    }

    fn at_enemy_hero(ping: &CardInstance) -> ActionBody {
        ActionBody::Activate {
            instance_id: ping.id.clone(),
            ability: Some("ping".to_string()),
            targets: Some(vec![Selection::Hero { player: PlayerId::P2 }]),
            modes: None,
            tributes: None,
        }
    }

    #[test]
    fn i8_is_clean_before_and_after_an_ability_s_one_use_and_on_the_other_turn() {
        let (mut g, ping) = board();
        let legal = legal_actions(g.state(), PlayerId::P1);
        assert!(
            legal.contains(&at_enemy_hero(&ping)),
            "the list should hold the ping"
        );
        assert_eq!(
            unlisted_accepted(g.state(), PlayerId::P1, &legal),
            no_violations()
        );

        g.act(at_enemy_hero(&ping), PlayerId::P1, "activate Ping");
        let legal = legal_actions(g.state(), PlayerId::P1);
        assert!(
            !legal.contains(&at_enemy_hero(&ping)),
            "a once-a-turn ability is spent"
        );
        assert_eq!(
            unlisted_accepted(g.state(), PlayerId::P1, &legal),
            no_violations()
        );

        g.end_turn();
        assert_eq!(
            create_invariant_monitor(g.state()).unlisted(g.state()),
            no_violations()
        );
    }

    #[test]
    fn i8_fires_on_an_activation_and_its_alias_reduce_accepts_that_the_list_leaves_out() {
        let (g, ping) = board();
        let left_out = at_enemy_hero(&ping);
        let legal: Vec<ActionBody> = legal_actions(g.state(), PlayerId::P1)
            .into_iter()
            .filter(|body| *body != left_out)
            .collect();
        let found = unlisted_accepted(g.state(), PlayerId::P1, &legal);
        assert_eq!(found.len(), 2, "{found:?}");
        for message in &found {
            assert!(
                message.starts_with("I8 unlisted activation accepted"),
                "{message}"
            );
            assert!(message.contains(&ping.id), "{message}");
            assert!(message.contains(PING), "{message}");
            assert!(message.contains("§10.2"), "{message}");
        }
        assert!(
            found
                .iter()
                .any(|message| message.contains("\"type\":\"activate\"")),
            "{found:?}"
        );
        assert!(
            found
                .iter()
                .any(|message| message.contains("\"type\":\"activatePower\"")),
            "{found:?}"
        );
    }

    #[test]
    fn every_malformed_variant_of_an_open_prompt_s_answer_is_refused() {
        let mut g = scenario(json!({
            "seed": "core-007-offer",
            "p1": { "hand": [SCARAB], "mana": 4, "library": [VANILLA] },
            "p2": { "field": [VANILLA] },
        }));
        g.play(SCARAB, json!({}));
        let answer = legal_actions(g.state(), PlayerId::P1)
            .into_iter()
            .find(|body| matches!(body, ActionBody::Answer { .. }))
            .expect("the Cry's prompt should list an answer");

        let malformed = malformed_answers(g.state(), &answer);
        assert_eq!(
            malformed.iter().map(|(name, _, _)| *name).collect::<Vec<_>>(),
            [
                STALE_CHOICE_ANSWER,
                OTHER_SEAT_ANSWER,
                DROPPED_PICK_ANSWER,
                DOUBLED_PICK_ANSWER,
                UNOFFERED_PICK_ANSWER
            ]
        );
        for (name, seat, body) in malformed {
            let result = reduce(g.state(), &Action::new(body, seat, "malformed".to_string()));
            assert!(result.error.is_some(), "{name} was accepted");
        }
        assert!(malformed_answers(g.state(), &ActionBody::EndTurn).is_empty());
    }

    #[test]
    fn every_malformed_variant_of_a_listed_play_and_activation_is_refused() {
        let g = scenario(json!({
            "p1": { "hand": [DIVIDEND], "mana": 3, "field": [PING], "library": [VANILLA, VANILLA] },
            "p2": { "field": [VANILLA], "library": [VANILLA, VANILLA] },
        }));
        let ping = g.card(PING).clone();
        let legal = legal_actions(g.state(), PlayerId::P1);
        let play = legal
            .iter()
            .find(|body| matches!(body, ActionBody::Play { x: Some(_), .. }))
            .expect("the list should hold a play of the X spell")
            .clone();
        assert!(
            legal.contains(&at_enemy_hero(&ping)),
            "the list should hold the ping"
        );

        let names = |malformed: &[(&'static str, PlayerId, ActionBody)]| {
            malformed.iter().map(|(name, _, _)| *name).collect::<Vec<_>>()
        };
        let forged_plays = malformed_actions(g.state(), PlayerId::P1, &play);
        assert_eq!(
            names(&forged_plays),
            [
                UNOFFERED_TARGET_ACTION,
                UNOFFERED_MODE_ACTION,
                UNOFFERED_ZONE_ACTION,
                NEGATIVE_X_ACTION,
                UNOFFERED_TRIBUTE_ACTION,
                UNOFFERED_PLAGUE_ACTION,
                UNOFFERED_REVEAL_ACTION
            ]
        );
        let forged_activations = malformed_actions(g.state(), PlayerId::P1, &at_enemy_hero(&ping));
        assert_eq!(
            names(&forged_activations),
            [
                UNOFFERED_ABILITY_ACTION,
                UNOFFERED_TARGET_ACTION,
                UNOFFERED_MODE_ACTION,
                UNOFFERED_TRIBUTE_ACTION,
                UNOFFERED_POWER_TARGET_ACTION
            ]
        );

        for (name, seat, body) in forged_plays.into_iter().chain(forged_activations) {
            assert_eq!(seat, PlayerId::P1, "{name}");
            let result = reduce(g.state(), &Action::new(body, seat, "malformed".to_string()));
            assert!(result.error.is_some(), "{name} was accepted");
            assert!(result.events.is_empty(), "{name} emitted {:?}", result.events);
            assert!(result.state == *g.state(), "{name} moved the state");
        }
        assert!(malformed_actions(g.state(), PlayerId::P1, &ActionBody::EndTurn).is_empty());
    }
}

/// R127, R436: a Cry run out of a graveyard runs with no card (`self` null), so the `chaosRolled` a
/// Call to Chaos in it sends names its definition beside an empty `instanceId`. The empty id is no
/// card, so I6 reads nothing hidden in it (fuzz seed 7232 of the handicapped wave, #1645). Every edition
/// announces its roll the same way (`announce_chaos`), so Core #95 fused into Mr Vanilla stands in.
mod r127_r436_a_call_to_chaos_cry_run_out_of_a_graveyard {
    use super::*;

    const REWIND: &str = "classic-054";
    const CALL_TO_CHAOS: &str = "core-095";

    #[test]
    fn r127_r436_i6_reads_the_empty_id_of_a_chaos_roll_with_no_card_as_no_card() {
        let mut g = scenario(json!({
            "p1": { "mana": 10, "hand": [REWIND], "library": [VANILLA, VANILLA, VANILLA] },
            "p2": { "hand": [BIGOT], "library": [VANILLA, VANILLA, VANILLA] },
        }));
        // A Unit whose Cry is Call to Chaos (R77), in p1's graveyard.
        let fused = craft_fused(&mut g, PlayerId::P1, &[VANILLA, CALL_TO_CHAOS]);
        let state = g.state_mut();
        state.players.p1.hand.retain(|card| card.id != fused.id);
        let mut buried = fused.clone();
        buried.zone = Zone::Graveyard { player: PlayerId::P1 };
        state.players.p1.graveyard.push(buried);
        g.play(
            REWIND,
            json!({ "targets": [{ "pick": "instance", "instanceId": fused.id }] }),
        );
        assert!(
            g.last_events().iter().any(|event| matches!(
                event,
                GameEvent::ChaosRolled { instance_id, def_id, .. }
                    if instance_id.is_empty() && *def_id == fused.def_id
            )),
            "the Cry rolled with no card: {:?}",
            g.last_events()
        );
        assert_eq!(
            create_invariant_monitor(g.state()).hidden(g.state()),
            no_violations()
        );
    }
}

/// R1122, R2285002: Traitorous Blood re-aims a declared attack at a Unit beside the attacker, and the
/// combat is fought there, so a Classic Golem's "when this attacks" transforms the Unit it was re-aimed
/// at, and a Golem that kills the Unit that took its place killed its defender (R42) and may attack
/// again: I4 reads it as readied through the redirect and the transform both (#1645, R424).
mod r1122_r2285002_a_redirected_classic_golem_that_kills_its_new_defender {
    use super::*;

    const GOLEM: &str = "classicplus-073-1";
    const TRAITOROUS_BLOOD: &str = "meditative-064";
    const MENACE: &str = "core-019";

    fn board(seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "field": [{ "def": GOLEM, "lane": 2 }, { "def": VANILLA, "lane": 3 }],
                "library": [VANILLA, VANILLA],
            },
            "p2": {
                "field": [{ "def": MENACE, "lane": 2 }],
                "backrow": [{ "def": TRAITOROUS_BLOOD, "faceDown": true }],
                "library": [VANILLA, VANILLA],
            },
        }))
    }

    fn killed_and_survived(g: &Scenario, golem: &str, ally: &str) -> Option<String> {
        let made = g.last_events().iter().find_map(|event| match event {
            GameEvent::Transformed {
                instance_id,
                new_instance_id,
                ..
            } if *instance_id == ally => Some(new_instance_id.clone()),
            _ => None,
        })?;
        let killed = g.last_events().iter().any(|event| {
            matches!(event, GameEvent::Destroyed { instance_id, killer_id: Some(killer), .. }
                if *instance_id == made && *killer == golem)
        });
        let alive = g.unit(PlayerId::P1, 2).is_some_and(|unit| unit.id == golem);
        (killed && alive).then_some(made)
    }

    #[test]
    fn r1122_r2285002_i4_reads_a_golem_that_killed_the_unit_its_redirected_attack_was_transformed_into_as_readied()
     {
        let seed = seed_where("redirected-golem", |seed| {
            let mut g = board(seed);
            let golem = g.unit(PlayerId::P1, 2).expect("p1's Golem").id;
            let ally = g.unit(PlayerId::P1, 3).expect("p1's Vanilla").id;
            let menace = g.unit(PlayerId::P2, 2).expect("p2's Menace").id;
            g.attack(golem.as_str(), menace.as_str());
            killed_and_survived(&g, &golem, &ally).is_some()
        });
        let mut g = board(&seed);
        let golem = g.unit(PlayerId::P1, 2).expect("p1's Golem").id;
        let ally = g.unit(PlayerId::P1, 3).expect("p1's Vanilla").id;
        let menace = g.unit(PlayerId::P2, 2).expect("p2's Menace").id;
        let mut monitor = create_invariant_monitor(g.state());
        g.attack(golem.as_str(), menace.as_str());
        assert!(
            g.last_events().iter().any(|event| matches!(
                event,
                GameEvent::Redirected { what: RedirectWhat::Attack, from_id, to_id, .. }
                    if *from_id == menace && *to_id == ally
            )),
            "the trap re-aimed the attack at the Golem's neighbor: {:?}",
            g.last_events()
        );
        assert!(killed_and_survived(&g, &golem, &ally).is_some());
        assert!(
            !g.card(golem.as_str()).exertion.attacked,
            "R2285002: it may attack again"
        );
        assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());
    }
}

mod r1200_i8_an_activation_left_with_no_pick_while_a_mayor_acts {
    use super::*;

    const FUSION_LAB: &str = "classicplus-031"; // C+ #31 Fusion Lab: Activate, choose a card in your hand
    const MAYOR: &str = "meditative-086"; // M #86 Mayor Medinamogger: all targets are chosen at random

    /// Fuzz seed 2870's board (#1728): p1's Fusion Lab with nothing in p1's hand while p2's Mayor acts.
    fn board() -> (Scenario, CardInstance) {
        let g = scenario(json!({
            "p1": { "hand": [], "backrow": [FUSION_LAB], "library": [VANILLA, VANILLA] },
            "p2": { "hand": [VANILLA], "field": [MAYOR], "library": [VANILLA, VANILLA] },
        }));
        let lab = g.card(FUSION_LAB).clone();
        (g, lab)
    }

    fn fuse(lab: &CardInstance) -> ActionBody {
        ActionBody::Activate {
            instance_id: lab.id.clone(),
            ability: Some("fuse".to_string()),
            targets: None,
            modes: None,
            tributes: None,
        }
    }

    #[test]
    fn r1200_i8_is_clean_on_an_activation_the_list_spells_with_no_targets() {
        let (g, lab) = board();
        let legal = legal_actions(g.state(), PlayerId::P1);
        assert!(legal.contains(&fuse(&lab)), "{legal:?}");
        assert_eq!(
            unlisted_accepted(g.state(), PlayerId::P1, &legal),
            no_violations()
        );
    }

    #[test]
    fn r1200_i8_still_fires_on_that_activation_and_its_alias_when_the_list_leaves_it_out() {
        let (g, lab) = board();
        let legal: Vec<ActionBody> = legal_actions(g.state(), PlayerId::P1)
            .into_iter()
            .filter(|body| *body != fuse(&lab))
            .collect();
        let found = unlisted_accepted(g.state(), PlayerId::P1, &legal);
        assert_eq!(found.len(), 2, "{found:?}");
        for message in &found {
            assert!(
                message.starts_with("I8 unlisted activation accepted"),
                "{message}"
            );
            assert!(message.contains(&lab.id), "{message}");
        }
    }
}

/// R1829000 (issue #1829): a card a Stack pile uncovers has entered the field on that turn, though no
/// event says so (R212). The monitor reads the uncovering off the piles of the states it is shown and
/// takes `summonedTurn`'s word only where an uncovering inside the one action was possible: a fusion's
/// kept card keeps its turn (R77, R1086), and a card in the zone a play was announced into may have been
/// covered and uncovered inside that action.
mod r1829000_a_card_uncovered_beneath_a_stack {
    use super::*;

    const FIENDER: &str = "core-092"; // (2) 5/7 Stack.

    fn covered_on_an_earlier_turn() -> (Scenario, InvariantMonitor, CardInstance, CardInstance, i32) {
        let mut g = scenario(json!({
            "p1": { "hand": [VANILLA, FIENDER, HIT_JOB], "library": [VANILLA, VANILLA] },
            "p2": { "hand": [VANILLA], "library": [VANILLA, VANILLA] },
        }));
        let mut monitor = create_invariant_monitor(g.state());
        let played_on = g.state().turn;
        g.play(VANILLA, json!({ "zone": 1 }));
        assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());
        let vanilla = g.unit_at(PlayerId::P1, 1);
        g.play(FIENDER, json!({ "zone": 1 }));
        assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());
        let fiender = g.unit_at(PlayerId::P1, 1);
        assert_eq!(fiender.def_id, FIENDER);
        for _ in 0..2 {
            g.end_turn();
            assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());
        }
        (g, monitor, vanilla, fiender, played_on)
    }

    fn entry_mismatches(found: &[String]) -> Vec<&String> {
        found
            .iter()
            .filter(|message| message.starts_with("I4 entry mismatch"))
            .collect()
    }

    #[test]
    fn r1829000_i4_stays_clean_as_a_unit_uncovered_on_a_later_turn_enters_again() {
        let (mut g, mut monitor, vanilla, fiender, _) = covered_on_an_earlier_turn();

        g.play(
            HIT_JOB,
            json!({ "targets": [{ "pick": "instance", "instanceId": fiender.id }] }),
        );

        assert_eq!(g.unit_at(PlayerId::P1, 1).id, vanilla.id);
        assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());
    }

    #[test]
    fn r1829000_i4_fires_on_an_uncovered_unit_left_with_the_turn_it_was_played() {
        let (mut g, mut monitor, vanilla, fiender, played_on) = covered_on_an_earlier_turn();
        g.play(
            HIT_JOB,
            json!({ "targets": [{ "pick": "instance", "instanceId": fiender.id }] }),
        );
        find_instance_mut(g.state_mut(), &vanilla.id)
            .expect("the Vanilla")
            .summoned_turn = Some(played_on);

        let found = monitor.after(g.last_events(), g.state());

        let mismatches = entry_mismatches(&found);
        assert_eq!(mismatches.len(), 1, "{found:?}");
        assert!(mismatches[0].contains(&vanilla.id), "{found:?}");
    }

    /// R1086: an Equip card whose play paused on top of its host fuses into it in a later action, and
    /// the host, which the fusion keeps, keeps the turn it arrived.
    #[test]
    fn r1829000_r1086_i4_stays_clean_as_the_card_a_fusion_keeps_acts_again_with_its_own_turn() {
        let (mut g, mut monitor, vanilla, fiender, played_on) = covered_on_an_earlier_turn();
        let mut gone = fiender.clone();
        jackioh_engine::zones::cease_to_exist(g.state_mut(), &mut gone);
        find_instance_mut(g.state_mut(), &vanilla.id)
            .expect("the Vanilla")
            .summoned_turn = Some(played_on);
        let fused = event(json!({
            "type": "fused", "instanceIds": [fiender.id, vanilla.id], "resultInstanceId": vanilla.id,
            "defId": vanilla.def_id,
        }));

        assert_eq!(monitor.after(&[fused], g.state()), no_violations());
    }

    #[test]
    fn r1829000_i4_takes_the_turn_of_a_unit_a_play_may_have_covered_and_uncovered_inside_one_action() {
        let (mut g, monitor, _, fiender, _) = covered_on_an_earlier_turn();
        let now = g.state().turn;
        find_instance_mut(g.state_mut(), &fiender.id)
            .expect("the Fiender")
            .summoned_turn = Some(now);
        let announced = |lane: i32| {
            event(json!({
                "type": "cardAnnounced", "player": "p1", "instanceId": "a-stack-card", "defId": FIENDER,
                "cardType": "Unit", "costPaid": 2, "targets": [], "row": "units", "lane": lane,
            }))
        };

        assert_eq!(monitor.clone().after(&[announced(1)], g.state()), no_violations());
        for events in [vec![], vec![announced(2)]] {
            let found = monitor.clone().after(&events, g.state());
            let mismatches = entry_mismatches(&found);
            assert_eq!(mismatches.len(), 1, "{found:?}");
            assert!(mismatches[0].contains(&fiender.id), "{found:?}");
        }
    }

    /// R14: a play announced into the zone beside the unit's, and the rings then turned (Core #52
    /// Silly Silas played onto a unit, the pile carried one step, Silas killed), may have covered and
    /// uncovered it too.
    #[test]
    fn r14_r1829000_i4_follows_a_covering_play_s_zone_around_the_ring() {
        use jackioh_engine::wire::RotationDirection;
        let (mut g, monitor, _, fiender, _) = covered_on_an_earlier_turn();
        let now = g.state().turn;
        find_instance_mut(g.state_mut(), &fiender.id)
            .expect("the Fiender")
            .summoned_turn = Some(now);
        let (from, direction) = [RotationDirection::Left, RotationDirection::Right]
            .into_iter()
            .flat_map(|direction| (2..=5).map(move |lane| (lane, direction)))
            .find(|(lane, direction)| {
                let to = jackioh_engine::zones::ring_neighbor(
                    jackioh_engine::zones::ZoneSlot {
                        player: PlayerId::P1,
                        row: jackioh_engine::wire::Row::Units,
                        lane: *lane,
                    },
                    *direction,
                    PlayerId::P1,
                );
                (to.player, to.lane) == (PlayerId::P1, 1)
            })
            .expect("a zone one step from lane 1");
        let played = event(json!({
            "type": "cardPlayed", "player": "p1", "instanceId": "silas", "defId": "core-052", "costPaid": 4,
        }));
        let announced = event(json!({
            "type": "cardAnnounced", "player": "p1", "instanceId": "silas", "defId": "core-052",
            "cardType": "Unit", "costPaid": 4, "targets": [], "row": "units", "lane": from,
        }));
        let rotated = event(json!({ "type": "rotated", "direction": direction }));

        assert_eq!(
            monitor
                .clone()
                .after(&[announced.clone(), played.clone(), rotated], g.state()),
            no_violations()
        );
        let found = monitor.clone().after(&[announced, played], g.state());
        assert_eq!(entry_mismatches(&found).len(), 1, "{found:?}");
    }
}

/// R1869002 (#1869), B5 E35: Stinkyscape #94 Tmack Trav attacks again after each Unit its attack destroys
/// (`mayAttackAgain`, no transform), so I2 counts a kill credit beside its one attack, and I1 and I4 read
/// the sickness the kill lifted as readied. Only an attacker whose text answers its attacks
/// (`afterAttack`) earns one, and the state the action leaves takes it back when the attacker's attack
/// is still spent.
mod r1869002_i2_a_kill_credit {
    use super::*;

    const TMACK_TRAV: &str = "stinkyscape-094";
    const WASP: &str = "stinkyscape-091";
    const MR_TOKEN: &str = "core-015";

    fn attack(attacker: &str, target: &str) -> ActionBody {
        ActionBody::Attack {
            attacker_id: attacker.to_string(),
            target_id: target.to_string(),
        }
    }

    #[test]
    fn r1869002_a_tmack_trav_played_this_turn_kills_and_attacks_a_unit_again_with_no_violation() {
        let mut g = scenario(json!({
            "p1": { "hand": [TMACK_TRAV, VANILLA] },
            "p2": { "field": [{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 2 }], "hand": [VANILLA] },
        }));
        let prey = g.unit_at(PlayerId::P2, 1).id;
        let next = g.unit_at(PlayerId::P2, 2).id;
        // The setup's cards enter as the shadow begins, their bookkeeping to match (p2's, so no sickness
        // of theirs is read on p1's turn).
        let turn = g.state().turn;
        for id in [&prey, &next] {
            find_instance_mut(g.state_mut(), id)
                .expect("p2's Vanilla")
                .summoned_turn = Some(turn);
        }
        let mut monitor = create_invariant_monitor(g.state());
        monitor.enter(&prey);
        monitor.enter(&next);
        g.play(TMACK_TRAV, json!({ "zone": 1 }));
        assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());
        let tmack = g.unit_at(PlayerId::P1, 1).id;

        let first = attack(&tmack, &prey);
        assert_eq!(monitor.before(g.state(), PlayerId::P1, &first), no_violations());
        g.attack(tmack.as_str(), prey.as_str());
        assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());
        // R2521002: the kill keeps its summoning sickness, so Rush lets it attack again a Unit only.
        assert_eq!(g.card(tmack.as_str()).summoned_turn, Some(turn));

        let second = attack(&tmack, &next);
        assert_eq!(monitor.before(g.state(), PlayerId::P1, &second), no_violations());
        g.attack(tmack.as_str(), next.as_str());
        assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());

        assert_eq!(
            monitor.before(g.state(), PlayerId::P1, &ActionBody::EndTurn),
            no_violations()
        );
        g.end_turn();
        assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());
    }

    #[test]
    fn r1869002_r636_a_windfury_tmack_trav_s_kill_on_its_second_attack_gives_it_two_more() {
        let mut g = scenario(json!({
            "p1": { "field": [{ "def": TMACK_TRAV, "lane": 1 }] },
            "p2": { "field": [{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 2 }] },
        }));
        let tmack = g.unit_at(PlayerId::P1, 1);
        let first = g.unit_at(PlayerId::P2, 1);
        let second = g.unit_at(PlayerId::P2, 2);
        find_instance_mut(g.state_mut(), &tmack.id)
            .expect("Tmack Trav")
            .granted_keywords = vec![json_as::<Keyword>(json!({ "kind": "Windfury" }))];
        let mut monitor = create_invariant_monitor(g.state());
        // Read between actions, as the fuzz does, for Windfury (R636); its I3 on the setup's cards is not
        // this test's.
        monitor.before(g.state(), PlayerId::P1, &ActionBody::EndTurn);
        let declared = |target: &str| {
            event(
                json!({ "type": "attackDeclared", "attackerId": tmack.id, "targetId": target, "forced": false }),
            )
        };
        let killed = |prey: &CardInstance| {
            event(json!({
                "type": "destroyed", "instanceId": prey.id, "defId": prey.def_id, "owner": "p2",
                "controller": "p2", "attack": 4, "maxHealth": 4, "killerId": tmack.id,
            }))
        };
        // Windfury's two, the second a kill whose fresh exertion is two more (B5 E35); a fifth is extra.
        let mut events = vec![
            declared("hero-p2"),
            declared(&first.id),
            killed(&first),
            declared("hero-p2"),
            declared(&second.id),
            killed(&second),
        ];
        assert_eq!(monitor.after(&events, g.state()), no_violations());
        events = vec![declared("hero-p2"), declared("hero-p2"), declared("hero-p2")];
        let found = monitor.after(&events, g.state());
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].contains("I2 extra attack: ") && found[0].contains("attack number 7"),
            "{}",
            found[0]
        );
    }

    /// R636: a Fuse brings its ingredients' text, and with it a Windfury an aura of theirs gives, which no
    /// `keywordGranted` reports; the fused unit has it from the fuse on, inside the one action that fused
    /// it and attacked twice (a handicapped seat's AI turn: M #65 Keymaster Keenus equipped onto S #113).
    #[test]
    fn r636_a_windfury_a_fuse_brings_allows_two_attacks_in_the_action_that_fused_it() {
        let mut g = scenario(json!({ "p1": { "field": [{ "def": VANILLA, "lane": 1 }] } }));
        let unit = g.unit_at(PlayerId::P1, 1).id;
        let mut monitor = create_invariant_monitor(g.state());
        find_instance_mut(g.state_mut(), &unit)
            .expect("the Vanilla")
            .granted_keywords = vec![json_as::<Keyword>(json!({ "kind": "Windfury" }))];
        let declared = || {
            event(
                json!({ "type": "attackDeclared", "attackerId": unit, "targetId": "hero-p2", "forced": false }),
            )
        };
        let fused = event(json!({
            "type": "fused", "instanceIds": ["c-gone", unit], "resultInstanceId": unit, "defId": VANILLA,
        }));
        assert_eq!(
            monitor.after(&[fused, declared(), declared()], g.state()),
            no_violations()
        );
    }

    /// R636: a fusion whose text draws its keywords as it fuses (M #65 Keymaster Keenus' Aura) and that
    /// leaves the field later in the same action took them away with its memory; two attacks it
    /// declared after the fuse are one legal Windfury turn (equipped onto C+ #26, two kills, then exiled).
    #[test]
    fn r636_a_keyword_drawing_fusion_gone_by_the_action_s_end_may_have_attacked_twice() {
        const KEYMASTER_KEENUS: &str = "meditative-065";
        let mut g = scenario(json!({ "p1": { "field": [{ "def": VANILLA, "lane": 1 }] } }));
        let mut monitor = create_invariant_monitor(g.state());
        let fused = craft_fused(&mut g, PlayerId::P1, &[KEYMASTER_KEENUS, VANILLA]);
        let declared = || {
            event(
                json!({ "type": "attackDeclared", "attackerId": fused.id, "targetId": "hero-p2", "forced": false }),
            )
        };
        let fuse = event(json!({
            "type": "fused", "instanceIds": ["c-gone", fused.id], "resultInstanceId": fused.id, "defId": fused.def_id,
        }));
        assert_eq!(
            monitor.after(&[fuse, declared(), declared()], g.state()),
            no_violations()
        );
    }

    /// R636: a token whose text draws its keywords as it arrives (the Stinkyscape Drone Corvette) and
    /// that ceased to exist later in the same action took them with it; two attacks it declared are one
    /// legal Windfury turn (summoned, two kills, killed, all inside one autopilot action).
    #[test]
    fn r636_a_keyword_drawing_token_gone_by_the_action_s_end_may_have_attacked_twice() {
        const DRONE_CORVETTE: &str = "stinkyscape-t-drone-corvette";
        let g = scenario(json!({ "p1": { "field": [{ "def": VANILLA, "lane": 1 }] } }));
        let mut monitor = create_invariant_monitor(g.state());
        let gone = "c-gone";
        let declared = || {
            event(
                json!({ "type": "attackDeclared", "attackerId": gone, "targetId": "hero-p1", "forced": false }),
            )
        };
        let summoned = event(json!({
            "type": "summoned", "player": "p2", "instanceId": gone, "defId": DRONE_CORVETTE, "row": "units", "lane": 4,
        }));
        assert_eq!(
            monitor.after(&[summoned, declared(), declared()], g.state()),
            no_violations()
        );
    }

    /// R12, R636: a steal keeps a card on the field with the keywords it drew, so a Drone Corvette
    /// summoned, attacking twice, stolen and attacking twice more for its new controller, then
    /// bounced, all inside one AI turn, attacked legally.
    #[test]
    fn r636_r12_a_keyword_drawing_token_stolen_inside_the_action_keeps_what_it_drew() {
        const DRONE_CORVETTE: &str = "stinkyscape-t-drone-corvette";
        let g = scenario(json!({ "p1": { "field": [{ "def": VANILLA, "lane": 1 }] } }));
        let mut monitor = create_invariant_monitor(g.state());
        let gone = "c-gone";
        let declared = |target: &str| {
            event(
                json!({ "type": "attackDeclared", "attackerId": gone, "targetId": target, "forced": false }),
            )
        };
        let summoned = event(json!({
            "type": "summoned", "player": "p1", "instanceId": gone, "defId": DRONE_CORVETTE, "row": "units", "lane": 3,
        }));
        let stolen = event(json!({
            "type": "controlChanged", "instanceId": gone, "controller": "p2", "row": "units", "lane": 2, "how": "steal",
        }));
        assert_eq!(
            monitor.after(
                &[
                    summoned,
                    declared("hero-p2"),
                    declared("hero-p2"),
                    stolen,
                    declared("hero-p1"),
                    declared("hero-p1")
                ],
                g.state()
            ),
            no_violations()
        );
    }

    #[test]
    fn r1869002_a_unit_whose_text_answers_no_attack_earns_no_kill_credit() {
        let g = scenario(json!({
            "p1": { "field": [{ "def": VANILLA, "lane": 1 }] },
            "p2": { "field": [{ "def": VANILLA, "lane": 1 }] },
        }));
        let unit = g.unit_at(PlayerId::P1, 1);
        let prey = g.unit_at(PlayerId::P2, 1);
        let mut monitor = create_invariant_monitor(g.state());
        let declared =
            json!({ "type": "attackDeclared", "attackerId": unit.id, "targetId": prey.id, "forced": false });
        let events = vec![
            event(declared.clone()),
            event(json!({
                "type": "destroyed", "instanceId": prey.id, "defId": prey.def_id, "owner": "p2",
                "controller": "p2", "attack": 4, "maxHealth": 4, "killerId": unit.id,
            })),
            event(declared),
        ];
        let found = monitor.after(&events, g.state());
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("I2 extra attack"), "{}", found[0]);
    }

    #[test]
    fn r1869002_a_kill_an_ambush_s_prompt_delayed_still_credits_the_attack_it_paused() {
        // The attack is declared, an ambush's Discover (S #22 Gatinsir's Phalanxes) pauses its combat,
        // and the kill lands in the answer's action: the attack the action left open is still the one
        // resolving, so the kill frees Tmack Trav.
        let g = scenario(json!({
            "p1": { "field": [{ "def": TMACK_TRAV, "lane": 1 }] },
            "p2": { "field": [{ "def": VANILLA, "lane": 1 }] },
        }));
        let tmack = g.unit_at(PlayerId::P1, 1);
        let prey = g.unit_at(PlayerId::P2, 1);
        let mut monitor = create_invariant_monitor(g.state());
        monitor.before(g.state(), PlayerId::P1, &ActionBody::EndTurn);
        let mut paused = g.state().clone();
        paused.pending = Some(json_as(json!({
            "id": "q1", "playerId": "p2", "kind": "discover", "prompt": "Discover", "options": [],
            "min": 1, "max": 1,
            "resume": { "defId": "stinkyscape-022", "hook": "trap", "step": "discover", "radiant": false, "data": {} },
        })));
        let declared = event(
            json!({ "type": "attackDeclared", "attackerId": tmack.id, "targetId": prey.id, "forced": false }),
        );
        assert_eq!(monitor.after(&[declared], &paused), no_violations());
        let killed = event(json!({
            "type": "destroyed", "instanceId": prey.id, "defId": prey.def_id, "owner": "p2",
            "controller": "p2", "attack": 4, "maxHealth": 4, "killerId": tmack.id,
        }));
        assert_eq!(monitor.after(&[killed], g.state()), no_violations());
        let again = json!({ "type": "attackDeclared", "attackerId": tmack.id, "targetId": "hero-p2", "forced": false });
        assert_eq!(monitor.after(&[event(again)], g.state()), no_violations());
    }

    /// A kill whose victim's Death steals the attacker (Core #86) gives it a new stint before its
    /// `afterAttack` frees it, and the prompt the action leaves open holds the credit unsettled: the
    /// lifted sickness is the credit's, not an entry mismatch.
    #[test]
    fn r1869002_r12_a_kill_credit_unsettled_across_a_steal_reads_as_readied() {
        let mut g = scenario(json!({
            "p1": { "field": [{ "def": TMACK_TRAV, "lane": 1 }] },
            "p2": { "field": [{ "def": VANILLA, "lane": 1 }] },
        }));
        let tmack = g.unit_at(PlayerId::P1, 1);
        let prey = g.unit_at(PlayerId::P2, 1);
        let mut monitor = create_invariant_monitor(g.state());
        monitor.before(g.state(), PlayerId::P1, &ActionBody::EndTurn);
        find_instance_mut(g.state_mut(), &tmack.id)
            .expect("Tmack Trav")
            .summoned_turn = None;
        let mut paused = g.state().clone();
        paused.pending = Some(json_as(json!({
            "id": "q1", "playerId": "p1", "kind": "mode", "prompt": "Choose", "options": [],
            "min": 1, "max": 1,
            "resume": { "defId": "core-086", "hook": "death", "step": "steal", "radiant": false, "data": {} },
        })));
        let events = [
            event(
                json!({ "type": "attackDeclared", "attackerId": tmack.id, "targetId": prey.id, "forced": false }),
            ),
            event(json!({
                "type": "destroyed", "instanceId": prey.id, "defId": prey.def_id, "owner": "p2",
                "controller": "p2", "attack": 4, "maxHealth": 4, "killerId": tmack.id,
            })),
            event(json!({
                "type": "controlChanged", "instanceId": tmack.id, "controller": "p1", "row": "units", "lane": 1,
                "how": "steal",
            })),
        ];
        let found = monitor.after(&events, &paused);
        assert!(
            !found
                .iter()
                .any(|message| message.starts_with("I4 entry mismatch")),
            "{found:?}"
        );
    }

    #[test]
    fn r1869002_a_kill_that_left_its_attack_spent_readied_nothing_and_its_credit_is_taken_back() {
        let mut g = scenario(json!({
            "p1": { "field": [{ "def": WASP, "lane": 1 }], "hand": [VANILLA] },
            "p2": { "field": [{ "def": MR_TOKEN, "lane": 1 }], "hand": [VANILLA] },
        }));
        let wasp = g.unit_at(PlayerId::P1, 1).id;
        let prey = g.unit_at(PlayerId::P2, 1).id;
        let mut monitor = create_invariant_monitor(g.state());
        g.attack(wasp.as_str(), prey.as_str());
        assert_eq!(monitor.after(g.last_events(), g.state()), no_violations());
        assert!(g.card(wasp.as_str()).exertion.attacked);

        let again =
            json!({ "type": "attackDeclared", "attackerId": wasp, "targetId": "hero-p2", "forced": false });
        let found = monitor.after(&[event(again)], g.state());
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("I2 extra attack"), "{}", found[0]);
    }
}

/// R678: a Glitch's boards outcome deals both fields again and places their cards with no `summoned`,
/// so the `glitched` event is the entry of every card the field holds after it (a Glitch played from
/// the graveyard mid-game).
mod r678_a_glitch_s_boards_are_entered {
    use super::*;

    #[test]
    fn r678_i3_reads_the_cards_a_glitch_s_boards_placed_as_entered() {
        let mut g = scenario(json!({ "p1": { "hand": [VANILLA] }, "p2": { "hand": [VANILLA] } }));
        let monitor = create_invariant_monitor(g.state());
        let turn = g.state().turn;
        let placed = {
            let state = g.state_mut();
            let mut card = new_instance(
                &mut *state,
                VANILLA,
                PlayerId::P1,
                Zone::Resolving { player: PlayerId::P1 },
            );
            let slot = jackioh_engine::zones::ZoneSlot {
                player: PlayerId::P1,
                row: jackioh_engine::wire::Row::Units,
                lane: 1,
            };
            assert!(jackioh_engine::zones::place_on_field(
                state,
                &mut card,
                slot,
                Default::default()
            ));
            find_instance_mut(state, &card.id)
                .expect("the placed card")
                .summoned_turn = Some(turn);
            card.id
        };
        let glitched = event(json!({ "type": "glitched", "player": "p2", "outcome": "boards" }));
        let silent = |found: &[String]| {
            found
                .iter()
                .filter(|message| message.starts_with("I3 silent arrival") && message.contains(&placed))
                .count()
        };

        let mut boards = monitor.clone();
        let mut found = boards.after(&[glitched], g.state());
        found.extend(boards.before(g.state(), PlayerId::P1, &ActionBody::EndTurn));
        assert_eq!(silent(&found), 0, "{found:?}");

        let mut none = monitor.clone();
        let mut found = none.after(&[], g.state());
        found.extend(none.before(g.state(), PlayerId::P1, &ActionBody::EndTurn));
        assert_eq!(silent(&found), 1, "{found:?}");
    }
}

/// R1279000, I10 (#2568): a prompt held for the opponent never opens while its holder is not the active
/// player. The monitor reads one that does as a violation, and reads the three cards that defer theirs,
/// and a prompt the asking card's own controller holds on the opponent's turn, as clean.
mod r1279000_opponent_held_prompt_off_turn {
    use super::*;

    const PICKLE: &str = "classic-008";
    const INCOME_TAX: &str = "classic-009";
    const IN_TOO_DEEP: &str = "classic-090";
    const PALANTIR: &str = "classic-004";
    const FIXTURE_ASKER: &str = "i10-asks-the-opponent";

    const CURSE: &str = "classic-001";
    const STOCKPILE: &str = "core-005";
    const TIMMY: &str = "core-011";
    const FELINORS: &str = "core-012";
    const MENACE: &str = "core-019";
    const SEVEN: &str = "core-025";

    /// Spells that need no board and cost nothing a side could lack: §2.5 never ends a turn on its own.
    const SPARE_HAND: [&str; 3] = [CURSE, STOCKPILE, TIMMY];
    const SPARE_LIBRARY: [&str; 4] = [VANILLA, VANILLA, VANILLA, VANILLA];
    /// Pickle's piles, as its own tests have them: the caster's deck, and the opponent's hand and deck.
    const CASTER_DECK: [&str; 4] = ["core-019", "core-025", "core-012", "core-020"];
    const HOLDER_HAND: [&str; 3] = ["core-008", "core-011", "core-015"];
    const HOLDER_DECK: [&str; 4] = ["core-006", "core-035", "core-048", "core-039"];

    /// A scenario run under the monitor: every step goes through `act`, which hands the monitor that step's
    /// events and keeps what it found.
    struct Watched {
        g: Scenario,
        monitor: InvariantMonitor,
        found: Vec<String>,
        /// How many steps left a prompt open for the player who is not active: the asking card's own
        /// controller's, since an opponent-held one would be a violation.
        off_turn_prompts: usize,
    }

    impl Watched {
        fn new(g: Scenario) -> Watched {
            let monitor = create_invariant_monitor(g.state());
            Watched {
                g,
                monitor,
                found: Vec::new(),
                off_turn_prompts: 0,
            }
        }

        fn act(&mut self, step: impl FnOnce(&mut Scenario)) {
            step(&mut self.g);
            self.found
                .extend(self.monitor.after(self.g.last_events(), self.g.state()));
            let state = self.g.state();
            if state
                .pending
                .as_ref()
                .is_some_and(|pending| pending.player_id != state.active)
            {
                self.off_turn_prompts += 1;
            }
        }

        /// Answers each prompt that is open with its first options, until none is.
        fn answer_all(&mut self) {
            while let Some(pending) = self.g.state().pending.clone() {
                let picks: Vec<Selection> = pending
                    .options
                    .iter()
                    .take(pending.min.max(1) as usize)
                    .map(|option| option.selection.clone())
                    .collect();
                self.act(|g| {
                    g.answer(json!(picks));
                });
            }
        }

        fn holder(&self) -> Option<PlayerId> {
            self.g.state().pending.as_ref().map(|pending| pending.player_id)
        }

        fn is_clean(&self) -> bool {
            self.found.is_empty()
        }
    }

    /// `pickle` played by `caster` on their own turn, the opponent holding the chain.
    fn pickle_board(caster: PlayerId) -> Scenario {
        let holder = opponent_of(caster);
        let mut setup = json!({ "active": caster });
        setup[caster.as_str()] = json!({ "hand": [PICKLE, STOCKPILE], "library": CASTER_DECK });
        setup[holder.as_str()] = json!({ "hand": HOLDER_HAND, "library": HOLDER_DECK });
        scenario(setup)
    }

    /// p1's Palantir steals what p2 plays and casts it as p1 (R831001): p2 is active, and p1 the card's
    /// controller, so what its Cry asks p2 for, p2 holds on its own turn.
    fn palantir_board(p1: Value, p2: Value) -> Scenario {
        let mut p1 = p1;
        p1["backrow"] = json!([{ "def": PALANTIR, "faceUp": true }]);
        scenario(json!({ "active": "p2", "p1": p1, "p2": p2 }))
    }

    /// Income Tax on `watcher`'s side, face down; `drawer` plays Stockpile, a second draw of their turn.
    fn tax_board(drawer: PlayerId, radiant: bool) -> Scenario {
        let watcher = opponent_of(drawer);
        let mut setup = json!({ "active": drawer });
        setup[drawer.as_str()] = json!({ "hand": [STOCKPILE, MENACE], "library": [FELINORS, SEVEN, TIMMY] });
        setup[watcher.as_str()] = json!({
            "hand": [VANILLA],
            "backrow": [{ "def": INCOME_TAX, "faceUp": false, "radiant": radiant }],
            "library": [VANILLA, VANILLA],
        });
        scenario(setup)
    }

    /// p1's In Too Deep at its quest 5 (15 damage dealt to the enemy), `progress` of it done.
    fn deep_board(active: PlayerId, progress: i32) -> Scenario {
        let mut g = scenario(json!({
            "active": active,
            "p1": {
                "hand": SPARE_HAND,
                "library": SPARE_LIBRARY,
                "backrow": [IN_TOO_DEEP],
                "field": [{ "def": VANILLA, "lane": 1 }],
            },
            "p2": {
                "hand": [VANILLA, TIMMY, MENACE],
                "library": SPARE_LIBRARY,
                "field": [{ "def": TIMMY, "lane": 1 }],
            },
        }));
        let line =
            json!({ "active": ["5"], "progress": { "5": progress }, "done": [], "auras": [], "waiting": [] });
        let id = g.card(IN_TOO_DEEP).id.clone();
        find_instance_mut(g.state_mut(), &id)
            .expect("In Too Deep in the state")
            .memory
            .insert(subsystems::QUEST_MEMORY_KEY.to_string(), line);
        g
    }

    #[test]
    fn r1279000_i10_a_fixture_card_asking_the_opponent_at_once_on_its_controller_s_turn_fails() {
        let mut g = scenario(json!({
            "p1": { "hand": [] },
            "p2": { "hand": [] },
        }));
        fixture(
            &mut g,
            FIXTURE_ASKER,
            CardType::Spell,
            Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::choose_mode(json_as(json!({
                        "options": ["ok"],
                        "step": "answered",
                        "prompt": "i10: the opponent's question",
                        "by": "enemy",
                    })))]
                })),
                resume: [("answered", hook(|_ctx| vec![]))].into_iter().collect(),
                ..Script::default()
            },
            None,
        );
        let card = in_hand(&mut g, FIXTURE_ASKER, PlayerId::P1);
        let mut monitor = create_invariant_monitor(g.state());
        g.play(&card.id, json!({}));

        // p1 is active, and the question is p2's: the state the three cards left open until v0.3.16.
        let pending = must(g.state().pending.clone(), "the question");
        assert_eq!(g.state().active, PlayerId::P1);
        assert_eq!(pending.player_id, PlayerId::P2);
        let found = monitor.after(g.last_events(), g.state());
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].starts_with("I10 opponent prompt off turn:"),
            "{}",
            found[0]
        );
        assert!(found[0].contains(&card.id), "names the card: {}", found[0]);
        assert!(
            found[0].contains(FIXTURE_ASKER),
            "names its definition: {}",
            found[0]
        );
        assert!(
            found[0].contains(&format!("on turn {}", g.state().turn)),
            "names the turn: {}",
            found[0]
        );
        assert!(
            found[0].contains("(§10.6, R1279000)"),
            "cites the ruling: {}",
            found[0]
        );
    }

    /// The one violation `w` holds, of I10, naming `def_id`'s card.
    fn assert_one_i10(w: &Watched, def_id: &str) {
        assert_eq!(w.found.len(), 1, "{:?}", w.found);
        assert!(
            w.found[0].starts_with("I10 opponent prompt off turn:"),
            "{}",
            w.found[0]
        );
        assert!(w.found[0].contains(def_id), "{}", w.found[0]);
    }

    /// The monitor finds what the three cards did through v0.3.16, which the fuzz played for all those
    /// patches without a word: their kept scripts, in the v0.4.0 catalog, ask at once.
    #[test]
    fn r1279000_i10_finds_the_three_cards_as_they_asked_at_once_through_v0_3_16() {
        jackioh_cards::register_all();
        let shipped = jackioh_cards::shipped_catalog("v0.4.0").expect("v0.4.0 is carried");
        jackioh_engine::with_shipped(Some(shipped), || {
            // Pickle's questions opened during p1's turn.
            let mut w = Watched::new(pickle_board(PlayerId::P1));
            w.act(|g| {
                g.play(PICKLE, json!({}));
            });
            assert_eq!(w.holder(), Some(PlayerId::P2));
            assert_one_i10(&w, PICKLE);

            // Income Tax's keep opened at once on p2's turn, p1 being the one who drew.
            let mut w = Watched::new(palantir_board(
                json!({ "hand": [VANILLA, MENACE], "library": [FELINORS, SEVEN, TIMMY] }),
                json!({
                    "hand": [STOCKPILE, VANILLA],
                    "backrow": [{ "def": INCOME_TAX, "faceUp": false }],
                    "library": [VANILLA, VANILLA],
                }),
            ));
            w.act(|g| {
                g.play(STOCKPILE, json!({}));
            });
            assert_eq!(w.holder(), Some(PlayerId::P1));
            assert_one_i10(&w, INCOME_TAX);

            // In Too Deep's reward G asked p2 at once, in p1's turn.
            let mut w = Watched::new(deep_board(PlayerId::P1, 15));
            w.act(|g| {
                g.play(CURSE, json!({}));
            });
            assert!(w.is_clean(), "the reward choice is p1's own: {:?}", w.found);
            w.act(|g| {
                g.answer(json!("G"));
            });
            assert_eq!(w.holder(), Some(PlayerId::P2));
            assert_one_i10(&w, IN_TOO_DEEP);
        });
    }

    #[test]
    fn r1279000_i10_a_prompt_the_asking_card_s_controller_holds_passes_on_the_opponent_s_turn() {
        // p2 attacks into p1's unit on their own turn, and the strike back is p1's cards dealing In Too
        // Deep's last point of damage, so p1's reward choice opens while p2 is active.
        let mut w = Watched::new(deep_board(PlayerId::P2, 14));
        let timmy = w.g.unit_at(PlayerId::P2, 1);
        let vanilla = w.g.unit_at(PlayerId::P1, 1);
        w.act(|g| {
            g.attack(&timmy, &vanilla);
        });
        assert_eq!(w.g.state().active, PlayerId::P2);
        assert_eq!(
            w.holder(),
            Some(PlayerId::P1),
            "the reward choice is p1's, held on p2's turn"
        );
        assert_eq!(w.off_turn_prompts, 1);
        assert!(w.is_clean(), "{:?}", w.found);
        // Reward G's discard is p2's own prompt, which p1 asked: it opens at once, p2 being active.
        w.act(|g| {
            g.answer(json!("G"));
        });
        assert_eq!(w.holder(), Some(PlayerId::P2));
        w.answer_all();
        assert!(w.is_clean(), "{:?}", w.found);
    }

    #[test]
    fn r1279000_i10_pickle_passes_cast_on_either_players_turn() {
        for caster in [PlayerId::P1, PlayerId::P2] {
            // Cast on its own turn, Pickle waits for the start of the opponent's.
            let mut w = Watched::new(pickle_board(caster));
            w.act(|g| {
                g.play(PICKLE, json!({}));
            });
            assert_eq!(
                w.holder(),
                None,
                "{caster}'s Pickle asks nothing while the opponent is not active"
            );
            w.act(|g| {
                g.end_turn();
            });
            assert_eq!(w.holder(), Some(opponent_of(caster)));
            w.answer_all();
            assert!(w.is_clean(), "{caster}: {:?}", w.found);
        }

        // Stolen and cast by p1's Palantir on p2's turn, it asks p2 at once, who is active.
        let mut w = Watched::new(palantir_board(
            json!({ "hand": [STOCKPILE], "library": CASTER_DECK }),
            json!({ "hand": [PICKLE, STOCKPILE], "library": HOLDER_DECK }),
        ));
        w.act(|g| {
            g.play(PICKLE, json!({}));
        });
        assert_eq!(w.holder(), Some(PlayerId::P2));
        w.answer_all();
        assert!(w.is_clean(), "{:?}", w.found);
    }

    #[test]
    fn r1279000_i10_income_tax_passes_fired_on_either_players_turn() {
        for drawer in [PlayerId::P1, PlayerId::P2] {
            // The drawer is the one it asks, and active: the keep opens at once.
            let mut w = Watched::new(tax_board(drawer, false));
            w.act(|g| {
                g.play(STOCKPILE, json!({}));
            });
            assert_eq!(w.holder(), Some(drawer));
            w.answer_all();
            assert!(w.is_clean(), "{drawer}: {:?}", w.found);
        }

        // p1's Palantir has p1 draw on p2's turn: the keep is p1's, so it waits for p1's turn.
        let mut w = Watched::new(palantir_board(
            json!({ "hand": [VANILLA, MENACE], "library": [FELINORS, SEVEN, TIMMY] }),
            json!({
                "hand": [STOCKPILE, VANILLA],
                "backrow": [{ "def": INCOME_TAX, "faceUp": false, "radiant": true }],
                "library": [VANILLA, VANILLA],
            }),
        ));
        assert_eq!(w.g.state().active, PlayerId::P2);
        w.act(|g| {
            g.play(STOCKPILE, json!({}));
        });
        assert_eq!(w.holder(), None, "the keep waits while p1 is not active");
        w.act(|g| {
            g.end_turn();
        });
        assert_eq!(w.holder(), Some(PlayerId::P1));
        w.answer_all();
        assert!(w.is_clean(), "{:?}", w.found);
    }

    #[test]
    fn r1279000_i10_in_too_deep_passes_with_its_quest_completed_on_either_players_turn() {
        // On p1's own turn, reward G's discard is p2's choice and waits for p2's turn.
        let mut w = Watched::new(deep_board(PlayerId::P1, 15));
        w.act(|g| {
            g.play(CURSE, json!({}));
        });
        assert_eq!(w.holder(), Some(PlayerId::P1), "the reward choice is p1's own");
        w.act(|g| {
            g.answer(json!("G"));
        });
        assert_eq!(w.holder(), None, "the discard waits while p2 is not active");
        w.act(|g| {
            g.end_turn();
        });
        assert_eq!(w.holder(), Some(PlayerId::P2));
        w.answer_all();
        assert!(w.is_clean(), "{:?}", w.found);
    }
}
