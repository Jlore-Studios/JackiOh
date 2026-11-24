//! IA #224 Rollback Point (SPEC §8.13 row 224), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Summon copies of the three most expensive friendly Units that died this game."
//!   Radiant: "Summon copies of the three most expensive friendly Units that died this game. They
//!            have Rush."
//!
//! The deaths are the game history's (`deaths_of`): one record per death under its controller
//! (R172), tokens in; a discard is no death. The three most expensive are read by printed cost
//! (`query_cost`), the list stable-sorted highest first; a tie at the cut is drawn from the match
//! rng, and with no tie no number is drawn (R129). Each is a fresh copy on its printed face, most
//! expensive first, with Rush on the Radiant face. With nothing to summon, nothing is drawn and
//! the cursor stands.
//!
//! Engine: `history::deaths_of`, `summon` (R4594005). Built by C10 (#4594).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-224";

/// How many of the dead come back.
const COPIES: i32 = 3;

fn rollback(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            // The controller's dead, highest printed cost first (a stable sort keeps the oldest of
            // equals first); defs the catalog has lost are passed by.
            let mut scored: Vec<(i32, String)> = deaths_of(&*ctx.state, ctx.controller)
                .iter()
                .filter_map(|death| {
                    find_def(Some(&*ctx.state), &death.def_id)
                        .map(|def| (query_cost(def), death.def_id.clone()))
                })
                .collect();
            scored.sort_by(|left, right| right.0.cmp(&left.0));
            if scored.is_empty() {
                return vec![];
            }
            let take = COPIES.min(scored.len() as i32) as usize;
            // A tie at the cut is drawn; without one the first three stand, oldest first.
            let mut chosen: Vec<String> = scored.iter().take(take).map(|(_, id)| id.clone()).collect();
            if take < scored.len() && scored[take - 1].0 == scored[take].0 {
                let cutoff = scored[take - 1].0;
                let above: Vec<String> = scored
                    .iter()
                    .filter(|(cost, _)| *cost > cutoff)
                    .map(|(_, id)| id.clone())
                    .collect();
                let tied: Vec<String> =
                    scored.iter().filter(|(cost, _)| *cost == cutoff).map(|(_, id)| id.clone()).collect();
                let drawn: Vec<String> = ctx.rng.shuffle(&tied);
                chosen = above.into_iter().chain(drawn).take(take).collect();
            }
            chosen
                .into_iter()
                .map(|def_id| {
                    let mut args = json!({ "defId": def_id, "radiant": radiant });
                    if radiant {
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
    CardScripts { base: rollback(false), radiant: rollback(true) }
}

// IA #224 Rollback Point — SPEC §8.13 row 224 (R4594005).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const WASTES: &str = "classicplus-012-6"; // Frozen Wastes: destroy all Units.
    const HINDER: &str = "core-021"; // Hinder: cast on draw, discard a random card.
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    const DEAR: &str = "stand-in (4) 6/6";
    const MID: &str = "stand-in (3) 5/5";
    const CHEAP: &str = "stand-in (2) 3/3";
    const ENEMY: &str = "stand-in (1) 2/2";

    fn field_ids(s: &Scenario) -> Vec<String> {
        s.state().players[P1]
            .units
            .iter()
            .flatten()
            .flatten()
            .map(|card| card.def_id.clone())
            .collect()
    }

    fn wipe_and_point(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "active": "p1",
            "p1": {
                "field": [DEAR, MID],
                "hand": [{ "def": ID, "radiant": radiant }, { "def": ID, "radiant": radiant }, CHEAP],
                "library": [HINDER, STOCKPILE, STOCKPILE, STOCKPILE],
            },
            "p2": { "field": [ENEMY], "hand": [WASTES, STOCKPILE], "library": crate::filler(4) },
        }));
        s.end_turn();
        // Frozen Wastes kills every Unit: two of p1's, one of p2's.
        s.play(WASTES, json!({}));
        s.end_turn();
        // The turn draw casts Hinder, discarding one of the hand at random; a Rollback survives it
        // either way, and the discarded card never died.
        s.play(ID, json!({}));
        s
    }

    mod base {
        use super::*;

        #[test]
        fn r4594005_the_two_most_expensive_come_back_most_expensive_first() {
            let s = wipe_and_point(false, "rollback-base");
            // The discarded hand card and the enemy's dead Unit are ignored.
            assert_eq!(field_ids(&s), vec![DEAR.to_string(), MID.to_string()]);
            for lane in [1, 2] {
                let view = unit_view(s.state(), &s.unit_at(P1, lane));
                assert!(!view.keywords.iter().any(|keyword| keyword.kind() == KeywordKind::Rush));
            }
        }

        #[test]
        fn r4594005_nothing_died_nothing_drawn() {
            let mut s = scenario(json!({
                "seed": "rollback-empty",
                "p1": { "hand": [ID, STOCKPILE], "library": crate::filler(4) },
                "p2": { "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            let before = s.state().rng_cursor;
            s.play(ID, json!({}));
            assert!(field_ids(&s).is_empty(), "nowhere to summon from");
            assert_eq!(s.state().rng_cursor, before, "no number drawn");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4594005_the_copies_have_rush() {
            let s = wipe_and_point(true, "rollback-radiant");
            assert_eq!(field_ids(&s), vec![DEAR.to_string(), MID.to_string()]);
            for lane in [1, 2] {
                let view = unit_view(s.state(), &s.unit_at(P1, lane));
                assert!(view.keywords.iter().any(|keyword| keyword.kind() == KeywordKind::Rush));
            }
        }
    }
}
