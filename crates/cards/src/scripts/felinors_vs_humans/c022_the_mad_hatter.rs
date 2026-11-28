//! FvH #22 The Mad Hatter (SPEC §8.14 row 22): (2) Unit, Human, Epic, 2/4 → 4/8, of the Felinors vs
//! Humans set.
//!   Base:    "Cry: Cast a Scandalous Top Hat at each Unit."
//!   Radiant: "Cry: Cast a Radiant Scandalous Top Hat at each of your Units."
//!
//! Cry (R1): the Units on the field are read once as the Cry begins (R113), the Hatter itself among them:
//! its controller's in lane order, then on the base face the opponent's. For each, a `cast_new` of FvH #21
//! aimed at it alone (`aim_only`, R946, R1180005): each a cast (R70) that resolves before the next, so a
//! Unit the cast cannot aim at (Immune to Spells) gets none, and a Dormant card is no Unit (R1839000). The
//! base face's Hats on enemy Units feed the enemy's hand as those Units die (R1107); the Radiant face casts
//! Radiant Hats at its controller's Units only. Declares no numbers.

use jackioh_engine::prelude::*;

use crate::scripts::felinors_vs_humans::c021_scandalous_top_hat::ID as TOP_HAT;

pub const ID: &str = "felinorsvshumans-022";

/// The Units a Hat is cast at: the controller's in lane order, then, with `enemies_too`, the opponent's.
fn hat_stands(ctx: &EffectContext<'_>, enemies_too: bool) -> Vec<String> {
    let mut sides = vec![ctx.controller];
    if enemies_too {
        sides.push(opponent_of(ctx.controller));
    }
    let mut ids = Vec::new();
    for side in sides {
        ids.extend(active_units_of(&*ctx.state, side).into_iter().map(|unit| unit.id.clone()));
    }
    ids
}

fn mad_hatter(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(move |ctx: &mut EffectContext<'_>| hat_stands(ctx, !radiant)),
                each: Arc::new(move |aim: &str| {
                    cast_new(json_as(json!({
                        "def": TOP_HAT,
                        "radiant": radiant,
                        "aimAt": aim,
                        "aimOnly": true,
                    })))
                }),
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: mad_hatter(false),
        radiant: mad_hatter(true),
    }
}

// FvH #22 The Mad Hatter — SPEC §8.14 row 22 (R946, R1180005).
#[cfg(test)]
mod tests {
    use super::{ID, TOP_HAT};
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scripts::felinors_vs_humans::c021_scandalous_top_hat::HAT_DEATH;
    use crate::{filler, scenario};

    /// A sturdy vanilla Unit for each side's lane 1.
    const WALL: &str = "stand-in (1) 1/5";
    /// Stockpile: a Spell each hand keeps, so no turn ends by itself.
    const FILLER: &str = "core-005";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "mad-hatter",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": [WALL], "library": filler(4) },
            "p2": { "hand": [FILLER], "field": [WALL], "library": filler(4) },
        }))
    }

    /// The Hat grants the card wears, and the face of each.
    fn hats_worn(s: &Scenario, id: &str) -> Vec<bool> {
        s.card(id)
            .grants
            .iter()
            .flatten()
            .filter(|grant| grant.grant == HAT_DEATH)
            .map(|grant| grant.radiant)
            .collect()
    }

    /// The Top Hats cast this play, which land in p1's graveyard.
    fn hats_cast(s: &Scenario) -> usize {
        s.pile(P1, "graveyard")
            .iter()
            .filter(|card| card.def_id == TOP_HAT)
            .count()
    }

    mod base {
        use super::*;

        #[test]
        fn casts_a_top_hat_at_every_unit_on_both_sides_itself_included() {
            let mut s = board(false);
            let mine = s.unit_at(P1, 1).id;
            let theirs = s.unit_at(P2, 1).id;
            s.play(ID, json!({ "zone": 2 }));
            let hatter = s.unit_at(P1, 2).id;
            for id in [&mine, &hatter, &theirs] {
                assert_eq!(hats_worn(&s, id), vec![false], "{id} wears one plain Hat");
            }
            s.expect_stats(&mine, json!({ "attack": 2, "health": 6 }));
            s.expect_stats(&hatter, json!({ "attack": 3, "health": 5 }));
            s.expect_stats(&theirs, json!({ "attack": 2, "health": 6 }));
            assert_eq!(hats_cast(&s), 3, "one cast for each Unit");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn casts_a_radiant_top_hat_at_each_of_its_controllers_units_only() {
            let mut s = board(true);
            let mine = s.unit_at(P1, 1).id;
            let theirs = s.unit_at(P2, 1).id;
            s.play(ID, json!({ "zone": 2 }));
            let hatter = s.unit_at(P1, 2).id;
            for id in [&mine, &hatter] {
                assert_eq!(hats_worn(&s, id), vec![true], "{id} wears one Radiant Hat");
            }
            s.expect_stats(&mine, json!({ "attack": 4, "health": 8 }));
            s.expect_stats(&hatter, json!({ "attack": 7, "health": 11 }));
            assert!(hats_worn(&s, &theirs).is_empty(), "the enemy's Unit gets none");
            s.expect_stats(&theirs, json!({ "attack": 1, "health": 5 }));
            assert_eq!(hats_cast(&s), 2);
        }
    }
}
