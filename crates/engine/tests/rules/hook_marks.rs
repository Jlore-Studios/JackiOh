//! R2783000: the `hookResolving` marker each first run of a card's Cry, Death, start-of-turn and
//! end-of-turn hook, Whenever trigger and Activate emits just before its effects (§10.3), a hook that
//! fizzles included, and who reads it (§10.8): a seat that could not read the card where it stood as the
//! hook began never sees it, and a marker takes no room in R168's floor.

use jackioh_engine::hook_marks::mark_hook;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::golden::{events_text, view_texts};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{
    act, act_ok, act_with_nonce, both, hand_ids, json_of, new_game, put, run_on_sink, slot,
};
use crate::rules::fixtures::keywords_783::{board_783, cloaked_unit};

// Fixtures.

fn def(name: &str, index: u32, type_: &str, extra: Value) -> CardDef {
    let mut literal = json!({
        "id": format!("hm-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (hook marks)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    });
    if let (Some(into), Some(from)) = (literal.as_object_mut(), extra.as_object()) {
        for (key, value) in from {
            into.insert(key.clone(), value.clone());
        }
    }
    json_as(literal)
}

fn unit(name: &str, index: u32) -> CardDef {
    def(
        name,
        index,
        "Unit",
        json!({
            "base": { "attack": 2, "health": 4, "keywords": [], "text": name },
            "radiant": { "attack": 4, "health": 8, "keywords": [], "text": name },
        }),
    )
}

/// A Cry that hits the enemy hero for 1, so the marker can be placed before it.
fn crier() -> CardDef {
    unit("crier", 27830)
}
/// A Cry whose list is empty: it fizzles (R61).
fn fizzler() -> CardDef {
    unit("fizzler", 27831)
}
/// A Cry that asks its controller first and hits the enemy hero once answered (§10.6, R113).
fn asker() -> CardDef {
    unit("asker", 27832)
}
/// A Death that hits the enemy hero for 1.
fn rattle() -> CardDef {
    unit("rattle", 27833)
}
/// A start-of-turn hook on its own turn and on the opponent's (§6.2).
fn dawn() -> CardDef {
    unit("dawn", 27834)
}
/// An end-of-turn hook that hits the enemy hero for 1.
fn dusk() -> CardDef {
    unit("dusk", 27835)
}
/// A Whenever trigger answering a summon with a hit on the enemy hero.
fn watcher() -> CardDef {
    def("watcher", 27836, "Field Spell", json!({}))
}
/// A Whenever trigger whose condition declines (R99).
fn picky() -> CardDef {
    def("picky", 27837, "Field Spell", json!({}))
}
/// A Trap answering the same summon, which `trapFired` marks and no `hookResolving` does (R2734000).
fn snap() -> CardDef {
    def("snap", 27838, "Trap", json!({}))
}
/// "Activate: deal 1 damage to the enemy hero".
fn pinger() -> CardDef {
    def("pinger", 27839, "Field Spell", json!({}))
}
/// A face-down Trap with an end-of-turn hook (R153, R177).
fn hidden() -> CardDef {
    def("hidden", 27840, "Trap", json!({}))
}
/// A vanilla body to summon.
fn body() -> CardDef {
    unit("body", 27841)
}
/// An end-of-turn hook whose list is empty, on a public card: its queue entry is numbered (R68), and
/// it does nothing else that takes a number.
fn quiet() -> CardDef {
    unit("quiet", 27842)
}
/// R3182000: a Cry that returns a random card from its controller's graveyard to hand. Triggered out of
/// a graveyard (R467), where "this" finds nothing, the one card there is itself.
fn recaller() -> CardDef {
    unit("recaller", 27843)
}
/// R3182000: a Unit token whose Activate tributes it for its cost: it ceases to exist (R11), so the
/// ability resolves with no card anywhere.
fn spent() -> CardDef {
    def(
        "spent",
        27844,
        "Unit",
        json!({
            "token": true,
            "rarity": "Token",
            "base": { "attack": 1, "health": 1, "keywords": [], "text": "spent" },
            "radiant": { "attack": 2, "health": 2, "keywords": [], "text": "spent" },
        }),
    )
}

fn defs() -> Vec<CardDef> {
    vec![
        crier(),
        fizzler(),
        asker(),
        rattle(),
        dawn(),
        dusk(),
        watcher(),
        picky(),
        snap(),
        pinger(),
        hidden(),
        body(),
        quiet(),
        recaller(),
        spent(),
    ]
}

fn hit_enemy_hero() -> Effect {
    effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 1 })))
}

