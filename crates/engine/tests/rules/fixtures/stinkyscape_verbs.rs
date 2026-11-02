//! Test-only cards for Stinkyscape's verbs (#1849): Immune to Spell Damage (R1849000), Evolve (R1849001),
//! destroy becomes exile (R1849002), reflected damage (R1849003), the own-turn redirect (R1849004), the
//! Gauntlet (R1849005), "Units cannot be in Defense Position" (R1849006), a random forced attacker
//! (R1849007), "when one of your Units is attacked" (R1849008), a trap's reveal chance (R1849009) and
//! (#1875) a Trap answering a combat death before anything moves (R1875006), each through the engine's
//! verbs and flags, so the engine never imports `crates/cards`. Ids are `sv-` from 18490, plus R1884001's
//! two reprieves at 18455 and 18456, so they collide with no other fixture file's.

use jackioh_engine::combat::ally_attacked_of;
use jackioh_engine::effects;
use jackioh_engine::testkit::*;

use super::harness::both;

pub const BOLT_DAMAGE: i32 = 3;
pub const EMBER_DAMAGE: i32 = 2;
pub const ZAP_DAMAGE: i32 = 3;
pub const BURN_DAMAGE: i32 = 15;
pub const SINGE_DAMAGE: i32 = 10;
/// The Masochism fixtures' cap a turn, and the Radiant one's multiplier.
pub const REDIRECT_CAP: i32 = 20;
pub const REDIRECT_MULTIPLIER: i32 = 2;
pub const JEMPIRE_ATTACK: i32 = 2;
pub const ASKER_ATTACK: i32 = 3;
pub const AERELRET_CHANCE: i32 = 30;
pub const AERELRET_DAMAGE: i32 = 1;
pub const AERELRET_LUCKY: i32 = 2;
/// What the reprieve's follow-up gives the Unit it answers for (Stinkyscape #138's shape, R1875006).
pub const REPRIEVE_HEALTH: i32 = 10;

/// A 2/4 Unit with Divine Shield, immune to Spell Damage (Paragon-X's shape).
pub const IMMUNE: &str = "sv-immune";
pub const BOLT: &str = "sv-bolt";
pub const EMBER: &str = "sv-ember";
pub const RALLY: &str = "sv-rally";
pub const ZAP: &str = "sv-zap";
pub const WASP: &str = "sv-wasp";
pub const EVOLVER: &str = "sv-evolver";
pub const DISINTEGREX: &str = "sv-disintegrex";
pub const DISINTEGREX_R: &str = "sv-disintegrex-r";
pub const PHOENIX: &str = "sv-phoenix";
pub const FIELD: &str = "sv-field";
pub const SMASH: &str = "sv-smash";
pub const WIPE: &str = "sv-wipe";
pub const OFFERING: &str = "sv-offering";
pub const PLATING: &str = "sv-plating";
pub const PLATING_R: &str = "sv-plating-r";
/// The Radiant plating's flags on a 2/6 Unit, which can attack.
pub const SPIKED: &str = "sv-spiked";
/// A Field Trap: "Reveals when you take damage on your turn: redirect it to your opponent (up to 20)."
pub const MASOCHISM: &str = "sv-masochism";
/// A Field Trap: the same, "and double it (up to 40)".
pub const MASOCHISM_R: &str = "sv-masochism-r";
pub const BURN: &str = "sv-burn";
pub const SINGE: &str = "sv-singe";
/// A Field Spell: "The middle 3 unit lanes are trapped in a Gauntlet."
pub const GAUNTLET: &str = "sv-gauntlet";
pub const GOAD: &str = "sv-goad";
pub const SHOVE: &str = "sv-shove";
/// A Field Spell: no Unit in Defense Position, and a random Unit of yours attacks each enemy summon.
pub const WARP: &str = "sv-warp";
pub const WARP_II: &str = "sv-warp-ii";
pub const WARDEN: &str = "sv-warden";
pub const JEMPIRE: &str = "sv-jempire";
/// A 1/5 Unit: "When one of your Units is attacked, choose left or right, then give it +3 attack."
pub const ASKER: &str = "sv-asker";
/// A 1/5 Unit: "When one of your Units is attacked, give it +3 attack, then choose left or right."
pub const ASKER_LAST: &str = "sv-asker-last";
/// A Trap: "30% chance to Reveal when an opponent attacks: deal 1 damage to the attacker."
pub const AERELRET: &str = "sv-aerelret";
pub const LUCKY_AERELRET: &str = "sv-lucky-aerelret";
/// A Trap with Aerelret's trigger and no chance: it reveals every time.
pub const SURE_TRAP: &str = "sv-sure-trap";
pub const GRUNT: &str = "sv-grunt";
pub const TAUNTER: &str = "sv-taunter";
pub const FLYER: &str = "sv-flyer";
pub const PLAIN: &str = "sv-plain";
/// A Trap: "Reveals when one of your Units would die in combat: give it +10 Health" (Stinkyscape #138
/// Pocket Peptides' shape, R1875006).
pub const REPRIEVE: &str = "sv-reprieve";
/// R1884001: a Field Spell holding a would-die replacement that is never spent (no Trap fires), whose
/// follow-up casts a bolt at the dying Unit: the cast's own state check would ask it again.
pub const ENDLESS_REPRIEVE: &str = "sv-endless-reprieve";
/// R1884001: the same, whose follow-up casts a bolt its controller aims, so it waits on their pick.
pub const ASKING_REPRIEVE: &str = "sv-asking-reprieve";
/// A Spell: "Cast a new bolt at the enemy hero, and only there" (a bolt aims at Units alone).
pub const AIM_ONLY: &str = "sv-aim-only";
/// A Spell: "Cast a new bolt at the enemy hero", which falls back to its caster's choice.
pub const AIM_FALLBACK: &str = "sv-aim-fallback";

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
        "name": format!("{id} (Stinkyscape verbs fixture)"),
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

