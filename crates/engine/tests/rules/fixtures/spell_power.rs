//! Test-only cards for the Meditative+ spell-power systems (issue #1180, docs/meditative-plus-set.md
//! MP5, R1180000–R1180005): Spell Damage from an aura for one or both players and from a "this turn"
//! player modifier, an aura giving its controller's Spells Lifesteal, the healing multiplier, health
//! as the next Spell's price, and the hand-and-deck card that answers a Spell that targeted an enemy.
//! Each reproduces a Meditative+ card's shape through the script hooks, the static flags and the
//! effects library, so the engine is proved without `crates/cards`; the real cards are tested again in
//! their own files. Ids are `sp-*`, indices from 11800 so they collide with nothing (BUILD §0).

use jackioh_engine::effects;
use jackioh_engine::testkit::*;

/// `{ ...defaults, ...extra }`: the keys of `extra` replace the defaults' (TS's object spread).
fn spread(mut base: Value, extra: Value) -> Value {
    if let (Some(fields), Value::Object(over)) = (base.as_object_mut(), extra) {
        fields.extend(over);
    }
    base
}

fn def(index: u32, name: &str, type_: &str, extra: Value) -> CardDef {
    json_as(spread(
        json!({
            "id": format!("sp-{name}"),
            "index": index.to_string(),
            "name": format!("{name} (spell power)"),
            "set": "Core",
            "type": type_,
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 1,
            "base": { "keywords": [], "text": name },
            "radiant": { "keywords": [], "text": name },
        }),
        extra,
    ))
}

