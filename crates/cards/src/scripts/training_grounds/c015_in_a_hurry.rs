//! SPEC §8.11 Training Grounds 15 In a hurry, a card of the held Training Grounds set (R1420,
//! R2194000). STUB (#2194): no behaviour yet; P5 (#2198) writes its script and its tests
//! (CLAUDE.md rule 6), from `docs/training-grounds-set.md` TG5. Its catalog row, sound, flavour and
//! Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-015";

pub fn script() -> CardScripts {
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// STUB (#2194): the card's BUILD M13 must-pass tests replace this one with its script.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    #[test]
    fn the_stub_is_a_card_of_the_held_training_grounds_set() {
        crate::register_all();
        let def = catalog::def_of(None, super::ID);
        assert_eq!(def.set, SetName::TrainingGrounds);
        assert!(!set_ships(def.set));
    }
}
