//! The §6.3 combat verbs a card script needs: Forced attack (#9 Moths to the Flame, #60 Bear
//! Honeypot), Cancel an attack (#96 My Pawn) and the AI turn behind it (§10.7, R44, R84).
//!
//! These effects are thin wrappers, so the tests here assert the wrapper's own decisions — who is
//! compelled, in what order, against what target, and what the wrapper leaves alone — and lean on
//! combat-resolution.test.ts for §4.3 and §4.4 themselves. Every assertion runs through a real
//! `EffectContext`, and the one verb the engine cannot service yet is a marked failing test rather
//! than a tautology.
//!
//! Port of `packages/engine/test/effects-combat.test.ts`.

use jackioh_engine::effects::{ai_plays_out_turn, cancel_attack, forced_attacks, forced_attacks_on, summon};
use jackioh_engine::testkit::*;
use jackioh_engine::{
    PlayerId::{P1, P2},
    Row::{Backrow, Units},
};

use super::fixtures::combat::{big_body, plain, taunter};
use super::fixtures::harness::{events_of_type, in_hand, new_game, put, slot};

/// TS `sinkFor(state)` (fixtures/harness.ts): the state, a fresh event list and an rng at the state's
/// cursor, as `reduce` starts one. A sink borrows all three, so they live here and `sink()` lends them
/// out, built from part 1's frozen `EngineSink::new` and `Rng::new`.
struct Bench<'a> {
    state: &'a mut GameState,
    events: Vec<GameEvent>,
    rng: Rng,
}

impl Bench<'_> {
    fn sink(&mut self) -> EngineSink<'_> {
        EngineSink::new(self.state, &mut self.events, &mut self.rng)
    }
}

fn sink_for(state: &mut GameState) -> Bench<'_> {
    let rng = Rng::new(&state.seed, state.rng_cursor);
    Bench {
        state,
        events: Vec::new(),
        rng,
    }
}

// ---------------------------------------------------------------------------
// Fixture cards: the two bodies fixtures/combat.ts does not carry.
// ---------------------------------------------------------------------------

/// TS `defOfKind(name, index, type, overrides)`: `overrides` is spread over the def, key by key.
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

/// The faces every backrow fixture below prints.
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
/// A Field Trap that answers every declaration and does nothing with it (R61's "fired, consumed,
/// did nothing"). It is a FIELD Trap on purpose: §5.1 leaves one on the field after it fires, so a
/// second offer of the same declaration would fire it a second time, which is what R100 forbids and
/// what the `attackDeclared` event reaching both the window and §10.3's immediate check would do.
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
    ]
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

/// `openPrompt`'s arguments for a prompt nothing can service.
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

/// TS `SCRIPTS`.
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

/// TS `RunOptions = { controller?: PlayerId; self?: CardInstance; targets?: Selection[] }`.
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
    let mut sink = sink_for(state);
    {
        let mut inner = sink.sink();
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

/// `effect.apply(makeContext(sink, null, { controller }))` on a sink of the caller's.
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

/// The card as it stands in the state now (TS held the live object).
fn live<'a>(state: &'a GameState, card: &CardInstance) -> &'a CardInstance {
    find_instance(state, &card.id).unwrap_or_else(|| panic!("{} is nowhere", card.id))
}

fn live_mut<'a>(state: &'a mut GameState, card: &CardInstance) -> &'a mut CardInstance {
    find_instance_mut(state, &card.id).unwrap_or_else(|| panic!("{} is nowhere", card.id))
}

fn json_of<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

fn unused() -> Exertion {
    Exertion {
        attacked: false,
        switched: false,
        attacks: None,
    }
}

/// One field of every event of a type, in order.
fn field_of(events: &[GameEvent], of: GameEventType, key: &str) -> Vec<Value> {
    events_of_type(events, of)
        .iter()
        .map(|event| json_of(event)[key].clone())
        .collect()
}

// ---------------------------------------------------------------------------
// forcedAttacksOn — #9 Moths to the Flame
// ---------------------------------------------------------------------------

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

        // One declaration per attacker, in lane order, all marked forced and all aimed at Moths.
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
        // Position is ignored but not changed: the Defense-Position unit still has its Armor +1, so
        // Moths' 1 attack strikes it for 0 while the two Attack-Position units take 1 each (§4.1).
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
        // §4.5: each forced attack is its own combat followed by its own state check, so the target is
        // already off the field when the second attacker's turn in the loop comes up.
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

// ---------------------------------------------------------------------------
// forcedAttacks — #60 Bear Honeypot
// ---------------------------------------------------------------------------

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
        // Exactly the two fresh tokens, in lane order, and nothing else on the side.
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

// ---------------------------------------------------------------------------
// aiPlaysOutTurn — §10.7's policy behind #96 My Pawn
// ---------------------------------------------------------------------------

/// A board with something to do, as aiPolicy.test.ts's `busyBoard` builds one (§10.7).
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

