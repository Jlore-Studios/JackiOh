//! Casts from anywhere and random casts (docs/classic-sets.md B5 E12, E39; R70, R452, R453): the cast
//! verbs of `src/effects/cast.ts` — a card out of a graveyard, a new card of a named definition, random
//! catalog cards — how a cast's choices are made (its caster's, or the rng's, narrowed to enemies when
//! it targets them), its X, a cast permanent's zone, a pause inside a cast surviving JSON, the chain cap,
//! and what the other seat reads.
//!
//! Port of `packages/engine/test/effects-cast.test.ts`.

use std::collections::BTreeSet;

use jackioh_engine::effects::{cast, cast_new, cast_random, damage, heal};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{events_of_type, in_hand, put, slot};
use crate::rules::fixtures::play_pipeline_b::{
    DISCOVER_POOL, RANDOM_POOL, ask_target, cast_field, cast_trap, discover_spell, grave_spell, in_graveyard, jogg_box,
    mode_spell, named_caster, pb_act, pb_playing, pb_reduce, solarius, target_spell, their_choice, tyrant, x_spell,
    x_target,
};

// Inline cards for the chain cap and the nested cases, indexed clear of the shared fixtures.
fn spell(name: &str, index: u32) -> CardDef {
    json_as(json!({
        "id": format!("pbc-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (casts)"),
        "set": "Core",
        "type": "Spell",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    }))
}

const PING_A: &str = "pbc-ping-a";
const PING_B: &str = "pbc-ping-b";
const ECHO_TARGET: &str = "pbc-echo-target";
const CASTS_NAMED: &str = "pbc-casts-named";
const CASTS_DISCOVER: &str = "pbc-casts-discover";
// R656: a helpful Spell — its target declaration aims "help", so a cast that targets enemies
// prefers friends for it.
const HEAL_FRIEND: &str = "pbc-heal-friend";

fn inline_defs() -> Vec<CardDef> {
    vec![
        spell("ping-a", 4601),
        spell("ping-b", 4602),
        spell("echo-target", 4603),
        spell("casts-named", 4604),
        spell("casts-discover", 4605),
        spell("heal-friend", 4606),
    ]
}

fn both(script: Script) -> CardScripts {
    CardScripts { base: script.clone(), radiant: script }
}

fn inline_scripts() -> IndexMap<String, CardScripts> {
    let mut scripts: IndexMap<String, CardScripts> = IndexMap::new();
    scripts.insert(
        PING_A.to_string(),
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![
                    damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 1 }))),
                    cast_random(json_as(json!({ "query": { "defId": PING_B }, "count": 2 }))),
                ]
            })),
            ..Script::default()
        }),
    );
    scripts.insert(
        PING_B.to_string(),
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![
                    damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 1 }))),
                    cast_random(json_as(json!({ "query": { "defId": PING_A }, "count": 2 }))),
                ]
            })),
            ..Script::default()
        }),
    );
    scripts.insert(
        ECHO_TARGET.to_string(),
        both(Script {
            static_flags: Some(StaticFlags { echo: Some(1), ..Default::default() }),
            targets: vec![json_as(json!({ "kind": "target", "min": 1, "max": 1, "filter": { "of": ["unit", "hero"] } }))],
            cry: Some(hook(|_ctx| vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 1 })))])),
            ..Script::default()
        }),
    );
    scripts.insert(
        CASTS_NAMED.to_string(),
        both(Script {
            cry: Some(hook(|_ctx| vec![cast_new(json_as(json!({ "def": target_spell().id })))])),
            ..Script::default()
        }),
    );
    scripts.insert(
        CASTS_DISCOVER.to_string(),
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![cast_random(json_as(json!({ "query": { "defId": discover_spell().id }, "count": 1 })))]
            })),
            ..Script::default()
        }),
    );
    scripts.insert(
        HEAL_FRIEND.to_string(),
        both(Script {
            targets: vec![json_as(json!({
                "kind": "target", "min": 1, "max": 1, "filter": { "of": ["unit"] }, "aim": "help"
            }))],
            cry: Some(hook(|_ctx| vec![heal(json_as(json!({ "target": { "of": "chosen" }, "amount": 3 })))])),
            ..Script::default()
        }),
    );
    scripts
}

