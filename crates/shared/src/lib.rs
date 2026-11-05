//! The logic the server and its clients share (#2956, Part B), pure by CLAUDE.md rule 4: the server
//! calls it natively, and the browser and Roblox call it through a body in `jackioh-wasm-core`.
//! `README.md` is the contract (the host contract among it); this file is the map.
//!
//! The first thing in is the Conquest series rules, from `crates/server` (#3098):
//! [`series`] holds the rows and the transitions over them (SPEC §9.5, R330–R337), [`deck`] what a
//! ticket, a room or a series freezes of a player's decks, and [`config`] their numbers. Next is the
//! match host's decisions (#3101): [`host`] holds the clocks, the disconnect grace, the rematch offers and
//! the relays as one state machine, `step(host, event, now) -> (host, outputs)`. Then the loadout
//! visibility rule (SPEC §10.8, R1834003, #3102): [`loadout`] decides when a card's cosmetics may be shown
//! without leaking a decklist.

pub mod config;
pub mod deck;
pub mod host;
pub mod loadout;
pub mod series;

/// A field with three states — absent, `null`, a value — kept apart:
/// `None` is absent, `Some(None)` is `null`, `Some(Some(x))` is `x`. Used with
/// `#[serde(default, skip_serializing_if = "Option::is_none", with = "absent_or_null")]`.
pub mod absent_or_null {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer, T: Serialize>(
        value: &Option<Option<T>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(inner) => inner.serialize(serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
        deserializer: D,
    ) -> Result<Option<Option<T>>, D::Error> {
        Option::<T>::deserialize(deserializer).map(Some)
    }
}
