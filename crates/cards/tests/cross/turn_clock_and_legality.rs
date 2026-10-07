//! The clock, the draw offer, Heroic Power's action, and "this turn" on the opponent's turn: what
//! `legalActions` offers and `reduce` accepts must agree with SPEC and with each other (§2.3, §2.5,
//! §9.3, §10.2, R36, R43, R79, R103). Found by the polish-4 edge-case hunt
//! (docs/polish/4-edge-cases.md, lenses L8 and L9, and in round 4 the engine-invariants lens, which
//! found two target options sharing one key, and in round 5 a play naming a lane between two lanes,
//! and in round 6 an answer listed in an order no offered answer has, R221, and in round 8 a play's
//! Tribute pick that names a unit the play keeps, R123, and a play's picks listed in an order no
//! offered play has, R221); every case here but the known gap failed before its fix. No Core card
//! declares a Tribute with an amount or one declaration whose two picks are made in a public order,
//! so round 8's cases build the card as a fixture.
//!
//! Port of `packages/cards/test/turn-clock-and-legality.test.ts` (SURFACE §4.1, §8).

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::effects;
use jackioh_engine::subsystems;
use jackioh_engine::testkit::*;

const SCARAB: &str = "core-007"; // Cry: Discover a 2-cost card — one prompt
const VANILLA: &str = "core-008";
const STOCKPILE: &str = "core-005";
const GARY: &str = "core-004"; // Unit, cost 1
const HINDER: &str = "core-021"; // cast on draw
const PANTHER: &str = "core-032";
const MAGIC_JAMMED: &str = "core-036";
const QUICKSTRIKER: &str = "core-038";
const SHEEPISH: &str = "core-041";
const BIG_FELINOR: &str = "core-043";
const TUTOR: &str = "core-051"; // three chained prompts
const MENACE: &str = "core-019";
const RENO: &str = "core-053";
const REMINISCE: &str = "core-072";
const FIENDER: &str = "core-092";
const HEROIC: &str = "core-098";
const CRAFT: &str = "core-099"; // two chained Discovers
const FELINORS: &str = "core-012";
const JELLY_BEAN: &str = "core-026"; // Choose a card in your hand; it becomes Radiant (radiant: choose 2)
const TWINSPELL: &str = "core-079"; // the next Spell you play gains Echo +1
const MC_TECH: &str = "classic-040"; // Radiant: "Cry: … steal one of your choice", a prompt at resolution
const CHAOS_GOLEM: &str = "core-095-1"; // a Token: no random pool or Discover may ever offer it (§5.1)
const LIBRARY: [&str; 5] = [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA];

/// `scenario(...)` over the real cards: the registry the TS harness imported.
fn game(setup: Value) -> Scenario {
    jackioh_cards::register_all();
    scenario(setup)
}

static NONCE: AtomicU32 = AtomicU32::new(0);

/// An action through `reduce` with a fresh nonce; the result as `reduce` gives it, a refusal included.
fn act(state: &GameState, body: Value) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    reduce(state, &json_as::<ActionInput>(body).with_nonce(format!("turn-clock-{nonce}")))
}

fn must<T>(value: Option<T>, what: &str) -> T {
    match value {
        Some(value) => value,
        None => panic!("expected {what}"),
    }
}

/// R103: the power names are state, so pinning one is writing what `ensurePower` writes.
fn with_power(s: &mut Scenario, card: &CardInstance, name: &str) -> CardInstance {
    let live = find_instance_mut(s.state_mut(), &card.id).expect("the Heroic Power");
    live.memory.insert(subsystems::POWER_KEY.to_string(), json!(name));
    live.clone()
}

fn event_types(events: &[GameEvent]) -> Vec<GameEventType> {
    events.iter().map(GameEvent::event_type).collect()
}

fn to_json(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).expect("serialises")
}

mod r79_a_timeout_acts_only_for_the_player_whose_clock_ran_out {
    use super::*;

