//! "A number on a card" (docs/classic-sets.md B3.4, R386): the numbers Degrade, Upgrade and Classic+
//! #41 KY's Constant read and move — the card's own cost (never an X), its attack and health (a Unit,
//! or an Animated card, which prints the stats of the Unit it becomes), a numbered keyword's value
//! (Armor, Lucky, Spell Damage, Brittle, Echo, Activate, Tribute) and a declared number (`params.rs`).
//! This module reads them; `effects/tune.rs` moves them.
//!
//! Every read is a pure function of the card as it stands, so a card script may ask it (KY's
//! Constant's "a card in your hand with a number that isn't already 3" is a `targetChecks` predicate
//! over `numbers_on`) and `legal_actions`, `view_for` and the verb all read the same answer.
//!
//! Port of `packages/engine/src/numbers.ts`.

use serde::{Deserialize, Serialize};

use crate::brittle_count::active_brittle_count;
use crate::config::TUNE_MIN_AMOUNT;
use crate::layers::{card_keywords, printed_keywords_of, stats_with_buffs, unit_view};
use crate::script::{ActivationUses, Script, empty_script};
use crate::state::{CardInstance, GameState};
use crate::wire::{AttackHealth, CardType, Keyword, KeywordKind, ParamBetter, ParamTunedOn, Row, Zone, has_keyword};

/// B3.4 rule 3: the numbered keywords, by the tuning key each is read under (`tuning::tuned_count`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NumberedKey {
    Armor,
    Lucky,
    #[serde(rename = "Spell Damage")]
    SpellDamage,
    Brittle,
    Echo,
    Activate,
    Tribute,
}

impl NumberedKey {
    /// The literal, as TS writes it (and as the tuning keys it).
    pub fn as_str(self) -> &'static str {
        match self {
            NumberedKey::Armor => "Armor",
            NumberedKey::Lucky => "Lucky",
            NumberedKey::SpellDamage => "Spell Damage",
            NumberedKey::Brittle => "Brittle",
            NumberedKey::Echo => "Echo",
            NumberedKey::Activate => "Activate",
            NumberedKey::Tribute => "Tribute",
        }
    }
}

/// Which number on a card: its cost, attack or health, a numbered keyword, or a declared number.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum NumberRef {
    Cost,
    Attack,
    Health,
    Keyword { key: NumberedKey },
    Param { key: String },
}

/// One number on a card as `numbers_on` lists it: `id` names it in a prompt's option and in a
/// continuation's data (`number_ref_id`, `parse_number_ref`), `label` is what a client shows, `value` what
/// it is now.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NumberOnCard {
    pub id: String,
    #[serde(rename = "ref")]
    pub ref_: NumberRef,
    pub label: String,
    pub value: i32,
}

/// A numbered keyword on a card: its value now, which way is better for the card's controller, and
/// the printed value its tuning steps from (`tuning::tuned_count`) — null for a Brittle count in force,
/// which is a count on the instance and moves itself (B3.3 rule 5).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NumberedKeyword {
    pub key: NumberedKey,
    pub value: i32,
    pub better: ParamBetter,
    pub printed: Option<i32>,
}

/// The numbered keywords that print as `Keyword`s, in the order `numbers_on` lists them.
const PRINTED_NUMBERED: &[(NumberedKey, KeywordKind)] = &[
    (NumberedKey::Armor, KeywordKind::Armor),
    (NumberedKey::Lucky, KeywordKind::Lucky),
    (NumberedKey::SpellDamage, KeywordKind::SpellDamage),
];

/// The field's unit row: where a card's stats are read through all five layers (§10.4).
fn in_unit_row(card: &CardInstance) -> bool {
    matches!(card.zone, Zone::Field { row: Row::Units, .. })
}

/// R65, B3.4 rule 3: the card's own cost — `costOverride` or its printed cost (a computed or embiggen
/// price where R65 reads one), plus its `costMod` — with no player discount, since a Degrade changes
/// the card and not a price for a play. `None` for an X-cost card, whose X is no cost a number may move.
pub fn own_cost(state: &GameState, card: &CardInstance) -> Option<i32> {
    if crate::mana::is_x_cost(state, card) {
        return None;
    }
    let base = match card.cost_override {
        Some(cost) => cost,
        None => crate::mana::printed_cost(state, card),
    };
    Some(base + card.cost_mod)
}