fn ask_controller() -> Effect {
    Effect::new("hm:ask", |ctx| {
        let resume = resume_self(&*ctx, "asked", Default::default());
        let player = ctx.controller;
        open_prompt(
            ctx,
            json_as(json!({
                "player": player,
                "kind": "target",
                "prompt": "the Cry asks its controller",
                "options": [{ "key": "none", "label": "nothing", "selection": { "pick": "none" } }],
                "resume": resume,
            })),
        );
    })
}

fn scripts() -> Vec<(String, CardScripts)> {
    vec![
        (
            crier().id,
            both(Script {
                cry: Some(hook(|_ctx| vec![hit_enemy_hero()])),
                ..Script::default()
            }),
        ),
        (
            fizzler().id,
            both(Script {
                cry: Some(hook(|_ctx| Vec::new())),
                ..Script::default()
            }),
        ),
        (
            asker().id,
            both(Script {
                cry: Some(hook(|_ctx| vec![ask_controller()])),
                resume: [("asked", hook(|_ctx| vec![hit_enemy_hero()]))]
                    .into_iter()
                    .collect(),
                ..Script::default()
            }),
        ),
        (
            rattle().id,
            both(Script {
                death: Some(hook(|_ctx| vec![hit_enemy_hero()])),
                ..Script::default()
            }),
        ),
        (
            dawn().id,
            both(Script {
                start_of_turn: Some(hook(|_ctx| vec![hit_enemy_hero()])),
                start_of_opponent_turn: Some(hook(|_ctx| vec![hit_enemy_hero()])),
                ..Script::default()
            }),
        ),
        (
            dusk().id,
            both(Script {
                end_of_turn: Some(hook(|_ctx| vec![hit_enemy_hero()])),
                ..Script::default()
            }),
        ),
        (
            watcher().id,
            both(Script {
                triggers: vec![TriggerDef::new(
                    "hm-watch",
                    &[GameEventType::Summoned],
                    |_ctx, _event| vec![hit_enemy_hero()],
                )],
                ..Script::default()
            }),
        ),
        (
            picky().id,
            both(Script {
                triggers: vec![
                    TriggerDef::new("hm-picky", &[GameEventType::Summoned], |_ctx, _event| {
                        vec![hit_enemy_hero()]
                    })
                    .with_when(|_ctx, _event| false),
                ],
                ..Script::default()
            }),
        ),
        (
            snap().id,
            both(Script {
                triggers: vec![TriggerDef::new(
                    "hm-snap",
                    &[GameEventType::Summoned],
                    |_ctx, _event| Vec::new(),
                )],
                ..Script::default()
            }),
        ),
        (
            pinger().id,
            both(Script {
                activations: vec![ActivationDecl {
                    id: "ping".to_string(),
                    label: "Deal 1 damage to the enemy hero".to_string(),
                    uses: ActivationUses::Count(1),
                    cost: None,
                    targets: vec![],
                    modes: vec![],
                    can_activate: None,
                    has: None,
                    run: hook(|_ctx| vec![hit_enemy_hero()]),
                }],
                ..Script::default()
            }),
        ),
        (
            hidden().id,
            both(Script {
                end_of_turn: Some(hook(|_ctx| Vec::new())),
                ..Script::default()
            }),
        ),
        (body().id, both(Script::default())),
        (
            recaller().id,
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::add_random_from_graveyard(json_as(json!({ "count": 1 })))]
                })),
                ..Script::default()
            }),
        ),
        (
            spent().id,
            both(Script {
                activations: vec![ActivationDecl {
                    id: "spend".to_string(),
                    label: "Tribute this: deal 1 damage to the enemy hero".to_string(),
                    uses: ActivationUses::Count(1),
                    cost: Some(ActivationCost {
                        tribute_self: Some(true),
                        ..ActivationCost::default()
                    }),
                    targets: vec![],
                    modes: vec![],
                    can_activate: None,
                    has: None,
                    run: hook(|_ctx| vec![hit_enemy_hero()]),
                }],
                ..Script::default()
            }),
        ),
        (
            quiet().id,
            both(Script {
                end_of_turn: Some(hook(|_ctx| Vec::new())),
                ..Script::default()
            }),
        ),
    ]
}