    #[test]
    fn r79_the_non_active_player_s_timeout_with_nothing_of_theirs_open_does_not_end_the_active_player_s_turn() {
        let g = game(json!({
            "p1": { "hand": [VANILLA], "field": [VANILLA], "library": LIBRARY },
            "p2": { "hand": [VANILLA], "field": [VANILLA], "library": LIBRARY },
        }));
        let result = act(g.state(), json!({ "type": "timeout", "playerId": "p2" }));

        assert_eq!(result.error, None);
        assert_eq!(
            (result.state.active, result.state.turn),
            (PlayerId::P1, g.state().turn)
        );
        assert!(!event_types(&result.events).contains(&GameEventType::TurnEnded));
    }

    #[test]
    fn r79_the_non_active_player_s_timeout_does_not_answer_the_active_player_s_prompt_or_end_their_turn() {
        let mut g = game(json!({ "p1": { "hand": [SCARAB, RENO], "mana": 4 }, "p2": { "hand": [RENO] } }));
        g.play(SCARAB, json!({}));
        let pending = must(g.state().pending.clone(), "the Scarab's Discover");
        assert_eq!(pending.player_id, PlayerId::P1);

        let result = act(g.state(), json!({ "type": "timeout", "playerId": "p2" }));
        assert_eq!(
            (
                result.state.active,
                result.state.turn,
                result.state.pending.as_ref().map(|open| open.id.clone())
            ),
            (PlayerId::P1, g.state().turn, Some(pending.id.clone()))
        );
    }

    #[test]
    fn r79_the_active_player_s_timeout_answers_every_prompt_of_theirs_a_chain_opens_then_ends_the_turn_2_5() {
        // KY's Private Tutor opens three chained prompts and Craft a Card two: one timeout answers them
        // all and the turn passes.
        let cases: [(&str, Vec<&str>); 2] = [
            (TUTOR, vec![MENACE, "core-020", STOCKPILE, "core-006", "core-035"]),
            (CRAFT, LIBRARY.to_vec()),
        ];
        for (card, library) in cases {
            let mut g = game(json!({ "p1": { "hand": [card, RENO], "mana": 4, "library": library }, "p2": { "hand": [RENO] } }));
            g.play(card, json!({}));
            assert_eq!(must(g.state().pending.clone(), "the first prompt").player_id, PlayerId::P1);
            let turn = g.state().turn;

            let result = act(g.state(), json!({ "type": "timeout", "playerId": "p1" }));
            assert_eq!(result.error, None);
            assert_eq!(
                (
                    result.state.active,
                    result.state.turn,
                    result.state.pending.as_ref().map(|open| open.player_id) == Some(PlayerId::P1)
                ),
                (PlayerId::P2, turn + 1, false)
            );
        }
    }
}

mod r36_a_draw_offer_is_answered_once {
    use super::*;

    fn answer_draws(state: &GameState, player: PlayerId) -> Vec<ActionBody> {
        legal_actions(state, player)
            .into_iter()
            .filter(|action| matches!(action, ActionBody::AnswerDraw { .. }))
            .collect()
    }

    #[test]
    fn r36_a_declined_draw_offer_is_closed_it_is_no_longer_offered_and_cannot_then_be_accepted() {
        let g = game(json!({ "p1": { "hand": [RENO] }, "p2": { "hand": [RENO] } }));
        let offered = act(g.state(), json!({ "type": "offerDraw", "playerId": "p1" }));
        assert_eq!(offered.error, None);
        assert_eq!(answer_draws(&offered.state, PlayerId::P2).len(), 2);

        let declined = act(&offered.state, json!({ "type": "answerDraw", "playerId": "p2", "accept": false }));
        assert_eq!(declined.error, None);
        assert_eq!(answer_draws(&declined.state, PlayerId::P2), Vec::<ActionBody>::new());

        let accepted = act(&declined.state, json!({ "type": "answerDraw", "playerId": "p2", "accept": true }));
        assert!(accepted.error.is_some());
        assert_eq!(accepted.state.result, None);
        // And the offerer is blocked, so the offer cannot simply be made again this turn.
        assert!(
            !legal_actions(&declined.state, PlayerId::P1)
                .iter()
                .any(|action| matches!(action, ActionBody::OfferDraw))
        );
    }
}

