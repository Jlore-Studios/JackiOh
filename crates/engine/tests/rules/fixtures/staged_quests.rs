//! Test-only cards for Stinkyscape's staged Quests (issue #1854, R1854000–R1854004): each is the shape
//! of the set card that will use a primitive, written with the engine's own verbs through the prelude,
//! so the engine never imports `crates/cards` for these.
//!
//! Ids are prefixed `sq-` and indexed from 18540, so they collide with no other fixture file's.

#![allow(non_upper_case_globals)]

use std::sync::LazyLock;

use jackioh_engine::effects;
// The verbs as a card script names them, through the prelude (CLAUDE.md rule 5).
use jackioh_engine::prelude::{
    ActivationDecl, ActivationUses, HearBackArgs, QuestGoal, QuestStage, QuestStageGoal, QuestStageReward,
    complete_quests, complete_this_quest, hear_back, next_stage, open_stage, steal_mana_crystals, subsystems,
};
use jackioh_engine::testkit::*;

use super::harness::both;

fn def(index: u32, name: &str, type_: &str, cost: i32, tags: Value, stats: Option<(i32, i32)>) -> CardDef {
    let (base, radiant) = match stats {
        Some((attack, health)) => (
            json!({ "attack": attack, "health": health, "keywords": [], "text": name }),
            json!({ "attack": 2 * attack, "health": 2 * health, "keywords": [], "text": name }),
        ),
        None => (
            json!({ "keywords": [], "text": name }),
            json!({ "keywords": [], "text": name }),
        ),
    };
    json_as(json!({
        "id": format!("sq-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (staged quests fixture)"),
        "set": "Core",
        "type": type_,
        "tags": tags,
        "rarity": "Common",
        "token": false,
        "cost": cost,
        "base": base,
        "radiant": radiant,
    }))
}

fn spell(index: u32, name: &str, cost: i32) -> CardDef {
    def(index, name, "Spell", cost, json!([]), None)
}

// The quest cards

/// Stinkyscape #7 The Greatest Heist's shape: three stages, each one goal and one reward.
pub static heist: LazyLock<CardDef> =
    LazyLock::new(|| def(18540, "heist", "Field Spell", 1, json!(["Quest"]), None));

/// The Greatest Heist's three stages, as S09 will write them (its rewards here are stand-ins).
pub static HEIST_STAGES: LazyLock<Vec<QuestStage>> = LazyLock::new(|| {
    json_as(json!([
        {
            "name": "Initiation",
            "goals": [{ "id": "radiant", "text": "Make an Acclaimed Unit Radiant",
                        "goal": { "kind": "unitsMadeRadiant", "count": 1, "tag": "Acclaimed" } }],
            "reward": { "id": "initiation", "text": "Draw a card" },
        },
        {
            "name": "Comradery",
            "goals": [{ "id": "deaths", "text": "Have 2 Acclaimed Units you control die",
                        "goal": { "kind": "unitsDied", "count": 2, "tag": "Acclaimed" } }],
            "reward": { "id": "comradery", "text": "Gain 1 mana" },
        },
        {
            "name": "Empty the Vaults",
            "goals": [{ "id": "steal", "text": "Steal an enemy card",
                        "goal": { "kind": "cardsStolen", "count": 1 } }],
            "reward": { "id": "vaults", "text": "Steal your enemy's mana crystals" },
        },
    ]))
});

fn heist_script() -> Script {
    Script {
        stages: HEIST_STAGES.clone(),
        triggers: vec![TriggerDef::new(
            "stage-completed",
            &[GameEventType::QuestCompleted],
            |ctx, event| match subsystems::completed_stage_of(ctx, event) {
                Some(1) => vec![effects::draw(json_as(json!({ "count": 1 }))), next_stage()],
                Some(2) => vec![effects::gain_mana(json_as(json!({ "amount": 1 }))), next_stage()],
                Some(3) => vec![steal_mana_crystals()],
                _ => vec![],
            },
        )],
        ..Script::default()
    }
}

/// Stinkyscape #112 Job Application's shape: a resume sent in by its Activate, a wait to hear back,
/// an Interview that draws one of three goals, and an Offer claimed or sent back to stage 1.
pub static application: LazyLock<CardDef> =
    LazyLock::new(|| def(18541, "application", "Field Spell", 1, json!(["Quest"]), None));

