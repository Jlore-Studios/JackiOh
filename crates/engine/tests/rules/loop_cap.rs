//! The loop cap (SPEC §10.3, R3925000): no interaction emits more than `RESOLUTION_EVENT_CAP` events before
//! its resolution loop stops; the rest is dropped, `loopCapped` says so, the dead are collected with their
//! text off, and the game goes on.
//!
//! Fixtures (`lc-`, from 9250): a Unit whose Death summons another of itself at 0/0 (statecheck.rs's
//! `endless`), one more that also hits the enemy hero as it dies, a Cry that starts either loop, a Unit
//! whose end of turn hits the enemy hero, and a pinger: "Whenever the enemy hero takes damage, deal 1 to
//! it", which stops at the enemy hero's health of 1, holds at each `stops` health and asks a question at
//! each `asks` health (its answer pings on). A chain's event total is read off small runs of it (`Tally`),
//! so no test hard-codes an event count.

use jackioh_engine::effects::{choose_mode, damage, gain_hero_armor, summon};
use jackioh_engine::loop_cap;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{Bench, act_ok, both, in_hand, new_game, put, slot};

fn def(name: &str, type_: &str, index: u32, attack: i32, health: i32) -> CardDef {
    let mut face = json!({ "keywords": [], "text": name });
    if type_ == "Unit" {
        face["attack"] = json!(attack);
        face["health"] = json!(health);
    }
    json_as(json!({
        "id": format!("lc-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (loop cap)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face,
        "radiant": face,
    }))
}

/// Death: summon another of itself at 0/0, which dies at the next check: §4.5 step 5 for ever.
fn endless() -> CardDef {
    def("endless", "Unit", 9250, 1, 1)
}

/// `endless`, whose Death first deals 1 damage to the enemy hero.
fn endless_hit() -> CardDef {
    def("endless-hit", "Unit", 9251, 1, 1)
}

/// Cry: summon an `endless` at 0/0.
fn spark() -> CardDef {
    def("spark", "Unit", 9252, 1, 1)
}

/// Cry: summon an `endless-hit` at 0/0.
fn spark_hit() -> CardDef {
    def("spark-hit", "Unit", 9253, 1, 1)
}

/// Whenever the enemy hero takes damage, deal 1 to it (see the header).
fn pinger() -> CardDef {
    def("pinger", "Unit", 9254, 1, 5)
}

/// Cry: deal 1 damage to the enemy hero, which starts the pinger's chain.
fn kick() -> CardDef {
    def("kick", "Spell", 9255, 0, 0)
}

/// `kick`, after 1 Armor for its own hero: one event more, a report nothing answers.
fn kick_pad() -> CardDef {
    def("kick-pad", "Spell", 9256, 0, 0)
}

/// At the end of your turn, deal 1 damage to the enemy hero, which starts the pinger's chain there.
fn dusk() -> CardDef {
    def("dusk", "Unit", 9257, 1, 5)
}

fn hit(to: &str) -> Effect {
    damage(json_as(json!({ "to": { "of": to }, "amount": 1 })))
}

fn summon_dead(def_id: String) -> Effect {
    summon(json_as(json!({
        "defId": def_id,
        "statsOverride": { "attack": 0, "health": 0 },
    })))
}

/// The pinger's answer goes on pinging.
const GO_ON: &str = "go-on";

/// Registers the fixtures, the pinger holding at each of `stops` and asking at each of `asks`.
fn register(stops: Vec<i32>, asks: Vec<i32>) {
    let mut catalog = registered_catalog().clone();
    for card in [
        endless(),
        endless_hit(),
        spark(),
        spark_hit(),
        pinger(),
        kick(),
        kick_pad(),
        dusk(),
    ] {
        catalog.insert(card.id.clone(), card);
    }
    register_catalog(catalog);
    let mut scripts = registered_scripts().clone();
    scripts.insert(
        endless().id,
        both(Script {
            death: Some(hook(|_ctx| vec![summon_dead(endless().id)])),
            ..Script::default()
        }),
    );
    scripts.insert(
        endless_hit().id,
        both(Script {
            death: Some(hook(|_ctx| vec![hit("enemyHero"), summon_dead(endless_hit().id)])),
            ..Script::default()
        }),
    );
    scripts.insert(
        spark().id,
        both(Script {
            cry: Some(hook(|_ctx| vec![summon_dead(endless().id)])),
            ..Script::default()
        }),
    );
    scripts.insert(
        spark_hit().id,
        both(Script {
            cry: Some(hook(|_ctx| vec![summon_dead(endless_hit().id)])),
            ..Script::default()
        }),
    );
    scripts.insert(
        pinger().id,
        both(Script {
            triggers: vec![TriggerDef::new(
                "ping",
                &[GameEventType::Damage],
                move |ctx, event| {
                    let enemy = opponent_of(ctx.controller);
                    let at_enemy = matches!(event, GameEvent::Damage { target_id, .. }
                    if *target_id == format!("hero-{}", enemy.as_str()));
                    let health = ctx.state.players[enemy].hero.health;
                    if !at_enemy || health <= 1 || stops.contains(&health) {
                        vec![]
                    } else if asks.contains(&health) {
                        vec![choose_mode(json_as(
                            json!({ "options": ["a", "b"], "step": GO_ON }),
                        ))]
                    } else {
                        vec![hit("enemyHero")]
                    }
                },
            )],
            resume: [(GO_ON, hook(|_ctx| vec![hit("enemyHero")]))]
                .into_iter()
                .collect(),
            ..Script::default()
        }),
    );
    scripts.insert(
        kick().id,
        both(Script {
            cry: Some(hook(|_ctx| vec![hit("enemyHero")])),
            ..Script::default()
        }),
    );
    scripts.insert(
        kick_pad().id,
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![gain_hero_armor(json_as(json!({ "amount": 1 }))), hit("enemyHero")]
            })),
            ..Script::default()
        }),
    );
    scripts.insert(
        dusk().id,
        both(Script {
            end_of_turn: Some(hook(|_ctx| vec![hit("enemyHero")])),
            ..Script::default()
        }),
    );
    register_scripts(scripts);
}

