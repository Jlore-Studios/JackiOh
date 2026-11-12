//! Test-only cards for Stinkyscape+ E8 (issue #3558, R3558000–R3558008): the hand, deck and fuse
//! verbs — hand positions, a hand aura on the neighbours, a meeting pair, a Spell filled once with
//! random Fruits, fusing until a cost, an end-of-turn hand fuse of a Grape, bottom starters, a deck
//! cast and casting one card N times — each through the engine's verbs and flags, so the engine
//! never imports `crates/cards`. Ids are `e8-` from 35580, clear of every other fixture file's.

use jackioh_engine::effects;
use jackioh_engine::testkit::*;

use super::harness::both;

/// ESSENCE's lend to the right: Radiance and −1 cost, while it is the leftmost card.
const ESSENCE_RIGHT_COST: i32 = -1;
/// ESSENCE's lend to the left, and every lend's price step: −1 cost.
const ESSENCE_LEFT_COST: i32 = -1;
/// A Grape is fused into the fermented card at most this many end-of-turn triggers in the tests.
pub const FRUIT_COST_CAP: i32 = 5;

/// Sweet Essence's shape: a 0-cost Spell lending its neighbours while it sits leftmost.
pub const ESSENCE: &str = "e8-essence";
/// Doomed Ship's shape: a 0-cost Spell adding the pair to the two ends of the hand.
pub const SHIP: &str = "e8-ship";
/// The pair's left card: a 2-cost 1/1 Unit with no text.
pub const PAIR_L: &str = "e8-pair-l";
/// The pair's right card: a 2-cost 1/1 Unit with no text.
pub const PAIR_R: &str = "e8-pair-r";
/// Smoothie's shape: a 3-cost Fruit Spell filled once with two random Fruits on arrival.
pub const SMOOTHIE: &str = "e8-smoothie";
/// Dingleberry Droat's shape: a Unit whose Cry fuses Fruits until they cost at least 5.
pub const DROAT: &str = "e8-droat";
/// Fine Fermented Grape's shape: a 1-cost Spell fusing a different Grape into itself at End of Turn.
pub const FERMENTED: &str = "e8-fermented";
/// A 1-cost Fruit Spell dealing 1 to a chosen target.
pub const RED_GRAPE: &str = "e8-red-grape";
/// A 2-cost Fruit Spell dealing 1 to a chosen target.
pub const GREEN_GRAPE: &str = "e8-green-grape";
/// A 3-cost Fruit Spell dealing 1 to a chosen target.
pub const APPLE: &str = "e8-apple";
/// A 0-cost Fruit Spell dealing 1 to a chosen target.
pub const ZERO_FRUIT: &str = "e8-zero-fruit";
/// A bottom starter: a 1-cost Spell with no text.
pub const BOTTOM_A: &str = "e8-bottom-a";
/// Another bottom starter.
pub const BOTTOM_B: &str = "e8-bottom-b";
/// Animosity's shape: a 2-cost Spell dealing 2 to the enemy hero, cast from the deck at 5 exiled.
pub const ANIMOSITY_FX: &str = "e8-animosity";
/// How many cards the owner's exile holds when Animosity casts itself.
pub const EXILE_TO_CAST: i32 = 5;
/// A 1-cost Spell exiling 1 random card of its owner's graveyard.
pub const EXILER: &str = "e8-exiler";
/// A 1-cost Spell dealing 1 to a chosen target, for the repeated cast.
pub const BOLT: &str = "e8-bolt";
/// Shadiris's shape: a Unit whose Cry casts BOLT twice.
pub const TWICE: &str = "e8-twice";

fn spell(index: u32, id: &str, name: &str, tags: Value, cost: i32) -> CardDef {
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": format!("{name} (e8 fixture)"),
        "set": "Core",
        "type": "Spell",
        "tags": tags,
        "rarity": "Common",
        "token": false,
        "cost": cost,
        "base": { "keywords": [], "text": id },
        "radiant": { "keywords": [], "text": id },
    }))
}