/// The turns a sent resume hears back in (the set's card evaluates the resume, S10).
pub const HEAR_BACK_TURNS: i32 = 2;
/// The cost the fixture remembers for its resume, as S10's card will read it off the card sent in.
pub const RESUME_COST: i32 = 2;
/// Where the fixture remembers its resume's cost (`remember`), which the Interview's first goal reads.
pub const RESUME_KEY: &str = "resumeCost";
pub const PROVE: &str = "prove";
pub const BEHAVIORAL: &str = "behavioral";
pub const HARDBALL: &str = "hardball";
pub const OFFER_MANA: i32 = 2;
pub const HARDBALL_DAMAGE: i32 = 12;

pub static APPLICATION_STAGES: LazyLock<Vec<QuestStage>> = LazyLock::new(|| {
    json_as(json!([
        {
            "name": "Upload Resume",
            "goals": [{ "id": "resume", "text": "Send a card from your hand in as your resume",
                        "goal": { "kind": "byEffect" } }],
            "reward": { "id": "upload", "text": "Hear back in 2 turns" },
        },
        {
            "name": "The Interview",
            "goals": [
                { "id": PROVE, "text": "Play a card that costs the same as your resume",
                  "goal": { "kind": "playCosting", "cost": 0, "remembered": RESUME_KEY } },
                { "id": BEHAVIORAL, "text": "Play 3 cards of different costs in a single turn",
                  "goal": { "kind": "differentCostsInTurn", "count": 3 } },
                { "id": HARDBALL, "text": "Take or deal 12+ damage in a single instance",
                  "goal": { "kind": "damageInstance", "amount": HARDBALL_DAMAGE } },
            ],
            "reward": { "id": "interview", "text": "Draw a card" },
        },
        {
            "name": "The Offer",
            "goals": [{ "id": "offer", "text": "Activate to claim it", "goal": { "kind": "byEffect" } }],
            "reward": { "id": "paycheck", "text": "Gain 2 mana" },
        },
    ]))
});

/// The Activate abilities Job Application's fixture keeps: "Send in a resume" (stage 1), "Claim the
/// offer" (stage 3) and "Apply again" (back to stage 1).
pub const SEND: &str = "send";
pub const CLAIM: &str = "claim";
pub const REAPPLY: &str = "reapply";
/// A test's shortcut, no card's text: "Activate: open the Interview now".
pub const INTERVIEW: &str = "interview";

fn ability(id: &str, run: Hook) -> ActivationDecl {
    ActivationDecl {
        id: id.to_string(),
        label: id.to_string(),
        uses: ActivationUses::Unlimited,
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run,
    }
}

fn application_script() -> Script {
    Script {
        stages: APPLICATION_STAGES.clone(),
        activations: vec![
            ability(
                SEND,
                hook(|_ctx| {
                    vec![
                        effects::remember(json_as(json!({ "key": RESUME_KEY, "value": RESUME_COST }))),
                        complete_this_quest(),
                    ]
                }),
            ),
            ability(CLAIM, hook(|_ctx| vec![complete_this_quest()])),
            ability(REAPPLY, hook(|_ctx| vec![open_stage(1)])),
            ability(INTERVIEW, hook(|_ctx| vec![open_stage(2)])),
        ],
        triggers: vec![TriggerDef::new(
            "stage-completed",
            &[GameEventType::QuestCompleted],
            |ctx, event| match subsystems::completed_stage_of(ctx, event) {
                Some(1) => vec![hear_back(HearBackArgs {
                    turns: HEAR_BACK_TURNS,
                    stage: None,
                })],
                Some(2) => vec![effects::draw(json_as(json!({ "count": 1 }))), next_stage()],
                Some(3) => vec![effects::gain_mana(json_as(json!({ "amount": OFFER_MANA })))],
                _ => vec![],
            },
        )],
        ..Script::default()
    }
}

/// A staged quest without the Quest tag: "complete the current stage on all your Quests" passes it by.
pub static untagged_stages: LazyLock<CardDef> =
    LazyLock::new(|| def(18542, "untagged-stages", "Field Spell", 1, json!([]), None));

