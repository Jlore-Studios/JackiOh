//! Degrade and Upgrade (docs/classic-sets.md B3.4, R386), and Classic+ #41 KY's Constant's "change a
//! number to 3": the verbs that write a card's `tuning`. Players read Degrade as Nerf and Upgrade as
//! Buff (R1320); the engine keeps the old names.
//!
//! One application is one change, drawn from the menu rows that can change the card now (B3.4 rule 3),
//! scaled by the card's tune multiplier (R1160, ME-TUNEMULT, Meditative #84 Volatility):
//!
//! ```text
//!   | Row     | Degrade                          | Upgrade                         | Can apply to
//!   | cost    | `costMod` +1, never above (4)    | −1, never below (0)             | off the field; never X-cost (R65)
//!   | stats   | −4 split, k to attack            | +4 split                        | a Unit or a card with stats
//!   | keyword | remove one it has                | add one it lacks                | add: see below
//!   | x       | one numbered keyword or X worse  | one better                      | never below 1
//!   | number  | one declared number a step worse | one step better                 | within its bounds
//!   | plague  | remove a Plague Counter          | place one                       | a `usesPlague` card on the field
//! ```
//!
//! On the field neither the cost row nor a number only the Cry reads is drawn, both spent (R1748000),
//! nor a Tribute (R2261000). An Upgrade's keyword is from R21's pool for a Unit, Indestructible or
//! Immutable for a Field Spell (R2436000), Pierce or Lifesteal for a damaging Spell (R1748001).
//!
//! R442: the row is drawn first, uniformly, then the item within it; one choice draws nothing (R129),
//! and a card no row can change, an Immutable one first (rule 2), is left alone with no draw.
//!
//! Events: `degraded` / `upgraded`, naming whoever may not read the card (`hiddenFrom`, R177). Every
//! application is cued, an unchanged card as `none` (R440), so the cues count applications, never changes.
//! Only `discover_number` opens a prompt, last in its effect, so a pause never splits an application (R113).

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::brittle_count::active_brittle_count;
use crate::config::{
    TUNE_ATTACK_FLOOR, TUNE_BUFF_ANY_KEYWORDS, TUNE_BUFF_REMOVES, TUNE_COST_CAP, TUNE_COST_FLOOR,
    TUNE_COST_STEP, TUNE_DAMAGE_KEYWORDS, TUNE_FIELD_SPELL_KEYWORDS, TUNE_HARMFUL_KEYWORDS,
    TUNE_HEALTH_FLOOR, TUNE_MULTIPLIER_NONE, TUNE_NERF_HAND_KEYWORDS, TUNE_PLAGUE_STEP, TUNE_STAT_TOTAL,
    TUNE_X_STEP,
};
use crate::faces::card_type_of;
use crate::layers::{card_keywords, printed_keywords_of, stats_with_buffs, unclamped_attack, unit_view};
use crate::mana::is_x_cost;
use crate::numbers::{
    NumberRef, NumberedKey, current_stats, number_key, number_on, number_ref_id, numbered_keywords_on,
    numbers_on, own_cost, parse_number_ref,
};
use crate::params::{set_param, step_param, steppable_params_times};
use crate::prompts::{OpenPromptArgs, open_prompt, resume_self};
use crate::script::{Effect, EffectContext};
use crate::state::{BrittleCounter, CardInstance, GameState, PromptOption, find_instance, find_instance_mut};
use crate::tuning::{TUNED_FLOOR, X_KEY, add_step, tidy_tuning, tuned_count, tuning_of, x_of};
use crate::wire::{
    CardType, GameEvent, Keyword, KeywordKind, PromptKind, Selection, TuningChange, ZoneName, has_keyword,
};

use super::buff::candidates_beside;
use super::card_scope::{CardScope, CardScopeOptions, cards_in_card_scope, unreadable_by};
use super::targets::{TargetSpec, instance_on_its_stay, resolve_target};
use crate::damage::DamageTarget;

/// Which way a change goes.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum TuneDirection {
    #[serde(rename = "degrade")]
    Degrade,
    #[serde(rename = "upgrade")]
    Upgrade,
}

impl TuneDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            TuneDirection::Degrade => "degrade",
            TuneDirection::Upgrade => "upgrade",
        }
    }
}

/// R1160: how many times as effective one Buff or Nerf application is on this card — that direction's
/// field of the running face's `tune_multiplier`, none on a Vanilla card, floored at plain.
pub fn tune_multiplier_of(state: &GameState, card: &CardInstance, direction: TuneDirection) -> i32 {
    let flags = crate::scripts::flags_of(state, card);
    let named = match direction {
        TuneDirection::Degrade => flags.tune_multiplier.and_then(|flags| flags.degrade),
        TuneDirection::Upgrade => flags.tune_multiplier.and_then(|flags| flags.upgrade),
    };
    named.unwrap_or(TUNE_MULTIPLIER_NONE).max(TUNE_MULTIPLIER_NONE)
}

/// Which cards a Degrade or Upgrade reaches: one named card — a pick the play or a prompt carried
/// (`target`, R81: Classic+ #71 Book of Buff's card, which may be in its caster's hand), `{ of:
/// "self" }` (#69 Buff Billy), or an id a script captured (`instanceId`) — or every card of a `scope`
/// (#8 Withering Storm's Radiant: "every card in your opponent's deck"), or `random` different cards
/// of it (R60: #8's "4 random cards", #70 Chaos Machine's, T-AI-10 Fine-Tuning's). `times` is how many
/// applications each card takes ("5 times" is five draws, B3.4 rule 1), default 1.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TuneArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<CardScope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub random: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub times: Option<i32>,
}

