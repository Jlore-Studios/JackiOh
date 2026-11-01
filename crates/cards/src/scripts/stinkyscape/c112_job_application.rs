//! SPEC §8.10 Stinkyscape 112 Job Application (§6.3 Quest stages, Hold beneath, Buff; §6.2 Death; §6.1
//! Lucky; R81, R384, R1854000–R1854004, R1865000–R1865002). Field Spell, Quest, cost 1, Legendary, a card
//! of the Stinkyscape set.
//!   Base:    "Death: Resummon this at Stage 1.
//!            Quest: Stage 1, Upload Resume. Activate this to send a card from your hand in as your
//!            resume. Reward: Hear back in 2 to 6 turns.
//!            Quest: Stage 2, The Interview. One of three, drawn at random: play a card that costs the
//!            same as your resume; play 3 cards of different costs in one turn; or take or deal 12 or
//!            more damage in one hit. Reward: Add your resume to your hand and Buff it
//!            {buffs|time|times}. It costs (0).
//!            Quest: Stage 3, The Offer. Activate this to accept the offer, or to send in another
//!            application, back to Stage 1, with every reward tripled. Reward: From your next turn on,
//!            add a Paycheck to your hand at the start of each of your turns." (buffs 5)
//!   Radiant: "Lucky 1", and the same text. (buffs 10)
//!
//! Quest stages (R1854000): Stage 1 Activate sends hand card as resume (R384, R703, R1865000, R1854001); hears back (R1854004, R1865001).
//! Stage 2 draws goal (R1854002); returns held resume at (0) buffed.
//! Stage 3 accepts or reapplies (R81, #112.1, R1865002). Death resummon (R64, R78, R1865000).
//! Radiant: Lucky 1 keeps shorter wait; buffs 10.

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
    let lucky =
        ctx.live_self().map_or(0, |me| lucky_on(&*ctx.state, me)) + luck_of(&*ctx.state, ctx.controller);
    ctx.rng.lucky(lucky, |rng| draw_wait(rng, weights), i32::min)
}

/// R1865002: the reward factor the card holds on its stay.
fn factor_of(ctx: &EffectContext<'_>) -> i32 {
    factor_in(recalled(ctx, REWARD_FACTOR_KEY).as_ref())
}

/// R1865002: the reward factor as `memory.rewardFactor` holds it, 1 before any re-application.
fn factor_in(stored: Option<&Value>) -> i32 {
    stored.and_then(Value::as_i64).map_or(BASE_FACTOR, |factor| {
        factor.clamp(i64::from(BASE_FACTOR), i64::from(REWARD_FACTOR_CAP)) as i32
    })
}