/// p1's main phase on turn 3, a vanilla unit on p2's side so no turn ends by itself (§2.5), p1's hero at
/// 30 and p2's at `enemy_health`.
fn board(seed: &str, stops: Vec<i32>, asks: Vec<i32>, enemy_health: i32) -> GameState {
    let mut state = new_game(seed, None);
    register(stops, asks);
    state.turn = 3;
    state.active = P1;
    state.phase = Phase::Main;
    state.players.p1.mana.current = 4;
    state.players.p2.hero.health = enemy_health;
    put(&mut state, "fx-21", slot(P2, Row::Units, 5), json!({}));
    state
}

/// `board` with the pinger on p1's side.
fn chain_board(seed: &str, stops: Vec<i32>, asks: Vec<i32>, enemy_health: i32) -> GameState {
    let mut state = board(seed, stops, asks, enemy_health);
    put(&mut state, &pinger().id, slot(P1, Row::Units, 1), json!({}));
    state
}

/// `chain_board` with `dusk` beside the pinger, so p1's End turn starts the chain.
fn dusk_board(seed: &str, asks: Vec<i32>, enemy_health: i32) -> GameState {
    let mut state = chain_board(seed, vec![], asks, enemy_health);
    put(&mut state, &dusk().id, slot(P1, Row::Units, 2), json!({}));
    state
}

fn play(state: &GameState, card: &CardInstance) -> ReduceResult {
    act_ok(
        state,
        json!({ "type": "play", "instanceId": card.id, "playerId": "p1" }),
    )
}

fn capped_events(events: &[GameEvent]) -> Vec<GameEvent> {
    events
        .iter()
        .filter(|event| matches!(event, GameEvent::LoopCapped { .. }))
        .cloned()
        .collect()
}

/// Where the one `loopCapped` stands, asserting there is exactly one and that it names `player`.
fn the_cap(events: &[GameEvent], player: PlayerId) -> usize {
    assert_eq!(
        capped_events(events),
        vec![GameEvent::LoopCapped { player }],
        "exactly one loopCapped, for the interaction's seat"
    );
    events
        .iter()
        .position(|event| matches!(event, GameEvent::LoopCapped { .. }))
        .unwrap_or_default()
}

/// Nothing is left owed or waiting once a capped interaction returns.
fn settled(state: &GameState) {
    assert!(state.trigger_queue.is_empty(), "the trigger queue is dropped");
    assert!(state.dispatch.is_empty(), "no event is left to offer");
    assert!(state.work.is_empty(), "owed work drained");
    assert!(state.pending.is_none(), "no prompt");
    assert!(state.reserved.is_empty(), "no zone is held for a Reborn return");
}