// The menu (B3.4 rule 3)

/// One applicable row: `apply` makes the change (drawing within the row as it must) and reports it.
struct MenuRow {
    row: TuneRow,
    apply: MenuApply,
}

/// What a row does when it is drawn, with everything it read as the menu was built.
enum MenuApply {
    /// `costMod` moves by `delta`.
    Cost { delta: i32 },
    /// A split drawn as it applies, floored by the stats the card had (`attack`, `health`).
    /// R1160: `total` is the split's size (`TUNE_STAT_TOTAL` times the multiplier).
    Stats { attack: i32, health: i32, total: i32 },
    /// A keyword row: `count` keywords drawn, each removed (`remove`, `own` naming the keyword the event
    /// shows) or added (`add`). A Degrade removes what it may and adds a harmful one (R2657005's Crater in
    /// a hand); an Upgrade adds what it may and removes a harmful one (R2657005's Can't attack and Can't
    /// Defend). The first draw is over both lists, `remove` first; any later one (R1160) keeps to its way.
    Keyword {
        own: Vec<Keyword>,
        remove: Vec<KeywordKind>,
        add: Vec<Keyword>,
        count: i32,
    },
    /// One of the X row's items drawn and written.
    X { items: Vec<XItem> },
    /// One declared number drawn and stepped: `(key, steps, delta)` per item.
    Number { items: Vec<(String, i32, i32)> },
    /// R1748002: `amount` Plague Counters placed on the card (an Upgrade) or removed (a Degrade).
    Plague { amount: i32 },
}

/// B3.4 rule 3's menu rows, in its order.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum TuneRow {
    #[serde(rename = "cost")]
    Cost,
    #[serde(rename = "stats")]
    Stats,
    #[serde(rename = "keyword")]
    Keyword,
    #[serde(rename = "x")]
    X,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "plague")]
    Plague,
}

impl TuneRow {
    pub fn as_str(self) -> &'static str {
        match self {
            TuneRow::Cost => "cost",
            TuneRow::Stats => "stats",
            TuneRow::Keyword => "keyword",
            TuneRow::X => "x",
            TuneRow::Number => "number",
            TuneRow::Plague => "plague",
        }
    }
}

/// The keywords the card has now — all five layers in the unit row or standing in a backrow zone
/// (§10.4, R1081000), layers 1 to 4 elsewhere.
fn keywords_now(state: &GameState, card: &CardInstance) -> Vec<Keyword> {
    if crate::zones::in_units_row_or_standing(state, card) {
        unit_view(state, card).keywords
    } else {
        card_keywords(state, card)
    }
}

/// A uniform pick (R442), drawing nothing when there is one choice (R129).
fn pick_one<'a, T>(ctx: &mut EffectContext<'_>, items: &'a [T]) -> &'a T {
    if items.len() == 1 {
        return &items[0];
    }
    match ctx.rng.pick(items) {
        Some(picked) => picked,
        None => panic!("B3.4: a menu row was offered with nothing in it"),
    }
}

/// B3.4 rule 3, cost: `costMod` one step toward (4) or toward (0); never an X-cost card (R65), and
/// never a card on the field, which has paid its price (R1748000). R1160: `times` steps, clamped.
fn cost_row(state: &GameState, card: &CardInstance, direction: TuneDirection, times: i32) -> Option<MenuRow> {
    if card.zone.z() == ZoneName::Field {
        return None;
    }
    let own = own_cost(state, card)?;
    let delta = match direction {
        TuneDirection::Degrade => (TUNE_COST_STEP * times).min(TUNE_COST_CAP - own),
        TuneDirection::Upgrade => 0 - (TUNE_COST_STEP * times).min(own - TUNE_COST_FLOOR),
    };
    if (direction == TuneDirection::Degrade && delta <= 0)
        || (direction == TuneDirection::Upgrade && delta >= 0)
    {
        return None;
    }
    Some(MenuRow {
        row: TuneRow::Cost,
        apply: MenuApply::Cost { delta },
    })
}

/// B3.4 rule 3, stats: a split of `TUNE_STAT_TOTAL`, k to attack and the rest to health. A Degrade's
/// floors (attack 0, current health 1) lose what they refuse, so it applies only while above one. On
/// the field the change moves max health, damage staying (B3.4 rule 6).
fn stats_row(
    state: &GameState,
    card: &CardInstance,
    direction: TuneDirection,
    times: i32,
) -> Option<MenuRow> {
    let stats = current_stats(state, card)?;
    if direction == TuneDirection::Degrade
        && stats.attack <= TUNE_ATTACK_FLOOR
        && stats.health <= TUNE_HEALTH_FLOOR
    {
        return None;
    }
    Some(MenuRow {
        row: TuneRow::Stats,
        apply: MenuApply::Stats {
            attack: stats.attack,
            health: stats.health,
            total: TUNE_STAT_TOTAL * times,
        },
    })
}

const HARMFUL: &[KeywordKind] = TUNE_HARMFUL_KEYWORDS;

