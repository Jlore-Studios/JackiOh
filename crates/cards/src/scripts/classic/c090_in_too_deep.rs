//! C #90 In Too Deep (SPEC §8.6 row 90). (1) Field Spell, Quickdraw, Mythic.
//!   Base:    "Indestructible\nQuest: Draw 2 cards. Each quest you complete offers rewards; the reward
//!            you choose sets your next quest."
//!   Radiant: "Indestructible\nQuest: Draw 2 cards. Each quest you complete gives every reward it
//!            offers, and you follow every path."
//!   Engine:  "Quests (§10.1, R404): `memory.quest`, the tree data in the card file. Quest 1 opens as
//!            the card enters the field. … Completion is noticed at the state check after the event
//!            that completes it, on either player's turn, and the reward choice is a `reward` prompt
//!            (§10.6) for the card's controller then. … Radiant: each completed quest grants all of its
//!            rewards and opens all of their quests, with no reward prompt; a quest reached by two paths
//!            opens once, and a reward two completed quests offer (D, G, H) is granted by each. …
//!            Quickdraw: it starts in your opening hand (§2.1). Tunes: none."
//!
//! THE SPLIT (B5 E33). The machinery is the engine's (`subsystems/quests.ts`): it keeps the count on
//! the instance (`memory.quest`), opens quest 1 as the card enters the field, counts each event the
//! resolution loop reaches from the moment a quest opens, notices a completion at the state check and
//! reports it (`questCompleted`), and shows the open quests in both views (§10.8). The TREE below is
//! this card's data (`Script.quests`): the ten quests as R404 reads them countable, and the thirteen
//! rewards with the quest each leads to. The REWARDS are this card's verbs, run by its own trigger on
//! its own `questCompleted`.
//!
//! BASE. The trigger asks the card's controller a `reward` prompt (§10.6) over the completed quest's
//! rewards — on the other player's turn too, a non-active player's prompt (R79) — and the answer runs
//! the chosen reward and opens the quest it leads to (`resume.reward`). A quest of one reward (7–10)
//! still asks: SPEC makes the reward a prompt. RADIANT. The trigger runs every reward of the quest in
//! the tree's order, then opens every quest they lead to: the rewards are what completing the quest
//! gives, and the paths begin after them, as the base face's next quest opens after its reward. A
//! quest reached by two paths opens once (`openQuest` opens only a quest never opened), and a reward
//! two completed quests offer (D, G, H) is granted by each, since each completion runs its own list.
//! Counting from the moment a quest opens holds inside a reward too: reward H draws its 2 before quest
//! 9 opens, and those draws are not its (R404); a deck H emptied is a deck "already empty when the
//! quest opens", which completes quest 9 at once.
//!
//! THE REWARDS, each with the reading it needs:
//!   A  heal your hero 6.
//!   B  deal 3 damage to a target — any Unit or hero, either side (§8 Conventions), asked as the reward
//!      resolves; no target fizzles and the next quest still opens.
//!   C  return 2 random cards from your graveyard to your hand — two different cards (R60), all of them
//!      if fewer; a full hand burns one back once (§2.4, `returnRandomFromGraveyard`).
//!   D  place 3 Plague Counters (§6.3, R471, R689): three placements, all on the one permanent you choose,
//!      either side (`placePlagueTokens`).
//!   E  a random Unit of yours gets +3/+3 (`buffRandomUnit`, R60); none, nothing.
//!   F  bounce a target permanent — a Unit or a backrow card, either side, face-down ones and this card
//!      included; this card bounced ends its own quest line (R78), so its next quest never opens.
//!   G  your opponent discards 2 cards of their choice (R16): their own hand prompt; fewer, all they have.
//!   H  draw 2.
//!   I  Recruit a card (§6.3): the first permanent from the top of your deck.
//!   J  gain 100 mana, as next-turn mana (R540): quest 7 completes as your turn ends, so mana for "this
//!      turn" would lapse unspent; it is the next refresh's rider, a badge until your next turn.
//!   K  exile your opponent's deck: every card of it, bottom up (no draw, no fatigue).
//!   L  Aura: you may play cards from your graveyard (§6.3 Play, E11): this card's `graveyardPlay`.
//!   M  Aura: your Units have Indestructible: this card's `aura` (§10.4 layer 5); Indestructible gives
//!      no Taunt (R347).
//! L and M hold while the card stays on the field; a Tribute or an exile ends them, and leaving the
//! field resets the quest line (R78). It is Indestructible (catalog keyword): a destroy leaves it (R46).
//!
//! THE QUESTS, as R404 reads them (counted from the moment each opens):
//!   1 draws of yours, 2 (R541: every draw that took a card — burned, cast on draw or kept; a draw a
//!     limit stopped and a fatigue draw take none); 2 enemy permanents destroyed by anything, 2 (on the
//!     side they died on); 3 permanents you control at once, 3, this card included (board); 4 a turn of
//!     yours ending with 3+ unspent mana; 5 damage your cards deal to enemies, 12 (R542: the side the
//!     source and the target stood on as the hit landed); 6 your Units' total attack and total health,
//!     both 10+, at once (board); 7 a turn of yours ending with 5+ unspent mana; 8 cards entering either
//!     exile, 3; 9 a draw of yours that takes your deck's last card; 10 Units in your graveyard, 6
//!     (board).
//!
//! Rulings: R404 (the card), R540 (J's mana), R541 (what a draw is), R542 (whose damage, whose death),
//! R543 (Radiant: rewards before paths), R78, R46, R347, R60, R16. No declared numbers (Tunes: none),
//! so the amounts are this file's named constants. Its proof: `test/classic/090-in-too-deep.test.ts`
//! (the `mod tests` at the bottom of this file).

use jackioh_engine::effects::{
    bounce, buff_random_unit, choose_from_hand, choose_reward, choose_target, chosen_options, damage,
    discard, draw, exile_bottom_of_library, heal, next_turn_mana, place_plague_tokens, recruit,
    return_random_from_graveyard,
};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-090";

// The tree's numbers (SPEC §8.6 row 90; no declared params).
const QUEST_DRAWS: i32 = 2;
const QUEST_KILLS: i32 = 2;
const QUEST_PERMANENTS: i32 = 3;
const QUEST_FLOAT: i32 = 3;
const QUEST_DAMAGE: i32 = 12;
const QUEST_STATS: i32 = 10;
const QUEST_BIG_FLOAT: i32 = 5;
const QUEST_EXILES: i32 = 3;
const QUEST_GRAVE_UNITS: i32 = 6;
const REWARD_HEAL: i32 = 6;
const REWARD_DAMAGE: i32 = 3;
const REWARD_RETURN: i32 = 2;
const REWARD_PLAGUE: i32 = 3;
const REWARD_BUFF: i32 = 3;
const REWARD_DISCARD: i32 = 2;
const REWARD_DRAW: i32 = 2;
const REWARD_MANA: i32 = 100;

/// One quest of the tree (a private builder for TS's object literal).
fn quest(id: &str, text: &str, goal: QuestGoal, rewards: &[&str]) -> QuestDef {
    QuestDef {
        id: id.to_string(),
        text: text.to_string(),
        goal,
        rewards: rewards.iter().map(|reward| reward.to_string()).collect(),
    }
}

/// One reward of the tree (a private builder for TS's object literal).
fn reward(id: &str, text: &str, next: Option<&str>) -> QuestRewardDef {
    QuestRewardDef {
        id: id.to_string(),
        text: text.to_string(),
        next: next.map(str::to_string),
    }
}

/// The quest tree (SPEC §8.6 row 90, R404): what completes each quest, and where each reward leads.
/// (TS's module constant `IN_TOO_DEEP_QUESTS`; a function here, since a `QuestBook` owns its strings.)
fn in_too_deep_quests() -> QuestBook {
    QuestBook {
        first: "1".to_string(),
        quests: vec![
            quest("1", "Draw 2 cards", QuestGoal::Draws { count: QUEST_DRAWS }, &["A", "B"]),
            quest(
                "2",
                "Destroy 2 enemy permanents",
                QuestGoal::EnemyPermanentsDestroyed { count: QUEST_KILLS },
                &["C", "D"],
            ),
            quest(
                "3",
                "Control 3 or more permanents at once",
                QuestGoal::PermanentsControlled {
                    count: QUEST_PERMANENTS,
                },
                &["D", "E"],
            ),
            quest(
                "4",
                "End a turn with 3 or more unspent mana",
                QuestGoal::UnspentManaAtTurnEnd { mana: QUEST_FLOAT },
                &["F", "G"],
            ),
            quest(
                "5",
                "Your cards deal 12 damage to enemies",
                QuestGoal::DamageToEnemies {
                    amount: QUEST_DAMAGE,
                },
                &["G", "H"],
            ),
            quest(
                "6",
                "Your Units have 10 or more total Attack and 10 or more total health at once",
                QuestGoal::UnitTotals { total: QUEST_STATS },
                &["H", "I"],
            ),
            quest(
                "7",
                "End a turn with 5 or more unspent mana",
                QuestGoal::UnspentManaAtTurnEnd {
                    mana: QUEST_BIG_FLOAT,
                },
                &["J"],
            ),
            quest(
                "8",
                "3 cards enter either exile",
                QuestGoal::CardsExiled { count: QUEST_EXILES },
                &["K"],
            ),
            quest(
                "9",
                "A draw of yours takes the last card of your deck",
                QuestGoal::DeckEmptiedByDraw,
                &["L"],
            ),
            quest(
                "10",
                "Have 6 or more Units in your graveyard",
                QuestGoal::UnitsInGraveyard {
                    count: QUEST_GRAVE_UNITS,
                },
                &["M"],
            ),
        ],
        rewards: vec![
            reward("A", "Heal your hero 6", Some("2")),
            reward("B", "Deal 3 damage to a target", Some("3")),
            reward("C", "Return 2 random cards from your graveyard to your hand", Some("4")),
            reward("D", "Place 3 Plague Counters", Some("5")),
            reward("E", "A random Unit of yours gets +3/+3", Some("6")),
            reward("F", "Bounce a target permanent", Some("7")),
            reward("G", "Your opponent discards 2 cards of their choice", Some("8")),
            reward("H", "Draw 2 cards", Some("9")),
            reward("I", "Recruit a card", Some("10")),
            reward("J", "Gain 100 mana on your next turn", None),
            reward("K", "Exile your opponent's deck", None),
            reward("L", "Aura: you may play cards from your graveyard", None),
            reward("M", "Aura: your Units have Indestructible", None),
        ],
    }
}