// Harness.

/// Past the mulligans, in p1's main phase, with this file's fixtures registered.
fn playing(seed: &str) -> GameState {
    let state = new_game(&format!("hook-marks-{seed}"), None);
    let mut catalog = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut registry = registered_scripts().clone();
    for (id, script) in scripts() {
        registry.insert(id, script);
    }
    register_scripts(registry);
    let mut state = begin_game(&state).state;
    for player in [P1, P2] {
        state = act(
            &state,
            json!({ "type": "mulligan", "keep": hand_ids(&state, player), "playerId": player }),
        );
    }
    state
}

fn in_hand(state: &mut GameState, def_id: &str, player: PlayerId) -> CardInstance {
    let card = new_instance(state, def_id, player, Zone::Hand { player });
    state.players[player].hand.push(card.clone());
    card
}

fn play_unit(state: &GameState, id: &str, lane: i32) -> ReduceResult {
    act_ok(
        state,
        json!({
            "type": "play",
            "instanceId": id,
            "playerId": "p1",
            "zone": { "row": "units", "lane": lane },
        }),
    )
}

/// Each marker as `(sourceId, defId, controller, hook)`, in order.
fn markers(events: &[GameEvent]) -> Vec<(String, String, PlayerId, HookKind)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::HookResolving {
                source_id,
                def_id,
                controller,
                hook,
                ..
            } => Some((source_id.clone(), def_id.clone(), *controller, *hook)),
            _ => None,
        })
        .collect()
}

/// Where the marker of `source_id` stands in `events`.
fn marker_at(events: &[GameEvent], source_id: &str) -> usize {
    events
        .iter()
        .position(|event| matches!(event, GameEvent::HookResolving { source_id: id, .. } if id == source_id))
        .unwrap_or_else(|| panic!("no marker of {source_id}"))
}

/// Where the first hit `source_id` dealt stands in `events`.
fn hit_at(events: &[GameEvent], source_id: &str) -> usize {
    events
        .iter()
        .position(|event| matches!(event, GameEvent::Damage { source_id: Some(id), .. } if id == source_id))
        .unwrap_or_else(|| panic!("no hit from {source_id}"))
}

fn view_events(state: &GameState, viewer: PlayerId) -> Vec<Value> {
    view_for(state, viewer)
        .events
        .iter()
        .map(|event| serde_json::to_value(event).expect("an event serializes"))
        .collect()
}

fn view_markers(state: &GameState, viewer: PlayerId) -> Vec<Value> {
    view_events(state, viewer)
        .into_iter()
        .filter(|event| event["type"] == "hookResolving")
        .collect()
}

fn names_anywhere(events: &[Value], needle: &str) -> bool {
    events.iter().any(|event| event.to_string().contains(needle))
}

/// A marker on a card nobody hides, as `mark_hook` writes one.
fn public_marker(source_id: &str, def_id: &str) -> GameEvent {
    GameEvent::HookResolving {
        source_id: source_id.into(),
        def_id: def_id.into(),
        controller: P1,
        hook: HookKind::Cry,
        hidden_from: None,
    }
}

// The hooks that are marked.

