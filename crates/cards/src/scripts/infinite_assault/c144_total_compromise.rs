//! SPEC §8.13 IA #144 Total Compromise ((4) Spell, Mythic), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Steal every (2) Cost or less enemy Unit. Your opponent's cards cost (1) more during their next turn."
//!   Radiant: "Steal every (3) Cost or less enemy Unit. Your opponent's cards cost (1) more during their next turn."
//!
//! Every enemy Unit under the face's cost cap, read as it resolves (R65), stolen in R68 order — a
//! Unit with no room is destroyed (R15, R4205000) — then #123 Cost Injection's price rule for the
//! opponent's next turn (R4590004). The caps are fixed prices (`tests/cross/params.rs`, R1748003);
//! the text is reworded to "(N) Cost or less" (R432, IA11 rule 7).

use jackioh_engine::effects::{add_cost_rule, steal};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-144";

/// "Every (2) Cost or less enemy Unit" (base) or "(3) Cost or less" (Radiant): R65's cost where
/// each Unit stands as this resolves.
const CAP: i32 = 2;
const RADIANT_CAP: i32 = 3;

/// "Your opponent's cards cost (1) more during their next turn" on both faces.
const HIKE: i32 = 1;

fn compromise(cap: i32) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let scope: BoardScope = json_as(json!({ "side": "enemy" }));
            // R4590005: costs read as this resolves (R65), stealing in the scope's walk (R68).
            let victims: Vec<CardInstance> =
                cards_in_scope_aimed(&*ctx, &scope, TargetAim::Harm)
                    .into_iter()
                    .filter(|card| effective_cost(&*ctx.state, card, Default::default()) <= cap)
                    .collect();
            let mut effects: Vec<Effect> = victims
                .into_iter()
                .map(|victim| {
                    steal(json_as(json!({
                        "instanceId": victim.id,
                    })))
                })
                .collect();
            effects.push(add_cost_rule(json_as(json!({
                "player": "enemy",
                "rule": { "amount": HIKE },
                "lasts": "theirNextTurn",
            }))));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: compromise(CAP),
        radiant: compromise(RADIANT_CAP),
    }
}

// IA #144 Total Compromise — SPEC §8.13 row 144 (R65, R68, R4590004, R4590005).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const CHEAP: &str = "stand-in (2) 3/3 R6/6";
    const PRICEY: &str = "stand-in (3) 6/6 R12/12";
    const FILLER: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn board(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "mana": 10, "library": crate::filler(3) },
            "p2": { "hand": [FILLER], "field": [CHEAP, PRICEY], "library": crate::filler(3) },
        }))
    }

    fn cost_of(s: &Scenario, card: &CardInstance) -> i32 {
        jackioh_engine::mana::effective_cost(s.state(), card, Default::default())
    }

    mod base {
        use super::*;

        #[test]
        fn r4590005_steals_every_2_cost_or_less_enemy_unit_and_hikes_next_turn() {
            let mut s = board(false, "total-compromise");
            s.play(ID, json!({}));
            let cheap = s.unit_at(P1, 1);
            assert_eq!(cheap.def_id, CHEAP, "stolen into its lane");
            assert_eq!(cheap.controller, P1);
            let pricey = s.unit_at(P2, 2);
            assert_eq!(pricey.def_id, PRICEY, "too dear to steal");
            s.end_turn();
            let filler = s.card(FILLER).clone();
            assert_eq!(cost_of(&s, &filler), 2, "the hike holds on their turn");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4590005_steals_every_3_cost_or_less_enemy_unit_and_hikes_next_turn() {
            let mut s = board(true, "total-compromise-radiant");
            s.play(ID, json!({}));
            let units = jackioh_engine::zones::units_on_field_of(s.state(), P1);
            assert_eq!(units.len(), 2, "both Units stolen");
            assert!(units.iter().all(|unit| unit.controller == P1));
            s.end_turn();
            let filler = s.card(FILLER).clone();
            assert_eq!(cost_of(&s, &filler), 2, "the hike holds on their turn");
        }
    }
}