fn unit(index: u32, name: &str, attack: i32, health: i32, keywords: Value) -> CardDef {
    def(
        index,
        name,
        "Unit",
        json!({
            "base": { "attack": attack, "health": health, "keywords": keywords.clone(), "text": name },
            "radiant": { "attack": attack * 2, "health": health * 2, "keywords": keywords, "text": name },
        }),
    )
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn flags(value: Value) -> Option<StaticFlags> {
    Some(json_as(value))
}

/// A Field Spell: "Aura: Both players have Spell Damage +2" (Meditative+ #9's shape).
pub const AMP: &str = "sp-amp";
/// A Field Spell: "Aura: You have Spell Damage +3" — an aura for its controller alone.
pub const AMP_OWN: &str = "sp-amp-own";
/// The both-players aura of 4, declaring that number as `spellDamage`, which a Buff or a Nerf moves (R386).
pub const AMP_DECLARED: &str = "sp-amp-declared";
/// A (1) Spell: 1 damage to a declared enemy Unit or the enemy hero.
pub const BOLT: &str = "sp-bolt";
/// A (2) Spell, untargeted: 1 damage to the enemy hero, twice.
pub const ZAP: &str = "sp-zap";
/// A (0) Spell that does nothing.
pub const NOTHING: &str = "sp-nothing";
/// An X Spell: X damage to the enemy hero.
pub const XBOLT: &str = "sp-xbolt";
/// A (1) Field Spell whose Cry gives "Your next Spell costs (1) less" (a `costDiscount` until used):
/// neither a Spell the health price reaches nor one the discount it gives does.
pub const DISCOUNT: &str = "sp-discount";
/// A Field Spell whose Cry deals 1 damage to the enemy hero: a Field Spell's own hit.
pub const FIELD_ZAP: &str = "sp-field-zap";
/// A Trap, face-up on the field, as a hit's source (`make_context` with it as self).
pub const TRAP: &str = "sp-trap";
/// A 1/1 Unit with Spell Damage +1.
pub const WIZARD: &str = "sp-wizard";
/// A 1/1 Unit with "Activate: you have Spell Damage +5 this turn", twice a turn (Meditative+ #11's shape).
pub const POTION: &str = "sp-potion";
/// A 1/2 Unit: "Aura: Your Spells have Lifesteal" (Meditative+ #7's shape).
pub const MAGE: &str = "sp-mage";
/// A 1/2 Unit: "Healing your hero and Units is twice as effective" (Meditative+ #7's Radiant shape).
pub const DOUBLER: &str = "sp-doubler";
/// A (1) Spell: heal a declared Unit or hero 3.
pub const MEND: &str = "sp-mend";
/// A Field Trap, face-up: heals on its controller's enemies become Pierce damage (Classic+ #22's
/// Radiant shape).
pub const MOON: &str = "sp-moon";
/// A (0) Spell: "Your next Spell costs health equal to 3 times its cost instead of mana"
/// (Meditative+ #6's shape).
pub const BLOOM: &str = "sp-bloom";
/// The same, and the Spell it prices has Echo 1 (the Radiant face's).
pub const BLOOM_ECHO: &str = "sp-bloom-echo";
/// A (1) Spell: 2 damage to a declared enemy Unit or hero; in its owner's hand or deck it answers
/// their Spells that targeted an enemy, cast at that enemy (Meditative+ #17's shape).
pub const PILER: &str = "sp-piler";
/// A (1) Spell whose own script asks for an enemy Unit or hero in a `target` prompt, then deals 1.
pub const ASKER: &str = "sp-asker";
/// A (1) Spell: Echo 1, and 1 damage to a declared enemy Unit or hero.
pub const ECHO_BOLT: &str = "sp-echo-bolt";
/// A vanilla 1/9 body.
pub const BODY: &str = "sp-body";
/// A vanilla 1/1.
pub const PAWN: &str = "sp-pawn";

/// Every fixture definition.
pub fn catalog() -> CardDefs {
    [
        def(11800, "amp", "Field Spell", json!({})),
        def(11801, "amp-own", "Field Spell", json!({})),
        def(
            11802,
            "amp-declared",
            "Field Spell",
            json!({
                "params": [{ "key": "spellDamage", "base": 4, "radiant": 4, "better": "up", "step": 1, "min": 1 }],
                "base": { "keywords": [], "text": "Aura: Both players have Spell Damage +{spellDamage}." },
                "radiant": { "keywords": [], "text": "Aura: Both players have Spell Damage +{spellDamage}." },
            }),
        ),
        def(11803, "bolt", "Spell", json!({})),
        def(11804, "zap", "Spell", json!({ "cost": 2 })),
        def(11805, "nothing", "Spell", json!({ "cost": 0 })),
        def(11806, "xbolt", "Spell", json!({ "cost": "X" })),
        def(11807, "discount", "Field Spell", json!({})),
        def(11808, "field-zap", "Field Spell", json!({})),
        def(11809, "trap", "Trap", json!({})),
        unit(11810, "wizard", 1, 1, json!([{ "kind": "Spell Damage", "n": 1 }])),
        unit(11811, "potion", 1, 1, json!([])),
        unit(11812, "mage", 1, 2, json!([])),
        unit(11813, "doubler", 1, 2, json!([])),
        def(11814, "mend", "Spell", json!({})),
        def(
            11815,
            "moon",
            "Field Trap",
            json!({}),
        ),
        def(11816, "bloom", "Spell", json!({ "cost": 0 })),
        def(11817, "bloom-echo", "Spell", json!({ "cost": 0 })),
        def(11818, "piler", "Spell", json!({})),
        def(11819, "asker", "Spell", json!({})),
        def(11820, "echo-bolt", "Spell", json!({})),
        unit(11821, "body", 1, 9, json!([])),
        unit(11822, "pawn", 1, 1, json!([])),
    ]
    .into_iter()
    .map(|card| (card.id.clone(), card))
    .collect()
}

fn hit_enemy_hero(amount: i32) -> Effect {
    effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": amount })))
}

fn hit_chosen(amount: i32) -> Effect {
    effects::damage(json_as(json!({ "to": { "of": "chosen" }, "amount": amount })))
}

fn enemy_unit_or_hero() -> TargetDecl {
    TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit", "hero"] }))
}

/// Meditative+ #17's trigger: the card the engine named on the resolution casts itself, aimed.
fn answer_trigger() -> TriggerDef {
    TriggerDef::new("sp-answer", &[GameEventType::CardResolved], |ctx, event| {
        let GameEvent::CardResolved {
            answer: Some(answer), ..
        } = event
        else {
            return vec![];
        };
        if ctx.self_.as_ref().map(|card| card.id.as_str()) != Some(answer.by.as_str()) {
            return vec![];
        }
        vec![effects::cast(json_as(json!({
            "target": { "of": "self" },
            "aimAt": option_key(&answer.at),
            "aimOnly": true,
        })))]
    })
}

fn health_price(echo: Option<i32>) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![effects::price_next_spell_in_health(effects::HealthPriceArgs {
                multiple: 3,
                echo,
            })]
        })),
        ..Script::default()
    }
}

