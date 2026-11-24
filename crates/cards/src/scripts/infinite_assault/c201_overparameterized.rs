//! IA #201 Overparameterized (SPEC §8.13 row 201), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Costs ({discount}) less for each other Unit you control."
//!   Radiant: "Costs ({discount}) less for each other Unit you control."
//!
//! Engine: a cost hook as M #74 Montaña Giant's (R65, floor 0): the printed cost minus the
//! declared `discount` for each of its controller's active Units — every one of them another
//! Unit while this sits in the hand, the only place the hook reads it. Built by C9 (#4593).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-201";

fn overparameterized_cost() -> CostHook {
    cost_hook(move |CostArgs { state, instance }| {
        let printed = query_cost(&crate::card_def(ID));
        let at = &instance.zone;
        let for_play = matches!(at, Zone::Hand { .. })
            || (matches!(at, Zone::Graveyard { .. }) && playable_from_graveyard(state, instance));
        if !for_play {
            return printed;
        }
        let discount = param(
            &HookArgs {
                state,
                self_: instance,
                radiant: instance.radiant,
            },
            "discount",
        );
        let others = active_units_of(state, at.player()).len() as i32;
        (printed - discount * others).max(0)
    })
}

pub fn script() -> CardScripts {
    // The same hook: the Radiant face's (2) is its declared `discount`.
    CardScripts {
        base: Script {
            cost: Some(overparameterized_cost()),
            ..Script::default()
        },
        radiant: Script {
            cost: Some(overparameterized_cost()),
            ..Script::default()
        },
    }
}

// IA #201 Overparameterized — SPEC §8.13 row 201.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";

    fn hand_cost(s: &Scenario, player: PlayerId) -> i32 {
        let card = s
            .hand(player)
            .into_iter()
            .find(|held| held.def_id == ID)
            .expect("no Overparameterized in hand");
        effective_cost(s.state(), &card, Default::default())
    }

    fn holding(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "field": [VANILLA, { "def": VANILLA, "lane": 2 }],
                "hand": [{ "def": ID, "radiant": radiant }],
                "library": crate::filler(4),
            },
            "p2": {},
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn costs_1_less_for_each_other_unit() {
            let s = holding(false, "overparameterized-base");
            assert_eq!(hand_cost(&s, PlayerId::P1), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn costs_2_less_for_each_other_unit_floored_at_0() {
            let s = holding(true, "overparameterized-radiant");
            assert_eq!(hand_cost(&s, PlayerId::P1), 0);
        }
    }
}