mod r43_r103_what_an_activate_power_or_a_heroic_power_play_may_carry {
    use super::*;

    #[test]
    fn r103_r13_activate_power_refuses_a_ping_target_the_power_cannot_reach_a_dormant_card_a_hand_card_a_backrow_card() {
        let mut g = game(json!({
            "p1": { "hand": [RENO], "mana": 8, "backrow": [HEROIC] },
            "p2": {
                "hand": [MENACE],
                "field": [BIG_FELINOR, { "def": FIENDER, "stack": true }],
                "backrow": [{ "def": SHEEPISH, "faceUp": false }],
            },
        }));
        let heroic = must(g.backrow(PlayerId::P1, 1), "the Heroic Power");
        let power = with_power(&mut g, &heroic, "ping");
        let pile = must(g.state().players.p2.units[0].clone(), "p2's lane-1 pile");
        let dormant = must(pile.iter().find(|card| card.def_id == BIG_FELINOR).cloned(), "the dormant Big Felinor");
        let top = must(pile.iter().find(|card| card.def_id == FIENDER).cloned(), "the Fiender on top");
        let in_hand = must(g.state().players.p2.hand.first().cloned(), "p2's hand card");
        let trap = must(g.backrow(PlayerId::P2, 1), "p2's face-down trap");

        for target in [&dormant, &in_hand, &trap] {
            let result = act(
                g.state(),
                json!({
                    "type": "activatePower",
                    "playerId": "p1",
                    "instanceId": power.id,
                    "targets": [{ "pick": "instance", "instanceId": target.id }],
                }),
            );
            assert!(
                result.error.is_some(),
                "a ping at {} in {}",
                target.def_id,
                target.zone.z()
            );
        }

        // The top of the pile and a hero are what the ping reaches, as the prompt would offer them.
        let on_top = act(
            g.state(),
            json!({
                "type": "activatePower",
                "playerId": "p1",
                "instanceId": power.id,
                "targets": [{ "pick": "instance", "instanceId": top.id }],
            }),
        );
        assert_eq!(on_top.error, None);
        assert_eq!(
            on_top.events.iter().filter(|event| matches!(event, GameEvent::Damage { .. })).count(),
            1
        );
    }

    #[test]
    fn r103_activate_power_cannot_carry_the_discover_s_answer_so_no_card_of_the_client_s_naming_reaches_the_hand_6_3_5_1() {
        let mut g = game(json!({ "p1": { "hand": [RENO], "mana": 8, "backrow": [HEROIC] } }));
        let heroic = must(g.backrow(PlayerId::P1, 1), "the Heroic Power");
        let power = with_power(&mut g, &heroic, "discover");
        let hand_before: Vec<String> = g.state().players.p1.hand.iter().map(|card| card.def_id.clone()).collect();

        let result = act(
            g.state(),
            json!({
                "type": "activatePower",
                "playerId": "p1",
                "instanceId": power.id,
                "targets": [{ "pick": "mode", "option": CHAOS_GOLEM }],
            }),
        );

        assert!(result.error.is_some());
        assert_eq!(
            result.state.players.p1.hand.iter().map(|card| card.def_id.clone()).collect::<Vec<_>>(),
            hand_before
        );
    }

    #[test]
    fn r752_r65_heroic_power_costs_0_legal_actions_offers_no_x_choice_a_play_naming_one_is_refused_and_a_play_pays_0_2_3()
     {
        let mut g = game(json!({ "p1": { "hand": [HEROIC, RENO], "mana": 4 } }));
        let first = must(g.state().players.p1.hand.first().cloned(), "the Heroic Power in hand");
        let card = with_power(&mut g, &first, "ping");

        let plays: Vec<ActionBody> = legal_actions(g.state(), PlayerId::P1)
            .into_iter()
            .filter(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == card.id))
            .collect();
        let per_zone: IndexSet<String> = plays
            .iter()
            .map(|play| match play {
                ActionBody::Play { zone, .. } => to_json(zone),
                _ => unreachable!("filtered to plays"),
            })
            .collect();
        assert_eq!(plays.len(), per_zone.len());
        assert_eq!(
            plays
                .iter()
                .filter(|play| matches!(play, ActionBody::Play { x: Some(_), .. }))
                .cloned()
                .collect::<Vec<_>>(),
            Vec::<ActionBody>::new()
        );

