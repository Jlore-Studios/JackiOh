//! Degrade and Upgrade (docs/classic-sets.md B3.4, R386), and Classic+ #41 KY's Constant's "change a
//! number to 3": the verbs that write a card's `tuning` (`tuning.rs` holds the readers, `numbers.rs`
//! the numbers on a card). Players read Degrade as Nerf and Upgrade as Buff (patch v0.3.4, R1320); the
//! engine keeps its names here, as `degrade`, `upgrade`, `tune_once`, `TuneDirection` and the events.
//!
//! One application is one change, drawn from the menu rows that can change the card now (B3.4 rule 3):
//!
//! ```text
//!   | Row     | Degrade                          | Upgrade                         | Can apply to
//!   | cost    | `costMod` +1, never above (4)    | −1, never below (0)             | never an X-cost card (R65)
//!   | stats   | −4 split, k to attack            | +4 split                        | a Unit or an Animated card
//!   | keyword | remove one it has                | add one it lacks (R21's pool)   | add: a Unit
//!   | x       | one numbered keyword or X worse  | one better                      | never below 1
//!   | number  | one declared number a step worse | one step better                 | within its bounds
//! ```
//!
//! R442: the draw is the row first, uniformly among the rows that can change the card, then the item
//! within the row (which keyword, which number), uniformly, and for a stats row the split k in 0–4. A
//! row or an item with one choice draws nothing (R129), and a card no row can change is left alone
//! with no draw at all (B3.4 rule 1) — an Immutable card first of all (rule 2).
//!
//! Events: `degraded` / `upgraded` for each application that changed the card, public on a public
//! card. On a card someone may not read — a hand card, a deck card, a face-down trap — the event says
//! who could not read it (`hiddenFrom`, R177), and every application is cued, a card nothing changed
//! with the change `none` (R440), so the count of cues over a hidden pile numbers the applications,
//! never the changes. For the same reason a scope over a hidden pile reaches every card in it: its
//! filters decide which cards change, and a card they leave out is cued `none` (`card_scope.rs`).
//! `numberChanged` reports KY's Constant's set the same way.
//!
//! None of these opens a prompt but `discover_number`, which is one prompt and nothing after it in the
//! same effect, so a pause never splits an application (R113): what a list does after a Degrade is the
//! list's own business, parked by `prompts::apply_resumable` like any other effect's.
//!
//! Port of `packages/engine/src/effects/tune.ts`. TS's menu rows were closures over the live card;
//! here a row is data (`MenuApply`) that `apply_row` writes onto the card found again by id, which is
//! the same thing: a row is built and applied in one go, with only the row's own draw between.

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::brittle_count::active_brittle_count;
use crate::config::{
    TUNE_ATTACK_FLOOR, TUNE_COST_CAP, TUNE_COST_FLOOR, TUNE_COST_STEP, TUNE_HARMFUL_KEYWORDS,
    TUNE_HEALTH_FLOOR, TUNE_STAT_TOTAL, TUNE_X_STEP,
};
use crate::faces::card_type_of;
use crate::layers::{card_keywords, printed_keywords_of, stats_with_buffs, unclamped_attack, unit_view};
use crate::mana::is_x_cost;
use crate::numbers::{
    NumberRef, current_stats, number_key, number_on, number_ref_id, numbered_keywords_on, numbers_on,
    own_cost, parse_number_ref,
};
use crate::params::{set_param, step_param, steppable_params};
use crate::prompts::{OpenPromptArgs, open_prompt, resume_self};
use crate::script::{Effect, EffectContext};
use crate::state::{BrittleCounter, CardInstance, GameState, PromptOption, find_instance, find_instance_mut};
use crate::tuning::{TUNED_FLOOR, X_KEY, add_step, tidy_tuning, tuned_count, tuning_of, x_of};
use crate::wire::{
    CardType, GameEvent, Keyword, KeywordKind, PromptKind, Row, Selection, TuningChange, Zone, ZoneName,
    has_keyword,
};

