//! The stages of the turn loop, each settled before the next (SPEC §2.2, §6.2, §10.3, R44, R62, R68,
//! R152). Found by the polish-4 edge-case hunt, round 6 (docs/polish/4-edge-cases.md, lens L8);
//! every case here failed before its fix.
//!
//!  - R62, §10.3: a stage that emits events settles before the next one runs, so a trigger answering
//!    the end-of-turn trap window resolves in that turn's end, and one answering a start-of-turn
//!    delayed effect resolves before the start-of-turn triggers are queued behind it.
//!  - R44, R152: the AI turn #96 My Pawn hands over goes on after the other player answers a question
//!    one of its actions put to them.
//!
//! Round 7 (lens L8) added two: the traps answer each start-of-turn delayed effect before the next one
//! resolves (R68, §10.3), and cleanup settles its own events — My Pawn reaching the graveyard at the
//! end of the turn it took (R152) — before the turn-cap check and the next turn (R62).
//!
//! Round 8 (lens L8) added two more: the deaths the check after a delayed effect collects reach the
//! traps before the next delayed effect resolves (R68, §4.5), and a "this turn" modifier made while
//! cleanup's own events are answered ends with that turn rather than lasting for good (§2.2, R62).
//!
//! No Core card answers a summon or a change of control, and no Core trap asks its controller
//! anything, so the card that makes each case observable is a fixture (a transient def, the way a
//! fusion's is held, as paused-sequences.test.ts does); every other card is a real one.
//!
//! Round 9 (lens L8) added two: cleanup clears the return flags again once its own events are
//! answered, so a return Spell cast then does not come back two turns later (R155, R62), and an
//! end-of-turn clause a Spell arms on the other player's turn is not armed at all (R241, §6.2).
//! Then, from the lens "engine invariants": the refresh's rider is a badge the view lists and the
//! refresh reports spent (R169, §6.3 Mana), and a fatigue draw Armor absorbs is still reported (R240),
//! since patch v0.3.X by the pipeline's own `damageAbsorbed` (R1362).
//!
//! Port of `packages/cards/test/turn-stages.test.ts` (SURFACE §4.1, §8).

use jackioh_engine::effects;
use jackioh_engine::testkit::*;

const VANILLA: &str = "core-008"; // Unit, cost 1, no Cry
const SUPPRESSIVE_AURA: &str = "core-046"; // radiant: enemy units -4/-4
const LUNAR_ECLIPSE: &str = "core-035"; // 3 damage; the next Spell you play this turn costs 1 less
const TEMPO_TIMMY: &str = "core-011"; // 3/3 Rush, First Strike
const BREAD_AND_BUTTER: &str = "core-018"; // Field Trap in the end-of-turn window (R62)
const ECHOES: &str = "core-040"; // Start of your turn: damage to the enemy hero = cards in your exile
const KPOP_FANATIC: &str = "core-050"; // Cry: at the start of your next turn, steal the chosen permanent
const RENO: &str = "core-053";
const MASOCHISM_MASK: &str = "core-065"; // Start of turn: a mode prompt
const MY_PAWN: &str = "core-096";
const GARY: &str = "core-004";
const SEVEN_SEVEN: &str = "core-025";

/// The actions a player takes for themselves on their own turn (everything but concede and draws).
const TURN_ACTIONS: [ActionType; 6] = [
    ActionType::Play,
    ActionType::Attack,
    ActionType::SwitchPosition,
    ActionType::ActivatePower,
    ActionType::OfferDraw,
    ActionType::EndTurn,
];

use super::scenario;

fn must<T>(value: Option<T>, what: &str) -> T {
    match value {
        Some(value) => value,
        None => panic!("expected {what}"),
    }
}

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