#[test]
fn r2783000_a_played_units_cry_is_marked_once_before_its_effects() {
    let mut state = playing("cry");
    let card = in_hand(&mut state, &crier().id, P1);
    let played = play_unit(&state, &card.id, 2);
    assert_eq!(
        markers(&played.events),
        vec![(card.id.clone(), crier().id, P1, HookKind::Cry)]
    );
    let at = marker_at(&played.events, &card.id);
    let played_at = played
        .events
        .iter()
        .position(|event| matches!(event, GameEvent::CardPlayed { .. }))
        .expect("a cardPlayed");
    assert!(played_at < at);
    assert!(at < hit_at(&played.events, &card.id));
    assert_eq!(played.state.players.p2.hero.health, HERO_HEALTH - 1);
    // Nothing answers it: the play's queue numbers are the ones a marker-less play takes.
    assert_eq!(
        json_of(&played.events[at]),
        json!({ "type": "hookResolving", "sourceId": card.id, "defId": crier().id, "controller": "p1", "hook": "cry" })
    );
}

#[test]
fn r2783000_a_fizzling_cry_is_still_marked() {
    let mut state = playing("fizzle");
    let card = in_hand(&mut state, &fizzler().id, P1);
    let played = play_unit(&state, &card.id, 1);
    assert_eq!(
        markers(&played.events),
        vec![(card.id.clone(), fizzler().id, P1, HookKind::Cry)]
    );
    assert_eq!(played.state.players.p2.hero.health, HERO_HEALTH);
}

#[test]
fn r2783000_a_cry_paused_by_its_prompt_is_marked_once() {
    let mut state = playing("asked");
    let card = in_hand(&mut state, &asker().id, P1);
    let played = play_unit(&state, &card.id, 3);
    assert_eq!(
        markers(&played.events),
        vec![(card.id.clone(), asker().id, P1, HookKind::Cry)]
    );
    let pending = played.state.pending.as_ref().expect("the Cry asks");
    let answered = act_ok(
        &played.state,
        ActionInput {
            body: ActionBody::Answer {
                choice_id: pending.id.clone(),
                selection: vec![Selection::None],
            },
            player_id: P1,
        },
    );
    // The answered step is the same run continued: no second marker.
    assert!(markers(&answered.events).is_empty());
    assert_eq!(answered.state.players.p2.hero.health, HERO_HEALTH - 1);
}

#[test]
fn r2783000_a_death_hook_is_marked_after_its_destroyed_and_before_its_effects() {
    let mut state = playing("death");
    let card = put(&mut state, &rattle().id, slot(P1, Row::Units, 2), json!({}));
    find_instance_mut(&mut state, &card.id)
        .expect("the rattle")
        .marked_destroyed = Some(true);
    let (_, events) = run_on_sink(&mut state, state_check);
    assert_eq!(
        markers(&events),
        vec![(card.id.clone(), rattle().id, P1, HookKind::Death)]
    );
    let destroyed_at = events
        .iter()
        .position(
            |event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == card.id),
        )
        .expect("its destroyed");
    let at = marker_at(&events, &card.id);
    assert!(destroyed_at < at);
    assert!(at < hit_at(&events, &card.id));
}

#[test]
fn r2783000_start_and_end_of_turn_hooks_are_marked() {
    let mut state = playing("turns");
    let early = put(&mut state, &dawn().id, slot(P1, Row::Units, 1), json!({}));
    let late = put(&mut state, &dusk().id, slot(P1, Row::Units, 2), json!({}));

    let ended = act_ok(&state, json!({ "type": "endTurn", "playerId": "p1" }));
    // p1's end of turn, then p2's start of turn, where the opponent's start-of-turn hook answers.
    assert_eq!(
        markers(&ended.events),
        vec![
            (late.id.clone(), dusk().id, P1, HookKind::EndOfTurn),
            (early.id.clone(), dawn().id, P1, HookKind::StartOfTurn),
        ]
    );
    assert!(marker_at(&ended.events, &late.id) < hit_at(&ended.events, &late.id));
    assert!(marker_at(&ended.events, &early.id) < hit_at(&ended.events, &early.id));

    let back = act_ok(&ended.state, json!({ "type": "endTurn", "playerId": "p2" }));
    assert_eq!(
        markers(&back.events),
        vec![(early.id.clone(), dawn().id, P1, HookKind::StartOfTurn)]
    );
}