/// B3.4 rule 3, keyword. A Degrade removes one keyword the card has of its own, printed or granted, never
/// one an aura or its position lends it, and never a harmful one (Can't attack, Brittle, Untributable,
/// Can't Defend, Crater): every entry of the kind goes. Or, R2657005, it adds Crater to a Unit, Field
/// Spell, Trap or Field Trap in a hand that lacks it (`TUNE_NERF_HAND_KEYWORDS`). An Upgrade adds one R21
/// keyword a Unit lacks, R1748001 one a Spell or Field Spell lacks (`non_unit_candidates`), and R2657005
/// Flashback to any card (`TUNE_BUFF_ANY_KEYWORDS`); or it removes a Can't attack or Can't Defend the card
/// has of its own (`TUNE_BUFF_REMOVES`). A Vanilla card's text is gone, so nothing is added to one.
fn keyword_row(
    state: &GameState,
    card: &CardInstance,
    direction: TuneDirection,
    times: i32,
) -> Option<MenuRow> {
    let (own, add): (Vec<Keyword>, Vec<Keyword>) = if direction == TuneDirection::Degrade {
        let own: Vec<Keyword> = card_keywords(state, card)
            .into_iter()
            .filter(|keyword| !HARMFUL.contains(&keyword.kind()))
            .collect();
        (own, nerf_candidates(state, card))
    } else {
        let own: Vec<Keyword> = card_keywords(state, card)
            .into_iter()
            .filter(|keyword| TUNE_BUFF_REMOVES.contains(&keyword.kind()))
            .collect();
        let add = if card.vanilla {
            Vec::new()
        } else {
            let held = keywords_now(state, card);
            let mut add: Vec<Keyword> = if card_type_of(state, card) == CardType::Unit {
                // R21, R781004: a keyword it lacks from the combat keyword pool, never Taunt beside
                // Indestructible.
                candidates_beside(&held)
            } else {
                non_unit_candidates(state, card, &held)
            };
            add.extend(lacking(&held, TUNE_BUFF_ANY_KEYWORDS));
            add
        };
        (own, add)
    };
    let remove: Vec<KeywordKind> = own
        .iter()
        .map(|keyword| keyword.kind())
        .collect::<IndexSet<KeywordKind>>()
        .into_iter()
        .collect();
    if remove.is_empty() && add.is_empty() {
        return None;
    }
    Some(MenuRow {
        row: TuneRow::Keyword,
        apply: MenuApply::Keyword {
            own,
            remove,
            add,
            count: times,
        },
    })
}

/// The keywords of `kinds` that `held` lacks, in `kinds`' order, each as its keyword.
fn lacking(held: &[Keyword], kinds: &[KeywordKind]) -> Vec<Keyword> {
    kinds
        .iter()
        .filter(|kind| !has_keyword(held, **kind))
        .map(|kind| Keyword::of_kind(*kind, 0))
        .collect()
}

/// R2657005: the harmful keywords a Nerf may add — Crater, to a Unit, a Field Spell, a Trap or a Field
/// Trap while it is in a hand and lacks it (`TUNE_NERF_HAND_KEYWORDS`). Never Can't attack or Can't
/// Defend, and nothing to a Vanilla card, whose text is gone.
fn nerf_candidates(state: &GameState, card: &CardInstance) -> Vec<Keyword> {
    if card.vanilla || !matches!(card.zone, crate::wire::Zone::Hand { .. }) {
        return Vec::new();
    }
    let card_type = card_type_of(state, card);
    if !matches!(
        card_type,
        CardType::Unit | CardType::FieldSpell | CardType::Trap | CardType::FieldTrap
    ) {
        return Vec::new();
    }
    lacking(&keywords_now(state, card), TUNE_NERF_HAND_KEYWORDS)
}

/// R1748001: the keywords a Buff may add to a card that is no Unit, in this order: a Field Spell's
/// (R2436000), then a damaging Spell's or Field Spell's, each only while it lacks it.
fn non_unit_candidates(state: &GameState, card: &CardInstance, held: &[Keyword]) -> Vec<Keyword> {
    let card_type = card_type_of(state, card);
    let mut kinds: Vec<KeywordKind> = Vec::new();
    if card_type == CardType::FieldSpell {
        kinds.extend_from_slice(TUNE_FIELD_SPELL_KEYWORDS);
    }
    if matches!(card_type, CardType::Spell | CardType::FieldSpell)
        && crate::catalog::def_of(Some(state), &card.def_id).deals_damage == Some(true)
    {
        kinds.extend_from_slice(TUNE_DAMAGE_KEYWORDS);
    }
    kinds
        .into_iter()
        .filter(|kind| !has_keyword(held, *kind))
        .map(|kind| Keyword::of_kind(kind, 0))
        .collect()
}

fn remove_kind(state: &mut GameState, card_id: &str, kind: KeywordKind) {
    let Some(snapshot) = find_instance(state, card_id).cloned() else {
        return;
    };
    let printed = printed_keywords_of(state, &snapshot)
        .iter()
        .any(|keyword| keyword.kind() == kind);
    let Some(card) = find_instance_mut(state, card_id) else {
        return;
    };
    let tuning = tuning_of(card);
    if printed {
        let mut removed = tuning.remove_keywords.clone().unwrap_or_default();
        if !removed.contains(&kind) {
            removed.push(kind);
            tuning.remove_keywords = Some(removed);
        }
    }
    let added: Vec<Keyword> = tuning
        .add_keywords
        .clone()
        .unwrap_or_default()
        .into_iter()
        .filter(|keyword| keyword.kind() != kind)
        .collect();
    tuning.add_keywords = Some(added);
    card.granted_keywords.retain(|keyword| keyword.kind() != kind);
    tidy_tuning(card);
}

/// One number the X row may move: its key, its value now and after one step, and the write.
struct XItem {
    key: String,
    before: i32,
    after: i32,
    write: XWrite,
}

/// What an X item writes.
enum XWrite {
    /// One more X step on `key` in `tuning.x`.
    Step { key: String, delta: i32 },
    /// A Brittle count in force set to `count` (the rest of the counter as it was).
    Brittle { brittle: BrittleCounter, count: i32 },
}

