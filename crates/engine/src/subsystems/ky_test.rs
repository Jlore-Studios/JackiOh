//! Classic+ #42 KY's Test's question bank (docs/classic-sets.md B5 E31, SPEC §8.7 row 42, R420, R580).
//!
//! The machinery that is not card data: the Easy generator, a problem drawn from a bank handed in as
//! data (the Medium and Hard problems live in `packages/cards`, which the engine never imports), the
//! reward roll, and the two prompts. Each step is a plain-data continuation, so a paused game survives
//! JSON and replays (§9.3):
//!
//!   cry    one reward rolled per difficulty (`KY_TEST_REWARDS`; Hard has one entry, so no draw, R129),
//!          then a `mode` prompt of the three difficulties, each labelled "<difficulty>: <reward>".
//!   ask    a problem of the chosen difficulty — generated for Easy (R580), drawn from the bank
//!          otherwise — as an `answer` prompt (R465): the statement and the options in an rng-shuffled
//!          order, the right letter in the resume data where no view reaches (§10.8).
//!   grade  a right answer gains that difficulty's rolled reward (every card Radiant on the Radiant
//!          face); a wrong one nothing. Pools are non-token cards of every set (R380) through
//!          `addRandomFromCatalog`, which never offers the running card (R387); the hand cap burns.
//!
//! Port of `packages/engine/src/subsystems/kyTest.ts`. `KyTestDifficulty` is `crate::config`'s (part 1
//! placed it beside `KY_TEST_REWARDS`, whose `of` takes it), re-exported here under its TS module.

use std::sync::Arc;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::config::{
    KY_TEST_DIFFICULTIES, KY_TEST_EASY_ADDENDS, KY_TEST_EASY_MISSES, KY_TEST_OPTIONS, KY_TEST_REWARDS,
    KyTestRewardCount,
};
use crate::prelude::json_as;
use crate::rng::Rng;
use crate::script::{Effect, Hook, hook};

pub use crate::config::KyTestDifficulty;

/// R420: one problem; `answer` is the one of its `KY_TEST_OPTIONS` options that is right.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KyTestProblem {
    pub id: String,
    pub difficulty: KyTestDifficulty,
    pub statement: String,
    pub options: Vec<String>,
    pub answer: String,
}

/// Resume-data keys: the rolled reward id per difficulty, and the difficulty chosen.
const ROLLED: &str = "kyTestRewards";
const CHOSEN: &str = "kyTestDifficulty";

/// R420, R580: a + b with each addend drawn from `KY_TEST_EASY_ADDENDS`, and three wrong sums, each
/// a + b moved by a different entry of `KY_TEST_EASY_MISSES` — so the four options always differ.
pub fn easy_problem(rng: &mut Rng) -> KyTestProblem {
    let min = KY_TEST_EASY_ADDENDS.min;
    let max = KY_TEST_EASY_ADDENDS.max;
    let a = min + rng.int(max - min + 1);
    let b = min + rng.int(max - min + 1);
    let wrong = (KY_TEST_OPTIONS - 1).max(0) as usize;
    let misses: Vec<i32> = rng.shuffle(KY_TEST_EASY_MISSES).into_iter().take(wrong).collect();
    let sum = a + b;
    let mut options = vec![sum.to_string()];
    options.extend(misses.iter().map(|miss| (sum + miss).to_string()));
    KyTestProblem {
        id: format!("easy:{a}+{b}"),
        difficulty: KyTestDifficulty::Easy,
        statement: format!("{a} + {b} = ?"),
        options,
        answer: sum.to_string(),
    }
}

/// TS `isDifficulty(value)`: the difficulty a string names, when it is one of `KY_TEST_DIFFICULTIES`.
fn is_difficulty(value: &str) -> Option<KyTestDifficulty> {
    let difficulty = value.parse::<KyTestDifficulty>().ok()?;
    KY_TEST_DIFFICULTIES.contains(&difficulty).then_some(difficulty)
}

/// `kyTestScript`'s answer: the card's Cry and the steps its prompts re-enter, by step name.
#[derive(Clone)]
pub struct KyTestScript {
    pub cry: Hook,
    pub resume: IndexMap<&'static str, Hook>,
}

/// TS `{ ...reward.pool }` as a catalog query: only the fields the pool states.
fn pool_query(pool: Option<crate::config::KyTestRewardPool>) -> Value {
    let mut query = Map::new();
    if let Some(pool) = pool {
        if let Some(tags) = pool.tags {
            query.insert("tags".to_string(), json!(tags));
        }
        if let Some(rarity) = pool.rarity {
            query.insert("rarity".to_string(), json!(rarity));
        }
        if let Some(cost) = pool.cost {
            query.insert("cost".to_string(), json!(cost));
        }
    }
    Value::Object(query)
}