fn playing(seed: &str) -> GameState {
    let state = pb_playing(seed);
    let mut catalog = registered_catalog().clone();
    for def in inline_defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut scripts = registered_scripts().clone();
    scripts.extend(inline_scripts());
    register_scripts(scripts);
    state
}

/// Apply effects as p1's own card would, then run the resolution loop, as a Cry's list is run. The
/// sink's events come back.
fn run(state: &mut GameState, effects: Vec<Effect>, self_id: Option<&str>) -> Vec<GameEvent> {
    let self_ = self_id.and_then(|id| state.players[PlayerId::P1].hand.iter().find(|card| card.id == id).cloned());
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(sink, self_, HookOptions { controller: Some(PlayerId::P1), ..Default::default() });
        apply_effects(&effects, &mut ctx);
        settle(&mut ctx.sink, Default::default());
    }
    state.rng_cursor = rng.cursor();
    events
}

/// TS `only` (fixtures/playPipelineB): the first item, which must exist.
fn only<T>(items: impl IntoIterator<Item = T>) -> T {
    items.into_iter().next().expect("expected at least one item")
}

/// `JSON.parse(JSON.stringify(state))`, the round trip a paused state must survive (§9.3).
fn round_trip(state: &GameState) -> GameState {
    serde_json::from_value(serde_json::to_value(state).unwrap()).unwrap()
}

/// TS `{ ...action, playerId }`: a legal action body as the literal `pb_act` takes.
fn body_of(action: &ActionBody, player: PlayerId) -> Value {
    let mut body = serde_json::to_value(action).unwrap();
    body["playerId"] = json!(player);
    body
}

fn answers(state: &GameState, player: PlayerId) -> Vec<ActionBody> {
    legal_actions(state, player).into_iter().filter(|action| matches!(action, ActionBody::Answer { .. })).collect()
}

/// The answer whose selection's JSON contains `text` (TS `JSON.stringify(action.selection).includes(text)`).
fn answer_containing(state: &GameState, player: PlayerId, text: &str) -> ActionBody {
    only(answers(state, player).into_iter().filter(|action| match action {
        ActionBody::Answer { selection, .. } => serde_json::to_string(selection).unwrap().contains(text),
        _ => false,
    }))
}

/// The answer picking exactly the enemy hero.
fn answer_hero(state: &GameState, player: PlayerId, hero: PlayerId) -> ActionBody {
    only(answers(state, player).into_iter().filter(|action| match action {
        ActionBody::Answer { selection, .. } => *selection == vec![Selection::Hero { player: hero }],
        _ => false,
    }))
}

/// TS `slice(-n)`.
fn last_n<T: Clone>(items: &[T], n: usize) -> Vec<T> {
    items[items.len().saturating_sub(n)..].to_vec()
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn option_selections(state: &GameState) -> Vec<Selection> {
    state.pending.as_ref().map(|pending| pending.options.iter().map(|option| option.selection.clone()).collect()).unwrap_or_default()
}

fn option_labels(state: &GameState) -> Vec<String> {
    state.pending.as_ref().map(|pending| pending.options.iter().map(|option| option.label.clone()).collect()).unwrap_or_default()
}

fn played_events(events: &[GameEvent]) -> Vec<(String, String, i32)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::CardPlayed { instance_id, def_id, cost_paid, .. } => {
                Some((instance_id.clone(), def_id.clone(), *cost_paid))
            }
            _ => None,
        })
        .collect()
}

fn damage_targets(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Damage { target_id, .. } => Some(target_id.clone()),
            _ => None,
        })
        .collect()
}

fn healed_targets(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Healed { target_id, .. } => Some(target_id.clone()),
            _ => None,
        })
        .collect()
}

fn added_def_ids(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::AddedToHand { def_id, .. } => Some(def_id.clone()),
            _ => None,
        })
        .collect()
}

fn set_damage(state: &mut GameState, id: &str, damage: i32) {
    find_instance_mut(state, id).expect("the unit").damage = damage;
}

mod e12_casts_from_anywhere_r70_r453 {
    use super::*;

