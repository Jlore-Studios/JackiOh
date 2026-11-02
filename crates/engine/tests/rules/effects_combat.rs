//! The §6.3 combat verbs a card script needs: Forced attack (#9 Moths to the Flame, #60 Bear
//! Honeypot), Cancel an attack (#96 My Pawn) and the AI turn behind it (§10.7, R44, R84).
//! The effects are thin wrappers, so the tests assert who is compelled, in what order, against what
//! target, and leave §4.3 and §4.4 to the combat tests.

use jackioh_engine::effects::{
    ai_plays_out_turn, cancel_attack, forced_attacks, forced_attacks_on, redirect_attack, summon,
};
use jackioh_engine::testkit::*;
use jackioh_engine::{
    PlayerId::{P1, P2},
    Row::{Backrow, Units},
};

use super::fixtures::combat::{big_body, plain, taunter};
use super::fixtures::harness::{Bench, events_of_type, in_hand, new_game, put, slot};
use crate::rules::fixtures::harness::{json_of, live, live_mut};

// Fixture cards fixtures/combat.rs does not carry.

fn def_of_kind(name: &str, index: &str, type_: &str, overrides: Value) -> CardDef {
    let mut def = json!({
        "id": format!("fc-{name}"),
        "index": index,
        "name": format!("{name} (forced combat)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "attack": 1, "health": 1, "keywords": [], "text": name },
        "radiant": { "attack": 2, "health": 2, "keywords": [], "text": format!("{name} radiant") },
    });
    if let (Value::Object(fields), Value::Object(extra)) = (&mut def, overrides) {
        for (key, value) in extra {
            fields.insert(key, value);
        }
    }
    json_as(def)
}

fn backrow_faces(text: &str) -> Value {
    json!({ "base": { "keywords": [], "text": text }, "radiant": { "keywords": [], "text": text } })
}

/// R53's "stops once the target has left the field": a 1/1 that dies to the first forced attack.
fn frail() -> CardDef {
    def_of_kind("frail", "941", "Unit", json!({}))
}
/// A backrow card to be the `self` of a trap's script, the way #96 My Pawn is (§4.2 step 4).
fn ambush() -> CardDef {
    def_of_kind("ambush", "942", "Trap", backrow_faces("trap"))
}
/// A Field Trap that answers every declaration and does nothing (R61); a Field Trap stays after firing,
/// so a second offer of one declaration would fire it again, which R100 forbids.
fn watcher() -> CardDef {
    def_of_kind("watcher", "943", "Field Trap", backrow_faces("field trap"))
}
/// A Trap whose whole body is §6.3's Cancel an attack, the way #96 My Pawn's first clause is.
fn canceller() -> CardDef {
    def_of_kind("canceller", "944", "Trap", backrow_faces("trap"))
}
/// A Trap that asks its controller something inside the window, so the window has to pause.
fn asker() -> CardDef {
    def_of_kind("asker", "945", "Trap", backrow_faces("trap"))
}
/// A Trap shaped like #96 itself: cancel the declaration, then hand the turn to the AI policy.
fn pawn() -> CardDef {
    def_of_kind("pawn", "946", "Trap", backrow_faces("trap"))
}
/// A Field Trap that answers every `manaChanged`, so a second dispatch of one is countable.
fn meter() -> CardDef {
    def_of_kind("meter", "947", "Field Trap", backrow_faces("field trap"))
}
/// A Trap shaped like #64 Traitorous Blood's base face: re-aim the declaration at the attacker's
/// first acting neighbour (R1122).
fn redirector() -> CardDef {
    def_of_kind("redirector", "948", "Trap", backrow_faces("trap"))
}
/// A Trap shaped like #64's Radiant face: the same re-aim, then a copy of what the combat destroyed
/// for its controller (R1123).
fn redirector_copies() -> CardDef {
    def_of_kind("redirector-copies", "949", "Trap", backrow_faces("trap"))
}

/// §8 #9's body, from the shared combat fixtures: 1/14, so three forced attacks do not kill it.
const MOTHS: &str = "cb-moths";
/// §7's shared Rush Token: 3/3 Rush, which is what #60 summons and compels.
const RUSH_TOKEN: &str = "fx-token-rush";

fn defs() -> Vec<CardDef> {
    vec![
        frail(),
        ambush(),
        watcher(),
        canceller(),
        asker(),
        pawn(),
        meter(),
        redirector(),
        redirector_copies(),
    ]
}

/// §6.3 Redirect's attack half (R1122, R1123), the way #64 Traitorous Blood calls it.
fn redirect_script(copy_destroyed: bool) -> CardScripts {
    let script = Script {
        triggers: vec![TriggerDef::new(
            "fc-attackDeclared",
            &[GameEventType::AttackDeclared],
            move |ctx, event| {
                let GameEvent::AttackDeclared { attacker_id, .. } = event else {
                    return Vec::new();
                };
                let Some(attacker) = find_instance(ctx.state, attacker_id) else {
                    return Vec::new();
                };
                let neighbours = redirect_neighbours(ctx.state, attacker);
                let Some(to) = neighbours.into_iter().next() else {
                    return Vec::new();
                };
                vec![redirect_attack(json_as(json!({
                    "to": to.id,
                    "copyDestroyed": copy_destroyed,
                })))]
            },
        )],
        ..Script::default()
    };
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

/// A resume nothing can service: answering the prompt just clears it (§10.6).
fn inert_resume() -> Resume {
    Resume {
        def_id: "fc-no-script".into(),
        hook: "resume".into(),
        step: "none".into(),
        radiant: false,
        instance_id: None,
        data: IndexMap::new(),
    }
}

fn mode_options(options: &[&str]) -> Vec<PromptOption> {
    options
        .iter()
        .map(|option| PromptOption {
            key: format!("mode:{option}"),
            label: option.to_string(),
            selection: Selection::Mode {
                option: option.to_string(),
            },
            cost: None,
            radiant: None,
        })
        .collect()
}

fn inert_prompt(player: PlayerId, kind: &str, prompt: &str, options: &[&str]) -> OpenPromptArgs {
    json_as(json!({
        "player": player,
        "kind": kind,
        "prompt": prompt,
        "options": mode_options(options),
        "resume": inert_resume(),
    }))
}

/// An effect that opens a prompt for the resolving controller and nothing else (§9.3, §10.6).
fn ask() -> Effect {
    Effect::new("fc:ask", |ctx| {
        let args = inert_prompt(ctx.controller, "mode", "the window pauses here", &["a", "b"]);
        open_prompt(ctx, args);
    })
}

/// The trap scripts the window tests need. Each watches `attackDeclared` with no `when`, so it
/// answers every declaration it is offered (R99) — which is what makes a second offer visible.
fn trap_script(on: GameEventType, run: fn() -> Vec<Effect>) -> CardScripts {
    let script = Script {
        triggers: vec![TriggerDef::new(format!("fc-{on}"), &[on], move |_ctx, _event| {
            run()
        })],
        ..Script::default()
    };
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn fc_scripts() -> IndexMap<String, CardScripts> {
    let mut scripts = IndexMap::new();
    scripts.insert(
        watcher().id,
        trap_script(GameEventType::AttackDeclared, std::vec::Vec::new),
    );
    scripts.insert(
        canceller().id,
        trap_script(GameEventType::AttackDeclared, || {
            vec![cancel_attack(Default::default())]
        }),
    );
    scripts.insert(
        asker().id,
        trap_script(GameEventType::AttackDeclared, || vec![ask()]),
    );
    // #96's own body, in its order (§8.5): cancel, then hand the rest of the turn to §10.7's policy.
    scripts.insert(
        pawn().id,
        trap_script(GameEventType::AttackDeclared, || {
            vec![
                cancel_attack(Default::default()),
                ai_plays_out_turn(json_as(json!({ "player": "enemy" }))),
            ]
        }),
    );
    scripts.insert(
        meter().id,
        trap_script(GameEventType::ManaChanged, std::vec::Vec::new),
    );
    scripts.insert(redirector().id, redirect_script(false));
    scripts.insert(redirector_copies().id, redirect_script(true));
    scripts
}

fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut scripts = registered_scripts().clone();
    scripts.extend(fc_scripts());
    register_scripts(scripts);
    state.turn = 5;
    state.phase = Phase::Main;
    state
}

#[derive(Default)]
struct RunOptions {
    controller: Option<PlayerId>,
    self_: Option<CardInstance>,
    targets: Option<Vec<Selection>>,
}

/// Apply a whole effect list the way a hook's list is applied: one context, one rng, in order.
fn run_all(state: &mut GameState, effects: Vec<Effect>, options: RunOptions) -> Vec<GameEvent> {
    let RunOptions {
        controller,
        self_,
        targets,
    } = options;
    let self_ = self_.map(|card| find_instance(state, &card.id).cloned().unwrap_or(card));
    let mut sink = Bench::new(state);
    {
        let mut inner = sink.sink(state);
        let mut ctx = make_context(
            &mut inner,
            self_.as_ref(),
            HookOptions {
                controller: Some(controller.unwrap_or(P1)),
                targets,
                ..Default::default()
            },
        );
        for effect in &effects {
            (effect.apply)(&mut ctx);
        }
    }
    let cursor = sink.rng.cursor();
    let events = std::mem::take(&mut sink.events);
    state.rng_cursor = cursor;
    events
}

fn run(state: &mut GameState, effect: Effect, options: RunOptions) -> Vec<GameEvent> {
    run_all(state, vec![effect], options)
}

fn apply_as(mut sink: EngineSink<'_>, effect: Effect, controller: PlayerId) {
    let mut ctx = make_context(
        &mut sink,
        None,
        HookOptions {
            controller: Some(controller),
            ..Default::default()
        },
    );
    (effect.apply)(&mut ctx);
}

fn declarations(events: &[GameEvent]) -> Vec<Value> {
    events_of_type(events, GameEventType::AttackDeclared)
        .iter()
        .map(|event| match event {
            GameEvent::AttackDeclared {
                attacker_id,
                target_id,
                forced,
                ..
            } => {
                json!({ "attackerId": attacker_id, "targetId": target_id, "forced": forced })
            }
            other => panic!("not an attackDeclared: {other:?}"),
        })
        .collect()
}

fn attacker_ids(events: &[GameEvent]) -> Vec<String> {
    declarations(events)
        .iter()
        .map(|event| event["attackerId"].as_str().expect("an attacker id").to_string())
        .collect()
}

fn unused() -> Exertion {
    Exertion {
        attacked: false,
        switched: false,
        attacks: None,
    }
}

fn field_of(events: &[GameEvent], of: GameEventType, key: &str) -> Vec<Value> {
    events_of_type(events, of)
        .iter()
        .map(|event| json_of(event)[key].clone())
        .collect()
}

mod forced_attacks_on_s6_3_forced_attack_s4_2_r53_c9 {
    use super::*;

    #[test]
    fn r53_every_enemy_unit_attacks_the_target_in_lane_order_ignoring_position_sickness_and_taunt() {
        let mut state = game("moths");
        state.active = P2;
        let target = put(&mut state, MOTHS, slot(P1, Units, 1), json!({}));
        // §4.2 step 3 would force the attacks onto this unit; a forced attack skips the step entirely.
        let wall = put(&mut state, &taunter.id, slot(P1, Units, 2), json!({}));

        let defending = put(&mut state, &plain.id, slot(P2, Units, 1), json!({}));
        live_mut(&mut state, &defending).position = Some(Position::Def);
        let sick = put(&mut state, &plain.id, slot(P2, Units, 2), json!({}));
        let turn = state.turn;
        live_mut(&mut state, &sick).summoned_turn = Some(turn);
        let ready = put(&mut state, &plain.id, slot(P2, Units, 3), json!({}));

        let events = run(
            &mut state,
            forced_attacks_on(json_as(
                json!({ "target": { "of": "self" }, "attackers": "enemy" }),
            )),
            RunOptions {
                self_: Some(target.clone()),
                controller: Some(P1),
                ..Default::default()
            },
        );

        assert_eq!(
            declarations(&events),
            vec![
                json!({ "attackerId": defending.id, "targetId": target.id, "forced": true }),
                json!({ "attackerId": sick.id, "targetId": target.id, "forced": true }),
                json!({ "attackerId": ready.id, "targetId": target.id, "forced": true }),
            ]
        );
        assert_eq!(live(&state, &target).damage, 9);
        assert_eq!(live(&state, &wall).damage, 0);

        // No exertion is spent, so each attacker may still take its own attack on its own turn.
        for attacker in [&defending, &sick, &ready] {
            assert_eq!(live(&state, attacker).exertion, unused());
        }
        // Position is ignored, not changed: the Defense unit's Armor +1 takes Moths' 1 attack to 0 (§4.1).
        assert_eq!(live(&state, &defending).position, Some(Position::Def));
        assert_eq!(
            [
                live(&state, &defending).damage,
                live(&state, &sick).damage,
                live(&state, &ready).damage
            ],
            [0, 1, 1]
        );
    }

    #[test]
    fn r53_stops_once_the_target_has_left_the_field_so_the_later_attackers_never_attack() {
        let mut state = game("moths-dies");
        state.active = P2;
        let target = put(&mut state, &frail().id, slot(P1, Units, 1), json!({}));
        let first = put(&mut state, &plain.id, slot(P2, Units, 1), json!({}));
        let second = put(&mut state, &plain.id, slot(P2, Units, 2), json!({}));
        let third = put(&mut state, &plain.id, slot(P2, Units, 3), json!({}));

        let events = run(
            &mut state,
            forced_attacks_on(json_as(
                json!({ "target": { "of": "self" }, "attackers": "enemy" }),
            )),
            RunOptions {
                self_: Some(target.clone()),
                controller: Some(P1),
                ..Default::default()
            },
        );

        assert_eq!(attacker_ids(&events), vec![first.id.clone()]);
        // §4.5: each forced attack is its own combat and state check, so the target is gone before the
        // second attacker comes up.
        assert!(card_at(&state, slot(P1, Units, 1)).is_none());
        assert_eq!(
            field_of(&events, GameEventType::Destroyed, "instanceId"),
            vec![json!(target.id)]
        );
        assert_eq!(
            [live(&state, &second).damage, live(&state, &third).damage],
            [0, 0]
        );
        for attacker in [&first, &second, &third] {
            assert_eq!(live(&state, attacker).exertion, unused());
        }
    }

    #[test]
    fn s6_3_fizzles_silently_when_the_target_resolves_to_nothing() {
        let mut state = game("moths-no-target");
        put(&mut state, &plain.id, slot(P2, Units, 1), json!({}));

        // No `self`, so `{ of: "self" }` names nothing and no attack is declared.
        assert_eq!(
            run(
                &mut state,
                forced_attacks_on(json_as(
                    json!({ "target": { "of": "self" }, "attackers": "enemy" })
                )),
                RunOptions::default(),
            ),
            Vec::<GameEvent>::new()
        );
    }
}

mod forced_attacks_s6_3_forced_attack_r53_c60 {
    use super::*;

    #[test]
    fn r53_compels_only_the_attackers_the_def_id_names() {
        let mut state = game("honeypot-defid");
        let victim = put(&mut state, &big_body.id, slot(P2, Units, 1), json!({}));
        let token = put(&mut state, RUSH_TOKEN, slot(P1, Units, 1), json!({}));
        let bystander = put(&mut state, &plain.id, slot(P1, Units, 2), json!({}));

        let events = run(
            &mut state,
            forced_attacks(json_as(json!({
                "attackers": { "side": "self", "defId": RUSH_TOKEN },
                "target": { "instanceId": victim.id },
            }))),
            RunOptions::default(),
        );

        assert_eq!(attacker_ids(&events), vec![token.id.clone()]);
        assert_eq!(live(&state, &victim).damage, 3);
        assert_eq!(live(&state, &bystander).damage, 0);
        assert_eq!(live(&state, &bystander).exertion, unused());
    }

    #[test]
    fn c60_summoned_this_script_compels_only_the_tokens_this_effect_list_just_summoned() {
        let mut state = game("honeypot-fresh");
        let victim = put(&mut state, &big_body.id, slot(P2, Units, 1), json!({}));
        // An unrelated Rush Token the controller already had: same defId, not summoned by this list.
        let older = put(&mut state, RUSH_TOKEN, slot(P1, Units, 1), json!({}));

        let events = run_all(
            &mut state,
            vec![
                summon(json_as(json!({ "defId": RUSH_TOKEN }))),
                summon(json_as(json!({ "defId": RUSH_TOKEN }))),
                forced_attacks(json_as(json!({
                    "attackers": { "side": "self", "defId": RUSH_TOKEN, "summonedThisScript": true },
                    "target": { "instanceId": victim.id },
                }))),
            ],
            RunOptions::default(),
        );

        let summoned: Vec<String> = field_of(&events, GameEventType::Summoned, "instanceId")
            .iter()
            .map(|id| id.as_str().expect("an instance id").to_string())
            .collect();
        assert_eq!(summoned.len(), 2);
        assert!(!summoned.contains(&older.id));
        assert_eq!(attacker_ids(&events), summoned);
        assert_eq!(live(&state, &victim).damage, 6);
        assert_eq!(live(&state, &older).damage, 0);
        assert_eq!(live(&state, &older).exertion, unused());
    }

    #[test]
    fn s6_3_fizzles_silently_when_the_named_target_is_already_gone() {
        let mut state = game("honeypot-gone");
        put(&mut state, RUSH_TOKEN, slot(P1, Units, 1), json!({}));

        let events = run(
            &mut state,
            forced_attacks(json_as(
                json!({ "attackers": { "side": "self" }, "target": { "instanceId": "c-not-a-card" } }),
            )),
            RunOptions::default(),
        );

        assert_eq!(events, Vec::<GameEvent>::new());
    }
}

// aiPlaysOutTurn — §10.7's policy behind #96 My Pawn

/// A board with something to do (§10.7).
fn busy_board(seed: &str) -> GameState {
    let mut state = game(seed);
    state.turn = 4;
    state.active = P1;
    state.phase = Phase::Main;
    state.players.p1.mana.current = 4;
    state.players.p1.mana.max = 4;
    in_hand(&mut state, "fx-1", P1, 2);
    in_hand(&mut state, "fx-2", P1, 2);
    put(&mut state, &big_body.id, slot(P1, Units, 1), json!({}));
    put(&mut state, &plain.id, slot(P2, Units, 1), json!({}));
    state
}

/// R283, R59: Radiant #96 destroys the attacker it stopped, then hands over; `destroy` only marks, so
/// `settleFirst` checks before the AI acts. Another player's prompt (§9.3) leaves only the check to see.
fn marked_board(seed: &str) -> (GameState, Vec<GameEvent>, CardInstance) {
    let mut state = busy_board(seed);
    let marked = card_at(&state, slot(P1, Units, 1)).expect("p1's unit").clone();
    live_mut(&mut state, &marked).marked_destroyed = Some(true);
    let mut sink = Bench::new(&state);
    open_prompt(
        &mut sink.sink(&mut state),
        inert_prompt(P2, "target", "not yours", &["x", "y"]),
    );
    let events = std::mem::take(&mut sink.events);
    (state, events, marked)
}

mod ai_plays_out_turn_s10_7_r44_r84_c96 {
    use super::*;

    #[test]
    fn r44_sets_ai_turn_on_the_named_player_and_takes_no_action_while_another_players_prompt_is_open() {
        let mut state = busy_board("ai-flag");
        let mut sink = Bench::new(&state);
        // §9.3: somebody else's open prompt stops the playout at once, leaving only the lockout flag
        // this verb sets before handing over.
        open_prompt(
            &mut sink.sink(&mut state),
            inert_prompt(P2, "target", "not yours", &["x", "y"]),
        );

        // The trap's controller is p2 (#96 is the defender's card), so "enemy" is the attacker, p1.
        apply_as(
            sink.sink(&mut state),
            ai_plays_out_turn(json_as(json!({ "player": "enemy" }))),
            P2,
        );

        assert!(state.players.p1.ai_turn);
        assert!(!state.players.p2.ai_turn);
        assert!(events_of_type(&sink.events, GameEventType::CardPlayed).is_empty());
        assert_eq!(state.active, P1);
    }

    #[test]
    fn r84_hands_the_rest_of_the_turn_to_s10_7s_policy_which_plays_it_out_and_never_concedes() {
        let mut state = busy_board("ai-playout");
        let address: *const GameState = &state;
        let mut sink = Bench::new(&state);

        apply_as(
            sink.sink(&mut state),
            ai_plays_out_turn(json_as(json!({ "player": "enemy" }))),
            P2,
        );

        // The turn really was played out and ended, on the caller's own state object.
        assert_eq!(
            field_of(&sink.events, GameEventType::TurnEnded, "player"),
            vec![json!("p1")]
        );
        assert!(std::ptr::eq(&state, address));
        assert_eq!(state.active, P2);
        // R84: the policy never picks `concede`, so the game is still running.
        assert!(state.result.is_none());
        assert!(sink.events.len() > 1);
        // R152: the lockout lasts "until end of turn" (§8 #96): that turn's cleanup clears it.
        assert!(!state.players.p1.ai_turn);
    }

    // Each nested playout action settles its own events (§10.3) before they join the caller's list
    // (R168); a per-event mark, not a cursor, keeps the caller's loop from offering them to traps again.
    #[test]
    fn s10_3_an_ai_turns_events_are_dispatched_once_not_again_by_the_callers_own_loop() {
        let (mut state, attacker, _defender, backrow) = swing("ai-redispatch", &[pawn().id, meter().id]);
        state.players.p2.mana = ManaState {
            current: 4,
            max: 4,
            next_turn_mod: 0,
            perm_mod: 0,
        };
        let field = backrow[1].clone();

        let result = reduce(&state, &attack_action(&attacker.id, "hero-p1", "n1"));
        assert_eq!(result.error, None);

        let mana_changes = events_of_type(&result.events, GameEventType::ManaChanged).len();
        let answered = field_of(&result.events, GameEventType::TrapFired, "instanceId")
            .iter()
            .filter(|id| id.as_str() == Some(field.id.as_str()))
            .count();
        assert!(mana_changes > 0);
        assert_eq!(answered, mana_changes);
    }

    #[test]
    fn r44_is_deterministic_from_the_seed_the_same_board_plays_out_the_same_way_twice() {
        let mut first = busy_board("ai-replay");
        let mut second = busy_board("ai-replay");

        let mut first_sink = Bench::new(&first);
        apply_as(
            first_sink.sink(&mut first),
            ai_plays_out_turn(json_as(json!({ "player": "self" }))),
            P1,
        );
        let first_events = std::mem::take(&mut first_sink.events);
        let mut second_sink = Bench::new(&second);
        apply_as(
            second_sink.sink(&mut second),
            ai_plays_out_turn(json_as(json!({ "player": "self" }))),
            P1,
        );
        let second_events = std::mem::take(&mut second_sink.events);

        assert_eq!(
            serde_json::to_string(&second_events).expect("serializes"),
            serde_json::to_string(&first_events).expect("serializes")
        );
        assert_eq!(
            serde_json::to_string(&second).expect("serializes"),
            serde_json::to_string(&first).expect("serializes")
        );
    }

    #[test]
    fn r283_settle_first_collects_a_marked_unit_before_the_ais_first_action_the_default_leaves_it_for_that_action()
     {
        let (mut settled, mut settled_events, marked) = marked_board("ai-settle-first");
        {
            let mut sink = Bench::new(&settled);
            apply_as(
                sink.sink(&mut settled),
                ai_plays_out_turn(json_as(json!({ "player": "enemy", "settleFirst": true }))),
                P2,
            );
            settled_events.extend(std::mem::take(&mut sink.events));
        }

        assert!(settled.players.p1.ai_turn);
        assert!(card_at(&settled, slot(P1, Units, 1)).is_none());
        assert!(
            settled
                .players
                .p1
                .graveyard
                .iter()
                .any(|card| card.id == marked.id)
        );
        assert_eq!(
            field_of(&settled_events, GameEventType::Destroyed, "instanceId"),
            vec![json!(marked.id)]
        );
        assert!(events_of_type(&settled_events, GameEventType::CardPlayed).is_empty());

        let (mut plain_run, mut plain_events, plain_marked) = marked_board("ai-settle-first");
        {
            let mut sink = Bench::new(&plain_run);
            apply_as(
                sink.sink(&mut plain_run),
                ai_plays_out_turn(json_as(json!({ "player": "enemy" }))),
                P2,
            );
            plain_events.extend(std::mem::take(&mut sink.events));
        }

        // Off by default: the mark is still waiting for the next check, which the AI's own first
        // action would have run.
        assert_eq!(
            card_at(&plain_run, slot(P1, Units, 1)).map(|card| card.id.clone()),
            Some(plain_marked.id.clone())
        );
        assert!(events_of_type(&plain_events, GameEventType::Destroyed).is_empty());
    }

    #[test]
    fn r283_settle_first_ends_the_effect_when_its_check_ends_the_game_the_ai_takes_no_action() {
        let mut state = busy_board("ai-settle-first-over");
        state.players.p2.hero.health = 0;
        let mut sink = Bench::new(&state);

        apply_as(
            sink.sink(&mut state),
            ai_plays_out_turn(json_as(json!({ "player": "self", "settleFirst": true }))),
            P1,
        );

        assert_eq!(
            state.result,
            Some(GameResult {
                winner: Winner::P1,
                reason: GameOverReason::HeroDeath
            })
        );
        assert!(events_of_type(&sink.events, GameEventType::CardPlayed).is_empty());
        assert!(events_of_type(&sink.events, GameEventType::TurnEnded).is_empty());
    }

    #[test]
    fn r283_settle_first_false_is_the_default_and_a_board_with_nothing_to_collect_plays_out_the_same_either_way()
     {
        let runs: Vec<(String, String)> = [None, Some(false), Some(true)]
            .into_iter()
            .map(|settle_first: Option<bool>| {
                let mut state = busy_board("ai-settle-first-same");
                let effect = match settle_first {
                    None => ai_plays_out_turn(json_as(json!({ "player": "self" }))),
                    Some(settle_first) => {
                        ai_plays_out_turn(json_as(json!({ "player": "self", "settleFirst": settle_first })))
                    }
                };
                let mut sink = Bench::new(&state);
                apply_as(sink.sink(&mut state), effect, P1);
                let events = std::mem::take(&mut sink.events);
                (
                    serde_json::to_string(&state).expect("serializes"),
                    serde_json::to_string(&events).expect("serializes"),
                )
            })
            .collect();

        assert_eq!(runs[1], runs[0]);
        assert_eq!(runs[2], runs[0]);
    }
}

// cancelAttack — §6.3 Cancel an attack, §4.2 step 4, R44

/// p2 swings a 3/3 into p1's 5/10, with whatever traps the case puts in p1's backrow.
fn swing(seed: &str, traps: &[String]) -> (GameState, CardInstance, CardInstance, Vec<CardInstance>) {
    let mut state = game(seed);
    state.active = P2;
    let attacker = put(&mut state, &plain.id, slot(P2, Units, 1), json!({}));
    // A second body, so `reduce`'s §2.5 auto-end does not close the turn under the assertions.
    put(&mut state, &plain.id, slot(P2, Units, 2), json!({}));
    let defender = put(&mut state, &big_body.id, slot(P1, Units, 1), json!({}));
    let backrow = traps
        .iter()
        .enumerate()
        .map(|(index, def_id)| put(&mut state, def_id, slot(P1, Backrow, index as i32 + 1), json!({})))
        .collect();
    (state, attacker, defender, backrow)
}

fn attack_action(attacker_id: &str, target_id: &str, nonce: &str) -> Action {
    json_as(
        json!({ "type": "attack", "attackerId": attacker_id, "targetId": target_id, "playerId": "p2", "nonce": nonce }),
    )
}

fn type_names(events: &[GameEvent]) -> Vec<&'static str> {
    events.iter().map(|event| event.event_type().as_str()).collect()
}

fn index_of(types: &[&str], wanted: &str) -> i64 {
    types.iter().position(|t| *t == wanted).map_or(-1, |at| at as i64)
}

fn on_unit(card: &CardInstance) -> AttackTarget {
    AttackTarget::Unit {
        instance: card.clone(),
    }
}

mod cancel_attack_s6_3_cancel_an_attack_s4_2_step_4_r44_c96 {
    use super::*;

    #[test]
    fn r44_a_canceled_attack_resolves_no_combat_and_emits_attack_canceled() {
        let (mut state, attacker, defender, backrow) = swing("cancel", &[canceller().id]);
        let trap = backrow[0].clone();
        let mut sink = Bench::new(&state);

        // §4.2 step 4's window belongs inside this call, before any damage: the trap fires there.
        let _ = declare_attack(&mut sink.sink(&mut state), &attacker, &on_unit(&defender));

        assert_eq!(
            json_of(events_of_type(&sink.events, GameEventType::AttackCancelled)),
            json!([{ "type": "attackCancelled", "attackerId": attacker.id, "targetId": defender.id, "byInstanceId": trap.id }])
        );
        // R44: no combat resolved, and the exertion is gone either way.
        assert!(events_of_type(&sink.events, GameEventType::Damage).is_empty());
        assert_eq!(live(&state, &defender).damage, 0);
        assert_eq!(live(&state, &attacker).damage, 0);
        assert!(live(&state, &attacker).exertion.attacked);
        // §4.2 step 5 has had its answer, so the window is shut again.
        assert!(state.declared_attack.is_none());
        // §3.2: the trap is spent whatever its effects achieved (R61), and the attacker is untouched.
        assert_eq!(live(&state, &trap).zone.z(), ZoneName::Graveyard);
        assert_eq!(
            card_at(&state, slot(P2, Units, 1)).map(|card| card.id.clone()),
            Some(attacker.id.clone())
        );
    }

    #[test]
    fn s4_2_step_4_the_window_opens_between_the_declaration_and_the_damage_not_after_it() {
        let (mut state, attacker, defender, _backrow) = swing("window-order", &[watcher().id]);
        let mut sink = Bench::new(&state);

        let _ = declare_attack(&mut sink.sink(&mut state), &attacker, &on_unit(&defender));

        let order = type_names(&sink.events);
        assert!(index_of(&order, "attackDeclared") >= 0);
        assert!(index_of(&order, "trapFired") > index_of(&order, "attackDeclared"));
        assert!(index_of(&order, "damage") > index_of(&order, "trapFired"));
        // R61: the trap did nothing, so the combat is ordinary and the 3/3 leaves the field (§4.5, R78).
        assert_eq!(live(&state, &defender).damage, 3);
        assert_eq!(
            field_of(&sink.events, GameEventType::Damage, "amount"),
            vec![json!(3), json!(5)]
        );
        assert!(card_at(&state, slot(P2, Units, 1)).is_none());
    }

    #[test]
    fn r100_the_declaration_reaches_the_traps_once_the_windows_event_is_not_offered_again() {
        // A Field Trap stays on the field after firing (§5.1, R33), so a second offer of the same
        // declaration (§10.3's immediate check after the window) would show up as a second `trapFired`.
        let (state, attacker, defender, backrow) = swing("window-once", &[watcher().id]);
        let field = backrow[0].clone();
        let result = reduce(&state, &attack_action(&attacker.id, &defender.id, "n1"));

        assert_eq!(result.error, None);
        assert_eq!(
            field_of(&result.events, GameEventType::TrapFired, "instanceId"),
            vec![json!(field.id)]
        );
        assert!(result.state.declared_attack.is_none());
    }

    #[test]
    fn r113_r117_a_trap_that_prompts_parks_the_combat_and_the_answer_finishes_the_window_first() {
        // R68 within a side is lane order, so the asker is offered the declaration first and the
        // canceller is still owed it when the prompt stops the window.
        let (state, attacker, defender, backrow) = swing("window-pause", &[asker().id, canceller().id]);
        let canceling = backrow[1].clone();

        let paused = reduce(&state, &attack_action(&attacker.id, &defender.id, "n1"));
        assert_eq!(paused.error, None);

        // The window stopped: the declaration is still open and the combat owed (§9.3, R113).
        let pending = paused.state.pending.clone();
        assert_eq!(pending.as_ref().map(|p| p.player_id), Some(P1));
        let declared = paused
            .state
            .declared_attack
            .clone()
            .expect("the declaration is still open");
        assert_eq!(declared.attacker_id, attacker.id);
        assert_eq!(declared.target_id, defender.id);
        assert!(!declared.cancelled);
        // R220: whose attack it is, and the stays it was declared on.
        assert_eq!(declared.by, Some(attacker.controller));
        assert!(declared.exits_from.is_some());
        // R113, §10.3: the asking trap's own end first — consumed, and its check, once the answer has
        // finished its list — then the traps the window still owes, then the combat.
        let hooks: Vec<String> = paused
            .state
            .work
            .iter()
            .map(|item| item.resume.hook.clone())
            .collect();
        assert_eq!(hooks, vec!["@trapFiring", "@trapWindow", "@attackWindow"]);
        assert!(events_of_type(&paused.events, GameEventType::Damage).is_empty());

        // §10.1: everything owed is plain JSON, so the paused attack survives a clone round trip.
        assert_eq!(
            clone_state(&paused.state).declared_attack,
            paused.state.declared_attack
        );
        assert_eq!(clone_state(&paused.state).work, paused.state.work);
        let through_json: GameState =
            serde_json::from_value(json_of(&paused.state)).expect("a paused state is plain JSON");
        assert_eq!(through_json.declared_attack, paused.state.declared_attack);
        assert_eq!(through_json.work, paused.state.work);

        let answered = reduce(
            &paused.state,
            &json_as(json!({
                "type": "answer",
                "choiceId": pending.as_ref().map(|p| p.id.clone()).unwrap_or_default(),
                "selection": [{ "pick": "mode", "option": "a" }],
                "playerId": "p1",
                "nonce": "n2",
            })),
        );
        assert_eq!(answered.error, None);

        // The traps still owed ran before the combat, so the second trap's cancel lands: no damage.
        assert_eq!(
            field_of(&answered.events, GameEventType::AttackCancelled, "byInstanceId"),
            vec![json!(canceling.id)]
        );
        assert!(events_of_type(&answered.events, GameEventType::Damage).is_empty());
        assert_eq!(
            card_at(&answered.state, slot(P1, Units, 1)).map(|card| card.damage),
            Some(0)
        );
        assert!(answered.state.declared_attack.is_none());
        assert!(answered.state.work.is_empty());
    }

    #[test]
    fn r121_a_forced_attack_opens_no_window_so_nothing_can_cancel_it() {
        let (mut state, attacker, defender, backrow) = swing("forced-window", &[canceller().id]);
        let trap = backrow[0].clone();
        let mut sink = Bench::new(&state);

        // §4.2's last paragraph: the compelling effect declares this, so no declaration is ever open;
        // R53's combat and state check happen inside the call.
        force_attack(&mut sink.sink(&mut state), &attacker, &on_unit(&defender));
        assert!(state.declared_attack.is_none());
        assert_eq!(live(&state, &defender).damage, 3);
        assert!(!live(&state, &attacker).exertion.attacked);

        // The forced declaration still reaches the traps through §10.3's immediate check (only the
        // window is skipped), and `cancelAttack` there has nothing to mark.
        settle(&mut sink.sink(&mut state), Default::default());
        assert_eq!(
            field_of(&sink.events, GameEventType::TrapFired, "instanceId"),
            vec![json!(trap.id)]
        );
        assert!(events_of_type(&sink.events, GameEventType::AttackCancelled).is_empty());
    }

    #[test]
    fn s6_3_fizzles_silently_with_no_open_declaration_so_the_card_still_resolves() {
        let mut state = game("cancel-nothing");
        let trap = put(&mut state, &ambush().id, slot(P1, Backrow, 1), json!({}));

        assert_eq!(
            run(
                &mut state,
                cancel_attack(Default::default()),
                RunOptions {
                    self_: Some(trap.clone()),
                    ..Default::default()
                },
            ),
            Vec::<GameEvent>::new()
        );
    }
}

mod redirect_attack_s6_3_redirect_an_attack_s4_2_step_4_r1122_r1123 {
    use super::*;

    /// P2's 3/3 with a second 3/3 beside it in lane 2, declaring on P1's 5/10, with `trap` behind P1.
    fn redirect_swing(
        seed: &str,
        trap: &str,
    ) -> (GameState, CardInstance, CardInstance, CardInstance, CardInstance) {
        let (state, attacker, defender, backrow) = swing(seed, &[trap.to_string()]);
        let neighbour = card_at(&state, slot(P2, Units, 2)).cloned().unwrap();
        let trap = backrow.into_iter().next().unwrap();
        (state, attacker, defender, neighbour, trap)
    }

    fn hits(events: &[GameEvent]) -> Vec<(Option<String>, String, i32)> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage {
                    source_id,
                    target_id,
                    amount,
                    ..
                } => Some((source_id.clone(), target_id.clone(), *amount)),
                _ => None,
            })
            .collect()
    }

    fn destroyed_ids(events: &[GameEvent]) -> Vec<String> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Destroyed { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    /// §4.3 between two allied 3/3s: both destroyed, the original target untouched.
    fn assert_allies_fought(
        sink: &Bench,
        state: &GameState,
        attacker: &CardInstance,
        defender: &CardInstance,
        neighbour: &CardInstance,
    ) {
        assert_eq!(
            hits(&sink.events),
            vec![
                (Some(attacker.id.clone()), neighbour.id.clone(), 3),
                (Some(neighbour.id.clone()), attacker.id.clone(), 3),
            ]
        );
        let mut gone = destroyed_ids(&sink.events);
        gone.sort();
        let mut both = vec![attacker.id.clone(), neighbour.id.clone()];
        both.sort();
        assert_eq!(gone, both);
        assert_eq!(live(state, neighbour).zone.z(), ZoneName::Graveyard);
        assert_eq!(live(state, defender).damage, 0);
    }

    #[test]
    fn r1122_redirected_attack_fights_the_ally() {
        let (mut state, attacker, defender, neighbour, trap) = redirect_swing("redirect", &redirector().id);
        let mut sink = Bench::new(&state);

        let _ = declare_attack(&mut sink.sink(&mut state), &attacker, &on_unit(&defender));

        assert_eq!(
            json_of(events_of_type(&sink.events, GameEventType::Redirected)),
            json!([{ "type": "redirected", "what": "attack", "fromId": defender.id, "toId": neighbour.id, "byInstanceId": trap.id }])
        );
        assert_allies_fought(&sink, &state, &attacker, &defender, &neighbour);
        assert!(state.declared_attack.is_none());
    }

    #[test]
    fn r1122_later_trap_skips_it() {
        // The redirector answers first (lane order), so the canceller is never offered the re-aimed
        // attack: it stays set and the allies still fight.
        let (mut state, attacker, defender, backrow) =
            swing("redirect-skip", &[redirector().id, canceller().id]);
        let neighbour = card_at(&state, slot(P2, Units, 2)).cloned().unwrap();
        let mut sink = Bench::new(&state);

        let _ = declare_attack(&mut sink.sink(&mut state), &attacker, &on_unit(&defender));

        assert_eq!(events_of_type(&sink.events, GameEventType::Redirected).len(), 1);
        assert!(events_of_type(&sink.events, GameEventType::AttackCancelled).is_empty());
        let canceller = live(&state, &backrow[1]);
        assert_eq!(canceller.zone.z(), ZoneName::Field);
        assert_ne!(canceller.face_up, Some(true));
        assert_allies_fought(&sink, &state, &attacker, &defender, &neighbour);
    }

    #[test]
    fn r1122_forced_attack_never_opens_the_window() {
        let (mut state, attacker, defender, backrow) = swing("redirect-forced", &[redirector().id]);
        let trap = backrow.into_iter().next().unwrap();
        let mut sink = Bench::new(&state);
        force_attack(&mut sink.sink(&mut state), &attacker, &on_unit(&defender));

        assert!(events_of_type(&sink.events, GameEventType::Redirected).is_empty());
        assert_eq!(live(&state, &defender).damage, 3);
        assert_eq!(live(&state, &trap).zone.z(), ZoneName::Field);
    }

    #[test]
    fn r1123_copies_after_the_check() {
        // The re-aim kills P2's own 3/3; P1, the Radiant trap's controller, gets a fresh copy of it.
        let mut state = game("redirect-copies");
        state.active = P2;
        let attacker = put(&mut state, &big_body.id, slot(P2, Units, 1), json!({}));
        let neighbour = put(&mut state, &plain.id, slot(P2, Units, 2), json!({}));
        let defender = put(&mut state, &plain.id, slot(P1, Units, 1), json!({}));
        put(
            &mut state,
            &redirector_copies().id,
            slot(P1, Backrow, 1),
            json!({}),
        );
        let mut sink = Bench::new(&state);

        let _ = declare_attack(&mut sink.sink(&mut state), &attacker, &on_unit(&defender));

        // A fresh copy stands for P1; P1's own 3/3, the attack's first target, is not it.
        let copies: Vec<CardInstance> = active_units_of(&state, P1)
            .into_iter()
            .filter(|unit| unit.def_id == plain.id && unit.id != defender.id)
            .cloned()
            .collect();
        assert_eq!(copies.len(), 1);
        assert_ne!(copies[0].id, neighbour.id);
        assert_eq!(copies[0].damage, 0);
        assert!(!copies[0].radiant);
        assert_eq!(live(&state, &neighbour).zone.z(), ZoneName::Graveyard);
        assert_eq!(events_of_type(&sink.events, GameEventType::Summoned).len(), 1);
        assert_eq!(live(&state, &defender).damage, 0);
    }
}
