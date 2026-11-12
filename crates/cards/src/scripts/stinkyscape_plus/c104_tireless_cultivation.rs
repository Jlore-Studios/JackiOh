//! SPEC §8.12 S+ #104 Tireless Cultivation, a card of the Stinkyscape+ set (R1420, R3535000). (1)
//! Spell, Rare, Temporary.
//!   Base:    "Temporary. Transform a permanent into one. It costs (1) more. End of turn: Return this to
//!            your hand."
//!   Radiant: the same, "It costs (2) more."
//!
//! A Spell (the designer's list names no type, SP5, R3557004). A declared target: one permanent of either
//! side, a Unit or a backrow card (R81). `transform_cost_step` replaces it in place by a card of its type
//! that costs `STEP` more (R3557004). Temporary is catalog data, 1 on both faces; the Radiant face's 2 is
//! granted to the card as it returns (`RADIANT_TEMPORARY`). At the end of the turn it was cast on, the spell goes from the graveyard back to its owner's hand,
//! a full hand burning it (§2.4, R4), as Core #23 Reoccurring Dream returns; the cleanup after the
//! end-of-turn steps then counts the returned card down (R637), so the base face's Temporary 1 discards it
//! at once and the Radiant face's Temporary 2 keeps it one more turn (R3557004).

use jackioh_engine::effects::{bounce, grant_keyword, transform_cost_step};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-104";

/// "It costs (1) more" on the base face, "(2) more" on the Radiant face.
const BASE_STEP: i32 = 1;
const RADIANT_STEP: i32 = 2;
/// The Radiant face's "Temporary 2": a printed Temporary carries no count (the catalog refuses one), so
/// the card gives itself the count each time it returns, and the larger entry counts (R2179001).
const RADIANT_TEMPORARY: i32 = 2;

/// §5.1 and R68: it returns when it was played, or returned, this turn.
fn returns_to_hand(ctx: &EffectContext<'_>) -> bool {
    let Some(self_) = ctx.live_self() else {
        return false;
    };
    self_.return_to_hand_at_end_of_turn == Some(true) || was_played_this_turn(ctx.state, self_.controller, self_)
}

fn cultivation(step: i32) -> Script {
    Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "backrow"] }))],
        cry: Some(hook(move |_ctx| {
            vec![transform_cost_step(json_as(json!({ "target": { "of": "chosen" }, "step": step })))]
        })),
        end_of_turn: Some(hook(move |ctx| {
            if !returns_to_hand(ctx) {
                return vec![];
            }
            let mut effects = vec![bounce(json_as(json!({ "target": { "of": "self" } })))];
            if step == RADIANT_STEP
                && let Some(this) = ctx.live_self()
            {
                effects.push(grant_keyword(json_as(json!({
                    "target": { "of": "instance", "instanceId": this.id },
                    "keyword": { "kind": "Temporary", "n": RADIANT_TEMPORARY },
                }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: cultivation(BASE_STEP),
        radiant: cultivation(RADIANT_STEP),
    }
}

// S+ #104 Tireless Cultivation — SPEC §8.12 row 104 (R3557004).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005";

    /// A card's printed price, the one the pool of a cost-step Transform reads (R3557004).
    fn printed(def_id: &str) -> i32 {
        jackioh_engine::catalog::query_cost(catalog::def_of(None, def_id))
    }

    fn pick(id: &str) -> Value {
        json!({ "targets": [{ "pick": "instance", "instanceId": id }] })
    }

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "tireless",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": ["core-008"], "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "field": ["core-008"], "library": filler(4) },
        }))
    }

    fn cost_of(s: &Scenario, id: &str) -> i32 {
        printed(&s.card(id).def_id)
    }

    #[test]
    fn r3557004_one_permanent_of_either_side_becomes_one_that_costs_one_more() {
        crate::register_all();
        for side in [P1, P2] {
            let mut s = board(false);
            let old = s.unit_at(side, 1).id;
            let cost = cost_of(&s, &old);
            s.play(ID, pick(&old));
            let new = s.unit(side, 1).expect("a Unit stands there");
            assert_ne!(new.id, old);
            assert_eq!(printed(&new.def_id), cost + 1);
        }
    }

    #[test]
    fn r3557004_radiant_costs_two_more() {
        crate::register_all();
        let mut s = board(true);
        let old = s.unit_at(P2, 1).id;
        let cost = cost_of(&s, &old);
        s.play(ID, pick(&old));
        let new = s.unit(P2, 1).expect("a Unit stands there");
        assert_eq!(printed(&new.def_id), cost + 2);
    }

    #[test]
    fn r3557004_base_returns_at_the_end_of_the_turn_and_its_temporary_1_discards_it_at_once() {
        crate::register_all();
        let mut s = board(false);
        let old = s.unit_at(P2, 1).id;
        s.play(ID, pick(&old));
        s.expect_in_zone(ID, "graveyard");
        s.end_turn();
        let kinds: Vec<&str> = s.last_events().iter().map(|event| event.event_type().as_str()).collect();
        let bounced = kinds.iter().position(|kind| *kind == "bounced");
        let discarded = kinds.iter().position(|kind| *kind == "discarded");
        assert!(bounced.is_some() && discarded.is_some() && bounced < discarded, "{kinds:?}");
        s.expect_in_zone(ID, "graveyard");
    }

    #[test]
    fn r3557004_radiant_returns_and_stays_in_the_hand_with_one_turn_of_its_temporary_2_left() {
        crate::register_all();
        let mut s = board(true);
        let old = s.unit_at(P2, 1).id;
        s.play(ID, pick(&old));
        s.end_turn();
        s.expect_in_zone(ID, "hand");
        assert_eq!(s.card(ID).temporary, Some(1));
    }

    #[test]
    fn r23_an_immutable_permanent_is_no_target() {
        let mut s = scenario(json!({
            "p1": { "hand": [ID, FILLER], "mana": 10, "library": filler(4) },
            "p2": { "hand": [FILLER], "field": [{ "def": "core-066", "radiant": true }], "library": filler(4) },
        }));
        let rock = s.unit_at(P2, 1).id;
        s.expect_refused(|s| s.play(ID, pick(&rock)));
    }
}
