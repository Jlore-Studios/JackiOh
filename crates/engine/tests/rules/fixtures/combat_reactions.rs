//! Test-only cards for Stinkyscape+ E6 (issue #3550, R3550000–R3550009): the set's small combat
//! and reaction verbs — attack an enemy as it is summoned, N attacks a turn and only across, the
//! Frozen trigger and the freeze rider, Activate cooldowns, a per-turn damage pool, counters, a kill
//! deadline, a side snapshot and its restore, a secret pick and take control — each through the
//! engine's verbs and flags, so the engine never imports `crates/cards`. Ids are `e6-` from 35500,
//! clear of every other fixture file's.

use jackioh_engine::effects;
use jackioh_engine::testkit::*;

use super::harness::both;

/// AMBUSHER's attack: Foreshadow's shape is 20/20, the fixture's 2/9.
pub const AMBUSHER_ATTACK: i32 = 2;
/// POOL's pool a turn, and the preview's printed figure.
pub const POOL_SIZE: i32 = 5;
/// COOLER's cooldown in the designer's count (Omura's shape).
pub const COOLER_COOLDOWN: i32 = 3;
/// PIP's Radiant consolation: The Coins on a failed deadline.
pub const PIP_COINS: i32 = 3;

/// A 2/9 Unit that Can't attack, Ranged: "When an enemy unit is summoned attack it and lose 1 mana
/// next turn" (Foreshadow's shape).
pub const AMBUSHER: &str = "e6-ambusher";
/// A 2/3 Unit with no text.
pub const PLAIN: &str = "e6-plain";
/// A 0/30 Unit with no text.
pub const BIG: &str = "e6-big";
/// A 1/9 Unit: "Can attack 3 times per turn. Can only attack the unit directly across from this"
/// (Scorch's shape).
pub const SPRAYER: &str = "e6-sprayer";
/// A 2/6 Unit: "When this is Frozen, fully heal it".
pub const THAWER: &str = "e6-thawer";
/// A 1/6 Unit: "Freeze any Unit damaged by this".
pub const FROSTER: &str = "e6-froster";
/// A Field Spell with a 3-turn-cooldown "zap" (Omura's shape).
pub const COOLER: &str = "e6-cooler";
/// A Field Spell with an Activate dealing 1 to the enemy hero.
pub const ZAPPER: &str = "e6-zapper";
/// A Field Spell: "The first 5 damage allies would take per turn is nullified" (Force Projector's
/// shape).
pub const POOL: &str = "e6-pool";
/// A Field Trap: "The first enemy Ranged attack or Activate per turn is countered" (Segment's
/// shape).
pub const SEGMENT: &str = "e6-segment";
/// A Field Trap: "All enemy Ranged attacks or Activates are countered" (Segment's Radiant face).
pub const SEGMENT_ALL: &str = "e6-segment-all";
/// A 3/3 Unit, Ranged, with no text.
pub const RANGER: &str = "e6-ranger";
/// A Spell: "Put a Unit on a performance enhancement plan..." (PIP's shape).
pub const PIP: &str = "e6-pip";
/// A 5/6 Unit: "Cry: Take a snapshot... Death: Reset your board to that state" (Bogos Binted's
/// shape).
pub const BOGOS: &str = "e6-bogos";
/// A Trap: "Cry: Look at your opponent's hand and choose a card secretly..." (Soul Read's shape).
pub const SOUL_READ: &str = "e6-soul-read";
/// A 2/2 Unit whose Death takes a random enemy permanent (Snom Bunny's shape).
pub const BUNNY: &str = "e6-bunny";
/// A Field Spell with an extra-turn-cooldown "slow": the shipped cooldown, unchanged by E6.
pub const SLOW: &str = "e6-slow";

