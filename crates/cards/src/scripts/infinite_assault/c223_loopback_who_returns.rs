//! SPEC §8.13 IA #223 Loopback, Who Returns ((4) Unit, Legendary, 6/6 → 12/12), a card of the held Infinite Assault set
//! (R4577000). STUB (#4602): no behaviour yet; C10 (#4594) writes its script and its tests (CLAUDE.md rule 6).
//!   Base:    "Reborn"
//!             "When this returns by Reborn, it gains +{buff}/+{buff} and Reborn, up to three times a game."
//!   Radiant: "Reborn"
//!             "When this returns by Reborn, it gains +{buff}/+{buff} and Reborn, up to three times a game."
//!
//! Its catalog entry, sound, flavour, Chinese, FX, art and audit rows are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-223";

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
