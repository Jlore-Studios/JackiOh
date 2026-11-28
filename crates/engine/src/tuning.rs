//! Degrade and Upgrade's lasting changes to a card (docs/classic-sets.md B3.4, R386). A card's `tuning`
//! rides it through every zone (R78 leaves it alone); what writes it is `effects::tune`.
//!
//! Echo, Activate, Tribute and an X-cost card's X are read through `tuned_count` wherever the engine
//! reads them; the numbered `Keyword`s are tuned in the layers (§10.4) by `tuned_keywords`.
//!
//! `attack`/`health` are summed deltas beside the layer-4 buffs, moving max health on the field and the
//! face a card will enter with anywhere else (B3.4 rule 6); `x` and `numbers` count steps per numbered
//! keyword or X and per declared number (each step `TUNE_X_STEP`, or that number's own, so a card made
//! Radiant keeps its steps on its Radiant numbers); `set` a number KY's Constant sets outright; `taken`
//! what End Times' countdown took off its price since its last use (R4828000). A declared number's key is a `params` key (camelCase), a keyword's its kind (capitalised), so the two
//! never share a key. The cost change is the card's `costMod` (R65), never a field here.

use indexmap::IndexMap;

use crate::config::{TUNE_MIN_AMOUNT, UNLUCKY_TUNED_FLOOR};
use crate::state::CardInstance;
use crate::wire::{Keyword, KeywordKind, Tuning};

/// B3.4: the least a tuned number may come to ("an amount never drops below 1").
pub const TUNED_FLOOR: i32 = TUNE_MIN_AMOUNT;

/// B3.4 rule 3: the tuning key of an X-cost card's X (`x_of`).
pub const X_KEY: &str = "X";

/// B3.4 rule 3: the numbered keywords that are `Keyword`s, which the layers tune (`tuned_keywords`).
/// Brittle is also a count once started (`brittle.rs`); Spell Damage is E6's; R5236005 Durability's X
/// and R5236021 Unlucky's, which alone is tuned from 0 (`unlucky_of`).
pub const LAYERED_NUMBERED_KEYWORDS: &[KeywordKind] = &[
    KeywordKind::Armor,
    KeywordKind::Lucky,
    KeywordKind::Brittle,
    KeywordKind::SpellDamage,
    KeywordKind::Durability,
    KeywordKind::Unlucky,
];

/// R5236020, R5236021: the kinds whose entries sum across their sources rather than count once (§6.1,
/// §10.4): Armor and Lucky, and Unlucky, which a Lucky offsets.
pub fn stacks(kind: KeywordKind) -> bool {
    matches!(kind, KeywordKind::Armor | KeywordKind::Lucky | KeywordKind::Unlucky)
}

/// R5236021: a card's Unlucky now: what it prints (or KY's Constant set) moved by its tuning steps, where
/// a Nerf's step adds one to a card printing none and a Buff's takes one off, never below
/// `UNLUCKY_TUNED_FLOOR` (0: no Unlucky).
pub fn unlucky_of(instance: &CardInstance, printed: i32) -> i32 {
    let key = KeywordKind::Unlucky.as_str();
    let tuning = instance.tuning.as_ref();
    let set = tuning
        .and_then(|t| t.set.as_ref())
        .and_then(|set| set.get(key))
        .copied();
    let step = tuning
        .and_then(|t| t.x.as_ref())
        .and_then(|x| x.get(key))
        .copied()
        .unwrap_or(0);
    UNLUCKY_TUNED_FLOOR.max(set.unwrap_or(printed) + step)
}

/// B3.4, R386: the value a card's numbered keyword or X has now: its printed value (or the value KY's
/// Constant set, `tuning.set`) moved by its tuning steps for `key`, never below `TUNED_FLOOR`. A number
/// the card does not print (0) is never tuned into existence.
pub fn tuned_count(instance: &CardInstance, key: &str, printed: i32) -> i32 {
    tuned_count_with_min(instance, key, printed, TUNED_FLOOR)
}

/// `tuned_count` with the floor given.
pub fn tuned_count_with_min(instance: &CardInstance, key: &str, printed: i32, min: i32) -> i32 {
    if printed <= 0 {
        return printed;
    }
    let tuning = instance.tuning.as_ref();
    let set = tuning
        .and_then(|t| t.set.as_ref())
        .and_then(|set| set.get(key))
        .copied();
    let step = tuning
        .and_then(|t| t.x.as_ref())
        .and_then(|x| x.get(key))
        .copied()
        .unwrap_or(0);
    if set.is_none() && step == 0 {
        return printed;
    }
    min.max(set.unwrap_or(printed) + step)
}

