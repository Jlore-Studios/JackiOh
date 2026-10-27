//! SPEC §8.10 Stinkyscape 112 Job Application (§6.3 Quest stages, Hold beneath, Buff; §6.2 Death; §6.1
//! Lucky; R81, R384, R1854000–R1854004, R1865000–R1865002). Field Spell, Quest, cost 1, Legendary, a card
//! of the held Stinkyscape set (R1420).
//!   Base:    "Get employed! Its quest runs in three stages, one at a time. Death: Resummon this at
//!            Stage 1. Stage 1, Upload Resume: Activate this to send a card from your hand in as your
//!            resume; hear back in 2 to 6 turns. Stage 2, The Interview, one of three drawn at random:
//!            play a card that costs the same as your resume, play 3 cards of different costs in one
//!            turn, or take or deal 12 or more damage in one hit; add your resume to your hand, Buffed
//!            {buffs} times, at (0). Stage 3, The Offer: Activate this to accept the offer, or to send in
//!            another application, back to Stage 1, with every reward tripled; from your next turn on, a
//!            Paycheck at the start of each of your turns." (buffs 5)
//!   Radiant: "Lucky 1", and the same text. (buffs 10)
//!
//! The quest is `Script.stages` (R1854000): Stage 1 opens as the card enters, and each stage's reward is
//! the card's own trigger on its own `questCompleted`. Stage 1's goal is an Activate (`SEND`, R384) that
//! needs a hand card (R703): it notes the resume's cost and its wait, sets the card aside beneath this
//! one (`set_aside_from_hand`, R1865000) and completes the stage (`complete_this_quest`, R1854001); the
//! reward hears back (`hear_back`, R1854004) after the wait the resume earned (R1865001): a weighted draw
//! from the match rng by its merit (its rarity, its cost and its Radiant face), an iKwzki always 6.
//! Stage 2 draws one of three goals as it opens (R1854002), the first asking for the noted cost; its
//! reward gives the resume back at (0), Buffed `buffs` times the reward factor (`return_held_to_hand`).
//! Stage 3's goal is an Activate of two modes (`ANSWER`, R81): accept, and the card adds factor
//! Paychecks (S #112.1) at each of its controller's later starts of turn while it stays; or re-apply,
//! tripling the factor (at most ×9) and opening Stage 1 again (R1865002). Death summons it from the
//! graveyard into its own backrow zone, else R64's, its line, its factor and its memory reset (R78); its
//! resume, whose holder left the field, is exiled (R1865000). An exile or a bounce ends it.
//! Radiant: its Lucky 1 (`lucky_on`) draws the wait again and keeps the shorter, and `buffs` is 10.
//! Tunes: buffs 5 ↑ (Radiant 10).

use jackioh_engine::effects::{
    AddToHandArgs, HearBackArgs, RememberArgs, ReturnHeldArgs, SetAsideArgs, SummonArgs, add_to_hand,
    chosen_options, complete_this_quest, hear_back, instance_of, next_stage, open_stage, remember,
    return_held_to_hand, set_aside_from_hand, summon,
};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-112";

/// Stage 3's reward, S #112.1.
const PAYCHECK: &str = "stinkyscape-112-1";
/// S #110 iKwzki: a resume that always hears back in `LONGEST_WAIT` (the designer's note).
const IKWZKI: &str = "stinkyscape-110";

/// Upload Resume, the stage the card starts at and a re-application opens again.
const UPLOAD_STAGE: u32 = 1;
/// The Interview, whose reward gives the resume back.
const INTERVIEW_STAGE: u32 = 2;
/// The Offer, the last stage.
const OFFER_STAGE: u32 = 3;

/// Stage 1's one goal, met by `SEND`.
const RESUME_GOAL: &str = "resume";
/// The Interview's three goals, one drawn as it opens.
const PROVE: &str = "prove";
const BEHAVIORAL: &str = "behavioral";
const HARDBALL: &str = "hardball";
/// Stage 3's one goal, met by `ANSWER`'s accept.
const OFFER_GOAL: &str = "offer";