        let zone = json!({ "row": "backrow", "lane": 2 });
        let with_x = act(
            g.state(),
            json!({ "type": "play", "playerId": "p1", "instanceId": card.id, "zone": zone, "x": 4 }),
        );
        assert_eq!(with_x.error.as_deref(), Some("Heroic Power does not cost X"));
        let result = act(
            g.state(),
            json!({ "type": "play", "playerId": "p1", "instanceId": card.id, "zone": zone }),
        );
        assert_eq!(result.error, None);
        let played = result.events.iter().find_map(|event| match event {
            GameEvent::CardPlayed { cost_paid, .. } => Some(*cost_paid),
            _ => None,
        });
        assert_eq!(played, Some(0));
    }
}

mod s6_2_this_turn_on_the_opponent_s_turn {
    use super::*;

    #[test]
    fn r40_r70_s6_2_a_card_cast_on_the_opponent_s_turn_counts_only_the_plays_of_that_turn_so_quickstriker_s_x_is_0() {
        let mut g = game(json!({
            "seed": "hunt-l8-stale-log",
            "p1": {
                "hand": [QUICKSTRIKER, VANILLA],
                "field": [PANTHER],
                "library": [{ "def": HINDER, "radiant": true }, VANILLA, VANILLA, VANILLA, VANILLA],
            },
            // p2's #9 Moths to the Flame, worn to 4 health: at p2's start of turn p1's Panther attacks it.
            "p2": { "hand": [VANILLA], "field": [{ "def": "core-009", "damage": 10 }], "library": LIBRARY },
        }));
        // p1's own turn: Quickstriker, then a second card (X = 1 on p2's hero).
        g.play(QUICKSTRIKER, json!({}));
        g.play(VANILLA, json!({}));
        assert_eq!(g.state().players.p2.hero.health, 29);
        // p2's turn begins: Moths makes the 5/4 Panther attack it, the Panther destroys it and survives,
        // so it draws 2 for p1 on p2's turn (R426) — Hinder first, which casts itself and counts as a card
        // p1 played this turn.
        g.end_turn();
        assert_eq!(g.state().active, PlayerId::P2);
        assert!(
            g.events()
                .iter()
                .any(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if def_id == HINDER))
        );

        // p1 played nothing earlier on this turn, so the cast deals 0.
        assert_eq!(g.state().players.p2.hero.health, 29);
    }
}

mod s10_6_a_prompt_s_options_can_each_be_picked_through_the_view {
    use super::*;

    #[test]
    fn r81_s10_6_a_target_prompt_s_options_have_distinct_keys_so_each_of_two_same_named_units_can_be_picked_10_8() {
        // Two Duplicating Felinors — #12's own copy makes this an ordinary board — among the four
        // permanents Classic #40 MC Tech's Radiant face picks from in a prompt its Cry opens (§10.6).
        let mut s = game(json!({
            "seed": "inv-r4-prompt-keys",
            "p1": { "hand": [{ "def": MC_TECH, "radiant": true }, RENO], "mana": 8 },
            "p2": { "field": [FELINORS, FELINORS, MENACE], "backrow": [{ "def": SHEEPISH, "faceUp": false }] },
        }));
        s.play(MC_TECH, json!({ "zone": 1 }));

        let pending = must(s.view(PlayerId::P1).pending, "MC Tech's target prompt");
        let PendingView::ForYou(pending) = pending else {
            panic!("the prompt should be p1's");
        };
        assert_eq!(
            pending
                .options
                .iter()
                .filter(|option| option.def_id.as_deref() == Some(FELINORS))
                .count(),
            2
        );
        // The view's contract (`PendingOption.key` in packages/shared/src/view.ts) is that the key is
        // what the client sends back, so one key names one option; two options sharing a key leave one
        // of them unpickable (the web client maps picked keys back to options through a Map).
        let keys: Vec<String> = pending.options.iter().map(|option| option.key.clone()).collect();
        let distinct: IndexSet<String> = keys.iter().cloned().collect();
        assert_eq!(distinct.len(), keys.len());
    }
}

