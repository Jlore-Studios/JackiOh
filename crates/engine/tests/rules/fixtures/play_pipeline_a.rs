//! Port of `packages/engine/test/fixtures/playPipelineA.ts`.
//!
//! Fixture cards for play pipeline A (docs/classic-sets.md B5 E1, E2, E4, E5, E9; Classic #4, #10, #17,
//! #23, #33, #72, #87, #89; Classic+ #37, #64, #68; AI Refusal, Autocomplete, Scaling Law). The engine
//! never imports `packages/cards` (CLAUDE.md), so each system is proved on a card of its shape here.
//! Ids are `pa-*`, indices from 5000, registered on top of the shared fixture catalog by `withPlayA`.
//!
//! TS numbered the definitions from a module counter (one step per `def`, in `PA`'s key order); each
//! index is written out here.

use std::sync::LazyLock;

use jackioh_engine::effects;
use jackioh_engine::testkit::*;

fn def(index: u32, id: &str, type_: &str, extra: Value) -> CardDef {
    let mut rest = match extra {
        Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    let attack = rest.remove("attack").and_then(|v| v.as_i64());
    let health = rest.remove("health").and_then(|v| v.as_i64());
    let keywords = rest.remove("keywords").unwrap_or_else(|| json!([]));
    let mut base = json!({ "keywords": keywords, "text": id });
    let mut radiant = json!({ "keywords": keywords, "text": format!("{id} radiant") });
    if type_ == "Unit" {
        let (attack, health) = (attack.unwrap_or(2), health.unwrap_or(2));
        if let (Some(base), Some(radiant)) = (base.as_object_mut(), radiant.as_object_mut()) {
            base.insert("attack".to_string(), json!(attack));
            base.insert("health".to_string(), json!(health));
            radiant.insert("attack".to_string(), json!(attack * 2));
            radiant.insert("health".to_string(), json!(health * 2));
        }
    }
    let mut card = json!({
        "id": format!("pa-{id}"),
        "index": index.to_string(),
        "name": format!("{id} (play A)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": base,
        "radiant": radiant,
    });
    if let Some(into) = card.as_object_mut() {
        for (key, value) in rest {
            into.insert(key, value);
        }
    }
    json_as(card)
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn any_target() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "of": ["unit", "hero"] }))]
}

/// TS's `PA` object: one definition per key.
pub struct Pa {
    /// A Spell that deals 2 to a declared target (unit or hero); Radiant deals 4.
    pub bolt: CardDef,
    /// A Spell with no choices that deals 1 to the enemy hero, so its resolution shows.
    pub ping: CardDef,
    /// A Unit whose Cry deals 3 to the enemy hero.
    pub crier: CardDef,
    /// A Unit whose Cry declares a target (not a Spell: Immune to Spells does not stop it).
    pub zapper: CardDef,
    /// A Trap that counters the opponent's play, whatever it is (Classic #72's shape).
    pub counter_trap: CardDef,
    /// A second one, to prove the rest find no card and stay set.
    pub counter_trap2: CardDef,
    /// A Trap that counters the opponent's Spell into exile when it cost 1 or less (Classic #10).
    pub exile_trap: CardDef,
    /// A Trap that steals the opponent's play (Classic #72 Radiant).
    pub steal_trap: CardDef,
    /// A Trap that counters a Spell targeting one of your units (AI Refusal).
    pub refusal: CardDef,
    /// A Field Spell whose aura counters any play that paid 1 (Classic #87's shape, count fixed at 1).
    pub chalice: CardDef,
    /// A Field Spell that asks, on the opponent's Spell, whether to Tribute itself to steal it (Classic #4).
    pub palantir: CardDef,
    /// A Field Spell that makes its controller discard their hand when the opponent announces anything.
    pub shredder: CardDef,
    /// A Trap the opponent sets face-down, and a Field Trap.
    pub hidden_trap: CardDef,
    pub hidden_field_trap: CardDef,
    /// Classic #33 Joro: intercepts the opponent's targeting of your units from your hand.
    pub joro: CardDef,
    /// Classic #89 Paul Allen's Ghost: 2 more cards to target it.
    pub ghost: CardDef,
    /// A Unit Immune to Spells.
    pub immune: CardDef,
    /// A Spell whose Cry asks for a unit target (a card continuation's prompt), then deals 2 to it.
    pub chooser: CardDef,
    /// A Spell with Echo 1 that deals 1 to a declared unit target: its repeat asks the pipeline's own prompt.
    pub echo_bolt: CardDef,
    /// Declarations with the v0.2.0 filter fields.
    pub grave_raiser: CardDef,
    pub cheap_hunter: CardDef,
    pub medic: CardDef,
    pub plague_hunter: CardDef,
    pub lane_hunter: CardDef,
    /// Classic #23 Devil's Pact's modifier, installed by a Spell: plays become Book of Flame.
    pub pact: CardDef,
    pub book: CardDef,
    /// Classic+ #68 Organic Produce: Fruits you play are Radiant.
    pub produce: CardDef,
    pub apple: CardDef,
    pub pear: CardDef,
    /// Classic #57 Echo's record: it records the last Spell played, never itself (R451).
    pub echo_copy: CardDef,
    /// An AI generated card (B8): the last face-up record passes it over.
    pub ai_card: CardDef,
    /// A Field Spell, for plays by type.
    pub field: CardDef,
    /// A Spell that deals 1 to each of two declared units.
    pub twin: CardDef,
    /// A Field Spell that asks its controller a question whenever the opponent announces a play.
    pub watcher: CardDef,
    /// Quickdraw copies (§6.2: they start in the opening hand), for games folded from a log.
    pub q_bolt: CardDef,
    pub q_palantir: CardDef,
}