    #[test]
    fn r453_casts_every_spell_in_the_graveyard_oldest_first_each_with_its_caster_s_choices_and_exiles_each_once_it_resolves(
    ) {
        let mut state = playing("r453-tyrant");
        state.players[PlayerId::P1].mana.current = 4;
        let first = in_graveyard(&mut state, &grave_spell().id, PlayerId::P1);
        let second = in_graveyard(&mut state, &mode_spell().id, PlayerId::P1);
        let third = in_graveyard(&mut state, &target_spell().id, PlayerId::P1);
        let card = only(in_hand(&mut state, &tyrant().id, PlayerId::P1, 1));
        let before = state.players[PlayerId::P2].hero.health;

        // The Tyrant's Cry casts the first (no choices), then asks for the Mode Spell's mode (R70).
        let paused = pb_act(
            &state,
            json!({ "type": "play", "instanceId": card.id, "zone": { "row": "units", "lane": 2 }, "playerId": "p1" }),
        );
        assert_eq!(paused.pending.as_ref().map(|pending| pending.player_id), Some(PlayerId::P1));
        assert!(paused.pending.as_ref().expect("a prompt").prompt.starts_with("Cast: "));
        assert_eq!(option_labels(&paused), vec!["a".to_string(), "b".to_string()]);
        assert!(paused.casts_resolving.is_none());

        // JSON round trip mid-pause: both copies finish the same way.
        let round = round_trip(&paused);
        let pick_b = answer_containing(&paused, PlayerId::P1, "\"b\"");
        let next = pb_act(&paused, body_of(&pick_b, PlayerId::P1));
        let next_round = pb_act(&round, body_of(&pick_b, PlayerId::P1));
        assert_eq!(hash_state(&next_round), hash_state(&next));
        // Now the Target Spell asks for its target.
        assert!(next.pending.as_ref().expect("a prompt").prompt.starts_with("Cast: "));
        let enemy_hero = answer_hero(&next, PlayerId::P1, PlayerId::P2);
        let done = pb_act(&next, body_of(&enemy_hero, PlayerId::P1));
        assert!(done.pending.is_none());
        // 1 + 2 + 3 damage to the enemy hero, each cast counted as a play paying 0 (R70).
        assert_eq!(done.players[PlayerId::P2].hero.health, before - 6);
        assert_eq!(
            last_n(&done.players[PlayerId::P1].turn_log.played_ids, 3),
            vec![first.id.clone(), second.id.clone(), third.id.clone()]
        );
        assert_eq!(
            done.players[PlayerId::P1].turn_log.costs_paid.as_ref().map(|costs| last_n(costs, 3)),
            Some(vec![0, 0, 0])
        );
        // "Then exile them": each resolved into exile, none back in the graveyard.
        let exiled = ids(&done.players[PlayerId::P1].exile);
        for id in [&first.id, &second.id, &third.id] {
            assert!(exiled.contains(id));
        }
        assert!(!ids(&done.players[PlayerId::P1].graveyard).contains(&first.id));
    }