#[test]
fn r2783000_a_whenever_trigger_is_marked_a_declining_one_and_a_trap_are_not() {
    let mut state = playing("whenever");
    let watching = put(&mut state, &watcher().id, slot(P1, Row::Backrow, 1), json!({}));
    let declining = put(&mut state, &picky().id, slot(P1, Row::Backrow, 2), json!({}));
    let trap = put(&mut state, &snap().id, slot(P2, Row::Backrow, 1), json!({}));
    let card = in_hand(&mut state, &body().id, P1);
    let played = play_unit(&state, &card.id, 1);

    assert!(
        played.events.iter().any(
            |event| matches!(event, GameEvent::TrapFired { instance_id, .. } if *instance_id == trap.id)
        )
    );
    assert_eq!(
        markers(&played.events),
        vec![(watching.id.clone(), watcher().id, P1, HookKind::Whenever)]
    );
    assert!(marker_at(&played.events, &watching.id) < hit_at(&played.events, &watching.id));
    assert!(
        !markers(&played.events)
            .iter()
            .any(|marker| marker.0 == declining.id)
    );
}

#[test]
fn r2783000_an_activate_is_marked() {
    let mut state = playing("activate");
    let card = put(&mut state, &pinger().id, slot(P1, Row::Backrow, 1), json!({}));
    let used = act_ok(
        &state,
        json!({ "type": "activate", "instanceId": card.id, "ability": "ping", "playerId": "p1" }),
    );
    assert_eq!(
        markers(&used.events),
        vec![(card.id.clone(), pinger().id, P1, HookKind::Activate)]
    );
    let activated_at = used
        .events
        .iter()
        .position(|event| matches!(event, GameEvent::Activated { .. }))
        .expect("an activated");
    let at = marker_at(&used.events, &card.id);
    assert!(activated_at < at);
    assert!(at < hit_at(&used.events, &card.id));
}

// Who reads it.

#[test]
fn r2783000_a_face_down_traps_hook_reaches_its_controller_alone() {
    let mut state = playing("hidden");
    let trap = put(&mut state, &hidden().id, slot(P1, Row::Backrow, 1), json!({}));
    let ended = act_ok(&state, json!({ "type": "endTurn", "playerId": "p1" }));
    assert_eq!(
        markers(&ended.events),
        vec![(trap.id.clone(), hidden().id, P1, HookKind::EndOfTurn)]
    );
    assert!(matches!(
        &ended.events[marker_at(&ended.events, &trap.id)],
        GameEvent::HookResolving { hidden_from: Some(hidden), .. } if *hidden == vec![P2]
    ));

    // Its controller reads it, without the engine's `hiddenFrom`.
    assert_eq!(
        view_markers(&ended.state, P1),
        vec![
            json!({ "type": "hookResolving", "sourceId": trap.id, "defId": hidden().id, "controller": "p1", "hook": "endOfTurn" })
        ]
    );
    // The other seat never learns that a hidden card's hook answered, nor which card it was.
    let theirs = view_events(&ended.state, P2);
    assert!(view_markers(&ended.state, P2).is_empty());
    assert!(!names_anywhere(&theirs, &trap.id));
    assert!(!names_anywhere(&theirs, &hidden().id));
    let fresh = ended.events.len();
    assert_eq!(
        viewed_of_newest(&ended.state, P2, fresh),
        viewed_of_newest(&ended.state, P1, fresh) - 1
    );
}

