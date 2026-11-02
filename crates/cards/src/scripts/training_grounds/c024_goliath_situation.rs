//! TG #24 Goliath Situation (SPEC §8.11 row 24). (1) Field Trap, Rare. "Reveals when your opponent plays
//! a Unit: Cards in your hand that cost less than that Unit cost ({setCost})." (Radiant: a ({minCost})
//! Cost or more Unit, and cards that cost no more than that Unit.)
//! A Field Trap stays after firing (§5.1, R33), so each Unit the opponent plays fires it, each firing the
//! designer's "On reveal" (R2199005). C #10 Exile's trigger: §10.5's announce, the condition in `when`
//! (R99, R61), "that Unit's cost" the cost paid (R56, R70). Then M+ #3 Wondrous Wand's `set_cost_override`
//! on each hand card whose cost now (R65, R396: an X card
//! in hand costs 0) is under it (Radiant: at or under it). A later discount adds to the override and
//! never takes it below (0) (R65). The opponent never reads the new costs (R97).

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-024";

/// The cost paid for a Unit the opponent plays, if it is one this face answers.
fn unit_played(ctx: &EffectContext<'_>, event: &GameEvent, radiant: bool) -> Option<i32> {
    let GameEvent::CardAnnounced { player, card_type, cost_paid, .. } = event else {
        return None;
    };
    if *player == ctx.controller || *card_type != CardType::Unit {
        return None;
    }
    if radiant && *cost_paid < param(ctx, "minCost") {
        return None;
    }
    Some(*cost_paid)
}

fn goliath(radiant: bool) -> Script {
    let trigger = TriggerDef::new("goliath-situation", &[GameEventType::CardAnnounced], move |ctx, event| {
        let Some(cost) = unit_played(ctx, event, radiant) else {
            return vec![];
        };
        let set_cost = param(&*ctx, "setCost");
        zone_cards(ctx.state, ctx.controller, OffFieldZone::Hand)
            .into_iter()
            .filter(|card| {
                let now = cost_now(ctx.state, card);
                if radiant { now <= cost } else { now < cost }
            })
            .map(|card| {
                set_cost_override(json_as(json!({
                    "target": { "of": "instance", "instanceId": card.id },
                    "cost": set_cost,
                    "inHandOnly": true,
                })))
            })
            .collect()
    })
    .with_when(move |ctx, event| unit_played(ctx, event, radiant).is_some());
    Script {
        triggers: vec![trigger],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: goliath(false),
        radiant: goliath(true),
    }
}

