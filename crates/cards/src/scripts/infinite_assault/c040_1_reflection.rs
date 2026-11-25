//! IA #40.1 Reflection (SPEC §8.13 row 40.1, §7), a token of the held Infinite Assault set
//! (R4577000).
//!
//! A (1) 1/3, Taunt → Radiant (1) 2/6, Taunt: Taunt is the designer's printed word on both
//! faces, kept by the token rule, so both faces run the default script. Hall of Mirrors
//! summons a plain Reflection on its base face and a Radiant one on its Radiant face.
//!
//! Built by C2 (#4586).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-040-1";

pub fn script() -> CardScripts {
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #40.1 Reflection — SPEC §8.13 row 40.1.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    #[test]
    fn taunt_both_faces_stats_double() {
        crate::register_all();
        let def = jackioh_engine::catalog::def_of(None, ID);
        for face in [&def.base, &def.radiant] {
            assert_eq!(face.keywords, vec![Keyword::Taunt], "the designer's printed word");
        }
        assert_eq!((def.base.attack, def.base.health), (Some(1), Some(3)));
        assert_eq!((def.radiant.attack, def.radiant.health), (Some(2), Some(6)));
    }
}