fn card(index: u32, id: &str, type_: &str, stats: Option<(i32, i32)>, keywords: Value) -> CardDef {
    let (base, radiant) = match stats {
        Some((attack, health)) => (
            json!({ "attack": attack, "health": health, "keywords": keywords, "text": id }),
            json!({ "attack": 2 * attack, "health": 2 * health, "keywords": keywords, "text": id }),
        ),
        None => (
            json!({ "keywords": keywords, "text": id }),
            json!({ "keywords": keywords, "text": id }),
        ),
    };
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": format!("{id} (e6 fixture)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": base,
        "radiant": radiant,
    }))
}

fn flags(value: Value) -> Option<StaticFlags> {
    Some(json_as(value))
}

fn pick(filter: Value) -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, filter)]
}

fn chosen() -> Value {
    json!({ "of": "chosen" })
}

fn instance(id: &str) -> Value {
    json!({ "of": "instance", "instanceId": id })
}

/// AMBUSHER's answer: an enemy arrival, which it attacks at once, losing 1 mana next turn.
fn ambusher() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new(
                "e6-ambush",
                &[GameEventType::Summoned, GameEventType::CardResolved],
                |ctx, event| {
                    let Some(target) = enemy_unit_arrival(ctx.state, ctx.controller, event) else {
                        return Vec::new();
                    };
                    vec![
                        effects::forced_attack(json_as(json!({
                            "attacker": { "of": "self" },
                            "target": instance(&target),
                        }))),
                        effects::next_turn_mana(json_as(json!({ "amount": -1 }))),
                    ]
                },
            )
            .with_when(|ctx, event| enemy_unit_arrival(ctx.state, ctx.controller, event).is_some()),
        ],
        ..Script::default()
    }
}

/// THAWER's answer: every freeze that lands on it heals it to full.
fn thawer() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("e6-thaw", &[GameEventType::Marked], |_ctx, _event| {
                vec![effects::heal(json_as(json!({
                    "target": { "of": "self" },
                    "toFull": true,
                })))]
            })
            .with_when(|ctx, event| {
                ctx.self_
                    .as_ref()
                    .is_some_and(|card| is_frozen_event(event, card.id.as_str()))
            }),
        ],
        ..Script::default()
    }
}

fn ability(id: &str, label: &str, uses: ActivationUses, cost: Option<ActivationCost>) -> ActivationDecl {
    ActivationDecl {
        id: id.to_string(),
        label: label.to_string(),
        uses,
        cost,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(|_| vec![]),
    }
}

/// SLOW's "slow": one use a turn with an extra-turn cooldown (R1841000).
fn slow() -> Script {
    Script {
        activations: vec![ability(
            "slow",
            "Slow",
            ActivationUses::Count(1),
            Some(ActivationCost {
                extra_turn_cooldown: Some(true),
                ..Default::default()
            }),
        )],
        ..Script::default()
    }
}

/// COOLER's "zap": one use a turn with a 3-turn cooldown.
fn cooler() -> Script {
    Script {
        activations: vec![ability(
            "zap",
            "Zap",
            ActivationUses::Count(1),
            Some(ActivationCost {
                cooldown_turns: Some(COOLER_COOLDOWN),
                ..Default::default()
            }),
        )],
        ..Script::default()
    }
}

/// ZAPPER's "zap": one use a turn dealing 1 to the enemy hero.
fn zapper() -> Script {
    Script {
        activations: vec![ActivationDecl {
            id: "zap".to_string(),
            label: "Zap".to_string(),
            uses: ActivationUses::Count(1),
            cost: None,
            targets: vec![],
            modes: vec![],
            can_activate: None,
            has: None,
            run: hook(|_ctx| {
                vec![effects::damage(json_as(json!({
                    "to": { "of": "enemyHero" },
                    "amount": 1,
                })))]
            }),
        }],
        ..Script::default()
    }
}