/// The two Activates, each once a turn under its own id (R384).
const SEND: &str = "send-resume";
const ANSWER: &str = "answer-offer";
/// `ANSWER`'s two modes (R81).
const ACCEPT: &str = "Accept the offer";
const REAPPLY: &str = "Send in another application";

/// What the card remembers on its stay: the resume's cost, which the Interview's first goal reads
/// (R1854002's `remembered`); the wait the resume earned, until Stage 1's reward reads it; the reward
/// factor; and whether the Offer was accepted.
const RESUME_COST_KEY: &str = "resumeCost";
const RESUME_WAIT_KEY: &str = "resumeWait";
const REWARD_FACTOR_KEY: &str = "rewardFactor";
const HIRED_KEY: &str = "hired";

/// "Hear back in 2 to 6 turns".
const SHORTEST_WAIT: i32 = 2;
const LONGEST_WAIT: i32 = 6;
/// "Play 3 cards of different costs in one turn".
const BEHAVIORAL_COSTS: i32 = 3;
/// "Take or deal 12 or more damage in one hit".
const HARDBALL_DAMAGE: i32 = 12;
/// The cost the Interview's first goal asks for while no resume's cost is noted (R1865001).
const NO_RESUME_COST: i32 = 0;
/// "It costs (0)".
const RESUME_PRICE: i32 = 0;
/// The reward factor before any re-application, what each one multiplies it by, and its cap (R1865002).
const BASE_FACTOR: i32 = 1;
const REWARD_TRIPLING: i32 = 3;
const REWARD_FACTOR_CAP: i32 = 9;
/// R1865001: the merit from which a resume waits on the solid table, and on the strong one; and what a
/// Radiant resume adds to it.
const SOLID_MERIT: i32 = 3;
const STRONG_MERIT: i32 = 6;
const RADIANT_MERIT: i32 = 1;

/// R1865001: each rarity's merit is its place here (a Token's, which is none of them, 0).
const RARITY_LADDER: [Rarity; 5] = [
    Rarity::Common,
    Rarity::Rare,
    Rarity::Epic,
    Rarity::Legendary,
    Rarity::Mythic,
];

/// R1865001: the weights of the waits `SHORTEST_WAIT` to `LONGEST_WAIT`, in that order, for a weak, a
/// solid and a strong resume.
const WEAK_WAITS: [i32; 5] = [1, 2, 3, 4, 5];
const SOLID_WAITS: [i32; 5] = [2, 3, 4, 3, 2];
const STRONG_WAITS: [i32; 5] = [5, 4, 3, 2, 1];

/// `remember` of one key.
fn note(key: &str, value: Value) -> Effect {
    remember(RememberArgs {
        key: key.to_string(),
        value,
    })
}

/// Whether `goal` is open on the card's line and not yet met, so its Activate is the card's now.
fn open_now(card: &CardInstance, goal: &str) -> bool {
    subsystems::quest_memory_of(card)
        .is_some_and(|line| line.active.iter().any(|id| id == goal) && !line.met.iter().any(|id| id == goal))
}

/// R1865001: a resume's merit — its rarity's place on `RARITY_LADDER`, its cost now (0 to `MAX_MANA`),
/// and `RADIANT_MERIT` more on its Radiant face.
fn merit_of(state: &GameState, resume: &CardInstance) -> i32 {
    let rarity = def_of(Some(state), &resume.def_id).rarity;
    let rank = RARITY_LADDER
        .iter()
        .position(|rung| *rung == rarity)
        .map_or(0, |at| at as i32);
    let radiant = if resume.radiant { RADIANT_MERIT } else { 0 };
    rank + cost_now(state, resume).clamp(0, MAX_MANA) + radiant
}

/// R1865001: the table a resume of `merit` waits on.
fn waits_for(merit: i32) -> &'static [i32; 5] {
    if merit >= STRONG_MERIT {
        &STRONG_WAITS
    } else if merit >= SOLID_MERIT {
        &SOLID_WAITS
    } else {
        &WEAK_WAITS
    }
}

