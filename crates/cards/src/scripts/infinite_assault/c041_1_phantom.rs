//! IA #41.1 Phantom (SPEC §8.13 row 41.1, §7), a token of the held Infinite Assault set
//! (R4577000).
//!
//! A (1) 3/1, Rush, Brittle 1 → Radiant (1) 6/2, Rush, Brittle 1: Rush and Brittle 1 are the
//! designer's printed word on both faces, kept by the token rule, so both faces run the default
//! script. Phantom Army fills with plain Phantoms on its base face and Radiant ones on its
//! Radiant face.
//!
//! Built by C2 (#4586).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-041-1";

pub fn script() -> CardScripts {
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #41.1 Phantom — SPEC §8.13 row 41.1.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    #[test]
    fn rush_and_brittle_1_both_faces_stats_double() {
        crate::register_all();
        let def = jackioh_engine::catalog::def_of(None, ID);
        for face in [&def.base, &def.radiant] {
            assert_eq!(
                face.keywords,
                vec![Keyword::Rush, Keyword::Brittle { n: 1 }],
                "the designer's printed word"
            );
        }
        assert_eq!((def.base.attack, def.base.health), (Some(3), Some(1)));
        assert_eq!((def.radiant.attack, def.radiant.health), (Some(6), Some(2)));
    }
}
