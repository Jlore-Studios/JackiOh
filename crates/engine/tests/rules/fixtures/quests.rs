//! Port of `packages/engine/test/fixtures/quests.ts`.
//!
//! Test-only cards for the quests subsystem (docs/classic-sets.md B5 E33; `src/subsystems/quests.ts`),
//! so the engine half of Classic #90 In Too Deep is proved without `packages/cards` (CLAUDE.md: the
//! engine never imports it). The real card arrives with its own script and test file.
//!
//!   - one Field Spell per goal kind (`GOAL_CARDS`), whose only quest is that goal and offers nothing:
//!     each count, and each board condition, proved alone;
//!   - `tree`, a small quest tree shaped like In Too Deep's: a reward prompt on the base face, every
//!     reward and every path on the Radiant face, a reward that asks a question of its own (a pause
//!     mid-reward), a quest reached by two paths, a reward two quests offer, a reward whose own draws
//!     come before the quest it opens, and an aura reward;
//!   - plain helper Spells and a draw-limiting Field Spell the tests drive the counts with.
//!
//! TS numbered the definitions from a module counter (`nextIndex = 3300`, one step per `def`): the
//! nine goal cards first, then the rest in source order. Each index is written out here.

use std::sync::LazyLock;

use jackioh_engine::effects;
use jackioh_engine::subsystems::quests::{
    held_quest_auras, hold_quest_aura, open_quest, quest_def_of, quest_reward_of,
};
use jackioh_engine::testkit::*;

/// TS's `{ ...a, ...b }` on two object literals: `b`'s keys replace `a`'s.
fn spread(mut base: Value, extra: Value) -> Value {
    if let (Some(into), Value::Object(from)) = (base.as_object_mut(), extra) {
        for (key, value) in from {
            into.insert(key, value);
        }
    }
    base
}