fn write_x(state: &mut GameState, card_id: &str, write: &XWrite) {
    let Some(card) = find_instance_mut(state, card_id) else {
        return;
    };
    match write {
        XWrite::Step { key, delta } => {
            let tuning = tuning_of(card);
            tuning.x = Some(add_step(tuning.x.as_ref(), key, *delta));
            tidy_tuning(card);
        }
        XWrite::Brittle { brittle, count } => {
            card.brittle = Some(BrittleCounter {
                count: *count,
                ..*brittle
            });
        }
    }
}

/// The card with one more X step on `key` in its tuning, to read the value the step would give.
fn with_x_step(card: &CardInstance, key: &str, delta: i32) -> CardInstance {
    let mut tuning = card.tuning.clone().unwrap_or_default();
    tuning.x = Some(add_step(
        card.tuning.as_ref().and_then(|t| t.x.as_ref()),
        key,
        delta,
    ));
    CardInstance {
        tuning: Some(tuning),
        ..card.clone()
    }
}

/// B3.4 rule 3, X: one numbered keyword or an X-cost card's X, one step (`TUNE_X_STEP`) worse or
/// better — more is better for all but Tribute, never below 1 (`TUNED_FLOOR`). A Brittle count in force
/// moves itself (B3.3 rule 5). An X-cost card with no X chosen yet always moves; on the field its X has
/// resolved (SPEC §8.7 row 69: Classic+ #69 Buff Billy) and a Tribute was paid (R2261000), so neither moves.
fn x_items(state: &GameState, card: &CardInstance, direction: TuneDirection, times: i32) -> Vec<XItem> {
    let mut items: Vec<XItem> = Vec::new();
    let better = if direction == TuneDirection::Upgrade {
        TUNE_X_STEP * times
    } else {
        -TUNE_X_STEP * times
    };
    let step = |key: &str, delta: i32| XWrite::Step {
        key: key.to_string(),
        delta,
    };

    if is_x_cost(state, card) && card.zone.z() != ZoneName::Field {
        match card.x {
            None => items.push(XItem {
                key: X_KEY.to_string(),
                before: 0,
                after: better,
                write: step(X_KEY, better),
            }),
            Some(x) => {
                let before = x_of(card);
                let after = tuned_count(&with_x_step(card, X_KEY, better), X_KEY, x);
                if after != before {
                    items.push(XItem {
                        key: X_KEY.to_string(),
                        before,
                        after,
                        write: step(X_KEY, after - before),
                    });
                }
            }
        }
    }

    for entry in numbered_keywords_on(state, card) {
        if entry.key == NumberedKey::Tribute && card.zone.z() == ZoneName::Field {
            continue;
        }
        let key: String = entry.key.as_str().to_string();
        let delta = if entry.better.as_str() == "up" {
            better
        } else {
            -better
        };
        match entry.printed {
            None => {
                let Some(brittle) = card.brittle else {
                    continue;
                };
                let after = TUNED_FLOOR.max(entry.value + delta);
                if after == entry.value {
                    continue;
                }
                items.push(XItem {
                    key,
                    before: entry.value,
                    after,
                    write: XWrite::Brittle {
                        brittle,
                        count: after,
                    },
                });
            }
            Some(printed) => {
                let after = tuned_count(&with_x_step(card, &key, delta), &key, printed);
                if after == entry.value {
                    continue;
                }
                items.push(XItem {
                    write: step(&key, after - entry.value),
                    key,
                    before: entry.value,
                    after,
                });
            }
        }
    }
    items
}

fn x_row(state: &GameState, card: &CardInstance, direction: TuneDirection, times: i32) -> Option<MenuRow> {
    let items = x_items(state, card, direction, times);
    if items.is_empty() {
        return None;
    }
    Some(MenuRow {
        row: TuneRow::X,
        apply: MenuApply::X { items },
    })
}

/// B3.4 rule 3, number: one declared number one step worse or better (`params::steppable_params`).
/// R1160: `times` steps at once (`steppable_params_times`).
fn number_row(
    state: &GameState,
    card: &CardInstance,
    direction: TuneDirection,
    times: i32,
) -> Option<MenuRow> {
    let items: Vec<(String, i32, i32)> = steppable_params_times(state, card, direction, times)
        .into_iter()
        .map(|item| (item.param.key.clone(), item.steps, item.delta))
        .collect();
    if items.is_empty() {
        return None;
    }
    Some(MenuRow {
        row: TuneRow::Number,
        apply: MenuApply::Number { items },
    })
}

/// R1748002: on the field, a card that uses its own Plague Counters takes `TUNE_PLAGUE_STEP` more from
/// an Upgrade, and gives one up to a Degrade while it carries any; never a Vanilla card, nor a face-down
/// one while it sleeps (R1487000). R1160: `times` counters at once.
fn plague_row(
    state: &GameState,
    card: &CardInstance,
    direction: TuneDirection,
    times: i32,
) -> Option<MenuRow> {
    if card.vanilla
        || crate::catalog::def_of(Some(state), &card.def_id).uses_plague != Some(true)
        || !crate::plague::carries_plague(state, card)
        || crate::preview::lasting_text_dormant(state, card)
    {
        return None;
    }
    if direction == TuneDirection::Degrade && crate::plague::plague_on(card) == 0 {
        return None;
    }
    Some(MenuRow {
        row: TuneRow::Plague,
        apply: MenuApply::Plague {
            amount: TUNE_PLAGUE_STEP * times,
        },
    })
}

