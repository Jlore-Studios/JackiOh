//! #96 My Pawn after it has fired: the window it leaves behind, and where the trap ends up (SPEC §3.2,
//! §4.2 step 4, §5.1, §6.3 "Cancel an attack" and Exile, R44, R99, R152). Found by the polish-4
//! edge-case hunt, round 2 (docs/polish/4-edge-cases.md, lenses L5 and L7); every case here failed
//! before its fix.
//!
//!  - §4.2 step 4, §6.3: a cancelled attack resolves no combat, so it "would be lethal" to nobody and
//!    a second My Pawn stays armed (R99) — the window no longer offers it the declaration, and it
//!    reads the trap as the board holds it after the first one's AI turn, not as it was before.
//!  - §3.2, §6.3 Exile: a My Pawn its own AI turn exiled stays in exile.
//!  - R152, §3.2: its effect is the rest of the turn it took, so it is in the graveyard by the time
//!    the next turn starts.
//!  - Round 5 (lens L7). R168, §10.10: the AI turn's events reach the view once, after the
//!    declaration that handed the turn over, not a second time ahead of it.
//!  - Round 7 (lenses "legality-agreement" and "engine invariants"). R117: the AI turn owed behind a
//!    play of the AI's that the other player's trap asked about waits for the play, so the play's Cry
//!    resolves on that turn. R44, §8 #96: a question of the locked-out player's that opens outside the
//!    AI's playout is the AI's to answer. R152: a My Pawn fused onto a My Pawn hands over one turn —
//!    its second half finds no turn of the attacker's left to hand over.
//!  - The review of round 10. R212: the AI turn My Pawn hands over happened after the window's
//!    earlier events, though the loop dispatched it first, so a card it drew does not answer them.
//!
//! Port of `packages/cards/test/my-pawn.test.ts` (SURFACE §4.1, §8).

use jackioh_engine::effects::{choose_mode, destroy};
use jackioh_engine::testkit::*;

const STOCKPILE: &str = "core-005";
const SEVEN_SEVEN: &str = "core-025";
const GIGA: &str = "core-029";
const COLLATERAL: &str = "core-034";
const GRAVEDIGGER: &str = "core-037";
const RENO: &str = "core-053";
const SORCERER: &str = "core-068";
const MY_PAWN: &str = "core-096";

fn count(s: &Scenario, type_: GameEventType) -> usize {
    s.events().iter().filter(|event| event.event_type() == type_).count()
}

fn backrow_at(s: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
    s.backrow(player, lane)
        .unwrap_or_else(|| panic!("setup: {player} should hold a backrow card in lane {lane}"))
}

/// TS's `Array.prototype.findIndex`: the first match's index, or -1.
fn find_index(events: &[GameEvent], predicate: impl Fn(&GameEvent) -> bool) -> i64 {
    events.iter().position(predicate).map_or(-1, |at| at as i64)
}

mod section_4_2_step_4_the_window_after_a_cancel {
    use super::*;

