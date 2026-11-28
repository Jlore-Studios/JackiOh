//! SPEC §8.14 FvH #98.2 Dream Portal ((1) Spell, Token, printed Legendary), the token FvH #98 Yarma and
//! #98.1 Yarma Prime shuffle. Built by S10 (#5260).
//!   Base:    "Cast on draw: Summon a random Felinor."
//!   Radiant: "Cast on draw: Summon a random Radiant Felinor."
//!
//! Cast on draw (`StaticFlags.cast_on_draw`, §2.4, R58), its effect also its Cry, so a Portal played from
//! the hand does the same (Core #90.1 Super-Virus's shape): `summon_random` of a random non-token Felinor
//! Unit of the sets that ship (R380, R1420; the Felinor Token is no such card), Radiant on the Radiant face,
//! into its caster's leftmost open unit zone (R64), no Cry (R1); with no zone open, nothing (R129). A cast
//! on draw is no play (R2179002). Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-098-2";

fn dream_portal(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(move |_ctx| {
            vec![summon_random(json_as(json!({
                "query": { "type": "Unit", "tags": ["Felinor"] },
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: dream_portal(false),
        radiant: dream_portal(true),
    }
}

// FvH #98.2 Dream Portal — SPEC §8.14 row 98.2 (R1, R58, R64, R380, R2179002).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::weapon_board;

    const P1: PlayerId = PlayerId::P1;

    /// A textless card to fill a deck with.
    const FILLER: &str = "stand-in (9) Spell";

    /// The cards summoned in the last step, as (def id, Radiant).
    fn summoned(s: &Scenario) -> Vec<(String, bool)> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned {
                    instance_id,
                    def_id,
                    player,
                    ..
                } if *player == P1 => Some((def_id.clone(), s.card(instance_id.as_str()).radiant)),
                _ => None,
            })
            .collect()
    }

    /// R380: each card summoned is a non-token Felinor Unit, Radiant as the face says.
    fn summons_a_felinor(s: &Scenario, radiant: bool) {
        let summoned = summoned(s);
        assert_eq!(summoned.len(), 1);
        for (def_id, is_radiant) in nonempty(summoned, "the Felinor summoned") {
            let def = def_of(Some(s.state()), &def_id);
            assert_eq!(def.type_, CardType::Unit);
            assert!(def.tags.contains(&Tag::Felinor), "{def_id} is no Felinor");
            assert!(!def.token);
            assert_eq!(is_radiant, radiant);
        }
    }

    /// p1's next draw is a Dream Portal.
    fn drawn(radiant: bool) -> Scenario {
        let mut s = weapon_board(
            "fvh098-2-draw",
            json!({ "hand": [FILLER], "library": [{ "def": ID, "radiant": radiant }, FILLER] }),
            json!({}),
        );
        s.start_turn();
        s
    }

    fn played(radiant: bool) -> Scenario {
        let mut s = weapon_board(
            "fvh098-2-play",
            json!({ "hand": [{ "def": ID, "radiant": radiant }, FILLER], "mana": 10 }),
            json!({}),
        );
        s.play(ID, json!({}));
        s
    }

    mod base {
        use super::*;

        #[test]
        fn r58_drawn_it_casts_and_summons_a_random_felinor() {
            let s = drawn(false);
            summons_a_felinor(&s, false);
            assert!(s.hand(P1).iter().all(|card| card.def_id != ID));
        }

        #[test]
        fn played_from_the_hand_it_summons_a_random_felinor() {
            let s = played(false);
            summons_a_felinor(&s, false);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r58_drawn_it_casts_and_summons_a_random_radiant_felinor() {
            let s = drawn(true);
            summons_a_felinor(&s, true);
        }

        #[test]
        fn played_from_the_hand_it_summons_a_random_radiant_felinor() {
            let s = played(true);
            summons_a_felinor(&s, true);
        }
    }
}