    #[test]
    fn r453_a_named_definition_is_cast_new_owned_by_the_caster_asked_of_the_caster_and_lands_in_the_caster_s_graveyard() {
        let mut state = playing("r453-named");
        let card = only(in_hand(&mut state, &named_caster().id, PlayerId::P1, 1));
        let paused = pb_act(
            &state,
            json!({ "type": "play", "instanceId": card.id, "zone": { "row": "units", "lane": 1 }, "playerId": "p1" }),
        );
        assert_eq!(paused.pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Target));
        let cast = only(paused.players[PlayerId::P1].resolving.clone());
        assert_eq!(cast.def_id, target_spell().id);
        assert_eq!(cast.owner, PlayerId::P1);
        let answer = answer_hero(&paused, PlayerId::P1, PlayerId::P2);
        let done = pb_act(&paused, body_of(&answer, PlayerId::P1));
        assert!(ids(&done.players[PlayerId::P1].graveyard).contains(&cast.id));
        assert_eq!(done.players[PlayerId::P2].hero.health, paused.players[PlayerId::P2].hero.health - 3);
    }

    #[test]
    fn r453_a_cast_x_card_asks_its_caster_for_x_first_1_up_to_their_mana_at_least_1_then_its_target_and_spends_no_mana() {
        let mut state = playing("r453-x-asked");
        state.players[PlayerId::P1].mana.current = 3;
        let events = run(&mut state, vec![cast_new(json_as(json!({ "def": x_target().id })))], None);
        assert_eq!(state.pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Number));
        assert_eq!(
            option_selections(&state),
            ["1", "2", "3"].iter().map(|option| Selection::Mode { option: option.to_string() }).collect::<Vec<_>>()
        );
        assert_eq!(events_of_type(&events, GameEventType::PromptOpened).len(), 1);
        // The other seat sees only that a prompt is open.
        assert_eq!(
            serde_json::to_value(&view_for(&state, PlayerId::P2).pending).unwrap(),
            json!({ "forYou": false, "pendingFor": "p1" })
        );

        let round = round_trip(&state);
        let two = answer_containing(&state, PlayerId::P1, "\"2\"");
        let asked = pb_act(&state, body_of(&two, PlayerId::P1));
        let asked_round = pb_act(&round, body_of(&two, PlayerId::P1));
        assert_eq!(hash_state(&asked_round), hash_state(&asked));
        // Then its target, asked of the card as it will resolve (X is on it now).
        assert_eq!(asked.pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Target));
        assert_eq!(only(asked.players[PlayerId::P1].resolving.clone()).x, Some(2));
        let hero = answer_hero(&asked, PlayerId::P1, PlayerId::P2);
        let done = pb_act(&asked, body_of(&hero, PlayerId::P1));
        assert_eq!(done.players[PlayerId::P2].hero.health, state.players[PlayerId::P2].hero.health - 2);
        assert_eq!(done.players[PlayerId::P1].mana.current, 3);

        // With no mana the one X there is is 1.
        let mut broke = playing("r453-x-broke");
        broke.players[PlayerId::P1].mana.current = 0;
        run(&mut broke, vec![cast_new(json_as(json!({ "def": x_spell().id })))], None);
        assert_eq!(option_labels(&broke), vec!["1".to_string()]);
    }

    #[test]
    fn r452_a_random_cast_s_x_is_its_caster_s_current_mana_and_at_least_1() {
        let mut rich = playing("r452-x-rich");
        rich.players[PlayerId::P1].mana.current = 3;
        let before = rich.players[PlayerId::P2].hero.health;
        run(&mut rich, vec![cast_random(json_as(json!({ "query": { "defId": x_spell().id }, "count": 1 })))], None);
        assert!(rich.pending.is_none());
        assert_eq!(rich.players[PlayerId::P2].hero.health, before - 3);
        assert_eq!(rich.players[PlayerId::P1].mana.current, 3);

        let mut broke = playing("r452-x-broke");
        broke.players[PlayerId::P1].mana.current = 0;
        let was = broke.players[PlayerId::P2].hero.health;
        run(&mut broke, vec![cast_random(json_as(json!({ "query": { "defId": x_spell().id }, "count": 1 })))], None);
        assert_eq!(broke.players[PlayerId::P2].hero.health, was - 1);
    }

    #[test]
    fn r453_a_cast_field_spell_or_trap_is_placed_a_trap_face_down_with_no_zone_it_fizzles_to_the_graveyard_still_a_play() {
        let mut state = playing("r453-zone");
        let before = state.players[PlayerId::P2].hero.health;
        run(&mut state, vec![cast_new(json_as(json!({ "def": cast_field().id })))], None);
        let field = state.players[PlayerId::P1].backrow.iter().flatten().find(|card| card.def_id == cast_field().id);
        assert!(field.is_some());
        assert_eq!(state.players[PlayerId::P2].hero.health, before - 1);

        let events = run(&mut state, vec![cast_new(json_as(json!({ "def": cast_trap().id })))], None);
        let trap = state.players[PlayerId::P1].backrow.iter().flatten().find(|card| card.def_id == cast_trap().id);
        assert!(trap.is_some());
        // Face-down to p2: the zone shows a back (R33, R227).
        assert!(view_for(&state, PlayerId::P2)
            .opponent
            .backrow
            .iter()
            .any(|view| matches!(view, Some(BackrowView::FaceDown(_)))));
        assert!(events.iter().any(|event| matches!(event, GameEvent::CardPlayed { former_id: Some(_), .. })));

        // Fill the backrow: the next cast Field Spell finds no zone and fizzles.
        let mut full = playing("r453-fizzle");
        for lane in [1, 2, 3, 4, 5] {
            put(&mut full, &cast_trap().id, slot(PlayerId::P1, Row::Backrow, lane));
        }
        let health = full.players[PlayerId::P2].hero.health;
        let played = full.counters.played;
        let fizzle = run(&mut full, vec![cast_new(json_as(json!({ "def": cast_field().id })))], None);
        // Counted as played, no Cry, and step 7 puts it in the graveyard (R138's landing).
        assert_eq!(full.counters.played, played + 1);
        assert_eq!(full.players[PlayerId::P2].hero.health, health);
        let landed = only(full.players[PlayerId::P1].graveyard.iter().filter(|card| card.def_id == cast_field().id).cloned());
        let permanent = only(fizzle.iter().filter_map(|event| match event {
            GameEvent::CardResolved { instance_id, permanent, .. } if *instance_id == landed.id => Some(*permanent),
            _ => None,
        }));
        assert!(!permanent);
    }

    #[test]
    fn r453_a_card_on_the_field_is_never_cast_a_graveyard_card_cast_without_a_rider_resolves_and_lands_in_the_graveyard_again(
    ) {
        let mut state = playing("r453-where");
        let on_field = put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 1));
        let played = state.counters.played;
        run(
            &mut state,
            vec![cast(json_as(json!({ "target": { "of": "instance", "instanceId": on_field.id } })))],
            None,
        );
        assert_eq!(state.counters.played, played);
        assert!(active_units_of(&state, PlayerId::P1).iter().map(|unit| unit.id.clone()).any(|id| id == on_field.id));

        let grave = in_graveyard(&mut state, &grave_spell().id, PlayerId::P1);
        let before = state.players[PlayerId::P2].hero.health;
        let events = run(
            &mut state,
            vec![cast(json_as(json!({ "target": { "of": "instance", "instanceId": grave.id } })))],
            None,
        );
        assert_eq!(state.players[PlayerId::P2].hero.health, before - 1);
        assert_eq!(state.counters.played, played + 1);
        assert_eq!(only(played_events(&events)).0, grave.id);
        assert!(ids(&state.players[PlayerId::P1].graveyard).contains(&grave.id));
        assert_eq!(state.players[PlayerId::P1].resolving, Vec::<CardInstance>::new());
    }
}