fn def(index: u32, name: &str, type_: &str, extra: Value) -> CardDef {
    json_as(spread(
        json!({
            "id": format!("qf-{name}"),
            "index": index.to_string(),
            "name": format!("{name} (quests)"),
            "set": "Core",
            "type": type_,
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 0,
            "base": { "keywords": [], "text": name },
            "radiant": { "keywords": [], "text": name },
        }),
        extra,
    ))
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

// ---------------------------------------------------------------------------------------------
// One goal kind per card
// ---------------------------------------------------------------------------------------------

/// TS `keyof typeof GOALS`: a goal kind's name, as `QuestGoal`'s `kind` writes it.
pub type GoalKind = &'static str;

/// The goal kinds, in `GOALS`' key order.
const GOAL_KINDS: [GoalKind; 9] = [
    "draws",
    "enemyPermanentsDestroyed",
    "unspentManaAtTurnEnd",
    "damageToEnemies",
    "cardsExiled",
    "deckEmptiedByDraw",
    "permanentsControlled",
    "unitTotals",
    "unitsInGraveyard",
];

pub static GOALS: LazyLock<IndexMap<GoalKind, QuestGoal>> = LazyLock::new(|| {
    let goals: [(GoalKind, Value); 9] = [
        ("draws", json!({ "kind": "draws", "count": 2 })),
        ("enemyPermanentsDestroyed", json!({ "kind": "enemyPermanentsDestroyed", "count": 2 })),
        ("unspentManaAtTurnEnd", json!({ "kind": "unspentManaAtTurnEnd", "mana": 3 })),
        ("damageToEnemies", json!({ "kind": "damageToEnemies", "amount": 5 })),
        ("cardsExiled", json!({ "kind": "cardsExiled", "count": 2 })),
        ("deckEmptiedByDraw", json!({ "kind": "deckEmptiedByDraw" })),
        ("permanentsControlled", json!({ "kind": "permanentsControlled", "count": 3 })),
        ("unitTotals", json!({ "kind": "unitTotals", "total": 6 })),
        ("unitsInGraveyard", json!({ "kind": "unitsInGraveyard", "count": 2 })),
    ];
    goals.into_iter().map(|(kind, goal)| (kind, json_as::<QuestGoal>(goal))).collect()
});

/// The id of the one quest each goal card has.
pub const ONLY_QUEST: &str = "q";

/// `QuestGoal["kind"]` as written (`goal.kind`).
fn goal_kind_of(goal: &QuestGoal) -> String {
    serde_json::to_value(goal)
        .ok()
        .and_then(|value| value.get("kind").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_default()
}

fn goal_book(goal: &QuestGoal) -> QuestBook {
    QuestBook {
        first: ONLY_QUEST.to_string(),
        quests: vec![QuestDef {
            id: ONLY_QUEST.to_string(),
            text: format!("goal: {}", goal_kind_of(goal)),
            goal: goal.clone(),
            rewards: vec![],
        }],
        rewards: vec![],
    }
}

pub static GOAL_CARDS: LazyLock<IndexMap<GoalKind, CardDef>> = LazyLock::new(|| {
    GOAL_KINDS
        .iter()
        .enumerate()
        .map(|(at, kind)| (*kind, def(3301 + at as u32, &format!("goal-{kind}"), "Field Spell", json!({ "cost": 1 }))))
        .collect()
});

// ---------------------------------------------------------------------------------------------
// The tree
// ---------------------------------------------------------------------------------------------

pub fn tree() -> CardDef {
    def(3310, "tree", "Field Spell", json!({ "cost": 1 }))
}

/// The amounts the tree's rewards deal, heal and draw.
pub const TREE_HEAL: i32 = 2;
pub const TREE_PING: i32 = 1;
pub const TREE_BUFF: i32 = 2;

/// draws 2 → heal (→ kill) or ask (→ board); kill → both (→ mana); board → both (→ mana) or hand
/// (→ fresh); mana → aura (end); fresh (draws 2) → heal (→ kill). So `mana` is reached by two paths,
/// `both` is offered by two quests, `ask` pauses for a target, `hand` draws 2 before `fresh` opens,
/// and `aura` is held.
pub static TREE: LazyLock<QuestBook> = LazyLock::new(|| {
    json_as(json!({
        "first": "draws",
        "quests": [
            { "id": "draws", "text": "Draw 2 cards", "goal": { "kind": "draws", "count": 2 }, "rewards": ["heal", "ask"] },
            { "id": "kill", "text": "Destroy an enemy permanent", "goal": { "kind": "enemyPermanentsDestroyed", "count": 1 }, "rewards": ["both"] },
            { "id": "board", "text": "Control 3 permanents", "goal": { "kind": "permanentsControlled", "count": 3 }, "rewards": ["both", "hand"] },
            { "id": "mana", "text": "End a turn with 3 mana", "goal": { "kind": "unspentManaAtTurnEnd", "mana": 3 }, "rewards": ["aura"] },
            { "id": "fresh", "text": "Draw 2 more cards", "goal": { "kind": "draws", "count": 2 }, "rewards": ["heal"] },
        ],
        "rewards": [
            { "id": "heal", "text": "Heal your hero 2", "next": "kill" },
            { "id": "ask", "text": "Deal 1 damage to a target", "next": "board" },
            { "id": "both", "text": "A random Unit of yours gets +2/+2", "next": "mana" },
            { "id": "hand", "text": "Draw 2 cards", "next": "fresh" },
            { "id": "aura", "text": "Aura: your Units have Indestructible", "next": null },
        ],
    }))
});

const TARGET_STEP: &str = "aimed";

/// A reward's own effects, before the quest it opens.
fn reward_effects(id: &str) -> Vec<Effect> {
    match id {
        "heal" => vec![effects::heal(json_as(json!({ "target": { "of": "selfHero" }, "amount": TREE_HEAL })))],
        "ask" => vec![effects::choose_target(json_as(json!({
            "step": TARGET_STEP,
            "scope": { "side": "any", "of": ["unit", "hero"] },
            "prompt": "Deal 1 damage",
        })))],
        "both" => vec![effects::buff_random_unit(json_as(json!({ "attack": TREE_BUFF, "health": TREE_BUFF })))],
        "hand" => vec![effects::draw(json_as(json!({ "count": 2 })))],
        "aura" => vec![hold_quest_aura("aura")],
        _ => vec![],
    }
}

fn next_of(id: &str) -> Vec<Effect> {
    let next = quest_reward_of(&TREE, id).and_then(|reward| reward.next.clone());
    match next {
        None => vec![],
        Some(next) => vec![open_quest(&next)],
    }
}

fn completed_quest(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<String> {
    match event {
        GameEvent::QuestCompleted { instance_id, quest, .. }
            if ctx.self_.as_ref().map(|card| card.id.as_str()) == Some(instance_id.as_str()) =>
        {
            Some(quest.clone())
        }
        _ => None,
    }
}

fn tree_script(radiant: bool) -> Script {
    let answer = TriggerDef::new("quest-completed", &[GameEventType::QuestCompleted], move |ctx, event| {
        let quest = completed_quest(ctx, event).and_then(|id| quest_def_of(&TREE, &id).cloned());
        let Some(quest) = quest else {
            return vec![];
        };
        if radiant {
            // Every reward, then every path.
            let mut list: Vec<Effect> = quest.rewards.iter().flat_map(|reward| reward_effects(reward)).collect();
            list.extend(quest.rewards.iter().flat_map(|reward| next_of(reward)));
            return list;
        }
        let rewards: Vec<Value> = quest
            .rewards
            .iter()
            .map(|reward| {
                let label = quest_reward_of(&TREE, reward)
                    .map(|found| found.text.clone())
                    .unwrap_or_else(|| reward.clone());
                json!({ "id": reward, "label": label })
            })
            .collect();
        vec![effects::choose_reward(json_as(json!({
            "step": "reward",
            "rewards": rewards,
            "prompt": format!("{}: choose a reward", quest.text),
        })))]
    });
    Script {
        // Quickdraw, so a replayable game deals it in the opening hand (§2.1, R225).
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        quests: Some(TREE.clone()),
        triggers: vec![answer],
        resume: IndexMap::from([
            (
                "reward",
                hook(|ctx| match effects::chosen_options(ctx).first() {
                    None => vec![],
                    Some(id) => {
                        let mut list = reward_effects(id);
                        list.extend(next_of(id));
                        list
                    }
                }),
            ),
            (
                TARGET_STEP,
                hook(|_ctx| vec![effects::damage(json_as(json!({ "to": { "of": "chosen" }, "amount": TREE_PING })))]),
            ),
        ]),
        aura: Some(aura_hook(|a| {
            if held_quest_auras(a.self_).iter().any(|held| held == "aura") {
                let controller = a.self_.controller;
                vec![AuraEntry {
                    applies: Box::new(move |unit: &CardInstance| unit.controller == controller),
                    mod_: StatMod {
                        keywords: Some(vec![json_as::<Keyword>(json!({ "kind": "Indestructible" }))]),
                        ..StatMod::default()
                    },
                }]
            } else {
                vec![]
            }
        })),
        ..Script::default()
    }
}

// ---------------------------------------------------------------------------------------------
// Helpers the tests drive the counts with
// ---------------------------------------------------------------------------------------------

/// Draw 1, and draw 2.
pub fn draw_one() -> CardDef {
    def(3311, "draw-one", "Spell", json!({}))
}

pub fn draw_two() -> CardDef {
    def(3312, "draw-two", "Spell", json!({}))
}

/// Destroy a declared unit (either side).
pub fn slay() -> CardDef {
    def(3313, "slay", "Spell", json!({}))
}

/// Exile a declared unit (either side).
pub fn banish() -> CardDef {
    def(3314, "banish", "Spell", json!({}))
}

/// Deal 3 to a declared unit or hero.
pub fn bolt() -> CardDef {
    def(3315, "bolt", "Spell", json!({}))
}

pub const BOLT: i32 = 3;

/// Draw 2, then Recruit (§6.3): a quest card recruited after the draws does not count them (R212).
pub fn draw_then_recruit() -> CardDef {
    def(3316, "draw-then-recruit", "Spell", json!({}))
}

/// Exile a declared backrow card (either side).
pub fn banish_any() -> CardDef {
    def(3317, "banish-any", "Spell", json!({}))
}

/// Exile the bottom card of your library: it leaves the deck, but no draw takes it.
pub fn mill() -> CardDef {
    def(3318, "mill", "Spell", json!({}))
}

/// "You can't draw more than 1 card each turn" (B5 E3).
pub fn limiter() -> CardDef {
    def(3319, "limiter", "Field Spell", json!({ "cost": 1 }))
}

/// A Field Spell with no text: the other half of a Fuse with the quest tree (R102).
pub fn blank() -> CardDef {
    def(3320, "blank", "Field Spell", json!({ "cost": 1 }))
}

fn unit_target() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
}

fn backrow_target() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["backrow"] }))]
}

