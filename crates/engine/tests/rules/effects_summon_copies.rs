//! The three summon verbs that make a card out of something other than a fixed def id:
//! R21's random keywords on a summon (#80 Zao Gao), §10.7's copy semantics (#12 Duplicating
//! Felinors, #61 Prejudiced Postdoc, R57) and §10.7's random pool (#67 Zoomerbin Oomen, §5.1).
//!
//! The fixture defs and scripts live here, on top of the shared fixture catalog, so no shared
//! fixture has to grow for them (CLAUDE.md, BUILD §0). Every assertion runs through a real
//! `EffectContext` built by `makeContext`, the way a hook's effects are applied.
//!
//! Port of `packages/engine/test/effects-summon-copies.test.ts`.

use jackioh_engine::effects::{damage, summon, summon_copy, summon_random};
use jackioh_engine::testkit::*;

use super::fixtures::harness::{events_of_type, new_game, put, slot};

// ---------------------------------------------------------------------------
// Fixture cards.
// ---------------------------------------------------------------------------

/// TS `defOfKind`. TS numbered each def from a module counter (`nextIndex`, starting at 760 and
/// raised before each def); the index each def drew is written out at its call.
fn def_of_kind(name: &str, index: i32, type_: &str, overrides: Value) -> CardDef {
    let mut def = json!({
        "id": format!("cp-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (copies)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "attack": 2, "health": 2, "keywords": [], "text": name },
        "radiant": { "attack": 4, "health": 4, "keywords": [], "text": format!("{name} radiant") },
    });
    spread(&mut def, overrides);
    json_as(def)
}

/// TS `{ ...base, ...overrides }`: the override's keys replace the base's.
fn spread(target: &mut Value, overrides: Value) {
    if let (Some(into), Value::Object(from)) = (target.as_object_mut(), overrides) {
        for (key, value) in from {
            into.insert(key, value);
        }
    }
}

/// The body every copy test duplicates: a plain 2/2 with nothing of its own.
fn body() -> CardDef {
    def_of_kind("body", 761, "Unit", json!({}))
}

/// #12-style: a Cry that pings the enemy hero, so "a copy fires no Cry" is observable (§6.2, R1).
fn crier() -> CardDef {
    def_of_kind("crier", 762, "Unit", json!({}))
}

/// Two defs that can share a `summonRandom` pool, so the draw has something to choose between.
fn pool_a() -> CardDef {
    def_of_kind("pool-a", 763, "Unit", json!({}))
}

fn pool_b() -> CardDef {
    def_of_kind("pool-b", 764, "Unit", json!({}))
}

/// §3.2: a Trap enters the backrow face-down, even when a random pool put it there (R33).
fn pool_trap() -> CardDef {
    def_of_kind(
        "pool-trap",
        765,
        "Trap",
        json!({
            "base": { "keywords": [], "text": "trap" },
            "radiant": { "keywords": [], "text": "trap" },
        }),
    )
}

/// §7's shared Rush Token from the fixture catalog: printed Rush, so R21 draws from the other ten.
const RUSH_TOKEN: &str = "fx-token-rush";

fn defs() -> Vec<CardDef> {
    vec![body(), crier(), pool_a(), pool_b(), pool_trap()]
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn scripts() -> IndexMap<String, CardScripts> {
    let mut scripts = IndexMap::new();
    scripts.insert(
        crier().id,
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 3 })))]
            })),
            ..Script::default()
        }),
    );
    scripts
}

/// A fresh game whose catalog and script registry also carry this file's fixtures.
fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut registry = registered_scripts().clone();
    registry.extend(scripts());
    register_scripts(registry);
    state.turn = 3;
    state
}

/// TS `RunOptions = { controller?, self?, targets? }`.
#[derive(Default)]
struct RunOptions {
    controller: Option<PlayerId>,
    self_: Option<CardInstance>,
    targets: Option<Vec<Selection>>,
}

