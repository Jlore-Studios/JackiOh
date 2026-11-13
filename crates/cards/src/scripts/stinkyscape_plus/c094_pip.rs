//! SPEC §8.12 S+ #94 PIP, a card of the Stinkyscape+ set (R1420, R3535000). (1) Spell, Rare.
//!   Base:    "Put a Unit on a performance enhancement plan, they have {count} turn to get a kill
//!            otherwise they are destroyed."
//!   Radiant: the same with {count2} turn, "If they fail, add {the} The Coins to your hand."
//!
//! A declared target: any Unit on either side (R81). `kill_deadline` (R3550006, E6) holds it to the end of
//! its controller's `count`th turn from now (R3557006): a Unit that destroys a Unit by then keeps its
//! life, and one that does not is destroyed (an Indestructible one is not, R46). The Radiant face adds
//! `the` The Coins (§7) to the caster's hand on a failure, a full hand burning what does not fit (§2.4).
//! The base face adds none.

use jackioh_engine::effects::kill_deadline;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-094";

fn pip(radiant: bool) -> Script {
    Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))],
        cry: Some(hook(move |ctx| {
            let (turns, coins) = if radiant {
                (param(ctx, "count2"), Some(param(ctx, "the")))
            } else {
                (param(ctx, "count"), None)
            };
            vec![kill_deadline(json_as(json!({
                "target": { "of": "chosen" },
                "coinsOnFail": coins,
                "turns": turns,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: pip(false),
        radiant: pip(true),
    }
}

// S+ #94 PIP — SPEC §8.12 row 94 (R3550006, R3557006).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005";
    const COIN: &str = "core-t-coin";
    const FIGHTER: &str = "stand-in (1) 5/5";
    const PREY: &str = "stand-in (1) 1/1";

    fn pick(id: &str) -> Value {
        json!({ "targets": [{ "pick": "instance", "instanceId": id }] })
    }

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "pip",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": [FIGHTER], "library": filler(8), "mana": 10 },
            "p2": { "hand": [FILLER], "field": [PREY], "library": filler(8) },
        }))
    }

    fn coins(s: &Scenario) -> usize {
        s.hand(P1).iter().filter(|card| card.def_id == COIN).count()
    }

    #[test]
    fn a_unit_that_gets_a_kill_in_time_lives() {
        crate::register_all();
        let mut s = board(false);
        let fighter = s.unit_at(P1, 1).id;
        let prey = s.unit_at(P2, 1).id;
        s.play(ID, pick(&fighter));
        s.attack(fighter.as_str(), prey.as_str());
        s.end_turn().end_turn().end_turn();
        s.expect_in_zone(fighter.as_str(), "field");
        assert_eq!(coins(&s), 0);
    }

    #[test]
    fn a_unit_without_a_kill_is_destroyed_and_the_base_face_adds_no_coin() {
        crate::register_all();
        let mut s = board(false);
        let fighter = s.unit_at(P1, 1).id;
        s.play(ID, pick(&fighter));
        s.end_turn().end_turn().end_turn();
        s.expect_in_zone(fighter.as_str(), "graveyard");
        assert_eq!(coins(&s), 0);
    }

    #[test]
    fn radiant_adds_three_coins_when_it_fails() {
        crate::register_all();
        let mut s = board(true);
        let fighter = s.unit_at(P1, 1).id;
        s.play(ID, pick(&fighter));
        s.end_turn().end_turn().end_turn();
        s.expect_in_zone(fighter.as_str(), "graveyard");
        assert_eq!(coins(&s), 3);
    }

    #[test]
    fn r3557006_two_turns_give_two_of_its_controllers_turns() {
        crate::register_all();
        let mut s = board(false);
        let fighter = s.unit_at(P1, 1).id;
        set_param(s.card_mut(ID), "count", 2);
        s.play(ID, pick(&fighter));
        s.end_turn().end_turn().end_turn();
        s.expect_in_zone(fighter.as_str(), "field");
        s.end_turn().end_turn();
        s.expect_in_zone(fighter.as_str(), "graveyard");
    }
}
