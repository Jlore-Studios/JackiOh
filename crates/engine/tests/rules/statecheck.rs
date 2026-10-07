//! The state check of SPEC §4.5 (BUILD M2-T5): collect the dying, check the heroes, fire Death
//! triggers in R68 order, return Reborn units to their reserved zones, repeat until stable.
//! The fixture defs and scripts this file needs live here, registered on top of the shared
//! fixture catalog, so no shared fixture has to grow for them (CLAUDE.md, BUILD §0).
//!
//! Port of `packages/engine/test/statecheck.test.ts`.

use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::token_def;
use crate::rules::fixtures::harness::{events_of_type, new_game, put, slot};

// ---------------------------------------------------------------------------
// The sink: TS `sinkFor(state, events)`, a sink whose rng starts at the state's cursor, as reduce
// does. Rust's `EngineSink` borrows the state, so the event list and the rng live here and each
// call borrows the state again; the state is read between calls as TS read `state`.
// ---------------------------------------------------------------------------

struct SinkFor {
    events: Vec<GameEvent>,
    rng: Rng,
}

fn sink_for(state: &GameState) -> SinkFor {
    SinkFor {
        events: Vec::new(),
        rng: Rng::new(&state.seed, state.rng_cursor),
    }
}

impl SinkFor {
    fn on<'a>(&'a mut self, state: &'a mut GameState) -> EngineSink<'a> {
        EngineSink::new(state, &mut self.events, &mut self.rng)
    }
}

/// The live card TS's test kept a handle on, read back from the state.
fn card<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).unwrap_or_else(|| panic!("no instance {id}"))
}

/// The live card, to write through as TS's test wrote through its handle.
fn card_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).unwrap_or_else(|| panic!("no instance {id}"))
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