mod s9_1_legal_actions_and_a_face_down_trap_s_instance_id {
    use super::*;

    // R177's last channel, closed by R227: an instance id is the only handle the action protocol has
    // for a face-down target (a play's targets, a prompt option, `activatePower`), so `legalActions`
    // names one whenever a card may target a face-down trap. A card set face-down takes a fresh id, so
    // a player who saw the id while the card was public (here, in p2's graveyard) finds it nowhere
    // once the card is set again: not in the actions, not in the view, not in the events.
    #[test]
    fn r227_r177_legal_actions_never_names_a_face_down_trap_by_an_id_its_viewer_saw_while_the_card_was_public() {
        let mut g = game(json!({
            "active": "p2",
            "p1": { "hand": [MAGIC_JAMMED, RENO], "mana": 4 },
            "p2": { "hand": [REMINISCE, RENO], "graveyard": [SHEEPISH], "mana": 4 },
        }));
        let trap_id = must(
            g.state().players.p2.graveyard.first().map(|card| card.id.clone()),
            "Sheepish in p2's graveyard",
        );
        assert!(to_json(&g.view(PlayerId::P1)).contains(&format!("\"{trap_id}\"")));

        g.play(REMINISCE, json!({}));
        g.answer(json!(SHEEPISH));
        let back = must(
            g.state().players.p2.hand.iter().find(|card| card.def_id == SHEEPISH).cloned(),
            "Sheepish back in hand",
        );
        assert_eq!(back.id, trap_id);
        g.play(&back.id, json!({}));
        g.end_turn();
        assert_eq!(g.state().active, PlayerId::P1);
        let set = must(g.backrow(PlayerId::P2, 1), "Sheepish set in p2's backrow lane 1");
        assert_eq!(set.def_id, SHEEPISH);
        assert_ne!(set.id, trap_id);
        assert!(!to_json(&g.view(PlayerId::P1)).contains(&format!("\"{trap_id}\"")));
        assert!(!to_json(&g.view(PlayerId::P1)).contains(&format!("\"{}\"", set.id)));

        // Magic Jammed can target the face-down trap: the action names it by the fresh id only.
        let actions = legal_actions(g.state(), PlayerId::P1);
        assert_eq!(
            actions
                .iter()
                .filter(|action| to_json(action).contains(&format!("\"{trap_id}\"")))
                .cloned()
                .collect::<Vec<_>>(),
            Vec::<ActionBody>::new()
        );
        assert!(actions.iter().any(|action| to_json(action).contains(&format!("\"{}\"", set.id))));
    }
}

mod s3_2_9_3_a_play_s_zone_is_one_of_the_row_s_lanes {
    use super::*;

    #[test]
    fn s9_3_a_play_naming_a_zone_between_two_lanes_is_refused_not_accepted_with_the_card_lost_and_its_mana_spent() {
        let s = game(json!({ "p1": { "hand": [GARY, STOCKPILE] }, "p2": { "hand": [STOCKPILE] } }));
        let gary = must(s.hand(PlayerId::P1).iter().find(|card| card.def_id == GARY).cloned(), "Gary in hand");
        let mana = s.state().players.p1.mana.current;

        // `legalActions` offers lanes 1 to 5 only…
        let offered = legal_actions(s.state(), PlayerId::P1).iter().any(|action| match action {
            ActionBody::Play { instance_id, zone, .. } => {
                *instance_id == gary.id && zone.map(|zone| f64::from(zone.lane) == 2.5).unwrap_or(false)
            }
            _ => false,
        });
        assert!(!offered);

        // …and §9.3 has `reduce` refuse what is illegal itself. Lane 2.5 passed the range check and read
        // as an empty, unlocked zone, so the play was accepted: the mana was spent and the card written to
        // `units[1.5]`, a property no lane reads, which the next JSON clone dropped — in no zone at all.
        //
        // A lane is an integer on the Rust wire (`ZoneChoice.lane: i32`, SURFACE §4.3), so the play is
        // refused where the action is read, before `reduce` could accept it: the same JSON the TS case
        // sends does not parse into an `Action`, and nothing is spent.
        let sent = json!({
            "type": "play",
            "playerId": "p1",
            "nonce": "lane-2.5",
            "instanceId": gary.id,
            "zone": { "row": "units", "lane": 2.5 },
        });
        let parsed = serde_json::from_value::<Action>(sent);
        assert!(parsed.is_err(), "a play naming lane 2.5 was read as {parsed:?}");
        assert_eq!(s.state().players.p1.mana.current, mana);
    }
}