/// R2476000: on the field, to both seats, what its rewards come to now: after "Buff it 5 times" (its
/// `buffs` as it stands, R386) the Buffs the resume would get, its `buffs` times the factor; after "add
/// a Paycheck" the Paychecks a turn, the factor; and after "Add your resume" the resume held beneath
/// it, by name, or the dash. Both seats read the resume while it waits (R1865000), and a re-application
/// is a public Activate. Dying and coming back starts it over (R78).
fn preview() -> PreviewHook {
    condition_hook(|ctx: ConditionContext<'_>| -> Vec<PreviewValue> {
        if ctx.zone != ConditionZone::Field {
            return vec![];
        }
        let factor = factor_in(ctx.self_.memory.get(REWARD_FACTOR_KEY));
        let buffs = param(&ctx, "buffs");
        let times = if buffs == 1 { "time" } else { "times" };
        let resume = held_beneath(ctx.state, ctx.self_);
        vec![
            count_preview(&format!("Buff it {buffs} {times}"), buffs * factor),
            count_preview("add a Paycheck", factor),
            card_preview(ctx.state, "Add your resume", resume.first().map(|card| (card.def_id.as_str(), card.radiant))),
        ]
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
    TriggerDef::new(
        "stage-completed",
        &[GameEventType::QuestCompleted],
        |ctx, event| match subsystems::completed_stage_of(ctx, event) {
            Some(UPLOAD_STAGE) => {
                // R1865001: with no resume sent (R1854001), the weak table.
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
        },
    )
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
/// the summon. The hook reads the card as it died (R89), so it names the card by its id, which finds
/// it where it is now.
fn resummon(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(me) = ctx.self_.as_ref() else {
        return vec![];
    };
    // R1884004: a Unit that died for want of health, its text fused onto a Unit the board holds at 0
    // max health (Core #46 Suppressive Aura's −2/−2 on a 2/2), would come back to the same board and
    // die again, without end: it stays dead. It is read as it died (R89).
    if card_type_of(&*ctx.state, me) == CardType::Unit
        && unit_view_as_it_died(&*ctx.state, me).max_health <= 0
    {
        return vec![];
    }
    vec![summon(SummonArgs {
        instance: Some(TargetSpec::Instance {
            instance_id: me.id.clone(),
        }),
        lane: resummon_lane(&*ctx.state, me),
        ..SummonArgs::default()
    })]
}

fn job_application() -> Script {
    Script {
        preview: Some(preview()),
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

// S #112 Job Application — SPEC §8.10 row 112, BUILD M12 row S 112 (R1865000, R1865002).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::subsystems::quests::QuestMemory;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008";
    const SLIME: &str = "classic-027";
    const RUSHER: &str = "core-011";
    const POINTMASTER: &str = "core-020";
    const MENACE: &str = "core-019";
    const SILAS: &str = "core-052";
    const BIG: &str = "core-025";
    const BIG_LANE: i32 = 5;
    const NETHER: &str = "core-088";
    const LAG: &str = "classic-025";
    const LIBRARY: usize = 10;
    const MANA: i32 = 10;

    /// ends it (R82).
    fn setup(seed: &str, radiant: bool, extra: &[&str]) -> Scenario {
        let mut hand = vec![
            json!({ "def": ID, "radiant": radiant }),
            json!(VANILLA),
            json!(SLIME),
            json!(RUSHER),
            json!(POINTMASTER),
        ];
        hand.extend(extra.iter().map(|card| json!(card)));
        let library = vec![MENACE; LIBRARY];
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": hand,
                "field": [{ "def": BIG, "lane": BIG_LANE, "statsOverride": { "attack": 12, "health": 12 } }],
                "library": library,
                "mana": MANA,
            },
            "p2": { "hand": [MENACE], "library": library },
        }));
        s.state_mut().players.p1.auto_end_turn = Some(false);
        s.state_mut().players.p2.auto_end_turn = Some(false);
        s
    }

    fn played(s: &mut Scenario, zone: Option<i32>) -> String {
        let opts = zone.map_or(json!({}), |lane| json!({ "zone": lane }));
        s.play(ID, opts);
        s.state()
            .players
            .p1
            .backrow
            .iter()
            .flatten()
            .find(|card| card.def_id == ID)
            .expect("Job Application in P1's backrow")
            .id
            .clone()
    }

    fn line(s: &Scenario) -> QuestMemory {
        subsystems::quest_memory_of(s.card(ID)).expect("a quest line")
    }

    fn in_hand(s: &Scenario, def_id: &str) -> CardInstance {
        s.hand(P1)
            .into_iter()
            .find(|card| card.def_id == def_id)
            .unwrap_or_else(|| panic!("{def_id} in P1's hand"))
    }

    fn send(s: &mut Scenario, def_id: &str) -> CardInstance {
        let resume = in_hand(s, def_id);
        s.activate(ID, json!({ "ability": SEND, "targets": pick::id(&resume.id) }));
        resume
    }

    fn answer(s: &mut Scenario, mode: &str) {
        s.activate(ID, json!({ "ability": ANSWER, "modes": [mode] }));
    }

    fn wait(s: &Scenario) -> i32 {
        line(s).hear_back.expect("a wait to hear back").turns
    }

    fn wait_of(seed: &str, radiant: bool, resume: &str) -> i32 {
        let mut s = setup(seed, radiant, &[SILAS]);
        played(&mut s, None);
        send(&mut s, resume);
        wait(&s)
    }

    fn until_interview(s: &mut Scenario) -> i32 {
        let mut started = 0;
        for _ in 0..2 * LONGEST_WAIT {
            s.end_turn();
            if s.state().active == P1 {
                started += 1;
                let memory = line(s);
                if memory.stage == Some(INTERVIEW_STAGE) && memory.hear_back.is_none() {
                    return started;
                }
            }
        }
        panic!("the Interview never opened");
    }

    fn interview(s: &mut Scenario) {
        let big = s.unit(P1, BIG_LANE).expect("BIG").id;
        s.attack(big.as_str(), "hero");
        for card in [SLIME, RUSHER, POINTMASTER] {
            if line(s).stage == Some(OFFER_STAGE) {
                return;
            }
            s.play(card, json!({}));
        }
        assert_eq!(line(s).stage, Some(OFFER_STAGE), "the Offer is open");
    }

    fn upgraded(events: &[GameEvent], id: &str) -> usize {
        events
            .iter()
            .filter(|event| matches!(event, GameEvent::Upgraded { instance_id, .. } if instance_id == id))
            .count()
    }

    /// The Paychecks the last step gave P1, added or burned by a full hand (§2.4).
    fn paychecks_given(s: &Scenario) -> usize {
        s.last_events()
            .iter()
            .filter(|event| match event {
                GameEvent::AddedToHand { player, def_id, .. } => *player == P1 && def_id == PAYCHECK,
                GameEvent::Burned { owner, def_id, .. } => *owner == P1 && def_id == PAYCHECK,
                _ => false,
            })
            .count()
    }

    fn next_paychecks(s: &mut Scenario) -> usize {
        s.end_turn();
        assert!(paychecks_given(s) == 0, "none on the opponent's turn");
        s.end_turn();
        paychecks_given(s)
    }

    /// R2476000: on the field, to both seats: Buff it 5 times {5}, add a Paycheck {1} and Add your resume
    /// {—}; the resume sent in shows by name, and a tripled factor shows 15 Buffs and 3 Paychecks.
    #[test]
    fn r2476000_it_previews_its_rewards_and_the_resume_it_holds() {
        let mut s = setup("job-preview", false, &[]);
        let job = played(&mut s, Some(1));
        let shown = |s: &Scenario, buffs: i32, paychecks: i32, resume: (i32, &str)| {
            let want = vec![
                ("Buff it 5 times".to_string(), buffs, None),
                ("add a Paycheck".to_string(), paychecks, None),
                ("Add your resume".to_string(), resume.0, Some(resume.1.to_string())),
            ];
            assert_eq!(crate::preview_seen(s, P1, &job), want);
            assert_eq!(crate::preview_seen(s, P2, &job), want);
        };
        shown(&s, 5, 1, (0, "—"));
        send(&mut s, VANILLA);
        shown(&s, 5, 1, (1, "Mr. Vanilla"));
        remember_on_it(&mut s, "rewardFactor", json!(3));
        shown(&s, 15, 3, (1, "Mr. Vanilla"));
    }

    fn remember_on_it(s: &mut Scenario, key: &str, value: Value) {
        s.card_mut(ID).memory.insert(key.to_string(), value);
    }

    fn resume_back(seed: &str, radiant: bool, factor: Option<i32>) -> (Scenario, CardInstance, usize) {
        let mut s = setup(seed, radiant, &[]);
        played(&mut s, None);
        let resume = send(&mut s, VANILLA);
        if let Some(factor) = factor {
            remember_on_it(&mut s, REWARD_FACTOR_KEY, json!(factor));
        }
        until_interview(&mut s);
        let before = s.events().len();
        interview(&mut s);
        let buffs = upgraded(&s.events()[before..], &resume.id);
        let back = s.card(resume.id.as_str()).clone();
        assert_eq!(back.zone.z(), ZoneName::Hand, "the resume is back in hand");
        assert_eq!(cost_now(s.state(), &back), RESUME_PRICE, "it costs (0)");
        assert_eq!(s.state().held, None, "nothing held any more");
        (s, back, buffs)
    }

    #[test]
    fn declares_three_stages_and_two_once_a_turn_activates() {
        crate::register_all();
        for face in [script().base, script().radiant] {
            let names: Vec<&str> = face.stages.iter().map(|stage| stage.name.as_str()).collect();
            assert_eq!(names, ["Upload Resume", "The Interview", "The Offer"]);
            let goals: Vec<usize> = face.stages.iter().map(|stage| stage.goals.len()).collect();
            assert_eq!(goals, [1, 3, 1]);
            let ids: Vec<&str> = face.activations.iter().map(|decl| decl.id.as_str()).collect();
            assert_eq!(ids, [SEND, ANSWER]);
            assert!(
                face.activations
                    .iter()
                    .all(|decl| decl.uses == ActivationUses::Count(1))
            );
            assert!(face.death.is_some() && face.start_of_turn.is_some());
        }
    }

    #[test]
    fn entering_opens_stage_1_and_an_empty_hand_has_nothing_to_send() {
        let mut s = scenario(json!({
            "p1": { "hand": [ID], "library": [MENACE] },
            "p2": { "hand": [MENACE] },
        }));
        played(&mut s, None);
        let memory = line(&s);
        assert_eq!(
            (memory.stage, memory.active),
            (Some(UPLOAD_STAGE), vec![RESUME_GOAL.to_string()])
        );
        // R703: the Activate needs its resume, so none is offered.
        let me = s.card(ID).id.clone();
        let offered = legal_actions(s.state(), P1).into_iter().any(|action| {
            matches!(action, ActionBody::Activate { instance_id, ability, .. }
                if instance_id == me && ability.as_deref() == Some(SEND))
        });
        assert!(!offered, "no Upload Resume with an empty hand");
        s.expect_refused(|s| s.activate(ID, json!({ "ability": SEND, "targets": [] })));
    }

    #[test]
    fn with_a_card_in_hand_upload_resume_is_offered() {
        let mut s = setup("ja-offered", false, &[]);
        let me = played(&mut s, None);
        let offered = legal_actions(s.state(), P1).into_iter().any(|action| {
            matches!(action, ActionBody::Activate { instance_id, ability, .. }
                if instance_id == me && ability.as_deref() == Some(SEND))
        });
        assert!(offered);
    }

    #[test]
    fn r1865000_sending_sets_the_resume_aside_and_hears_back_in_2_to_6() {
        let mut s = setup("ja-send", false, &[]);
        let me = played(&mut s, None);
        let resume = send(&mut s, VANILLA);
        assert!(
            s.hand(P1).iter().all(|card| card.id != resume.id),
            "out of the hand"
        );
        let held = s.state().held.clone().expect("the resume held");
        assert_eq!(held.len(), 1);
        assert_eq!(
            (held[0].card.id.as_str(), held[0].holder_id.as_str()),
            (resume.id.as_str(), me.as_str())
        );
        let memory = line(&s);
        let heard = memory.hear_back.expect("a wait");
        assert!(
            (SHORTEST_WAIT..=LONGEST_WAIT).contains(&heard.turns),
            "{}",
            heard.turns
        );
        assert_eq!(heard.stage, INTERVIEW_STAGE);
        assert!(memory.active.is_empty(), "nothing open while it waits");
        assert_eq!(
            s.card(ID).memory.get(RESUME_COST_KEY),
            Some(&json!(1)),
            "the resume's cost noted"
        );
        s.expect_refused(|s| {
            s.activate(
                ID,
                json!({ "ability": SEND, "targets": pick::id(&in_hand(s, SLIME).id) }),
            )
        });
        s.expect_refused(|s| s.activate(ID, json!({ "ability": ANSWER, "modes": [ACCEPT] })));
    }

    #[test]
    fn r1865000_the_resume_reads_to_both_seats_once_sent() {
        let mut s = setup("ja-public", false, &[]);
        played(&mut s, None);
        let resume = send(&mut s, VANILLA);
        for viewer in [P1, P2] {
            let named = s.view(viewer).events.into_iter().any(|event| {
                matches!(event, GameEvent::Held { instance_id, def_id, .. }
                    if instance_id == resume.id && def_id == VANILLA)
            });
            assert!(named, "{viewer:?} reads the resume sent in");
        }
    }

    #[test]
    fn r1865001_an_ikwzki_resume_always_hears_back_in_6() {
        for seed in 0..10 {
            let mut s = setup(&format!("ja-ikwzki-{seed}"), false, &[IKWZKI]);
            played(&mut s, None);
            let cursor = s.state().rng_cursor;
            send(&mut s, IKWZKI);
            assert_eq!(wait(&s), LONGEST_WAIT);
            assert_eq!(s.state().rng_cursor, cursor, "no draw");
        }
    }

    #[test]
    fn r1865001_merit_is_rarity_cost_and_radiance() {
        let s = setup("ja-merit", false, &[SILAS, MENACE]);
        let merit = |def_id: &str| merit_of(s.state(), &in_hand(&s, def_id));
        assert_eq!(merit(SLIME), 0);
        assert_eq!(merit(VANILLA), 1);
        assert_eq!(merit(MENACE), 3);
        assert_eq!(merit(SILAS), 7, "Legendary is 3, and its cost 4");
        let mut radiant = in_hand(&s, VANILLA);
        radiant.radiant = true;
        assert_eq!(merit_of(s.state(), &radiant), 1 + RADIANT_MERIT);
        assert_eq!(waits_for(2), &WEAK_WAITS);
        assert_eq!(waits_for(SOLID_MERIT), &SOLID_WAITS);
        assert_eq!(waits_for(STRONG_MERIT - 1), &SOLID_WAITS);
        assert_eq!(waits_for(STRONG_MERIT), &STRONG_WAITS);
    }

    #[test]
    fn r1865001_a_strong_resume_waits_less_than_a_weak_one_over_the_seeds() {
        let (mut strong, mut weak) = (0, 0);
        let mut seen: IndexSet<i32> = IndexSet::new();
        for seed in 0..40 {
            let seed = format!("ja-merit-{seed}");
            for (resume, total) in [(SILAS, &mut strong), (SLIME, &mut weak)] {
                let turns = wait_of(&seed, false, resume);
                assert!((SHORTEST_WAIT..=LONGEST_WAIT).contains(&turns), "{turns}");
                seen.insert(turns);
                *total += turns;
            }
        }
        assert!(strong < weak, "strong {strong} against weak {weak}");
        assert_eq!(
            seen.len(),
            (LONGEST_WAIT - SHORTEST_WAIT + 1) as usize,
            "every wait seen: {seen:?}"
        );
    }

    #[test]
    fn r1865001_radiant_lucky_1_keeps_the_shorter_wait() {
        let mut shorter = 0;
        for seed in 0..30 {
            let seed = format!("ja-lucky-{seed}");
            let (base, radiant) = (wait_of(&seed, false, VANILLA), wait_of(&seed, true, VANILLA));
            assert!(radiant <= base, "{seed}: radiant {radiant} against base {base}");
            if radiant < base {
                shorter += 1;
            }
        }
        assert!(shorter > 0, "the second draw shortened some wait");
    }

    #[test]
    fn r1865001_the_wait_counts_its_controllers_turns_and_the_interview_opens_after_it() {
        let mut s = setup("ja-heard", false, &[]);
        played(&mut s, None);
        send(&mut s, VANILLA);
        let turns = wait(&s);
        assert_eq!(until_interview(&mut s), turns);
    }

    #[test]
    fn r1865001_the_interview_draws_each_of_its_three_goals_over_the_seeds() {
        let mut seen: IndexSet<String> = IndexSet::new();
        for seed in 0..30 {
            let mut s = setup(&format!("ja-interview-{seed}"), false, &[]);
            played(&mut s, None);
            send(&mut s, VANILLA);
            until_interview(&mut s);
            let open = line(&s).active;
            assert_eq!(open.len(), 1, "one goal drawn: {open:?}");
            seen.insert(open[0].clone());
        }
        let all: IndexSet<String> = [PROVE, BEHAVIORAL, HARDBALL]
            .into_iter()
            .map(str::to_string)
            .collect();
        assert_eq!(seen, all);
    }

    #[test]
    fn r1865001_the_interview_asks_for_the_resumes_cost() {
        let mut s = setup(
            &seed_where("ja-prove", |seed| {
                let mut s = setup(seed, false, &[]);
                played(&mut s, None);
                send(&mut s, VANILLA);
                until_interview(&mut s);
                line(&s).active == [PROVE]
            }),
            false,
            &[],
        );
        played(&mut s, None);
        send(&mut s, VANILLA);
        until_interview(&mut s);
        s.play(SLIME, json!({}));
        assert_eq!(
            line(&s).stage,
            Some(INTERVIEW_STAGE),
            "a (0) is not the resume's (1)"
        );
        s.play(RUSHER, json!({}));
        assert_eq!(line(&s).stage, Some(OFFER_STAGE), "a (1) is");
    }

    #[test]
    fn r1865001_a_stage_completed_with_no_resume_waits_as_a_weak_one_and_asks_for_a_0() {
        // R1854001: a "complete the current stage" from elsewhere (S #6 Tylannt's), as it marks the
        let seed = seed_where("ja-no-resume", |seed| {
            let mut s = setup(seed, false, &[]);
            played(&mut s, None);
            if let Some(quest) = s
                .card_mut(ID)
                .memory
                .get_mut(subsystems::quests::QUEST_MEMORY_KEY)
            {
                quest["met"] = json!([RESUME_GOAL]);
            }
            s.play(POINTMASTER, json!({}));
            until_interview(&mut s);
            line(&s).active == [PROVE]
        });
        let mut s = setup(&seed, false, &[]);
        played(&mut s, None);
        if let Some(quest) = s
            .card_mut(ID)
            .memory
            .get_mut(subsystems::quests::QUEST_MEMORY_KEY)
        {
            quest["met"] = json!([RESUME_GOAL]);
        }
        s.play(POINTMASTER, json!({}));
        assert!((SHORTEST_WAIT..=LONGEST_WAIT).contains(&wait(&s)));
        assert_eq!(s.state().held, None, "nothing sent in");
        until_interview(&mut s);
        s.play(RUSHER, json!({}));
        assert_eq!(
            line(&s).stage,
            Some(INTERVIEW_STAGE),
            "a (1) is not the (0) asked"
        );
        s.play(SLIME, json!({}));
        assert_eq!(line(&s).stage, Some(OFFER_STAGE), "a (0) is");
        assert!(
            s.hand(P1)
                .iter()
                .all(|card| card.def_id != VANILLA || card.cost_override.is_none())
        );
    }

    #[test]
    fn r1865000_the_interview_returns_the_resume_buffed_5_times_at_0() {
        let (_, _, buffs) = resume_back("ja-back", false, None);
        assert_eq!(buffs, 5);
    }

    #[test]
    fn r1865000_the_interview_returns_the_resume_radiant_buffs_it_10_times() {
        let (_, _, buffs) = resume_back("ja-back-radiant", true, None);
        assert_eq!(buffs, 10);
    }

    #[test]
    fn buffs_reads_through_param() {
        let mut s = setup("ja-param", false, &[]);
        played(&mut s, None);
        step_param(s.card_mut(ID), "buffs", 1);
        let resume = send(&mut s, VANILLA);
        until_interview(&mut s);
        let before = s.events().len();
        interview(&mut s);
        assert_eq!(upgraded(&s.events()[before..], &resume.id), 6);
    }

    #[test]
    fn r1865002_accepting_adds_a_paycheck_at_each_of_its_controllers_later_turn_starts() {
        let (mut s, _, _) = resume_back("ja-hired", false, None);
        answer(&mut s, ACCEPT);
        assert_eq!(paychecks_given(&s), 0, "none on the turn of accepting");
        assert_eq!(line(&s).active, Vec::<String>::new(), "the line is done");
        assert_eq!(next_paychecks(&mut s), 1);
        assert_eq!(next_paychecks(&mut s), 1);
        s.expect_refused(|s| s.activate(ID, json!({ "ability": ANSWER, "modes": [ACCEPT] })));
    }

    #[test]
    fn r1865002_a_radiant_job_application_adds_its_paychecks_on_their_base_face() {
        let (mut s, _, _) = resume_back("ja-paycheck", true, None);
        answer(&mut s, ACCEPT);
        assert_eq!(next_paychecks(&mut s), 1);
        let added: Vec<String> = s
            .last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand {
                    instance_id, def_id, ..
                }
                | GameEvent::Burned {
                    instance_id, def_id, ..
                } if def_id == PAYCHECK => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        for id in added {
            assert!(!s.card(id.as_str()).radiant, "{id} on its base face");
        }
    }

    #[test]
    fn r1865002_reapplying_reopens_stage_1_tripled_and_a_new_resume_may_go_in_that_turn() {
        let (mut s, _, _) = resume_back("ja-reapply", false, None);
        answer(&mut s, REAPPLY);
        let memory = line(&s);
        assert_eq!(
            (memory.stage, memory.active),
            (Some(UPLOAD_STAGE), vec![RESUME_GOAL.to_string()])
        );
        assert_eq!(
            s.card(ID).memory.get(REWARD_FACTOR_KEY),
            Some(&json!(REWARD_TRIPLING))
        );
        let resume = send(&mut s, MENACE);
        assert!((SHORTEST_WAIT..=LONGEST_WAIT).contains(&wait(&s)));
        assert_eq!(
            s.card(ID).memory.get(RESUME_COST_KEY),
            Some(&json!(3)),
            "the new resume's cost"
        );
        until_interview(&mut s);
        let before = s.events().len();
        if line(&s).active == [PROVE] {
            s.play(MENACE, json!({}));
        } else {
            interview(&mut s);
        }
        assert_eq!(line(&s).stage, Some(OFFER_STAGE));
        assert_eq!(
            upgraded(&s.events()[before..], &resume.id),
            15,
            "5 Buffs, tripled"
        );
    }

    #[test]
    fn r1865002_a_factor_of_3_buffs_15_times_and_adds_3_paychecks_a_turn() {
        let (mut s, _, buffs) = resume_back("ja-factor", false, Some(REWARD_TRIPLING));
        assert_eq!(buffs, 15);
        answer(&mut s, ACCEPT);
        assert_eq!(next_paychecks(&mut s), 3);
    }

    #[test]
    fn r1865002_the_factor_triples_up_to_9() {
        assert_eq!(tripled(BASE_FACTOR), 3);
        assert_eq!(tripled(3), 9);
        assert_eq!(tripled(REWARD_FACTOR_CAP), REWARD_FACTOR_CAP);
    }

    #[test]
    fn r1865002_destroyed_it_is_resummoned_into_its_zone_at_stage_1_and_its_resume_exiled() {
        let mut s = setup("ja-death", false, &[NETHER]);
        played(&mut s, Some(3));
        let resume = send(&mut s, VANILLA);
        remember_on_it(&mut s, REWARD_FACTOR_KEY, json!(REWARD_TRIPLING));
        s.state_mut().players.p1.mana.current = MANA;
        s.play(NETHER, json!({}));
        let back = s.backrow(P1, 3).expect("Job Application back in its zone");
        assert_eq!(back.def_id, ID);
        let memory = line(&s);
        assert_eq!(
            (memory.stage, memory.active, memory.hear_back),
            (Some(UPLOAD_STAGE), vec![RESUME_GOAL.to_string()], None)
        );
        assert_eq!(back.memory.get(REWARD_FACTOR_KEY), None, "R78: its factor reset");
        assert!(
            s.pile(P1, "exile").iter().any(|card| card.id == resume.id),
            "its resume exiled"
        );
        assert_eq!(s.state().held, None);
        assert!(s.pile(P1, "graveyard").iter().all(|card| card.def_id != ID));
    }

    #[test]
    fn r1865002_its_paychecks_stop_once_it_dies() {
        let mut s = setup("ja-fired", false, &[NETHER]);
        played(&mut s, None);
        remember_on_it(&mut s, HIRED_KEY, json!(true));
        assert_eq!(next_paychecks(&mut s), 1);
        s.state_mut().players.p1.mana.current = MANA;
        s.play(NETHER, json!({}));
        assert!(
            s.backrow(P1, 1).is_some_and(|card| card.def_id == ID),
            "resummoned"
        );
        assert_eq!(next_paychecks(&mut s), 0, "resummoned at Stage 1, not hired");
    }

    #[test]
    fn r1865002_resummon_lane_is_its_own_open_zone_else_none() {
        let mut s = setup("ja-lane", false, &[]);
        played(&mut s, Some(2));
        let me = s.card(ID).clone();
        assert_eq!(
            resummon_lane(s.state(), &me),
            None,
            "its zone is taken (by itself)"
        );
        let mut elsewhere = me.clone();
        elsewhere.zone = Zone::Field {
            player: P1,
            row: Row::Backrow,
            lane: 4,
        };
        assert_eq!(resummon_lane(s.state(), &elsewhere), Some(4));
        let mut in_hand = me;
        in_hand.zone = Zone::Hand { player: P1 };
        assert_eq!(resummon_lane(s.state(), &in_hand), None);
    }

    #[test]
    fn r1865002_an_exiled_job_application_stays_gone() {
        let mut s = setup("ja-exiled", false, &[LAG]);
        played(&mut s, None);
        let resume = send(&mut s, VANILLA);
        s.play(LAG, json!({}));
        assert!(s.pile(P1, "exile").iter().any(|card| card.def_id == ID), "exiled");
        assert!(
            s.state()
                .players
                .p1
                .backrow
                .iter()
                .flatten()
                .all(|card| card.def_id != ID),
            "not back"
        );
        assert!(
            s.pile(P1, "exile").iter().any(|card| card.id == resume.id),
            "its resume exiled too"
        );
    }
    #[test]
    fn r1884004_fused_onto_a_unit_the_board_holds_at_0_health_it_stays_dead() {
        const TAOTIE: &str = "meditative-047"; // (4) 2/2.
        const SUPPRESSIVE_AURA: &str = "core-046"; // −1/−1 to all Units, −2/−2 Embiggened.
        let library = vec![MENACE; LIBRARY];
        let mut s = scenario(json!({
            "seed": "ja-fused-at-0",
            "p1": { "library": library, "mana": MANA },
            "p2": { "backrow": [{ "def": SUPPRESSIVE_AURA, "lane": 1 }], "library": library },
        }));
        let aura = s.backrow(P2, 1).expect("the Suppressive Aura").id;
        find_instance_mut(s.state_mut(), &aura).expect("the aura").embiggened = Some(true);
        // 饕餮 and Job Application fused into p1's hand, as Craft a Card fuses (R77): a 2/2 Unit.
        let mut events: Vec<GameEvent> = vec![];
        let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
        let fused = {
            let state = s.state_mut();
            let ingredients: Vec<CardInstance> = [TAOTIE, ID]
                .iter()
                .map(|def_id| new_instance(&mut *state, def_id, P1, Zone::Gone { player: P1 }))
                .collect();
            let mut sink = EngineSink::new(state, &mut events, &mut rng);
            subsystems::fuse(
                &mut sink,
                subsystems::FuseArgs {
                    ingredients,
                    to_hand: Some(P1),
                    ..subsystems::FuseArgs::default()
                },
            )
        }
        .expect("the fused card");
        s.state_mut().rng_cursor = rng.cursor();
        s.play(&fused.id, json!({ "zone": 1 }));
        s.expect_in_zone(&fused, "graveyard");
    }
}