/// The aura rewards, held on the instance while the card stands (`subsystems.holdQuestAura`).
const GRAVEYARD_AURA: &str = "L";
const INDESTRUCTIBLE_AURA: &str = "M";

// The continuations a reward that asks re-enters.
const TARGET_DAMAGE_STEP: &str = "rewardB";
const BOUNCE_STEP: &str = "rewardF";
const DISCARD_STEP: &str = "rewardG";
const REWARD_STEP: &str = "reward";

/// One reward's own effects, before the quest it leads to opens.
fn reward_effects(id: &str) -> Vec<Effect> {
    match id {
        "A" => vec![heal(json_as(json!({ "target": { "of": "selfHero" }, "amount": REWARD_HEAL })))],
        "B" => vec![choose_target(json_as(json!({
            "step": TARGET_DAMAGE_STEP,
            "scope": { "side": "any", "of": ["unit", "hero"] },
            "prompt": "Deal 3 damage",
        })))],
        "C" => vec![return_random_from_graveyard(json_as(json!({ "count": REWARD_RETURN })))],
        "D" => vec![place_plague_tokens(json_as(json!({ "count": REWARD_PLAGUE })))],
        "E" => vec![buff_random_unit(json_as(json!({ "attack": REWARD_BUFF, "health": REWARD_BUFF })))],
        "F" => vec![choose_target(json_as(json!({
            "step": BOUNCE_STEP,
            "scope": { "side": "any", "of": ["unit", "backrow"] },
            "prompt": "Bounce a permanent",
        })))],
        "G" => vec![choose_from_hand(json_as(json!({
            "of": "enemy",
            "by": "enemy",
            "count": REWARD_DISCARD,
            "step": DISCARD_STEP,
            "prompt": "In Too Deep: discard 2 cards",
        })))],
        "H" => vec![draw(json_as(json!({ "count": REWARD_DRAW })))],
        "I" => vec![recruit(Default::default())],
        "J" => vec![next_turn_mana(json_as(json!({ "amount": REWARD_MANA })))],
        "K" => {
            // "The deck" is every card of it: the bottom card, again and again, until none is left (a
            // library never holds more than LIBRARY_CAP).
            vec![exile_bottom_of_library(json_as(json!({ "player": "enemy", "count": LIBRARY_CAP })))]
        }
        GRAVEYARD_AURA | INDESTRUCTIBLE_AURA => vec![subsystems::hold_quest_aura(id)],
        _ => vec![],
    }
}

/// The quest a reward leads to, opened once (`openQuest` passes over a quest ever opened).
fn path_of(id: &str) -> Vec<Effect> {
    let book = in_too_deep_quests();
    let next = subsystems::quest_reward_of(&book, id).and_then(|reward| reward.next.clone());
    match next {
        None => vec![],
        Some(next) => vec![subsystems::open_quest(next)],
    }
}

