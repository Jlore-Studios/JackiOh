//! IA #34.1 Husk (SPEC §8.13 row 34.1, §7), a token of the held Infinite Assault set
//! (R4577000).
//!
//! A textless (1) 1/1 → Radiant (1) 2/2: catalog data only, so both faces run the default
//! script. Mass Hallucination makes a plain Husk on both faces.
//!
//! Built by C2 (#4586).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-034-1";

pub fn script() -> CardScripts {
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #34.1 Husk — SPEC §8.13 row 34.1.
#[cfg(test)]
mod tests {
    use super::ID;

    #[test]
    fn a_textless_1_1_designers_word_radiant_2_2() {
        crate::register_all();
        let def = jackioh_engine::catalog::def_of(None, ID);
        for face in [&def.base, &def.radiant] {
            assert!(face.keywords.is_empty(), "no keywords");
            assert!(face.text.is_empty(), "no text");
        }
        assert_eq!((def.base.attack, def.base.health), (Some(1), Some(1)));
        assert_eq!((def.radiant.attack, def.radiant.health), (Some(2), Some(2)));
    }
}