/// B2.7, B3.4: the X an X-cost card counts on the field and as it resolves, moved by its X steps,
/// never below 1 (R348), and 0 with no chosen X (a Recruit, a copy), as R65 reads its cost off the field.
/// R1848003: a card whose X is set (`fixed_x`) counts that X wherever no play chose one. An X-cost
/// card's is read at most `MANA_CEILING` (`mana::counted_x`, R5148000).
pub fn x_of(instance: &CardInstance) -> i32 {
    match instance.x.or(instance.fixed_x) {
        None => 0,
        Some(x) => tuned_count(instance, X_KEY, x),
    }
}

/// §10.4 layer 1 with B3.4 on top: a face's printed keywords as its tuning leaves them; the entries of
/// one numbered kind (a fused Armor 7 and Armor 3) are tuned as their sum and print as one.
pub fn tuned_keywords(keywords: &[Keyword], instance: &CardInstance) -> Vec<Keyword> {
    let Some(tuning) = instance.tuning.as_ref() else {
        return keywords.to_vec();
    };
    let removed: &[KeywordKind] = tuning.remove_keywords.as_deref().unwrap_or(&[]);
    let mut out: Vec<Keyword> = keywords
        .iter()
        .filter(|keyword| !removed.contains(&keyword.kind()))
        .cloned()
        .collect();

    for &kind in LAYERED_NUMBERED_KEYWORDS {
        let key = kind.as_str();
        let set = tuning.set.as_ref().and_then(|set| set.get(key));
        let step = tuning.x.as_ref().and_then(|x| x.get(key)).copied().unwrap_or(0);
        if set.is_none() && step == 0 {
            continue;
        }
        // R5236021: Unlucky is tuned from 0 — a Nerf's step gives a card printing none Unlucky 1 — and
        // a Buff's step that brings it to 0 takes it off.
        if kind == KeywordKind::Unlucky {
            let value = unlucky_of(instance, numbered_sum(&out, kind).unwrap_or(0));
            let first = out.iter().position(|keyword| keyword.kind() == kind);
            out.retain(|keyword| keyword.kind() != kind);
            if value > 0 {
                let at = first.unwrap_or(out.len()).min(out.len());
                out.insert(at, Keyword::Unlucky { n: value });
            }
            continue;
        }
        let Some(printed) = numbered_sum(&out, kind) else {
            continue;
        };
        let Some(first) = out.iter().position(|keyword| keyword.kind() == kind) else {
            continue;
        };
        let value = tuned_count(instance, key, printed);
        let mut at = 0;
        out.retain(|keyword| {
            let keep = keyword.kind() != kind || at == first;
            at += 1;
            keep
        });
        out[first] = Keyword::of_kind(kind, value);
    }

    for keyword in tuning.add_keywords.iter().flatten() {
        if stacks(keyword.kind()) || !out.iter().any(|held| held.kind() == keyword.kind()) {
            out.push(keyword.clone());
        }
    }
    out
}

/// The sum of a numbered keyword's entries in a list, or `None` when the list has none of that kind.
pub fn numbered_sum(keywords: &[Keyword], kind: KeywordKind) -> Option<i32> {
    let mut found = false;
    let mut sum = 0;
    for keyword in keywords {
        if keyword.kind() != kind {
            continue;
        }
        let Some(n) = keyword.n() else {
            continue;
        };
        found = true;
        sum += n;
    }
    if found { Some(sum) } else { None }
}

/// B3.4: whether the card carries any tuning at all.
pub fn is_tuned(instance: &CardInstance) -> bool {
    let Some(tuning) = instance.tuning.as_ref() else {
        return false;
    };
    tuning.attack.unwrap_or(0) != 0
        || tuning.health.unwrap_or(0) != 0
        || tuning.add_keywords.as_ref().is_some_and(|list| !list.is_empty())
        || tuning
            .remove_keywords
            .as_ref()
            .is_some_and(|list| !list.is_empty())
        || tuning.x.iter().flat_map(|x| x.values()).any(|step| *step != 0)
        || tuning
            .numbers
            .iter()
            .flat_map(|n| n.values())
            .any(|step| *step != 0)
        || tuning.set.as_ref().is_some_and(|set| !set.is_empty())
}

/// The card's tuning record, made on first write.
pub fn tuning_of(instance: &mut CardInstance) -> &mut Tuning {
    instance.tuning.get_or_insert_with(Tuning::default)
}

/// R4828000: what the card's own countdown has taken off its declared number `key` since its last use
/// (`Tuning.taken`), 0 when nothing.
pub fn taken_of(instance: &CardInstance, key: &str) -> i32 {
    instance
        .tuning
        .as_ref()
        .and_then(|tuning| tuning.taken.as_ref())
        .and_then(|taken| taken.get(key))
        .copied()
        .unwrap_or(0)
}