/// The quest this card's own `questCompleted` names, or `None` for another card's report.
fn completed_here(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<QuestDef> {
    let GameEvent::QuestCompleted {
        instance_id, quest, ..
    } = event
    else {
        return None;
    };
    if ctx.self_.as_ref().map(|card| card.id.as_str()) != Some(instance_id.as_str()) {
        return None;
    }
    let book = in_too_deep_quests();
    subsystems::quest_def_of(&book, quest).cloned()
}

/// The reward prompt's options: each reward's id under its caption (TS `{ id, label }[]`).
fn reward_options(quest: &QuestDef) -> Vec<Value> {
    let book = in_too_deep_quests();
    quest
        .rewards
        .iter()
        .map(|id| {
            let label = subsystems::quest_reward_of(&book, id)
                .map(|reward| reward.text.clone())
                .unwrap_or_else(|| id.clone());
            json!({ "id": id, "label": label })
        })
        .collect()
}

fn quest_completed(radiant: bool) -> TriggerDef {
    TriggerDef::new(
        "quest-completed",
        &[GameEventType::QuestCompleted],
        move |ctx, event| {
            let Some(quest) = completed_here(ctx, event) else {
                return vec![];
            };
            // R543: every reward, in the tree's order, then every path they lead to.
            if radiant {
                let mut effects: Vec<Effect> = quest.rewards.iter().flat_map(|id| reward_effects(id)).collect();
                effects.extend(quest.rewards.iter().flat_map(|id| path_of(id)));
                return effects;
            }
            vec![choose_reward(json_as(json!({
                "step": REWARD_STEP,
                "rewards": reward_options(&quest),
                "prompt": format!("Quest complete: {}", quest.text),
            })))]
        },
    )
}

fn in_too_deep(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        quests: Some(in_too_deep_quests()),
        triggers: vec![quest_completed(radiant)],
        resume: IndexMap::from([
            // The base face's chosen reward, then its path.
            (
                REWARD_STEP,
                hook(|ctx| match chosen_options(ctx).into_iter().next() {
                    None => vec![],
                    Some(id) => {
                        let mut effects = reward_effects(&id);
                        effects.extend(path_of(&id));
                        effects
                    }
                }),
            ),
            (
                TARGET_DAMAGE_STEP,
                hook(|_ctx| vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": REWARD_DAMAGE })))]),
            ),
            (
                BOUNCE_STEP,
                hook(|_ctx| vec![bounce(json_as(json!({ "target": { "of": "chosen" } })))]),
            ),
            (
                DISCARD_STEP,
                hook(|ctx| {
                    (0..ctx.targets.len())
                        .map(|index| discard(json_as(json!({ "target": { "of": "chosen", "index": index } }))))
                        .collect()
                }),
            ),
        ]),
        // Reward M: your Units have Indestructible while this stands.
        aura: Some(aura_hook(|a| {
            if subsystems::held_quest_auras(a.self_).iter().any(|id| id == INDESTRUCTIBLE_AURA) {
                let controller = a.self_.controller;
                vec![AuraEntry {
                    applies: Box::new(move |unit: &CardInstance| unit.controller == controller),
                    mod_: StatMod {
                        keywords: Some(vec![Keyword::Indestructible]),
                        ..StatMod::default()
                    },
                }]
            } else {
                vec![]
            }
        })),
        // Reward L: you may play cards from your graveyard while this stands (E11, R454).
        graveyard_play: Some(read_hook(|a| {
            if subsystems::held_quest_auras(a.self_).iter().any(|id| id == GRAVEYARD_AURA) {
                vec![GraveyardPlayPermission::default()]
            } else {
                vec![]
            }
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: in_too_deep(false),
        radiant: in_too_deep(true),
    }
}

// C #90 In Too Deep (SPEC §8.6 row 90; BUILD M9 row C 90; R404, R540–R543). (1) Field Spell,
// Quickdraw, Mythic: Indestructible, and a quest line — ten quests, thirteen rewards, a `reward`
// prompt on the base face, every reward and every path on the Radiant face. Quest 1's text lives in
// the quest line, not the card text: it appears on the card face only after the card is played.
//
// The quest machinery is the engine's (`subsystems/quests.ts`, proved alone by
// `packages/engine/test/quests.test.ts`); this file proves the card: its tree, each of its ten quests
// as R404 reads them, each of its thirteen rewards and the quest it leads to, both faces, both views,
// a pause through JSON and a game folded from its log.
//
// Reaching a deep quest by playing the whole path before it would make each test the length of the
// tree, so a test of quest N writes the quest line the path would have left (`line`), as the Heroic
// Power tests write the power its arrival rolls: the path itself is proved by the reward tests, each
// of which checks the quest it opens. A reward test completes its quest by writing its count at the
// goal and letting the next state check notice it (`completeAtNextCheck`), the counting being each
// quest's own test.
#[cfg(test)]
mod tests {
    use super::script;
    use jackioh_engine::testkit::*;
    use std::collections::BTreeMap;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const ITD: &str = "classic-090";

    /// Spell 1: draw 2, heal your hero 2
    const STOCKPILE: &str = "core-005";
    /// Spell 1: draw 1
    const NOTEBOOK: &str = "core-051-1";
    /// Spell 3: destroy target Unit
    const HIT_JOB: &str = "core-016";
    /// Spell 4: destroy all permanents
    const NETHER: &str = "core-088";
    /// Spell 1: 3 damage to a target
    const LUNAR: &str = "core-035";
    /// Spell 1: Pierce, 4 damage, exile this
    const TRUE_STRIKE: &str = "core-044";
    /// Spell 4: exile target permanent and a random card of the enemy deck
    const COLLATERAL: &str = "core-034";
    /// Spell 1: destroy target backrow card, Lock its zone
    const MAGIC_JAMMED: &str = "core-036";
    /// Spell 2: exile 7 random cards from your deck
    const EUGENICS: &str = "core-042";
    /// 4/4
    const VANILLA: &str = "core-008";
    /// 3/3 Rush, First Strike
    const TIMMY: &str = "core-011";
    /// 9/9 Taunt
    const MENACE: &str = "core-019";
    /// 1/1, Cry: summon a Rush Token
    const TOKEN_MAKER: &str = "core-015";
    /// cast on draw: take 1 damage
    const VIRUS: &str = "core-090-1";
    const RUSH_TOKEN: &str = "core-t-rush";

    /// The cards a side holds so §2.5 never ends a turn on its own (TS `SPARE`).
    const SPARE_HAND: [&str; 3] = [NOTEBOOK, STOCKPILE, TIMMY];
    const SPARE_LIBRARY: [&str; 4] = [VANILLA, VANILLA, VANILLA, VANILLA];
    /// Spell 4: steal target enemy permanent
    const MIND_CONTROL: &str = "core-049";
    /// Field Spell 3
    const MANA_WELL: &str = "core-006";
    /// Field Spell 2
    const GIFTED: &str = "core-064";
    /// Trap: fuse the opponent's played permanent onto one of yours of its type
    const EXPERIMENT: &str = "core-085";
    /// Field Spell: Indestructible; Activate: Tribute this
    const LOCKDOWN: &str = "classic-084";

    /// TS's object spread: `extra`'s keys written over `target`'s.
    fn merge_into(target: &mut Value, extra: Value) {
        if let (Some(target), Value::Object(extra)) = (target.as_object_mut(), extra) {
            for (key, value) in extra {
                target.insert(key, value);
            }
        }
    }

    /// TS `{ …extra, ...SPARE }` (and `{ ...SPARE, …extra }`): no side setup here names a key SPARE has
    /// as well, so the order of the spread does not matter.
    fn spare(extra: Value) -> Value {
        let mut side = json!({ "hand": SPARE_HAND, "library": SPARE_LIBRARY });
        merge_into(&mut side, extra);
        side
    }

    /// TS `[ITD, ...(SPARE.hand ?? [])]`.
    fn itd_and_spare_hand() -> Vec<&'static str> {
        let mut hand = vec![ITD];
        hand.extend(SPARE_HAND);
        hand
    }

    fn line(s: &Scenario) -> Option<subsystems::QuestMemory> {
        subsystems::quest_memory_of(s.card(ITD))
    }

    /// The quest line as JSON, for TS's `toEqual` and `toMatchObject` on it.
    fn line_json(memory: &subsystems::QuestMemory) -> Value {
        json!({
            "active": memory.active,
            "progress": memory.progress,
            "done": memory.done,
            "auras": memory.auras,
            "waiting": memory.waiting,
        })
    }

    fn must<T>(value: Option<T>, what: &str) -> T {
        match value {
            Some(value) => value,
            None => panic!("expected {what}"),
        }
    }

    /// TS `toMatchObject`: every key of `pattern` is in `actual` with a matching value (objects match
    /// recursively and may hold more keys; arrays match element by element and are as long).
    fn matches_object(actual: &Value, pattern: &Value) -> bool {
        match (actual, pattern) {
            (Value::Object(actual), Value::Object(pattern)) => pattern
                .iter()
                .all(|(key, want)| actual.get(key).is_some_and(|got| matches_object(got, want))),
            (Value::Array(actual), Value::Array(pattern)) => {
                actual.len() == pattern.len()
                    && actual.iter().zip(pattern).all(|(got, want)| matches_object(got, want))
            }
            _ => actual == pattern,
        }
    }

    /// TS `expect(line(s)).toMatchObject(pattern)`.
    fn expect_line_matches(s: &Scenario, pattern: Value) {
        let memory = line_json(&must(line(s), "a quest line"));
        assert!(matches_object(&memory, &pattern), "quest line {memory} does not match {pattern}");
    }

    /// TS `expect(line(s)).toEqual(want)`.
    fn expect_line_equals(s: &Scenario, want: Value) {
        assert_eq!(line_json(&must(line(s), "a quest line")), want);
    }

    /// TS `line(s)?.progress`.
    fn progress_of(s: &Scenario) -> Value {
        json!(must(line(s), "a quest line").progress)
    }

    /// TS `line(s)?.active`.
    fn active_of(s: &Scenario) -> Vec<String> {
        must(line(s), "a quest line").active
    }

    /// Write the quest line a path through the tree would have left (see the header).
    fn set_line(s: &mut Scenario, memory: Value) {
        let mut line = json!({ "active": [], "progress": {}, "done": [], "auras": [], "waiting": [] });
        merge_into(&mut line, memory);
        let id = s.card(ITD).id.clone();
        find_instance_mut(s.state_mut(), &id)
            .expect("In Too Deep in the state")
            .memory
            .insert(subsystems::QUEST_MEMORY_KEY.to_string(), line);
    }

    /// Quest `id` open with its count at `progress` (TS's defaults: `progress = 0`, `extra = {}`).
    fn on_quest(s: &mut Scenario, id: &str, progress: i32, extra: Value) {
        let mut memory = json!({ "active": [id], "progress": { id: progress } });
        merge_into(&mut memory, extra);
        set_line(s, memory);
    }

    /// The Notebook's draw on a deck of spares: one action, so one state check after it.
    fn any_action(s: &mut Scenario) {
        s.play(NOTEBOOK, json!({}));
    }

    /// The In Too Deep card view as `viewer` is shown it.
    fn shown(s: &Scenario, viewer: PlayerId) -> PublicBackrowView {
        let view = s.view(viewer);
        let card = s.card(ITD);
        let side = if view.you.player == card.controller {
            &view.you
        } else {
            &view.opponent
        };
        for entry in side.backrow.iter().flatten() {
            if let BackrowView::Public(entry) = entry {
                if entry.instance_id == card.id {
                    return entry.clone();
                }
            }
        }
        panic!("{viewer} is not shown In Too Deep");
    }

    /// TS `shown(s, viewer).quest?.open.map((quest) => quest.id)`.
    fn open_ids(s: &Scenario, viewer: PlayerId) -> Option<Vec<String>> {
        shown(s, viewer)
            .quest
            .map(|quest| quest.open.iter().map(|open| open.id.clone()).collect())
    }

    /// TS `shown(s, viewer).quest?.open[0]`, as JSON (null when absent).
    fn first_open(s: &Scenario, viewer: PlayerId) -> Value {
        json!(shown(s, viewer).quest.and_then(|quest| quest.open.first().cloned()))
    }

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|id| id.to_string()).collect()
    }

    fn completed_in(events: &[GameEvent]) -> Vec<String> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::QuestCompleted { quest, .. } => Some(quest.clone()),
                _ => None,
            })
            .collect()
    }

    fn reward_keys(s: &Scenario, player: PlayerId) -> Vec<String> {
        let pending = must(s.state().pending.clone(), "a reward prompt");
        assert_eq!(pending.kind, PromptKind::Reward);
        assert_eq!(pending.player_id, player);
        pending.options.iter().map(|option| option.key.clone()).collect()
    }

    fn pending_kind(s: &Scenario) -> Option<PromptKind> {
        s.state().pending.as_ref().map(|pending| pending.kind)
    }

    /// `{ pick: "instance", instanceId }`.
    fn instance_pick(instance_id: &str) -> Value {
        json!({ "pick": "instance", "instanceId": instance_id })
    }

    /// The first `destroyed` event's owner and controller.
    fn destroyed_sides(events: &[GameEvent]) -> Option<(PlayerId, PlayerId)> {
        events.iter().find_map(|event| match event {
            GameEvent::Destroyed { owner, controller, .. } => Some((*owner, *controller)),
            _ => None,
        })
    }

    /// Whether `legalActions` offers `player` a play of the card `instance_id`.
    fn offers_play_of(s: &Scenario, player: PlayerId, instance_id: &str) -> bool {
        legal_actions(s.state(), player)
            .iter()
            .any(|action| matches!(action, ActionBody::Play { instance_id: id, .. } if id == instance_id))
    }

    fn prompt_opened_for_a_reward(events: &[GameEvent]) -> bool {
        events.iter().any(|event| {
            matches!(
                event,
                GameEvent::PromptOpened {
                    kind: PromptKind::Reward,
                    ..
                }
            )
        })
    }

    mod c_n90_in_too_deep {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn s2_1_quickdraw_both_faces_carry_the_flag_that_starts_it_in_the_opening_hand() {
                crate::register_all();
                let s = scenario(json!({ "p1": { "library": [ITD, { "def": ITD, "radiant": true }] } }));
                let library = s.pile(P1, "library");
                let plain = must(library.first().cloned(), "base");
                let shiny = must(library.get(1).cloned(), "radiant");
                assert_eq!(jackioh_engine::scripts::flags_of(s.state(), &plain).quickdraw, Some(true));
                assert_eq!(jackioh_engine::scripts::flags_of(s.state(), &shiny).quickdraw, Some(true));
            }

            #[test]
            fn r404_the_quest_tree_is_data_in_the_card_file_ten_quests_their_rewards_and_where_each_reward_leads() {
                crate::register_all();
                let scripts = script();
                for face in [&scripts.base, &scripts.radiant] {
                    let book = must(face.quests.clone(), "the tree");
                    assert_eq!(book.first, "1");
                    let rewards_of: BTreeMap<String, String> = book
                        .quests
                        .iter()
                        .map(|quest| (quest.id.clone(), quest.rewards.concat()))
                        .collect();
                    let want: BTreeMap<String, String> = [
                        ("1", "AB"),
                        ("2", "CD"),
                        ("3", "DE"),
                        ("4", "FG"),
                        ("5", "GH"),
                        ("6", "HI"),
                        ("7", "J"),
                        ("8", "K"),
                        ("9", "L"),
                        ("10", "M"),
                    ]
                    .iter()
                    .map(|(id, rewards)| (id.to_string(), rewards.to_string()))
                    .collect();
                    assert_eq!(rewards_of, want);
                    let next_of: BTreeMap<String, Option<String>> = book
                        .rewards
                        .iter()
                        .map(|reward| (reward.id.clone(), reward.next.clone()))
                        .collect();
                    let want: BTreeMap<String, Option<String>> = [
                        ("A", Some("2")),
                        ("B", Some("3")),
                        ("C", Some("4")),
                        ("D", Some("5")),
                        ("E", Some("6")),
                        ("F", Some("7")),
                        ("G", Some("8")),
                        ("H", Some("9")),
                        ("I", Some("10")),
                        ("J", None),
                        ("K", None),
                        ("L", None),
                        ("M", None),
                    ]
                    .iter()
                    .map(|(id, next)| (id.to_string(), next.map(str::to_string)))
                    .collect();
                    assert_eq!(next_of, want);
                }
            }

            #[test]
            fn r46_indestructible_a_destroy_leaves_it_in_its_zone_with_its_quest_line() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": spare(json!({ "backrow": [ITD] })),
                    "p2": { "hand": [MAGIC_JAMMED, STOCKPILE] },
                    "active": "p2",
                }));
                on_quest(&mut s, "2", 1, json!({}));
                let itd = s.card(ITD).id.clone();
                s.play(MAGIC_JAMMED, json!({ "targets": [instance_pick(&itd)] }));
                s.expect_in_zone(ITD, "field");
                expect_line_matches(&s, json!({ "active": ["2"], "progress": { "2": 1 } }));
            }

            #[test]
            fn quest_1_lives_in_the_quest_line_not_the_card_text_the_static_face_names_no_quest() {
                crate::register_all();
                // Balance patch 1: the first quest appears on the card face only after it is played.
                let def = crate::card_def(ITD);
                assert!(!def.base.text.contains("Quest:"));
                assert!(!def.radiant.text.contains("Quest:"));
                let mut s = scenario(json!({
                    "p1": { "hand": itd_and_spare_hand(), "library": SPARE_LIBRARY },
                    "p2": spare(json!({})),
                }));
                s.play(ITD, json!({}));
                assert_eq!(open_ids(&s, P1), Some(ids(&["1"])));
                assert_eq!(open_ids(&s, P2), Some(ids(&["1"])));
            }

            #[test]
            fn r404_quest_1_opens_as_it_enters_0_of_2_and_both_views_show_it_with_rewards_a_and_b_on_offer() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": itd_and_spare_hand(), "library": SPARE_LIBRARY },
                    "p2": spare(json!({})),
                }));
                s.play(ITD, json!({}));
                expect_line_equals(
                    &s,
                    json!({ "active": ["1"], "progress": { "1": 0 }, "done": [], "auras": [], "waiting": [] }),
                );
                for viewer in [P1, P2] {
                    assert_eq!(
                        json!(shown(&s, viewer).quest),
                        json!({
                            "open": [
                                {
                                    "id": "1",
                                    "text": "Draw 2 cards",
                                    "progress": 0,
                                    "goal": 2,
                                    "rewards": [
                                        { "id": "A", "text": "Heal your hero 6" },
                                        { "id": "B", "text": "Deal 3 damage to a target" },
                                    ],
                                },
                            ],
                            "auras": [],
                        })
                    );
                    // `questProgressed` names the public card in both views.
                    let reports: Vec<String> = s
                        .view(viewer)
                        .events
                        .iter()
                        .filter_map(|event| match event {
                            GameEvent::QuestProgressed { instance_id, .. } => Some(instance_id.clone()),
                            _ => None,
                        })
                        .collect();
                    assert_eq!(reports, vec![s.card(ITD).id.clone()]);
                }
            }

            #[test]
            fn r404_counted_from_the_moment_it_opens_draws_before_it_entered_do_not_count_two_after_it_complete_quest_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [STOCKPILE, ITD, STOCKPILE, TIMMY],
                        "library": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA],
                    },
                    "p2": spare(json!({})),
                }));
                s.play(STOCKPILE, json!({}));
                s.play(ITD, json!({}));
                assert_eq!(progress_of(&s), json!({ "1": 0 }));
                s.play(STOCKPILE, json!({}));
                assert_eq!(completed_in(s.last_events()), vec!["1"]);
                assert_eq!(reward_keys(&s, P1), vec!["mode:A", "mode:B"]);
                let labels: Vec<String> = must(s.state().pending.clone(), "the prompt")
                    .options
                    .iter()
                    .map(|option| option.label.clone())
                    .collect();
                assert_eq!(labels, vec!["Heal your hero 6", "Deal 3 damage to a target"]);
                // `legalActions` offers the two answers to the prompt's holder and none to the other seat.
                let answers: Vec<Vec<Selection>> = legal_actions(s.state(), P1)
                    .into_iter()
                    .filter_map(|action| match action {
                        ActionBody::Answer { selection, .. } => Some(selection),
                        _ => None,
                    })
                    .collect();
                assert_eq!(
                    answers,
                    vec![
                        vec![Selection::Mode { option: "A".to_string() }],
                        vec![Selection::Mode { option: "B".to_string() }],
                    ]
                );
                assert!(
                    !legal_actions(s.state(), P2)
                        .iter()
                        .any(|action| matches!(action, ActionBody::Answer { .. }))
                );
                expect_line_matches(&s, json!({ "active": [], "done": ["1"] }));
            }

            #[test]
            fn r404_the_opponents_draws_count_for_nothing_of_yours() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": spare(json!({ "backrow": [ITD] })), "p2": spare(json!({})) }));
                s.end_turn();
                assert_eq!(s.state().active, P2);
                assert!(
                    s.events()
                        .iter()
                        .any(|event| matches!(event, GameEvent::Drawn { player: PlayerId::P2, .. }))
                );
                assert_eq!(progress_of(&s), json!({ "1": 0 }));
            }

            #[test]
            fn r541_a_draw_burned_at_the_hand_cap_and_a_draw_cast_on_draw_are_draws_of_yours() {
                crate::register_all();
                // Eleven cards: the Notebook leaves ten, and its draw comes to a full hand (§2.4, R4).
                let mut hand = vec![NOTEBOOK];
                hand.extend([VANILLA; 10]);
                let mut burn = scenario(json!({
                    "p1": { "backrow": [ITD], "hand": hand, "library": [VANILLA] },
                    "p2": spare(json!({})),
                }));
                burn.play(NOTEBOOK, json!({}));
                assert!(
                    burn.last_events()
                        .iter()
                        .any(|event| matches!(event, GameEvent::Burned { .. }))
                );
                assert_eq!(progress_of(&burn), json!({ "1": 1 }));

                let mut cast = scenario(json!({
                    "p1": { "backrow": [ITD], "hand": [NOTEBOOK, TIMMY], "library": [VIRUS, VANILLA, VANILLA] },
                    "p2": spare(json!({})),
                }));
                cast.play(NOTEBOOK, json!({}));
                // The virus is drawn and cast (R58), and the draw goes on: two draws of yours.
                assert_eq!(
                    cast.last_events()
                        .iter()
                        .filter(|event| matches!(event, GameEvent::Drawn { player: PlayerId::P1, .. }))
                        .count(),
                    2
                );
                assert_eq!(completed_in(cast.last_events()), vec!["1"]);
            }

            #[test]
            fn r404_reward_a_heal_your_hero_6_then_quest_2_opens() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": spare(json!({ "backrow": [ITD], "health": 20 })),
                    "p2": spare(json!({})),
                }));
                s.play(STOCKPILE, json!({}));
                // Stockpile healed 2; A heals 6.
                s.answer(json!("A"));
                s.expect_health(P1, 28);
                expect_line_matches(&s, json!({ "active": ["2"], "done": ["1"] }));
                let open: Vec<Value> = must(shown(&s, P2).quest, "the quest view")
                    .open
                    .iter()
                    .map(|quest| json!([quest.id, quest.progress, quest.goal]))
                    .collect();
                assert_eq!(open, vec![json!(["2", 0, 2])]);
            }

            #[test]
            fn r404_reward_b_deal_3_damage_to_a_target_asked_as_it_resolves_then_quest_3_opens() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": spare(json!({ "backrow": [ITD] })),
                    "p2": spare(json!({ "field": [VANILLA] })),
                }));
                s.play(STOCKPILE, json!({}));
                s.answer(json!("B"));
                assert_eq!(pending_kind(&s), Some(PromptKind::Target));
                // Quest 3 waits for the reward to finish.
                assert!(active_of(&s).is_empty());
                let foe = must(s.unit(P2, 1), "the enemy unit");
                s.answer(json!(foe.id));
                assert_eq!(s.card(&foe).damage, 3);
                expect_line_matches(&s, json!({ "active": ["3"], "done": ["1"] }));
            }

            #[test]
            fn r404_quest_2_two_enemy_permanents_destroyed_by_anything_your_own_deaths_count_for_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "backrow": [ITD],
                        "field": [TIMMY],
                        "hand": [HIT_JOB, NETHER, STOCKPILE],
                        "library": SPARE_LIBRARY,
                        "mana": 10,
                    },
                    "p2": spare(json!({ "field": [VANILLA, { "def": RUSH_TOKEN }] })),
                }));
                on_quest(&mut s, "2", 0, json!({}));
                let timmy = must(s.unit(P1, 1), "Timmy");
                s.play(HIT_JOB, json!({ "targets": [instance_pick(&timmy.id)] }));
                assert_eq!(progress_of(&s), json!({ "2": 0 }));
                // Twisting Nether destroys both enemy permanents (a token among them) and leaves In Too Deep.
                s.play(NETHER, json!({}));
                s.expect_in_zone(ITD, "field");
                assert_eq!(completed_in(s.last_events()), vec!["2"]);
                assert_eq!(reward_keys(&s, P1), vec!["mode:C", "mode:D"]);
            }

            #[test]
            fn r542_quest_2_counts_the_side_a_permanent_died_on_an_enemy_unit_you_have_stolen_is_yours_as_it_dies() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "backrow": [ITD],
                        "hand": [MIND_CONTROL, HIT_JOB, HIT_JOB, STOCKPILE],
                        "library": SPARE_LIBRARY,
                        "mana": 10,
                    },
                    "p2": spare(json!({ "field": [VANILLA, TIMMY] })),
                }));
                on_quest(&mut s, "2", 0, json!({}));
                let vanilla = must(s.unit(P2, 1), "Vanilla");
                s.play(MIND_CONTROL, json!({ "targets": [instance_pick(&vanilla.id)] }));
                assert_eq!(s.card(&vanilla).controller, P1);
                s.play(HIT_JOB, json!({ "targets": [instance_pick(&vanilla.id)] }));
                assert_eq!(destroyed_sides(s.last_events()), Some((P2, P1)));
                assert_eq!(progress_of(&s), json!({ "2": 0 }));
                let timmy = must(s.unit(P2, 2), "Timmy");
                s.play(HIT_JOB, json!({ "targets": [instance_pick(&timmy.id)] }));
                assert_eq!(destroyed_sides(s.last_events()), Some((P2, P2)));
                assert_eq!(progress_of(&s), json!({ "2": 1 }));
            }

            #[test]
            fn r404_reward_c_2_different_random_cards_from_your_graveyard_to_your_hand_then_quest_4() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": spare(json!({ "backrow": [ITD], "graveyard": [VANILLA, TIMMY, MENACE] })),
                    "p2": spare(json!({})),
                }));
                on_quest(&mut s, "2", 2, json!({}));
                any_action(&mut s);
                // The Notebook has drawn and gone to the graveyard: four cards lie there, three in hand.
                assert_eq!([s.hand(P1).len(), s.pile(P1, "graveyard").len()], [3, 4]);
                s.answer(json!("C"));
                let back: IndexSet<String> = s
                    .last_events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::AddedToHand { instance_id, .. } => Some(instance_id.clone()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(back.len(), 2);
                assert_eq!([s.hand(P1).len(), s.pile(P1, "graveyard").len()], [5, 2]);
                expect_line_matches(&s, json!({ "active": ["4"], "done": ["2"] }));
            }

            #[test]
            fn r60_reward_c_with_one_card_in_the_graveyard_returns_that_one() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": spare(json!({ "backrow": [ITD], "graveyard": [MENACE] })),
                    "p2": spare(json!({})),
                }));
                on_quest(&mut s, "2", 2, json!({}));
                any_action(&mut s);
                // The Notebook itself reached the graveyard before the reward: two cards lie there now.
                s.answer(json!("C"));
                let hand: Vec<String> = s.hand(P1).iter().map(|card| card.def_id.clone()).collect();
                assert!(hand.iter().any(|id| id == MENACE));
                assert!(hand.iter().any(|id| id == NOTEBOOK));
                assert!(s.pile(P1, "graveyard").is_empty());
            }

            #[test]
            fn r471_r689_reward_d_three_plague_counter_placements_on_the_one_permanent_a_single_prompt_of_yours_names_then_quest_5() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": spare(json!({ "backrow": [ITD], "field": [VANILLA] })),
                    "p2": spare(json!({ "field": [MENACE] })),
                }));
                on_quest(&mut s, "2", 2, json!({}));
                any_action(&mut s);
                s.answer(json!("D"));
                let foe = must(s.unit(P2, 1), "Menace");
                assert_eq!(pending_kind(&s), Some(PromptKind::Target));
                assert!(active_of(&s).is_empty());
                // One answer puts all three on the pick: no second prompt opens.
                s.answer(json!(foe.id));
                assert_eq!(s.card(&foe).counters.plague, Some(3));
                expect_line_matches(&s, json!({ "active": ["5"], "done": ["2"] }));
            }

            #[test]
            fn r404_quest_3_3_or_more_permanents_of_yours_at_once_this_card_included_the_opponents_do_not_count() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "backrow": [ITD], "field": [TIMMY], "hand": [VANILLA, STOCKPILE], "library": SPARE_LIBRARY },
                    "p2": spare(json!({ "field": [MENACE, VANILLA] })),
                }));
                on_quest(&mut s, "3", 0, json!({}));
                assert!(matches_object(
                    &first_open(&s, P1),
                    &json!({ "id": "3", "progress": 2, "goal": 3 })
                ));
                s.play(VANILLA, json!({}));
                assert_eq!(completed_in(s.last_events()), vec!["3"]);
                assert_eq!(reward_keys(&s, P1), vec!["mode:D", "mode:E"]);
            }

            #[test]
            fn r60_reward_e_a_random_unit_of_yours_gets_3_3_then_quest_6_with_none_nothing_and_quest_6_still_opens() {
                crate::register_all();
                // Two small Units (their totals stay under quest 6's 10 after the +3/+3) and In Too Deep.
                let mut s = scenario(json!({
                    "p1": spare(json!({ "backrow": [ITD], "field": [TIMMY, TOKEN_MAKER] })),
                    "p2": spare(json!({ "field": [MENACE] })),
                }));
                on_quest(&mut s, "3", 0, json!({}));
                any_action(&mut s);
                s.answer(json!("E"));
                let buffed: Vec<GameEvent> = s
                    .last_events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::Buffed { .. }))
                    .cloned()
                    .collect();
                assert_eq!(buffed.len(), 1);
                assert!(matches_object(&json!(buffed[0]), &json!({ "attack": 3, "health": 3 })));
                let ours = [must(s.unit(P1, 1), "a").id, must(s.unit(P1, 2), "b").id];
                let buffed_id = match &buffed[0] {
                    GameEvent::Buffed { instance_id, .. } => instance_id.clone(),
                    _ => String::new(),
                };
                assert!(ours.contains(&buffed_id));
                expect_line_matches(&s, json!({ "active": ["6"], "done": ["3"] }));

                // No Unit of yours: three backrow permanents complete quest 3, and E buffs nothing.
                let mut none = scenario(json!({
                    "p1": spare(json!({ "backrow": [ITD, MANA_WELL, GIFTED] })),
                    "p2": spare(json!({ "field": [MENACE] })),
                }));
                on_quest(&mut none, "3", 0, json!({}));
                any_action(&mut none);
                none.answer(json!("E"));
                assert!(
                    !none
                        .last_events()
                        .iter()
                        .any(|event| matches!(event, GameEvent::Buffed { .. }))
                );
                expect_line_matches(&none, json!({ "active": ["6"], "done": ["3"] }));
            }

            #[test]
            fn r404_quest_4_a_turn_of_yours_ending_with_3_or_more_unspent_mana_2_is_not_enough_and_the_opponents_turn_is_not_yours() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": spare(json!({ "backrow": [ITD], "mana": 3 })),
                    "p2": spare(json!({})),
                }));
                on_quest(&mut s, "4", 0, json!({}));
                s.end_turn();
                assert_eq!(completed_in(s.last_events()), vec!["4"]);
                assert_eq!(reward_keys(&s, P1), vec!["mode:F", "mode:G"]);

                let mut short = scenario(json!({
                    "p1": spare(json!({ "backrow": [ITD], "mana": 2 })),
                    "p2": spare(json!({})),
                }));
                on_quest(&mut short, "4", 0, json!({}));
                short.end_turn();
                assert!(short.state().pending.is_none());
                assert_eq!(short.state().active, P2);
                short.state_mut().players.p2.mana.current = 4;
                short.end_turn();
                assert_eq!(progress_of(&short), json!({ "4": 0 }));
            }

            #[test]
            fn r404_reward_f_bounce_a_target_permanent_then_quest_7() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": spare(json!({ "backrow": [ITD] })),
                    "p2": spare(json!({ "field": [MENACE] })),
                }));
                on_quest(&mut s, "4", 1, json!({}));
                any_action(&mut s);
                s.answer(json!("F"));
                assert_eq!(pending_kind(&s), Some(PromptKind::Target));
                let foe = must(s.unit(P2, 1), "Menace");
                s.answer(json!(foe.id));
                s.expect_in_zone(&foe, "hand");
                expect_line_matches(&s, json!({ "active": ["7"], "done": ["4"] }));
            }

            #[test]
            fn r78_reward_f_on_in_too_deep_itself_it_goes_to_your_hand_its_quest_line_ends_and_played_again_it_starts_at_quest_1() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": spare(json!({ "backrow": [ITD] })), "p2": spare(json!({})) }));
                on_quest(&mut s, "4", 1, json!({}));
                any_action(&mut s);
                s.answer(json!("F"));
                let itd = s.card(ITD).id.clone();
                s.answer(json!(itd));
                s.expect_in_zone(ITD, "hand");
                assert!(line(&s).is_none());
                s.play(ITD, json!({}));
                expect_line_equals(
                    &s,
                    json!({ "active": ["1"], "progress": { "1": 0 }, "done": [], "auras": [], "waiting": [] }),
                );
            }

            #[test]
            fn r16_reward_g_your_opponent_discards_2_cards_of_their_choice_their_own_prompt_then_quest_8() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": spare(json!({ "backrow": [ITD] })),
                    "p2": { "hand": [VANILLA, TIMMY, MENACE], "library": SPARE_LIBRARY },
                }));
                on_quest(&mut s, "4", 1, json!({}));
                any_action(&mut s);
                s.answer(json!("G"));
                let pending = must(s.state().pending.clone(), "the discard prompt");
                assert_eq!(pending.kind, PromptKind::Hand);
                assert_eq!(pending.player_id, P2);
                assert_eq!([pending.min, pending.max], [2, 2]);
                // p1 sees only that a prompt is open (R81, §10.8).
                assert_eq!(json!(s.view(P1).pending), json!({ "forYou": false, "pendingFor": "p2" }));
                let hand = s.hand(P2);
                let first = must(hand.first().cloned(), "a card");
                let second = must(hand.get(1).cloned(), "a card");
                s.answer(json!([first.id, second.id]));
                let left: Vec<String> = s.hand(P2).iter().map(|card| card.def_id.clone()).collect();
                assert_eq!(left, vec![MENACE]);
                expect_line_matches(&s, json!({ "active": ["8"], "done": ["4"] }));
            }

            #[test]
            fn r16_reward_g_with_one_card_in_the_opponents_hand_discards_that_one() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": spare(json!({ "backrow": [ITD] })),
                    "p2": { "hand": [MENACE], "library": SPARE_LIBRARY },
                }));
                on_quest(&mut s, "4", 1, json!({}));
                any_action(&mut s);
                s.answer(json!("G"));
                let pending = s.state().pending.clone();
                assert_eq!(
                    [
                        pending.as_ref().map(|pending| pending.min),
                        pending.as_ref().map(|pending| pending.max)
                    ],
                    [Some(1), Some(1)]
                );
                let menace = must(s.hand(P2).first().cloned(), "Menace");
                s.answer(json!(menace.id));
                assert!(s.hand(P2).is_empty());
            }

            #[test]
            fn r542_quest_5_damage_your_cards_deal_to_the_enemy_hero_and_enemy_units_12_in_all_your_own_side_and_the_opponents_hits_count_for_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "backrow": [ITD],
                        "field": [MENACE],
                        "hand": [LUNAR, LUNAR, STOCKPILE],
                        "library": SPARE_LIBRARY,
                        "mana": 10,
                    },
                    "p2": spare(json!({ "field": [VANILLA] })),
                }));
                on_quest(&mut s, "5", 0, json!({}));
                s.play(LUNAR, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
                assert_eq!(progress_of(&s), json!({ "5": 0 }));
                let vanilla = must(s.unit(P2, 1), "Vanilla");
                s.play(LUNAR, json!({ "targets": [instance_pick(&vanilla.id)] }));
                assert_eq!(progress_of(&s), json!({ "5": 3 }));
                s.attack(MENACE, "hero");
                assert_eq!(completed_in(s.last_events()), vec!["5"]);
                assert_eq!(reward_keys(&s, P1), vec!["mode:G", "mode:H"]);
                assert_eq!(json!(must(shown(&s, P2).quest, "the quest view").open), json!([]));
            }

            #[test]
            fn r404_reward_h_draw_2_then_quest_9_whose_count_starts_after_those_draws() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": spare(json!({ "backrow": [ITD] })), "p2": spare(json!({})) }));
                on_quest(&mut s, "5", 12, json!({}));
                any_action(&mut s);
                s.answer(json!("H"));
                assert_eq!(
                    s.last_events()
                        .iter()
                        .filter(|event| matches!(event, GameEvent::Drawn { .. }))
                        .count(),
                    2
                );
                expect_line_matches(&s, json!({ "active": ["9"], "done": ["5"], "progress": { "9": 0 } }));
            }

            #[test]
            fn r404_reward_h_that_empties_your_deck_opens_quest_9_on_an_empty_deck_which_completes_it_at_once() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "backrow": [ITD], "hand": [NOTEBOOK, TIMMY], "library": [VANILLA, VANILLA, VANILLA] },
                    "p2": spare(json!({})),
                }));
                on_quest(&mut s, "5", 12, json!({}));
                s.play(NOTEBOOK, json!({}));
                s.answer(json!("H"));
                assert!(s.pile(P1, "library").is_empty());
                assert_eq!(completed_in(s.last_events()), vec!["9"]);
                assert_eq!(reward_keys(&s, P1), vec!["mode:L"]);
            }

            #[test]
            fn r404_quest_6_your_units_total_attack_and_total_health_both_10_or_more_at_once() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "backrow": [ITD], "field": [MENACE], "hand": [TIMMY, STOCKPILE], "library": SPARE_LIBRARY },
                    "p2": spare(json!({ "field": [MENACE] })),
                }));
                on_quest(&mut s, "6", 0, json!({}));
                assert!(matches_object(
                    &first_open(&s, P1),
                    &json!({ "id": "6", "progress": 9, "goal": 10 })
                ));
                s.play(TIMMY, json!({}));
                assert_eq!(completed_in(s.last_events()), vec!["6"]);
                assert_eq!(reward_keys(&s, P1), vec!["mode:H", "mode:I"]);
            }

            #[test]
            fn r404_reward_i_recruit_the_first_permanent_from_the_top_of_your_deck_then_quest_10() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "backrow": [ITD],
                        "field": [MENACE, TIMMY],
                        "hand": [NOTEBOOK, TIMMY],
                        "library": [STOCKPILE, STOCKPILE, VANILLA, MENACE],
                    },
                    "p2": spare(json!({})),
                }));
                on_quest(&mut s, "6", 0, json!({}));
                s.play(NOTEBOOK, json!({}));
                s.answer(json!("I"));
                // The Notebook drew the top Stockpile; the Recruit passes the next one over for the Vanilla.
                assert_eq!(must(s.unit(P1, 3), "the recruit").def_id, VANILLA);
                let library: Vec<String> = s.pile(P1, "library").iter().map(|card| card.def_id.clone()).collect();
                assert_eq!(library, vec![STOCKPILE, MENACE]);
                expect_line_matches(&s, json!({ "active": ["10"], "done": ["6"] }));
            }

            #[test]
            fn r404_quest_7_a_turn_of_yours_ending_with_5_or_more_unspent_mana_4_is_not_enough() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": spare(json!({ "backrow": [ITD], "mana": 5 })),
                    "p2": spare(json!({})),
                }));
                on_quest(&mut s, "7", 0, json!({}));
                s.end_turn();
                assert_eq!(completed_in(s.last_events()), vec!["7"]);
                assert_eq!(reward_keys(&s, P1), vec!["mode:J"]);

                let mut short = scenario(json!({
                    "p1": spare(json!({ "backrow": [ITD], "mana": 4 })),
                    "p2": spare(json!({})),
                }));
                on_quest(&mut short, "7", 0, json!({}));
                short.end_turn();
                assert!(short.state().pending.is_none());
            }

            #[test]
            fn r540_reward_j_100_mana_on_your_next_turn_next_turn_mana_and_the_line_ends_there() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": spare(json!({ "backrow": [ITD], "mana": 5 })),
                    "p2": spare(json!({})),
                }));
                on_quest(&mut s, "7", 0, json!({}));
                s.end_turn();
                s.answer(json!("J"));
                expect_line_matches(&s, json!({ "active": [], "done": ["7"] }));
                assert_eq!(s.state().active, P2);
                assert_eq!(s.state().players.p1.mana.next_turn_mod, 100);
                s.end_turn();
                assert_eq!(s.state().active, P1);
                let mana = &s.state().players.p1.mana;
                assert_eq!(mana.current, mana.max + 100);
            }

            #[test]
            fn r404_quest_8_three_cards_entering_either_exile_pile_a_unit_token_ceases_to_exist_instead_r11() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "backrow": [ITD],
                        "hand": [TRUE_STRIKE, COLLATERAL, STOCKPILE],
                        "library": SPARE_LIBRARY,
                        "mana": 10,
                    },
                    "p2": spare(json!({ "field": [VANILLA, RUSH_TOKEN] })),
                }));
                on_quest(&mut s, "8", 0, json!({}));
                // True Strike exiles itself as it resolves: 1.
                let token = must(s.unit(P2, 2), "the token");
                s.play(TRUE_STRIKE, json!({ "targets": [instance_pick(&token.id)] }));
                assert_eq!(progress_of(&s), json!({ "8": 1 }));
                // Collateral Damage exiles the Vanilla and a card of p2's deck: 3.
                let vanilla = must(s.unit(P2, 1), "Vanilla");
                s.play(COLLATERAL, json!({ "targets": [instance_pick(&vanilla.id)] }));
                assert_eq!(completed_in(s.last_events()), vec!["8"]);
                assert_eq!(reward_keys(&s, P1), vec!["mode:K"]);
            }

            #[test]
            fn r404_quest_8_does_not_count_a_unit_token_exiled_from_the_field() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "backrow": [ITD], "hand": [COLLATERAL, STOCKPILE], "library": SPARE_LIBRARY, "mana": 10 },
                    "p2": spare(json!({ "field": [RUSH_TOKEN] })),
                }));
                on_quest(&mut s, "8", 0, json!({}));
                let token = must(s.unit(P2, 1), "the token");
                s.play(COLLATERAL, json!({ "targets": [instance_pick(&token.id)] }));
                // The token ceased to exist; the deck card did enter exile.
                assert_eq!(progress_of(&s), json!({ "8": 1 }));
            }

            #[test]
            fn r404_reward_k_your_opponents_deck_every_card_of_it_to_their_exile() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": spare(json!({ "backrow": [ITD] })),
                    "p2": { "hand": [STOCKPILE], "library": [VANILLA, TIMMY, MENACE] },
                }));
                on_quest(&mut s, "8", 3, json!({}));
                any_action(&mut s);
                s.answer(json!("K"));
                assert!(s.pile(P2, "library").is_empty());
                let mut exiled: Vec<String> = s.pile(P2, "exile").iter().map(|card| card.def_id.clone()).collect();
                exiled.sort();
                let mut want = vec![MENACE, TIMMY, VANILLA];
                want.sort();
                assert_eq!(exiled, want);
                expect_line_matches(&s, json!({ "active": [], "done": ["8"] }));
            }

            #[test]
            fn r404_quest_9_only_the_draw_of_yours_that_takes_your_decks_last_card_a_deck_emptied_any_other_way_does_not() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "backrow": [ITD], "hand": [NOTEBOOK, NOTEBOOK, TIMMY], "library": [VANILLA, VANILLA] },
                    "p2": spare(json!({})),
                }));
                on_quest(&mut s, "9", 0, json!({}));
                s.play(NOTEBOOK, json!({}));
                assert_eq!(progress_of(&s), json!({ "9": 0 }));
                s.play(NOTEBOOK, json!({}));
                assert_eq!(completed_in(s.last_events()), vec!["9"]);
                assert_eq!(reward_keys(&s, P1), vec!["mode:L"]);

                let mut milled = scenario(json!({
                    "p1": { "backrow": [ITD], "hand": [EUGENICS, NOTEBOOK, TIMMY], "library": [VANILLA, VANILLA] },
                    "p2": spare(json!({})),
                }));
                on_quest(&mut milled, "9", 0, json!({}));
                milled.play(EUGENICS, json!({}));
                assert!(milled.pile(P1, "library").is_empty());
                assert_eq!(progress_of(&milled), json!({ "9": 0 }));
                // A fatigue draw takes no card either.
                milled.play(NOTEBOOK, json!({}));
                assert!(must(line(&milled), "a quest line").done.is_empty());
            }

            #[test]
            fn r454_reward_l_you_may_play_cards_from_your_graveyard_while_it_stands_an_exile_ends_the_aura_and_the_line_r78() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": spare(json!({ "backrow": [ITD], "graveyard": [TIMMY] })),
                    "p2": { "hand": [COLLATERAL, STOCKPILE], "library": SPARE_LIBRARY, "mana": 10 },
                }));
                on_quest(&mut s, "9", 1, json!({}));
                any_action(&mut s);
                s.answer(json!("L"));
                expect_line_matches(&s, json!({ "auras": ["L"], "active": [], "done": ["9"] }));
                assert_eq!(
                    json!(must(shown(&s, P2).quest, "the quest view").auras),
                    json!([{ "id": "L", "text": "Aura: you may play cards from your graveyard" }])
                );
                let timmy = must(
                    s.pile(P1, "graveyard").into_iter().find(|card| card.def_id == TIMMY),
                    "Timmy in the graveyard",
                );
                assert!(offers_play_of(&s, P1, &timmy.id));

                s.end_turn();
                let itd = s.card(ITD).id.clone();
                s.play(COLLATERAL, json!({ "targets": [instance_pick(&itd)] }));
                s.expect_in_zone(ITD, "exile");
                assert!(line(&s).is_none());
                s.end_turn();
                assert!(!offers_play_of(&s, P1, &timmy.id));
            }

            #[test]
            fn r347_reward_m_your_units_have_indestructible_and_so_no_taunt_while_it_stands_an_exile_ends_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": spare(json!({ "backrow": [ITD], "field": [MENACE] })),
                    "p2": { "hand": [COLLATERAL, STOCKPILE], "library": SPARE_LIBRARY, "mana": 10 },
                }));
                let menace = must(s.unit(P1, 1), "Menace");
                assert!(has_keyword(&s.stats(&menace).keywords, KeywordKind::Taunt));
                // The line as reward M leaves it (the reward itself: quest 10's test).
                set_line(&mut s, json!({ "active": [], "done": ["10"], "auras": ["M"] }));
                assert!(has_keyword(&s.stats(&menace).keywords, KeywordKind::Indestructible));
                assert!(!has_keyword(&s.stats(&menace).keywords, KeywordKind::Taunt));
                s.end_turn();
                let itd = s.card(ITD).id.clone();
                s.play(COLLATERAL, json!({ "targets": [instance_pick(&itd)] }));
                assert!(!has_keyword(&s.stats(&menace).keywords, KeywordKind::Indestructible));
                assert!(has_keyword(&s.stats(&menace).keywords, KeywordKind::Taunt));
            }

            #[test]
            fn r102_a_lockdown_fused_onto_it_unlicensed_experimentation_leaves_its_quest_line_and_its_tree_quest_2_counts_on_and_asks() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": spare(json!({ "backrow": [ITD, { "def": EXPERIMENT, "faceUp": false }] })),
                    "p2": {
                        "field": [VANILLA],
                        "hand": [LOCKDOWN, HIT_JOB, STOCKPILE],
                        "library": SPARE_LIBRARY,
                        "mana": 10,
                    },
                }));
                on_quest(&mut s, "2", 1, json!({}));
                let itd = s.card(ITD).clone();
                s.play(LOCKDOWN, json!({}));
                assert!(
                    s.last_events()
                        .iter()
                        .any(|event| matches!(event, GameEvent::Fused { .. }))
                );
                assert_ne!(s.card(&itd.id).def_id, ITD);
                let memory = line_json(&must(subsystems::quest_memory_of(s.card(&itd.id)), "the quest line"));
                assert!(matches_object(&memory, &json!({ "active": ["2"], "progress": { "2": 1 } })));
                let vanilla = must(s.unit(P2, 1), "Vanilla");
                s.play(HIT_JOB, json!({ "targets": [instance_pick(&vanilla.id)] }));
                assert_eq!(completed_in(s.last_events()), vec!["2"]);
                assert_eq!(reward_keys(&s, P1), vec!["mode:C", "mode:D"]);
            }

            #[test]
            fn r78_a_tribute_ends_aura_m_fused_with_a_lockdown_its_activate_tributes_the_card_and_your_units_lose_indestructible() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": spare(json!({ "backrow": [ITD, { "def": EXPERIMENT, "faceUp": false }], "field": [MENACE] })),
                    "p2": { "hand": [LOCKDOWN, STOCKPILE], "library": SPARE_LIBRARY, "mana": 10 },
                }));
                set_line(&mut s, json!({ "active": [], "done": ["10"], "auras": ["M"] }));
                let itd = s.card(ITD).clone();
                let menace = must(s.unit(P1, 1), "Menace");
                s.play(LOCKDOWN, json!({}));
                assert_ne!(s.card(&itd.id).def_id, ITD);
                assert!(has_keyword(&s.stats(&menace).keywords, KeywordKind::Indestructible));
                s.end_turn();
                s.activate(&itd.id, json!({ "ability": "tribute" }));
                s.expect_in_zone(&itd.id, "graveyard");
                assert!(!has_keyword(&s.stats(&menace).keywords, KeywordKind::Indestructible));
            }

            #[test]
            fn r404_quest_10_six_units_in_your_graveyard_a_spell_there_is_none_then_reward_m() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "backrow": [ITD],
                        "field": [TIMMY],
                        "graveyard": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, STOCKPILE],
                        "hand": [HIT_JOB, STOCKPILE],
                        "library": SPARE_LIBRARY,
                    },
                    "p2": spare(json!({})),
                }));
                on_quest(&mut s, "10", 0, json!({}));
                assert!(matches_object(
                    &first_open(&s, P1),
                    &json!({ "id": "10", "progress": 5, "goal": 6 })
                ));
                let timmy = must(s.unit(P1, 1), "Timmy");
                s.play(HIT_JOB, json!({ "targets": [instance_pick(&timmy.id)] }));
                assert_eq!(completed_in(s.last_events()), vec!["10"]);
                assert_eq!(reward_keys(&s, P1), vec!["mode:M"]);
                s.answer(json!("M"));
                expect_line_matches(&s, json!({ "active": [], "auras": ["M"], "done": ["10"] }));
            }

            #[test]
            fn r79_a_quest_completed_on_the_opponents_turn_is_noticed_then_and_the_reward_prompt_is_yours() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": spare(json!({ "backrow": [ITD] })),
                    "p2": { "field": [VANILLA], "hand": [HIT_JOB, STOCKPILE], "library": SPARE_LIBRARY },
                }));
                on_quest(&mut s, "2", 1, json!({}));
                // p2 destroys its own unit: an enemy permanent of p1's, destroyed by anything.
                let vanilla = must(s.unit(P2, 1), "Vanilla");
                s.play(HIT_JOB, json!({ "targets": [instance_pick(&vanilla.id)] }));
                assert_eq!(s.state().active, P2);
                assert_eq!(reward_keys(&s, P1), vec!["mode:C", "mode:D"]);
                assert_eq!(json!(s.view(P2).pending), json!({ "forYou": false, "pendingFor": "p1" }));
                // The reports name the public card in the other player's view too.
                let reports: Vec<String> = s
                    .view(P2)
                    .events
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::QuestCompleted { instance_id, .. } => Some(instance_id.clone()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(reports, vec![s.card(ITD).id.clone()]);
            }

            #[test]
            fn r113_a_reward_paused_on_its_own_question_survives_a_json_round_trip_and_finishes_the_same() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": spare(json!({ "backrow": [ITD] })),
                    "p2": spare(json!({ "field": [VANILLA] })),
                }));
                s.play(STOCKPILE, json!({}));
                s.answer(json!("B"));
                let paused = s.state().clone();
                let revived: GameState = serde_json::from_str(&serde_json::to_string(&paused).unwrap()).unwrap();
                assert_eq!(revived, paused);
                let foe = must(s.unit(P2, 1), "Vanilla");
                let choice_id = must(revived.pending.clone(), "the target prompt").id;
                let result = reduce(
                    &revived,
                    &json_as::<Action>(json!({
                        "type": "answer",
                        "playerId": "p1",
                        "choiceId": choice_id,
                        "selection": [instance_pick(&foe.id)],
                        "nonce": "itd-round-trip",
                    })),
                );
                assert!(result.error.is_none());
                s.answer(json!(foe.id));
                assert_eq!(hash_state(&result.state), hash_state(s.state()));
                let card = must(result.state.players.p1.backrow[0].clone(), "In Too Deep");
                assert_eq!(must(subsystems::quest_memory_of(&card), "the quest line").active, vec!["3"]);
            }

            #[test]
            fn s9_2_a_game_through_quest_1_and_its_reward_folds_from_its_log_to_the_same_state_and_views() {
                crate::register_all();
                let deck: Vec<&str> = vec![
                    ITD, "core-002", "core-005", "core-006", "core-008", "core-011", "core-012", "core-013",
                    "core-015", "core-016", "core-019", "core-020", "core-025", "core-026", "core-032",
                    "core-036", "core-043", "core-044", "core-053", "core-055",
                ];
                let other: Vec<&str> = deck.iter().map(|id| if *id == ITD { "core-003" } else { *id }).collect();
                let seed = "itd-replay";
                let mut log: Vec<Action> = Vec::new();
                let mut n: u32 = 0;
                fn act(state: &GameState, body: Value, n: &mut u32, log: &mut Vec<Action>) -> GameState {
                    *n += 1;
                    let mut action = body;
                    action["nonce"] = json!(format!("itd-{n}"));
                    let action: Action = json_as(action);
                    let result = reduce(state, &action);
                    if let Some(error) = result.error {
                        panic!("{error}");
                    }
                    log.push(action);
                    result.state
                }
                let mut state = begin_game(&create_game(&json_as(json!({ "seed": seed, "decks": [deck, other] })))).state;
                for player in [P1, P2] {
                    let keep: Vec<String> = state.players[player].hand.iter().map(|card| card.id.clone()).collect();
                    state = act(
                        &state,
                        json!({ "type": "mulligan", "keep": keep, "playerId": player }),
                        &mut n,
                        &mut log,
                    );
                }
                // Quickdraw put it in p1's opening hand.
                let card = must(
                    state.players.p1.hand.iter().find(|card| card.def_id == ITD).cloned(),
                    "In Too Deep in the opening hand",
                );
                state = act(
                    &state,
                    json!({ "type": "play", "instanceId": card.id, "playerId": "p1" }),
                    &mut n,
                    &mut log,
                );
                // Two of p1's turn draws complete quest 1.
                let mut turn = 0;
                while turn < 4 && state.pending.is_none() {
                    let active = state.active;
                    state = act(&state, json!({ "type": "endTurn", "playerId": active }), &mut n, &mut log);
                    turn += 1;
                }
                let pending = must(state.pending.clone(), "the reward prompt");
                assert_eq!(pending.kind, PromptKind::Reward);
                state = act(
                    &state,
                    json!({
                        "type": "answer",
                        "playerId": "p1",
                        "choiceId": pending.id,
                        "selection": [{ "pick": "mode", "option": "A" }],
                    }),
                    &mut n,
                    &mut log,
                );
                let itd = must(
                    state.players.p1.backrow.iter().flatten().find(|card| card.def_id == ITD).cloned(),
                    "ITD",
                );
                assert_eq!(must(subsystems::quest_memory_of(&itd), "the quest line").active, vec!["2"]);

                let replayed = fold(&json_as(json!({ "seed": seed, "decks": [deck, other], "log": log })));
                assert!(replayed.errors.is_empty());
                assert_eq!(hash_state(&replayed.state), hash_state(&state));
                for viewer in [P1, P2] {
                    assert_eq!(view_for(&replayed.state, viewer), view_for(&state, viewer));
                }
            }
        }

        mod radiant {
            use super::*;

            fn radiant_itd() -> Value {
                json!({ "def": ITD, "radiant": true })
            }

            #[test]
            fn r543_quest_1_grants_a_and_b_with_no_reward_prompt_b_still_asks_its_target_and_only_then_opens_quests_2_and_3() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": spare(json!({ "backrow": [radiant_itd()], "health": 20 })),
                    "p2": spare(json!({ "field": [VANILLA] })),
                }));
                s.play(STOCKPILE, json!({}));
                assert!(!prompt_opened_for_a_reward(s.events()));
                // A has healed (Stockpile 2 + A 6); B asks.
                s.expect_health(P1, 28);
                assert_eq!(pending_kind(&s), Some(PromptKind::Target));
                assert!(active_of(&s).is_empty());
                let vanilla = must(s.unit(P2, 1), "Vanilla");
                s.answer(json!(vanilla.id));
                expect_line_matches(&s, json!({ "active": ["2", "3"], "done": ["1"] }));
                assert_eq!(open_ids(&s, P2), Some(ids(&["2", "3"])));
            }

            #[test]
            fn r404_a_quest_reached_by_two_paths_opens_once_and_a_reward_two_completed_quests_offer_d_is_granted_by_each() {
                crate::register_all();
                let mut s = scenario(json!({
                    // Small Units, so the +3/+3 does not complete quest 6 as well.
                    "p1": spare(json!({
                        "backrow": [radiant_itd()],
                        "field": [TIMMY, TOKEN_MAKER],
                        "graveyard": [MENACE, STOCKPILE],
                    })),
                    "p2": spare(json!({ "field": [MENACE] })),
                }));
                // Quests 2 and 3 open, as quest 1's Radiant rewards leave them; quest 2 at its goal, and quest 3
                // met by the board (In Too Deep and two Units).
                set_line(&mut s, json!({ "active": ["2", "3"], "progress": { "2": 2 }, "done": ["1"] }));
                any_action(&mut s);
                // Quest 2: C (two graveyard cards), D (three placements on one pick); quest 3: D again, E.
                let foe = must(s.unit(P2, 1), "Menace");
                let mut placements = 0;
                while s.state().pending.is_some() {
                    assert_eq!(pending_kind(&s), Some(PromptKind::Target));
                    s.answer(json!(foe.id));
                    placements += 1;
                }
                // One prompt per D grant (R689): two answers, six counters.
                assert_eq!(placements, 2);
                assert_eq!(s.card(&foe).counters.plague, Some(6));
                assert_eq!(
                    s.events()
                        .iter()
                        .filter(|event| matches!(event, GameEvent::Buffed { .. }))
                        .count(),
                    1
                );
                let memory = must(line(&s), "the line");
                assert_eq!(memory.done, vec!["1", "2", "3"]);
                // 4 and 5 from quest 2's rewards, 6 from quest 3's; 5 opened once.
                assert_eq!(memory.active, vec!["4", "5", "6"]);
            }

            #[test]
            fn r404_quest_5_grants_g_the_opponents_own_discard_and_h_then_opens_quests_8_and_9() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": spare(json!({ "backrow": [radiant_itd()] })),
                    "p2": { "hand": [VANILLA, TIMMY, MENACE], "library": SPARE_LIBRARY },
                }));
                on_quest(&mut s, "5", 12, json!({ "done": ["1", "2"] }));
                any_action(&mut s);
                let pending = must(s.state().pending.clone(), "G's discard");
                assert_eq!((pending.kind, pending.player_id), (PromptKind::Hand, P2));
                let hand = s.hand(P2);
                let a = must(hand.first().cloned(), "a");
                let b = must(hand.get(1).cloned(), "b");
                s.answer(json!([a.id, b.id]));
                assert_eq!(s.hand(P2).len(), 1);
                expect_line_matches(
                    &s,
                    json!({ "active": ["8", "9"], "done": ["1", "2", "5"], "progress": { "9": 0 } }),
                );
            }

            #[test]
            fn r404_quests_7_to_10_end_their_lines_quest_7_grants_j_and_opens_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": spare(json!({ "backrow": [radiant_itd()], "mana": 5 })),
                    "p2": spare(json!({})),
                }));
                on_quest(&mut s, "7", 0, json!({}));
                s.end_turn();
                assert!(!prompt_opened_for_a_reward(s.events()));
                assert_eq!(s.state().players.p1.mana.next_turn_mod, 100);
                expect_line_matches(&s, json!({ "active": [], "done": ["7"] }));
            }
        }
    }
}