    #[test]
    fn r99_section_6_3_a_second_my_pawn_does_not_fire_on_an_attack_the_first_one_already_cancelled() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "hunt-cw2-two-pawns",
            "p1": { "field": [SORCERER], "library": [GIGA, GIGA, GIGA] },
            "p2": {
                "health": 5,
                "hand": [STOCKPILE],
                "backrow": [
                    { "def": MY_PAWN, "lane": 1, "faceUp": false },
                    { "def": MY_PAWN, "lane": 2, "faceUp": false },
                ],
                "library": [GIGA, GIGA],
            },
        }));
        let first = backrow_at(&s, PlayerId::P2, 1);
        let second = backrow_at(&s, PlayerId::P2, 2);

        s.attack(SORCERER, "hero");

        let fired: Vec<String> = s
            .events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::TrapFired { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(fired, vec![first.id.clone()]);
        s.expect_in_zone(&first, "graveyard");
        assert_eq!(s.backrow("p2", 2).map(|card| card.id), Some(second.id.clone()));
        assert_eq!(s.backrow("p2", 2).and_then(|card| card.face_up), Some(false));
        assert_eq!(count(&s, GameEventType::AttackCancelled), 1);
    }

    #[test]
    fn r152_r44_a_second_my_pawn_does_not_hand_the_players_next_turn_to_the_ai_after_the_first_ones_turn_is_over() {
        jackioh_cards::register_all();
        // p2 has nothing to do on turn 10, so R82 ends it inside the first My Pawn's AI turn and p1's
        // turn 11 begins. That turn is p1's own: nothing may play it for them.
        let mut s = scenario(json!({
            "seed": "hunt-cw2-two-pawns-next",
            "p1": { "field": [SORCERER], "library": [GIGA, GIGA, GIGA] },
            "p2": {
                "health": 5,
                "backrow": [
                    { "def": MY_PAWN, "lane": 1, "faceUp": false },
                    { "def": MY_PAWN, "lane": 2, "faceUp": false },
                ],
                "library": [GIGA, GIGA],
            },
        }));

        s.attack(SORCERER, "hero");

        assert!(s.state().result.is_none());
        assert_eq!(s.state().turn, 11);
        assert_eq!(s.state().active, PlayerId::P1);
        assert!(!s.state().players.p1.ai_turn);
        assert_eq!(count(&s, GameEventType::TrapFired), 1);
        assert_eq!(s.backrow("p2", 2).and_then(|card| card.face_up), Some(false));
    }
}

mod section_3_2_section_5_1_where_a_fired_my_pawn_ends_up {
    use super::*;

    #[test]
    fn section_6_3_a_my_pawn_exiled_during_its_own_ai_turn_stays_in_exile_it_is_not_pulled_into_the_graveyard() {
        jackioh_cards::register_all();
        // On this seed the AI turn My Pawn hands over plays p1's Collateral Damage on the face-up My
        // Pawn itself, which is still in the backrow while its effects run: it goes to p2's exile.
        let mut s = scenario(json!({
            "seed": "hunt-cw2-pawn-exiled-0",
            "p1": { "field": [SORCERER], "hand": [COLLATERAL], "library": [GIGA, GIGA, GIGA] },
            "p2": {
                "health": 5,
                "hand": [STOCKPILE],
                "backrow": [{ "def": MY_PAWN, "lane": 1, "faceUp": false }],
                "library": [GIGA, GIGA, GIGA],
            },
        }));
        let pawn = backrow_at(&s, PlayerId::P2, 1);

        s.attack(SORCERER, "hero");

        // The seed's AI does exile it; without that this test proves nothing.
        assert!(
            s.events()
                .iter()
                .any(|event| matches!(event, GameEvent::Exiled { instance_id, .. } if *instance_id == pawn.id))
        );
        s.expect_in_zone(&pawn, "exile");
        assert!(
            !s.events()
                .iter()
                .any(|event| matches!(event, GameEvent::EnteredGraveyard { instance_id, .. } if *instance_id == pawn.id))
        );
    }

    #[test]
    fn r152_my_pawn_is_in_its_owners_graveyard_once_the_ai_turn_it_gave_has_ended_before_the_next_turn_starts() {
        jackioh_cards::register_all();
        // My Pawn "fires … then goes to the graveyard" (§5.1), and its effect is the rest of p1's turn,
        // which ends at p1's cleanup (R152). So by p2's start of turn it is in p2's graveyard, and p2's
        // Gravedigger ("Start of turn: add a random card from your GY to your hand") finds it there — it
        // is the only card in that graveyard.
        let mut g = scenario(json!({
            "seed": "pawn-gravedigger",
            "p1": { "field": [{ "def": SEVEN_SEVEN, "lane": 1 }], "library": [RENO, RENO] },
            "p2": {
                "health": 5,
                "field": [{ "def": GRAVEDIGGER, "lane": 1 }],
                "backrow": [MY_PAWN],
                "library": [RENO, RENO, RENO],
            },
        }));
        let pawn = backrow_at(&g, PlayerId::P2, 1);

        g.attack(SEVEN_SEVEN, "hero");

        assert_eq!(g.state().active, PlayerId::P2);
        g.expect_health("p2", 5);
        g.expect_in_zone(&pawn, "hand");
    }
}