#[test]
fn r2783000_a_face_down_cloaked_card_is_never_named_to_the_other_seat() {
    let mut state = board_783("hook-marks-cloaked");
    let card = new_instance(&mut state, &cloaked_unit().id, P1, Zone::Hand { player: P1 });
    state.players.p1.hand.push(card.clone());
    let played = act_ok(
        &state,
        json!({
            "type": "play",
            "instanceId": card.id,
            "zone": { "row": "backrow", "lane": 2 },
            "faceDown": "cloaked",
            "playerId": "p1",
        }),
    );
    // Set by its Cloaked, its text is inert (R1040): the Cry the play pipeline runs as it is set is
    // no hook of its resolving, so it marks nothing, for either seat.
    assert_eq!(markers(&played.events), vec![]);
    let mut state = played.state;
    let set = state.players.p1.backrow[1].clone().expect("the set card");
    let (_, events) = run_on_sink(&mut state, |sink| mark_hook(sink, &set, P1, HookKind::Activate));
    assert_eq!(events, vec![]);
    // Revealed (R783006), its text resolves: its Cry is marked as it runs, the card public by then.
    let revealed = act_ok(
        &state,
        json!({ "type": "activate", "instanceId": set.id, "ability": "reveal", "playerId": "p1" }),
    );
    // It asks for its Cry's target first; the run the answer starts is the Cry's, and is marked.
    assert_eq!(markers(&revealed.events), vec![]);
    let pending = revealed
        .state
        .pending
        .as_ref()
        .expect("the Cry asks for its target");
    let enemy_hero = pending
        .options
        .iter()
        .find(|option| option.key == "hero:p2")
        .expect("the enemy hero")
        .selection
        .clone();
    let answered = act_ok(
        &revealed.state,
        ActionInput {
            body: ActionBody::Answer {
                choice_id: pending.id.clone(),
                selection: vec![enemy_hero],
            },
            player_id: P1,
        },
    );
    assert_eq!(
        markers(&answered.events),
        vec![(set.id.clone(), cloaked_unit().id, P1, HookKind::Cry)]
    );
    assert_eq!(view_markers(&answered.state, P2).len(), 1);

    // A hook of a Cloaked card where the other seat cannot read it, in its controller's hand.
    let held = new_instance(&mut state, &cloaked_unit().id, P1, Zone::Hand { player: P1 });
    state.players.p1.hand.push(held.clone());
    let (_, events) = run_on_sink(&mut state, |sink| mark_hook(sink, &held, P1, HookKind::Whenever));
    assert!(matches!(
        &events[..],
        [GameEvent::HookResolving { hidden_from: Some(hidden), .. }] if *hidden == vec![P2]
    ));
    state.applied.push(AppliedAction {
        nonce: "hook-marks-cloaked".into(),
        events,
    });

    // Its controller reads it; the other seat reads neither marker nor card.
    assert_eq!(
        view_markers(&state, P1),
        vec![
            json!({ "type": "hookResolving", "sourceId": held.id, "defId": cloaked_unit().id, "controller": "p1", "hook": "whenever" })
        ]
    );
    let theirs = view_events(&state, P2);
    assert!(view_markers(&state, P2).is_empty());
    assert!(!names_anywhere(&theirs, &set.id));
    assert!(!names_anywhere(&theirs, &held.id));
    assert!(!names_anywhere(&theirs, &cloaked_unit().id));
}

#[test]
fn r2783000_a_marker_names_its_card_as_the_card_itself_is() {
    let mut state = playing("named");
    // The card's hook was public as it ran; the card is in p1's hand now.
    let card = in_hand(&mut state, &crier().id, P1);
    state.applied.push(AppliedAction {
        nonce: "hook-marks-named".into(),
        events: vec![public_marker(&card.id, &crier().id)],
    });
    assert_eq!(
        view_markers(&state, P1),
        vec![
            json!({ "type": "hookResolving", "sourceId": card.id, "defId": crier().id, "controller": "p1", "hook": "cry" })
        ]
    );
    assert_eq!(
        view_markers(&state, P2),
        vec![
            json!({ "type": "hookResolving", "sourceId": HIDDEN_ID, "defId": HIDDEN_ID, "controller": "p1", "hook": "cry" })
        ]
    );
}

