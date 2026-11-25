//! SPEC §8.13 IA #113 Rerun ((3) Spell, Epic), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cast the last three Spells you played this game, at random targets."
//!   Radiant: "Cast the last four Spells you played this game, at random targets."
//!
//! The last `COUNT` entries of the controller's `played_this_game` log (R4589000) whose type is
//! Spell and whose id is not this card's (R4589002: never itself), oldest first, each cast new on
//! the face it was played with (`cast_new`, R57's fresh copy) with every choice random (R452).
//! A random cast inside a cast is capped by `RANDOM_CAST_CHAIN_CAP`.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-113";

/// "the last three Spells", the base face's.
const COUNT: usize = 3;
/// "the last four Spells", the Radiant face's.
const RADIANT_COUNT: usize = 4;

fn rerun(count: usize) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let view: &EffectContext = ctx;
            let mut spells: Vec<_> = played_this_game(&*view.state, view.controller)
                .into_iter()
                .filter(|record| record.type_ == CardType::Spell && record.def_id != ID)
                .collect();
            if spells.len() > count {
                spells.drain(..spells.len() - count);
            }
            spells
                .into_iter()
                .map(|record| {
                    cast_new(CastNewArgs {
                        def: CastNewDef::Def(CastDef {
                            def_id: record.def_id,
                            radiant: Some(record.radiant),
                        }),
                        radiant: None,
                        how: CastHow {
                            random: Some(true),
                            ..CastHow::default()
                        },
                    })
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: rerun(COUNT),
        radiant: rerun(RADIANT_COUNT),
    }
}

// IA #113 Rerun — SPEC §8.13 row 113 (R4589002).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::recursion_log::played_this_game;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "stand-in (0) Spell";
    const BOLT: &str = "stand-in (0) Spell";

    /// p1 casts four Bolts, then Rerun: the base face casts the last three again, the Radiant
    /// face the last four.
    fn rerun(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "hand": [BOLT, BOLT, BOLT, BOLT, { "def": ID, "radiant": radiant }],
                "library": ["infiniteassault-097", FILLER, FILLER, FILLER],
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(3) },
        }));
        for _ in 0..4 {
            s.play(BOLT, json!({}));
        }
        s.play(ID, json!({}));
        s
    }

    fn bolts(s: &Scenario) -> usize {
        played_this_game(s.state(), P1)
            .iter()
            .filter(|record| record.def_id == BOLT)
            .count()
    }

    mod base {
        use super::*;

        #[test]
        fn r4589002_casts_the_last_spells_not_itself() {
            let s = rerun(false);
            // Four Bolts played, three cast again; Rerun itself is never among them.
            assert_eq!(bolts(&s), 7);
            assert_eq!(played_this_game(s.state(), P1).len(), 8);
        }

        #[test]
        fn a_cast_with_no_spells_does_nothing() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": ID }],
                    "library": ["infiniteassault-097", FILLER, FILLER],
                    "mana": 10,
                },
                "p2": { "hand": [FILLER], "library": crate::filler(3) },
            }));
            s.play(ID, json!({}));
            assert_eq!(played_this_game(s.state(), P1).len(), 1, "only Rerun itself");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4589002_radiant_casts_the_last_four() {
            let s = rerun(true);
            assert_eq!(bolts(&s), 8);
            assert_eq!(played_this_game(s.state(), P1).len(), 9);
        }
    }
}