fn unit(index: u32, id: &str, name: &str, cost: i32, attack: i32, health: i32) -> CardDef {
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": format!("{name} (e8 fixture)"),
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": cost,
        "base": { "attack": attack, "health": health, "keywords": [], "text": id },
        "radiant": { "attack": attack * 2, "health": health * 2, "keywords": [], "text": id },
    }))
}

fn flags(value: Value) -> Option<StaticFlags> {
    Some(json_as(value))
}

fn chosen_target() -> Vec<TargetDecl> {
    vec![TargetDecl::target(
        1,
        1,
        json!({ "side": "any", "of": ["unit", "hero"] }),
    )]
}

fn deal_one_to_chosen() -> Vec<Effect> {
    vec![effects::damage(json_as(json!({
        "to": { "of": "chosen" },
        "amount": 1,
    })))]
}

/// ESSENCE: the right neighbour is Radiant at −1 while this is the leftmost card; the left
/// neighbour costs −1 whenever this sits beside it.
fn essence() -> Script {
    Script {
        hand_aura: Some(hand_aura_hook(|args| {
            let mut lends = vec![HandAura {
                to: HandSide::Left,
                radiant: false,
                cost: ESSENCE_LEFT_COST,
            }];
            if is_leftmost_in_hand(args.state, &args.self_.id) {
                lends.push(HandAura {
                    to: HandSide::Right,
                    radiant: true,
                    cost: ESSENCE_RIGHT_COST,
                });
            }
            lends
        })),
        ..Script::default()
    }
}

/// SHIP: the pair to the two ends of the hand, meeting at 0.
fn ship() -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![effects::add_meeting_pair(json_as(json!({
                "left": PAIR_L,
                "right": PAIR_R,
                "meetCost": 0,
            })))]
        })),
        ..Script::default()
    }
}

/// SMOOTHIE: two random Fruits fused into itself as it arrives, never filled again.
fn smoothie() -> Script {
    Script {
        enters_hand: Some(hook(|_ctx| {
            vec![effects::fuse_random_into(json_as(json!({
                "into": { "target": { "of": "self" } },
                "query": { "tags": ["Fruit"], "type": "Spell" },
                "count": 2,
                "unfusedOnly": true,
            })))]
        })),
        ..Script::default()
    }
}

/// DROAT: fuse Fruits until they cost at least 5, the fusion costing 2 in the hand.
fn droat() -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![effects::fuse_generated(json_as(json!({
                "query": { "tags": ["Fruit"] },
                "untilCost": FRUIT_COST_CAP,
                "costOverride": 2,
                "toHand": "self",
            })))]
        })),
        ..Script::default()
    }
}

/// FERMENTED: at its controller's End of Turn, fuse a different Grape into itself.
fn fermented() -> Script {
    Script {
        hand_triggers: vec![
            TriggerDef::new("e8-fermented-fuse", &[GameEventType::TurnEnded], |_ctx, _event| {
                vec![effects::fuse_random_into(json_as(json!({
                    "into": { "target": { "of": "self" } },
                    "query": { "nameContains": "grape" },
                })))]
            })
            .with_when(|ctx, event| {
                matches!(event, GameEvent::TurnEnded { player, .. } if *player == ctx.controller)
            }),
        ],
        ..Script::default()
    }
}

/// ANIMOSITY_FX: 2 to the enemy hero, cast from the deck once the owner's exile holds 5.
fn animosity() -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![effects::damage(json_as(json!({
                "to": { "of": "enemyHero" },
                "amount": 2,
            })))]
        })),
        cast_from_deck_when: Some(read_hook(|args| {
            args.state.players[args.self_.owner].exile.len() as i32 >= EXILE_TO_CAST
        })),
        ..Script::default()
    }
}

/// EXILER: exile 1 random card of its owner's graveyard.
fn exiler() -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![effects::exile_random_from(json_as(json!({
                "player": "self",
                "from": "graveyard",
                "count": 1,
            })))]
        })),
        ..Script::default()
    }
}