/// A drawn row's change, written onto the card (`card_id`) and reported.
fn apply_row(
    ctx: &mut EffectContext<'_>,
    card_id: &str,
    direction: TuneDirection,
    row: &MenuRow,
) -> TuningChange {
    match &row.apply {
        MenuApply::Cost { delta } => {
            if let Some(card) = find_instance_mut(ctx.state, card_id) {
                card.cost_mod += delta;
            }
            TuningChange::Cost { delta: *delta }
        }
        MenuApply::Stats {
            attack: had_attack,
            health: had_health,
            total,
        } => {
            let k = ctx.rng.int(total + 1);
            let rest = total - k;
            let attack = if direction == TuneDirection::Upgrade {
                k
            } else {
                0 - k.min((had_attack - TUNE_ATTACK_FLOOR).max(0))
            };
            let health = if direction == TuneDirection::Upgrade {
                rest
            } else {
                0 - rest.min((had_health - TUNE_HEALTH_FLOOR).max(0))
            };
            if let Some(card) = find_instance_mut(ctx.state, card_id) {
                let tuning = tuning_of(card);
                tuning.attack = Some(tuning.attack.unwrap_or(0) + attack);
                tuning.health = Some(tuning.health.unwrap_or(0) + health);
                tidy_tuning(card);
            }
            TuningChange::Stats { attack, health }
        }
        MenuApply::Keyword {
            own,
            remove,
            add,
            count,
        } => {
            // R1160: one draw per pick, distinct kinds, today's write each time; the first pick is over
            // both ways (R2657005), the rest keep to its way.
            let mut left: Vec<(bool, Keyword)> = remove
                .iter()
                .map(|kind| {
                    let shown = own
                        .iter()
                        .find(|keyword| keyword.kind() == *kind)
                        .cloned()
                        .unwrap_or_else(|| Keyword::of_kind(*kind, 0));
                    (false, shown)
                })
                .chain(add.iter().map(|keyword| (true, keyword.clone())))
                .collect();
            let mut picked: Vec<Keyword> = Vec::new();
            let mut way: Option<bool> = None;
            for _ in 0..(*count).max(1) {
                if left.is_empty() {
                    break;
                }
                let (added, keyword) = pick_one(ctx, &left).clone();
                way.get_or_insert(added);
                left.retain(|(kept_way, kept)| *kept_way == added && kept.kind() != keyword.kind());
                if added {
                    if let Some(card) = find_instance_mut(ctx.state, card_id) {
                        let tuning = tuning_of(card);
                        let mut gained = tuning.add_keywords.clone().unwrap_or_default();
                        gained.push(keyword.clone());
                        tuning.add_keywords = Some(gained);
                        // §10.4: a keyword the card gains anew is up, as a granted one is (`buff::grant_to`).
                        if keyword.kind() == KeywordKind::DivineShield {
                            card.divine_shield_spent = None;
                        }
                        if keyword.kind() == KeywordKind::Reborn {
                            card.reborn_spent = None;
                        }
                    }
                } else {
                    remove_kind(ctx.state, card_id, keyword.kind());
                }
                picked.push(keyword);
            }
            let mut moved = picked.into_iter();
            let first = moved.next().expect("a keyword row always moves one");
            let also: Vec<Keyword> = moved.collect();
            TuningChange::Keyword {
                keyword: first,
                added: way == Some(true),
                also: if also.is_empty() { None } else { Some(also) },
            }
        }
        MenuApply::X { items } => {
            let item = pick_one(ctx, items);
            write_x(ctx.state, card_id, &item.write);
            TuningChange::X {
                key: item.key.clone(),
                delta: item.after - item.before,
            }
        }
        MenuApply::Number { items } => {
            let (key, steps, delta) = pick_one(ctx, items).clone();
            if let Some(card) = find_instance_mut(ctx.state, card_id) {
                step_param(card, &key, steps);
            }
            TuningChange::Number { key, delta }
        }
        MenuApply::Plague { amount } => {
            let Some(live) = find_instance(ctx.state, card_id).cloned() else {
                return TuningChange::Plague { delta: 0 };
            };
            // R471: a Buff's counters are a placement, which "whenever Plague Counters are placed on
            // this" answers; a Nerf's come off as a removal.
            let delta = match direction {
                TuneDirection::Upgrade => crate::plague::place_plague_on(ctx, &live, *amount),
                TuneDirection::Degrade => 0 - crate::plague::remove_plague(ctx, &live, *amount),
            };
            TuningChange::Plague { delta }
        }
    }
}