/// The quest fixtures' tree (`quests::tree`, In Too Deep's shape) with the Quest tag In Too Deep has
/// (R1837000), so Tylannt's completion reaches a tree's line too.
pub static quest_tree: LazyLock<CardDef> = LazyLock::new(|| {
    let mut card = super::quests::tree();
    card.id = "sq-quest-tree".to_string();
    card.index = "18543".to_string();
    card.tags = vec![Tag::Quest];
    card
});

/// A staged quest whose first reward hears back in 0 turns: "Stage 1: Activate (hear back in 0 turns).
/// Stage 2: Activate."
pub static instant: LazyLock<CardDef> =
    LazyLock::new(|| def(18560, "instant", "Field Spell", 1, json!(["Quest"]), None));

fn instant_script() -> Script {
    Script {
        stages: json_as(json!([
            { "name": "Ask", "goals": [{ "id": "ask", "text": "Activate", "goal": { "kind": "byEffect" } }],
              "reward": { "id": "asked", "text": "Hear back in 0 turns" } },
            { "name": "Answer", "goals": [{ "id": "answer", "text": "Activate", "goal": { "kind": "byEffect" } }],
              "reward": { "id": "answered", "text": "Nothing" } },
        ])),
        activations: vec![ability(SEND, hook(|_ctx| vec![complete_this_quest()]))],
        triggers: vec![TriggerDef::new(
            "stage-completed",
            &[GameEventType::QuestCompleted],
            |ctx, event| match subsystems::completed_stage_of(ctx, event) {
                Some(1) => vec![hear_back(HearBackArgs {
                    turns: 0,
                    stage: None,
                })],
                _ => vec![],
            },
        )],
        ..Script::default()
    }
}

// One staged quest per new goal

/// A goal kind's name, as `QuestGoal`'s `kind` writes it.
pub type GoalKind = &'static str;

/// Each new goal (R1854002, and `byEffect`, R1854001) with the numbers its card asks for.
pub static GOALS: LazyLock<IndexMap<GoalKind, QuestGoal>> = LazyLock::new(|| {
    let goals: [(GoalKind, Value); 7] = [
        (
            "unitsMadeRadiant",
            json!({ "kind": "unitsMadeRadiant", "count": 1, "tag": "Acclaimed" }),
        ),
        (
            "unitsDied",
            json!({ "kind": "unitsDied", "count": 2, "tag": "Acclaimed" }),
        ),
        ("cardsStolen", json!({ "kind": "cardsStolen", "count": 1 })),
        ("playCosting", json!({ "kind": "playCosting", "cost": 2 })),
        (
            "differentCostsInTurn",
            json!({ "kind": "differentCostsInTurn", "count": 3 }),
        ),
        (
            "damageInstance",
            json!({ "kind": "damageInstance", "amount": 12 }),
        ),
        ("byEffect", json!({ "kind": "byEffect" })),
    ];
    goals
        .into_iter()
        .map(|(kind, goal)| (kind, json_as::<QuestGoal>(goal)))
        .collect()
});

/// The id of the one goal each goal card's one stage has.
pub const ONLY_GOAL: &str = "goal";

/// One Field Spell per new goal, tagged Quest: one stage, that goal, a reward that does nothing.
pub static GOAL_CARDS: LazyLock<IndexMap<GoalKind, CardDef>> = LazyLock::new(|| {
    GOALS
        .keys()
        .enumerate()
        .map(|(at, kind)| {
            (
                *kind,
                def(
                    18561 + at as u32,
                    &format!("goal-{kind}"),
                    "Field Spell",
                    0,
                    json!(["Quest"]),
                    None,
                ),
            )
        })
        .collect()
});

fn goal_stages(kind: GoalKind) -> Vec<QuestStage> {
    vec![QuestStage {
        name: kind.to_string(),
        goals: vec![QuestStageGoal {
            id: ONLY_GOAL.to_string(),
            text: format!("goal: {kind}"),
            goal: GOALS[kind].clone(),
        }],
        reward: QuestStageReward {
            id: "none".to_string(),
            text: "Nothing".to_string(),
        },
    }]
}

// Tylannt