/// B3.4 rule 3: whether the card has stats a number may move — a Unit, or an Animated card, which
/// prints the attack and health of the Unit it becomes (B3.1 rule 1).
pub fn has_stats(state: &GameState, card: &CardInstance) -> bool {
    if crate::faces::card_type_of(state, card) == CardType::Unit {
        return true;
    }
    let keywords = card_keywords(state, card);
    has_keyword(&keywords, KeywordKind::Animated) || has_keyword(&keywords, KeywordKind::AnimatedOnYourTurn)
}

/// B3.4 rule 6: the card's attack and current health as they stand — through all five layers in the
/// unit row (§10.4), and elsewhere its face with its buffs and tuning (§10.4 layers 1 to 4, R243) less
/// any damage it kept (an Animated card back in its backrow, B3.1 rule 5). `None` for a card with no
/// stats (`has_stats`).
pub fn current_stats(state: &GameState, card: &CardInstance) -> Option<AttackHealth> {
    if !has_stats(state, card) {
        return None;
    }
    if in_unit_row(card) {
        let view = unit_view(state, card);
        return Some(AttackHealth {
            attack: view.attack,
            health: view.health,
        });
    }
    let stats = stats_with_buffs(state, card);
    Some(AttackHealth {
        attack: stats.attack.max(0),
        health: stats.max_health - card.damage,
    })
}

/// B3.4 rule 3's numbered keywords on the card, with their values now: Armor, Lucky and Spell Damage
/// its running face prints (and no Degrade removed); its Brittle (the count in force, else the printed
/// one that has not started); and the Echo, Activate and Tribute its text declares (`staticFlags.echo`,
/// the numbered `ActivationDecl.uses`, `staticFlags.tribute`). A Vanilla card prints and declares none
/// of them (§6.3), though a Brittle count it was given stays (B3.3 rule 5).
pub fn numbered_keywords_on(state: &GameState, card: &CardInstance) -> Vec<NumberedKeyword> {
    let mut out: Vec<NumberedKeyword> = Vec::new();
    let add = |out: &mut Vec<NumberedKeyword>, key: NumberedKey, printed: i32, better: ParamBetter| {
        if printed > 0 {
            out.push(NumberedKeyword {
                key,
                value: crate::tuning::tuned_count(card, key.as_str(), printed),
                better,
                printed: Some(printed),
            });
        }
    };
    let removed: Vec<KeywordKind> = card
        .tuning
        .as_ref()
        .and_then(|tuning| tuning.remove_keywords.clone())
        .unwrap_or_default();
    let keywords: Vec<Keyword> = if card.vanilla {
        Vec::new()
    } else {
        printed_keywords_of(state, card)
    };
    for &(key, kind) in PRINTED_NUMBERED {
        if !removed.contains(&kind) {
            add(&mut out, key, crate::tuning::numbered_sum(&keywords, kind).unwrap_or(0), ParamBetter::Up);
        }
    }
    match active_brittle_count(card) {
        Some(count) => {
            if count > 0 {
                out.push(NumberedKeyword {
                    key: NumberedKey::Brittle,
                    value: count,
                    better: ParamBetter::Up,
                    printed: None,
                });
            }
        }
        None => add(
            &mut out,
            NumberedKey::Brittle,
            crate::tuning::numbered_sum(&keywords, KeywordKind::Brittle).unwrap_or(0),
            ParamBetter::Up,
        ),
    }
    let script = crate::scripts::script_of(state, card);
    let flags = script.flags();
    add(&mut out, NumberedKey::Echo, flags.echo.unwrap_or(0), ParamBetter::Up);
    add(
        &mut out,
        NumberedKey::Activate,
        activate_uses(&script).unwrap_or(0),
        ParamBetter::Up,
    );
    add(&mut out, NumberedKey::Tribute, flags.tribute.unwrap_or(0), ParamBetter::Down);
    out
}

/// B3.2 rule 9, B3.4: the printed N of the card's first numbered Activate ability ("Activate" is 1,
/// "Activate N" is N), or `None` when it has none — Activate ♾️ has no number to move.
fn activate_uses(script: &Script) -> Option<i32> {
    script.activations.iter().find_map(|ability| match ability.uses {
        ActivationUses::Count(uses) if uses > 0 => Some(uses),
        _ => None,
    })
}