/// The whole card: its Cry and the two steps its prompts re-enter. `bank` holds the Medium and Hard problems.
pub fn ky_test_script(bank: &[KyTestProblem]) -> KyTestScript {
    let bank: Arc<Vec<KyTestProblem>> = Arc::new(bank.to_vec());

    let offer = Effect::new("kyTestOffer", |ctx| {
        let mut rolled = Map::new();
        let mut labels = Map::new();
        for &difficulty in KY_TEST_DIFFICULTIES {
            let list = KY_TEST_REWARDS.of(difficulty);
            // R129: one entry, no draw
            let reward = if list.len() == 1 {
                list.first()
            } else {
                ctx.sink.rng.pick(list)
            };
            rolled.insert(
                difficulty.as_str().to_string(),
                json!(reward.map(|reward| reward.id).unwrap_or("")),
            );
            labels.insert(
                difficulty.as_str().to_string(),
                json!(format!(
                    "{}: {}",
                    difficulty.as_str(),
                    reward.map(|reward| reward.label).unwrap_or("")
                )),
            );
        }
        let options: Vec<&str> = KY_TEST_DIFFICULTIES
            .iter()
            .map(|difficulty| difficulty.as_str())
            .collect();
        let prompt = crate::effects::choose_mode(json_as(json!({
            "options": options,
            "labels": Value::Object(labels),
            "step": "ask",
            "prompt": "Choose a problem",
            "data": { ROLLED: Value::Object(rolled) }
        })));
        (prompt.apply)(ctx);
    });

    let ask = {
        let bank = Arc::clone(&bank);
        Effect::new("kyTestAsk", move |ctx| {
            let Some(difficulty) = crate::effects::chosen_options(ctx)
                .iter()
                .find_map(|option| is_difficulty(option))
            else {
                return;
            };
            let problems: Vec<&KyTestProblem> = bank
                .iter()
                .filter(|problem| problem.difficulty == difficulty)
                .collect();
            let problem: Option<KyTestProblem> = if difficulty == KyTestDifficulty::Easy {
                Some(easy_problem(&mut *ctx.sink.rng))
            } else if problems.len() == 1 {
                problems.first().map(|problem| (*problem).clone())
            } else {
                ctx.sink.rng.pick(&problems).map(|problem| (*problem).clone())
            };
            let Some(problem) = problem else {
                return;
            };
            let correct = problem
                .options
                .iter()
                .position(|option| *option == problem.answer)
                .map_or(-1, |at| at as i32);
            let question = crate::effects::choose_answer(json_as(json!({
                "step": "grade",
                "statement": problem.statement,
                "options": problem.options,
                "correct": correct,
                "data": { CHOSEN: difficulty, "kyTestProblem": problem.id }
            })));
            (question.apply)(ctx);
        })
    };

    let grade: Hook = hook(|ctx| {
        let difficulty = ctx
            .data
            .get(CHOSEN)
            .and_then(Value::as_str)
            .and_then(is_difficulty);
        let Some(difficulty) = difficulty.filter(|_| crate::effects::answered_correctly(ctx)) else {
            return vec![];
        };
        let id = ctx
            .data
            .get(ROLLED)
            .and_then(|rolled| rolled.get(difficulty.as_str()))
            .cloned();
        let Some(reward) = KY_TEST_REWARDS
            .of(difficulty)
            .iter()
            .find(|entry| id.as_ref().and_then(Value::as_str) == Some(entry.id))
        else {
            return vec![];
        };
        let count = match reward.count {
            KyTestRewardCount::Fill => {
                crate::query::hand_cap_of(ctx.sink.state, ctx.controller)
                    - ctx.sink.state.players[ctx.controller].hand.len() as i32
            }
            KyTestRewardCount::N(n) => n,
        };
        if count <= 0 {
            return vec![]; // R129: a full hand has no room to fill, and nothing is drawn
        }
        let mut riders = Map::new();
        riders.insert("radiant".to_string(), json!(ctx.radiant));
        if let Some(cost_override) = reward.cost_override {
            riders.insert("costOverride".to_string(), json!(cost_override));
        }
        if let Some(def_id) = reward.def_id {
            return (0..count)
                .map(|_| {
                    let mut args = riders.clone();
                    args.insert("defId".to_string(), json!(def_id));
                    crate::effects::add_to_hand(json_as(Value::Object(args)))
                })
                .collect();
        }
        let mut args = riders;
        args.insert("query".to_string(), pool_query(reward.pool));
        args.insert("count".to_string(), json!(count));
        vec![crate::effects::add_random_from_catalog(json_as(Value::Object(
            args,
        )))]
    });

    let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
    resume.insert("ask", hook(move |_ctx| vec![ask.clone()]));
    resume.insert("grade", grade);
    KyTestScript {
        cry: hook(move |_ctx| vec![offer.clone()]),
        resume,
    }
}