/// No unit at 0 health or less stands.
fn no_dead_standing(state: &GameState) {
    for player in [P1, P2] {
        for unit in units_on_field_of(state, player) {
            assert!(
                unit_view(state, unit).health > 0,
                "{} stands at 0 health",
                unit.def_id
            );
        }
    }
}

/// After the cap, nothing of card text speaks: no hook marker, no summon, no trap.
fn text_off_after(events: &[GameEvent], from: usize) {
    for event in &events[from..] {
        assert!(
            !matches!(
                event,
                GameEvent::HookResolving { .. } | GameEvent::Summoned { .. } | GameEvent::TrapFired { .. }
            ),
            "card text spoke after the cap: {event:?}"
        );
    }
}

/// The next action is legal: the game goes on.
fn goes_on(state: &GameState) {
    assert!(state.result.is_none());
    let ended = act_ok(state, json!({ "type": "endTurn", "playerId": "p1" }));
    assert_eq!(ended.state.active, P2);
    assert!(capped_events(&ended.events).is_empty());
}

#[test]
fn r3925000_a_self_recursive_death_loop_stops_at_the_cap() {
    let mut state = board("r3925000-death-loop", vec![], vec![], 30);
    let card = in_hand(&mut state, &spark().id, P1, 1).remove(0);
    let played = play(&state, &card);
    let at = the_cap(&played.events, P1);
    assert!(
        at >= RESOLUTION_EVENT_CAP,
        "the cap trips once the count reaches it, at {at}"
    );
    text_off_after(&played.events, at);
    settled(&played.state);
    no_dead_standing(&played.state);
    // The last body summoned before the cap is collected without its Death, into p1's graveyard.
    let last_destroyed = played.events[at..].iter().find_map(|event| match event {
        GameEvent::Destroyed { instance_id, .. } => Some(instance_id.clone()),
        _ => None,
    });
    let last = last_destroyed.expect("the capped check collects the body standing at 0/0");
    assert!(
        played
            .state
            .players
            .p1
            .graveyard
            .iter()
            .any(|card| card.id == last)
    );
    goes_on(&played.state);
}

/// #3888's loop: in a Radiant world (S+ #156, R3569000) every core-003 Right-house defender is Radiant, so
/// each of its deaths summons another, and p1's Radiant S #24 Warp Disruptor II sends both of p1's Units at
/// each arrival, the Reborn return among them, which R2197004's terminator does not reach. It plays on the
/// Disruptor's text through v0.6.3, which answered every arrival: R3800004's once-a-turn answer ends the
/// loop on the shipped text since, and a replay of a game recorded before it folds under the old one
/// (R767000), where only the loop cap ends it.
#[test]
fn r3925000_the_radiant_world_right_house_defender_and_warp_disruptor_loop_stops_at_the_cap() {
    const DISRUPTOR: &str = "stinkyscape-024";
    const DEFENDER: &str = "core-003";
    const STURDY: &str = "core-008";
    const FILLER: &str = "core-005";
    jackioh_cards::register_all();
    let mut scripts = registered_scripts().clone();
    scripts.insert(
        DISRUPTOR.to_string(),
        jackioh_cards::scripts::stinkyscape::c024_warp_disruptor_ii::script_v0_6_3(),
    );
    register_scripts(scripts);
    let sturdy = json!({ "def": STURDY, "statsOverride": { "attack": 3, "health": 99_999 } });
    let mut s = scenario(json!({
        "seed": "r3925000-radiant-world",
        "p1": {
            "hand": [{ "def": DISRUPTOR, "radiant": true }, FILLER],
            "field": [sturdy.clone(), sturdy],
            "library": [FILLER, FILLER, FILLER, FILLER],
            "mana": 10,
        },
        "p2": {
            "hand": [FILLER],
            "field": [{ "def": DEFENDER, "radiant": true }],
            "library": [FILLER, FILLER, FILLER, FILLER],
        },
    }));
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s.state_mut().radiant_world = Some(true);
    s.play(DISRUPTOR, json!({}));
    let first = s.unit_at(P1, 1).id;
    let second = s.unit_at(P1, 2).id;
    let defender = s.unit_at(P2, 1).id;
    s.attack(first.as_str(), defender.as_str());
    assert!(
        capped_events(s.last_events()).is_empty(),
        "the shield takes the first hit"
    );
    s.attack(second.as_str(), defender.as_str());

    let events = s.last_events().to_vec();
    let at = the_cap(&events, P1);
    assert!(at >= RESOLUTION_EVENT_CAP);
    text_off_after(&events, at);
    settled(s.state());
    no_dead_standing(s.state());
    // Every card the capped check destroyed went to its owner's graveyard, none came back.
    for event in &events[at..] {
        if let GameEvent::Destroyed {
            instance_id, owner, ..
        } = event
        {
            assert!(
                s.state().players[*owner]
                    .graveyard
                    .iter()
                    .any(|card| card.id == *instance_id),
                "{instance_id} is in its owner's graveyard"
            );
        }
    }
    assert!(s.state().result.is_none());
    s.end_turn();
    assert_eq!(s.state().active, P2);
}