/// The string a `NumberRef` travels as: in a prompt's option and in a continuation's data.
pub fn number_ref_id(ref_: &NumberRef) -> String {
    match ref_ {
        NumberRef::Keyword { key } => format!("keyword:{}", key.as_str()),
        NumberRef::Param { key } => format!("param:{key}"),
        NumberRef::Cost => "cost".to_string(),
        NumberRef::Attack => "attack".to_string(),
        NumberRef::Health => "health".to_string(),
    }
}

const NUMBERED_KEYS: &[NumberedKey] = &[
    NumberedKey::Armor,
    NumberedKey::Lucky,
    NumberedKey::SpellDamage,
    NumberedKey::Brittle,
    NumberedKey::Echo,
    NumberedKey::Activate,
    NumberedKey::Tribute,
];

/// The inverse of `number_ref_id`, or `None` for a string that names no number.
pub fn parse_number_ref(id: &str) -> Option<NumberRef> {
    match id {
        "cost" => return Some(NumberRef::Cost),
        "attack" => return Some(NumberRef::Attack),
        "health" => return Some(NumberRef::Health),
        _ => {}
    }
    if let Some(key) = id.strip_prefix("keyword:") {
        return NUMBERED_KEYS
            .iter()
            .find(|known| known.as_str() == key)
            .map(|&key| NumberRef::Keyword { key });
    }
    if let Some(key) = id.strip_prefix("param:") {
        return if key.is_empty() {
            None
        } else {
            Some(NumberRef::Param { key: key.to_string() })
        };
    }
    None
}

/// What the event that sets a number names it by (`numberChanged.key`): its stat, keyword or param key.
pub fn number_key(ref_: &NumberRef) -> String {
    match ref_ {
        NumberRef::Keyword { key } => key.as_str().to_string(),
        NumberRef::Param { key } => key.clone(),
        NumberRef::Cost => "cost".to_string(),
        NumberRef::Attack => "attack".to_string(),
        NumberRef::Health => "health".to_string(),
    }
}

/// B3.4, Classic+ #41 KY's Constant: every number on the card now, in a fixed order — cost, attack,
/// health, the numbered keywords, the declared numbers. An Immutable card has none a change may reach
/// (B3.4 rule 2), so it lists none.
pub fn numbers_on(state: &GameState, card: &CardInstance) -> Vec<NumberOnCard> {
    let keywords = if in_unit_row(card) {
        unit_view(state, card).keywords
    } else {
        card_keywords(state, card)
    };
    if has_keyword(&keywords, KeywordKind::Immutable) {
        return Vec::new();
    }
    let mut out: Vec<NumberOnCard> = Vec::new();
    let add = |out: &mut Vec<NumberOnCard>, ref_: NumberRef, label: String, value: i32| {
        out.push(NumberOnCard {
            id: number_ref_id(&ref_),
            ref_,
            label,
            value,
        });
    };
    if let Some(cost) = own_cost(state, card) {
        add(&mut out, NumberRef::Cost, "Cost".to_string(), cost);
    }
    if let Some(stats) = current_stats(state, card) {
        add(&mut out, NumberRef::Attack, "Attack".to_string(), stats.attack);
        add(&mut out, NumberRef::Health, "Health".to_string(), stats.health);
    }
    for entry in numbered_keywords_on(state, card) {
        add(
            &mut out,
            NumberRef::Keyword { key: entry.key },
            entry.key.as_str().to_string(),
            entry.value,
        );
    }
    for param in crate::params::params_of(state, &card.def_id).iter() {
        // R749: a number the base face does not print is not on it.
        if param.tuned_on == Some(ParamTunedOn::Radiant) && !card.radiant {
            continue;
        }
        // The key as a word, never a raw `{key}` placeholder (a label is shown as it is).
        let value = crate::params::param_value(state, Some(card), &param.key, Default::default());
        add(
            &mut out,
            NumberRef::Param { key: param.key.clone() },
            param.key.clone(),
            value,
        );
    }
    out
}

/// The number a `NumberRef` names on the card now, or `None` when the card has no such number.
pub fn number_on(state: &GameState, card: &CardInstance, ref_: &NumberRef) -> Option<i32> {
    let id = number_ref_id(ref_);
    numbers_on(state, card)
        .into_iter()
        .find(|entry| entry.id == id)
        .map(|entry| entry.value)
}
