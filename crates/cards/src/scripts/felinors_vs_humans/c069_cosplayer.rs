//! SPEC §8.14 FvH #69 Cosplayer ((1) Unit, Human, Felinor, Rare, 2/2 → 4/4), a card of the held Felinors vs
//! Humans set (R5235000).
//!   Base:    "Has +{stats}/+{stats} for each other Felinor or Human on the field." (stats 1)
//!   Radiant: "Rush\nHas +{stats}/+{stats} for each other Felinor or Human on the field." (stats 2)
//!
//! An aura over itself alone (§10.4 layer 5, IA #80's shape): the declared `stats` for every other Unit on
//! either side's field whose tags now hold Felinor or Human (`tags_of`, granted ones included, R923), a Unit
//! with both counted once, read live. A Dormant card is no Unit to count (R1839000). FvH #88 Cattail prints
//! the Felinor tag, so it counts. The Radiant face's Rush is catalog data. Built by S07 (#5257).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-069";

/// "+N/+N for each other Felinor or Human on the field".
fn cosplay() -> AuraHook {
    aura_hook(move |ctx| {
        let per = param(&ctx, "stats");
        let me = ctx.self_.id.clone();
        let others = PLAYER_IDS
            .into_iter()
            .flat_map(|side| active_units_of(ctx.state, side))
            .filter(|unit| unit.id != me)
            .filter(|unit| {
                let tags = tags_of(ctx.state, unit);
                tags.contains(&Tag::Felinor) || tags.contains(&Tag::Human)
            })
            .count();
        let amount = per.saturating_mul(i32::try_from(others).unwrap_or(0));
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| unit.id == me),
            mod_: StatMod {
                attack: Some(amount),
                max_health: Some(amount),
                ..StatMod::default()
            },
        }]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        aura: Some(cosplay()),
        ..Script::default()
    };
    // The same script: the Radiant face's +2/+2 is its declared `stats`, its Rush catalog data.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #69 Cosplayer — SPEC §8.14 row 69.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const HUMAN: &str = "core-008"; // Mr. Vanilla: a Human
    const FELINOR: &str = "core-t-felinor"; // Felinor Token: a Felinor
    const UNTAGGED: &str = "core-019"; // Midrange Menace: no tag
    const HIT_JOB: &str = "core-016"; // Hit Job: the kill
    const FILLER: &str = "core-005"; // Stockpile: a card to hold

    fn board(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        let mut field = vec![json!({ "def": ID, "radiant": radiant })];
        field.extend(mine.as_array().cloned().unwrap_or_default());
        scenario(json!({
            "p1": { "hand": [HIT_JOB, FILLER], "field": field, "mana": 10, "library": crate::filler(4) },
            "p2": { "hand": [FILLER], "field": theirs, "library": crate::filler(4) },
        }))
    }

    fn cosplayer(s: &Scenario) -> CardInstance {
        s.unit_at(P1, 1)
    }

    mod base {
        use super::*;

        #[test]
        fn alone_it_is_a_2_2() {
            let mut s = board(false, json!([]), json!([UNTAGGED]));
            s.expect_stats(cosplayer(&s), json!({ "attack": 2, "health": 2 }));
        }

        #[test]
        fn it_counts_every_other_felinor_or_human_on_both_sides_read_live() {
            let mut s = board(false, json!([HUMAN, FELINOR, UNTAGGED]), json!([FELINOR, HUMAN, UNTAGGED]));
            s.expect_stats(cosplayer(&s), json!({ "attack": 2 + 4, "health": 2 + 4 }));
            // A death drops one.
            let human = s.unit_at(P2, 2);
            s.play(HIT_JOB, pick::targets(pick::instance(&human)));
            s.expect_stats(cosplayer(&s), json!({ "attack": 2 + 3, "health": 2 + 3 }));
        }

        #[test]
        fn a_unit_with_both_tags_counts_once() {
            // Another Cosplayer is a Human and a Felinor: one count each way.
            let mut s = board(false, json!([ID]), json!([]));
            s.expect_stats(cosplayer(&s), json!({ "attack": 2 + 1, "health": 2 + 1 }));
            let other = s.unit_at(P1, 2);
            s.expect_stats(&other, json!({ "attack": 2 + 1, "health": 2 + 1 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn it_has_plus_2_plus_2_for_each_and_rush() {
            let mut s = board(true, json!([HUMAN]), json!([FELINOR, UNTAGGED]));
            let me = cosplayer(&s);
            s.expect_stats(&me, json!({ "attack": 4 + 4, "health": 4 + 4 }));
            assert!(s.stats(&me).keywords.iter().any(|keyword| keyword.kind() == KeywordKind::Rush));
        }
    }
}