/// Aerelret's trigger: an attack an opponent of its controller declared (never a forced one, R121).
fn opponent_attacks(id: &'static str) -> TriggerDef {
    TriggerDef::new(id, &[GameEventType::AttackDeclared], |_ctx, event| {
        let GameEvent::AttackDeclared { attacker_id, .. } = event else {
            return Vec::new();
        };
        vec![effects::damage(json_as(
            json!({ "to": instance(attacker_id), "amount": AERELRET_DAMAGE }),
        ))]
    })
    .with_when(|ctx, event| match event {
        GameEvent::AttackDeclared {
            attacker_id,
            forced: false,
            ..
        } => find_instance(ctx.state, attacker_id)
            .is_some_and(|attacker| attacker.controller != ctx.controller),
        _ => false,
    })
}

/// Masochism's reveal: its controller's hero takes damage on their own turn. Its list is empty: what
/// it does is its static flag, once it is face-up.
fn own_turn_damage(id: &'static str) -> TriggerDef {
    TriggerDef::new(id, &[GameEventType::Damage], |_ctx, _event| Vec::new()).with_when(|ctx, event| {
        matches!(event, GameEvent::Damage { target_id, .. } if *target_id == format!("hero-{}", ctx.controller))
            && ctx.state.active == ctx.controller
    })
}

/// The Warp Disruptors' trigger: an enemy summon, which one random Unit of this side, or all, attack.
fn enemy_summons(id: &'static str, random: bool) -> TriggerDef {
    TriggerDef::new(id, &[GameEventType::Summoned], move |_ctx, event| {
        let GameEvent::Summoned { instance_id, .. } = event else {
            return Vec::new();
        };
        let attackers = if random {
            json!({ "side": "self", "random": true })
        } else {
            json!({ "side": "self" })
        };
        vec![effects::forced_attacks(json_as(
            json!({ "attackers": attackers, "target": { "instanceId": instance_id } }),
        ))]
    })
    .with_when(|ctx, event| matches!(event, GameEvent::Summoned { player, .. } if *player != ctx.controller))
}