use super::buff::random_pool_keywords;
use super::card_scope::{CardScope, CardScopeOptions, cards_in_card_scope, unreadable_by};
use super::targets::{TargetSpec, instance_on_its_stay, resolve_target};
use crate::damage::DamageTarget;

/// Which way a change goes.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TuneDirection {
    #[serde(rename = "degrade")]
    Degrade,
    #[serde(rename = "upgrade")]
    Upgrade,
}

impl TuneDirection {
    /// The literal, as TS writes it.
    pub fn as_str(self) -> &'static str {
        match self {
            TuneDirection::Degrade => "degrade",
            TuneDirection::Upgrade => "upgrade",
        }
    }
}

/// Which cards a Degrade or Upgrade reaches: one named card — a pick the play or a prompt carried
/// (`target`, R81: Classic+ #71 Book of Buff's card, which may be in its caster's hand), `{ of:
/// "self" }` (#69 Buff Billy), or an id a script captured (`instanceId`) — or every card of a `scope`
/// (#8 Withering Storm's Radiant: "every card in your opponent's deck"), or `random` different cards
/// of it (R60: #8's "4 random cards", #70 Chaos Machine's, T-AI-10 Fine-Tuning's). `times` is how many
/// applications each card takes ("5 times" is five draws, B3.4 rule 1), default 1.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
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

// ---------------------------------------------------------------------------
// The menu (B3.4 rule 3)
// ---------------------------------------------------------------------------

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
    Stats { attack: i32, health: i32 },
    /// A Degrade's keyword: one of `kinds` drawn, `own` naming the keyword the event shows.
    RemoveKeyword {
        own: Vec<Keyword>,
        kinds: Vec<KeywordKind>,
    },
    /// An Upgrade's keyword: one of `candidates` drawn and added.
    AddKeyword { candidates: Vec<Keyword> },
    /// One of the X row's items drawn and written.
    X { items: Vec<XItem> },
    /// One declared number drawn and stepped: `(key, steps, delta)` per item.
    Number { items: Vec<(String, i32, i32)> },
}

/// B3.4 rule 3's menu rows, in its order.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
}

impl TuneRow {
    /// The literal, as TS writes it.
    pub fn as_str(self) -> &'static str {
        match self {
            TuneRow::Cost => "cost",
            TuneRow::Stats => "stats",
            TuneRow::Keyword => "keyword",
            TuneRow::X => "x",
            TuneRow::Number => "number",
        }
    }
}

