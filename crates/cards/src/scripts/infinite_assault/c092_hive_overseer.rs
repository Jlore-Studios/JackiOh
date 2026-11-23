//! SPEC §8.13 IA #92 Hive Overseer ((3) Unit, Epic, 2/8 → 4/16), a card of the held Infinite Assault set
//! (R4577000). STUB (#4602): no behaviour yet; C4 (#4588) writes its script and its tests (CLAUDE.md rule 6).
//!   Base:    "Taunt"
//!             "Whenever a friendly Unit dies, summon a Drone."
//!   Radiant: "Taunt"
//!             "Whenever a friendly Unit dies, summon a Radiant Drone."
//!
//! Its catalog entry, sound, flavour, Chinese, FX, art and audit rows are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-092";

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
