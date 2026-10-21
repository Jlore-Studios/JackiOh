//! Meditative+ #1.4 Leg Day (SPEC §8.9 row 1.4, §7, R1178002, R1563003). (2) Spell, AI, Token: an AI
//! generated card M+ #1 Bob's Plan offers after M #20 Aestheticize the Game (#1563).
//!   Both faces: "Give your Units +{buff}/+{buff}." — buff 5, Radiant 10
//!
//! M #20's board line (your Units total 100 Attack and 100 Health, Radiant 90) wants big Units: every
//! Unit you control gains the stats (#43's `buff_all_units`, in lane order, one `buffed` each). With no
//! Unit it does nothing. The Radiant ten is the declared `buff`, so both faces run this one script.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-001-4";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            let amount = param(&*ctx, "buff");
            vec![buff_all_units(json_as(json!({ "attack": amount, "health": amount })))]
        })),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Meditative+ #1.4 Leg Day — SPEC §8.9 row 1.4, BUILD M11 row M+ 1.4: "gives every Unit its caster
// controls +5/+5, the opponent's none; with no Unit, nothing; buff reads through `param()`; radiant
// +10/+10".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005";
    const VANILLA: &str = "core-008"; // 4/4.
    const TIMMY: &str = "core-011"; // 3/3.

    fn leg_day(radiant: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": "leg-day",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": [VANILLA, TIMMY] },
            "p2": { "hand": [FILLER], "field": [VANILLA] },
        }))
    }

    #[test]
    fn r1563003_gives_your_units_5_5_and_the_opponents_nothing() {
        let mut s = leg_day(false);
        s.play(ID, json!({}));
        s.expect_stats(s.unit(P1, 1).expect("Vanilla"), json!({ "attack": 9, "maxHealth": 9 }));
        s.expect_stats(s.unit(P1, 2).expect("Timmy"), json!({ "attack": 8, "maxHealth": 8 }));
        s.expect_stats(s.unit(P2, 1).expect("theirs"), json!({ "attack": 4, "maxHealth": 4 }));
    }

    #[test]
    fn r1563003_radiant_gives_10_10() {
        let mut s = leg_day(true);
        s.play(ID, json!({}));
        s.expect_stats(s.unit(P1, 1).expect("Vanilla"), json!({ "attack": 14, "maxHealth": 14 }));
    }

    #[test]
    fn r386_buff_reads_through_param() {
        let mut s = leg_day(false);
        step_param(s.card_mut(ID), "buff", 1);
        s.play(ID, json!({}));
        s.expect_stats(s.unit(P1, 2).expect("Timmy"), json!({ "attack": 9, "maxHealth": 9 }));
    }
}