mod r168_section_10_8_my_pawns_ai_turn_reaches_the_view_once_in_order {
    use super::*;

    #[test]
    fn r168_r44_the_views_events_after_my_pawns_ai_turn_are_that_actions_events_once_each_in_the_order_they_happened() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "hunt-cw2-two-pawns",
            "p1": { "field": [SORCERER], "library": [GIGA, GIGA, GIGA] },
            "p2": {
                "health": 5,
                "hand": [STOCKPILE],
                "backrow": [{ "def": MY_PAWN, "lane": 1, "faceUp": false }],
                "library": [GIGA, GIGA],
            },
        }));
        // A lethal attack: My Pawn cancels it and hands the rest of p1's turn to the AI (R44), which
        // ends it, so p2's turn starts inside this one action.
        s.attack(SORCERER, "hero");
        let ended = s
            .last_events()
            .iter()
            .filter(|event| event.event_type() == GameEventType::TurnEnded)
            .count();
        assert_eq!(ended, 1);

        // The scenario began with no history, so the view's stream is exactly this action's events: the
        // declaration and the cancel first, then the AI's turn end and p2's turn start, each once. The
        // AI's own actions are part of this one (`aiPolicy.adoptState` keeps the enclosing history).
        let seen: Vec<GameEventType> = view_for(s.state(), PlayerId::P2)
            .events
            .iter()
            .map(GameEvent::event_type)
            .collect();
        let happened: Vec<GameEventType> = s.last_events().iter().map(GameEvent::event_type).collect();
        assert_eq!(seen, happened);
    }
}

// ---------------------------------------------------------------------------
// Round 7 (lenses "legality-agreement" and "engine invariants"): the AI turn's questions, and a My
// Pawn fused onto a My Pawn.
// ---------------------------------------------------------------------------

const TEMPO_TIMMY: &str = "core-011";
const JEWELOSCO_SCARAB: &str = "core-007";
const MR_VANILLA: &str = "core-008";
const JLOCKEED_SHREDDER: &str = "core-013";
const UNLICENSED_EXPERIMENTATION: &str = "core-085";

/// TS `registeredScripts()`: the registry as `registerScripts` last set it — this thread's testkit
/// override once a fixture is in (SURFACE §8), the production registry before.
fn registered_scripts_now() -> IndexMap<String, CardScripts> {
    match scripts_override() {
        Some(scripts) => scripts.clone(),
        None => registered_scripts().clone(),
    }
}

/// A fixture card: a transient def in the match state and its script in the registry.
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
        "type": type_.as_str(),
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face.clone(),
        "radiant": face,
    }));
    s.state_mut().transient_defs.insert(id.to_string(), def);
    let mut scripts = registered_scripts_now();
    scripts.insert(
        id.to_string(),
        CardScripts {
            base: script.clone(),
            radiant: script,
        },
    );
    register_scripts(scripts);
}

fn place_fixture(s: &mut Scenario, def_id: &str, player: PlayerId, row: Row, lane: i32) -> CardInstance {
    let mut card = new_instance(s.state_mut(), def_id, player, Zone::Hand { player });
    if !place_on_field(
        s.state_mut(),
        &mut card,
        ZoneSlot { player, row, lane },
        PlaceOnFieldOptions::default(),
    ) {
        panic!("could not place {def_id}");
    }
    card
}