/// Apply a whole effect list the way a hook's list is applied: one context, one rng, in order.
/// TS's `sinkFor(state)` is the sink built here: its rng starts at the state's cursor, as reduce does.
fn run_all(state: &mut GameState, effects: &[Effect], options: RunOptions) -> Vec<GameEvent> {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            options.self_.as_ref(),
            HookOptions {
                controller: Some(options.controller.unwrap_or(PlayerId::P1)),
                targets: options.targets,
                ..HookOptions::default()
            },
        );
        for effect in effects {
            (effect.apply)(&mut ctx);
        }
    }
    state.rng_cursor = rng.cursor();
    events
}

fn run(state: &mut GameState, effect: Effect, options: RunOptions) -> Vec<GameEvent> {
    run_all(state, &[effect], options)
}

/// TS `put(state, defId, slot, { radiant: true })`, the harness's `put` with its option: the card is
/// made in its owner's hand, flagged Radiant, then placed (TS's order), and a copy of it as placed comes
/// back. Written here from part 1's `new_instance` and `zones::place_on_field`, since the harness's
/// `put` is called with three arguments in every other file of this part.
fn put_radiant(state: &mut GameState, def_id: &str, at: ZoneSlot) -> CardInstance {
    let mut card = new_instance(state, def_id, at.player, Zone::Hand { player: at.player });
    card.radiant = true;
    assert!(
        place_on_field(state, &mut card, at, PlaceOnFieldOptions::default()),
        "could not place {def_id} in {} {}",
        at.row.as_str(),
        at.lane
    );
    live(state, &card.id)
}

/// TS `unitAt(state, lane, player = "p1")`.
fn unit_at(state: &GameState, lane: i32, player: PlayerId) -> Option<CardInstance> {
    card_at(state, slot(player, Row::Units, lane)).cloned()
}

fn keyword_kinds_at(state: &GameState, lane: i32) -> Vec<KeywordKind> {
    unit_at(state, lane, PlayerId::P1)
        .map(|card| card.granted_keywords.iter().map(Keyword::kind).collect())
        .unwrap_or_default()
}

/// TS held the live instance and read it after an effect; Rust reads the card again by id.
fn live(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id).cloned().unwrap_or_else(|| panic!("no card {id} in the state"))
}

/// TS wrote through the live instance; Rust writes through the card found by id.
fn live_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).unwrap_or_else(|| panic!("no card {id} in the state"))
}

/// `eventsOfType(events, "summoned").map((event) => event.lane)`.
fn summoned_lanes(events: &[GameEvent]) -> Vec<i32> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Summoned { lane, .. } => Some(*lane),
            _ => None,
        })
        .collect()
}

/// `eventsOfType(events, "summoned").map((event) => [event.row, event.lane])`.
fn summoned_rows_and_lanes(events: &[GameEvent]) -> Vec<(Row, i32)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Summoned { row, lane, .. } => Some((*row, *lane)),
            _ => None,
        })
        .collect()
}

fn kinds_of(keywords: &[Keyword]) -> Vec<KeywordKind> {
    keywords.iter().map(Keyword::kind).collect()
}

// ---------------------------------------------------------------------------
// summon({ randomKeywords }) — R21, #80 Zao Gao
// ---------------------------------------------------------------------------

mod summon_with_random_keywords_r21_s7_80 {
    use super::*;

    #[test]
    fn r21_gives_the_summoned_token_exactly_2_distinct_keywords_from_the_pool() {
        let mut state = game("kw-one");

        run(
            &mut state,
            summon(json_as(json!({ "defId": RUSH_TOKEN, "randomKeywords": 2 }))),
            RunOptions::default(),
        );

        let token = unit_at(&state, 1, PlayerId::P1);
        assert_eq!(token.as_ref().map(|card| card.def_id.as_str()), Some(RUSH_TOKEN));
        let kinds = keyword_kinds_at(&state, 1);
        assert_eq!(kinds.len(), 2);
        assert_eq!(kinds.iter().collect::<IndexSet<_>>().len(), 2);
        // R21's pool, and never Rush, which the Rush Token already has.
        for kind in &kinds {
            assert!(RANDOM_KEYWORD_POOL.iter().any(|entry| entry.starts_with(kind.as_str())));
            assert_ne!(*kind, KeywordKind::Rush);
        }
        // The layers really see them (§10.4), so the grant landed on the summoned instance.
        let token = token.expect("the summoned token");
        let view = unit_view(&state, &token);
        assert!(view.keywords.len() >= 3);
    }