/// B3.4 rules 1–3: the rows that can change the card now, in the menu's order, plague last. R1160:
/// `times` scales each row's change. R1845001: no cost row without `with_cost`, as a hidden Nerf lands.
fn menu_of(
    state: &GameState,
    card: &CardInstance,
    direction: TuneDirection,
    times: i32,
    with_cost: bool,
) -> Vec<MenuRow> {
    // Rule 2: an Immutable card is never changed, and nothing is drawn for it.
    if has_keyword(&keywords_now(state, card), KeywordKind::Immutable) {
        return Vec::new();
    }
    [
        if with_cost {
            cost_row(state, card, direction, times)
        } else {
            None
        },
        stats_row(state, card, direction, times),
        keyword_row(state, card, direction, times),
        x_row(state, card, direction, times),
        number_row(state, card, direction, times),
        plague_row(state, card, direction, times),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// B3.4 rule 3: the rows that can change the card now, in the menu's order — a pure read, for a
/// `conditionMet` or a `targetChecks` predicate (a Degrade with nothing to change) and for the tests.
pub fn applicable_changes(state: &GameState, card: &CardInstance, direction: TuneDirection) -> Vec<TuneRow> {
    menu_of(state, card, direction, TUNE_MULTIPLIER_NONE, true)
        .into_iter()
        .map(|row| row.row)
        .collect()
}

// One application, and the cards a verb reaches

/// B3.4 rule 1: one Degrade or Upgrade of one card — a row drawn among those that can change it, then
/// the change. `matches` is false for a card of a hidden pile the scope's filters left out, which is
/// cued and never changed (R440). The card is read again by its id, so a second application of it
/// draws from the menu the first one left.
pub fn tune_once(ctx: &mut EffectContext<'_>, card: &CardInstance, direction: TuneDirection, matches: bool) {
    tune_once_with(ctx, card, direction, matches, true);
}

/// R1845001: one Nerf as a hidden Nerf lands — `tune_once`'s, drawn from every row but the cost's,
/// and reported as every Nerf is (R177, R440).
pub(crate) fn degrade_sparing_cost(ctx: &mut EffectContext<'_>, card: &CardInstance) {
    tune_once_with(ctx, card, TuneDirection::Degrade, true, false);
}

fn tune_once_with(
    ctx: &mut EffectContext<'_>,
    card: &CardInstance,
    direction: TuneDirection,
    matches: bool,
    with_cost: bool,
) {
    let card = find_instance(ctx.state, &card.id)
        .cloned()
        .unwrap_or_else(|| card.clone());
    // R177: who could not read the card where it changed, judged before the change moves anything.
    let hidden_from = unreadable_by(ctx.state, &card);
    let times = tune_multiplier_of(ctx.state, &card, direction);
    let menu = if matches {
        menu_of(ctx.state, &card, direction, times, with_cost)
    } else {
        Vec::new()
    };
    if menu.is_empty() && hidden_from.is_empty() {
        return;
    }
    let change = if menu.is_empty() {
        TuningChange::None
    } else {
        let row = pick_one(ctx, &menu);
        apply_row(ctx, &card.id, direction, row)
    };
    let hidden_from = if hidden_from.is_empty() {
        None
    } else {
        Some(hidden_from)
    };
    ctx.events.push(match direction {
        TuneDirection::Upgrade => GameEvent::Upgraded {
            instance_id: card.id.clone(),
            def_id: card.def_id.clone(),
            change,
            hidden_from,
        },
        TuneDirection::Degrade => GameEvent::Degraded {
            instance_id: card.id.clone(),
            def_id: card.def_id.clone(),
            change,
            hidden_from,
        },
    });
}

/// The one card a verb names, by id or by spec, in whatever zone it is (B3.4 rule 2).
pub(crate) fn named_card(
    ctx: &EffectContext<'_>,
    target: Option<&TargetSpec>,
    instance_id: Option<&str>,
) -> Option<CardInstance> {
    // R174: a card named by id is aimed at the stay it had when the run began (`instance_on_its_stay`).
    if let Some(instance_id) = instance_id {
        return instance_on_its_stay(ctx, instance_id);
    }
    let target = target?;
    match resolve_target(ctx, target)? {
        // A card that has ceased to exist (R11, R86) is in no pile to change.
        DamageTarget::Unit { instance } if instance.zone.z() != ZoneName::Gone => Some(instance),
        _ => None,
    }
}

/// One card a verb reaches, and whether it may change (`reached_cards`).
#[derive(Clone, Debug, PartialEq)]
pub struct ReachedCard {
    pub card: CardInstance,
    pub matches: bool,
}

/// The cards a verb over `args` reaches, each with whether it may change: the one named card, or every
/// card of a scope, or `random` different ones (R60) drawn uniformly over it with every card of a hidden
/// pile in it whatever the filters say (R440), so the odds hang on the piles' sizes and public cards
/// alone. The picks keep the scope's order (R242); a pick of all there are draws nothing (R129).
pub fn reached_cards(
    ctx: &mut EffectContext<'_>,
    args: &TuneArgs,
    direction: TuneDirection,
) -> Vec<ReachedCard> {
    let Some(scope) = &args.scope else {
        return named_card(ctx, args.target.as_ref(), args.instance_id.as_deref())
            .map(|card| vec![ReachedCard { card, matches: true }])
            .unwrap_or_default();
    };
    // MD-B1, R940: a Degrade is harmful, passing an immune card in a tribal scope by; a Buff reaches it.
    let aim = match direction {
        TuneDirection::Degrade => Some(crate::wire::TargetAim::Harm),
        TuneDirection::Upgrade => None,
    };
    let pool: Vec<ReachedCard> = cards_in_card_scope(
        ctx,
        scope,
        Some(&CardScopeOptions {
            whole_hidden_piles: Some(true),
            aim,
        }),
    )
    .into_iter()
    .map(|entry| ReachedCard {
        card: entry.card.clone(),
        matches: entry.matches,
    })
    .collect();
    let Some(random) = args.random else {
        return pool;
    };
    let count = random.max(0) as usize;
    if count == 0 {
        return Vec::new();
    }
    if pool.len() <= count {
        return pool;
    }
    let ids: Vec<String> = pool.iter().map(|entry| entry.card.id.clone()).collect();
    let picked: IndexSet<String> = ctx.rng.shuffle(&ids).into_iter().take(count).collect();
    pool.into_iter()
        .filter(|entry| picked.contains(&entry.card.id))
        .collect()
}

fn tune_effect(kind: &'static str, direction: TuneDirection, args: TuneArgs) -> Effect {
    Effect::new(kind, move |ctx| {
        let times = args.times.unwrap_or(1).max(0);
        if times == 0 {
            return;
        }
        for reached in reached_cards(ctx, &args, direction) {
            for _ in 0..times {
                tune_once(ctx, &reached.card, direction, reached.matches);
            }
        }
    })
}

/// B3.4, R386: Degrade (Nerf, R1320) — `times` applications to each card reached, each one change
/// drawn from the menu. Classic+ #8 Withering Storm, #72 Book of Nerf, #70 Chaos Machine, #73.
pub fn degrade(args: TuneArgs) -> Effect {
    tune_effect("degrade", TuneDirection::Degrade, args)
}

/// B3.4, R386: Upgrade (Buff, R1320) — Degrade's mirror (B9 #7). Classic+ #69 Buff Billy, #71 Book of
/// Buff, #70, #73, T-AI-10 Fine-Tuning.
pub fn upgrade(args: TuneArgs) -> Effect {
    tune_effect("upgrade", TuneDirection::Upgrade, args)
}

// KY's Constant: a number set outright (Classic+ #41)

/// Classic+ #41: set one number on a card to `value`, as `tuning` (B3.4 rule 4) so it stays with the
/// card: attack and health by the delta that makes them read `value` now (current health on the field),
/// a numbered keyword or declared number by `tuning.set`, spending its earlier steps (a Brittle count
/// in force is set itself).
fn write_number(ctx: &mut EffectContext<'_>, card_id: &str, number: &NumberRef, value: i32) {
    let Some(card) = find_instance(ctx.state, card_id).cloned() else {
        return;
    };
    let state: &GameState = ctx.state;
    match number {
        NumberRef::Cost => {
            if let Some(own) = own_cost(state, &card)
                && let Some(live) = find_instance_mut(ctx.state, card_id)
            {
                live.cost_mod += value - own;
            }
        }
        NumberRef::Attack | NumberRef::Health => {
            let on_field = crate::zones::in_units_row_or_standing(state, &card);
            let attack = matches!(number, NumberRef::Attack);
            let now = if attack {
                if on_field {
                    unclamped_attack(state, &card)
                } else {
                    stats_with_buffs(state, &card).attack
                }
            } else if on_field {
                unit_view(state, &card).health
            } else {
                stats_with_buffs(state, &card).max_health - card.damage
            };
            if let Some(live) = find_instance_mut(ctx.state, card_id) {
                let tuning = tuning_of(live);
                if attack {
                    tuning.attack = Some(tuning.attack.unwrap_or(0) + (value - now));
                } else {
                    tuning.health = Some(tuning.health.unwrap_or(0) + (value - now));
                }
                tidy_tuning(live);
            }
        }
        NumberRef::Param { key } => {
            if let Some(live) = find_instance_mut(ctx.state, card_id) {
                set_param(live, key, value);
            }
        }
        // R2657006: the count left (R2179001), never below 1, so the card stays Temporary.
        NumberRef::Temporary => {
            if let Some(live) = find_instance_mut(ctx.state, card_id) {
                live.temporary = Some(value.max(TUNED_FLOOR));
            }
        }
        // R2657006: the Plague Counters set outright, a set and no placement (R471), reported as any
        // change of the count is.
        NumberRef::Plague => {
            let count = value.max(0);
            if let Some(live) = find_instance_mut(ctx.state, card_id) {
                live.counters.plague = if count == 0 { None } else { Some(count) };
            }
            ctx.events.push(GameEvent::CounterChanged {
                instance_id: card_id.to_string(),
                counter: crate::wire::CounterKind::Plague,
                value: count,
                placed: None,
            });
        }
        NumberRef::Keyword { key } => {
            let key: &str = key.as_str();
            if key == "Brittle"
                && active_brittle_count(&card).is_some()
                && let Some(brittle) = card.brittle
            {
                if let Some(live) = find_instance_mut(ctx.state, card_id) {
                    live.brittle = Some(BrittleCounter {
                        count: value.max(0),
                        ..brittle
                    });
                }
                return;
            }
            if let Some(live) = find_instance_mut(ctx.state, card_id) {
                let tuning = tuning_of(live);
                let mut set: IndexMap<String, i32> = tuning.set.clone().unwrap_or_default();
                set.insert(key.to_string(), value);
                tuning.set = Some(set);
                let mut x: IndexMap<String, i32> = tuning.x.clone().unwrap_or_default();
                x.shift_remove(key);
                tuning.x = Some(x);
                tidy_tuning(live);
            }
        }
    }
}

/// `setNumber`'s `which`: `NumberRef | string | "random"` — a reference, its id as a prompt option
/// carries it (`number_ref_id`), or `"random"`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(untagged)]
pub enum NumberWhich {
    Ref(NumberRef),
    Id(String),
}

/// `setNumber`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SetNumberArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    pub which: NumberWhich,
    pub value: i32,
}

/// Classic+ #41 KY's Constant: change a number on a card to `value`. `which` names the number, or is
/// `"random"`: a uniform pick (R60) among the card's numbers not `value` already, drawing nothing when
/// there is none (R129). A number the card lacks, and an Immutable card (B3.4 rule 2), are left alone.
/// `numberChanged` reports what it came to (bounds may hold it off `value`), hidden as a Degrade's is
/// (R177): the base face picks at random on a card in its caster's hand.
pub fn set_number(args: SetNumberArgs) -> Effect {
    Effect::new("setNumber", move |ctx| {
        let Some(card) = named_card(ctx, args.target.as_ref(), args.instance_id.as_deref()) else {
            return;
        };
        let value = args.value;
        let Some(number) = ref_for(ctx, &card, &args.which, value) else {
            return;
        };
        let hidden_from = unreadable_by(ctx.state, &card);
        write_number(ctx, &card.id, &number, value);
        let card = find_instance(ctx.state, &card.id).cloned().unwrap_or(card);
        let came_to = number_on(ctx.state, &card, &number).unwrap_or(value);
        ctx.events.push(GameEvent::NumberChanged {
            instance_id: card.id.clone(),
            def_id: card.def_id.clone(),
            key: number_key(&number),
            value: came_to,
            hidden_from: if hidden_from.is_empty() {
                None
            } else {
                Some(hidden_from)
            },
        });
    })
}

/// The number `which` names on the card, or `None` when the card has no such number to change.
fn ref_for(
    ctx: &mut EffectContext<'_>,
    card: &CardInstance,
    which: &NumberWhich,
    value: i32,
) -> Option<NumberRef> {
    let numbers = numbers_on(ctx.state, card);
    if let NumberWhich::Id(id) = which
        && id == "random"
    {
        let open: Vec<_> = numbers.into_iter().filter(|entry| entry.value != value).collect();
        if open.is_empty() {
            return None;
        }
        return Some(pick_one(ctx, &open).ref_.clone());
    }
    let number = match which {
        NumberWhich::Ref(number) => number.clone(),
        NumberWhich::Id(id) => parse_number_ref(id)?,
    };
    let id = number_ref_id(&number);
    if numbers.iter().any(|entry| entry.id == id) {
        Some(number)
    } else {
        None
    }
}

/// The key under which `discover_number` hands its card's id to the step its answer re-enters.
pub const NUMBER_CARD_KEY: &str = "numberOf";

/// `discoverNumber`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DiscoverNumberArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    pub value: i32,
    pub count: i32,
    pub step: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

/// Classic+ #41 KY's Constant's Radiant: "Discover a number on it" — up to `count` different numbers
/// on the card not `value` already, drawn uniformly (R60; all, with no draw, when no more, R129), in a
/// `discover` prompt to the caster alone (R81). The answer re-enters `step` with the card's id under
/// `NUMBER_CARD_KEY`, read by `chosen_tuning_number` (not E18's `number` reader: this names a number on a
/// card). No number to offer: nothing opens, and the list goes on.
pub fn discover_number(args: DiscoverNumberArgs) -> Effect {
    Effect::new("discoverNumber", move |ctx| {
        let Some(card) = named_card(ctx, args.target.as_ref(), args.instance_id.as_deref()) else {
            return;
        };
        let open: Vec<_> = numbers_on(ctx.state, &card)
            .into_iter()
            .filter(|entry| entry.value != args.value)
            .collect();
        let count = args.count.max(0) as usize;
        if open.is_empty() || count == 0 {
            return;
        }
        let offered: Vec<_> = if open.len() <= count {
            open
        } else {
            let ids: Vec<String> = open.iter().map(|entry| entry.id.clone()).collect();
            let picked: IndexSet<String> = ctx.rng.shuffle(&ids).into_iter().take(count).collect();
            open.into_iter()
                .filter(|entry| picked.contains(&entry.id))
                .collect()
        };
        let options: Vec<PromptOption> = offered
            .iter()
            .map(|entry| PromptOption {
                key: format!("mode:{}", entry.id),
                label: format!("{} ({})", entry.label, entry.value),
                selection: Selection::Mode {
                    option: entry.id.clone(),
                },
                cost: None,
                radiant: None,
            })
            .collect();
        let mut data: IndexMap<String, Value> = IndexMap::new();
        data.insert(NUMBER_CARD_KEY.to_string(), json!(card.id));
        let resume = resume_self(ctx, &args.step, data);
        let controller = ctx.controller;
        let _ = open_prompt(
            ctx,
            OpenPromptArgs {
                player: controller,
                kind: PromptKind::Discover,
                aim: None,
                prompt: args
                    .prompt
                    .clone()
                    .unwrap_or_else(|| "Choose a number".to_string()),
                options,
                min: None,
                max: None,
                budget: None,
                owner: None,
                resume,
            },
        );
    })
}

/// The number a `discover_number` answer picked and the card it is on (`chosen_tuning_number`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ChosenTuningNumber {
    pub instance_id: String,
    pub which: NumberRef,
}