/// A fixture card: a transient def in the match state and its script in the registry (a Unit's
/// `stats` default to 2/2).
fn fixture(s: &mut Scenario, id: &str, type_: CardType, script: Script, stats: Option<AttackHealth>) {
    let stats = stats.unwrap_or(AttackHealth { attack: 2, health: 2 });
    let face = if type_ == CardType::Unit {
        json!({ "attack": stats.attack, "health": stats.health, "keywords": [], "text": id })
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

fn place_fixture(s: &mut Scenario, def_id: &str, player: PlayerId, row: Row, lane: i32) -> CardInstance {
    let mut card = new_instance(s.state_mut(), def_id, player, Zone::Hand { player });
    if !place_on_field(
        s.state_mut(),
        &mut card,
        ZoneSlot { player, row, lane },
        Default::default(),
    ) {
        panic!("could not place {def_id}");
    }
    let live = find_instance_mut(s.state_mut(), &card.id).expect("the placed fixture");
    if row == Row::Units {
        live.position = Some(Position::Atk);
    }
    live.clone()
}

fn any_event(s: &Scenario, pick: impl Fn(&GameEvent) -> bool) -> bool {
    s.events().iter().any(pick)
}

fn winner(s: &Scenario) -> Option<Winner> {
    s.state().result.map(|result| result.winner)
}

/// `{ of: "enemyHero" }` takes `amount` (the fixtures' one effect).
fn hit_enemy_hero(amount: i32) -> Effect {
    effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": amount })))
}

mod r62_10_3_a_stage_of_the_turn_loop_settles_its_events_before_the_next_stage {
    use super::*;

    #[test]
    fn r62_r216_a_trigger_answering_the_end_of_turn_trap_window_resolves_in_that_turn_s_end_before_the_next_turn_starts_10_3()
     {
        // p1 ends the turn with 1 mana unspent: p1's Bread and Butter summons a Bread Token for p1 in
        // the window, and p1's fixture unit answers that summon with 1 damage to p2's hero, which is at
        // 1. p2's Echoes of the Forgotten would deal p1 (at 2) 3 at the start of p2's turn.
        let mut s = scenario(json!({
            "p1": {
                "hand": [RENO],
                "backrow": [{ "def": BREAD_AND_BUTTER, "faceUp": false }],
                "mana": 1,
                "health": 2,
                "library": [RENO, RENO],
            },
            "p2": { "hand": [RENO], "backrow": [ECHOES], "exile": [RENO, RENO, RENO], "health": 1, "library": [RENO, RENO] },
        }));
        fixture(
            &mut s,
            "edge-r6-juggler",
            CardType::Unit,
            Script {
                triggers: vec![TriggerDef::new(
                    "edge-r6-juggle",
                    &[GameEventType::Summoned],
                    |ctx, event| {
                        if matches!(event, GameEvent::Summoned { player, .. } if *player == ctx.controller) {
                            vec![hit_enemy_hero(1)]
                        } else {
                            vec![]
                        }
                    },
                )],
                ..Script::default()
            },
            None,
        );
        place_fixture(&mut s, "edge-r6-juggler", PlayerId::P1, Row::Units, 3);

        s.end_turn();

        // §10.3: the window's firing is an effect like any other, so the trigger its summon wakes
        // resolves before R62 moves on to the delayed effects, cleanup and the turn cap; p2 is at 0 in
        // that check and loses (§4.5 step 2) at the end of p1's turn. p2's turn never starts, so
        // Echoes never fires (R216).
        assert!(any_event(&s, |event| matches!(
            event,
            GameEvent::TrapFired { .. }
        )));
        assert_eq!(winner(&s), Some(Winner::P1));
        assert!(!any_event(&s, |event| matches!(
            event,
            GameEvent::TurnStarted {
                player: PlayerId::P2,
                ..
            }
        )));
    }

    #[test]
    fn r62_r68_a_trigger_answering_a_start_of_turn_delayed_effect_resolves_before_the_start_of_turn_triggers_6_2()
     {
        // p1's K-Pop Fanatic steals p2's Tempo Timmy at the start of p1's next turn. p1's fixture unit
        // answers the change of control with 1 damage to p2's hero, which is at 1. p1's Masochism Mask
        // (backrow) asks p1 something at the start of the turn.
        let mut s = scenario(json!({
            "p1": { "hand": [KPOP_FANATIC, RENO], "backrow": [MASOCHISM_MASK], "library": [RENO, RENO] },
            "p2": { "field": [TEMPO_TIMMY], "hand": [RENO], "health": 1, "library": [RENO, RENO] },
        }));
        fixture(
            &mut s,
            "edge-r6-bounty",
            CardType::Unit,
            Script {
                triggers: vec![TriggerDef::new(
                    "edge-r6-bounty",
                    &[GameEventType::ControlChanged],
                    |ctx, event| {
                        if matches!(event, GameEvent::ControlChanged { controller, .. } if *controller == ctx.controller)
                        {
                            vec![hit_enemy_hero(1)]
                        } else {
                            vec![]
                        }
                    },
                )],
                ..Script::default()
            },
            None,
        );
        place_fixture(&mut s, "edge-r6-bounty", PlayerId::P1, Row::Units, 4);
        let timmy = must(s.unit(PlayerId::P2, 1), "p2's Tempo Timmy");
        s.play(
            KPOP_FANATIC,
            json!({ "targets": [{ "pick": "instance", "instanceId": timmy.id }] }),
        );

        s.start_turn();

        // §6.2 and R62: the delayed effects first, then the trigger queue. The steal's trigger is queued
        // by the steal, before the start-of-turn stage begins, and by R68 a unit's trigger comes before a
        // backrow card's anyway: p2 is at 0 in the check after it and loses before the Mask asks.
        assert!(any_event(&s, |event| matches!(
            event,
            GameEvent::ControlChanged { instance_id, .. } if *instance_id == timmy.id
        )));
        assert_eq!(winner(&s), Some(Winner::P1));
        assert_eq!(s.state().pending, None);
    }
}

mod r44_r152_my_pawn_s_ai_plays_the_rest_of_the_turn {
    use super::*;

    #[test]
    fn r44_r152_the_ai_turn_goes_on_after_the_other_player_answers_a_prompt_one_of_its_actions_opened_10_3_8_96()
     {
        // p1's 3/3 Timmy swings at p2's hero at 3: lethal, so p2's My Pawn cancels it and hands the
        // rest of p1's turn to the AI (R44). p2's fixture trap asks p2 something when p1 plays a card.
        let mut s = scenario(json!({
            "p1": { "field": [TEMPO_TIMMY], "hand": [VANILLA, VANILLA, VANILLA], "mana": 4 },
            "p2": { "health": 3, "backrow": [{ "def": MY_PAWN, "faceUp": false }], "hand": [RENO], "library": [RENO, RENO] },
        }));
        fixture(
            &mut s,
            "edge-r6-asker",
            CardType::Trap,
            Script {
                triggers: vec![
                    TriggerDef::new("edge-r6-asks", &[GameEventType::CardPlayed], |_ctx, _event| {
                        vec![effects::choose_mode(json_as(json!({
                            "options": ["ok"],
                            "step": "asked",
                            "prompt": "edge-r6: asked",
                        })))]
                    })
                    .with_when(|ctx, event| {
                        matches!(event, GameEvent::CardPlayed { player, .. } if *player != ctx.controller)
                    }),
                ],
                resume: IndexMap::from([("asked", hook(|_ctx| vec![]))]),
                ..Script::default()
            },
            None,
        );
        place_fixture(&mut s, "edge-r6-asker", PlayerId::P2, Row::Backrow, 2);
        let turn = s.state().turn;

        s.attack(TEMPO_TIMMY, "hero");
        // The AI took p1's turn and played a card, and p2's trap is asking p2 about it.
        assert!(s.state().players.p1.ai_turn);
        assert!(s.last_events().iter().any(|event| matches!(
            event,
            GameEvent::CardPlayed {
                player: PlayerId::P1,
                ..
            }
        )));
        assert_eq!(
            s.state().pending.as_ref().map(|pending| pending.prompt.clone()),
            Some("edge-r6: asked".to_string())
        );

        s.answer(json!("ok"));

        // §10.3: the trap resolves to completion, its prompt included, and then the action it
        // interrupted continues — here the AI turn (§8 #96: "an AI plays the rest of their turn"). The
        // AI goes on until it ends p1's turn, which is where R152 ends the lockout; p1's own client is
        // never handed the turn back.
        assert!(s.state().turn > turn);
        assert!(!s.state().players.p1.ai_turn);
    }
}

mod r44_r152_a_locked_out_player_is_never_handed_back_the_turn_my_pawn_gave_the_ai {
    use super::*;

    #[test]
    fn r44_r152_once_the_other_player_answers_a_prompt_the_ai_turn_ran_into_the_rest_of_the_turn_is_still_the_ai_s_8_96()
     {
        // The seed only fixes which of p1's actions the AI draws: here it attacks the fixture unit with
        // one of its 1/1s early in the playout, with plays and switches still left to take.
        let mut s = scenario(json!({
            "seed": "edge-r6-pawn-1",
            "p1": {
                "field": [SEVEN_SEVEN, KPOP_FANATIC, GARY],
                "hand": [VANILLA, VANILLA],
                "library": [RENO, RENO, RENO],
            },
            "p2": {
                "health": 5,
                "backrow": [{ "def": MY_PAWN, "faceUp": false }],
                "hand": [RENO],
                "library": [RENO, RENO, RENO],
            },
        }));
        // p2's 1/1 whose Death asks its controller something.
        fixture(
            &mut s,
            "edge-r6-asking-death",
            CardType::Unit,
            Script {
                death: Some(hook(|_ctx| {
                    vec![effects::choose_mode(json_as(
                        json!({ "options": ["keep", "drop"], "step": "picked" }),
                    ))]
                })),
                resume: IndexMap::from([("picked", hook(|_ctx| vec![]))]),
                ..Script::default()
            },
            Some(AttackHealth { attack: 1, health: 1 }),
        );
        place_fixture(&mut s, "edge-r6-asking-death", PlayerId::P2, Row::Units, 3);

        // A lethal swing: My Pawn cancels it, and the AI plays out the rest of p1's turn (R44).
        s.attack("4-mana 7/7", "hero");
        assert!(s.state().players.p1.ai_turn);
        // The AI killed the fixture unit, whose Death asks p2: the AI turn waits on p2's answer.
        assert_eq!(
            s.state().pending.as_ref().map(|pending| pending.player_id),
            Some(PlayerId::P2)
        );

        s.answer(json!("keep"));

        // R44: "it plays out the turn while the opponent is locked out", and R152 ends the lockout only
        // at the cleanup of that turn. So p1 is never offered, and never allowed, an action of its own
        // on this turn: the AI finishes it, whatever p2 was asked on the way.
        let offered: Vec<ActionBody> = legal_actions(s.state(), PlayerId::P1)
            .into_iter()
            .filter(|action| TURN_ACTIONS.contains(&action.action_type()))
            .collect();
        assert_eq!(
            offered,
            Vec::<ActionBody>::new(),
            "p1 is locked out on turn {}, yet offered its own turn",
            s.state().turn
        );
        let own = reduce(
            s.state(),
            &json_as::<Action>(json!({ "type": "endTurn", "playerId": "p1", "nonce": "edge-r6-locked-out" })),
        );
        assert!(
            own.error.is_some(),
            "reduce accepted an action from the locked-out player"
        );
    }
}

mod r68_10_3_a_trap_answers_a_delayed_effect_before_the_next_delayed_effect_runs {
    use super::*;

    #[test]
    fn r68_r59_r76_a_trap_answering_the_first_of_two_start_of_turn_delayed_steals_fires_before_the_second_steal_10_3()
     {
        // p1 plays two K-Pop Fanatics: one on p2's Mr. Vanilla, one on p2's Tempo Timmy. Both steals are
        // due at the start of p1's next turn, in that order (R68). p2's fixture trap answers the
        // opponent taking one of p2's permanents by returning all of p2's units to p2's hand.
        let mut s = scenario(json!({
            "p1": { "hand": [KPOP_FANATIC, KPOP_FANATIC, RENO], "library": [RENO, RENO, RENO] },
            "p2": { "field": [VANILLA, TEMPO_TIMMY], "hand": [RENO], "library": [RENO, RENO, RENO] },
        }));
        fixture(
            &mut s,
            "edge-r7-reclaimer",
            CardType::Trap,
            Script {
                triggers: vec![
                    TriggerDef::new("edge-r7-reclaim", &[GameEventType::ControlChanged], |_ctx, _event| {
                        vec![effects::bounce_all(json_as(json!({ "side": "self" })))]
                    })
                    .with_when(|ctx, event| {
                        matches!(event, GameEvent::ControlChanged { controller, .. } if *controller != ctx.controller)
                    }),
                ],
                ..Script::default()
            },
            None,
        );
        place_fixture(&mut s, "edge-r7-reclaimer", PlayerId::P2, Row::Backrow, 1);
        let vanilla = must(s.unit(PlayerId::P2, 1), "p2's Mr. Vanilla");
        let timmy = must(s.unit(PlayerId::P2, 2), "p2's Tempo Timmy");
        s.play(
            KPOP_FANATIC,
            json!({ "targets": [{ "pick": "instance", "instanceId": vanilla.id }] }),
        );
        s.play(
            KPOP_FANATIC,
            json!({ "targets": [{ "pick": "instance", "instanceId": timmy.id }] }),
        );

        // p1's turn ends, p2 passes, and p1's next turn starts with the two steals.
        s.end_turn();
        s.end_turn();
        assert_eq!(s.state().active, PlayerId::P1);

        // The first steal takes Mr. Vanilla. §10.3: its event goes to the traps, which fire at once —
        // a delayed effect is a whole effect like any other (R59), and a trap is a response — so p2's
        // trap returns Tempo Timmy to p2's hand before the second delayed effect runs, and that steal
        // fizzles on a target that has left the field (R76, R174).
        assert!(any_event(&s, |event| matches!(
            event,
            GameEvent::TrapFired { .. }
        )));
        s.expect_in_zone(&timmy.id, "hand");
        assert_eq!(s.card(&timmy.id).owner, PlayerId::P2);
        assert!(
            !any_event(&s, |event| matches!(
                event,
                GameEvent::ControlChanged { instance_id, .. } if *instance_id == timmy.id
            )),
            "the second delayed steal ran before the trap answered the first"
        );
    }
}

mod r62_10_3_cleanup_s_events_are_answered_before_the_turn_cap_check_and_the_next_turn {
    use super::*;

    #[test]
    fn r62_r152_a_trigger_answering_my_pawn_reaching_the_graveyard_at_cleanup_resolves_before_the_next_turn_starts_10_3()
     {
        // p1's Tempo Timmy (3/3) swings at p2's hero at 3: lethal, so p2's My Pawn cancels it and the AI
        // plays the rest of p1's turn (R44). p1 has nothing left to do, so the AI ends the turn, and
        // R152 sends My Pawn to p2's graveyard at that turn's cleanup. p2's fixture unit answers a card
        // entering p2's graveyard with 1 damage to the enemy hero, and p1 is at 1.
        let mut s = scenario(json!({
            "p1": { "field": [TEMPO_TIMMY], "health": 1, "library": [RENO, RENO] },
            "p2": { "health": 3, "backrow": [{ "def": MY_PAWN, "faceUp": false }], "hand": [RENO], "library": [RENO, RENO] },
        }));
        fixture(
            &mut s,
            "edge-r7-grave-watcher",
            CardType::Unit,
            Script {
                triggers: vec![TriggerDef::new(
                    "edge-r7-grave-watch",
                    &[GameEventType::EnteredGraveyard],
                    |ctx, event| {
                        if matches!(event, GameEvent::EnteredGraveyard { owner, .. } if *owner == ctx.controller)
                        {
                            vec![hit_enemy_hero(1)]
                        } else {
                            vec![]
                        }
                    },
                )],
                ..Script::default()
            },
            None,
        );
        place_fixture(&mut s, "edge-r7-grave-watcher", PlayerId::P2, Row::Units, 3);
        let pawn = must(s.backrow(PlayerId::P2, 1), "p2's My Pawn");

        s.attack(TEMPO_TIMMY, "hero");

        // My Pawn reached p2's graveyard at p1's cleanup (R152), and the trigger answering it killed p1.
        assert!(any_event(&s, |event| matches!(
            event,
            GameEvent::EnteredGraveyard { instance_id, .. } if *instance_id == pawn.id
        )));
        assert_eq!(winner(&s), Some(Winner::P2));
        // §10.3, R62: cleanup is a stage of p1's turn like the window and the delayed effects, so what
        // its events wake resolves there — p1 loses at the end of p1's turn, and p2's turn never starts.
        assert!(
            !any_event(&s, |event| matches!(
                event,
                GameEvent::TurnStarted {
                    player: PlayerId::P2,
                    ..
                }
            )),
            "p2's turn started before the trigger answering p1's cleanup resolved"
        );
    }
}

mod r68_4_5_a_delayed_effect_s_check_is_answered_before_the_next_delayed_effect {
    use super::*;

    #[test]
    fn r68_a_trap_answering_a_death_the_first_start_of_turn_delayed_effect_caused_fires_before_the_second_delayed_effect_10_3_4_5()
     {
        // p2's radiant Suppressive Aura shrinks p2's enemies by -2/-2. p1's two K-Pop Fanatics take p2's
        // Tempo Timmy and then p2's Mr. Vanilla at the start of p1's next turn, in that order (R68).
        // Timmy (3/3 with 1 damage) stolen onto p1's side is at 0 health there and dies in the check
        // after the first steal. p2's fixture trap answers one of p2's own units dying by returning p2's units to hand.
        let mut s = scenario(json!({
            "p1": { "hand": [KPOP_FANATIC, KPOP_FANATIC, RENO], "mana": 5, "library": [RENO, RENO, RENO] },
            "p2": {
                "field": [{ "def": TEMPO_TIMMY, "damage": 1 }, VANILLA],
                "backrow": [{ "def": SUPPRESSIVE_AURA, "radiant": true }],
                "hand": [RENO],
                "library": [RENO, RENO, RENO],
            },
        }));
        fixture(
            &mut s,
            "edge-r8-mourner",
            CardType::Trap,
            Script {
                triggers: vec![
                    TriggerDef::new("edge-r8-mourn", &[GameEventType::Destroyed], |_ctx, _event| {
                        vec![effects::bounce_all(json_as(json!({ "side": "self" })))]
                    })
                    .with_when(|ctx, event| {
                        matches!(event, GameEvent::Destroyed { owner, .. } if *owner == ctx.controller)
                    }),
                ],
                ..Script::default()
            },
            None,
        );
        place_fixture(&mut s, "edge-r8-mourner", PlayerId::P2, Row::Backrow, 3);
        let timmy = must(s.unit(PlayerId::P2, 1), "p2's Tempo Timmy");
        let vanilla = must(s.unit(PlayerId::P2, 2), "p2's Mr. Vanilla");
        s.play(
            KPOP_FANATIC,
            json!({ "targets": [{ "pick": "instance", "instanceId": timmy.id }] }),
        );
        s.play(
            KPOP_FANATIC,
            json!({ "targets": [{ "pick": "instance", "instanceId": vanilla.id }] }),
        );

        // p1 keeps 3 mana for Reno, so the turn does not end by itself (R82); p2 passes.
        s.end_turn();
        s.end_turn();
        assert_eq!(s.state().active, PlayerId::P1);

        // The first steal takes Timmy, which dies in the check that follows that whole delayed effect
        // (§4.5, R59). §10.3: a trap is a response and fires at once, and R68 has each delayed effect's
        // consequences answered before the next delayed effect resolves, as settle dispatches a check's
        // deaths before anything else pops. So p2's trap returns Mr. Vanilla to p2's hand, and the
        // second steal fizzles on a target that has left the field (R76, R174).
        assert!(any_event(&s, |event| matches!(
            event,
            GameEvent::Destroyed { instance_id, .. } if *instance_id == timmy.id
        )));
        assert!(any_event(&s, |event| matches!(
            event,
            GameEvent::TrapFired { .. }
        )));
        assert!(
            !any_event(&s, |event| matches!(
                event,
                GameEvent::ControlChanged { instance_id, .. } if *instance_id == vanilla.id
            )),
            "the second delayed steal ran before the trap answered the death the first one caused"
        );
        s.expect_in_zone(&vanilla.id, "hand");
    }
}

mod r62_2_2_a_this_turn_effect_made_after_cleanup_ends_with_that_turn {
    use super::*;

    #[test]
    fn r62_a_this_turn_discount_made_while_cleanup_s_events_are_answered_ends_with_that_turn_instead_of_lasting_for_good_2_2()
     {
        // p1 plays Lunar Eclipse on turn N and plays no Spell after it, so cleanup expires its discount
        // (§2.2) and reports the removal. p1's fixture unit answers one of p1's modifiers ending, on turn
        // N only, with "this turn your cards cost 1 less" (/fullsend's rider) for the turn that is ending.
        // Round 7 made cleanup's events answered at the end of turn N (R62), which is where it lands.
        let mut s = scenario(json!({
            "p1": { "hand": [LUNAR_ECLIPSE, RENO, RENO], "library": [RENO, RENO, RENO] },
            "p2": { "hand": [RENO], "library": [RENO, RENO, RENO] },
        }));
        let turn_n = s.state().turn;
        fixture(
            &mut s,
            "edge-r8-afterglow",
            CardType::Unit,
            Script {
                triggers: vec![TriggerDef::new(
                    "edge-r8-afterglow",
                    &[GameEventType::ModifierChanged],
                    move |ctx, event| {
                        let ending = matches!(
                            event,
                            GameEvent::ModifierChanged { player, added, .. } if *player == ctx.controller && !*added
                        );
                        if ending && ctx.state.turn == turn_n {
                            vec![effects::add_player_modifier(json_as(json!({
                                "player": "self",
                                "mod": {
                                    "kind": "costDiscount",
                                    "amount": 1,
                                    "expiry": { "until": "thisTurn", "turn": ctx.state.turn },
                                },
                            })))]
                        } else {
                            vec![]
                        }
                    },
                )],
                ..Script::default()
            },
            None,
        );
        place_fixture(&mut s, "edge-r8-afterglow", PlayerId::P1, Row::Units, 3);

        s.play(
            LUNAR_ECLIPSE,
            json!({ "targets": [{ "pick": "hero", "player": "p2" }] }),
        );
        // p1 keeps 3 mana for Reno (no R82 auto-end), ends turn N; p2 ends turn N+1.
        s.end_turn();
        assert_eq!(s.state().turn, turn_n + 1);
        s.end_turn();
        assert_eq!((s.state().turn, s.state().active), (turn_n + 2, PlayerId::P1));

        // §2.2: "Cleanup expires every 'this turn' effect". Whatever was made for turn N lasts at most
        // to the end of turn N — two turns on, p1's Reno costs its printed 3, and no turn-N rider is left.
        let leftover: Vec<PlayerModifier> = s
            .state()
            .players
            .p1
            .mods
            .iter()
            .filter(|modifier| {
                matches!(modifier.kind, ModifierKind::CostDiscount { .. })
                    && matches!(modifier.expiry, ModifierExpiry::ThisTurn { turn } if turn == turn_n)
            })
            .cloned()
            .collect();
        assert_eq!(
            leftover,
            Vec::<PlayerModifier>::new(),
            "a turn-N 'this turn' discount is still live on turn N+2"
        );
        let reno = must(
            s.hand(PlayerId::P1)
                .iter()
                .find(|card| card.def_id == RENO)
                .cloned(),
            "p1's Reno",
        );
        assert_eq!(effective_cost(s.state(), &reno, CostOptions::default()), 3);
    }
}

// ---------------------------------------------------------------------------
// Round 9: a Spell's clauses belong to the turn it was played on (R155, R241, §6.2, R62)
// ---------------------------------------------------------------------------

const PREM_PANTHER: &str = "core-032"; // 5/4 Rush; after it attacks and survives, draw 2 per Unit destroyed
const MOTHS: &str = "core-009"; // 1/14; start of turn: every enemy Unit attacks this

/// Put a fresh instance of `defId` on top of a player's library (index 0 is the top, §3).
fn on_top_of_library(s: &mut Scenario, def_id: &str, player: PlayerId) -> CardInstance {
    let card = new_instance(s.state_mut(), def_id, player, Zone::Library { player });
    s.state_mut().players[player].library.insert(0, card.clone());
    card
}

mod r155_a_return_spell_cast_after_cleanup_does_not_come_back_on_a_later_turn {
    use super::*;

    #[test]
    fn r155_r62_a_spell_with_an_end_of_turn_return_cast_while_cleanup_s_events_are_answered_stays_in_the_graveyard_at_the_end_of_its_caster_s_next_turn_5_1()
     {
        // p1 plays Lunar Eclipse on turn N and no Spell after it, so cleanup expires its discount and
        // reports the removal (§2.2). p1's fixture unit answers that removal, on turn N only, by drawing
        // a card: p1's fixture Spell, cast on draw (§2.4, R70), whose text is #23's "End of turn: returns
        // from the GY to your hand". Round 7 made cleanup's events answered at the end of turn N (R62),
        // so the Spell is played on turn N, after that turn's end-of-turn triggers have run: it does not
        // come back at the end of turn N, and R155 says it "stays in the graveyard rather than coming
        // back at the end of a later turn it was not played on".
        let mut s = scenario(json!({
            "p1": { "hand": [LUNAR_ECLIPSE, RENO, RENO], "library": [RENO, RENO, RENO] },
            "p2": { "hand": [RENO], "library": [RENO, RENO, RENO] },
        }));
        let turn_n = s.state().turn;

        // #23's return, verbatim in shape: the flag §10.5 step 7 writes, or a play this turn (R155).
        fixture(
            &mut s,
            "edge-r9-boomerang",
            CardType::Spell,
            Script {
                static_flags: Some(StaticFlags {
                    cast_on_draw: Some(true),
                    ..Default::default()
                }),
                cry: Some(hook(|_ctx| vec![])),
                end_of_turn: Some(hook(|ctx| {
                    let Some(self_) = ctx.live_self() else {
                        return vec![];
                    };
                    let returns = self_.return_to_hand_at_end_of_turn == Some(true)
                        || was_played_this_turn(&*ctx.state, self_.controller, &self_.id);
                    if returns {
                        vec![effects::bounce(json_as(json!({ "target": { "of": "self" } })))]
                    } else {
                        vec![]
                    }
                })),
                ..Script::default()
            },
            None,
        );
        fixture(
            &mut s,
            "edge-r9-cleanup-reader",
            CardType::Unit,
            Script {
                triggers: vec![TriggerDef::new(
                    "edge-r9-cleanup-reader",
                    &[GameEventType::ModifierChanged],
                    move |ctx, event| {
                        let ending = matches!(
                            event,
                            GameEvent::ModifierChanged { player, added, .. } if *player == ctx.controller && !*added
                        );
                        if ending && ctx.state.turn == turn_n {
                            vec![effects::draw(json_as(json!({ "count": 1 })))]
                        } else {
                            vec![]
                        }
                    },
                )],
                ..Script::default()
            },
            None,
        );
        place_fixture(&mut s, "edge-r9-cleanup-reader", PlayerId::P1, Row::Units, 3);
        let boomerang = on_top_of_library(&mut s, "edge-r9-boomerang", PlayerId::P1);

        s.play(
            LUNAR_ECLIPSE,
            json!({ "targets": [{ "pick": "hero", "player": "p2" }] }),
        );
        s.end_turn();
        assert_eq!(s.state().turn, turn_n + 1);
        // Cast at cleanup on turn N: it is in p1's graveyard, not back in hand.
        s.expect_in_zone(&boomerang.id, "graveyard");

        s.end_turn();
        assert_eq!((s.state().turn, s.state().active), (turn_n + 2, PlayerId::P1));
        // p2's turn end is not p1's (§6.2): the Spell is still in the graveyard as p1's turn N+2 begins.
        s.expect_in_zone(&boomerang.id, "graveyard");
        s.end_turn();
        assert_eq!(s.state().turn, turn_n + 3);

        // The end of turn N+2 is p1's own turn end, but the Spell was not played on it.
        assert_eq!(
            s.card(&boomerang.id).zone.z(),
            ZoneName::Graveyard,
            "the Spell cast at turn N's cleanup came back at the end of turn N+2"
        );
    }
}

mod r241_r155_r71_a_spell_s_end_of_turn_clause_belongs_to_the_turn_it_was_played_on_6_2 {
    use super::*;

    #[test]
    fn r241_r155_r70_a_spell_cast_on_the_opponent_s_turn_with_78_s_at_end_of_turn_exile_your_hand_does_not_exile_its_caster_s_hand_at_the_end_of_the_caster_s_next_turn_6_2()
     {
        // At p2's start of turn p2's #9 Moths to the Flame (worn to 4 health) makes p1's Prem Panther
        // (5/4) attack it: the Panther kills it and survives, so p1 draws 2 on p2's turn (#32, R426). The
        // top card is a cast-on-draw Spell carrying /fullsend's clause verbatim in shape — `delay({ at: {
        // phase: "end", player: "self" } })` re-entering an `exileHand` step — so p1 casts it on p2's turn
        // (§2.4, R70).
        let mut s = scenario(json!({
            "p1": { "field": [PREM_PANTHER], "hand": [RENO], "library": [RENO, RENO, RENO, RENO] },
            "p2": { "field": [{ "def": MOTHS, "damage": 10 }], "hand": [RENO], "library": [RENO, RENO, RENO, RENO] },
        }));
        fixture(
            &mut s,
            "edge-r9-late-exile",
            CardType::Spell,
            Script {
                static_flags: Some(StaticFlags {
                    cast_on_draw: Some(true),
                    ..Default::default()
                }),
                cry: Some(hook(|_ctx| {
                    vec![effects::delay(json_as(json!({
                        "at": { "phase": "end", "player": "self" },
                        "step": "exile",
                        "hook": RESUME_HOOK,
                    })))]
                })),
                resume: IndexMap::from([(
                    "exile",
                    hook(|_ctx| vec![effects::exile_hand(json_as(json!({ "player": "self" })))]),
                )]),
                ..Script::default()
            },
            None,
        );
        let cod = on_top_of_library(&mut s, "edge-r9-late-exile", PlayerId::P1);

        s.end_turn();
        assert_eq!(s.state().active, PlayerId::P2);
        assert!(any_event(&s, |event| matches!(
            event,
            GameEvent::CardPlayed { instance_id, .. } if *instance_id == cod.id
        )));

        // p2's turn (the one the Spell was cast on) ends, and p1's next turn starts with its draw.
        s.end_turn();
        assert_eq!(s.state().active, PlayerId::P1);
        let drawn_on_own_turn = must(
            s.hand(PlayerId::P1).last().cloned(),
            "the card p1 drew at the start of its turn",
        );

        // p1's own turn ends. §6.2 makes "End of turn" the controller's turn end, and R155 reads it for
        // a Spell cast on the other player's turn: that turn's end is not its controller's, so nothing of
        // the clause happens "at the end of a later turn it was not played on". /fullsend's own riders
        // say the same: its "this turn" discount and Combo draw were p2's turn's and ended with it
        // (§2.2), so an exile at the end of p1's turn is an exile no "this turn" of the card ever
        // covered — and #39's "every other card you played this turn" would read a turn log the Spell
        // was never in (R71).
        s.end_turn();
        assert_eq!(s.state().active, PlayerId::P2);
        assert_eq!(
            s.card(&drawn_on_own_turn.id).zone.z(),
            ZoneName::Hand,
            "the end-of-turn clause of a Spell cast on p2's turn exiled p1's hand at the end of p1's next turn"
        );
    }
}

// ---------------------------------------------------------------------------
// Round 9: the start of a turn reports what it changes (R169, R240, §10.3)
// ---------------------------------------------------------------------------

const HINDER: &str = "core-021"; // Cast on draw: the opponent's next mana refresh is 1 lower
const HIT_JOB: &str = "core-016"; // a Spell with no hand trigger
const STOCKPILE: &str = "core-005"; // likewise
const GOING_LONG: &str = "core-084"; // Field Spell: your hero has Armor 2

/// The `damage` and `damageAbsorbed` events on p2's hero among `events`.
fn hits_on_p2_hero(events: &[GameEvent]) -> Vec<GameEvent> {
    events
        .iter()
        .filter(|event| match event {
            GameEvent::Damage { target_id, .. } | GameEvent::DamageAbsorbed { target_id, .. } => {
                target_id == "hero-p2"
            }
            _ => false,
        })
        .cloned()
        .collect()
}

/// R240's report, R1362's form of it: `{ type: "damageAbsorbed", sourceId: null, targetId: "hero-p2",
/// absorbed: 1, combat: false }`, the whole of the 1st fatigue hit taken by the Armor.
fn absorbed_fatigue_report() -> GameEvent {
    GameEvent::DamageAbsorbed {
        source_id: None,
        target_id: "hero-p2".to_string(),
        absorbed: 1,
        combat: false,
    }
}

fn modifier_ids(modifiers: &[ModifierView]) -> Vec<String> {
    modifiers.iter().map(|modifier| modifier.id.clone()).collect()
}

mod r169_r240_what_the_start_of_a_turn_changes_it_reports_10_3 {
    use super::*;

    #[test]
    fn r169_every_modifier_a_modifier_changed_event_announces_is_on_the_view_s_badge_list_and_is_reported_gone_when_hinder_s_refresh_spends_it_10_3_6_3_mana()
     {
        let mut s = scenario(json!({
            "seed": "r9-inv-hinder",
            "p1": { "hand": [HIT_JOB], "field": [TEMPO_TIMMY], "library": [HINDER, TEMPO_TIMMY, TEMPO_TIMMY, TEMPO_TIMMY] },
            "p2": { "hand": [STOCKPILE], "field": [TEMPO_TIMMY], "library": [STOCKPILE, STOCKPILE, STOCKPILE] },
        }));
        s.end_turn(); // p2's turn
        s.end_turn(); // p1's draw casts Hinder: p2's next refresh is 1 lower (§8 #21)
        // R431 (balance patch 1): the base face then discards 1 at random with no prompt — here p1's
        // only card left in hand, HIT_JOB.
        let hit_job = s.card(HIT_JOB).id.clone();
        s.expect_in_zone(&hit_job, "graveyard");

        // BUILD M5-T4: `modifierChanged` is the badge by the hero appearing or fading, and "badge list
        // equals the view's modifiers"; R169 puts that list on both seats under the id the event names.
        let announced: Vec<String> = s
            .events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::ModifierChanged {
                    player: PlayerId::P2,
                    modifier_id,
                    added: true,
                } => Some(modifier_id.clone()),
                _ => None,
            })
            .collect();
        for id in &announced {
            assert!(
                modifier_ids(&s.view(PlayerId::P2).you.modifiers).contains(id),
                "announced {id}"
            );
            assert!(
                modifier_ids(&s.view(PlayerId::P1).opponent.modifiers).contains(id),
                "announced {id}"
            );
        }

        s.end_turn(); // p2's refresh spends the rider
        s.expect_mana(PlayerId::P2, 3);
        // Whatever was announced as added and is not on the list any more was reported gone (§10.3).
        let listed = modifier_ids(&s.view(PlayerId::P2).you.modifiers);
        for id in announced.iter().filter(|added| !listed.contains(added)) {
            assert!(
                s.events().iter().any(|event| matches!(
                    event,
                    GameEvent::ModifierChanged { player: PlayerId::P2, modifier_id, added: false } if modifier_id == id
                )),
                "removal of {id}"
            );
        }
    }

    #[test]
    fn r240_r1362_r3_a_fatigue_draw_that_going_long_s_armor_absorbs_still_reports_itself_since_the_public_fatigue_count_moved_10_3()
     {
        let mut s = scenario(json!({
            "seed": "r9-inv-fatigue",
            "p1": { "hand": [HIT_JOB], "field": [TEMPO_TIMMY], "library": [TEMPO_TIMMY, TEMPO_TIMMY] },
            "p2": { "hand": [STOCKPILE], "field": [TEMPO_TIMMY], "backrow": [GOING_LONG], "library": [] },
        }));
        assert_eq!(s.view(PlayerId::P1).opponent.hero.armor, 2);
        assert_eq!(s.view(PlayerId::P1).opponent.fatigue_count, 0);

        s.end_turn(); // p2's turn: its draw meets an empty library, and the 1st fatigue deals 1 (R3)

        // Armor 2 takes the whole 1 (§4.4 step 2), so the hero keeps 30, and the hit is no damage
        // instance (R63). The fatigue still happened: the count both seats read went from 0 to 1, and
        // the next one deals 2.
        assert_eq!(s.state().players.p2.hero.health, 30);
        assert_eq!(s.state().players.p2.fatigue_count, 1);
        assert_eq!(s.view(PlayerId::P1).opponent.fatigue_count, 1);
        // §10.3: "every visible state change emits an event", so the draw reports itself — R1362: by
        // the pipeline's `damageAbsorbed` from no source on p2's hero, once, the whole 1 absorbed, and
        // by nothing else: no `damage` (not even the 0 R240 once wrote), and R3 draws no card, so
        // there is no `drawn` for p2.
        assert_eq!(hits_on_p2_hero(s.last_events()), vec![absorbed_fatigue_report()]);
        assert!(!s.last_events().iter().any(|event| matches!(
            event,
            GameEvent::Drawn {
                player: PlayerId::P2,
                ..
            }
        )));
    }

    /// TS `onHeroHit(ctx)`: a `damage` event whose target is a hero.
    /// R1361: a report of a hit Armor took whole counts here too, so only the engine keeps it unanswered.
    fn on_hero_hit(event: &GameEvent) -> bool {
        match event {
            GameEvent::Damage { target_id, .. } | GameEvent::DamageAbsorbed { target_id, .. } => {
                target_id.starts_with("hero-")
            }
            _ => false,
        }
    }

    #[test]
    fn r240_r1362_r63_the_report_of_an_absorbed_fatigue_draw_is_answered_by_no_trigger_and_no_trap() {
        let mut s = scenario(json!({
            "seed": "r11-fatigue-report",
            "p1": { "hand": [HIT_JOB], "field": [TEMPO_TIMMY], "library": [TEMPO_TIMMY, TEMPO_TIMMY] },
            "p2": { "hand": [STOCKPILE], "field": [TEMPO_TIMMY], "backrow": [GOING_LONG], "library": [] },
        }));
        // p1's unit and p1's face-down trap each answer any hit on a hero by dealing 1 to p2's hero.
        fixture(
            &mut s,
            "fixture:r11-hero-hit-watcher",
            CardType::Unit,
            Script {
                // R1361: it listens for the report too, which still wakes nothing.
                triggers: vec![TriggerDef::new(
                    "on-hero-hit",
                    &[GameEventType::Damage, GameEventType::DamageAbsorbed],
                    |_ctx, event| {
                        if on_hero_hit(event) {
                            vec![hit_enemy_hero(1)]
                        } else {
                            vec![]
                        }
                    },
                )],
                ..Script::default()
            },
            None,
        );
        fixture(
            &mut s,
            "fixture:r11-hero-hit-trap",
            CardType::Trap,
            Script {
                triggers: vec![
                    TriggerDef::new(
                        "on-hero-hit",
                        &[GameEventType::Damage, GameEventType::DamageAbsorbed],
                        |_ctx, _event| vec![hit_enemy_hero(1)],
                    )
                    .with_when(|_ctx, event| on_hero_hit(event)),
                ],
                ..Script::default()
            },
            None,
        );
        let watcher = place_fixture(
            &mut s,
            "fixture:r11-hero-hit-watcher",
            PlayerId::P1,
            Row::Units,
            3,
        );
        let trap = place_fixture(&mut s, "fixture:r11-hero-hit-trap", PlayerId::P1, Row::Backrow, 3);
        find_instance_mut(s.state_mut(), &trap.id)
            .expect("the fixture trap")
            .face_up = Some(false);

        s.end_turn(); // p2's draw meets an empty library; Going Long's Armor 2 takes the whole 1

        // The draw is reported by `damageAbsorbed` (R1362), and it is no damage instance (R63, R1361):
        // the unit queues nothing for it, the trap stays set, and p2's hero keeps its 30.
        assert_eq!(hits_on_p2_hero(s.last_events()), vec![absorbed_fatigue_report()]);
        assert!(!s.last_events().iter().any(|event| matches!(
            event,
            GameEvent::Damage { source_id: Some(source), .. } if *source == watcher.id
        )));
        assert!(
            !s.last_events()
                .iter()
                .any(|event| matches!(event, GameEvent::TrapFired { .. }))
        );
        assert_eq!(
            s.backrow(PlayerId::P1, 3).map(|card| card.id.clone()),
            Some(trap.id.clone())
        );
        assert_eq!(s.state().players.p2.hero.health, 30);
    }
}
