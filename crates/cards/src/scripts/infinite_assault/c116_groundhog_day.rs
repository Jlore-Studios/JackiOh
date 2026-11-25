//! SPEC §8.13 IA #116 Groundhog Day ((4) Spell, Epic), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Summon copies of the friendly Units that died since your last turn."
//!   Radiant: "Summon copies of the friendly Units that died since your last turn. They have Rush."
//!
//! One summon per record of the controller's `deaths_since_last_turn` log (R4589000, R4589001),
//! oldest first — every Unit, not one pick — each a fresh copy on the face it died with (R4577002),
//! with no Cry (R1). The Radiant face's copies land with Rush, IA #7 Bulk Order's `keywords`.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-116";

fn groundhog(rush: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            deaths_since_last_turn(&*ctx.state, ctx.controller)
                .into_iter()
                .map(|death| {
                    let mut args = json!({
                        "defId": death.def_id,
                        "radiant": death.radiant,
                    });
                    if rush {
                        args["keywords"] = json!([{ "kind": "Rush" }]);
                    }
                    summon(json_as(args))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: groundhog(false),
        radiant: groundhog(true),
    }
}

// IA #116 Groundhog Day — SPEC §8.13 row 116 (R4589001).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const HIT_JOB: &str = "core-016"; // Hit Job: the kill
    const FODDER: &str = "stand-in (1) 2/2";
    const OTHER: &str = "stand-in (1) 3/3";
    const FILLER: &str = "stand-in (0) Spell";

    /// p1's two Units die, then Groundhog Day is cast, with IA #97 in the library to keep the log.
    fn day(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "hand": [HIT_JOB, HIT_JOB, { "def": ID, "radiant": radiant }],
                "field": [FODDER, OTHER],
                "library": ["infiniteassault-097", FILLER, FILLER],
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(3) },
        }));
        let first = s.unit_at(P1, 1);
        s.play(HIT_JOB, pick::targets(pick::instance(&first)));
        let second = s.unit_at(P1, 2);
        s.play(HIT_JOB, pick::targets(pick::instance(&second)));
        s.play(ID, json!({}));
        s
    }

    fn summoned(s: &Scenario) -> Vec<CardInstance> {
        active_units_of(s.state(), P1)
            .into_iter()
            .filter(|unit| unit.def_id == FODDER || unit.def_id == OTHER)
            .cloned()
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r4589001_summons_the_deaths_since_last_turn() {
            let s = day(false);
            assert_eq!(summoned(&s).len(), 2, "both deaths come back");
            for copy in nonempty(summoned(&s), "the copies") {
                assert!(!has_keyword(&s.stats(&copy).keywords, KeywordKind::Rush));
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4589001_the_copies_have_rush() {
            let s = day(true);
            let back = summoned(&s);
            assert_eq!(back.len(), 2, "both deaths come back");
            for copy in nonempty(back, "the copies") {
                assert!(has_keyword(&s.stats(&copy).keywords, KeywordKind::Rush));
            }
        }
    }
}