/// A Trap that fires when its controller's opponent plays a card, asks its controller, then runs
/// `after` (TS's default `() => []` when `None`).
fn asking_trap(s: &mut Scenario, id: &str, after: Option<Hook>) {
    let after = after.unwrap_or_else(|| hook(|_ctx| vec![]));
    let prompt = format!("{id}: asked");
    let asks = TriggerDef::new(format!("{id}-asks"), &[GameEventType::CardPlayed], move |_ctx, _event| {
        vec![choose_mode(json_as(json!({
            "options": ["ok"],
            "step": "asked",
            "prompt": prompt.as_str(),
        })))]
    })
    .with_when(|ctx, event| matches!(event, GameEvent::CardPlayed { player, .. } if *player != ctx.controller));
    fixture(
        s,
        id,
        CardType::Trap,
        Script {
            triggers: vec![asks],
            resume: [("asked", after)].into_iter().collect(),
            ..Script::default()
        },
    );
}

mod r44_section_8_96_the_ai_answers_every_question_of_the_turn_it_plays {
    use super::*;

    #[test]
    fn r117_r44_r113_a_card_the_ai_played_resolves_its_cry_on_that_turn_after_the_other_players_trap_asked_about_the_play()
     {
        jackioh_cards::register_all();
        // p1's Tempo Timmy swings for lethal, so p2's My Pawn cancels it and hands the rest of p1's turn
        // to the AI (R44). The AI plays Jewelosco Scarab, and p2's trap asks p2 about that play at
        // §10.5 step 4, before the Scarab's Cry (Discover a 2-cost card).
        let mut s = scenario(json!({
            "seed": "edge-r7-lock-0",
            "p1": { "field": [TEMPO_TIMMY], "hand": [JEWELOSCO_SCARAB], "library": [RENO, RENO], "mana": 1 },
            "p2": {
                "health": 3,
                "backrow": [{ "def": MY_PAWN, "faceUp": false }],
                "hand": [RENO],
                "library": [RENO, RENO],
            },
        }));
        asking_trap(&mut s, "edge-r7-asker", None);
        place_fixture(&mut s, "edge-r7-asker", PlayerId::P2, Row::Backrow, 2);
        let turn = s.state().turn;

        s.attack(TEMPO_TIMMY, "hero");
        assert!(s.state().players.p1.ai_turn);
        assert_eq!(
            s.state().pending.as_ref().map(|pending| pending.prompt.as_str()),
            Some("edge-r7-asker: asked"),
            "p2's trap asks about the AI's Scarab"
        );
        assert_eq!(s.state().turn, turn);

        s.answer(json!("ok"));

        // §10.3: the trap resolves to completion, and then the play it interrupted goes on to step 5
        // (§10.5) before anything owed behind it: the rest of the AI turn is the enclosing sequence,
        // and waits for the play (R113, R117). So the Scarab's Discover opens on the turn the Scarab
        // was played, before that turn ends.
        let events = s.last_events();
        let discover = find_index(events, |event| {
            matches!(
                event,
                GameEvent::PromptOpened {
                    player: PlayerId::P1,
                    ..
                }
            )
        });
        let ended = find_index(events, |event| event.event_type() == GameEventType::TurnEnded);
        assert!(discover >= 0, "the Scarab's Discover opens once p2 has answered");
        assert!(ended == -1 || discover < ended, "the Scarab's Cry resolves before p1's turn ends");
        // And p1 is never left holding a question of its own turn while it is p2's turn.
        let p1_holds_one_on_p2s_turn = s
            .state()
            .pending
            .as_ref()
            .is_some_and(|pending| pending.player_id == PlayerId::P1)
            && s.state().active == PlayerId::P2;
        assert!(!p1_holds_one_on_p2s_turn);
    }