/// POOL's preview: what its pool has left this turn, after Masochism Unmasked's own.
fn pool_preview() -> PreviewHook {
    condition_hook(|ctx: ConditionContext<'_>| -> Vec<PreviewValue> {
        if ctx.zone != ConditionZone::Field {
            return vec![];
        }
        let Some(left) = damage_pool_left(ctx.state, ctx.self_) else {
            return vec![];
        };
        vec![count_preview(
            &format!("up to {} each turn", param_or(&ctx, "pool", POOL_SIZE)),
            left,
        )]
    })
}

fn pool() -> Script {
    Script {
        static_flags: flags(json!({ "damagePool": POOL_SIZE })),
        preview: Some(pool_preview()),
        ..Script::default()
    }
}

/// PIP's plan: the chosen Unit has until the end of its controller's next turn to get a kill.
fn pip(radiant: bool) -> Script {
    Script {
        targets: pick(json!({ "of": ["unit"] })),
        cry: Some(hook(move |_ctx| {
            let mut args = json!({ "target": chosen() });
            if radiant {
                args["coinsOnFail"] = json!(PIP_COINS);
            }
            vec![effects::kill_deadline(json_as(args))]
        })),
        ..Script::default()
    }
}

/// BOGOS's Cry and Death: the snapshot, then the restore, Radiant rebuilt Radiant.
fn bogos(radiant: bool) -> Script {
    Script {
        cry: Some(hook(|_ctx| vec![effects::snapshot_side()])),
        death: Some(hook(move |_ctx| {
            vec![effects::restore_side_snapshot(json_as(
                json!({ "radiant": radiant }),
            ))]
        })),
        ..Script::default()
    }
}

/// SOUL_READ's answer to an announce: the chosen card is stolen into its owner's hand, another
/// seen card tributes the base trap. The Radiant face answers the chosen card alone.
fn soul_read(radiant: bool) -> Script {
    let trigger = TriggerDef::new("e6-soul-read", &[GameEventType::CardAnnounced], |ctx, event| {
        let Some(card) = ctx.self_.clone() else {
            return Vec::new();
        };
        match secret_pick_hit(&card, event) {
            Some(SecretPickHit::Chosen) => match event {
                GameEvent::CardAnnounced { instance_id, .. } => {
                    vec![effects::counter_play(json_as(json!({
                        "to": "thief",
                        "target": instance(instance_id),
                    })))]
                }
                _ => Vec::new(),
            },
            Some(SecretPickHit::OtherSeen) => {
                vec![effects::sacrifice(json_as(json!({ "target": { "of": "self" } })))]
            }
            None => Vec::new(),
        }
    });
    let trigger = if radiant {
        trigger.with_when(|ctx, event| {
            ctx.self_
                .as_ref()
                .is_some_and(|card| secret_pick_hit(card, event) == Some(SecretPickHit::Chosen))
        })
    } else {
        trigger.with_when(|ctx, event| {
            ctx.self_
                .as_ref()
                .is_some_and(|card| secret_pick_hit(card, event).is_some())
        })
    };
    Script {
        cry: Some(hook(|_ctx| {
            vec![effects::secret_pick(json_as(json!({ "step": "pick" })))]
        })),
        resume: IndexMap::from([("pick", hook(|_ctx| vec![effects::keep_secret_pick()]))]),
        triggers: vec![trigger],
        ..Script::default()
    }
}

/// BUNNY's Death: a random enemy permanent comes under its controller's control.
fn bunny() -> Script {
    Script {
        death: Some(hook(|ctx| {
            let controller = ctx.controller;
            let enemies = enemy_permanents(ctx.state, controller);
            let shuffled = ctx.rng.shuffle(&enemies);
            let Some(first) = shuffled.first() else {
                return Vec::new();
            };
            vec![effects::steal(json_as(json!({ "instanceId": first.id })))]
        })),
        ..Script::default()
    }
}