fn any_target() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "hero"] }))]
}

fn helper_scripts() -> IndexMap<String, CardScripts> {
    let mut table: IndexMap<String, CardScripts> = IndexMap::new();
    table.insert(
        draw_one().id,
        both(Script {
            cry: Some(hook(|_ctx| vec![effects::draw(json_as(json!({ "count": 1 })))])),
            ..Script::default()
        }),
    );
    table.insert(
        draw_two().id,
        both(Script {
            static_flags: Some(StaticFlags {
                quickdraw: Some(true),
                ..StaticFlags::default()
            }),
            cry: Some(hook(|_ctx| vec![effects::draw(json_as(json!({ "count": 2 })))])),
            ..Script::default()
        }),
    );
    table.insert(
        draw_then_recruit().id,
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::draw(json_as(json!({ "count": 2 }))), effects::recruit(Default::default())]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        banish_any().id,
        both(Script {
            targets: backrow_target(),
            cry: Some(hook(|_ctx| vec![effects::exile(json_as(json!({ "target": { "of": "chosen" } })))])),
            ..Script::default()
        }),
    );
    table.insert(
        slay().id,
        both(Script {
            targets: unit_target(),
            cry: Some(hook(|_ctx| vec![effects::destroy(json_as(json!({ "target": { "of": "chosen" } })))])),
            ..Script::default()
        }),
    );
    table.insert(
        banish().id,
        both(Script {
            targets: unit_target(),
            cry: Some(hook(|_ctx| vec![effects::exile(json_as(json!({ "target": { "of": "chosen" } })))])),
            ..Script::default()
        }),
    );
    table.insert(
        bolt().id,
        both(Script {
            targets: any_target(),
            cry: Some(hook(|_ctx| {
                vec![effects::damage(json_as(json!({ "to": { "of": "chosen" }, "amount": BOLT })))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        mill().id,
        both(Script {
            cry: Some(hook(|_ctx| vec![effects::exile_bottom_of_library(json_as(json!({ "count": 1 })))])),
            ..Script::default()
        }),
    );
    table.insert(
        limiter().id,
        both(Script {
            draw_limit: Some(read_hook(|_args| {
                vec![DrawLimit {
                    player: DrawLimitPlayer::SelfSide,
                    count: 1,
                }]
            })),
            ..Script::default()
        }),
    );
    table
}

// ---------------------------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------------------------

/// Graveyard cards back to hand: `returnRandomFromGraveyard` proved through a Spell.
pub fn recall() -> CardDef {
    def(3321, "recall", "Spell", json!({}))
}

pub const RECALL: i32 = 2;

/// A random Unit of yours +1/+1: `buffRandomUnit` proved through a Spell.
pub fn bless() -> CardDef {
    def(3322, "bless", "Spell", json!({}))
}

pub static QUEST_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    let mut defs = vec![
        tree(),
        draw_one(),
        draw_two(),
        draw_then_recruit(),
        banish_any(),
        slay(),
        banish(),
        bolt(),
        mill(),
        limiter(),
        blank(),
        recall(),
        bless(),
    ];
    defs.extend(GOAL_CARDS.values().cloned());
    defs
});

/// TS `QUEST_SCRIPTS` (module-private there), as a call.
pub fn quest_scripts() -> IndexMap<String, CardScripts> {
    let mut table: IndexMap<String, CardScripts> = IndexMap::new();
    table.insert(
        tree().id,
        CardScripts {
            base: tree_script(false),
            radiant: tree_script(true),
        },
    );
    table.extend(helper_scripts());
    table.insert(
        recall().id,
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::return_random_from_graveyard(json_as(json!({ "count": RECALL })))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        bless().id,
        both(Script {
            cry: Some(hook(|_ctx| vec![effects::buff_random_unit(json_as(json!({ "attack": 1, "health": 1 })))])),
            ..Script::default()
        }),
    );
    for kind in GOAL_KINDS {
        table.insert(
            GOAL_CARDS[kind].id.clone(),
            both(Script {
                quests: Some(goal_book(&GOALS[kind])),
                ..Script::default()
            }),
        );
    }
    table
}

/// This file's definitions, by id (the brief's `catalog()`).
pub fn catalog() -> CardDefs {
    QUEST_DEFS.iter().map(|card| (card.id.clone(), card.clone())).collect()
}

/// This file's scripts, by id (the brief's `scripts()`).
pub fn scripts() -> IndexMap<String, CardScripts> {
    quest_scripts()
}

/// Add these fixtures to whatever the harness registered (`newGame` registers its own first).
pub fn register_quest_fixtures() {
    let mut all_defs = registered_catalog().clone();
    all_defs.extend(catalog());
    register_catalog(all_defs);
    let mut all_scripts = registered_scripts().clone();
    all_scripts.extend(scripts());
    register_scripts(all_scripts);
}