fn self_burn(amount: i32) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![effects::damage(json_as(
                json!({ "to": { "of": "selfHero" }, "amount": amount }),
            ))]
        })),
        ..Script::default()
    }
}

fn masochism(redirect: Value) -> Script {
    Script {
        static_flags: flags(json!({ "ownTurnRedirect": redirect })),
        triggers: vec![own_turn_damage("sv-masochism-reveal")],
        ..Script::default()
    }
}

fn warp(random: bool) -> Script {
    Script {
        static_flags: flags(json!({ "unitsNeverDefense": true })),
        triggers: vec![enemy_summons("sv-warp-summoned", random)],
        ..Script::default()
    }
}

/// The reprieve's `wouldDie` replacement: one of its controller's Units the check collected was killed
/// in combat (`DyingUnit.combat`); its follow-up resolves before anything moves (`resolveFirst`).
fn reprieve() -> Script {
    let in_combat = |event: &ReplacedEvent, controller: PlayerId| match event {
        ReplacedEvent::WouldDie { units } => units
            .iter()
            .find(|unit| unit.controller == controller && unit.combat == Some(true))
            .map(|unit| unit.instance_id.clone()),
        _ => None,
    };
    Script {
        replacements: vec![ReplacementDef {
            id: "sv-reprieve".into(),
            on: ReplacementMoment::WouldDie,
            where_: None,
            when: Some(replacement_when(move |rc| {
                in_combat(rc.event, rc.controller).is_some()
            })),
            instead: ReplacementInstead {
                resolve_first: Some(true),
                ..ReplacementInstead::default()
            },
            then: Some("reprieve".into()),
            by: None,
        }],
        resume: IndexMap::from([(
            "reprieve",
            hook(move |ctx| {
                replacement_of(&*ctx)
                    .and_then(|record| in_combat(&record.event, ctx.controller))
                    .map(|id| {
                        vec![effects::buff(json_as(
                            json!({ "target": instance(&id), "health": REPRIEVE_HEALTH }),
                        ))]
                    })
                    .unwrap_or_default()
            }),
        )]),
        ..Script::default()
    }
}

/// R1884001: the reprieve's replacement on a card no firing spends, its follow-up a bolt cast at the
/// Unit that would die, so the cast's own state check meets the same death.
fn endless_reprieve(asking: bool) -> Script {
    let in_combat = |event: &ReplacedEvent, controller: PlayerId| match event {
        ReplacedEvent::WouldDie { units } => units
            .iter()
            .find(|unit| unit.controller == controller && unit.combat == Some(true))
            .map(|unit| unit.instance_id.clone()),
        _ => None,
    };
    Script {
        replacements: vec![ReplacementDef {
            id: if asking { ASKING_REPRIEVE } else { ENDLESS_REPRIEVE }.into(),
            on: ReplacementMoment::WouldDie,
            where_: None,
            when: Some(replacement_when(move |rc| {
                in_combat(rc.event, rc.controller).is_some()
            })),
            instead: ReplacementInstead {
                resolve_first: Some(true),
                ..ReplacementInstead::default()
            },
            then: Some("bolt".into()),
            by: None,
        }],
        resume: IndexMap::from([(
            "bolt",
            hook(move |ctx| {
                replacement_of(&*ctx)
                    .and_then(|record| in_combat(&record.event, ctx.controller))
                    .map(|id| {
                        let args = if asking {
                            json!({ "def": BOLT })
                        } else {
                            json!({ "def": BOLT, "aimAt": id })
                        };
                        vec![effects::cast_new(json_as(args))]
                    })
                    .unwrap_or_default()
            }),
        )]),
        ..Script::default()
    }
}

fn aimed_bolt(only: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            let mut args = json!({ "def": BOLT, "aimAt": "hero:p2" });
            if only {
                args["aimOnly"] = json!(true);
            }
            vec![effects::cast_new(json_as(args))]
        })),
        ..Script::default()
    }
}

