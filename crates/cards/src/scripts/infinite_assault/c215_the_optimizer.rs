//! SPEC §8.13 IA #215 The Optimizer ((3) Unit, Legendary, 3/6 → 6/12), a card of the held Infinite Assault set
//! (R4577000). STUB (#4602): no behaviour yet; C9 (#4593) writes its script and its tests (CLAUDE.md rule 6).
//!   Base:    "End of turn: Transform a random card in your hand into a random Infinite Assault card that costs
//!             "({step}) more."
//!   Radiant: "End of turn: Transform two random cards in your hand into random Infinite Assault cards that cost
//!             "({step}) more."
//!
//! Its catalog entry, sound, flavour, Chinese, FX, art and audit rows are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-215";

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