    #[test]
    fn r44_r152_a_question_of_the_locked_out_players_that_opens_during_the_ai_turn_is_the_ais_to_answer() {
        jackioh_cards::register_all();
        // p1 has a unit whose Death asks its controller. p2's trap asks p2 about the AI's play and then
        // destroys that unit, so p1's question opens during p1's AI turn, inside p2's answer.
        let mut s = scenario(json!({
            "seed": "edge-r7-lock-b",
            "p1": { "field": [TEMPO_TIMMY], "hand": [MR_VANILLA], "library": [RENO, RENO], "mana": 1 },
            "p2": {
                "health": 3,
                "backrow": [{ "def": MY_PAWN, "faceUp": false }],
                "hand": [RENO],
                "library": [RENO, RENO],
            },
        }));
        fixture(
            &mut s,
            "edge-r7-last-word",
            CardType::Unit,
            Script {
                death: Some(hook(|_ctx| {
                    vec![choose_mode(json_as(json!({
                        "options": ["ok"],
                        "step": "said",
                        "prompt": "edge-r7: a last word",
                    })))]
                })),
                resume: [("said", hook(|_ctx| vec![]))].into_iter().collect(),
                ..Script::default()
            },
        );
        let dying = place_fixture(&mut s, "edge-r7-last-word", PlayerId::P1, Row::Units, 5);
        let dying_id = dying.id.clone();
        asking_trap(
            &mut s,
            "edge-r7-killer",
            Some(hook(move |_ctx| {
                vec![destroy(json_as(json!({
                    "target": { "of": "instance", "instanceId": dying_id.as_str() },
                })))]
            })),
        );
        place_fixture(&mut s, "edge-r7-killer", PlayerId::P2, Row::Backrow, 2);

        s.attack(TEMPO_TIMMY, "hero");
        assert!(s.state().players.p1.ai_turn);
        assert_eq!(
            s.state().pending.as_ref().map(|pending| pending.prompt.as_str()),
            Some("edge-r7-killer: asked"),
            "p2's trap asks about the AI's Mr. Vanilla"
        );

        s.answer(json!("ok"));
        s.expect_in_zone(&dying, "graveyard");

        // §8 #96: "an AI plays the rest of their turn with random legal actions", and §10.7 and R44 have
        // it answer prompts from `legalActions` like any other action, while p1 is locked out until the
        // end of the turn. So a question p1's card asks during that turn is the AI's to answer; it is
        // never left open for the locked-out player, whose client does not act while `aiTurn` is set.
        let held = s.state().pending.clone();
        let locked_out_holds_it = held.as_ref().is_some_and(|held| held.player_id == PlayerId::P1)
            && s.state().players.p1.ai_turn;
        assert!(
            !locked_out_holds_it,
            "p1 is locked out but holds \"{}\"",
            held.as_ref().map_or("", |held| held.prompt.as_str())
        );
    }
}

mod r152_the_ai_turns_lockout_ends_with_the_turn_it_was_set_for {
    use super::*;

    #[test]
    fn r152_r44_r102_a_my_pawn_fused_onto_a_my_pawn_by_85_leaves_no_lockout_on_the_player_whose_turn_its_first_half_already_played_out()
     {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "r7-double-pawn",
            "active": "p2",
            "turn": 10,
            // p1 keeps a unit, so its own turn does not end by itself (R82) and the test can look at it.
            "p1": {
                "health": 5,
                "field": [MR_VANILLA],
                "backrow": [
                    { "def": MY_PAWN, "faceUp": false },
                    { "def": UNLICENSED_EXPERIMENTATION, "faceUp": false },
                ],
            },
            "p2": { "hand": [MY_PAWN], "field": [JLOCKEED_SHREDDER], "mana": 4 },
        }));

        // p2 sets its own My Pawn; p1's #85 answers the played Trap by fusing it onto p1's My Pawn (R61).
        s.play(MY_PAWN, json!({}));
        assert!(
            s.backrow("p1", 1)
                .is_some_and(|card| card.def_id.ends_with("core-096+core-096"))
        );

        // p2 swings 8 at p1's 5: the fused trap fires, cancels the attack and hands p2's turn to the AI.
        s.attack(JLOCKEED_SHREDDER, "hero");
        assert!(
            s.last_events()
                .iter()
                .any(|event| event.event_type() == GameEventType::AttackCancelled)
        );
        assert_eq!(s.state().active, PlayerId::P1);
        assert!(s.state().pending.is_none());
        // R152: the lockout ended at the cleanup of the turn My Pawn took. The fused trap's second My
        // Pawn half ran only after that turn was over and set the lockout again on p2, for p1's turn —
        // a turn no My Pawn handed to the AI.
        assert!(!s.state().players.p2.ai_turn);
        assert!(!s.state().players.p1.ai_turn);
    }
}