/// R283, R59: Radiant #96 destroys the attacker it stopped and then hands the turn over, and the
/// destroyed attacker is collected "before the AI takes the turn". `destroy` only marks, so the
/// effect's `settleFirst` runs the state check between the lockout and the AI's first action.
/// Another player's open prompt stops the playout before it acts (§9.3), which leaves exactly the
/// check to observe. TS handed back the open sink; here its events come back beside the state, and
/// the caller's next sink starts its rng where this one left it (nothing here draws).
fn marked_board(seed: &str) -> (GameState, Vec<GameEvent>, CardInstance) {
    let mut state = busy_board(seed);
    let marked = card_at(&state, slot(P1, Units, 1)).expect("p1's unit").clone();
    live_mut(&mut state, &marked).marked_destroyed = Some(true);
    let mut sink = sink_for(&mut state);
    open_prompt(
        &mut sink.sink(),
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
        let mut sink = sink_for(&mut state);
        // §9.3: somebody else's open prompt blocks every action, so the playout stops at once and the
        // only thing left to observe is the lockout flag this verb sets before handing over.
        open_prompt(
            &mut sink.sink(),
            inert_prompt(P2, "target", "not yours", &["x", "y"]),
        );

        // The trap's controller is p2 (#96 is the defender's card), so "enemy" is the attacker, p1.
        apply_as(
            sink.sink(),
            ai_plays_out_turn(json_as(json!({ "player": "enemy" }))),
            P2,
        );

        assert!(sink.state.players.p1.ai_turn);
        assert!(!sink.state.players.p2.ai_turn);
        assert!(events_of_type(&sink.events, GameEventType::CardPlayed).is_empty());
        assert_eq!(sink.state.active, P1);
    }

    #[test]
    fn r84_hands_the_rest_of_the_turn_to_s10_7s_policy_which_plays_it_out_and_never_concedes() {
        let mut state = busy_board("ai-playout");
        let address: *const GameState = &state;
        let mut sink = sink_for(&mut state);

        apply_as(
            sink.sink(),
            ai_plays_out_turn(json_as(json!({ "player": "enemy" }))),
            P2,
        );

        // The turn really was played out and ended, on the caller's own state object.
        assert_eq!(
            field_of(&sink.events, GameEventType::TurnEnded, "player"),
            vec![json!("p1")]
        );
        assert!(std::ptr::eq(&*sink.state, address));
        assert_eq!(sink.state.active, P2);
        // R84: the policy never picks `concede`, so the game is still running.
        assert!(sink.state.result.is_none());
        assert!(sink.events.len() > 1);
        // R152: the lockout lasts "until end of turn" (§8 #96), so `turn.ts`'s cleanup clears it as the
        // turn the AI just played out closes — a player is never locked out of a turn that is no longer
        // the one the effect took. `startTurn` keeps its own clear only as a backstop.
        assert!(!sink.state.players.p1.ai_turn);
    }

    // `subsystems/aiPolicy.playOutTurn` drives a NESTED `reduce` per action of the playout, and each
    // of those settles its own events to completion (§10.3) before the playout copies them onto the
    // caller's event list, so the client is told about them (R168). The caller's resolution loop must
    // not offer them to the traps and the trigger queue again: `triggers.markDispatched` gives each of
    // those events a per-event "already dispatched" mark, the one shape that covers a range in the
    // MIDDLE of the window's own events (`trapFired`, `attackCancelled`, the playout, then
    // `enteredGraveyard`), which a cursor cannot skip. Below: one Field Trap firing per `manaChanged`.
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

        let mut first_sink = sink_for(&mut first);
        apply_as(
            first_sink.sink(),
            ai_plays_out_turn(json_as(json!({ "player": "self" }))),
            P1,
        );
        let first_events = std::mem::take(&mut first_sink.events);
        let mut second_sink = sink_for(&mut second);
        apply_as(
            second_sink.sink(),
            ai_plays_out_turn(json_as(json!({ "player": "self" }))),
            P1,
        );
        let second_events = std::mem::take(&mut second_sink.events);

        assert_eq!(
            serde_json::to_string(&second_events).expect("serialises"),
            serde_json::to_string(&first_events).expect("serialises")
        );
        assert_eq!(
            serde_json::to_string(&second).expect("serialises"),
            serde_json::to_string(&first).expect("serialises")
        );
    }

    #[test]
    fn r283_settle_first_collects_a_marked_unit_before_the_ais_first_action_the_default_leaves_it_for_that_action()
     {
        let (mut settled, mut settled_events, marked) = marked_board("ai-settle-first");
        {
            let mut sink = sink_for(&mut settled);
            apply_as(
                sink.sink(),
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
            let mut sink = sink_for(&mut plain_run);
            apply_as(
                sink.sink(),
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
        let mut sink = sink_for(&mut state);

        apply_as(
            sink.sink(),
            ai_plays_out_turn(json_as(json!({ "player": "self", "settleFirst": true }))),
            P1,
        );

        assert_eq!(
            sink.state.result,
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
                let mut sink = sink_for(&mut state);
                apply_as(sink.sink(), effect, P1);
                let events = std::mem::take(&mut sink.events);
                (
                    serde_json::to_string(&state).expect("serialises"),
                    serde_json::to_string(&events).expect("serialises"),
                )
            })
            .collect();

        assert_eq!(runs[1], runs[0]);
        assert_eq!(runs[2], runs[0]);
    }
}

// ---------------------------------------------------------------------------
// cancelAttack — §6.3 Cancel an attack, §4.2 step 4, R44
// ---------------------------------------------------------------------------

/// p2 swings a 3/3 into p1's 5/10, with whatever traps the case puts in p1's backrow. TS answered
/// `{ state, attacker, defender, backrow }`.
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

fn types_of(events: &[GameEvent]) -> Vec<&'static str> {
    events.iter().map(|event| event.event_type().as_str()).collect()
}