#[test]
fn r2783000_a_marker_brings_no_match_made_definition_into_the_view() {
    let mut state = playing("defs");
    let card = put(&mut state, &crier().id, slot(P1, Row::Units, 1), json!({}));
    // A definition made during the match (R243) that the board no longer names: the card's own, as it
    // stood when the hook began, before the hook fused something into it.
    let made = "t-9:hm-crier";
    let mut def = crier();
    def.id = made.to_string();
    state.transient_defs.insert(made.to_string(), def);
    state.applied.push(AppliedAction {
        nonce: "hook-marks-defs".into(),
        events: vec![public_marker(&card.id, made)],
    });
    let view = view_for(&state, P1);
    assert_eq!(view_markers(&state, P1)[0]["defId"], json!(made));
    assert!(view.defs.is_none());
}

#[test]
fn r2783000_markers_take_no_room_in_the_views_window() {
    let mut state = playing("window");
    let card = put(&mut state, &crier().id, slot(P1, Row::Units, 1), json!({}));
    let older: Vec<GameEvent> = (0..VIEW_EVENT_LIMIT)
        .flat_map(|turn| {
            [
                public_marker(&card.id, &crier().id),
                GameEvent::TurnStarted {
                    player: P2,
                    turn: turn as i32,
                    extra: None,
                },
            ]
        })
        .collect();
    let newest = vec![
        GameEvent::DrawOffered { player: P1 },
        public_marker(&card.id, &crier().id),
    ];
    let mut bare = state.clone();
    for (nonce, events) in [("older", older), ("newest", newest)] {
        bare.applied.push(AppliedAction {
            nonce: nonce.into(),
            events: events
                .iter()
                .filter(|event| !matches!(event, GameEvent::HookResolving { .. }))
                .cloned()
                .collect(),
        });
        state.applied.push(AppliedAction {
            nonce: nonce.into(),
            events,
        });
    }

    for viewer in [P1, P2] {
        let with: Vec<Value> = view_events(&state, viewer)
            .into_iter()
            .filter(|event| event["type"] != "hookResolving")
            .collect();
        assert_eq!(with, view_events(&bare, viewer));
        assert_eq!(with.len(), VIEW_EVENT_LIMIT);
        // The markers between the events it holds travel with them.
        assert_eq!(view_markers(&state, viewer).len(), VIEW_EVENT_LIMIT);
    }
}

#[test]
fn r2783000_r2783001_golden_texts_leave_out_markers_and_via() {
    let hit = |via: Option<KeywordKind>| GameEvent::Damage {
        source_id: Some("c4".into()),
        target_id: "hero-p2".into(),
        amount: 2,
        combat: true,
        absorbed: 0,
        via,
    };
    assert_eq!(
        events_text(&[public_marker("c4", "core-001"), hit(Some(KeywordKind::Trample))]),
        events_text(&[hit(None)])
    );

    let mut state = playing("golden");
    let mut bare = state.clone();
    state.applied.push(AppliedAction {
        nonce: "golden".into(),
        events: vec![public_marker("c4", "core-001"), hit(Some(KeywordKind::Cleave))],
    });
    bare.applied.push(AppliedAction {
        nonce: "golden".into(),
        events: vec![hit(None)],
    });
    assert_eq!(view_texts(&state), view_texts(&bare));
}

#[test]
fn r2783000_a_round_tripped_state_replays_its_markers_and_they_take_no_number() {
    let mut state = playing("round-trip");
    let bare = act_with_nonce(&state, json!({ "type": "endTurn", "playerId": "p1" }), "hm-end");
    let late = put(&mut state, &quiet().id, slot(P1, Row::Units, 2), json!({}));
    let copy: GameState = serde_json::from_value(json_of(&state)).expect("a state reads back");

    let live = act_ok(&state, json!({ "type": "endTurn", "playerId": "p1" }));
    let replayed = act_ok(&copy, json!({ "type": "endTurn", "playerId": "p1" }));
    assert_eq!(
        markers(&live.events),
        vec![(late.id.clone(), quiet().id, P1, HookKind::EndOfTurn)]
    );
    assert_eq!(live.events, replayed.events);
    assert_eq!(hash_state(&live.state), hash_state(&replayed.state));
    // The hook's queue entry takes its number, and its marker none (R68's one counter).
    assert_eq!(live.state.next_seq, bare.next_seq + 1);
}