/// The number a `discover_number` answer picked and the card it is on, for the step it re-enters.
pub fn chosen_tuning_number(ctx: &EffectContext<'_>) -> Option<ChosenTuningNumber> {
    let pick = ctx.targets.first();
    let instance_id = ctx.data.get(NUMBER_CARD_KEY).and_then(Value::as_str);
    let (Some(Selection::Mode { option }), Some(instance_id)) = (pick, instance_id) else {
        return None;
    };
    let which = parse_number_ref(option)?;
    Some(ChosenTuningNumber {
        instance_id: instance_id.to_string(),
        which,
    })
}

/// `gainKeywordForGood`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct GainKeywordForGoodArgs {
    /// The keyword the card keeps: Lucky 1 for Stinkyscape #154 Dice of Doom.
    pub keyword: Keyword,
}

/// R1870007 (Stinkyscape #154 Dice of Doom): the card running the script keeps `keyword` in every zone,
/// in its `tuning` as an Upgrade's is (R386), so neither leaving the field (R78) nor a graveyard (R215)
/// takes it. A numbered Armor or Lucky stacks (`tuning::tuned_keywords`); another kind is not had twice.
pub fn gain_keyword_for_good(args: GainKeywordForGoodArgs) -> Effect {
    Effect::new("gainKeywordForGood", move |ctx| {
        let Some(card_id) = ctx.live_self().map(|card| card.id.clone()) else {
            return;
        };
        let Some(card) = find_instance_mut(ctx.state, &card_id) else {
            return;
        };
        let tuning = tuning_of(card);
        let mut added = tuning.add_keywords.clone().unwrap_or_default();
        added.push(args.keyword.clone());
        tuning.add_keywords = Some(added);
        ctx.events.push(GameEvent::KeywordGranted {
            instance_id: card_id,
            keyword: args.keyword.clone(),
            lost: None,
        });
    })
}
