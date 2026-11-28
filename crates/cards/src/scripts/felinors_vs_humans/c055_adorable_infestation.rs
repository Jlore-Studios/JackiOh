//! FvH #55 Adorable Infestation (SPEC §8.14 row 55): (0) Spell, Common, of the Felinors vs Humans set.
//!   Base:    "Give a Unit +{buff}/+{buff}. Summon a Felinor Token and add a Felinor Token to your hand."
//!            (buff 1)
//!   Radiant: "Echo 1" and the same.
//!
//! One declared Unit on either side (R81) takes a `buff` of the declared `buff` (R386); then a `summon` of
//! a Felinor Token into the leftmost open unit zone (R64) and an `add_to_hand` of another, each the token's
//! base face (the text says 1/1 on both faces), whatever the buff found. The Radiant face prints Echo 1
//! (`StaticFlags.echo`, §6.2): it resolves once more and asks a new target, since it keeps none (R3048002).
//! The Echo's 1 is a keyword count, fixed (R1748003).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-055";

/// The token it summons and adds.
const FELINOR_TOKEN: &str = "core-t-felinor";

/// The Radiant face's Echo 1: one more resolution.
const RADIANT_ECHO: i32 = 1;

fn infestation(echo: Option<i32>) -> Script {
    Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))],
        cry: Some(hook(|ctx| {
            let amount = param(&*ctx, "buff");
            vec![
                buff(json_as(json!({ "target": { "of": "chosen" }, "attack": amount, "health": amount }))),
                summon(json_as(json!({ "defId": FELINOR_TOKEN }))),
                add_to_hand(json_as(json!({ "defId": FELINOR_TOKEN }))),
            ]
        })),
        static_flags: echo.map(|times| StaticFlags {
            echo: Some(times),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: infestation(None),
        radiant: infestation(Some(RADIANT_ECHO)),
    }
}

// FvH #55 Adorable Infestation — SPEC §8.14 row 55 (R64, R3048002).
#[cfg(test)]
mod tests {
    use super::{FELINOR_TOKEN, ID};
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    /// A sturdy vanilla Unit for each side's lane 1.
    const WALL: &str = "stand-in (1) 1/5";
    /// Stockpile: a Spell each hand keeps.
    const FILLER: &str = "core-005";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "adorable-infestation",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": [WALL], "library": filler(4) },
            "p2": { "hand": [FILLER], "field": [WALL], "library": filler(4) },
        }))
    }

    /// The Felinor Tokens on p1's field, lane by lane.
    fn tokens_on_field(s: &Scenario) -> Vec<CardInstance> {
        (1..=UNIT_ZONES)
            .filter_map(|lane| s.unit(P1, lane))
            .filter(|unit| unit.def_id == FELINOR_TOKEN)
            .collect()
    }

    /// The Felinor Tokens in p1's hand.
    fn tokens_in_hand(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id == FELINOR_TOKEN).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r64_buffs_the_unit_then_summons_a_felinor_token_and_adds_another() {
            let mut s = board(false);
            let wall = s.unit_at(P1, 1).id;
            s.play(ID, pick::targets(pick::id(&wall)));
            s.expect_stats(&wall, json!({ "attack": 2, "health": 6 }));
            let summoned = tokens_on_field(&s);
            assert_eq!(summoned.len(), 1);
            assert_eq!(s.unit(P1, 2).map(|unit| unit.def_id), Some(FELINOR_TOKEN.to_string()), "the leftmost open zone");
            assert!(!summoned[0].radiant, "the token's base face");
            let added = tokens_in_hand(&s);
            assert_eq!(added.len(), 1);
            assert!(!added[0].radiant);
            assert!(s.state().pending.is_none(), "no Echo on the base face");
        }

        #[test]
        fn it_may_buff_an_enemy_unit_and_the_tokens_are_still_its_casters() {
            let mut s = board(false);
            let theirs = s.unit_at(P2, 1).id;
            s.play(ID, pick::targets(pick::id(&theirs)));
            s.expect_stats(&theirs, json!({ "attack": 2, "health": 6 }));
            assert_eq!(tokens_on_field(&s).len(), 1);
            assert_eq!(tokens_in_hand(&s).len(), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3048002_echo_1_resolves_again_on_a_new_target() {
            let mut s = board(true);
            let wall = s.unit_at(P1, 1).id;
            let theirs = s.unit_at(P2, 1).id;
            s.play(ID, pick::targets(pick::id(&wall)));
            assert!(s.state().pending.is_some(), "the Echo repeat asks its own target");
            s.answer(pick::id(&theirs));
            s.expect_stats(&wall, json!({ "attack": 2, "health": 6 }));
            s.expect_stats(&theirs, json!({ "attack": 2, "health": 6 }));
            assert_eq!(tokens_on_field(&s).len(), 2, "a token summoned each time");
            assert_eq!(tokens_in_hand(&s).len(), 2, "and one added each time");
        }
    }
}
