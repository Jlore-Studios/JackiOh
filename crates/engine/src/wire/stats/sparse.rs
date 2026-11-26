//! A part of a card's totals that most of them leave empty, held behind a pointer.
//!
//! The server cuts every statistics window and history from one snapshot of every patch's totals
//! (R3067000): on 2026-11-25 staging's held 329,000 `CardTotal`s. The parts R4552001, R4552002, R4672003
//! and R4673001 added to each (its two swing tallies, its choices and its two faces' drills) stood in
//! every one, empty or not, and took a `CardTotal` from 312 bytes to 1,600, so the snapshot outgrew the
//! game server's 1 GiB and loading it killed the server. A `Sparse` part that holds nothing is one null
//! pointer. It compares, is stored and reads back exactly as the value it holds, an absent one as the
//! empty value, so no figure and no stored total changes.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// What a `Sparse` holds: a value whose default is empty, and which adds into another of its kind.
pub trait SparsePart: Default + PartialEq {
    /// Whether it holds nothing: such a part is not kept.
    fn is_empty_part(&self) -> bool;

    /// Adds `other` into this one.
    fn add_part(&mut self, other: &Self);
}

/// A part most totals leave empty: `None` while it holds nothing.
#[derive(Clone, Debug)]
pub struct Sparse<T>(Option<Box<T>>);

impl<T> Default for Sparse<T> {
    fn default() -> Self {
        Sparse(None)
    }
}

impl<T: SparsePart> From<T> for Sparse<T> {
    fn from(part: T) -> Self {
        if part.is_empty_part() {
            Sparse(None)
        } else {
            Sparse(Some(Box::new(part)))
        }
    }
}

impl<T: SparsePart> Sparse<T> {
    /// The part, or `None` while it holds nothing.
    pub fn get(&self) -> Option<&T> {
        self.0.as_deref().filter(|part| !part.is_empty_part())
    }

    /// The part, or `empty` while there is none: for a reader that takes a value either way.
    pub fn or<'a>(&'a self, empty: &'a T) -> &'a T {
        self.0.as_deref().unwrap_or(empty)
    }

    /// The part to write into, an empty one made first if there is none.
    pub fn get_mut(&mut self) -> &mut T {
        self.0.get_or_insert_with(Box::default)
    }

    /// Whether it holds nothing.
    pub fn is_empty(&self) -> bool {
        self.0.as_deref().is_none_or(T::is_empty_part)
    }

    /// Adds `other` into this one. Adding an empty part makes nothing.
    pub fn add(&mut self, other: &Sparse<T>) {
        if let Some(other) = other.get() {
            self.get_mut().add_part(other);
        }
    }

    /// Adds a value into this one, as `add` does.
    pub fn add_value(&mut self, other: &T) {
        if !other.is_empty_part() {
            self.get_mut().add_part(other);
        }
    }

    /// The part, leaving none behind.
    pub fn take(&mut self) -> T {
        self.0.take().map(|part| *part).unwrap_or_default()
    }
}

/// As the values compare, an absent part as the empty value.
impl<T: SparsePart> PartialEq for Sparse<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self.0.as_deref(), other.0.as_deref()) {
            (None, None) => true,
            (Some(mine), Some(theirs)) => mine == theirs,
            (Some(held), None) | (None, Some(held)) => *held == T::default(),
        }
    }
}

impl<T: SparsePart + Eq> Eq for Sparse<T> {}

/// As the value, an absent part as the empty value.
impl<T: SparsePart + Serialize> Serialize for Sparse<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0.as_deref() {
            Some(part) => part.serialize(serializer),
            None => T::default().serialize(serializer),
        }
    }
}

/// As the value, an empty one kept as none.
impl<'de, T: SparsePart + Deserialize<'de>> Deserialize<'de> for Sparse<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(Sparse::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::stats::{CardChoices, CardStats, CardTotal, FaceDrills, FaceTally, SwingTally};

    use std::mem::size_of;

    /// What one card's totals may hold inline: the snapshot holds hundreds of thousands of them. A
    /// `CardTotal` was 312 bytes before R4552001 and 1,600 with its five parts inline (64-bit targets).
    const CARD_TOTAL_BUDGET_BYTES: usize = 400;

    #[test]
    fn an_empty_part_is_one_pointer_and_a_card_total_stays_within_its_budget() {
        assert_eq!(size_of::<Sparse<SwingTally>>(), size_of::<usize>());
        assert_eq!(size_of::<Sparse<CardChoices>>(), size_of::<usize>());
        assert_eq!(size_of::<Sparse<FaceDrills>>(), size_of::<usize>());
        let inline = size_of::<CardTotal>() - 5 * size_of::<usize>()
            + 2 * size_of::<SwingTally>()
            + size_of::<CardChoices>()
            + 2 * size_of::<FaceDrills>();
        assert!(
            size_of::<CardTotal>() <= CARD_TOTAL_BUDGET_BYTES,
            "a CardTotal is {} bytes (its parts inline, {inline})",
            size_of::<CardTotal>(),
        );
        assert!(Sparse::<CardChoices>::default().get().is_none());
    }

    #[test]
    fn a_part_reads_compares_and_is_stored_as_the_value_it_holds() {
        let mut held = CardChoices::default();
        held.variants.insert("v".into(), FaceTally::default());
        held.variants.get_mut("v").unwrap().radiant.games = 2;
        let part = Sparse::from(held.clone());
        assert_eq!(part.get(), Some(&held));
        assert_eq!(
            serde_json::to_value(&part).unwrap(),
            serde_json::to_value(&held).unwrap()
        );
        let read: Sparse<CardChoices> = serde_json::from_value(serde_json::to_value(&held).unwrap()).unwrap();
        assert_eq!(read, part);

        // An empty value is kept as none, and equals none.
        let empty = Sparse::from(CardChoices::default());
        assert!(empty.0.is_none());
        let mut made = Sparse::<CardChoices>::default();
        made.get_mut();
        assert!(made.0.is_some());
        assert_eq!(made, empty);
        assert_eq!(empty, made);
        assert_ne!(empty, part);
        let read: Sparse<CardChoices> = serde_json::from_str("{}").unwrap();
        assert!(read.0.is_none());

        // Adding an empty part makes none; adding a held one makes it.
        let mut sum = Sparse::<CardChoices>::default();
        sum.add(&empty);
        assert!(sum.0.is_none());
        sum.add(&part);
        sum.add(&part);
        assert_eq!(sum.get().unwrap().variants["v"].radiant.games, 4);
        assert_eq!(sum.take().variants["v"].radiant.games, 4);
        assert!(sum.is_empty());
    }

    #[test]
    fn a_card_stats_without_them_reads_and_writes_as_before() {
        let stats: CardStats = serde_json::from_str(
            r#"{"card":"core-x","inDeck":{"games":3,"wins":1,"draws":0},"openingHand":{"games":0,"wins":0,"draws":0},
                "goingFirst":{"games":0,"wins":0,"draws":0},"goingSecond":{"games":0,"wins":0,"draws":0},
                "played":{"games":0,"wins":0,"draws":0},"drawnNotPlayed":{"games":0,"wins":0,"draws":0}}"#,
        )
        .unwrap();
        assert!(stats.swing.0.is_none() && stats.created_swing.0.is_none() && stats.choices.0.is_none());
        let written = serde_json::to_value(&stats).unwrap();
        for absent in ["swing", "createdSwing", "choices"] {
            assert!(written.get(absent).is_none(), "{absent}: {written}");
        }
    }
}