    #[test]
    fn r21_rolls_each_tokens_keywords_independently_so_two_tokens_in_one_list_can_differ() {
        let pairs: Vec<(Vec<KeywordKind>, Vec<KeywordKind>)> = (0..12)
            .map(|seed| {
                let mut state = game(&format!("kw-pair-{seed}"));
                run_all(
                    &mut state,
                    &[
                        summon(json_as(json!({ "defId": RUSH_TOKEN, "randomKeywords": 2 }))),
                        summon(json_as(json!({ "defId": RUSH_TOKEN, "randomKeywords": 2 }))),
                    ],
                    RunOptions::default(),
                );
                (keyword_kinds_at(&state, 1), keyword_kinds_at(&state, 2))
            })
            .collect();

        for (first, second) in &pairs {
            assert_eq!(first.iter().collect::<IndexSet<_>>().len(), 2);
            assert_eq!(second.iter().collect::<IndexSet<_>>().len(), 2);
        }
        // Independent rolls, so on some seed the two tokens are not the same pair of keywords.
        assert!(pairs.iter().any(|(first, second)| first != second));
    }

    #[test]
    fn r21_grants_nothing_when_the_summon_fizzled_and_takes_no_draw_for_it() {
        let mut state = game("kw-fizzle");
        for lane in [1, 2, 3, 4, 5] {
            put(&mut state, &body().id, slot(PlayerId::P1, Row::Units, lane), json!({}));
        }
        let cursor_before = state.rng_cursor;

        let events = run(
            &mut state,
            summon(json_as(json!({ "defId": RUSH_TOKEN, "randomKeywords": 2 }))),
            RunOptions::default(),
        );

        assert!(events_of_type(&events, GameEventType::Summoned).is_empty());
        assert!(events_of_type(&events, GameEventType::KeywordGranted).is_empty());
        assert_eq!(state.rng_cursor, cursor_before);
    }
}

// ---------------------------------------------------------------------------
// summonCopy — §10.7, R57, #12 and #61
// ---------------------------------------------------------------------------

mod summon_copy_s10_7_copy_semantics_r57_12_61 {
    use super::*;

    #[test]
    fn r57_keeps_the_radiant_flag_the_buffs_the_granted_keywords_vanilla_and_stats_override() {
        let mut state = game("copy-keeps");
        let source = put_radiant(&mut state, &body().id, slot(PlayerId::P1, Row::Units, 2));
        {
            let card = live_mut(&mut state, &source.id);
            card.buffs = AttackHealth { attack: 2, health: 3 };
            card.granted_keywords = vec![Keyword::Taunt, Keyword::Armor { n: 1 }];
            card.stats_override = Some(AttackHealth { attack: 7, health: 9 });
            card.vanilla = true;
        }

        let events = run(
            &mut state,
            summon_copy(json_as(json!({ "of": { "of": "instance", "instanceId": source.id } }))),
            RunOptions::default(),
        );

        // R64: the leftmost free zone, and one `summoned` event for it.
        assert_eq!(summoned_lanes(&events), vec![1]);
        let copy = unit_at(&state, 1, PlayerId::P1).expect("the copy");
        let source = live(&state, &source.id);
        assert_ne!(copy.id, source.id);
        assert_eq!(copy.def_id, source.def_id);
        assert_eq!(copy.owner, PlayerId::P1);
        assert!(copy.radiant);
        assert!(copy.vanilla);
        assert_eq!(copy.buffs, AttackHealth { attack: 2, health: 3 });
        assert_eq!(copy.stats_override, Some(AttackHealth { attack: 7, health: 9 }));
        assert_eq!(kinds_of(&copy.granted_keywords), vec![KeywordKind::Taunt, KeywordKind::Armor]);

        // §10.4 reads the copy through the same layers: the override is the base face, buffs on top.
        let view = unit_view(&state, &copy);
        assert_eq!(view.attack, 9);
        assert_eq!(view.max_health, 12);

        // The copy's state is its own: writing the source afterwards does not reach it. (TS pinned
        // this as object identity, `not.toBe`; Rust values are never shared, so the write is made.)
        {
            let card = live_mut(&mut state, &source.id);
            card.buffs = AttackHealth { attack: 0, health: 0 };
            card.stats_override = None;
            card.granted_keywords.clear();
        }
        let copy_after = live(&state, &copy.id);
        assert_eq!(copy_after.buffs, AttackHealth { attack: 2, health: 3 });
        assert_eq!(copy_after.stats_override, Some(AttackHealth { attack: 7, health: 9 }));
        assert_eq!(
            kinds_of(&copy_after.granted_keywords),
            vec![KeywordKind::Taunt, KeywordKind::Armor]
        );
    }