/// The keywords the card has now — all five layers in the unit row (§10.4), layers 1 to 4 elsewhere.
fn keywords_now(state: &GameState, card: &CardInstance) -> Vec<Keyword> {
    match card.zone {
        Zone::Field { row: Row::Units, .. } => unit_view(state, card).keywords,
        _ => card_keywords(state, card),
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

/// B3.4 rule 3, cost: `costMod` one step toward (4) or toward (0); never an X-cost card (R65).
fn cost_row(state: &GameState, card: &CardInstance, direction: TuneDirection) -> Option<MenuRow> {
    let own = own_cost(state, card)?;
    let delta = match direction {
        TuneDirection::Degrade => TUNE_COST_STEP.min(TUNE_COST_CAP - own),
        TuneDirection::Upgrade => 0 - TUNE_COST_STEP.min(own - TUNE_COST_FLOOR),
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

/// B3.4 rule 3, stats: a split of `TUNE_STAT_TOTAL` rolled as k to attack and the rest to health. A
/// Degrade's attack floors at 0 and its current health at 1, and what the floors refuse is lost; so a
/// Degrade can apply only while the card has attack above 0 or health above 1. On the field the change
/// moves max health, damage staying, so current health moves with it (B3.4 rule 6).
fn stats_row(state: &GameState, card: &CardInstance, direction: TuneDirection) -> Option<MenuRow> {
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
        },
    })
}

const HARMFUL: &[KeywordKind] = TUNE_HARMFUL_KEYWORDS;

/// B3.4 rule 3, keyword. A Degrade removes one keyword the card has of its own — printed (as tuning
/// leaves it) or granted, never one an aura or its position lends it, and never a harmful one (Can't
/// attack, Brittle): every entry of the kind goes, a printed one by `tuning.removeKeywords`, an added
/// one out of `tuning.addKeywords`, a granted one off the instance. An Upgrade adds one R21 keyword a
/// Unit lacks (`tuning.addKeywords`); a Vanilla unit's text is gone, and an added keyword would be
/// text, so nothing is added to one.
fn keyword_row(state: &GameState, card: &CardInstance, direction: TuneDirection) -> Option<MenuRow> {
    if direction == TuneDirection::Degrade {
        let own: Vec<Keyword> = card_keywords(state, card)
            .into_iter()
            .filter(|keyword| !HARMFUL.contains(&keyword.kind()))
            .collect();
        let kinds: Vec<KeywordKind> = own
            .iter()
            .map(|keyword| keyword.kind())
            .collect::<IndexSet<KeywordKind>>()
            .into_iter()
            .collect();
        if kinds.is_empty() {
            return None;
        }
        return Some(MenuRow {
            row: TuneRow::Keyword,
            apply: MenuApply::RemoveKeyword { own, kinds },
        });
    }
    if card_type_of(state, card) != CardType::Unit || card.vanilla {
        return None;
    }
    let held = keywords_now(state, card);
    let candidates: Vec<Keyword> = random_pool_keywords()
        .into_iter()
        .filter(|keyword| !has_keyword(&held, keyword.kind()))
        .collect();
    if candidates.is_empty() {
        return None;
    }
    Some(MenuRow {
        row: TuneRow::Keyword,
        apply: MenuApply::AddKeyword { candidates },
    })
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

/// What an X item writes (TS: a closure over the card).
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

/// A copy of the card's tuning with one more X step on `key`, to read the value the step would give.
/// (TS built a bare `{ tuning }`; the readers take a card, so this is the card with that tuning.)
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
/// better — more is better for all but Tribute, and nothing goes below 1 (`TUNED_FLOOR`). A Brittle
/// count in force moves itself (B3.3 rule 5); a printed Brittle that has not started moves the number
/// it will start at. An X-cost card with no chosen X (off the field) has its X still to come, so its
/// step always applies ("its X counts 1 less or more when it resolves"). On the field its X has
/// resolved, so it has no X to move (SPEC §8.7 row 69: Classic+ #69 Buff Billy's Cry Upgrades draw
/// from the stats and keyword rows only); elsewhere one applies while the X it counts can move.
fn x_items(state: &GameState, card: &CardInstance, direction: TuneDirection) -> Vec<XItem> {
    let mut items: Vec<XItem> = Vec::new();
    let better = if direction == TuneDirection::Upgrade {
        TUNE_X_STEP
    } else {
        -TUNE_X_STEP
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
                        write: step(X_KEY, better),
                    });
                }
            }
        }
    }

    for entry in numbered_keywords_on(state, card) {
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
                    write: step(&key, delta),
                    key,
                    before: entry.value,
                    after,
                });
            }
        }
    }
    items
}

fn x_row(state: &GameState, card: &CardInstance, direction: TuneDirection) -> Option<MenuRow> {
    let items = x_items(state, card, direction);
    if items.is_empty() {
        return None;
    }
    Some(MenuRow {
        row: TuneRow::X,
        apply: MenuApply::X { items },
    })
}

