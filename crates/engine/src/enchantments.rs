//! Enchantments that ride a card (docs/classic-sets.md B5 E39): lasting instructions stored on the
//! instance and kept in every zone, which R78's reset leaves alone. Each is read where its rule acts:
//! `returnAfterResolve` by the play pipeline's step 7 and by the cost calculation (its floor, E15),
//! `castOnDraw` by the draw (§2.4), `targetEnemies` by a random cast's target picks (E12).
//!
//! Port of `packages/engine/src/enchantments.ts`.

use std::borrow::Borrow;

use serde::{Deserialize, Serialize};

use crate::state::CardInstance;
use crate::wire::Enchantment;

/// `Enchantment["kind"]`: the four kinds, as the wire writes them.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EnchantmentKind {
    #[serde(rename = "returnAfterResolve")]
    ReturnAfterResolve,
    #[serde(rename = "castOnDraw")]
    CastOnDraw,
    #[serde(rename = "targetEnemies")]
    TargetEnemies,
    #[serde(rename = "swapsBook")]
    SwapsBook,
}

impl EnchantmentKind {
    /// The literal, as TS writes it.
    pub fn as_str(self) -> &'static str {
        match self {
            EnchantmentKind::ReturnAfterResolve => "returnAfterResolve",
            EnchantmentKind::CastOnDraw => "castOnDraw",
            EnchantmentKind::TargetEnemies => "targetEnemies",
            EnchantmentKind::SwapsBook => "swapsBook",
        }
    }
}

/// `entry.kind`.
pub fn enchantment_kind(entry: &Enchantment) -> EnchantmentKind {
    match entry {
        Enchantment::ReturnAfterResolve { .. } => EnchantmentKind::ReturnAfterResolve,
        Enchantment::CastOnDraw => EnchantmentKind::CastOnDraw,
        Enchantment::TargetEnemies => EnchantmentKind::TargetEnemies,
        Enchantment::SwapsBook { .. } => EnchantmentKind::SwapsBook,
    }
}

pub fn enchantments_of(instance: &CardInstance) -> &[Enchantment] {
    instance.enchantments.as_deref().unwrap_or(&[])
}

/// The card's enchantments of one kind, in the order they were given.
pub fn enchantments_of_kind(instance: &CardInstance, kind: EnchantmentKind) -> Vec<Enchantment> {
    enchantments_of(instance)
        .iter()
        .filter(|entry| enchantment_kind(entry) == kind)
        .cloned()
        .collect()
}

pub fn has_enchantment(instance: &CardInstance, kind: EnchantmentKind) -> bool {
    enchantments_of(instance)
        .iter()
        .any(|entry| enchantment_kind(entry) == kind)
}

/// Two enchantments are one when every field agrees, so a card is never given the same one twice.
/// (TS compared `JSON.stringify`s; SURFACE §4.4.3 makes that `==` on the derived `PartialEq`.)
fn same_enchantment(a: &Enchantment, b: &Enchantment) -> bool {
    a == b
}

/// B5 E39: put an enchantment on a card. It rides the card through every zone from now on and R78's
/// reset never takes it (`zones::reset_instance`); one the card already carries is not added twice, while
/// two of a kind that differ (two floors) are both kept for their reader to combine. Returns whether
/// the card gained it. Also what the play pipeline stamps with (Classic+ #14 Forever&'s "the next Spell
/// you play gains …").
pub fn add_enchantment(instance: &mut CardInstance, enchantment: &Enchantment) -> bool {
    let held: Vec<Enchantment> = instance.enchantments.clone().unwrap_or_default();
    if held.iter().any(|entry| same_enchantment(entry, enchantment)) {
        return false;
    }
    let mut next = held;
    next.push(enchantment.clone());
    instance.enchantments = Some(next);
    true
}

/// R443: what a copy or a Fuse carries — every enchantment of the cards it is made from, once each, in
/// their order. `None` when none carries any, so the card stores nothing.
pub fn united_enchantments<I>(instances: I) -> Option<Vec<Enchantment>>
where
    I: IntoIterator,
    I::Item: Borrow<CardInstance>,
{
    let mut out: Vec<Enchantment> = Vec::new();
    for instance in instances {
        for entry in enchantments_of(instance.borrow()) {
            if !out.iter().any(|held| same_enchantment(held, entry)) {
                out.push(entry.clone());
            }
        }
    }
    if out.is_empty() { None } else { Some(out) }
}