pub fn scripts() -> IndexMap<String, CardScripts> {
    let entries: Vec<(&str, Script)> = vec![
        (AMBUSHER, ambusher()),
        (
            SPRAYER,
            Script {
                static_flags: flags(json!({ "attacksPerTurn": 3, "attacksOnlyAcross": true })),
                ..Script::default()
            },
        ),
        (THAWER, thawer()),
        (
            FROSTER,
            Script {
                static_flags: flags(json!({ "freezesDamaged": true })),
                ..Script::default()
            },
        ),
        (COOLER, cooler()),
        (SLOW, slow()),
        (ZAPPER, zapper()),
        (POOL, pool()),
        (
            SEGMENT,
            Script {
                static_flags: flags(json!({ "countersRanged": 1 })),
                ..Script::default()
            },
        ),
        (
            SEGMENT_ALL,
            Script {
                static_flags: flags(json!({ "countersRangedAll": true })),
                ..Script::default()
            },
        ),
    ];
    let mut all: IndexMap<String, CardScripts> = entries
        .into_iter()
        .map(|(id, script)| (id.to_string(), both(script)))
        .collect();
    for id in [PLAIN, BIG, RANGER] {
        all.insert(id.to_string(), both(Script::default()));
    }
    all.insert(BUNNY.to_string(), both(bunny()));
    all.insert(
        PIP.to_string(),
        CardScripts {
            base: pip(false),
            radiant: pip(true),
        },
    );
    all.insert(
        BOGOS.to_string(),
        CardScripts {
            base: bogos(false),
            radiant: bogos(true),
        },
    );
    all.insert(
        SOUL_READ.to_string(),
        CardScripts {
            base: soul_read(false),
            radiant: soul_read(true),
        },
    );
    all
}

pub fn defs() -> Vec<CardDef> {
    vec![
        card(
            35500,
            AMBUSHER,
            "Unit",
            Some((AMBUSHER_ATTACK, 9)),
            json!([{ "kind": "Can't attack" }, { "kind": "Ranged" }]),
        ),
        card(35501, PLAIN, "Unit", Some((2, 3)), json!([])),
        card(35502, BIG, "Unit", Some((0, 30)), json!([])),
        card(35503, SPRAYER, "Unit", Some((1, 9)), json!([])),
        card(35504, THAWER, "Unit", Some((2, 6)), json!([])),
        card(35505, FROSTER, "Unit", Some((1, 6)), json!([])),
        card(35506, COOLER, "Field Spell", None, json!([])),
        card(35507, ZAPPER, "Field Spell", None, json!([])),
        card(35508, POOL, "Field Spell", None, json!([])),
        card(35509, SEGMENT, "Field Trap", None, json!([])),
        card(35510, SEGMENT_ALL, "Field Trap", None, json!([])),
        card(35511, RANGER, "Unit", Some((3, 3)), json!([{ "kind": "Ranged" }])),
        card(35512, PIP, "Spell", None, json!([])),
        card(35513, BOGOS, "Unit", Some((5, 6)), json!([])),
        card(35514, SOUL_READ, "Trap", None, json!([])),
        card(35515, BUNNY, "Unit", Some((2, 2)), json!([])),
        card(35516, SLOW, "Field Spell", None, json!([])),
        // PIP's Radiant consolation rides on the shipped The Coin; the fixture catalog holds this
        // stand-in under its id, since fixtures register no shipped defs.
        json_as(json!({
            "id": COIN_DEF_ID,
            "index": "35517",
            "name": "The Coin (e6 fixture)",
            "set": "Core",
            "type": "Spell",
            "tags": [],
            "rarity": "Token",
            "token": true,
            "cost": 0,
            "base": { "keywords": [], "text": COIN_DEF_ID },
            "radiant": { "keywords": [], "text": COIN_DEF_ID },
        })),
    ]
}

/// The engine's fixture catalog (`harness::setup_catalog`) with these cards and scripts on top.
pub fn register_e6_fixtures() {
    super::harness::setup_catalog();
    let mut catalog: CardDefs = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(scripts());
    register_scripts(merged);
}