pub static PA: LazyLock<Pa> = LazyLock::new(|| Pa {
    bolt: def(5001, "bolt", "Spell", json!({ "cost": 1 })),
    ping: def(5002, "ping", "Spell", json!({ "cost": 1 })),
    crier: def(5003, "crier", "Unit", json!({ "cost": 1 })),
    zapper: def(5004, "zapper", "Unit", json!({ "cost": 1 })),
    counter_trap: def(5005, "counter-trap", "Trap", json!({})),
    counter_trap2: def(5006, "counter-trap-2", "Trap", json!({})),
    exile_trap: def(5007, "exile-trap", "Trap", json!({})),
    steal_trap: def(5008, "steal-trap", "Trap", json!({})),
    refusal: def(5009, "refusal", "Trap", json!({})),
    chalice: def(5010, "chalice", "Field Spell", json!({})),
    palantir: def(5011, "palantir", "Field Spell", json!({})),
    shredder: def(5012, "shredder", "Field Spell", json!({})),
    hidden_trap: def(5013, "hidden-trap", "Trap", json!({})),
    hidden_field_trap: def(5014, "hidden-field-trap", "Field Trap", json!({})),
    joro: def(5015, "joro", "Unit", json!({ "attack": 1, "health": 3 })),
    ghost: def(
        5016,
        "ghost",
        "Unit",
        json!({ "cost": 2, "attack": 5, "health": 6 }),
    ),
    immune: def(
        5017,
        "immune",
        "Unit",
        json!({ "keywords": [{ "kind": "Immune to Spells" }] }),
    ),
    chooser: def(5018, "chooser", "Spell", json!({ "cost": 1 })),
    echo_bolt: def(5019, "echo-bolt", "Spell", json!({ "cost": 1 })),
    grave_raiser: def(5020, "grave-raiser", "Spell", json!({ "cost": 1 })),
    cheap_hunter: def(5021, "cheap-hunter", "Spell", json!({ "cost": 1 })),
    medic: def(5022, "medic", "Spell", json!({ "cost": 1 })),
    plague_hunter: def(5023, "plague-hunter", "Spell", json!({ "cost": 1 })),
    lane_hunter: def(5024, "lane-hunter", "Spell", json!({ "cost": 1 })),
    pact: def(5025, "pact", "Spell", json!({ "cost": 0 })),
    book: def(5026, "book", "Spell", json!({ "cost": 1, "tags": ["Book"] })),
    produce: def(5027, "produce", "Field Spell", json!({ "tags": ["Fruit"] })),
    apple: def(5028, "apple", "Spell", json!({ "cost": 1, "tags": ["Fruit"] })),
    pear: def(5029, "pear", "Unit", json!({ "cost": 1, "tags": ["Fruit"] })),
    echo_copy: def(5030, "echo-copy", "Spell", json!({ "cost": 1 })),
    ai_card: def(
        5031,
        "ai-card",
        "Spell",
        json!({ "cost": 0, "tags": ["AI", "Token"], "rarity": "Token", "token": true }),
    ),
    field: def(5032, "field", "Field Spell", json!({})),
    twin: def(5033, "twin", "Spell", json!({ "cost": 1 })),
    watcher: def(5034, "watcher", "Field Spell", json!({})),
    q_bolt: def(5035, "q-bolt", "Spell", json!({ "cost": 1 })),
    q_palantir: def(5036, "q-palantir", "Field Spell", json!({})),
});

