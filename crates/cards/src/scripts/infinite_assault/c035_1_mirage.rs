//! IA #35.1 Mirage (SPEC §8.13 row 35.1, §7), a token of the held Infinite Assault set
//! (R4577000).
//!
//! A (1) 2/2, Rush → Radiant (1) 4/4, Rush: Rush is the designer's printed word on both faces,
//! kept by the token rule, so both faces run the default script. Fata Morgana summons a plain
//! Mirage on its base face and a Radiant one on its Radiant face.
//!
//! Built by C2 (#4586).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-035-1";

pub fn script() -> CardScripts {
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #35.1 Mirage — SPEC §8.13 row 35.1.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    #[test]
    fn rush_both_faces_stats_double() {
        crate::register_all();
        let def = jackioh_engine::catalog::def_of(None, ID);
        for face in [&def.base, &def.radiant] {
            assert_eq!(face.keywords, vec![Keyword::Rush], "the designer's printed word");
        }
        assert_eq!((def.base.attack, def.base.health), (Some(2), Some(2)));
        assert_eq!((def.radiant.attack, def.radiant.health), (Some(4), Some(4)));
    }
}