// R3182000: a hook that runs with no card as its `self` still names the card it belongs to.

/// `trigger_cry` on `card` for p1, as Rewind's list runs it (R467), its events kept as one action.
fn trigger_cry_for_p1(state: &mut GameState, card: &CardInstance) -> Vec<GameEvent> {
    let ((), events) = run_on_sink(state, |sink| {
        {
            let mut ctx = make_context(
                sink,
                None,
                HookOptions {
                    controller: Some(P1),
                    ..Default::default()
                },
            );
            (effects::trigger_cry(json_as(json!({ "instanceId": card.id }))).apply)(&mut ctx);
        }
        settle(sink, Default::default());
    });
    state.applied.push(AppliedAction {
        nonce: "hook-marks-grave-cry".into(),
        events: events.clone(),
    });
    events
}

#[test]
fn r3182000_a_graveyard_crys_marker_names_its_card_and_is_hidden_once_the_cry_returns_it_to_a_hand() {
    let mut state = playing("grave-cry");
    let card = new_instance(&mut state, &recaller().id, P1, Zone::Graveyard { player: P1 });
    state.players.p1.graveyard.push(card.clone());

    let events = trigger_cry_for_p1(&mut state, &card);
    // Out of the graveyard "this" finds nothing (R467), yet the marker is the card's, not `""`.
    assert_eq!(
        markers(&events),
        vec![(card.id.clone(), recaller().id, P1, HookKind::Cry)]
    );
    let back = find_instance(&state, &card.id).expect("the card is still in the game");
    assert_eq!(back.zone.z(), ZoneName::Hand);

    // Its owner reads it; the other seat reads that a hook began, never whose (R97), as the card's
    // `addedToHand` hides it.
    assert_eq!(
        view_markers(&state, P1),
        vec![
            json!({ "type": "hookResolving", "sourceId": card.id, "defId": recaller().id, "controller": "p1", "hook": "cry" })
        ]
    );
    assert_eq!(
        view_markers(&state, P2),
        vec![
            json!({ "type": "hookResolving", "sourceId": HIDDEN_ID, "defId": HIDDEN_ID, "controller": "p1", "hook": "cry" })
        ]
    );
    assert!(!names_anywhere(&view_events(&state, P2), &recaller().id));
    assert!(!names_anywhere(&view_events(&state, P2), &card.id));
}

#[test]
fn r3182000_an_activate_whose_cost_took_its_card_out_of_the_game_still_names_that_card() {
    let mut state = playing("spent");
    let token = put(&mut state, &spent().id, slot(P1, Row::Units, 1), json!({}));
    let used = act_ok(
        &state,
        json!({ "type": "activate", "instanceId": token.id, "ability": "spend", "playerId": "p1" }),
    );
    // The token ceased to exist as its cost tributed it (R11): the ability resolves with no card.
    assert!(find_instance(&used.state, &token.id).is_none());
    assert_eq!(used.state.players.p2.hero.health, HERO_HEALTH - 1);
    assert_eq!(
        markers(&used.events),
        vec![(token.id.clone(), spent().id, P1, HookKind::Activate)]
    );
    // Its every event was public, so both seats read it, as they read its `destroyed`.
    for viewer in [P1, P2] {
        assert_eq!(
            view_markers(&used.state, viewer),
            vec![
                json!({ "type": "hookResolving", "sourceId": token.id, "defId": spent().id, "controller": "p1", "hook": "activate" })
            ]
        );
    }
}