/// `Array.prototype.indexOf`: the first position, or -1.
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
    fn r44_a_cancelled_attack_resolves_no_combat_and_emits_attack_cancelled() {
        let (mut state, attacker, defender, backrow) = swing("cancel", &[canceller().id]);
        let trap = backrow[0].clone();
        let mut sink = sink_for(&mut state);

        // §4.2 step 4's window belongs inside this call, before any damage: the trap fires there.
        let _ = declare_attack(&mut sink.sink(), &attacker, &on_unit(&defender));

        assert_eq!(
            json_of(&events_of_type(&sink.events, GameEventType::AttackCancelled)),
            json!([{ "type": "attackCancelled", "attackerId": attacker.id, "targetId": defender.id, "byInstanceId": trap.id }])
        );
        // R44: no combat resolved, and the exertion is gone either way.
        assert!(events_of_type(&sink.events, GameEventType::Damage).is_empty());
        assert_eq!(live(sink.state, &defender).damage, 0);
        assert_eq!(live(sink.state, &attacker).damage, 0);
        assert!(live(sink.state, &attacker).exertion.attacked);
        // §4.2 step 5 has had its answer, so the window is shut again.
        assert!(sink.state.declared_attack.is_none());
        // §3.2: the trap is spent whatever its effects achieved (R61), and the attacker is untouched.
        assert_eq!(live(sink.state, &trap).zone.z(), ZoneName::Graveyard);
        assert_eq!(
            card_at(sink.state, slot(P2, Units, 1)).map(|card| card.id.clone()),
            Some(attacker.id.clone())
        );
    }

    #[test]
    fn s4_2_step_4_the_window_opens_between_the_declaration_and_the_damage_not_after_it() {
        let (mut state, attacker, defender, _backrow) = swing("window-order", &[watcher().id]);
        let mut sink = sink_for(&mut state);

        let _ = declare_attack(&mut sink.sink(), &attacker, &on_unit(&defender));

        let order = types_of(&sink.events);
        assert!(index_of(&order, "attackDeclared") >= 0);
        assert!(index_of(&order, "trapFired") > index_of(&order, "attackDeclared"));
        assert!(index_of(&order, "damage") > index_of(&order, "trapFired"));
        // R61: the trap fired and did nothing, so the combat is the ordinary one — the 3/3 hits the
        // 5/10 for 3 and is struck back for 5, which kills it and takes it off the field (§4.5, R78).
        assert_eq!(live(sink.state, &defender).damage, 3);
        assert_eq!(
            field_of(&sink.events, GameEventType::Damage, "amount"),
            vec![json!(3), json!(5)]
        );
        assert!(card_at(sink.state, slot(P2, Units, 1)).is_none());
    }

    #[test]
    fn r100_the_declaration_reaches_the_traps_once_the_windows_event_is_not_offered_again() {
        // A Field Trap stays on the field after firing (§5.1, R33), so a second offer of the same
        // declaration — §10.3's immediate check taking the event off the frontier after the window has
        // already delivered it — would show up here as a second `trapFired`.
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
        let cancelling = backrow[1].clone();

        let paused = reduce(&state, &attack_action(&attacker.id, &defender.id, "n1"));
        assert_eq!(paused.error, None);

        // The window stopped where it stood: the prompt is state, the declaration is still open, and
        // the combat is owed rather than resolved or dropped (§9.3, R113).
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

        // The traps the window still owed ran before the combat it precedes, so the second trap's
        // cancel still lands and no damage is ever dealt.
        assert_eq!(
            field_of(&answered.events, GameEventType::AttackCancelled, "byInstanceId"),
            vec![json!(cancelling.id)]
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
        let mut sink = sink_for(&mut state);

        // §4.2's last paragraph: the compelling effect declares this, so there is no open declaration
        // at any point — and R53's combat and state check happen inside the call, as before.
        force_attack(&mut sink.sink(), &attacker, &on_unit(&defender));
        assert!(sink.state.declared_attack.is_none());
        assert_eq!(live(sink.state, &defender).damage, 3);
        assert!(!live(sink.state, &attacker).exertion.attacked);

        // The forced declaration still reaches the traps, through §10.3's immediate check — it is only
        // the window that a forced attack skips — and `cancelAttack` there has nothing to mark.
        settle(&mut sink.sink(), Default::default());
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
