//! SPEC §8.13 IA #172 Training Data ((2) Spell, Common), a card of the held Infinite Assault set
//! (R4577000). STUB (#4602): no behaviour yet; C8 (#4592) writes its script and its tests (CLAUDE.md rule 6).
//!   Base:    "Add copies of the last two cards your opponent played to your hand."
//!   Radiant: "Add copies of the last three cards your opponent played to your hand."
//!
//! Its catalog entry, sound, flavour, Chinese, FX, art and audit rows are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-172";

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