// TG #24 Goliath Situation — SPEC §8.11 row 24, BUILD M13 row TG 24.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::{degrade_number, upgrade_number};

    const GOLIATH: &str = super::ID;
    // p1's hand, (1) to (5).
    const ONE: &str = "core-008";
    const TWO: &str = "core-002";
    const THREE: &str = "core-053";
    const FOUR: &str = "core-043";
    
    // p2's plays.
    const VANILLA: &str = "core-008"; // (1) Unit
    const BIGOT: &str = "core-002"; // (2) Unit
    const BIG_FELINOR: &str = "core-043"; // (4) Unit
    const TRUE_STRIKE: &str = "core-044"; // a Spell
    const TOKEN_MAKER: &str = "core-015"; // (1) Unit, Cry: summon a Rush Token
    const HIT_JOB: &str = "core-016"; // rides along so the turn does not auto-end (R82)

    fn setup(radiant: bool, p2_hand: Value) -> Scenario {
        crate::register_all();
        let mut hand = p2_hand.as_array().cloned().unwrap_or_default();
        hand.push(json!(HIT_JOB));
        scenario(json!({
            "seed": "goliath",
            "active": "p2",
            "p1": {
                "hand": [ONE, TWO, THREE, FOUR, { "def": ONE, "costOverride": 5 }],
                "backrow": [{ "def": GOLIATH, "radiant": radiant, "faceUp": false, "lane": 1 }],
            },
            "p2": { "hand": hand, "mana": 20 },
        }))
    }

    /// p1's hand costs, in hand order.
    fn costs(s: &Scenario) -> Vec<i32> {
        s.hand(P1).iter().map(|card| cost_now(s.state(), card)).collect()
    }

    fn fired(s: &Scenario) -> usize {
        s.events().iter().filter(|e| matches!(e, GameEvent::TrapFired { .. })).count()
    }

    #[test]
    fn r33_it_is_set_face_down() {
        let s = setup(false, json!([]));
        assert_eq!(s.backrow(P1, 1).unwrap().face_up, Some(false));
        assert_eq!(costs(&s), vec![1, 2, 3, 4, 5]);
    }

    mod base {
        use super::*;

        #[test]
        fn r2199005_an_opponents_4_unit_reveals_it_and_it_stays_face_up() {
            let mut s = setup(false, json!([BIG_FELINOR]));
            s.play(BIG_FELINOR, json!({ "zone": 1 }));

            assert_eq!(fired(&s), 1);
            let trap = s.backrow(P1, 1).expect("a Field Trap stays");
            assert_eq!(trap.def_id, GOLIATH);
            assert_ne!(trap.face_up, Some(false));
        }

        #[test]
        fn r56_your_1_2_and_3_cards_cost_0_your_4_and_5_do_not() {
            let mut s = setup(false, json!([BIG_FELINOR]));
            s.play(BIG_FELINOR, json!({ "zone": 1 }));

            assert_eq!(costs(&s), vec![0, 0, 0, 4, 5]);
        }

        #[test]
        fn r2199005_a_later_dearer_unit_fires_it_again() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "goliath",
                "active": "p2",
                "p1": {
                    "hand": [ONE, FOUR, { "def": TWO, "costOverride": 5 }, { "def": THREE, "costOverride": 5 }],
                    "backrow": [{ "def": GOLIATH, "faceUp": false, "lane": 1 }],
                },
                "p2": { "hand": [BIGOT, { "def": BIG_FELINOR, "costMod": 1 }, HIT_JOB], "mana": 20 },
            }));
            s.play(BIGOT, json!({ "zone": 1 }));
            assert_eq!(costs(&s), vec![0, 4, 5, 5]);

            // A (5) Unit fires it a second time: the (4) costs (0) now, the (5)s do not.
            s.play(BIG_FELINOR, json!({ "zone": 2 }));
            assert_eq!(fired(&s), 2);
            assert_eq!(costs(&s), vec![0, 0, 5, 5]);
        }

        #[test]
        fn r61_a_spell_or_a_summon_fires_nothing() {
            let mut s = setup(false, json!([TRUE_STRIKE, TOKEN_MAKER]));
            s.play(TRUE_STRIKE, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
            assert_eq!(fired(&s), 0);

            // The (1) Unit is played and fires it; the Rush Token its Cry summons is no play.
            s.play(TOKEN_MAKER, json!({ "zone": 1 }));
            assert_eq!(fired(&s), 1);
        }

        #[test]
        fn r61_its_controllers_own_unit_fires_nothing() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [BIG_FELINOR, ONE, TWO, HIT_JOB], "mana": 20,
                    "backrow": [{ "def": GOLIATH, "faceUp": false, "lane": 1 }],
                },
            }));
            s.play(BIG_FELINOR, json!({ "zone": 1 }));

            assert_eq!(fired(&s), 0);
            assert_eq!(s.backrow(P1, 1).unwrap().face_up, Some(false));
        }

        #[test]
        fn r56_a_discounted_unit_is_read_at_the_price_paid() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "goliath",
                "active": "p2",
                "p1": {
                    "hand": [ONE, TWO, THREE, FOUR],
                    "backrow": [{ "def": GOLIATH, "faceUp": false, "lane": 1 }],
                },
                "p2": { "hand": [{ "def": BIG_FELINOR, "costMod": -2 }, HIT_JOB], "mana": 20 },
            }));
            s.play(BIG_FELINOR, json!({ "zone": 1 }));

            // Paid (2): only the (1) is under it.
            assert_eq!(costs(&s), vec![0, 2, 3, 4]);
        }

        #[test]
        fn r97_the_opponent_never_reads_the_new_costs() {
            let mut s = setup(false, json!([BIG_FELINOR]));
            s.play(BIG_FELINOR, json!({ "zone": 1 }));

            let seen = s.view(P2);
            let changes: Vec<&GameEvent> =
                seen.events.iter().filter(|e| matches!(e, GameEvent::CostChanged { .. })).collect();
            assert!(!changes.is_empty());
            for change in changes {
                let json = serde_json::to_value(change).unwrap();
                assert_eq!(json["cost"], json!(HIDDEN_COST), "{json}");
            }
        }

        #[test]
        fn r386_set_cost_reads_through_param() {
            let mut s = setup(false, json!([BIG_FELINOR]));
            assert_eq!(degrade_number(&mut s, GOLIATH, "setCost"), 1);
            s.play(BIG_FELINOR, json!({ "zone": 1 }));

            assert_eq!(costs(&s), vec![1, 1, 1, 4, 5]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r2199005_a_1_unit_leaves_it_set_a_2_unit_fires_it() {
            let mut s = setup(true, json!([VANILLA, BIGOT]));
            s.play(VANILLA, json!({ "zone": 1 }));
            assert_eq!(fired(&s), 0);
            assert_eq!(s.backrow(P1, 1).unwrap().face_up, Some(false));

            s.play(BIGOT, json!({ "zone": 2 }));
            assert_eq!(fired(&s), 1);
        }

        #[test]
        fn r2199005_cards_costing_exactly_the_paid_cost_become_0_too() {
            let mut s = setup(true, json!([BIG_FELINOR]));
            s.play(BIG_FELINOR, json!({ "zone": 1 }));

            assert_eq!(costs(&s), vec![0, 0, 0, 0, 5]);
        }

        #[test]
        fn r386_min_cost_reads_through_param() {
            let mut s = setup(true, json!([VANILLA]));
            assert_eq!(upgrade_number(&mut s, GOLIATH, "minCost"), 1);
            s.play(VANILLA, json!({ "zone": 1 }));

            assert_eq!(fired(&s), 1);
            assert_eq!(costs(&s), vec![0, 2, 3, 4, 5]);
        }
    }
}