mod e12_random_casts_r452 {
    use super::*;

    #[test]
    fn r452_a_random_cast_makes_every_choice_at_random_declared_targets_and_modes_discover_picks_a_text_s_prompts_and_nothing_pauses(
    ) {
        let mut state = playing("r452-random");
        let jogg = only(in_hand(&mut state, &jogg_box().id, PlayerId::P1, 1));
        let hand_before = state.players[PlayerId::P1].hand.len();
        let result = pb_reduce(&state, json!({ "type": "play", "instanceId": jogg.id, "playerId": "p1" }));
        assert_eq!(result.error, None);
        let after = &result.state;
        assert!(after.pending.is_none());
        assert!(after.casts_resolving.is_none());
        // Three casts, each a play for 0 (R70), none of them Jogg's Box itself (B4.1, R387).
        let casts: Vec<(String, String, i32)> =
            played_events(&result.events).into_iter().filter(|(instance_id, _, _)| *instance_id != jogg.id).collect();
        assert_eq!(casts.len(), 3);
        for (_, def_id, cost_paid) in &casts {
            assert_eq!(*cost_paid, 0);
            assert!(RANDOM_POOL.contains(&def_id.as_str()));
        }
        // Nothing was asked of anyone.
        assert!(events_of_type(&result.events, GameEventType::PromptOpened).is_empty());
        // A random Discover put one of its options in the caster's hand.
        let discovers = casts.iter().filter(|(_, def_id, _)| *def_id == discover_spell().id).count();
        let added: Vec<String> =
            added_def_ids(&result.events).into_iter().filter(|def_id| DISCOVER_POOL.contains(&def_id.as_str())).collect();
        assert_eq!(added.len(), discovers);
        assert_eq!(after.players[PlayerId::P1].hand.len(), hand_before - 1 + discovers);
    }

    #[test]
    fn r452_random_casts_are_the_rng_s_the_same_state_casts_the_same_way_live_and_after_a_json_round_trip() {
        let mut state = playing("r452-determinism");
        let jogg = only(in_hand(&mut state, &jogg_box().id, PlayerId::P1, 1));
        let body = json!({ "type": "play", "instanceId": jogg.id, "playerId": "p1" });
        let live = pb_act(&state, body.clone());
        let again = pb_act(&round_trip(&state), body);
        assert_eq!(hash_state(&again), hash_state(&live));
    }

