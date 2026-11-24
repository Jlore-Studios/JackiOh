//! SPEC §8.13 IA #13 Training Run ((2) Spell, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Discover an Infinite Assault card. It costs ({discount}) less." (discount 2)
//!   Radiant: "Discover an Infinite Assault card twice. Each costs ({discount}) less." (discount 2)
//!
//! S #89 Spice's Discover over the set-named pool (R1846001, R4577000; this card and the tokens left out, R387, §5.1), the pick
//! added with `costMod` −`discount` (a price never below (0), R65). The Radiant face opens a second
//! Discover once the first is answered, as Core #99 Craft a Card chains its (§10.6).

use jackioh_engine::effects::{add_to_hand, chosen_options, discover_from_catalog};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-013";

/// The set every "Infinite Assault card" names (R4577000).
const SET: &str = "Infinite Assault";

/// The resume steps: the first Discover's answer, and the Radiant face's second (§10.6).
const FIRST: &str = "training-run";
const SECOND: &str = "training-run-again";

fn discover(step: &str) -> Effect {
    discover_from_catalog(json_as(json!({
        "step": step,
        "query": { "set": SET },
        "prompt": "Discover an Infinite Assault card",
    })))
}

/// The answered pick, added at its discount.
fn add_pick(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(picked) = chosen_options(ctx).into_iter().next() else {
        return vec![];
    };
    vec![add_to_hand(json_as(json!({ "defId": picked, "costMod": -param(&*ctx, "discount") })))]
}

fn training_run(twice: bool) -> Script {
    let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
    if twice {
        resume.insert(
            FIRST,
            hook(|ctx| {
                let mut effects = add_pick(ctx);
                effects.push(discover(SECOND));
                effects
            }),
        );
        resume.insert(SECOND, hook(add_pick));
    } else {
        resume.insert(FIRST, hook(add_pick));
    }
    Script {
        cry: Some(hook(|_ctx| vec![discover(FIRST)])),
        resume,
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: training_run(false),
        radiant: training_run(true),
    }
}

// IA #13 Training Run — SPEC §8.13 row 13 (§10.6).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::catalog::def_of;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    fn board(radiant: bool) -> Scenario {
        scenario(json!({ "seed": "training-run", "p1": { "hand": [{ "def": ID, "radiant": radiant }] } }))
    }

    /// Answers the open Discover with its first option, after checking every option is an Infinite
    /// Assault card and no token.
    fn pick_first(s: &mut Scenario) -> String {
        let options = s.prompt_modes();
        for option in nonempty(options.clone(), "the Discover's options") {
            let def = def_of(Some(s.state()), &option);
            assert_eq!(def.set, SetName::InfiniteAssault, "{option}");
            assert!(!def.is_token(), "{option}");
        }
        let picked = options[0].clone();
        s.answer(pick::mode(&picked));
        picked
    }

    fn expect_discounted(s: &Scenario, picked: &str) {
        let card = s.hand(P1).into_iter().find(|card| card.def_id == picked).expect("the pick is in hand");
        let printed = query_cost(def_of(Some(s.state()), picked));
        assert_eq!(card.cost_mod, -2);
        assert_eq!(cost_now(s.state(), &card), (printed - 2).max(0));
    }

    mod base {
        use super::*;

        #[test]
        fn discovers_one_infinite_assault_card_that_costs_2_less() {
            let mut s = board(false);
            s.play(ID, json!({}));
            let picked = pick_first(&mut s);
            assert!(s.state().pending.is_none(), "one Discover");
            assert_eq!(s.hand(P1).len(), 1);
            expect_discounted(&s, &picked);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn discovers_twice_each_pick_costing_2_less() {
            let mut s = board(true);
            s.play(ID, json!({}));
            let first = pick_first(&mut s);
            let second = pick_first(&mut s);
            assert!(s.state().pending.is_none());
            assert_eq!(s.hand(P1).len(), 2);
            expect_discounted(&s, &first);
            expect_discounted(&s, &second);
        }
    }
}