/// Every fixture script.
pub fn scripts() -> IndexMap<String, CardScripts> {
    let entries: Vec<(&str, Script)> = vec![
        (
            AMP,
            Script {
                static_flags: flags(json!({ "spellDamageAura": { "amount": 2, "both": true } })),
                ..Script::default()
            },
        ),
        (
            AMP_OWN,
            Script {
                static_flags: flags(json!({ "spellDamageAura": { "amount": 3 } })),
                ..Script::default()
            },
        ),
        (
            AMP_DECLARED,
            Script {
                static_flags: flags(json!({ "spellDamageAura": { "amount": 4, "both": true } })),
                ..Script::default()
            },
        ),
        (
            BOLT,
            Script {
                targets: vec![enemy_unit_or_hero()],
                cry: Some(hook(|_ctx| vec![hit_chosen(1)])),
                ..Script::default()
            },
        ),
        (
            ZAP,
            Script {
                cry: Some(hook(|_ctx| vec![hit_enemy_hero(1), hit_enemy_hero(1)])),
                ..Script::default()
            },
        ),
        (NOTHING, Script::default()),
        (
            XBOLT,
            Script {
                cry: Some(hook(|ctx| vec![hit_enemy_hero(ctx.x)])),
                ..Script::default()
            },
        ),
        (
            DISCOUNT,
            Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::add_player_modifier(json_as(json!({
                        "mod": { "expiry": { "until": "used" }, "kind": "costDiscount", "amount": 1, "onlyType": "Spell" },
                    })))]
                })),
                ..Script::default()
            },
        ),
        (
            FIELD_ZAP,
            Script {
                cry: Some(hook(|_ctx| vec![hit_enemy_hero(1)])),
                ..Script::default()
            },
        ),
        (TRAP, Script::default()),
        (WIZARD, Script::default()),
        (
            POTION,
            Script {
                activations: vec![ActivationDecl {
                    id: "potion".to_string(),
                    label: "Spell Damage this turn".to_string(),
                    uses: ActivationUses::Count(2),
                    cost: None,
                    targets: vec![],
                    modes: vec![],
                    can_activate: None,
                    has: None,
                    run: hook(|ctx| {
                        vec![effects::add_player_modifier(json_as(json!({
                            "mod": {
                                "expiry": { "until": "thisTurn", "turn": ctx.state.turn },
                                "kind": "spellDamage",
                                "amount": 5,
                            },
                        })))]
                    }),
                }],
                ..Script::default()
            },
        ),
        (
            MAGE,
            Script {
                aura: Some(spells_have_aura(Keyword::Lifesteal)),
                ..Script::default()
            },
        ),
        (
            DOUBLER,
            Script {
                static_flags: flags(json!({ "doublesHealing": true })),
                ..Script::default()
            },
        ),
        (
            MEND,
            Script {
                targets: vec![TargetDecl::target(
                    1,
                    1,
                    json!({ "side": "any", "of": ["unit", "hero"] }),
                )],
                cry: Some(hook(|_ctx| {
                    vec![effects::heal(json_as(
                        json!({ "target": { "of": "chosen" }, "amount": 3 }),
                    ))]
                })),
                ..Script::default()
            },
        ),
        (
            MOON,
            Script {
                static_flags: flags(json!({ "healToDamage": true })),
                ..Script::default()
            },
        ),
        (BLOOM, health_price(None)),
        (BLOOM_ECHO, health_price(Some(1))),
        (
            PILER,
            Script {
                targets: vec![enemy_unit_or_hero()],
                cry: Some(hook(|_ctx| vec![hit_chosen(2)])),
                static_flags: flags(json!({ "answersTargetedSpells": true })),
                hand_triggers: vec![answer_trigger()],
                deck_triggers: vec![answer_trigger()],
                ..Script::default()
            },
        ),
        (
            ASKER,
            Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::choose_target(json_as(json!({
                        "step": "hit",
                        "scope": { "side": "enemy", "of": ["unit", "hero"] },
                    })))]
                })),
                resume: IndexMap::from([("hit", hook(|_ctx| vec![hit_chosen(1)]))]),
                ..Script::default()
            },
        ),
        (
            ECHO_BOLT,
            Script {
                targets: vec![enemy_unit_or_hero()],
                static_flags: flags(json!({ "echo": 1 })),
                cry: Some(hook(|_ctx| vec![hit_chosen(1)])),
                ..Script::default()
            },
        ),
        (BODY, Script::default()),
        (PAWN, Script::default()),
    ];
    entries
        .into_iter()
        .map(|(id, script)| (id.to_string(), both(script)))
        .collect()
}

/// The real catalog, for anchors, and these fixtures with their scripts, on this test's thread.
pub fn register() {
    let mut defs: CardDefs = serde_json::from_str(include_str!("../../../../cards/catalog.json"))
        .expect("crates/cards/catalog.json parses as CardDefs");
    defs.extend(catalog());
    register_catalog(defs);
    register_scripts(scripts());
}