mod r221_10_2_10_6_every_answer_reduce_accepts_is_one_legal_actions_offers {
    use super::*;

    #[test]
    fn r221_r16_r60_radiant_26_glowy_jelly_bean_s_two_echo_picks_listed_the_other_way_round_mean_the_same_as_the_answer_legal_actions_offers()
     {
        // #80 Zao Gao's chosen discard was this test's two-pick prompt until patch v0.1.1 made that
        // discard random (R354). A radiant Glowy Jelly Bean under #79 Twinspell asks the same shape:
        // its Echo repeat reopens its hand pick as a prompt for two cards (§10.6).
        let mut s = game(json!({
            "p1": {
                "hand": [TWINSPELL, { "def": JELLY_BEAN, "radiant": true }, RENO, VANILLA, BIG_FELINOR, GARY],
                "library": [RENO, RENO],
                "mana": 5,
            },
            "p2": { "hand": [RENO], "library": [RENO] },
        }));
        s.play(TWINSPELL, json!({ "zone": 1 }));
        let picks: Vec<Value> = [RENO, VANILLA]
            .iter()
            .map(|id| json!({ "pick": "instance", "instanceId": s.card(*id).id }))
            .collect();
        s.play(JELLY_BEAN, json!({ "targets": picks }));
        let state = s.state().clone();
        assert_eq!(state.pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Hand));
        assert_eq!(state.pending.as_ref().map(|pending| pending.max), Some(2));

        let offered: Vec<ActionBody> = legal_actions(&state, PlayerId::P1)
            .into_iter()
            .filter(|action| matches!(action, ActionBody::Answer { .. }))
            .collect();
        let mut meanings: IndexMap<String, String> = IndexMap::new();
        for answer in &offered {
            let result = reduce(&state, &Action::new(answer.clone(), PlayerId::P1, "edge-r6-offered"));
            assert_eq!(result.error, None);
            meanings.insert(hash_state(&result.state), graveyard_ids(&result.state));
        }

        // The same two cards, listed the other way round: reduce takes it (no prompt answer is
        // order-checked, R60's "N different cards" is a set), so it has to be a play legalActions offers.
        for answer in &offered {
            let ActionBody::Answer { choice_id, selection } = answer else {
                unreachable!("filtered to answers");
            };
            let mut reversed_selection = selection.clone();
            reversed_selection.reverse();
            let reversed = ActionBody::Answer {
                choice_id: choice_id.clone(),
                selection: reversed_selection.clone(),
            };
            let result = reduce(&state, &Action::new(reversed, PlayerId::P1, "edge-r6-reversed"));
            assert_eq!(result.error, None);
            let graveyard = graveyard_ids(&result.state);
            assert!(
                meanings.contains_key(&hash_state(&result.state)),
                "answer {} leaves the graveyard {graveyard}, which no offered answer does (offered: {})",
                to_json(&reversed_selection),
                meanings.values().cloned().collect::<Vec<_>>().join(" | ")
            );
        }
    }

    fn graveyard_ids(state: &GameState) -> String {
        state
            .players
            .p1
            .graveyard
            .iter()
            .map(|card| card.id.clone())
            .collect::<Vec<_>>()
            .join(",")
    }
}

// ---------------------------------------------------------------------------------------------
// Round 8 (lens "legality-agreement"): a play's own choices, as legalActions offers them
// ---------------------------------------------------------------------------------------------