#[test]
fn r3925000_a_chain_of_one_event_fewer_than_the_cap_resolves_in_full() {
    let tally = Tally::measure("r3925000-tally");
    let (health, padded) = tally.health_for(RESOLUTION_EVENT_CAP - 1);
    let mut state = chain_board("r3925000-cap-less-one", vec![], vec![], health);
    let card = in_hand(&mut state, &tally.kick_of(padded), P1, 1).remove(0);
    let played = play(&state, &card);
    assert_eq!(played.events.len(), RESOLUTION_EVENT_CAP - 1);
    assert!(capped_events(&played.events).is_empty());
    assert_eq!(played.state.players.p2.hero.health, 1, "the chain ran to its end");

    // Two links more, and the same chain is capped: its count reached the cap.
    let mut state = chain_board("r3925000-cap-plus-one", vec![], vec![], health + 1);
    let card = in_hand(&mut state, &tally.kick_of(padded), P1, 1).remove(0);
    let played = play(&state, &card);
    let at = the_cap(&played.events, P1);
    assert!(at >= RESOLUTION_EVENT_CAP);
    text_off_after(&played.events, at);
    settled(&played.state);
    goes_on(&played.state);
}

#[test]
fn r3925000_the_count_starts_over_after_each_action() {
    let tally = Tally::measure("r3925000-tally-actions");
    // Two kicks, each a chain of about 0.6 of the cap: together past it, each under it.
    let links = tally.links_for(RESOLUTION_EVENT_CAP * 3 / 5);
    let stop = links + 2;
    let mut state = chain_board("r3925000-actions", vec![stop], vec![], stop + links + 1);
    let kicks = in_hand(&mut state, &kick().id, P1, 2);
    let first = play(&state, &kicks[0]);
    assert_eq!(first.state.players.p2.hero.health, stop);
    let second = play(&first.state, &kicks[1]);
    assert!(first.events.len() + second.events.len() > RESOLUTION_EVENT_CAP);
    assert!(capped_events(&first.events).is_empty());
    assert!(capped_events(&second.events).is_empty());
    assert_eq!(
        second.state.players.p2.hero.health, 1,
        "both chains ran to their ends"
    );
}

#[test]
fn r3925000_the_count_starts_over_after_each_answer() {
    let tally = Tally::measure("r3925000-tally-answer");
    let links = tally.links_for(RESOLUTION_EVENT_CAP * 3 / 5);
    // The chain asks once it has run `links`, and the answer runs `links` more.
    let ask = links + 2;
    let mut state = chain_board("r3925000-answer", vec![], vec![ask], ask + links + 1);
    let card = in_hand(&mut state, &kick().id, P1, 1).remove(0);
    let asked = play(&state, &card);
    let pending = asked.state.pending.clone().expect("the chain asks");
    assert_eq!(asked.state.players.p2.hero.health, ask);
    let answered = act_ok(
        &asked.state,
        json!({ "type": "answer", "choiceId": pending.id, "selection": [{ "pick": "mode", "option": "a" }], "playerId": "p1" }),
    );
    assert!(asked.events.len() + answered.events.len() > RESOLUTION_EVENT_CAP);
    assert!(capped_events(&asked.events).is_empty());
    assert!(capped_events(&answered.events).is_empty());
    assert_eq!(answered.state.players.p2.hero.health, 1);
}

#[test]
fn r3925000_a_timeout_counts_each_answer_it_gives() {
    let tally = Tally::measure("r3925000-tally-timeout");
    let links = tally.links_for(RESOLUTION_EVENT_CAP * 3 / 5);
    // Three chains, the first two ending in a question: the timeout answers both (R79), then ends p1's turn.
    let second_ask = links + 2;
    let first_ask = second_ask + links + 1;
    let mut state = chain_board(
        "r3925000-timeout",
        vec![],
        vec![first_ask, second_ask],
        first_ask + links + 1,
    );
    let card = in_hand(&mut state, &kick().id, P1, 1).remove(0);
    let asked = play(&state, &card);
    assert_eq!(asked.state.players.p2.hero.health, first_ask);
    let timed_out = act_ok(&asked.state, json!({ "type": "timeout", "playerId": "p1" }));
    assert!(
        timed_out.events.len() > RESOLUTION_EVENT_CAP,
        "one action, two answers"
    );
    assert!(capped_events(&timed_out.events).is_empty());
    assert_eq!(timed_out.state.players.p2.hero.health, 1);
    assert_eq!(timed_out.state.active, P2, "and the turn ended");
}

