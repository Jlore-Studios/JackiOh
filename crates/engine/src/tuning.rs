//! Degrade and Upgrade's lasting changes to a card (docs/classic-sets.md B3.4, R386): the readers every
//! other module uses. A card's `tuning` rides it through every zone (R78 leaves it alone); what writes
//! it is `effects::tune`.
//!
//! Echo, Activate, Tribute (flags a script declares, not `Keyword`s) and an X-cost card's X are read
//! through `tuned_count`; the numbered `Keyword`s (Armor, Lucky, Brittle, Spell Damage) are tuned in
//! the layers (§10.4) by `tuned_keywords`, read by `layers::face_of`.
//!
//! `attack`/`health` are deltas beside the layer-4 buffs (B3.4 rule 6); `addKeywords`/`removeKeywords`
//! an Upgrade's additions and a Degrade's removed printed kinds; `x` and `numbers` step counts (each
//! step `TUNE_X_STEP` or `params::param_step`); `set` KY's Constant, steps counting from it. Keys: a
//! `params` key (camelCase) or a keyword kind (capitalised), never shared. The cost is `costMod` (R65).

use indexmap::IndexMap;

use crate::config::TUNE_MIN_AMOUNT;
use crate::state::CardInstance;
use crate::wire::{Keyword, KeywordKind, Tuning};

/// B3.4: the least a tuned number may come to — "an amount never drops below 1" (`TUNE_MIN_AMOUNT`).
pub const TUNED_FLOOR: i32 = TUNE_MIN_AMOUNT;

/// B3.4 rule 3: the tuning key of an X-cost card's X (`x_of`).
pub const X_KEY: &str = "X";

/// B3.4 rule 3: the numbered keywords that are `Keyword`s, which the layers tune (`tuned_keywords`).
/// Armor and Lucky print as keywords on any card; Brittle is also a count once started (`brittle.rs`);
/// Spell Damage is E6's.
pub const LAYERED_NUMBERED_KEYWORDS: &[KeywordKind] = &[
    KeywordKind::Armor,
    KeywordKind::Lucky,
    KeywordKind::Brittle,
    KeywordKind::SpellDamage,
];

/// B3.4 rule 3: the numbered keywords a script declares as data rather than as a `Keyword`, read by
/// their owners through `tuned_count` under these keys: Echo (`echo.rs`), Activate (B3.2's
/// `ActivationDecl.uses`), Tribute (`staticFlags.tribute`, where less is better).
pub const FLAG_NUMBERED_KEYS: &[&str] = &["Echo", "Activate", "Tribute"];

/// B3.4, R386: the value a card's numbered keyword or X has now — its printed value (or the value KY's
/// Constant set outright, `tuning.set`) moved by the card's tuning steps for `key`. A number the card
/// does not print (0) is never tuned into existence. Never below `TUNED_FLOOR` (`tuned_count_with_min`
/// takes another floor).
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

/// B2.7, B3.4: the X an X-cost card counts on the field and as it resolves — the X it was played for
/// moved by its X steps, never below 1 (R348's floor) — and 0 for a card that has no chosen X (a
/// Recruit, a copy, a card outside a play), as R65 reads an X card's cost off the field.
pub fn x_of(instance: &CardInstance) -> i32 {
    match instance.x {
        None => 0,
        Some(x) => tuned_count(instance, X_KEY, x),
    }
}

/// §10.4 layer 1 with B3.4 on top: a face's printed keywords as the card's tuning leaves them. The
/// printed kinds a Degrade removed go, a numbered keyword a Degrade, an Upgrade or KY's Constant moved
/// prints its tuned value (several printed entries of one kind — a fused Armor 7 and Armor 3 — are
/// tuned as their sum and print as one), and the keywords an Upgrade added are appended.
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
        let stacks = matches!(keyword.kind(), KeywordKind::Armor | KeywordKind::Lucky);
        if stacks || !out.iter().any(|held| held.kind() == keyword.kind()) {
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
    for record in [&mut tuning.x, &mut tuning.numbers, &mut tuning.set] {
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
}

/// Drop every empty field, and the record itself once nothing is left, so a card whose changes have
/// cancelled out stores and hashes exactly as a card never tuned (§9.3).
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

/// R102, B3.4 rule 4: a Fuse sums its ingredients' tuning — the stats and every step count add up,
/// the keyword changes unite, and a number KY's Constant set is kept from the first ingredient that
/// set it. `None` when no ingredient carries any.
pub fn sum_tunings(tunings: &[Option<Tuning>]) -> Option<Tuning> {
    let mut out = Tuning::default();
    for tuning in tunings.iter().flatten() {
        out.attack = Some(out.attack.unwrap_or(0) + tuning.attack.unwrap_or(0));
        out.health = Some(out.health.unwrap_or(0) + tuning.health.unwrap_or(0));
        for keyword in tuning.add_keywords.iter().flatten() {
            let stacks = matches!(keyword.kind(), KeywordKind::Armor | KeywordKind::Lucky);
            let held = out
                .add_keywords
                .iter()
                .flatten()
                .any(|held| held.kind() == keyword.kind());
            if stacks || !held {
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
            }
        }
    }
    if tidy(&mut out) { None } else { Some(out) }
}