mod r212_my_pawns_ai_turn_happened_after_the_window_it_was_handed_over_in {
    use super::*;

    const CORPSE_EATER: &str = "core-089";

    #[test]
    fn r212_r44_a_corpse_eater_drawn_during_my_pawns_ai_turn_does_not_feed_on_a_death_a_trap_dealt_earlier_in_the_same_window()
     {
        jackioh_cards::register_all();
        // p1's Sorcerer swings for lethal. In the window, p2's lane-1 trap destroys p1's Mr. Vanilla,
        // and then My Pawn cancels the swing and hands the rest of p1's turn to the AI (R44), which ends
        // it: p2's turn starts inside this one action, and p2 draws Corpse Eater. The loop hands the
        // window's death to the triggers only after the window, AI turn included (§10.3) — but the Eater
        // reached the hand after that death, and R212 has "a #89 Corpse Eater drawn after a death not
        // feed on it".
        let mut s = scenario(json!({
            "seed": "edge-r11-pawn-eater",
            "p1": { "field": [SORCERER, { "def": MR_VANILLA, "lane": 2 }], "library": [GIGA, GIGA, GIGA] },
            "p2": {
                "health": 5,
                "backrow": [{ "def": MY_PAWN, "lane": 2, "faceUp": false }],
                "library": [CORPSE_EATER, GIGA, GIGA],
            },
        }));
        let victim = must(s.unit("p1", 2));
        let victim_id = victim.id.clone();
        let fires = TriggerDef::new(
            "edge-r11-sniper-fires",
            &[GameEventType::AttackDeclared],
            move |_ctx, _event| {
                vec![destroy(json_as(json!({
                    "target": { "of": "instance", "instanceId": victim_id.as_str() },
                })))]
            },
        )
        .with_when(|_ctx, event| matches!(event, GameEvent::AttackDeclared { forced: false, .. }));
        fixture(
            &mut s,
            "edge-r11-sniper",
            CardType::Trap,
            Script {
                triggers: vec![fires],
                ..Script::default()
            },
        );
        let sniper = place_fixture(&mut s, "edge-r11-sniper", PlayerId::P2, Row::Backrow, 1);
        // TS wrote `sniper.faceUp = false` through the live object, which is the card on the field.
        find_instance_mut(s.state_mut(), &sniper.id)
            .expect("the sniper on p2's backrow")
            .face_up = Some(false);

        s.attack(SORCERER, "hero");

        let is_victims_death =
            |event: &GameEvent| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == victim.id);
        assert!(s.events().iter().any(is_victims_death));
        assert_eq!(
            s.events()
                .iter()
                .filter(|event| event.event_type() == GameEventType::TrapFired)
                .count(),
            2
        );
        let eater = s.card(CORPSE_EATER).clone();
        s.expect_in_zone(&eater, "hand");
        let died_at = find_index(s.events(), is_victims_death);
        let drawn_at = find_index(s.events(), |event| {
            matches!(event, GameEvent::Drawn { instance_id, .. } if *instance_id == eater.id)
        });
        assert!(drawn_at > died_at);
        let fed: Vec<&GameEvent> = s
            .events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Buffed { instance_id, .. } if *instance_id == eater.id))
            .collect();
        assert!(fed.is_empty(), "{fed:?}");
    }
}

fn must<T>(value: Option<T>) -> T {
    value.unwrap_or_else(|| panic!("setup: expected a value"))
}