/// Stinkyscape #6 Tylannt's shape: "Cry: Complete the current stage on all your Quests."
pub static tylannt: LazyLock<CardDef> = LazyLock::new(|| {
    def(
        18544,
        "tylannt",
        "Unit",
        2,
        json!(["Acclaimed", "Human"]),
        Some((6, 4)),
    )
});

/// Tylannt's Radiant Cry, on a Spell: "Summon The Greatest Heist. Complete the current stage on all
/// your Quests."
pub static radiant_tylannt: LazyLock<CardDef> = LazyLock::new(|| spell(18545, "radiant-tylannt", 0));

// The goals' drivers

pub static acclaimed: LazyLock<CardDef> =
    LazyLock::new(|| def(18546, "acclaimed", "Unit", 1, json!(["Acclaimed"]), Some((1, 1))));
pub static nobody: LazyLock<CardDef> =
    LazyLock::new(|| def(18547, "nobody", "Unit", 1, json!([]), Some((1, 1))));

/// GIGA Glowy Jelly Bean's shape: "Make every card in your hand Radiant."
pub static jelly: LazyLock<CardDef> = LazyLock::new(|| spell(18548, "jelly", 0));
/// "Make a Unit Radiant" (either side).
pub static glow: LazyLock<CardDef> = LazyLock::new(|| spell(18549, "glow", 0));
/// "Destroy a Unit" (either side).
pub static smite: LazyLock<CardDef> = LazyLock::new(|| spell(18550, "smite", 0));
/// "Steal a Unit" (§6.3 Steal, onto your side).
pub static thief: LazyLock<CardDef> = LazyLock::new(|| spell(18551, "thief", 0));
/// "Steal a random card from your opponent's hand" (`stolen`).
pub static pickpocket: LazyLock<CardDef> = LazyLock::new(|| spell(18552, "pickpocket", 0));
/// "Your opponent takes a random card from your hand" — a card the opponent receives.
pub static charity: LazyLock<CardDef> = LazyLock::new(|| spell(18553, "charity", 0));
/// "Deal 12 damage", and "Deal 11 damage", to a target.
pub static big_hit: LazyLock<CardDef> = LazyLock::new(|| spell(18554, "big-hit", 0));
pub static near_hit: LazyLock<CardDef> = LazyLock::new(|| spell(18555, "near-hit", 0));
/// "Bounce a permanent in a backrow" (either side), so a quest card leaves the field (R78).
pub static recall: LazyLock<CardDef> = LazyLock::new(|| spell(18568, "recall", 0));
/// "Steal your enemy's mana crystals", on a Spell (R1854003).
pub static heist_spell: LazyLock<CardDef> = LazyLock::new(|| spell(18556, "heist-spell", 0));
/// Spells of no text at costs 0, 1 and 2, for "different costs".
pub static cost_zero: LazyLock<CardDef> = LazyLock::new(|| spell(18557, "cost-zero", 0));
pub static cost_one: LazyLock<CardDef> = LazyLock::new(|| spell(18558, "cost-one", 1));
pub static cost_two: LazyLock<CardDef> = LazyLock::new(|| spell(18559, "cost-two", 2));

pub const BIG_HIT: i32 = 12;
pub const NEAR_HIT: i32 = 11;

fn chosen() -> Value {
    json!({ "target": { "of": "chosen" } })
}

fn unit_target() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
}

fn backrow_target() -> Vec<TargetDecl> {
    vec![TargetDecl::target(
        1,
        1,
        json!({ "side": "any", "of": ["backrow"] }),
    )]
}

fn any_target() -> Vec<TargetDecl> {
    vec![TargetDecl::target(
        1,
        1,
        json!({ "side": "any", "of": ["unit", "hero"] }),
    )]
}

fn cry(targets: Vec<TargetDecl>, run: Hook) -> CardScripts {
    both(Script {
        targets,
        cry: Some(run),
        ..Script::default()
    })
}

static SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    let tree_scripts = super::quests::scripts()
        .get(&super::quests::tree().id)
        .cloned()
        .expect("the quest fixtures' tree");
    IndexMap::from([
        (heist.id.clone(), both(heist_script())),
        (application.id.clone(), both(application_script())),
        (
            untagged_stages.id.clone(),
            both(Script {
                stages: HEIST_STAGES.clone(),
                ..Script::default()
            }),
        ),
        (quest_tree.id.clone(), tree_scripts),
        (
            tylannt.id.clone(),
            cry(vec![], hook(|_ctx| vec![complete_quests()])),
        ),
        (
            radiant_tylannt.id.clone(),
            cry(
                vec![],
                hook(|_ctx| {
                    vec![
                        effects::summon(json_as(json!({ "defId": heist.id }))),
                        complete_quests(),
                    ]
                }),
            ),
        ),
        (
            jelly.id.clone(),
            cry(
                vec![],
                hook(|_ctx| {
                    vec![effects::set_radiant_random(json_as(
                        json!({ "zones": "hand", "count": HAND_CAP }),
                    ))]
                }),
            ),
        ),
        (
            glow.id.clone(),
            cry(
                unit_target(),
                hook(|_ctx| vec![effects::set_radiant(json_as(chosen()))]),
            ),
        ),
        (
            smite.id.clone(),
            cry(
                unit_target(),
                hook(|_ctx| vec![effects::destroy(json_as(chosen()))]),
            ),
        ),
        (
            thief.id.clone(),
            cry(
                unit_target(),
                hook(|_ctx| vec![effects::steal(json_as(chosen()))]),
            ),
        ),
        (
            pickpocket.id.clone(),
            cry(
                vec![],
                hook(|_ctx| {
                    vec![effects::give_from_hand(json_as(
                        json!({ "cards": "random", "count": 1 }),
                    ))]
                }),
            ),
        ),
        (
            charity.id.clone(),
            cry(
                vec![],
                hook(|_ctx| {
                    vec![effects::give_from_hand(json_as(
                        json!({ "from": "self", "cards": "random", "count": 1 }),
                    ))]
                }),
            ),
        ),
        (
            big_hit.id.clone(),
            cry(
                any_target(),
                hook(|_ctx| {
                    vec![effects::damage(json_as(
                        json!({ "to": { "of": "chosen" }, "amount": BIG_HIT }),
                    ))]
                }),
            ),
        ),
        (
            near_hit.id.clone(),
            cry(
                any_target(),
                hook(|_ctx| {
                    vec![effects::damage(json_as(
                        json!({ "to": { "of": "chosen" }, "amount": NEAR_HIT }),
                    ))]
                }),
            ),
        ),
        (
            heist_spell.id.clone(),
            cry(vec![], hook(|_ctx| vec![steal_mana_crystals()])),
        ),
        (
            recall.id.clone(),
            cry(
                backrow_target(),
                hook(|_ctx| vec![effects::bounce(json_as(chosen()))]),
            ),
        ),
        (instant.id.clone(), both(instant_script())),
    ])
    .into_iter()
    .chain(GOAL_CARDS.iter().map(|(kind, card)| {
        (
            card.id.clone(),
            both(Script {
                stages: goal_stages(kind),
                ..Script::default()
            }),
        )
    }))
    .collect()
});

pub static STAGED_QUEST_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        heist.clone(),
        application.clone(),
        untagged_stages.clone(),
        quest_tree.clone(),
        tylannt.clone(),
        radiant_tylannt.clone(),
        acclaimed.clone(),
        nobody.clone(),
        jelly.clone(),
        glow.clone(),
        smite.clone(),
        thief.clone(),
        pickpocket.clone(),
        charity.clone(),
        big_hit.clone(),
        near_hit.clone(),
        heist_spell.clone(),
        cost_zero.clone(),
        cost_one.clone(),
        cost_two.clone(),
        recall.clone(),
        instant.clone(),
    ]
    .into_iter()
    .chain(GOAL_CARDS.values().cloned())
    .collect()
});

pub fn register_staged_quest_fixtures() {
    super::harness::setup_catalog();
    super::quests::register_quest_fixtures();
    let mut defs: CardDefs = registered_catalog().clone();
    for card in STAGED_QUEST_DEFS.iter() {
        defs.insert(card.id.clone(), card.clone());
    }
    register_catalog(defs);
    let mut merged: IndexMap<String, CardScripts> = registered_scripts().clone();
    merged.extend(SCRIPTS.clone());
    register_scripts(merged);
}