    #[test]
    fn r57_resets_damage_exertion_counters_and_summoned_turn_on_the_copy() {
        let mut state = game("copy-resets");
        let source = put(&mut state, &body().id, slot(PlayerId::P1, Row::Units, 2), json!({}));
        {
            let card = live_mut(&mut state, &source.id);
            card.damage = 4;
            card.exertion = Exertion {
                attacked: true,
                switched: true,
                attacks: None,
            };
            card.counters = Counters {
                plague: Some(2),
                grade: Some(1),
            };
            card.memory = IndexMap::from([("seen".to_string(), json!(7))]);
            card.summoned_turn = Some(1);
            card.position = Some(Position::Def);
        }

        run(
            &mut state,
            summon_copy(json_as(json!({ "of": { "of": "instance", "instanceId": source.id } }))),
            RunOptions::default(),
        );

        let copy = unit_at(&state, 1, PlayerId::P1).expect("the copy");
        assert_eq!(copy.damage, 0);
        assert_eq!(
            copy.exertion,
            Exertion {
                attacked: false,
                switched: false,
                attacks: None,
            }
        );
        assert_eq!(copy.counters, Counters::default());
        assert!(copy.memory.is_empty());
        // §4.1: a new body enters in Attack Position and is summoning sick for this turn.
        assert_eq!(copy.position, Some(Position::Atk));
        assert_eq!(copy.summoned_turn, Some(state.turn));
        // The source is untouched by its own copy.
        let source = live(&state, &source.id);
        assert_eq!(source.damage, 4);
        assert_eq!(
            source.counters,
            Counters {
                plague: Some(2),
                grade: Some(1),
            }
        );
    }

    #[test]
    fn s8_3_61_vanilla_true_and_granted_keywords_false_keep_the_buffs_and_the_radiant_flag() {
        let mut state = game("copy-postdoc");
        let source = put_radiant(&mut state, &body().id, slot(PlayerId::P1, Row::Units, 2));
        {
            let card = live_mut(&mut state, &source.id);
            card.buffs = AttackHealth { attack: 1, health: 1 };
            card.granted_keywords = vec![Keyword::Taunt];
            card.damage = 2;
        }

        run(
            &mut state,
            summon_copy(json_as(json!({ "of": { "of": "chosen" }, "vanilla": true, "grantedKeywords": false }))),
            RunOptions {
                targets: Some(vec![Selection::Instance {
                    instance_id: source.id.clone(),
                }]),
                ..RunOptions::default()
            },
        );

        let copy = unit_at(&state, 1, PlayerId::P1).expect("the copy");
        assert!(copy.vanilla);
        assert!(copy.granted_keywords.is_empty());
        assert_eq!(copy.buffs, AttackHealth { attack: 1, health: 1 });
        assert!(copy.radiant);
        assert_eq!(copy.damage, 0);
        // The source keeps its own text and its own keyword: only the copy is textless (R23).
        let source = live(&state, &source.id);
        assert!(!source.vanilla);
        assert_eq!(kinds_of(&source.granted_keywords), vec![KeywordKind::Taunt]);
    }

