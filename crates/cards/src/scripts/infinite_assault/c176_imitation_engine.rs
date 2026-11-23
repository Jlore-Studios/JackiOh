//! SPEC §8.13 IA #176 Imitation Engine ((3) Unit, Common, 4/6 → 8/12), a card of the held Infinite Assault set
//! (R4577000). STUB (#4602): no behaviour yet; C8 (#4592) writes its script and its tests (CLAUDE.md rule 6).
//!   Base:    "Cry: Set this's Attack to the highest Attack among enemy Units."
//!   Radiant: "Cry: Set this's Attack to the highest Attack and its Health to the highest Health among enemy Units."
//!
//! Its catalog entry, sound, flavour, Chinese, FX, art and audit rows are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-176";

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