const EDGE_VANILLA: &str = "core-008";

/// `registerScripts({ ...registeredScripts(), [id]: { base: script, radiant: script } })`.
fn register_fixture_script(id: &str, script: Script) {
    let mut scripts = registered_scripts().clone();
    scripts.insert(
        id.to_string(),
        CardScripts {
            base: script.clone(),
            radiant: script,
        },
    );
    register_scripts(scripts);
}

/// A test-only card, registered the way the other hunt cases register theirs.
fn fixture(s: &mut Scenario, id: &str, type_: CardType, script: Script) {
    let face = if type_ == CardType::Unit {
        json!({ "attack": 2, "health": 2, "keywords": [], "text": id })
    } else {
        json!({ "keywords": [], "text": id })
    };
    let def: CardDef = json_as(json!({
        "id": id,
        "index": id,
        "name": id,
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face.clone(),
        "radiant": face,
    }));
    s.state_mut().transient_defs.insert(id.to_string(), def);
    register_fixture_script(id, script);
}

fn in_hand(s: &mut Scenario, def_id: &str, player: PlayerId) -> CardInstance {
    let card = new_instance(s.state_mut(), def_id, player, Zone::Hand { player });
    s.state_mut().players[player].hand.push(card.clone());
    card
}

/// The plays `legalActions` offers for `card` (TS `PlayBody[]`).
fn plays_of(s: &Scenario, player: PlayerId, card: &CardInstance) -> Vec<ActionBody> {
    legal_actions(s.state(), player)
        .into_iter()
        .filter(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == card.id))
        .collect()
}

