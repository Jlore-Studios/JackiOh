//! SPEC §8.13 IA #205 Doomsday Clock ((2) Unit, Rare, 0/8 → 0/16), a card of the held Infinite Assault set
//! (R4577000). STUB (#4602): no behaviour yet; C9 (#4593) writes its script and its tests (CLAUDE.md rule 6).
//!   Base:    "Can't attack"
//!             "In {turns|turn|turns}: Deal {damage} damage to the enemy hero."
//!   Radiant: "Can't attack"
//!             "In {turns|turn|turns}: Deal {damage} damage to the enemy hero."
//!
//! Its catalog entry, sound, flavour, Chinese, FX, art and audit rows are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-205";

pub fn script() -> CardScripts {
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// STUB (#4602): the card's part replaces this test with its script's tests.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    #[test]
    fn the_stub_is_a_card_of_the_held_infinite_assault_set() {
        crate::register_all();
        let def = catalog::def_of(None, super::ID);
        assert_eq!(def.set, SetName::InfiniteAssault);
        assert!(!set_ships(def.set));
    }
}