/// TWICE: cast BOLT twice, each cast asking its own target.
fn twice() -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            let (args, times) = twice_args();
            effects::cast_new_times(args, times)
        })),
        ..Script::default()
    }
}

fn fruit_script() -> Script {
    Script {
        targets: chosen_target(),
        cry: Some(hook(|_ctx| deal_one_to_chosen())),
        ..Script::default()
    }
}

/// TWICE: cast BOLT twice (see the fixture list above).
fn twice_args() -> (effects::CastNewArgs, i32) {
    (
        effects::CastNewArgs {
            def: effects::CastNewDef::Id(BOLT.to_string()),
            radiant: None,
            how: effects::CastHow::default(),
        },
        2,
    )
}

pub fn scripts() -> IndexMap<String, CardScripts> {
    let mut all: IndexMap<String, CardScripts> = IndexMap::new();
    all.insert(ESSENCE.to_string(), both(essence()));
    all.insert(SHIP.to_string(), both(ship()));
    all.insert(SMOOTHIE.to_string(), both(smoothie()));
    all.insert(FERMENTED.to_string(), both(fermented()));
    all.insert(ANIMOSITY_FX.to_string(), both(animosity()));
    all.insert(EXILER.to_string(), both(exiler()));
    all.insert(TWICE.to_string(), both(twice()));
    for id in [RED_GRAPE, GREEN_GRAPE, APPLE, ZERO_FRUIT, BOLT] {
        all.insert(id.to_string(), both(fruit_script()));
    }
    all.insert(DROAT.to_string(), both(droat()));
    for id in [PAIR_L, PAIR_R] {
        all.insert(id.to_string(), both(Script::default()));
    }
    // R3558006: the bottom starters carry the flag on both faces.
    for id in [BOTTOM_A, BOTTOM_B] {
        all.insert(
            id.to_string(),
            both(Script {
                static_flags: flags(json!({ "startsAtBottom": true })),
                ..Script::default()
            }),
        );
    }
    all
}

pub fn defs() -> Vec<CardDef> {
    vec![
        spell(35580, ESSENCE, "Sweet Essence", json!([]), 0),
        spell(35581, SHIP, "Doomed Ship", json!([]), 0),
        unit(35582, PAIR_L, "Kwzki", 2, 1, 1),
        unit(35583, PAIR_R, "Shockwave447", 2, 1, 1),
        spell(35584, SMOOTHIE, "Smoothie", json!(["Fruit"]), 3),
        unit(35585, DROAT, "Dingleberry Droat", 1, 4, 2),
        // No Fruit tag: the Smoothie pool stays the four damage Fruits, so the filled card always
        // asks two targets (R3558003); its Grape-ness is its name (R3558005).
        spell(35586, FERMENTED, "Fermented Grape", json!([]), 1),
        spell(35587, RED_GRAPE, "Red Grape", json!(["Fruit"]), 1),
        spell(35588, GREEN_GRAPE, "Green Grape", json!(["Fruit"]), 2),
        spell(35589, APPLE, "Apple", json!(["Fruit"]), 3),
        spell(35590, ZERO_FRUIT, "Zero Fruit", json!(["Fruit"]), 0),
        spell(35591, BOTTOM_A, "Animosity Watcher A", json!([]), 1),
        spell(35592, BOTTOM_B, "Animosity Watcher B", json!([]), 1),
        spell(35593, ANIMOSITY_FX, "Animosity", json!([]), 2),
        spell(35594, EXILER, "Exiler", json!([]), 1),
        spell(35595, BOLT, "Shadiris Visions", json!([]), 1),
        unit(35596, TWICE, "Shadiris", 1, 6, 26),
    ]
}

/// The engine's fixture catalog (`harness::setup_catalog`) with these cards and scripts on top.
pub fn register_e8_fixtures() {
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