/// `play.targets ?? []`.
fn targets_of(play: &ActionBody) -> Vec<Selection> {
    match play {
        ActionBody::Play { targets, .. } => targets.clone().unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// `play.tributes ?? []`.
fn tributes_of(play: &ActionBody) -> Vec<String> {
    match play {
        ActionBody::Play { tributes, .. } => tributes.clone().unwrap_or_default(),
        _ => Vec::new(),
    }
}

mod r123_a_declared_tribute_names_the_same_units_in_targets_and_tributes {
    use super::*;

    #[test]
    fn r123_legal_actions_offers_and_reduce_accepts_no_play_whose_declared_tribute_pick_is_a_unit_the_play_does_not_tribute()
     {
        let mut s = game(json!({
            "p1": { "field": [EDGE_VANILLA, EDGE_VANILLA], "hand": [EDGE_VANILLA] },
            "p2": { "hand": [EDGE_VANILLA] },
        }));
        // R123's shape: one `tribute` declaration with a pick AND an amount. The tributed unit travels
        // in `tributes` (§6.3's cost, paid at §10.5 step 2) and in `targets` (what the script reads as
        // the unit it tributed): "a card that declares both therefore names its units in both lists".
        fixture(
            &mut s,
            "edge-r8-devourer",
            CardType::Unit,
            Script {
                targets: vec![json_as::<TargetDecl>(json!({ "kind": "tribute", "amount": 1, "min": 1, "max": 1 }))],
                ..Script::default()
            },
        );
        let devourer = in_hand(&mut s, "edge-r8-devourer", PlayerId::P1);
        let a = must(s.unit(PlayerId::P1, 1), "p1's first unit");
        let b = must(s.unit(PlayerId::P1, 2), "p1's second unit");

        let plays = plays_of(&s, PlayerId::P1, &devourer);
        assert!(!plays.is_empty());
        let mismatched: Vec<&ActionBody> = plays
            .iter()
            .filter(|play| {
                let tributes = tributes_of(play);
                targets_of(play).iter().any(|pick| match pick {
                    Selection::Instance { instance_id } => !tributes.contains(instance_id),
                    _ => false,
                })
            })
            .collect();
        // Offered before the fix: tributes [a] with the pick b, and tributes [b] with the pick a — the unit
        // sacrificed and the unit the script is told it sacrificed are two different units.
        // (TS `expect.soft`: a soft assertion lets the rest run; this one stops the test, a stricter form.)
        assert_eq!(
            mismatched
                .iter()
                .map(|play| (tributes_of(play), targets_of(play)))
                .collect::<Vec<_>>(),
            Vec::<(Vec<String>, Vec<Selection>)>::new(),
            "offered plays whose declared Tribute pick is not a unit they tribute"
        );

        let result = reduce(
            s.state(),
            &json_as::<Action>(json!({
                "type": "play",
                "playerId": "p1",
                "nonce": "edge-r8-r123",
                "instanceId": devourer.id,
                "zone": { "row": "units", "lane": 3 },
                "tributes": [a.id],
                "targets": [{ "pick": "instance", "instanceId": b.id }],
            })),
        );
        assert!(
            result.error.is_some(),
            "a play that tributes one unit and names another as its Tribute pick"
        );
    }
}

mod r221_r90_10_2_a_play_s_picks_for_one_declaration_are_a_set {
    use super::*;

    fn play(s: &Scenario, spell: &CardInstance, targets: &[String], nonce: &str) -> ReduceResult {
        let picks: Vec<Value> = targets
            .iter()
            .map(|instance_id| json!({ "pick": "instance", "instanceId": instance_id }))
            .collect();
        reduce(
            s.state(),
            &json_as::<Action>(json!({
                "type": "play",
                "playerId": "p1",
                "nonce": nonce,
                "instanceId": spell.id,
                "targets": picks,
            })),
        )
    }

    #[test]
    fn r221_r90_a_play_listing_one_declaration_s_two_picks_the_other_way_round_means_the_same_as_the_play_legal_actions_offers_3()
     {
        let mut s = game(json!({
            "p1": { "hand": [EDGE_VANILLA] },
            "p2": { "field": [EDGE_VANILLA, EDGE_VANILLA], "hand": [EDGE_VANILLA] },
        }));
        // One declaration that picks two enemy units, exiled in turn. The exile pile is public and
        // chronological (§3), so the order the picks are exiled in is on the table for both seats.
        fixture(
            &mut s,
            "edge-r8-exile-two",
            CardType::Spell,
            Script {
                targets: vec![json_as::<TargetDecl>(json!({
                    "kind": "target",
                    "min": 2,
                    "max": 2,
                    "filter": { "side": "enemy", "of": ["unit"] },
                }))],
                cry: Some(hook(|_ctx| {
                    vec![
                        effects::exile(json_as(json!({ "target": { "of": "chosen", "index": 0 } }))),
                        effects::exile(json_as(json!({ "target": { "of": "chosen", "index": 1 } }))),
                    ]
                })),
                ..Script::default()
            },
        );
        let spell = in_hand(&mut s, "edge-r8-exile-two", PlayerId::P1);
        let a = must(s.unit(PlayerId::P2, 1), "p2's lane-1 unit");
        let b = must(s.unit(PlayerId::P2, 2), "p2's lane-2 unit");

        // legalActions offers the set {a, b} once, in the order the declaration offers its options.
        assert_eq!(
            plays_of(&s, PlayerId::P1, &spell)
                .iter()
                .map(|play| {
                    targets_of(play)
                        .iter()
                        .map(|pick| match pick {
                            Selection::Instance { instance_id } => instance_id.clone(),
                            _ => "?".to_string(),
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>(),
            vec![vec![a.id.clone(), b.id.clone()]]
        );

        let offered = play(&s, &spell, &[a.id.clone(), b.id.clone()], "edge-r8-order-offered");
        let reversed = play(&s, &spell, &[b.id.clone(), a.id.clone()], "edge-r8-order-reversed");
        assert_eq!(offered.error, None);

        // `reduce` may refuse the unoffered listing, or accept it as it accepts an answer's (R221) —
        // but then it must mean what the offered play means: the same cards, exiled in the same order.
        if reversed.error.is_some() {
            return;
        }
        let exile_ids = |state: &GameState| -> Vec<String> {
            state.players.p2.exile.iter().map(|card| card.id.clone()).collect()
        };
        assert_eq!(
            exile_ids(&reversed.state),
            exile_ids(&offered.state),
            "the public exile pile after the reversed listing"
        );
    }
}