    #[test]
    fn s6_2_the_copy_fires_no_cry_where_the_sources_own_cry_is_observable() {
        let mut state = game("copy-no-cry");
        let source = put(&mut state, &crier().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let before = state.players.p2.hero.health;

        let events = run(
            &mut state,
            summon_copy(json_as(json!({ "of": { "of": "self" }, "player": "self" }))),
            RunOptions {
                self_: Some(source.clone()),
                ..RunOptions::default()
            },
        );

        assert_eq!(unit_at(&state, 2, PlayerId::P1).map(|card| card.def_id), Some(crier().id));
        assert!(events_of_type(&events, GameEventType::Damage).is_empty());
        assert_eq!(state.players.p2.hero.health, before);

        // The fixture's Cry really does ping the enemy hero, so the assertion above means something.
        let source = live(&state, &source.id);
        let mut events = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        {
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            run_hook(
                &mut sink,
                &source,
                HookName::Cry,
                HookOptions {
                    controller: Some(PlayerId::P1),
                    ..HookOptions::default()
                },
            );
        }
        assert_eq!(events_of_type(&events, GameEventType::Damage).len(), 1);
        assert_eq!(state.players.p2.hero.health, before - 3);
    }

    #[test]
    fn r64_fizzles_silently_with_a_full_row_creating_nothing() {
        let mut state = game("copy-full");
        let source = put(&mut state, &body().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        for lane in [2, 3, 4, 5] {
            put(&mut state, &body().id, slot(PlayerId::P1, Row::Units, lane), json!({}));
        }
        let ids_before = state.next_id;

        let events = run(
            &mut state,
            summon_copy(json_as(json!({ "of": { "of": "instance", "instanceId": source.id } }))),
            RunOptions::default(),
        );

        assert!(events_of_type(&events, GameEventType::Summoned).is_empty());
        assert_eq!(state.next_id, ids_before);
    }

    #[test]
    fn s6_3_fizzles_silently_when_the_target_resolves_to_nothing() {
        let mut state = game("copy-no-target");
        let ids_before = state.next_id;

        // No selection carried, so `{ of: "chosen" }` names nothing; `{ of: "self" }` with no self too.
        let mut events = run(
            &mut state,
            summon_copy(json_as(json!({ "of": { "of": "chosen" } }))),
            RunOptions::default(),
        );
        events.extend(run(
            &mut state,
            summon_copy(json_as(json!({ "of": { "of": "self" } }))),
            RunOptions::default(),
        ));
        events.extend(run(
            &mut state,
            summon_copy(json_as(json!({ "of": { "of": "selfHero" } }))),
            RunOptions::default(),
        ));

        assert!(events.is_empty());
        assert_eq!(state.next_id, ids_before);
        assert!(unit_at(&state, 1, PlayerId::P1).is_none());
    }
}

// ---------------------------------------------------------------------------
// summonRandom — §5.1, §10.7, R60, #67 Zoomerbin Oomen
// ---------------------------------------------------------------------------

mod summon_random_s5_1_s10_7_r60_67 {
    use super::*;

    /// TS `const pool = { defId: [poolA.id, poolB.id] }`.
    fn pool() -> Value {
        json!({ "defId": [pool_a().id, pool_b().id] })
    }

    #[test]
    fn s10_7_draws_one_def_from_the_query_pool_and_summons_it_per_r64() {
        let picks: Vec<Option<String>> = (0..12)
            .map(|seed| {
                let mut state = game(&format!("random-pool-{seed}"));
                let events = run(
                    &mut state,
                    summon_random(json_as(json!({ "query": pool() }))),
                    RunOptions::default(),
                );
                assert_eq!(events_of_type(&events, GameEventType::Summoned).len(), 1);
                unit_at(&state, 1, PlayerId::P1).map(|card| card.def_id)
            })
            .collect();

        for pick in &picks {
            assert!([Some(pool_a().id), Some(pool_b().id)].contains(pick));
        }
        // A draw, not the first entry of the pool: both members come up across the seeds.
        assert_eq!(picks.iter().collect::<IndexSet<_>>().len(), 2);
    }

    #[test]
    fn s5_1_never_summons_the_requesting_cards_own_definition() {
        for seed in 0..12 {
            let mut state = game(&format!("random-exclude-{seed}"));
            let self_ = put(&mut state, &pool_a().id, slot(PlayerId::P1, Row::Units, 5), json!({}));

            run(
                &mut state,
                summon_random(json_as(json!({ "query": pool() }))),
                RunOptions {
                    self_: Some(self_),
                    ..RunOptions::default()
                },
            );

            assert_eq!(unit_at(&state, 1, PlayerId::P1).map(|card| card.def_id), Some(pool_b().id));
        }
    }

    #[test]
    fn r33_summons_a_trap_into_the_named_backrow_lane_face_down() {
        let mut state = game("random-trap");

        let events = run(
            &mut state,
            summon_random(json_as(json!({ "query": { "defId": [pool_trap().id] }, "lane": 2 }))),
            RunOptions::default(),
        );

        assert_eq!(summoned_rows_and_lanes(&events), vec![(Row::Backrow, 2)]);
        let trap = card_at(&state, slot(PlayerId::P1, Row::Backrow, 2)).cloned();
        assert_eq!(trap.as_ref().map(|card| card.def_id.clone()), Some(pool_trap().id));
        assert_eq!(trap.as_ref().and_then(|card| card.face_up), None);
    }

    #[test]
    fn r47_fizzles_on_an_occupied_backrow_lane_but_a_locked_one_takes_the_summon_r688() {
        let mut occupied = game("random-occupied");
        put(&mut occupied, &pool_trap().id, slot(PlayerId::P1, Row::Backrow, 2), json!({}));
        let before = occupied.next_id;
        assert!(
            run(
                &mut occupied,
                summon_random(json_as(json!({ "query": { "defId": [pool_trap().id] }, "lane": 2 }))),
                RunOptions::default(),
            )
            .is_empty()
        );
        assert_eq!(occupied.next_id, before);

        // A Lock refuses plays, never summons: the generic summon lands and the lock stays.
        let mut locked = game("random-locked");
        lock_zone(&mut locked, slot(PlayerId::P1, Row::Backrow, 2));
        assert!(
            !run(
                &mut locked,
                summon_random(json_as(json!({ "query": { "defId": [pool_trap().id] }, "lane": 2 }))),
                RunOptions::default(),
            )
            .is_empty()
        );
        assert_eq!(
            card_at(&locked, slot(PlayerId::P1, Row::Backrow, 2)).map(|card| card.def_id.clone()),
            Some(pool_trap().id)
        );
    }

    #[test]
    fn s9_3_takes_its_rng_draw_inside_apply_and_the_same_cursor_gives_the_same_pick() {
        let mut state = game("random-cursor");
        let state_cursor = state.rng_cursor;
        let mut events = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);

        // Building the effect touches no rng at all: a draw here would escape the reducer.
        let effect = summon_random(json_as(json!({ "query": pool() })));
        assert_eq!(rng.cursor(), state_cursor);

        {
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            let mut ctx = make_context(
                &mut sink,
                None,
                HookOptions {
                    controller: Some(PlayerId::P1),
                    ..HookOptions::default()
                },
            );
            (effect.apply)(&mut ctx);
        }
        assert!(rng.cursor() > state_cursor);

        // Two contexts built at the same (seed, cursor) draw the same def (§9.3, R60).
        let mut first = game("random-same");
        let mut second = game("random-same");
        run(&mut first, summon_random(json_as(json!({ "query": pool() }))), RunOptions::default());
        run(&mut second, summon_random(json_as(json!({ "query": pool() }))), RunOptions::default());
        assert_eq!(
            unit_at(&second, 1, PlayerId::P1).map(|card| card.def_id),
            unit_at(&first, 1, PlayerId::P1).map(|card| card.def_id)
        );
    }

    #[test]
    fn s5_1_fizzles_silently_when_the_pool_is_empty_taking_no_draw() {
        let mut state = game("random-empty");
        let cursor_before = state.rng_cursor;

        let events = run(
            &mut state,
            summon_random(json_as(json!({ "query": { "defId": ["cp-does-not-exist"] } }))),
            RunOptions::default(),
        );

        assert!(events.is_empty());
        assert_eq!(state.rng_cursor, cursor_before);
    }
}