fn buff_attacked(attack: i32) -> Effect {
    effects::buff(json_as(json!({ "target": { "of": "chosen" }, "attack": attack })))
}

/// An `allyAttacked` hook's buff of the attacked Unit, named by the facts it was handed.
fn buff_defender(ctx: &EffectContext<'_>, attack: i32) -> Vec<Effect> {
    let Some(facts) = ally_attacked_of(ctx) else {
        return Vec::new();
    };
    vec![effects::buff(json_as(
        json!({ "target": instance(&facts.defender_id), "attack": attack }),
    ))]
}

fn ask() -> Effect {
    effects::choose_mode(json_as(
        json!({ "options": ["left", "right"], "step": "after", "prompt": "asker" }),
    ))
}

fn scripts() -> IndexMap<String, CardScripts> {
    IndexMap::from([
        (
            IMMUNE.to_string(),
            both(Script {
                static_flags: flags(json!({ "immuneToSpellDamage": true })),
                ..Script::default()
            }),
        ),
        (
            BOLT.to_string(),
            both(Script {
                targets: pick(json!({ "of": ["unit"] })),
                cry: Some(hook(|_ctx| {
                    vec![effects::damage(json_as(
                        json!({ "to": chosen(), "amount": BOLT_DAMAGE }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            EMBER.to_string(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::damage_all(json_as(
                        json!({ "amount": EMBER_DAMAGE, "side": "enemy" }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            RALLY.to_string(),
            both(Script {
                targets: pick(json!({ "of": ["unit"] })),
                cry: Some(hook(|_ctx| vec![buff_attacked(1)])),
                ..Script::default()
            }),
        ),
        (
            ZAP.to_string(),
            both(Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::damage(json_as(
                        json!({ "to": { "of": "enemyHero" }, "amount": ZAP_DAMAGE }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            WASP.to_string(),
            both(Script {
                after_attack: Some(hook(|_ctx| {
                    vec![
                        effects::summon_copy(json_as(json!({ "of": { "of": "self" } }))),
                        effects::evolve(json_as(json!({ "target": { "of": "self" } }))),
                    ]
                })),
                evolve: Some(hook(|_ctx| {
                    vec![effects::grant_keyword(json_as(
                        json!({ "target": { "of": "self" }, "keyword": { "kind": "Flying" } }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            EVOLVER.to_string(),
            both(Script {
                targets: pick(json!({ "of": ["unit"] })),
                cry: Some(hook(|_ctx| {
                    vec![effects::evolve(json_as(json!({ "target": chosen() })))]
                })),
                ..Script::default()
            }),
        ),
        (
            DISINTEGREX.to_string(),
            both(Script {
                static_flags: flags(json!({ "destroyedExiled": true })),
                ..Script::default()
            }),
        ),
        (
            DISINTEGREX_R.to_string(),
            both(Script {
                static_flags: flags(json!({ "enemyDestroyedExiled": true })),
                ..Script::default()
            }),
        ),
        (
            PHOENIX.to_string(),
            both(Script {
                death: Some(hook(|_ctx| vec![effects::draw(json_as(json!({ "count": 1 })))])),
                ..Script::default()
            }),
        ),
        (
            SMASH.to_string(),
            both(Script {
                targets: pick(json!({ "of": ["unit", "backrow"] })),
                cry: Some(hook(|_ctx| {
                    vec![effects::destroy(json_as(json!({ "target": chosen() })))]
                })),
                ..Script::default()
            }),
        ),
        (
            WIPE.to_string(),
            both(Script {
                cry: Some(hook(|_ctx| vec![effects::destroy_all(json_as(json!({})))])),
                ..Script::default()
            }),
        ),
        (
            OFFERING.to_string(),
            both(Script {
                targets: pick(json!({ "side": "ally", "of": ["unit"] })),
                cry: Some(hook(|_ctx| {
                    vec![effects::sacrifice(json_as(json!({ "target": chosen() })))]
                })),
                ..Script::default()
            }),
        ),
        (
            PLATING.to_string(),
            both(Script {
                static_flags: flags(json!({ "reflectsDamage": true })),
                ..Script::default()
            }),
        ),
        (
            PLATING_R.to_string(),
            both(Script {
                static_flags: flags(json!({ "reflectsDamage": true, "reflectsToAttacker": true })),
                ..Script::default()
            }),
        ),
        (
            SPIKED.to_string(),
            both(Script {
                static_flags: flags(json!({ "reflectsDamage": true, "reflectsToAttacker": true })),
                ..Script::default()
            }),
        ),
        (
            MASOCHISM.to_string(),
            both(masochism(json!({ "cap": REDIRECT_CAP }))),
        ),
        (
            MASOCHISM_R.to_string(),
            both(masochism(
                json!({ "cap": REDIRECT_CAP, "multiplier": REDIRECT_MULTIPLIER }),
            )),
        ),
        (BURN.to_string(), both(self_burn(BURN_DAMAGE))),
        (SINGE.to_string(), both(self_burn(SINGE_DAMAGE))),
        (
            GAUNTLET.to_string(),
            both(Script {
                static_flags: flags(json!({ "gauntlet": true })),
                ..Script::default()
            }),
        ),
        (
            GOAD.to_string(),
            both(Script {
                targets: pick(json!({ "side": "enemy", "of": ["unit"] })),
                cry: Some(hook(|_ctx| {
                    vec![effects::forced_attacks_on(json_as(
                        json!({ "target": chosen(), "attackers": "self" }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            SHOVE.to_string(),
            both(Script {
                targets: pick(json!({ "of": ["unit"] })),
                cry: Some(hook(|_ctx| {
                    vec![effects::switch_position_of(json_as(
                        json!({ "target": chosen() }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (WARP.to_string(), both(warp(true))),
        (
            WARDEN.to_string(),
            both(Script {
                static_flags: flags(json!({ "unitsNeverDefense": true })),
                ..Script::default()
            }),
        ),
        (WARP_II.to_string(), both(warp(false))),
        (
            JEMPIRE.to_string(),
            both(Script {
                ally_attacked: Some(hook(|ctx| {
                    let taunted = ally_attacked_of(ctx)
                        .and_then(|facts| find_instance(ctx.state, &facts.defender_id))
                        .is_some_and(|defender| unit_has(ctx.state, defender, KeywordKind::Taunt));
                    if !taunted {
                        return Vec::new();
                    }
                    buff_defender(ctx, JEMPIRE_ATTACK)
                })),
                ..Script::default()
            }),
        ),
        (
            ASKER.to_string(),
            both(Script {
                ally_attacked: Some(hook(|ctx| {
                    let mut effects = vec![ask()];
                    effects.extend(buff_defender(ctx, ASKER_ATTACK));
                    effects
                })),
                resume: IndexMap::from([("after", hook(|_ctx| Vec::new()))]),
                ..Script::default()
            }),
        ),
        (
            ASKER_LAST.to_string(),
            both(Script {
                ally_attacked: Some(hook(|ctx| {
                    let mut effects = buff_defender(ctx, ASKER_ATTACK);
                    effects.push(ask());
                    effects
                })),
                resume: IndexMap::from([("after", hook(|_ctx| Vec::new()))]),
                ..Script::default()
            }),
        ),
        (
            AERELRET.to_string(),
            both(Script {
                static_flags: flags(json!({ "revealChance": AERELRET_CHANCE })),
                triggers: vec![opponent_attacks("sv-aerelret-attack")],
                ..Script::default()
            }),
        ),
        (
            LUCKY_AERELRET.to_string(),
            both(Script {
                static_flags: flags(json!({ "revealChance": AERELRET_CHANCE })),
                triggers: vec![opponent_attacks("sv-lucky-aerelret-attack")],
                ..Script::default()
            }),
        ),
        (
            SURE_TRAP.to_string(),
            both(Script {
                triggers: vec![opponent_attacks("sv-sure-trap-attack")],
                ..Script::default()
            }),
        ),
        (REPRIEVE.to_string(), both(reprieve())),
        (ENDLESS_REPRIEVE.to_string(), both(endless_reprieve(false))),
        (ASKING_REPRIEVE.to_string(), both(endless_reprieve(true))),
        (AIM_ONLY.to_string(), both(aimed_bolt(true))),
        (AIM_FALLBACK.to_string(), both(aimed_bolt(false))),
    ])
}

fn defs() -> Vec<CardDef> {
    vec![
        card(
            18490,
            IMMUNE,
            "Unit",
            Some((2, 4)),
            json!([{ "kind": "Divine Shield" }]),
        ),
        card(18491, BOLT, "Spell", None, json!([])),
        card(18492, EMBER, "Field Spell", None, json!([])),
        card(18493, RALLY, "Spell", None, json!([])),
        card(18494, ZAP, "Spell", None, json!([])),
        card(18495, WASP, "Unit", Some((3, 3)), json!([])),
        card(18496, EVOLVER, "Spell", None, json!([])),
        card(18497, DISINTEGREX, "Unit", Some((6, 3)), json!([])),
        card(18498, DISINTEGREX_R, "Unit", Some((6, 3)), json!([])),
        card(
            18499,
            PHOENIX,
            "Unit",
            Some((2, 2)),
            json!([{ "kind": "Reborn" }]),
        ),
        card(18500, FIELD, "Field Spell", None, json!([])),
        card(18501, SMASH, "Spell", None, json!([])),
        card(18502, WIPE, "Spell", None, json!([])),
        card(18503, OFFERING, "Spell", None, json!([])),
        card(18504, PLATING, "Unit", Some((0, 6)), json!([])),
        card(18505, PLATING_R, "Unit", Some((0, 6)), json!([])),
        card(18506, SPIKED, "Unit", Some((2, 6)), json!([])),
        card(18507, MASOCHISM, "Field Trap", None, json!([])),
        card(18508, MASOCHISM_R, "Field Trap", None, json!([])),
        card(18509, BURN, "Spell", None, json!([])),
        card(18510, SINGE, "Spell", None, json!([])),
        card(18511, GAUNTLET, "Field Spell", None, json!([])),
        card(18512, GOAD, "Spell", None, json!([])),
        card(18513, SHOVE, "Spell", None, json!([])),
        card(18514, WARP, "Field Spell", None, json!([])),
        card(18515, WARP_II, "Field Spell", None, json!([])),
        card(18516, JEMPIRE, "Unit", Some((4, 5)), json!([])),
        card(18517, ASKER, "Unit", Some((1, 5)), json!([])),
        card(18518, ASKER_LAST, "Unit", Some((1, 5)), json!([])),
        card(18519, AERELRET, "Trap", None, json!([])),
        card(
            18520,
            LUCKY_AERELRET,
            "Trap",
            None,
            json!([{ "kind": "Lucky", "n": AERELRET_LUCKY }]),
        ),
        card(18521, SURE_TRAP, "Trap", None, json!([])),
        card(18522, GRUNT, "Unit", Some((3, 3)), json!([])),
        card(18523, TAUNTER, "Unit", Some((2, 4)), json!([{ "kind": "Taunt" }])),
        card(18524, FLYER, "Unit", Some((2, 3)), json!([{ "kind": "Flying" }])),
        card(18525, PLAIN, "Unit", Some((1, 1)), json!([])),
        card(18526, WARDEN, "Unit", Some((2, 2)), json!([])),
        card(18527, REPRIEVE, "Trap", None, json!([])),
        card(18528, AIM_ONLY, "Spell", None, json!([])),
        card(18529, AIM_FALLBACK, "Spell", None, json!([])),
        card(18455, ENDLESS_REPRIEVE, "Field Spell", None, json!([])),
        card(18456, ASKING_REPRIEVE, "Field Spell", None, json!([])),
    ]
}

/// The engine's fixture catalog (`harness::setup_catalog`) with these cards and scripts on top.
pub fn register_stinkyscape_verbs() {
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