/// `Object.values(PA)`, in `PA`'s key order.
pub static PA_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    let pa = &*PA;
    vec![
        pa.bolt.clone(),
        pa.ping.clone(),
        pa.crier.clone(),
        pa.zapper.clone(),
        pa.counter_trap.clone(),
        pa.counter_trap2.clone(),
        pa.exile_trap.clone(),
        pa.steal_trap.clone(),
        pa.refusal.clone(),
        pa.chalice.clone(),
        pa.palantir.clone(),
        pa.shredder.clone(),
        pa.hidden_trap.clone(),
        pa.hidden_field_trap.clone(),
        pa.joro.clone(),
        pa.ghost.clone(),
        pa.immune.clone(),
        pa.chooser.clone(),
        pa.echo_bolt.clone(),
        pa.grave_raiser.clone(),
        pa.cheap_hunter.clone(),
        pa.medic.clone(),
        pa.plague_hunter.clone(),
        pa.lane_hunter.clone(),
        pa.pact.clone(),
        pa.book.clone(),
        pa.produce.clone(),
        pa.apple.clone(),
        pa.pear.clone(),
        pa.echo_copy.clone(),
        pa.ai_card.clone(),
        pa.field.clone(),
        pa.twin.clone(),
        pa.watcher.clone(),
        pa.q_bolt.clone(),
        pa.q_palantir.clone(),
    ]
});

/// `{ of: "instance", instanceId }` aimed at the announced play.
fn counter_instance(instance_id: &str) -> Effect {
    effects::counter_play(json_as(
        json!({ "target": { "of": "instance", "instanceId": instance_id } }),
    ))
}

/// The trigger every counter Trap here shares: an opponent's `cardAnnounced`, with `when` (R99).
fn announced_by_the_opponent(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    matches!(event, GameEvent::CardAnnounced { player, .. } if *player != ctx.controller)
}

/// JS `String(value)` for a data bag entry: a string as itself, `undefined` for a missing key.
fn js_string(value: Option<&Value>) -> String {
    match value {
        None => "undefined".to_string(),
        Some(Value::String(text)) => text.clone(),
        Some(other) => other.to_string(),
    }
}

fn quickdraw_of(script: &Script) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        ..script.clone()
    }
}