/// `eventsOfType(events, type).map((e) => e.instanceId)` for the event types that carry one.
fn instance_ids(events: &[GameEvent], kind: GameEventType) -> Vec<String> {
    events_of_type(events, kind)
        .iter()
        .filter_map(|event| match event {
            GameEvent::Destroyed { instance_id, .. }
            | GameEvent::EnteredGraveyard { instance_id, .. }
            | GameEvent::Summoned { instance_id, .. } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

fn to_json<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// The text of a caught panic (TS `toThrow(/…/)` reads the thrown error's message).
fn panic_text(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else {
        String::new()
    }
}

// ---------------------------------------------------------------------------
// Fixture effects: the three verbs these tests need before M3-T1 ships them.
// ---------------------------------------------------------------------------

/// One effect, one hit per unit on the board: the Big Felinor / Jlockeed Shredder case (R59).
fn smite_every_unit(amount: i32) -> Effect {
    Effect::new("statecheck:smiteEveryUnit", move |ctx| {
        for player in PLAYER_IDS {
            let units: Vec<CardInstance> = active_units_of(ctx.state, player).into_iter().cloned().collect();
            for unit in units {
                let source = ctx.self_.clone();
                deal_damage(
                    ctx,
                    DamageArgs {
                        source,
                        target: DamageTarget::Unit { instance: unit },
                        amount,
                        flags: None,
                    },
                );
            }
        }
    })
}

/// Summon per R64: the leftmost empty, unlocked, unreserved unit zone, or nothing.
fn summon_first_free(def_id: String) -> Effect {
    Effect::new("statecheck:summonFirstFree", move |ctx| {
        let controller = ctx.controller;
        let Some(zone) = first_free_zone(ctx.state, controller, Row::Units) else {
            return;
        };
        let mut card = new_instance(ctx.state, &def_id, controller, Zone::Hand { player: controller });
        if !place_on_field(ctx.state, &mut card, zone, Default::default()) {
            return;
        }
        ctx.events.push(GameEvent::Summoned {
            player: controller,
            instance_id: card.id.clone(),
            def_id: def_id.clone(),
            row: zone.row,
            lane: zone.lane,
            former_id: None,
            arrived_during: None,
            exits_from: None,
        });
    })
}

/// Lock (§3.2) this controller's unit lane 1, which is where the R688 test parks its Reborn unit.
fn lock_own_first_lane() -> Effect {
    Effect::new("statecheck:lockLane1", |ctx| {
        let zone = ZoneRef {
            player: ctx.controller,
            row: Row::Units,
            lane: 1,
        };
        lock_zone(ctx.state, zone);
        ctx.events.push(GameEvent::Locked {
            player: zone.player,
            row: zone.row,
            lane: zone.lane,
        });
    })
}

// ---------------------------------------------------------------------------
// Fixture cards. TS numbered them from a module counter starting at 900; each def's index is
// written out here in the order TS created them.
// ---------------------------------------------------------------------------

fn unit_def_of(name: &str, index: u32, attack: i32, health: i32, keywords: Value) -> CardDef {
    json_as(json!({
        "id": format!("sc-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (state check)"),
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": { "attack": attack, "health": health, "keywords": keywords, "text": name },
        "radiant": { "attack": attack * 2, "health": health * 2, "keywords": keywords, "text": format!("{name} radiant") },
    }))
}

fn spell_def_of(name: &str, index: u32) -> CardDef {
    json_as(json!({
        "id": format!("sc-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (state check)"),
        "set": "Core",
        "type": "Spell",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    }))
}

/// A 1/1 whose Death trigger pings the enemy hero, so the `damage` events give the firing order.
fn pinger() -> CardDef {
    unit_def_of("pinger", 901, 1, 1, json!([]))
}
/// Reborn plus a Death trigger that summons: one card for R8, R64 and R47's zone logic.
fn reborn_summoner() -> CardDef {
    unit_def_of("reborn-summoner", 902, 2, 2, json!([{ "kind": "Reborn" }]))
}
/// Reborn with a Cry, which §4.5 step 4 says must not fire on the way back.
fn reborn_crier() -> CardDef {
    unit_def_of("reborn-crier", 903, 2, 2, json!([{ "kind": "Reborn" }]))
}
/// Reborn with no hooks at all, for the Locked-zone return (R688).
fn reborn_plain() -> CardDef {
    unit_def_of("reborn-plain", 904, 2, 2, json!([{ "kind": "Reborn" }]))
}
/// Its Death trigger Locks its controller's lane 1 while the state check is still running.
fn locker() -> CardDef {
    unit_def_of("locker", 905, 1, 1, json!([]))
}
/// #55r / #66: Indestructible with printed Taunt, for R46's "loses Taunt until end of turn".
fn warded_taunter() -> CardDef {
    unit_def_of(
        "indestructible-taunt",
        906,
        4,
        4,
        json!([{ "kind": "Indestructible" }, { "kind": "Taunt" }]),
    )
}
/// Indestructible with a Death trigger, so R69 can show Death firing on a max-health death.
fn warded_pinger() -> CardDef {
    unit_def_of(
        "indestructible-pinger",
        907,
        4,
        4,
        json!([{ "kind": "Indestructible" }]),
    )
}
/// #46 Suppressive Aura, trimmed: enemy units get -4 max health (§10.4 layer 5).
fn suppressor() -> CardDef {
    unit_def_of("suppressor", 908, 1, 5, json!([]))
}
/// What a Death trigger summons; a plain unit, so R11's token test stays about the token.
fn spawn() -> CardDef {
    unit_def_of("spawn", 909, 1, 1, json!([]))
}
/// #12 Big Felinor, trimmed: one effect, nine damage to every unit on the board.
fn mass_smite() -> CardDef {
    spell_def_of("mass-smite", 910)
}

/// A backrow Field Spell, for §4.5 step 1's "and backrow cards marked destroyed".
fn field_spell() -> CardDef {
    CardDef {
        type_: CardType::FieldSpell,
        ..spell_def_of("field-spell", 911)
    }
}
/// #98: an Indestructible Field Spell, which R46 says simply stays when marked.
fn warded_field_spell() -> CardDef {
    let face: CardFace = json_as(json!({
        "keywords": [{ "kind": "Indestructible" }],
        "text": "indestructible field spell",
    }));
    CardDef {
        type_: CardType::FieldSpell,
        base: face.clone(),
        radiant: face,
        ..spell_def_of("warded-field-spell", 912)
    }
}
/// #22 Carnivorous Cube, trimmed: its Death trigger reads what it remembered while on the field.
fn rememberer() -> CardDef {
    unit_def_of("rememberer", 913, 2, 2, json!([]))
}
/// A unit that replaces itself on death, so the state check can never come to rest (§4.5 step 5).
fn endless() -> CardDef {
    unit_def_of("endless", 914, 1, 1, json!([]))
}

fn rush_token() -> CardDef {
    token_def("rush", [Tag::Token])
}

fn defs() -> Vec<CardDef> {
    vec![
        pinger(),
        reborn_summoner(),
        reborn_crier(),
        reborn_plain(),
        locker(),
        warded_taunter(),
        warded_pinger(),
        suppressor(),
        spawn(),
        mass_smite(),
        field_spell(),
        warded_field_spell(),
        rememberer(),
    ]
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn ping_enemy_hero() -> Script {
    Script {
        death: Some(hook(|_ctx| {
            vec![effects::damage(json_as(
                json!({ "to": { "of": "enemyHero" }, "amount": 1 }),
            ))]
        })),
        ..Script::default()
    }
}

fn scripts() -> Vec<(String, CardScripts)> {
    vec![
        (pinger().id, both(ping_enemy_hero())),
        (warded_pinger().id, both(ping_enemy_hero())),
        (
            reborn_summoner().id,
            both(Script {
                death: Some(hook(|_ctx| vec![summon_first_free(spawn().id)])),
                ..Script::default()
            }),
        ),
        (
            reborn_crier().id,
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::damage(json_as(
                        json!({ "to": { "of": "enemyHero" }, "amount": 5 }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            locker().id,
            both(Script {
                death: Some(hook(|_ctx| vec![lock_own_first_lane()])),
                ..Script::default()
            }),
        ),
        (
            suppressor().id,
            both(Script {
                aura: Some(aura_hook(|args| {
                    vec![AuraEntry {
                        applies: Box::new(move |unit: &CardInstance| {
                            unit.controller != args.self_.controller
                        }),
                        mod_: StatMod {
                            max_health: Some(-4),
                            ..StatMod::default()
                        },
                    }]
                })),
                ..Script::default()
            }),
        ),
        (
            mass_smite().id,
            both(Script {
                cry: Some(hook(|_ctx| vec![smite_every_unit(9)])),
                ..Script::default()
            }),
        ),
        // R78: the Death hook reads the meal this card remembered before it left the field.
        (
            rememberer().id,
            both(Script {
                death: Some(hook(|ctx| {
                    let eaten = ctx
                        .self_
                        .as_ref()
                        .and_then(|card| card.memory.get("eaten"))
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    match eaten {
                        Some(eaten) => vec![summon_first_free(eaten)],
                        None => vec![],
                    }
                })),
                ..Script::default()
            }),
        ),
    ]
}

/// A fresh game whose catalog and script registry also carry this file's fixtures.
fn game(seed: &str) -> GameState {
    let state = new_game(seed, None);
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
    state
}

/// Enough damage to put a unit at 0 health, read through the layers (§10.4).
fn lethal_damage(state: &mut GameState, id: &str) {
    let unit = find_instance(state, id).unwrap_or_else(|| panic!("no instance {id}"));
    let max_health = unit_view(state, unit).max_health;
    card_mut(state, id).damage = max_health;
}

fn hero_pings(events: &[GameEvent]) -> Vec<Option<String>> {
    events_of_type(events, GameEventType::Damage)
        .iter()
        .filter_map(|event| match event {
            GameEvent::Damage {
                source_id, target_id, ..
            } if target_id.starts_with("hero-") => Some(source_id.clone()),
            _ => None,
        })
        .collect()
}

mod the_state_check_m2_t5 {
    use super::*;

    #[test]
    fn r68_one_effect_kills_six_units_in_one_check_and_fires_six_death_triggers_active_side_first_then_lane_order()
     {
        // Placed out of lane order on purpose: R68 orders by lane, not by creation.
        let placements = [
            (PlayerId::P1, 3),
            (PlayerId::P1, 1),
            (PlayerId::P1, 5),
            (PlayerId::P2, 4),
            (PlayerId::P2, 5),
            (PlayerId::P2, 2),
        ];

        for active in PLAYER_IDS {
            let mut state = game(&format!("six-deaths-{active}"));
            state.turn = 4;
            state.active = active;

            let mut placed: Vec<(PlayerId, i32, String)> = Vec::new();
            for (player, lane) in placements {
                let id = put(
                    &mut state,
                    &pinger().id,
                    slot(player, Row::Units, lane),
                    json!({}),
                )
                .id;
                placed.push((player, lane, id));
            }
            let sides = if active == PlayerId::P1 {
                [PlayerId::P1, PlayerId::P2]
            } else {
                [PlayerId::P2, PlayerId::P1]
            };
            let mut expected: Vec<Option<String>> = Vec::new();
            for player in sides {
                let mut mine: Vec<&(PlayerId, i32, String)> =
                    placed.iter().filter(|entry| entry.0 == player).collect();
                mine.sort_by_key(|a| a.1);
                expected.extend(mine.into_iter().map(|entry| Some(entry.2.clone())));
            }

            let mut sink = sink_for(&state);
            let spell = new_instance(
                &mut state,
                &mass_smite().id,
                PlayerId::P1,
                Zone::Resolving { player: PlayerId::P1 },
            );

            run_hook(
                &mut sink.on(&mut state),
                &spell,
                HookName::Cry,
                HookOptions::default(),
            );
            // §4.5: the whole effect lands first; nothing has died yet.
            assert_eq!(events_of_type(&sink.events, GameEventType::Destroyed).len(), 0);

            state_check(&mut sink.on(&mut state));

            assert_eq!(events_of_type(&sink.events, GameEventType::Destroyed).len(), 6);
            assert_eq!(state.counters.destroyed, 6);
            assert_eq!(hero_pings(&sink.events), expected);
            assert_eq!(active_units_of(&state, PlayerId::P1).len(), 0);
            assert_eq!(active_units_of(&state, PlayerId::P2).len(), 0);
            assert_eq!(state.players.p1.graveyard.len(), 3);
            assert_eq!(state.players.p2.graveyard.len(), 3);
        }
    }

    #[test]
    fn r8_r64_a_reborn_unit_reserves_its_zone_a_death_trigger_summon_lands_elsewhere_and_death_fires_on_both_deaths()
     {
        let mut state = game("reborn-reserved");
        state.turn = 2;
        state.active = PlayerId::P1;
        let reborner = put(
            &mut state,
            &reborn_summoner().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        lethal_damage(&mut state, &reborner.id);

        let mut sink = sink_for(&state);
        state_check(&mut sink.on(&mut state));
        let events = sink.events;

        // Its own Death trigger summoned while lane 1 was reserved, so the spawn took lane 2 (R64).
        assert_eq!(
            card_at(&state, slot(PlayerId::P1, Row::Units, 2)).map(|c| c.def_id.clone()),
            Some(spawn().id)
        );
        // And the unit itself came back to its own zone at 1 health without Reborn.
        assert_eq!(
            card_at(&state, slot(PlayerId::P1, Row::Units, 1)).map(|c| c.id.clone()),
            Some(reborner.id.clone())
        );
        let view = unit_view(&state, card(&state, &reborner.id));
        assert_eq!(view.max_health, 2);
        assert_eq!(view.health, 1);
        assert!(!unit_has(&state, card(&state, &reborner.id), KeywordKind::Reborn));
        assert_eq!(card(&state, &reborner.id).reborn_spent, Some(true));
        assert_eq!(state.reserved.len(), 0);
        assert_eq!(state.players.p1.graveyard.len(), 0);
        // A Reborn unit only passes through the graveyard, so it reports no stay there (§4.5 step 4).
        assert_eq!(events_of_type(&events, GameEventType::EnteredGraveyard).len(), 0);
        assert_eq!(
            instance_ids(&events, GameEventType::Destroyed),
            vec![reborner.id.clone()]
        );

        // R8: the second death fires Death again, and this time the unit stays dead.
        card_mut(&mut state, &reborner.id).damage += 1;
        let mut again = sink_for(&state);
        state_check(&mut again.on(&mut state));
        let again = again.events;

        assert_eq!(
            instance_ids(&again, GameEventType::Destroyed),
            vec![reborner.id.clone()]
        );
        assert_eq!(state.counters.destroyed, 2);
        // Nothing reserved the zone this time, so the second spawn took the leftmost free lane 1.
        assert_eq!(
            card_at(&state, slot(PlayerId::P1, Row::Units, 1)).map(|c| c.def_id.clone()),
            Some(spawn().id)
        );
        assert_eq!(ids(&state.players.p1.graveyard), vec![reborner.id.clone()]);
        assert_eq!(
            instance_ids(&again, GameEventType::EnteredGraveyard),
            vec![reborner.id.clone()]
        );
    }

    #[test]
    fn r688_reborn_into_a_zone_that_was_locked_meanwhile_still_returns_the_return_is_no_play() {
        let mut state = game("reborn-locked");
        state.turn = 2;
        state.active = PlayerId::P1;
        let reborner = put(
            &mut state,
            &reborn_plain().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let lane2 = put(
            &mut state,
            &locker().id,
            slot(PlayerId::P1, Row::Units, 2),
            json!({}),
        );
        lethal_damage(&mut state, &reborner.id);
        lethal_damage(&mut state, &lane2.id);

        let mut sink = sink_for(&state);
        state_check(&mut sink.on(&mut state));
        let events = sink.events;

        // The locker's Death trigger fired after the Reborn unit was collected, so the zone it had
        // reserved is Locked by the time step 4 puts it back — and the body stands there anyway.
        assert!(is_locked(&state, slot(PlayerId::P1, Row::Units, 1)));
        assert_eq!(
            card_at(&state, slot(PlayerId::P1, Row::Units, 1)).map(|c| c.id.clone()),
            Some(reborner.id.clone())
        );
        assert!(!ids(&state.players.p1.graveyard).contains(&reborner.id));
        assert_eq!(state.reserved.len(), 0);
        assert_eq!(events_of_type(&events, GameEventType::Summoned).len(), 1);
        assert_eq!(events_of_type(&events, GameEventType::Destroyed).len(), 2);
    }

    #[test]
    fn r46_an_indestructible_unit_destroyed_by_an_effect_is_in_attack_position_with_no_taunt_until_end_of_turn()
     {
        let mut state = game("indestructible-mark");
        state.turn = 3;
        state.active = PlayerId::P1;
        let warded = put(
            &mut state,
            &warded_taunter().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        card_mut(&mut state, &warded.id).position = Some(Position::Def);
        // R347: its printed Taunt, and Defense Position's, give way to its Indestructible.
        assert!(!unit_has(&state, card(&state, &warded.id), KeywordKind::Taunt));

        card_mut(&mut state, &warded.id).marked_destroyed = Some(true);
        let mut sink = sink_for(&state);
        state_check(&mut sink.on(&mut state));
        let events = sink.events;

        assert_eq!(
            card_at(&state, slot(PlayerId::P1, Row::Units, 1)).map(|c| c.id.clone()),
            Some(warded.id.clone())
        );
        assert_ne!(card(&state, &warded.id).marked_destroyed, Some(true));
        assert_eq!(
            unit_view(&state, card(&state, &warded.id)).position,
            Position::Atk
        );
        assert!(!unit_has(&state, card(&state, &warded.id), KeywordKind::Taunt));
        assert_eq!(card(&state, &warded.id).taunt_suppressed_turn, Some(3));
        assert_eq!(
            to_json(&events_of_type(&events, GameEventType::PositionSwitched)),
            json!([{ "type": "positionSwitched", "instanceId": warded.id, "position": "ATK" }])
        );
        assert_eq!(events_of_type(&events, GameEventType::Destroyed).len(), 0);
        assert_eq!(state.counters.destroyed, 0);
        assert_eq!(state.players.p1.graveyard.len(), 0);

        // R347 before and after: an Indestructible unit has no Taunt on any turn, so nothing is lost
        // and no `keywordGranted … lost` is reported (R46).
        assert!(events_of_type(&events, GameEventType::KeywordGranted).is_empty());
        state.turn = 4;
        assert!(!unit_has(&state, card(&state, &warded.id), KeywordKind::Taunt));
    }

    #[test]
    fn r69_an_indestructible_unit_at_0_max_health_dies_and_counts_as_destroyed_one_at_0_health_with_max_health_above_0_stays()
     {
        let mut state = game("indestructible-max-health");
        state.turn = 2;
        state.active = PlayerId::P1;
        // Suppressive Aura on p2's side: p1's units get -4 max health.
        put(
            &mut state,
            &suppressor().id,
            slot(PlayerId::P2, Row::Units, 5),
            json!({}),
        );
        let dying = put(
            &mut state,
            &warded_pinger().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let survivor = put(
            &mut state,
            &warded_pinger().id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        // Damage taken before it became Indestructible: health is negative, max health is not.
        card_mut(&mut state, &survivor.id).damage = 10;

        assert_eq!(unit_view(&state, card(&state, &dying.id)).max_health, 0);
        let survivor_view = unit_view(&state, card(&state, &survivor.id));
        assert_eq!(survivor_view.max_health, 4);
        assert_eq!(survivor_view.health, -6);

        let mut sink = sink_for(&state);
        state_check(&mut sink.on(&mut state));
        let events = sink.events;

        assert_eq!(
            instance_ids(&events, GameEventType::Destroyed),
            vec![dying.id.clone()]
        );
        assert_eq!(state.counters.destroyed, 1);
        assert_eq!(ids(&state.players.p1.graveyard), vec![dying.id.clone()]);
        // No destroy effect was involved, so Death fires like any other death.
        assert_eq!(hero_pings(&events), vec![Some(dying.id.clone())]);
        let targets: Vec<String> = events_of_type(&events, GameEventType::Damage)
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { target_id, .. } => Some(target_id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(targets, vec!["hero-p2".to_string()]);

        assert_eq!(
            card_at(&state, slot(PlayerId::P2, Row::Units, 1)).map(|c| c.id.clone()),
            Some(survivor.id.clone())
        );
        assert_eq!(unit_view(&state, card(&state, &survivor.id)).health, -6);
    }

    #[test]
    fn r59_the_check_runs_after_a_whole_effect_not_between_its_hits() {
        let mut state = game("r59-whole-effect");
        let begun = begin_game(&state);
        state = begun.state;
        for player in PLAYER_IDS {
            let keep: Vec<String> = ids(&state.players[player].hand);
            let answered = reduce(
                &state,
                &json_as::<Action>(json!({
                    "type": "mulligan",
                    "keep": keep,
                    "playerId": player,
                    "nonce": format!("m-{player}"),
                })),
            );
            assert_eq!(answered.error, None);
            state = answered.state;
        }

        let first = put(
            &mut state,
            &pinger().id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        let second = put(
            &mut state,
            &pinger().id,
            slot(PlayerId::P2, Row::Units, 2),
            json!({}),
        );
        let spell = new_instance(
            &mut state,
            &mass_smite().id,
            PlayerId::P1,
            Zone::Hand { player: PlayerId::P1 },
        );
        state.players.p1.hand.push(spell.clone());

        let result = reduce(
            &state,
            &json_as::<Action>(
                json!({ "type": "play", "instanceId": spell.id, "playerId": "p1", "nonce": "r59" }),
            ),
        );
        assert_eq!(result.error, None);

        let types: Vec<GameEventType> = result.events.iter().map(GameEvent::event_type).collect();
        let hits: Vec<i64> = result
            .events
            .iter()
            .enumerate()
            .filter_map(|(index, event)| match event {
                GameEvent::Damage {
                    source_id: Some(source),
                    ..
                } if *source == spell.id => Some(index as i64),
                _ => None,
            })
            .collect();
        let first_destroyed: i64 = types
            .iter()
            .position(|kind| *kind == GameEventType::Destroyed)
            .map_or(-1, |index| index as i64);

        // Both hits of the one effect land before anything is collected.
        assert_eq!(hits.len(), 2);
        assert!(first_destroyed > hits.iter().copied().max().unwrap_or(i64::MIN));
        // And both Death triggers fire after the collection, not between the hits.
        let pings: Vec<i64> = result
            .events
            .iter()
            .enumerate()
            .filter_map(|(index, event)| match event {
                GameEvent::Damage { target_id, .. } if target_id == "hero-p1" => Some(index as i64),
                _ => None,
            })
            .collect();
        assert_eq!(pings.len(), 2);
        assert!(pings.iter().copied().min().unwrap_or(i64::MAX) > first_destroyed);
        assert_eq!(
            instance_ids(&result.events, GameEventType::Destroyed),
            vec![first.id.clone(), second.id.clone()]
        );
        assert_eq!(
            find_instance(&result.state, &first.id).map(|c| c.zone.z()),
            Some(ZoneName::Graveyard)
        );
        assert_eq!(
            find_instance(&result.state, &second.id).map(|c| c.zone.z()),
            Some(ZoneName::Graveyard)
        );
    }

    #[test]
    fn r78_a_reborn_unit_returns_reset_at_1_health_without_reborn_and_its_cry_does_not_fire() {
        let mut state = game("reborn-cry");
        state.turn = 2;
        state.active = PlayerId::P1;
        let crier = put(
            &mut state,
            &reborn_crier().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        {
            let live = card_mut(&mut state, &crier.id);
            live.buffs = AttackHealth { attack: 3, health: 3 };
            live.granted_keywords = vec![Keyword::Taunt];
            live.position = Some(Position::Def);
        }
        lethal_damage(&mut state, &crier.id);
        let enemy_hero = state.players.p2.hero.health;

        let mut sink = sink_for(&state);
        state_check(&mut sink.on(&mut state));
        let events = sink.events;

        assert_eq!(
            card_at(&state, slot(PlayerId::P1, Row::Units, 1)).map(|c| c.id.clone()),
            Some(crier.id.clone())
        );
        let view = unit_view(&state, card(&state, &crier.id));
        assert_eq!(view.attack, 2);
        assert_eq!(view.max_health, 2);
        assert_eq!(view.health, 1);
        assert!(!unit_has(&state, card(&state, &crier.id), KeywordKind::Reborn));
        assert!(!unit_has(&state, card(&state, &crier.id), KeywordKind::Taunt));
        assert_eq!(
            card(&state, &crier.id).buffs,
            AttackHealth { attack: 0, health: 0 }
        );
        assert_eq!(card(&state, &crier.id).position, Some(Position::Atk));
        // The Cry would have hit the enemy hero for 5; a Reborn return never fires it (R1, §4.5).
        assert_eq!(state.players.p2.hero.health, enemy_hero);
        assert_eq!(events_of_type(&events, GameEventType::Damage).len(), 0);
        assert_eq!(
            instance_ids(&events, GameEventType::Summoned),
            vec![crier.id.clone()]
        );
    }

    #[test]
    fn r11_a_destroyed_unit_token_vanishes_instead_of_entering_a_graveyard() {
        let mut state = game("token-vanishes");
        state.turn = 2;
        state.active = PlayerId::P1;
        let token = put(
            &mut state,
            &rush_token().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        lethal_damage(&mut state, &token.id);

        let mut sink = sink_for(&state);
        state_check(&mut sink.on(&mut state));
        let events = sink.events;

        assert!(card_at(&state, slot(PlayerId::P1, Row::Units, 1)).is_none());
        assert_eq!(
            instance_ids(&events, GameEventType::Destroyed),
            vec![token.id.clone()]
        );
        assert_eq!(state.counters.destroyed, 1);
        assert_eq!(state.players.p1.graveyard.len(), 0);
        assert_eq!(state.players.p1.exile.len(), 0);
        assert_eq!(events_of_type(&events, GameEventType::EnteredGraveyard).len(), 0);
        assert!(find_instance(&state, &token.id).is_none());
    }

    #[test]
    fn r59_both_heroes_at_0_in_one_check_is_a_draw_one_hero_at_0_is_a_loss() {
        let mut drawn = game("both-heroes");
        drawn.turn = 5;
        drawn.players.p1.hero.health = 0;
        drawn.players.p2.hero.health = -3;
        let mut draw_sink = sink_for(&drawn);
        state_check(&mut draw_sink.on(&mut drawn));
        let draw_events = draw_sink.events;

        assert_eq!(
            drawn.result,
            Some(GameResult {
                winner: Winner::Draw,
                reason: GameOverReason::BothHeroesDead,
            })
        );
        assert_eq!(drawn.phase, Phase::Over);
        assert_eq!(
            to_json(&events_of_type(&draw_events, GameEventType::GameOver)),
            json!([{ "type": "gameOver", "winner": "draw", "reason": "both-heroes-dead" }])
        );

        let mut lost = game("one-hero");
        lost.turn = 5;
        lost.players.p2.hero.health = 0;
        let mut loss_sink = sink_for(&lost);
        state_check(&mut loss_sink.on(&mut lost));
        let loss_events = loss_sink.events;

        assert_eq!(
            lost.result,
            Some(GameResult {
                winner: Winner::P1,
                reason: GameOverReason::HeroDeath,
            })
        );
        assert_eq!(lost.phase, Phase::Over);
        assert_eq!(
            to_json(&events_of_type(&loss_events, GameEventType::GameOver)),
            json!([{ "type": "gameOver", "winner": "p1", "reason": "hero-death" }])
        );
    }

    #[test]
    fn s4_5_step_1_collects_a_backrow_card_an_effect_marked_destroyed() {
        let mut state = game("backrow-destroy");
        let placed = put(
            &mut state,
            &field_spell().id,
            slot(PlayerId::P1, Row::Backrow, 2),
            json!({}),
        );
        card_mut(&mut state, &placed.id).marked_destroyed = Some(true);

        let mut sink = sink_for(&state);
        state_check(&mut sink.on(&mut state));
        let events = sink.events;

        assert!(card_at(&state, slot(PlayerId::P1, Row::Backrow, 2)).is_none());
        assert_eq!(ids(&state.players.p1.graveyard), vec![placed.id.clone()]);
        assert_eq!(
            instance_ids(&events, GameEventType::Destroyed),
            vec![placed.id.clone()]
        );
        assert_eq!(
            instance_ids(&events, GameEventType::EnteredGraveyard),
            vec![placed.id.clone()]
        );
        assert_eq!(state.counters.destroyed, 1);
    }

    #[test]
    fn r46_leaves_an_indestructible_field_spell_where_it_is_and_drops_the_mark() {
        let mut state = game("backrow-indestructible");
        let placed = put(
            &mut state,
            &warded_field_spell().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        card_mut(&mut state, &placed.id).marked_destroyed = Some(true);

        let mut sink = sink_for(&state);
        state_check(&mut sink.on(&mut state));
        let events = sink.events;

        assert_eq!(
            card_at(&state, slot(PlayerId::P1, Row::Backrow, 1)).map(|c| c.id.clone()),
            Some(placed.id.clone())
        );
        assert_eq!(card(&state, &placed.id).marked_destroyed, Some(false));
        assert!(events_of_type(&events, GameEventType::Destroyed).is_empty());
        // a Field Spell has no position
        assert!(events_of_type(&events, GameEventType::PositionSwitched).is_empty());
        assert_eq!(state.counters.destroyed, 0);
    }

    #[test]
    fn r78_a_death_trigger_reads_what_the_unit_remembered_before_it_left_the_field() {
        let mut state = game("death-last-known");
        let eater = put(
            &mut state,
            &rememberer().id,
            slot(PlayerId::P1, Row::Units, 2),
            json!({}),
        );
        {
            let live = card_mut(&mut state, &eater.id);
            live.memory.insert("eaten".into(), json!(spawn().id));
            live.marked_destroyed = Some(true);
        }

        let mut sink = sink_for(&state);
        state_check(&mut sink.on(&mut state));
        let events = sink.events;

        // R78 wipes the instance's memory on the way out, so the hook must read the snapshot.
        assert!(card(&state, &eater.id).memory.is_empty());
        assert_eq!(ids(&state.players.p1.graveyard), vec![eater.id.clone()]);
        let summoned: Vec<String> = events_of_type(&events, GameEventType::Summoned)
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { def_id, .. } => Some(def_id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(summoned.len(), 1);
        assert_eq!(summoned.first().cloned(), Some(spawn().id));
        assert_eq!(
            card_at(&state, slot(PlayerId::P1, Row::Units, 1)).map(|c| c.def_id.clone()),
            Some(spawn().id)
        );
    }

    #[test]
    fn r688_the_event_stream_shows_the_reborn_return_into_a_locked_zone_as_a_summon_not_a_graveyard_stay() {
        let mut state = game("reborn-fizzle-event");
        let unit = put(
            &mut state,
            &reborn_plain().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let locker_unit = put(
            &mut state,
            &locker().id,
            slot(PlayerId::P1, Row::Units, 3),
            json!({}),
        );
        card_mut(&mut state, &unit.id).marked_destroyed = Some(true);
        card_mut(&mut state, &locker_unit.id).marked_destroyed = Some(true);

        let mut sink = sink_for(&state);
        state_check(&mut sink.on(&mut state));
        let events = sink.events;

        assert!(is_locked(&state, slot(PlayerId::P1, Row::Units, 1)));
        assert_eq!(
            card_at(&state, slot(PlayerId::P1, Row::Units, 1)).map(|c| c.id.clone()),
            Some(unit.id.clone())
        );
        assert!(!ids(&state.players.p1.graveyard).contains(&unit.id));
        assert!(!instance_ids(&events, GameEventType::EnteredGraveyard).contains(&unit.id));
        assert_eq!(events_of_type(&events, GameEventType::Summoned).len(), 1);
    }
}

mod what_a_death_reports_r89_m3 {
    use super::*;

    #[test]
    fn r89_reports_the_dying_cards_owner_its_last_stats_and_the_unit_that_killed_it() {
        let mut state = game("death-report");
        state.active = PlayerId::P1;
        let killer = put(
            &mut state,
            &pinger().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        // printed 1/1
        let victim = put(
            &mut state,
            &spawn().id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        {
            let live = card_mut(&mut state, &victim.id);
            // a 4/5 body when it dies
            live.buffs = AttackHealth { attack: 3, health: 4 };
            live.granted_keywords = vec![Keyword::Taunt];
        }

        let view = unit_view(&state, card(&state, &victim.id));
        assert_eq!(view.attack, 4);
        assert_eq!(view.max_health, 5);

        let mut sink = sink_for(&state);
        let source = card(&state, &killer.id).clone();
        let target = card(&state, &victim.id).clone();
        deal_damage(
            &mut sink.on(&mut state),
            DamageArgs {
                source: Some(source),
                target: DamageTarget::Unit { instance: target },
                amount: 5,
                flags: None,
            },
        );
        state_check(&mut sink.on(&mut state));
        let events = sink.events;

        assert_eq!(
            to_json(&events_of_type(&events, GameEventType::Destroyed)),
            json!([
                {
                    "type": "destroyed",
                    "instanceId": victim.id,
                    "defId": spawn().id,
                    "owner": "p2",
                    "controller": "p2",
                    // R89: the stats the layers computed at the moment it died, not the printed 1/1.
                    "attack": 4,
                    "maxHealth": 5,
                    "killerId": killer.id,
                },
            ])
        );

        // R78 has wiped the instance by now, which is why the event has to carry it.
        assert_eq!(
            card(&state, &victim.id).buffs,
            AttackHealth { attack: 0, health: 0 }
        );
        assert_eq!(card(&state, &victim.id).last_damaged_by, None);
    }

    #[test]
    fn r89_reports_no_killer_when_nothing_dealt_the_lethal_damage() {
        let mut state = game("death-report-no-killer");
        state.active = PlayerId::P1;
        let victim = put(
            &mut state,
            &spawn().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        // destroyed by an effect, not by a unit's damage
        card_mut(&mut state, &victim.id).marked_destroyed = Some(true);

        let mut sink = sink_for(&state);
        state_check(&mut sink.on(&mut state));
        let events = sink.events;

        assert_eq!(
            to_json(&events_of_type(&events, GameEventType::Destroyed)),
            json!([
                {
                    "type": "destroyed",
                    "instanceId": victim.id,
                    "defId": spawn().id,
                    "owner": "p1",
                    "controller": "p1",
                    "attack": 1,
                    "maxHealth": 1,
                    "killerId": null,
                },
            ])
        );
    }
}

mod the_pass_cap_s4_5_step_5 {
    use super::*;

    #[test]
    fn throws_when_a_board_cannot_settle_rather_than_returning_a_half_checked_one() {
        let mut state = game("never-settles");
        state.active = PlayerId::P1;

        // A unit whose Death summons another of the same kind, each arriving already dead: §4.5 step 5
        // would repeat for ever, so the cap is what stops it — loudly (STATE_CHECK_PASS_CAP).
        let mut catalog = registered_catalog().clone();
        catalog.insert(endless().id, endless());
        register_catalog(catalog);
        let endless_script = || Script {
            death: Some(hook(|_ctx| {
                vec![effects::summon(json_as(json!({
                    "defId": endless().id,
                    "statsOverride": { "attack": 0, "health": 0 },
                })))]
            })),
            ..Script::default()
        };
        let mut registry = registered_scripts().clone();
        registry.insert(
            endless().id,
            CardScripts {
                base: endless_script(),
                radiant: endless_script(),
            },
        );
        register_scripts(registry);

        let doomed = put(
            &mut state,
            &endless().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        card_mut(&mut state, &doomed.id).stats_override = Some(AttackHealth { attack: 0, health: 0 });

        let mut sink = sink_for(&state);
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            state_check(&mut sink.on(&mut state));
        }));
        let message = panic_text(caught.expect_err("the state check settled"));
        assert!(
            message.contains("did not settle in 100 passes"),
            "unexpected panic: {message}"
        );
        assert_eq!(STATE_CHECK_PASS_CAP, 100);
    }
}