/// B3.4 rule 3, number: one declared number one step worse or better (`params::steppable_params`).
fn number_row(state: &GameState, card: &CardInstance, direction: TuneDirection) -> Option<MenuRow> {
    let items: Vec<(String, i32, i32)> = steppable_params(state, card, direction)
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
        } => {
            let k = ctx.rng.int(TUNE_STAT_TOTAL + 1);
            let rest = TUNE_STAT_TOTAL - k;
            // TS wrote `0 - n`, never `-n`, so a share the floors refuse whole is 0, not −0; integers
            // have no −0.
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
        MenuApply::RemoveKeyword { own, kinds } => {
            let kind = *pick_one(ctx, kinds);
            let shown = own.iter().find(|keyword| keyword.kind() == kind).cloned();
            remove_kind(ctx.state, card_id, kind);
            TuningChange::Keyword {
                keyword: shown.unwrap_or_else(|| Keyword::of_kind(kind, 0)),
                added: false,
            }
        }
        MenuApply::AddKeyword { candidates } => {
            let keyword = pick_one(ctx, candidates).clone();
            if let Some(card) = find_instance_mut(ctx.state, card_id) {
                let tuning = tuning_of(card);
                let mut added = tuning.add_keywords.clone().unwrap_or_default();
                added.push(keyword.clone());
                tuning.add_keywords = Some(added);
                // §10.4: a keyword the card gains anew is up, as a granted one is (`buff::grant_to`).
                if keyword.kind() == KeywordKind::DivineShield {
                    card.divine_shield_spent = None;
                }
                if keyword.kind() == KeywordKind::Reborn {
                    card.reborn_spent = None;
                }
            }
            TuningChange::Keyword { keyword, added: true }
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
    }
}