fn scripts_table() -> IndexMap<String, CardScripts> {
    let pa = &*PA;
    let mut table: IndexMap<String, CardScripts> = IndexMap::new();
    table.insert(
        pa.bolt.id.clone(),
        CardScripts {
            base: Script {
                targets: any_target(),
                cry: Some(hook(|_ctx| {
                    vec![effects::damage(json_as(
                        json!({ "to": { "of": "chosen" }, "amount": 2 }),
                    ))]
                })),
                ..Script::default()
            },
            radiant: Script {
                targets: any_target(),
                cry: Some(hook(|_ctx| {
                    vec![effects::damage(json_as(
                        json!({ "to": { "of": "chosen" }, "amount": 4 }),
                    ))]
                })),
                ..Script::default()
            },
        },
    );
    table.insert(
        pa.ping.id.clone(),
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::damage(json_as(
                    json!({ "to": { "of": "enemyHero" }, "amount": 1 }),
                ))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        pa.crier.id.clone(),
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::damage(json_as(
                    json!({ "to": { "of": "enemyHero" }, "amount": 3 }),
                ))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        pa.zapper.id.clone(),
        both(Script {
            targets: vec![TargetDecl::target(1, 1, Value::Null)],
            cry: Some(hook(|_ctx| {
                vec![effects::damage(json_as(
                    json!({ "to": { "of": "chosen" }, "amount": 1 }),
                ))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        pa.counter_trap.id.clone(),
        both(Script {
            triggers: vec![
                TriggerDef::new(
                    "counter",
                    &[GameEventType::CardAnnounced],
                    |_ctx, event| match event {
                        GameEvent::CardAnnounced { instance_id, .. } => vec![counter_instance(instance_id)],
                        _ => vec![],
                    },
                )
                .with_when(|ctx, event| announced_by_the_opponent(ctx, event)),
            ],
            ..Script::default()
        }),
    );
    table.insert(
        pa.counter_trap2.id.clone(),
        both(Script {
            triggers: vec![
                TriggerDef::new("counter", &[GameEventType::CardAnnounced], |_ctx, _event| {
                    vec![effects::counter_play(Default::default())]
                })
                .with_when(|ctx, event| announced_by_the_opponent(ctx, event)),
            ],
            ..Script::default()
        }),
    );
    table.insert(
        pa.exile_trap.id.clone(),
        both(Script {
            triggers: vec![
                TriggerDef::new(
                    "exile",
                    &[GameEventType::CardAnnounced],
                    |_ctx, event| match event {
                        GameEvent::CardAnnounced { instance_id, .. } => {
                            vec![effects::counter_play(json_as(json!({
                                "to": "exile",
                                "target": { "of": "instance", "instanceId": instance_id },
                            })))]
                        }
                        _ => vec![],
                    },
                )
                .with_when(|ctx, event| {
                    matches!(
                        event,
                        GameEvent::CardAnnounced { player, cost_paid, .. }
                            if *player != ctx.controller && *cost_paid <= 1
                    )
                }),
            ],
            ..Script::default()
        }),
    );
    table.insert(
        pa.steal_trap.id.clone(),
        both(Script {
            triggers: vec![
                TriggerDef::new("steal", &[GameEventType::CardAnnounced], |_ctx, _event| {
                    vec![effects::counter_play(json_as(json!({ "to": "thief" })))]
                })
                .with_when(|ctx, event| announced_by_the_opponent(ctx, event)),
            ],
            ..Script::default()
        }),
    );
    table.insert(
        pa.refusal.id.clone(),
        both(Script {
            triggers: vec![
                TriggerDef::new("refuse", &[GameEventType::CardAnnounced], |_ctx, _event| {
                    vec![effects::counter_play(Default::default())]
                })
                .with_when(|ctx, event| {
                    let GameEvent::CardAnnounced {
                        player,
                        card_type,
                        targets,
                        ..
                    } = event
                    else {
                        return false;
                    };
                    if *player == ctx.controller || *card_type != CardType::Spell {
                        return false;
                    }
                    let mine: IndexSet<String> = ctx.state.players[ctx.controller]
                        .units
                        .iter()
                        .filter_map(|pile| {
                            pile.as_ref()
                                .map(|pile| pile.first().map(|card| card.id.clone()).unwrap_or_default())
                        })
                        .collect();
                    targets.iter().any(|id| mine.contains(id))
                }),
            ],
            ..Script::default()
        }),
    );
    table.insert(
        pa.chalice.id.clone(),
        both(Script {
            triggers: vec![TriggerDef::new(
                "chalice",
                &[GameEventType::CardAnnounced],
                |_ctx, event| match event {
                    GameEvent::CardAnnounced {
                        cost_paid: 1,
                        instance_id,
                        ..
                    } => {
                        vec![counter_instance(instance_id)]
                    }
                    _ => vec![],
                },
            )],
            ..Script::default()
        }),
    );
    let palantir = Script {
        triggers: vec![TriggerDef::new(
            "palantir",
            &[GameEventType::CardAnnounced],
            |ctx, event| match event {
                GameEvent::CardAnnounced {
                    player,
                    card_type,
                    instance_id,
                    ..
                } if *player != ctx.controller && *card_type == CardType::Spell => {
                    vec![effects::choose_mode(json_as(json!({
                        "options": ["steal", "pass"],
                        "step": "decide",
                        "data": { "stolen": instance_id },
                    })))]
                }
                _ => vec![],
            },
        )],
        resume: IndexMap::from([(
            "decide",
            hook(|ctx| {
                if effects::chosen_options(ctx)
                    .iter()
                    .any(|option| option == "steal")
                {
                    let stolen = js_string(ctx.data.get("stolen"));
                    vec![
                        effects::sacrifice(json_as(json!({ "target": { "of": "self" } }))),
                        effects::counter_play(json_as(json!({
                            "to": "thief",
                            "target": { "of": "instance", "instanceId": stolen },
                        }))),
                    ]
                } else {
                    vec![]
                }
            }),
        )]),
        ..Script::default()
    };
    table.insert(pa.palantir.id.clone(), both(palantir));
    table.insert(
        pa.shredder.id.clone(),
        both(Script {
            triggers: vec![TriggerDef::new(
                "shred",
                &[GameEventType::CardAnnounced],
                |ctx, event| {
                    if announced_by_the_opponent(ctx, event) {
                        vec![effects::discard_hand(json_as(json!({ "player": "enemy" })))]
                    } else {
                        vec![]
                    }
                },
            )],
            ..Script::default()
        }),
    );
    // Its Cry would hit the enemy hero for 5: an interception summons it, so the Cry never fires.
    table.insert(
        pa.joro.id.clone(),
        both(Script {
            replacements: vec![ReplacementDef {
                id: "interpose".to_string(),
                on: ReplacementMoment::Targeted,
                where_: Some(ReplacementWhere::Hand),
                when: None,
                instead: ReplacementInstead {
                    interpose: Some(true),
                    ..ReplacementInstead::default()
                },
                then: None,
                by: None,
            }],
            cry: Some(hook(|_ctx| {
                vec![effects::damage(json_as(
                    json!({ "to": { "of": "enemyHero" }, "amount": 5 }),
                ))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        pa.ghost.id.clone(),
        both(Script {
            targeting_discards: Some(read_hook(|_args| 2)),
            ..Script::default()
        }),
    );
    table.insert(
        pa.chooser.id.clone(),
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::choose_target(json_as(json!({
                    "step": "hit",
                    "scope": { "side": "any", "of": ["unit"] },
                })))]
            })),
            resume: IndexMap::from([(
                "hit",
                hook(|_ctx| {
                    vec![effects::damage(json_as(
                        json!({ "to": { "of": "chosen" }, "amount": 2 }),
                    ))]
                }),
            )]),
            ..Script::default()
        }),
    );
    table.insert(
        pa.echo_bolt.id.clone(),
        both(Script {
            static_flags: Some(StaticFlags {
                echo: Some(1),
                ..StaticFlags::default()
            }),
            targets: vec![TargetDecl::target(1, 1, Value::Null)],
            cry: Some(hook(|_ctx| {
                vec![effects::damage(json_as(
                    json!({ "to": { "of": "chosen" }, "amount": 1 }),
                ))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        pa.grave_raiser.id.clone(),
        both(Script {
            targets: vec![TargetDecl::target(
                1,
                1,
                json!({ "side": "any", "of": ["graveyard"], "type": "Unit" }),
            )],
            cry: Some(hook(|_ctx| vec![])),
            ..Script::default()
        }),
    );
    table.insert(
        pa.cheap_hunter.id.clone(),
        both(Script {
            targets: vec![TargetDecl::target(
                1,
                1,
                json!({ "of": ["unit", "hand", "hero"], "costRange": { "max": 1 } }),
            )],
            cry: Some(hook(|_ctx| vec![])),
            ..Script::default()
        }),
    );
    table.insert(
        pa.medic.id.clone(),
        both(Script {
            targets: vec![TargetDecl::target(
                1,
                1,
                json!({ "of": ["unit", "hero"], "damaged": true }),
            )],
            cry: Some(hook(|_ctx| vec![])),
            ..Script::default()
        }),
    );
    table.insert(
        pa.plague_hunter.id.clone(),
        both(Script {
            targets: vec![TargetDecl::target(
                1,
                1,
                json!({ "of": ["unit", "backrow"], "plague": true }),
            )],
            cry: Some(hook(|_ctx| vec![])),
            ..Script::default()
        }),
    );
    table.insert(
        pa.lane_hunter.id.clone(),
        both(Script {
            targets: vec![TargetDecl::target(
                1,
                1,
                json!({ "of": ["unit", "hero", "zone"], "check": "lane2" }),
            )],
            target_checks: IndexMap::from([(
                "lane2",
                target_check(|args| match args.candidate {
                    Some(candidate) => matches!(candidate.zone, Zone::Field { lane: 2, .. }),
                    None => matches!(args.selection, Selection::Zone { lane: 2, .. }),
                }),
            )]),
            cry: Some(hook(|_ctx| vec![])),
            ..Script::default()
        }),
    );
    let book_id = pa.book.id.clone();
    let radiant_book_id = pa.book.id.clone();
    table.insert(
        pa.pact.id.clone(),
        CardScripts {
            base: Script {
                cry: Some(hook(move |ctx| {
                    vec![effects::add_player_modifier(json_as(json!({
                        "mod": {
                            "kind": "replacePlays",
                            "defId": book_id,
                            "radiant": false,
                            "expiry": { "until": "thisTurn", "turn": ctx.state.turn },
                        },
                    })))]
                })),
                ..Script::default()
            },
            radiant: Script {
                cry: Some(hook(move |ctx| {
                    vec![effects::add_player_modifier(json_as(json!({
                        "mod": {
                            "kind": "replacePlays",
                            "defId": radiant_book_id,
                            "radiant": true,
                            "expiry": { "until": "thisTurn", "turn": ctx.state.turn },
                        },
                    })))]
                })),
                ..Script::default()
            },
        },
    );
    table.insert(
        pa.book.id.clone(),
        CardScripts {
            base: Script {
                targets: any_target(),
                cry: Some(hook(|_ctx| {
                    vec![effects::damage(json_as(
                        json!({ "to": { "of": "chosen" }, "amount": 4 }),
                    ))]
                })),
                ..Script::default()
            },
            radiant: Script {
                targets: any_target(),
                cry: Some(hook(|_ctx| {
                    vec![effects::damage(json_as(
                        json!({ "to": { "of": "chosen" }, "amount": 8 }),
                    ))]
                })),
                ..Script::default()
            },
        },
    );
    table.insert(
        pa.produce.id.clone(),
        both(Script {
            static_flags: Some(json_as(json!({ "radiantPlaysTagged": ["Fruit"] }))),
            ..Script::default()
        }),
    );
    // R214: its Radiant face declares a target its base face does not, so the face step 1 reads shows.
    table.insert(
        pa.apple.id.clone(),
        CardScripts {
            base: Script {
                cry: Some(hook(|_ctx| vec![])),
                ..Script::default()
            },
            radiant: Script {
                targets: any_target(),
                cry: Some(hook(|_ctx| {
                    vec![effects::damage(json_as(
                        json!({ "to": { "of": "chosen" }, "amount": 3 }),
                    ))]
                })),
                ..Script::default()
            },
        },
    );
    table.insert(
        pa.twin.id.clone(),
        both(Script {
            targets: vec![TargetDecl::target(2, 2, Value::Null)],
            cry: Some(hook(|_ctx| {
                vec![
                    effects::damage(json_as(
                        json!({ "to": { "of": "chosen", "index": 0 }, "amount": 1 }),
                    )),
                    effects::damage(json_as(
                        json!({ "to": { "of": "chosen", "index": 1 }, "amount": 1 }),
                    )),
                ]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        pa.watcher.id.clone(),
        both(Script {
            triggers: vec![TriggerDef::new(
                "watch",
                &[GameEventType::CardAnnounced],
                |ctx, event| {
                    if announced_by_the_opponent(ctx, event) {
                        vec![effects::choose_mode(json_as(
                            json!({ "options": ["noted"], "step": "noted" }),
                        ))]
                    } else {
                        vec![]
                    }
                },
            )],
            resume: IndexMap::from([("noted", hook(|_ctx| vec![]))]),
            ..Script::default()
        }),
    );
    table.insert(
        pa.echo_copy.id.clone(),
        both(Script {
            records_play_as: Some(read_hook(|args| query::last_spell_played(args.state))),
            ..Script::default()
        }),
    );
    table.insert(
        pa.ai_card.id.clone(),
        both(Script {
            cry: Some(hook(|_ctx| vec![])),
            ..Script::default()
        }),
    );

    let bolt = table.get(&pa.bolt.id).cloned().unwrap_or_default();
    table.insert(
        pa.q_bolt.id.clone(),
        CardScripts {
            base: quickdraw_of(&bolt.base),
            radiant: quickdraw_of(&bolt.radiant),
        },
    );
    let palantir = table.get(&pa.palantir.id).cloned().unwrap_or_default();
    table.insert(
        pa.q_palantir.id.clone(),
        CardScripts {
            base: quickdraw_of(&palantir.base),
            radiant: quickdraw_of(&palantir.radiant),
        },
    );
    table
}

/// This file's definitions, by id (the brief's `catalog()`).
pub fn catalog() -> CardDefs {
    PA_DEFS
        .iter()
        .map(|card| (card.id.clone(), card.clone()))
        .collect()
}

/// This file's scripts, by id (the brief's `scripts()`).
pub fn scripts() -> IndexMap<String, CardScripts> {
    scripts_table()
}

/// Register this file's defs and scripts on top of whatever is registered now.
pub fn register_play_a() {
    let mut all_defs = registered_catalog().clone();
    all_defs.extend(catalog());
    register_catalog(all_defs);
    let mut all_scripts = registered_scripts().clone();
    all_scripts.extend(scripts());
    register_scripts(all_scripts);
}

/// `registerPlayA` for a game `newGame` has just made (its catalog is registered by then).
pub fn with_play_a(state: GameState) -> GameState {
    register_play_a();
    state
}