/// One draw of the match rng over the weights' sum, walked from the shortest wait.
fn draw_wait(rng: &mut Rng, weights: &[i32]) -> i32 {
    let mut at = rng.int(weights.iter().sum());
    for (index, weight) in weights.iter().enumerate() {
        if at < *weight {
            return SHORTEST_WAIT + index as i32;
        }
        at -= weight;
    }
    LONGEST_WAIT
}

/// R1865001, §6.1: the wait, a luck-based roll whose better side is the shorter wait, so the card's
/// Lucky (printed and given, R1438) and its controller's Luck (R987) each draw it again.
fn roll_wait(ctx: &mut EffectContext<'_>, weights: &[i32]) -> i32 {
    let lucky = ctx.live_self().map_or(0, |me| lucky_on(&*ctx.state, me)) + luck_of(&*ctx.state, ctx.controller);
    ctx.rng.lucky(lucky, |rng| draw_wait(rng, weights), i32::min)
}

/// R1865002: the reward factor the card holds on its stay.
fn factor_of(ctx: &EffectContext<'_>) -> i32 {
    recalled(ctx, REWARD_FACTOR_KEY)
        .and_then(|value| value.as_i64())
        .map_or(BASE_FACTOR, |factor| {
            factor.clamp(i64::from(BASE_FACTOR), i64::from(REWARD_FACTOR_CAP)) as i32
        })
}

/// R1865002: "with every reward tripled", never past `REWARD_FACTOR_CAP`.
fn tripled(factor: i32) -> i32 {
    (factor * REWARD_TRIPLING).min(REWARD_FACTOR_CAP)
}

/// R1865002: the lane a dead Job Application is summoned back into: its own backrow zone while that
/// takes a summon (R688), else none, and R64's zone.
fn resummon_lane(state: &GameState, me: &CardInstance) -> Option<i32> {
    slot_of(state, me)
        .filter(|slot| takes_move(state, *slot))
        .map(|slot| slot.lane)
}

fn stages() -> Vec<QuestStage> {
    json_as(json!([
        {
            "name": "Upload Resume",
            "goals": [{ "id": RESUME_GOAL, "text": "Activate this to send a card from your hand in as your resume",
                        "goal": { "kind": "byEffect" } }],
            "reward": { "id": "hearBack", "text": format!("Hear back in {SHORTEST_WAIT} to {LONGEST_WAIT} turns") },
        },
        {
            "name": "The Interview",
            "goals": [
                { "id": PROVE, "text": "Play a card that costs the same as your resume",
                  "goal": { "kind": "playCosting", "cost": NO_RESUME_COST, "remembered": RESUME_COST_KEY } },
                { "id": BEHAVIORAL, "text": "Play 3 cards of different costs in one turn",
                  "goal": { "kind": "differentCostsInTurn", "count": BEHAVIORAL_COSTS } },
                { "id": HARDBALL, "text": "Take or deal 12 or more damage in one hit",
                  "goal": { "kind": "damageInstance", "amount": HARDBALL_DAMAGE } },
            ],
            "reward": { "id": "resumeBack", "text": "Add your resume to your hand and Buff it. It costs (0)" },
        },
        {
            "name": "The Offer",
            "goals": [{ "id": OFFER_GOAL, "text": "Activate this to accept the offer, or to send in another application",
                        "goal": { "kind": "byEffect" } }],
            "reward": { "id": "paychecks",
                        "text": "From your next turn on, add a Paycheck to your hand at the start of each of your turns" },
        },
    ]))
}

/// Stage 1's Activate: the resume's cost and wait noted, the card set aside, the stage complete. A
/// resume that is not there to send (R90) sends nothing.
fn send(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(resume) = instance_of(ctx, &TargetSpec::Chosen { index: None }) else {
        return vec![];
    };
    let turns = if resume.def_id == IKWZKI {
        LONGEST_WAIT
    } else {
        let merit = merit_of(&*ctx.state, &resume);
        roll_wait(ctx, waits_for(merit))
    };
    vec![
        note(RESUME_COST_KEY, json!(cost_now(&*ctx.state, &resume))),
        note(RESUME_WAIT_KEY, json!(turns)),
        set_aside_from_hand(SetAsideArgs::default()),
        complete_this_quest(),
    ]
}