#[test]
fn r3925000_a_hero_killed_in_the_capped_resolution_still_ends_the_game() {
    // The hits the loop lands before the cap, with the hero out of reach.
    let mut state = board("r3925000-hero", vec![], vec![], 100_000);
    let card = in_hand(&mut state, &spark_hit().id, P1, 1).remove(0);
    let reached = play(&state, &card);
    let at = the_cap(&reached.events, P1);
    let hits = 100_000 - reached.state.players.p2.hero.health;
    assert!(hits > 0);
    let hits_before = reached.events[..at]
        .iter()
        .filter(|event| matches!(event, GameEvent::Damage { target_id, .. } if target_id == "hero-p2"))
        .count();
    assert_eq!(hits_before as i32, hits, "no hit lands after the cap");

    // The same loop with the hero at exactly that much: its last hit is the one before the cap.
    let mut state = board("r3925000-hero", vec![], vec![], hits);
    let card = in_hand(&mut state, &spark_hit().id, P1, 1).remove(0);
    let ended = play(&state, &card);
    let at = the_cap(&ended.events, P1);
    assert_eq!(
        ended.state.result.as_ref().map(|result| result.winner),
        Some(Winner::P1)
    );
    let over = ended
        .events
        .iter()
        .rposition(|event| matches!(event, GameEvent::GameOver { .. }))
        .expect("the game ended");
    assert!(at < over, "the capped check's hero check ends it");
    assert_eq!(over, ended.events.len() - 1, "gameOver is the last event (R216)");
}

/// p1's turn 3 ended once, after the cap at `at`, and p2's turn 4 started after it.
fn turn_passed_after(result: &ReduceResult, at: usize) {
    let ended: Vec<usize> = (0..result.events.len())
        .filter(|&i| matches!(result.events[i], GameEvent::TurnEnded { .. }))
        .collect();
    assert_eq!(ended.len(), 1, "turnEnded is emitted exactly once");
    assert!(ended[0] > at, "the turn ends after the cap");
    assert!(matches!(
        result.events[ended[0]],
        GameEvent::TurnEnded {
            player: P1,
            turn: 3,
            ..
        }
    ));
    let started = result.events.iter().rposition(|event| {
        matches!(
            event,
            GameEvent::TurnStarted {
                player: P2,
                turn: 4,
                ..
            }
        )
    });
    assert!(started > Some(ended[0]), "p2's turn starts after it");
    assert_eq!((result.state.active, result.state.turn), (P2, 4));
    assert_eq!(result.state.phase, Phase::Main);
}

/// R3925000: a turn's end the cap stops mid-way is owed engine work, so it still drains with the text
/// off: `turnEnded` once, after the cap, then p2's turn, whether the chain runs inside End turn or in the
/// answer to a prompt that parked the rest of the turn's end (#4550, docs/implicit-states.md §2 E11).
#[test]
fn r3925000_a_turn_end_capped_mid_way_still_ends_the_turn() {
    // Inline: `dusk`'s end-of-turn hit starts the chain inside End turn, which reaches the cap.
    let state = dusk_board("r3925000-turn-end", vec![], 100_000);
    let ended = act_ok(&state, json!({ "type": "endTurn", "playerId": "p1" }));
    let at = the_cap(&ended.events, P1);
    assert!(at >= RESOLUTION_EVENT_CAP);
    text_off_after(&ended.events, at);
    settled(&ended.state);
    no_dead_standing(&ended.state);
    turn_passed_after(&ended, at);

    // Paused: the chain asks at its first link, so End turn parks the rest of the turn's end; the answer
    // is the next interaction, and its chain reaches the cap.
    let state = dusk_board("r3925000-turn-end-asked", vec![100_000], 100_001);
    let asked = act_ok(&state, json!({ "type": "endTurn", "playerId": "p1" }));
    assert!(capped_events(&asked.events).is_empty());
    assert!(
        !asked
            .events
            .iter()
            .any(|event| matches!(event, GameEvent::TurnEnded { .. })),
        "the turn has not ended under the prompt"
    );
    assert_eq!(owed_work(&asked.state, Some(END_OF_TURN_WORK)).len(), 1);
    assert_eq!((asked.state.active, asked.state.turn), (P1, 3));
    let pending = asked.state.pending.clone().expect("the chain asks");
    let answered = act_ok(
        &asked.state,
        json!({ "type": "answer", "choiceId": pending.id, "selection": [{ "pick": "mode", "option": "a" }], "playerId": "p1" }),
    );
    let at = the_cap(&answered.events, P1);
    assert!(at >= RESOLUTION_EVENT_CAP);
    text_off_after(&answered.events, at);
    settled(&answered.state);
    no_dead_standing(&answered.state);
    turn_passed_after(&answered, at);
}