/// R4828000: writes `Tuning.taken` for `key`, dropping it at 0 so a card with nothing banked stores and
/// hashes as one that never banked (§9.3).
pub fn set_taken(instance: &mut CardInstance, key: &str, value: i32) {
    let tuning = tuning_of(instance);
    let mut taken = tuning.taken.clone().unwrap_or_default();
    if value == 0 {
        taken.shift_remove(key);
    } else {
        taken.insert(key.to_string(), value);
    }
    tuning.taken = Some(taken);
    tidy_tuning(instance);
}

/// Add `delta` to a step count, dropping a count that comes back to 0 so an untuned card stores nothing
/// (`shift_remove` keeps the other keys in order).
pub fn add_step(record: Option<&IndexMap<String, i32>>, key: &str, delta: i32) -> IndexMap<String, i32> {
    let mut out = record.cloned().unwrap_or_default();
    let next = out.get(key).copied().unwrap_or(0) + delta;
    if next == 0 {
        out.shift_remove(key);
    } else {
        out.insert(key.to_string(), next);
    }
    out
}

/// Drop every empty field of a record; true when nothing is left.
fn tidy(tuning: &mut Tuning) -> bool {
    if tuning.attack.unwrap_or(0) == 0 {
        tuning.attack = None;
    }
    if tuning.health.unwrap_or(0) == 0 {
        tuning.health = None;
    }
    if tuning.add_keywords.as_ref().is_none_or(|list| list.is_empty()) {
        tuning.add_keywords = None;
    }
    if tuning.remove_keywords.as_ref().is_none_or(|list| list.is_empty()) {
        tuning.remove_keywords = None;
    }
    for record in [
        &mut tuning.x,
        &mut tuning.numbers,
        &mut tuning.set,
        &mut tuning.taken,
    ] {
        if record.as_ref().is_some_and(|map| map.is_empty()) {
            *record = None;
        }
    }
    tuning.attack.is_none()
        && tuning.health.is_none()
        && tuning.add_keywords.is_none()
        && tuning.remove_keywords.is_none()
        && tuning.x.is_none()
        && tuning.numbers.is_none()
        && tuning.set.is_none()
        && tuning.taken.is_none()
}

/// Drop every empty field, and the record itself once nothing is left, so a card whose changes have
/// canceled out stores and hashes exactly as a card never tuned (§9.3).
pub fn tidy_tuning(instance: &mut CardInstance) {
    let Some(tuning) = instance.tuning.as_mut() else {
        return;
    };
    if tidy(tuning) {
        instance.tuning = None;
    }
}

/// A deep copy, so a copy's tuning shares nothing with its source's (R57, B3.4 rule 4).
pub fn copy_tuning(tuning: Option<&Tuning>) -> Option<Tuning> {
    tuning.cloned()
}

/// R102, B3.4 rule 4: a Fuse sums its ingredients' tuning; a number KY's Constant set is kept from the
/// first ingredient that set it, and a bank (`taken`, R4828000) goes with the set it records.
pub fn sum_tunings(tunings: &[Option<Tuning>]) -> Option<Tuning> {
    let mut out = Tuning::default();
    for tuning in tunings.iter().flatten() {
        out.attack = Some(out.attack.unwrap_or(0) + tuning.attack.unwrap_or(0));
        out.health = Some(out.health.unwrap_or(0) + tuning.health.unwrap_or(0));
        for keyword in tuning.add_keywords.iter().flatten() {
            let stacking = stacks(keyword.kind());
            let held = out
                .add_keywords
                .iter()
                .flatten()
                .any(|held| held.kind() == keyword.kind());
            if stacking || !held {
                out.add_keywords
                    .get_or_insert_with(Vec::new)
                    .push(keyword.clone());
            }
        }
        for kind in tuning.remove_keywords.iter().flatten() {
            if !out.remove_keywords.iter().flatten().any(|held| held == kind) {
                out.remove_keywords.get_or_insert_with(Vec::new).push(*kind);
            }
        }
        for (key, step) in tuning.x.iter().flatten() {
            out.x = Some(add_step(out.x.as_ref(), key, *step));
        }
        for (key, step) in tuning.numbers.iter().flatten() {
            out.numbers = Some(add_step(out.numbers.as_ref(), key, *step));
        }
        for (key, value) in tuning.set.iter().flatten() {
            let held = out.set.as_ref().is_some_and(|set| set.contains_key(key));
            if !held {
                out.set
                    .get_or_insert_with(IndexMap::new)
                    .insert(key.clone(), *value);
                if let Some(bank) = tuning.taken.as_ref().and_then(|taken| taken.get(key)) {
                    out.taken
                        .get_or_insert_with(IndexMap::new)
                        .insert(key.clone(), *bank);
                }
            }
        }
    }
    if tidy(&mut out) { None } else { Some(out) }
}