/// Stage 3's Activate: accept, or apply again with the factor tripled.
fn answer(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    match chosen_options(ctx).first().map(String::as_str) {
        Some(ACCEPT) => vec![complete_this_quest()],
        Some(REAPPLY) => vec![
            note(REWARD_FACTOR_KEY, json!(tripled(factor_of(ctx)))),
            open_stage(UPLOAD_STAGE),
        ],
        _ => vec![],
    }
}

/// Each stage's reward, on the card's own `questCompleted` (R1854000).
fn stage_completed() -> TriggerDef {
    TriggerDef::new("stage-completed", &[GameEventType::QuestCompleted], |ctx, event| {
        match subsystems::completed_stage_of(ctx, event) {
            Some(UPLOAD_STAGE) => {
                // R1865001: a stage completed with no resume sent (R1854001) waits as a weak one.
                let noted = recalled(ctx, RESUME_WAIT_KEY).and_then(|value| value.as_i64());
                let turns = match noted {
                    Some(turns) => turns as i32,
                    None => roll_wait(ctx, &WEAK_WAITS),
                };
                vec![
                    hear_back(HearBackArgs { turns, stage: None }),
                    note(RESUME_WAIT_KEY, Value::Null),
                ]
            }
            Some(INTERVIEW_STAGE) => vec![
                return_held_to_hand(ReturnHeldArgs {
                    cost_override: Some(RESUME_PRICE),
                    upgrades: Some(param(&*ctx, "buffs") * factor_of(ctx)),
                }),
                note(RESUME_COST_KEY, Value::Null),
                next_stage(),
            ],
            Some(OFFER_STAGE) => vec![note(HIRED_KEY, json!(true))],
            _ => vec![],
        }
    })
}

/// "From your next turn on, add a Paycheck to your hand at the start of each of your turns", a
/// Paycheck for each step of the factor (R1865002).
fn paychecks(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    if recalled(ctx, HIRED_KEY) != Some(Value::Bool(true)) {
        return vec![];
    }
    (0..factor_of(ctx))
        .map(|_| {
            add_to_hand(AddToHandArgs {
                def_id: Some(PAYCHECK.to_string()),
                ..AddToHandArgs::default()
            })
        })
        .collect()
}

/// "Death: Resummon this at Stage 1" (R1865002): from the graveyard, into its own zone while it takes
/// the summon.
fn resummon(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let lane = ctx.self_.as_ref().and_then(|me| resummon_lane(&*ctx.state, me));
    vec![summon(SummonArgs {
        instance: Some(TargetSpec::SelfCard),
        lane,
        ..SummonArgs::default()
    })]
}

fn job_application() -> Script {
    Script {
        stages: stages(),
        triggers: vec![stage_completed()],
        start_of_turn: Some(hook(paychecks)),
        death: Some(hook(resummon)),
        activations: vec![
            ActivationDecl {
                id: SEND.to_string(),
                label: "Activate this to send a card from your hand in as your resume".to_string(),
                uses: ActivationUses::Count(1),
                cost: None,
                // R703: the Activate needs its resume, so an empty hand offers none.
                targets: vec![TargetDecl {
                    required: Some(true),
                    ..TargetDecl::hand(1, 1, json!({ "of": ["hand"] }))
                }],
                modes: vec![],
                can_activate: None,
                has: Some(read_hook(|a| open_now(a.self_, RESUME_GOAL))),
                run: hook(send),
            },
            ActivationDecl {
                id: ANSWER.to_string(),
                label: "Activate this to accept the offer, or to send in another application".to_string(),
                uses: ActivationUses::Count(1),
                cost: None,
                targets: vec![],
                modes: vec![ModeDecl {
                    kind: PromptKind::Mode,
                    options: vec![ACCEPT.to_string(), REAPPLY.to_string()],
                }],
                can_activate: None,
                has: Some(read_hook(|a| open_now(a.self_, OFFER_GOAL))),
                run: hook(answer),
            },
        ],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The same script: the Radiant face's Lucky 1 is its printed keyword, which `lucky_on` reads, and
    // its 10 Buffs its declared `buffs`, which `param` reads.
    let base = job_application();
    let radiant = job_application();
    CardScripts { base, radiant }
}
