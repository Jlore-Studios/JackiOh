//! SPEC §8.12 S+ #82 Endless Expedition (1) Spell, Quickdraw, Mythic, a card of the held Stinkyscape+
//! set (R1420, R3535000; R3569007, R3569008, R640).
//!   Base:    "For the rest of the game lose {mana} mana. Add a random Terminus card to your hand, it
//!            costs (3)."
//!   Radiant: "For the rest of the game lose {mana} mana. Add a random Radiant Terminus card to your
//!            hand, it costs (3)."
//!
//! Engine: `loseMaxMana` lowers the caster's persistent mana modifier (§2.3), so their max mana is
//! `mana` lower at every refresh from now on, an empty crystal being the one lost first and max mana
//! flooring at 0 (R3569008). The card added is one drawn from the Terminus pool, every Terminus card of
//! the shipped sets, tokens included (R3552001, R380), through the generator (R387), costing
//! (`TERMINUS_COST`) whatever it printed (R3569007). Quickdraw (R640) is a static flag beside the tag.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-082";

/// "It costs (3)": the added card's cost.
const TERMINUS_COST: i32 = 3;

fn expedition(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(move |ctx| {
            vec![
                lose_max_mana(json_as(json!({ "amount": param(ctx, "mana") }))),
                add_random_from_catalog(json_as(json!({
                    "query": { "tags": ["Terminus"], "withTokens": true },
                    "radiant": radiant,
                    "costOverride": TERMINUS_COST,
                }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: expedition(false),
        radiant: expedition(true),
    }
}

// S+ #82 Endless Expedition — SPEC §8.12 row 82 (R3569007, R3569008; R640, R3552001, R387).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";

    fn game(seed: &str, radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "library": filler(10), "mana": 4 },
            "p2": { "library": filler(10) },
        }));
        // Four mana crystals: the fourth turn's, full and open.
        let side = &mut s.state_mut().players[P1];
        side.turns_started = 4;
        side.mana.max = 4;
        side.mana.current = 4;
        for player in [P1, PlayerId::P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    /// The Terminus pool of the shipped sets, tokens included.
    fn pool() -> Vec<String> {
        crate::register_all();
        catalog::query(&json_as(json!({ "tags": ["Terminus"], "withTokens": true })))
            .into_iter()
            .map(|def| def.id.clone())
            .collect()
    }

    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1)
            .into_iter()
            .filter(|card| card.def_id != FILLER)
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r640_it_is_quickdraw() {
            assert_eq!(script().base.flags().quickdraw, Some(true));
            assert_eq!(script().radiant.flags().quickdraw, Some(true));
        }

        #[test]
        fn r3569007_r3569008_expedition_lowers_max_mana_and_adds_a_3_cost_terminus_card() {
            let pool = pool();
            for n in 0..6 {
                let mut s = game(&format!("expedition-base-{n}"), false);
                s.play(ID, json!({}));
                // Cast for (1) from 4: 3 left of a max that fell from 4 to 3.
                assert_eq!(s.state().players[P1].mana.max, 3, "seed {n}");
                assert_eq!(s.state().players[P1].mana.current, 3, "seed {n}");
                let made = added(&s);
                assert_eq!(made.len(), 1, "seed {n}: {made:?}");
                assert!(
                    pool.contains(&made[0].def_id),
                    "{} is no Terminus card",
                    made[0].def_id
                );
                assert!(!made[0].radiant);
                assert_eq!(
                    effective_cost(s.state(), &made[0], Default::default()),
                    TERMINUS_COST,
                    "{} costs (3)",
                    made[0].def_id
                );
            }
        }

        #[test]
        fn r3569008_the_max_stays_lower_at_every_refresh() {
            let mut s = game("expedition-refresh", false);
            s.play(ID, json!({}));
            s.end_turn().end_turn();
            assert_eq!(s.state().active, P1);
            // A turn on: the fifth, whose crystals would cap at 4, minus the one lost.
            assert_eq!(s.state().players[P1].mana.max, 3);
            assert_eq!(s.state().players[P1].mana.current, 3);
            s.end_turn().end_turn();
            assert_eq!(s.state().players[P1].mana.max, 3);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3569007_r3569008_radiant_adds_a_radiant_one() {
            let pool = pool();
            for n in 0..6 {
                let mut s = game(&format!("expedition-radiant-{n}"), true);
                s.play(ID, json!({}));
                assert_eq!(s.state().players[P1].mana.max, 3, "seed {n}");
                let made = added(&s);
                assert_eq!(made.len(), 1, "seed {n}: {made:?}");
                assert!(pool.contains(&made[0].def_id));
                assert!(made[0].radiant, "seed {n}");
                assert_eq!(
                    effective_cost(s.state(), &made[0], Default::default()),
                    TERMINUS_COST
                );
            }
        }
    }
}