/// B3.4 rules 1–3: the rows that can change the card now, in the menu's order.
fn menu_of(state: &GameState, card: &CardInstance, direction: TuneDirection) -> Vec<MenuRow> {
    // Rule 2: an Immutable card is never changed, and nothing is drawn for it.
    if has_keyword(&keywords_now(state, card), KeywordKind::Immutable) {
        return Vec::new();
    }
    [
        cost_row(state, card, direction),
        stats_row(state, card, direction),
        keyword_row(state, card, direction),
        x_row(state, card, direction),
        number_row(state, card, direction),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// B3.4 rule 3: the rows that can change the card now, in the menu's order — a pure read, for a
/// `conditionMet` or a `targetChecks` predicate (a Degrade with nothing to change) and for the tests.
pub fn applicable_changes(state: &GameState, card: &CardInstance, direction: TuneDirection) -> Vec<TuneRow> {
    menu_of(state, card, direction)
        .into_iter()
        .map(|row| row.row)
        .collect()
}

// ---------------------------------------------------------------------------
// One application, and the cards a verb reaches
// ---------------------------------------------------------------------------

/// B3.4 rule 1: one Degrade or Upgrade of one card — a row drawn among those that can change it, then
/// the change. `matches` is false for a card of a hidden pile the scope's filters left out, which is
/// cued and never changed (R440); TS's default is `true`. Reported as this file's header says.
///
/// TS read the live card on every application; the card is read again by its id here, so a second
/// application of the same card draws from the menu the first one left.
pub fn tune_once(ctx: &mut EffectContext<'_>, card: &CardInstance, direction: TuneDirection, matches: bool) {
    let card = find_instance(ctx.state, &card.id)
        .cloned()
        .unwrap_or_else(|| card.clone());
    // R177: who could not read the card where it changed, judged before the change moves anything.
    let hidden_from = unreadable_by(ctx.state, &card);
    let menu = if matches {
        menu_of(ctx.state, &card, direction)
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
fn named_card(
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

/// The cards a verb over `args` reaches, each with whether it may change: the one named card, or a
/// scope's cards — every card of the scope, or `random` different ones (R60), drawn uniformly over
/// the scope with every card of a hidden pile in it whatever the filters say (R440), so the odds and
/// the count of what is reached hang on the piles' sizes and the public cards alone. The picks are
/// applied in the scope's order (R242), never in the order they were drawn. A pick of at least as many
/// cards as there are takes them all and draws nothing (R129). (TS took `{ target?, instanceId?,
/// scope?, random? }`; `times` is not read.)
pub fn reached_cards(ctx: &mut EffectContext<'_>, args: &TuneArgs) -> Vec<ReachedCard> {
    let Some(scope) = &args.scope else {
        return named_card(ctx, args.target.as_ref(), args.instance_id.as_deref())
            .map(|card| vec![ReachedCard { card, matches: true }])
            .unwrap_or_default();
    };
    let pool: Vec<ReachedCard> = cards_in_card_scope(
        ctx,
        scope,
        Some(&CardScopeOptions {
            whole_hidden_piles: Some(true),
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
        for reached in reached_cards(ctx, &args) {
            for _ in 0..times {
                tune_once(ctx, &reached.card, direction, reached.matches);
            }
        }
    })
}

/// B3.4, R386: Degrade, which players read as Nerf (R1320) — `times` applications to each card reached,
/// each one change drawn from the menu (this file's header). Classic+ #8 Withering Storm ("Nerf 4 random
/// cards in your opponent's deck": `{ scope: { side: "enemy", zones: ["library"] }, random: 4 }`), #72
/// Book of Nerf, #70 Chaos Machine, #73's "Nerf every card on your opponent's field and in their hand
/// three times".
pub fn degrade(args: TuneArgs) -> Effect {
    tune_effect("degrade", TuneDirection::Degrade, args)
}

/// B3.4, R386: Upgrade, which players read as Buff (R1320) — Degrade's mirror (B9 #7). Classic+ #69 Buff
/// Billy ("Buff this X times": `{ target: { of: "self" }, times: ctx.x }`), #71 Book of Buff, #70, #73's
/// "every card in your hand and deck twice", T-AI-10 Fine-Tuning.
pub fn upgrade(args: TuneArgs) -> Effect {
    tune_effect("upgrade", TuneDirection::Upgrade, args)
}

// ---------------------------------------------------------------------------
// KY's Constant: a number set outright (Classic+ #41)
// ---------------------------------------------------------------------------

/// Classic+ #41: set one number on a card to `value`. It is `tuning` (B3.4 rule 4), so it stays with
/// the card: the cost by `costMod`, attack and health by the stats delta that makes them read `value`
/// now (current health on the field), a numbered keyword and a declared number by `tuning.set` (a
/// Brittle count in force is set itself). The steps recorded on that number before are spent.
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
            let on_field = matches!(card.zone, Zone::Field { row: Row::Units, .. });
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
#[serde(untagged)]
pub enum NumberWhich {
    Ref(NumberRef),
    Id(String),
}

/// `setNumber`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SetNumberArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    pub which: NumberWhich,
    pub value: i32,
}

/// Classic+ #41 KY's Constant: change a number on a card to `value`. `which` names the number
/// (`numbers::NumberRef`, or its id as a prompt option carries it, `number_ref_id`), or is `"random"`: a
/// uniform pick (R60) among the card's numbers that are not `value` already, which changes nothing
/// and draws nothing when there is none (R129). The card is named as a Degrade's is. A number the
/// card does not have, and an Immutable card (B3.4 rule 2), are left alone.
///
/// Reported by `numberChanged` with the number it came to (a declared number's bounds may hold it off
/// `value`), hidden like a Degrade's event (R177, `hiddenFrom`) — the base face picks at random on a
/// card in its caster's hand, so which number moved is the card's to keep.
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
        // TS read the live card after the write.
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
/// on the card that are not `value` already, drawn uniformly (R60; all of them, with no draw, when
/// there are no more than `count`, R129), offered to the card's controller in a `discover` prompt whose
/// options are the numbers' ids (`number_ref_id`) labelled with what each is now. The answer re-enters
/// the card's own step `step`, whose data carries the card's id under `NUMBER_CARD_KEY`; the step
/// reads the pick with `chosen_tuning_number` (not E18's `number` prompt reader: this answer names a
/// number on a card) and sets it with `set_number`. The options name the caster's own card's numbers
/// and go to the caster alone (R81). No number to offer: nothing opens, and the list goes on.
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
#[derive(Clone, Debug, PartialEq)]
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