#[test]
fn r3925000_halted_trips_at_the_cap_and_not_before() {
    let mut state = board("r3925000-halted", vec![], vec![], 30);
    let mut bench = Bench::new(&state);
    bench.events = vec![GameEvent::DrawLimited { player: P1 }; RESOLUTION_EVENT_CAP - 1];
    assert!(!bench.run(&mut state, |sink| loop_cap::halted(sink, 0)));
    // A loop that goes round as often as the cap trips it too, though it emits nothing.
    assert!(!bench.run(&mut state, |sink| loop_cap::halted(
        sink,
        RESOLUTION_EVENT_CAP - 1
    )));
    bench.events.push(GameEvent::DrawLimited { player: P1 });
    assert!(bench.run(&mut state, |sink| loop_cap::halted(sink, 0)));
    assert_eq!(
        capped_events(&bench.events),
        vec![GameEvent::LoopCapped { player: P1 }]
    );

    let mut state = board("r3925000-halted-passes", vec![], vec![], 30);
    let mut bench = Bench::new(&state);
    assert!(bench.run(&mut state, |sink| loop_cap::halted(sink, RESOLUTION_EVENT_CAP)));
    // An ended game (as an open prompt, `work::paused`) is never a trip, whatever the count.
    let mut state = board("r3925000-halted-paused", vec![], vec![], 30);
    state.result = Some(GameResult {
        winner: Winner::P1,
        reason: GameOverReason::Concede,
    });
    let mut bench = Bench::new(&state);
    bench.events = vec![GameEvent::DrawLimited { player: P1 }; RESOLUTION_EVENT_CAP];
    assert!(!bench.run(&mut state, |sink| loop_cap::halted(sink, 0)));
}

/// A chain's events, read off two short runs: `base` for the kick with no link, `per_link` for each
/// link, `pad` for the padded kick's one more.
struct Tally {
    base: usize,
    per_link: usize,
    pad: usize,
}

impl Tally {
    fn run(seed: &str, health: i32, padded: bool) -> usize {
        let mut state = chain_board(seed, vec![], vec![], health);
        let kick = if padded { kick_pad() } else { kick() };
        let card = in_hand(&mut state, &kick.id, P1, 1).remove(0);
        let played = play(&state, &card);
        assert!(capped_events(&played.events).is_empty());
        assert_eq!(played.state.players.p2.hero.health, 1);
        played.events.len()
    }

    /// The kick at health 2 leaves the enemy hero at 1 with no link; each point of health more is a link.
    fn measure(seed: &str) -> Tally {
        let base = Tally::run(seed, 2, false);
        let longer = Tally::run(seed, 12, false);
        let padded = Tally::run(seed, 2, true);
        assert!(longer > base && (longer - base).is_multiple_of(10));
        Tally {
            base,
            per_link: (longer - base) / 10,
            pad: padded - base,
        }
    }

    /// The links of a chain about `events` long.
    fn links_for(&self, events: usize) -> i32 {
        ((events - self.base) / self.per_link) as i32
    }

    /// The enemy hero's health for a chain exactly `events` long, and whether the kick is padded.
    fn health_for(&self, events: usize) -> (i32, bool) {
        for padded in [false, true] {
            let start = self.base + if padded { self.pad } else { 0 };
            if (events - start).is_multiple_of(self.per_link) {
                return (2 + ((events - start) / self.per_link) as i32, padded);
            }
        }
        panic!(
            "no chain is exactly {events} events long (base {}, {} a link, pad {})",
            self.base, self.per_link, self.pad
        );
    }

    fn kick_of(&self, padded: bool) -> String {
        if padded { kick_pad().id } else { kick().id }
    }
}