    #[test]
    fn r452_a_random_cast_that_targets_enemies_picks_an_enemy_whenever_one_is_legal() {
        for seed in ["r452-enemy-1", "r452-enemy-2", "r452-enemy-3", "r452-enemy-4", "r452-enemy-5"] {
            let mut state = playing(seed);
            let mine = put(&mut state, "fx-2", slot(PlayerId::P1, Row::Units, 2));
            put(&mut state, "fx-3", slot(PlayerId::P2, Row::Units, 3));
            let card = only(in_hand(&mut state, &solarius().id, PlayerId::P1, 1));
            let result = pb_reduce(
                &state,
                json!({ "type": "play", "instanceId": card.id, "zone": { "row": "units", "lane": 1 }, "playerId": "p1" }),
            );
            assert_eq!(result.error, None);
            assert!(result.state.pending.is_none());
            let hits = damage_targets(&result.events);
            assert!(!hits.is_empty());
            for target_id in &hits {
                assert!(![mine.id.clone(), "hero-p1".to_string()].contains(target_id));
            }
            assert_eq!(result.state.players[PlayerId::P1].hero.health, state.players[PlayerId::P1].hero.health);
        }
    }

    #[test]
    fn r452_the_other_player_s_prompts_are_theirs_a_random_cast_pauses_on_one_and_the_rest_of_the_casts_follow_the_answer() {
        let mut state = playing("r452-theirs");
        let before = state.players[PlayerId::P2].hero.health;
        let events = run(
            &mut state,
            vec![cast_random(json_as(json!({ "query": { "defId": their_choice().id }, "count": 2 })))],
            None,
        );
        assert_eq!(state.pending.as_ref().map(|pending| pending.player_id), Some(PlayerId::P2));
        assert_eq!(state.pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Mode));
        // At rest: no cast mode left on the state (the stack is transient).
        assert!(cast_modes_of(&state).is_empty());
        assert_eq!(events_of_type(&events, GameEventType::PromptOpened).len(), 1);

        let round = round_trip(&state);
        let answer = only(answers(&state, PlayerId::P2));
        let next = pb_act(&state, body_of(&answer, PlayerId::P2));
        let next_round = pb_act(&round, body_of(&answer, PlayerId::P2));
        assert_eq!(hash_state(&next_round), hash_state(&next));
        // The second random cast asks p2 again.
        assert_eq!(next.pending.as_ref().map(|pending| pending.player_id), Some(PlayerId::P2));
        let last = pb_act(&next, body_of(&only(answers(&next, PlayerId::P2)), PlayerId::P2));
        assert!(last.pending.is_none());
        // Each cast's answer hurt the one who answered (1) and its tail the enemy hero (1): p2 twice over.
        assert_eq!(last.players[PlayerId::P2].hero.health, before - 4);
        assert_eq!(last.players[PlayerId::P1].hero.health, state.players[PlayerId::P1].hero.health);
    }

    #[test]
    fn r452_a_cast_made_while_a_random_cast_resolves_is_random_too_and_its_echo_repeats_pick_at_random() {
        let mut state = playing("r452-nested");
        run(&mut state, vec![cast_random(json_as(json!({ "query": { "defId": CASTS_NAMED }, "count": 1 })))], None);
        assert!(state.pending.is_none());
        let graveyard_defs: Vec<String> =
            state.players[PlayerId::P1].graveyard.iter().map(|card| card.def_id.clone()).collect();
        assert!(graveyard_defs.contains(&CASTS_NAMED.to_string()));
        assert!(graveyard_defs.contains(&target_spell().id));

        let mut echoing = playing("r452-echo");
        let events = run(
            &mut echoing,
            vec![cast_random(json_as(json!({ "query": { "defId": ECHO_TARGET }, "count": 1 })))],
            None,
        );
        assert!(echoing.pending.is_none());
        // The cast and its one repeat each dealt 1 to a target the rng picked.
        assert_eq!(events_of_type(&events, GameEventType::Damage).len(), 2);
    }

    #[test]
    fn r452_a_random_cast_chain_stops_at_random_cast_chain_cap_casts() {
        // TS title: `R452 a random cast chain stops at RANDOM_CAST_CHAIN_CAP casts (${RANDOM_CAST_CHAIN_CAP})`.
        let mut state = playing("r452-cap");
        state.players[PlayerId::P2].hero.health = 1000;
        let card = only(in_hand(&mut state, PING_A, PlayerId::P1, 1));
        let result = pb_reduce(&state, json!({ "type": "play", "instanceId": card.id, "playerId": "p1" }));
        assert_eq!(result.error, None);
        // The played Ping A's two random casts each start a chain of at most the cap.
        let plays = events_of_type(&result.events, GameEventType::CardPlayed).len();
        assert_eq!(plays, 1 + 2 * RANDOM_CAST_CHAIN_CAP as usize);
        assert_eq!(result.state.players[PlayerId::P2].hero.health, 1000 - plays as i32);
        assert!(result.state.casts_resolving.is_none());
    }

    #[test]
    fn r452_a_random_answer_shows_the_other_seat_nothing_it_may_not_read_no_prompt_the_discovered_card_hidden() {
        let mut state = playing("r452-hidden");
        let card = only(in_hand(&mut state, CASTS_DISCOVER, PlayerId::P1, 1));
        let result = pb_reduce(&state, json!({ "type": "play", "instanceId": card.id, "playerId": "p1" }));
        let after = &result.state;
        assert!(after.pending.is_none());
        let count = result.events.len();
        let theirs = last_n(&view_for(after, PlayerId::P2).events, count);
        assert!(events_of_type(&theirs, GameEventType::PromptOpened).is_empty());
        let added = only(added_def_ids(&theirs));
        assert_eq!(added, HIDDEN_ID);
        let mine = only(added_def_ids(&last_n(&view_for(after, PlayerId::P1).events, count)));
        assert!(DISCOVER_POOL.contains(&mine.as_str()));
    }
}

mod e39_target_enemies_on_a_cast_its_caster_makes_r452 {
    use super::*;

    #[test]
    fn r452_a_card_carrying_the_target_enemies_enchantment_cast_offers_its_caster_only_enemies_when_there_is_one() {
        let mut state = playing("r452-enchanted");
        put(&mut state, "fx-2", slot(PlayerId::P1, Row::Units, 1));
        let enemy = put(&mut state, "fx-3", slot(PlayerId::P2, Row::Units, 1));
        let mut card = new_instance(&mut state, &target_spell().id, PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
        card.enchantments = Some(vec![json_as(json!({ "kind": "targetEnemies" }))]);
        state.players[PlayerId::P1].hand.push(card.clone());
        let mut events = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        cast_card(&mut EngineSink::new(&mut state, &mut events, &mut rng), &card, Default::default());
        assert_eq!(
            option_selections(&state),
            vec![Selection::Instance { instance_id: enemy.id.clone() }, Selection::Hero { player: PlayerId::P2 }]
        );
    }

    #[test]
    fn r452_the_prompts_a_targeting_enemies_cast_s_own_text_opens_are_narrowed_the_same_way() {
        let mut state = playing("r452-own-prompt");
        let card = new_instance(&mut state, &ask_target().id, PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
        state.players[PlayerId::P1].hand.push(card.clone());
        let mut events = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        cast_card(
            &mut EngineSink::new(&mut state, &mut events, &mut rng),
            &card,
            json_as(json!({ "targetEnemies": true })),
        );
        assert_eq!(option_selections(&state), vec![Selection::Hero { player: PlayerId::P2 }]);
    }

    #[test]
    fn r452_with_no_enemy_to_pick_or_too_few_every_option_stays() {
        let mut state = playing("r452-no-enemy");
        let own = put(&mut state, "fx-2", slot(PlayerId::P1, Row::Units, 1));
        let options = vec![Selection::Instance { instance_id: own.id.clone() }, Selection::Hero { player: PlayerId::P1 }];
        assert_eq!(prefer_enemies(&state, PlayerId::P1, &options, |selection: &Selection| selection.clone(), 1), options);
        let mut mixed = options.clone();
        mixed.push(Selection::Hero { player: PlayerId::P2 });
        assert_eq!(
            prefer_enemies(&state, PlayerId::P1, &mixed, |selection: &Selection| selection.clone(), 1),
            vec![Selection::Hero { player: PlayerId::P2 }]
        );
        assert_eq!(prefer_enemies(&state, PlayerId::P1, &mixed, |selection: &Selection| selection.clone(), 2), mixed);
    }
}

mod r656_aimed_random_targets {
    use super::*;

    #[test]
    fn r656_a_helpful_pick_under_target_enemies_narrows_to_friends_when_one_is_legal() {
        let mut state = playing("r651-enchanted");
        let own = put(&mut state, "fx-2", slot(PlayerId::P1, Row::Units, 1));
        put(&mut state, "fx-3", slot(PlayerId::P2, Row::Units, 1));
        let mut card = new_instance(&mut state, HEAL_FRIEND, PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
        card.enchantments = Some(vec![json_as(json!({ "kind": "targetEnemies" }))]);
        state.players[PlayerId::P1].hand.push(card.clone());
        let mut events = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        cast_card(
            &mut EngineSink::new(&mut state, &mut events, &mut rng),
            &card,
            json_as(json!({ "targetEnemies": true })),
        );
        assert_eq!(option_selections(&state), vec![Selection::Instance { instance_id: own.id.clone() }]);
    }

    #[test]
    fn r656_with_no_friend_to_pick_or_too_few_every_option_stays_and_a_mode_pick_is_kept_either_way() {
        let mut state = playing("r651-no-friend");
        let foe = put(&mut state, "fx-3", slot(PlayerId::P2, Row::Units, 1));
        let options = vec![Selection::Instance { instance_id: foe.id.clone() }, Selection::Hero { player: PlayerId::P2 }];
        assert_eq!(prefer_friends(&state, PlayerId::P1, &options, |selection: &Selection| selection.clone(), 1), options);
        let own = put(&mut state, "fx-2", slot(PlayerId::P1, Row::Units, 1));
        let mode = Selection::Mode { option: "a".to_string() };
        let mut mixed = options.clone();
        mixed.push(mode.clone());
        mixed.push(Selection::Instance { instance_id: own.id.clone() });
        mixed.push(Selection::Hero { player: PlayerId::P1 });
        assert_eq!(
            prefer_friends(&state, PlayerId::P1, &mixed, |selection: &Selection| selection.clone(), 1),
            vec![mode, Selection::Instance { instance_id: own.id.clone() }, Selection::Hero { player: PlayerId::P1 }]
        );
        assert_eq!(prefer_friends(&state, PlayerId::P1, &mixed, |selection: &Selection| selection.clone(), 4), mixed);
    }

    #[test]
    fn r656_a_random_cast_that_targets_enemies_aims_each_pick_harm_at_enemies_help_at_friends() {
        let mut saw_damage = false;
        let mut saw_heal = false;
        for seed in ["r651-aim-1", "r651-aim-2", "r651-aim-3", "r651-aim-4", "r651-aim-5"] {
            let mut state = playing(seed);
            let mine = put(&mut state, "fx-2", slot(PlayerId::P1, Row::Units, 1));
            let foe = put(&mut state, "fx-3", slot(PlayerId::P2, Row::Units, 1));
            set_damage(&mut state, &mine.id, 1);
            set_damage(&mut state, &foe.id, 1);
            let events = run(
                &mut state,
                vec![cast_random(json_as(json!({
                    "query": { "defId": [target_spell().id, HEAL_FRIEND] },
                    "count": 4,
                    "targetEnemies": true,
                })))],
                None,
            );
            assert!(state.pending.is_none());
            for target_id in damage_targets(&events) {
                saw_damage = true;
                assert!([foe.id.clone(), "hero-p2".to_string()].contains(&target_id));
            }
            for target_id in healed_targets(&events) {
                saw_heal = true;
                assert_eq!(target_id, mine.id);
            }
        }
        assert!(saw_damage);
        assert!(saw_heal);
    }

    #[test]
    fn r656_jogg_s_box_stays_fully_random_with_no_target_enemies_even_a_helpful_cast_may_land_on_enemies() {
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for seed in ["r651-box-1", "r651-box-2", "r651-box-3", "r651-box-4", "r651-box-5", "r651-box-6"] {
            let mut state = playing(seed);
            let mine = put(&mut state, "fx-2", slot(PlayerId::P1, Row::Units, 1));
            let foe = put(&mut state, "fx-3", slot(PlayerId::P2, Row::Units, 1));
            set_damage(&mut state, &mine.id, 1);
            set_damage(&mut state, &foe.id, 1);
            let events = run(
                &mut state,
                vec![cast_random(json_as(json!({ "query": { "defId": [HEAL_FRIEND] }, "count": 2 })))],
                None,
            );
            assert!(state.pending.is_none());
            for target_id in healed_targets(&events) {
                seen.insert(if target_id == mine.id { "friend" } else { "enemy" });
            }
        }
        assert_eq!(seen, BTreeSet::from(["friend", "enemy"]));
    }
}
