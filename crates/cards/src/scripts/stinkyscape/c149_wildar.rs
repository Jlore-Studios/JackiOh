//! SPEC §8.10 Stinkyscape 149 Wildar (R60, R97, R380, R1420, R1882000). (3) Spell, Legendary.
//!   Base:    "Add a random Call to Chaos to your opponent's hand. It costs (1). Then add a random Radiant
//!            Call to Chaos to your hand."
//!   Radiant: "Add a random Call to Chaos to your opponent's hand. It costs (1). Then add a random
//!            Radiant Call to Chaos to your hand. It costs (1)." (through v0.5.9f "Add a random Radiant
//!            Call to Chaos to your hand.", `script_v0_5_9f`, kept, R767000)
//!
//! Two independent picks (R60, R1882000) from the Call to Chaos pool, the cards tagged Call to Chaos of
//! the sets that ship (R380; the set's own two join it as it ships, R1420): the first a fresh card of the
//! opponent's, owned by them, at `costOverride` 1, its edition hidden from its caster (R97); then the
//! second its caster's, Radiant (§5.2), at its printed cost. A full hand burns its card (§2.4). The
//! Radiant face does the same and its caster's card costs (1) too. Through v0.6.7k (issue #4285) both
//! "cost (0)" (`script_v0_6_7k`, kept, R767000).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-149";

/// "It costs (1)": the opponent's Call to Chaos (`costOverride`, a price, R1748003), on both faces.
const OPPONENT_COST: i32 = 1;
/// R767000: "It costs (0)", both cards' price through v0.6.7k (issue #4285).
const COST_V0_6_7K: i32 = 0;

/// The Call to Chaos pool (`query::call_to_chaos_editions`, R3869001), with the riders a pick takes.
fn add_call_to_chaos(riders: Value) -> Effect {
    let mut args = json!({ "query": crate::query::call_to_chaos_editions() });
    if let (Value::Object(into), Value::Object(more)) = (&mut args, riders) {
        into.extend(more);
    }
    add_random_from_catalog(json_as(args))
}

/// "It costs (1)": the caster's Radiant Call to Chaos on the Radiant face (`costOverride`, a price,
/// R1748003).
const OWN_RADIANT_COST: i32 = 1;

/// Then "add a random Radiant Call to Chaos to your hand", at `cost` when it names one.
fn add_own(cost: Option<i32>) -> Effect {
    let mut riders = json!({ "radiant": true });
    if let Some(cost) = cost {
        riders["costOverride"] = json!(cost);
    }
    add_call_to_chaos(riders)
}

/// "Add a random Call to Chaos to your opponent's hand. It costs (`cost`)."
fn add_theirs(cost: i32) -> Effect {
    add_call_to_chaos(json!({ "player": "enemy", "costOverride": cost }))
}

fn base_face(theirs_cost: i32) -> Script {
    Script {
        cry: Some(hook(move |_ctx| vec![add_theirs(theirs_cost), add_own(None)])),
        ..Script::default()
    }
}

/// Both faces, the opponent's card at `theirs_cost` and the Radiant face's own at `own_cost`.
fn wildar(theirs_cost: i32, own_cost: i32) -> CardScripts {
    let radiant = Script {
        cry: Some(hook(move |_ctx| vec![add_theirs(theirs_cost), add_own(Some(own_cost))])),
        ..Script::default()
    };
    CardScripts {
        base: base_face(theirs_cost),
        radiant,
    }
}

pub fn script() -> CardScripts {
    wildar(OPPONENT_COST, OWN_RADIANT_COST)
}

/// R767000: after v0.5.9f through v0.6.7k both cards cost (0) (issue #4285); `KEPT_SCRIPTS` folds those
/// replays under it.
pub fn script_v0_6_7k() -> CardScripts {
    wildar(COST_V0_6_7K, COST_V0_6_7K)
}

/// R767000: through v0.5.9f the Radiant face added only its caster's Radiant Call to Chaos, at its
/// printed cost; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_9f() -> CardScripts {
    let radiant = Script {
        cry: Some(hook(|_ctx| vec![add_own(None)])),
        ..Script::default()
    };
    CardScripts {
        base: base_face(COST_V0_6_7K),
        radiant,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, js, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// The Call to Chaos editions of the sets that ship, in catalog order: the set's own two joined
    /// them as it shipped (#1884, R1420). Re-pinned for the Training Grounds release (#2204): its #39
    /// joins them (R2200004); and for Stinkyscape+'s (#3573): its #142.
    const SHIPPED_EDITIONS: [&str; 7] = [
        "core-095",
        "classicplus-073",
        "meditative-095",
        "stinkyscape-150",
        "stinkyscape-151",
        "traininggrounds-039",
        "stinkyscapeplus-142",
    ];
    const ANCHOR: &str = "core-008";
    const SWEEP: u32 = 60;

    fn wildar_at(cursor: u32, radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": "wildar",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, ANCHOR], "library": filler(4), "mana": 10 },
            "p2": { "hand": [ANCHOR], "library": filler(4) },
        }));
        s.state_mut().rng_cursor = cursor;
        s.play(ID, json!({}));
        s
    }

    fn gained(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        s.hand(player).into_iter().filter(|card| card.def_id != ANCHOR).collect()
    }

    fn cost_of(s: &Scenario, card: &CardInstance) -> i32 {
        jackioh_engine::mana::effective_cost(s.state(), card, Default::default())
    }

    #[test]
    fn r1882000_the_opponent_gains_a_call_to_chaos_at_1_and_its_caster_a_radiant_one() {
        let mut s = wildar_at(0, false);
        let theirs = gained(&s, P2);
        assert_eq!(theirs.len(), 1);
        assert!(SHIPPED_EDITIONS.contains(&theirs[0].def_id.as_str()));
        assert_eq!(theirs[0].owner, P2);
        assert!(!theirs[0].radiant);
        assert_eq!(cost_of(&s, &theirs[0]), 1, "It costs (1)");
        let mine = gained(&s, P1);
        assert_eq!(mine.len(), 1);
        assert!(SHIPPED_EDITIONS.contains(&mine[0].def_id.as_str()));
        assert!(mine[0].radiant);
        assert_eq!(mine[0].cost_override, None);
        assert_eq!(cost_of(&s, &mine[0]), 4);
        s.expect_in_zone(ID, "graveyard");
    }

    #[test]
    fn r1882000_the_opponent_s_edition_is_hidden_from_its_caster() {
        let s = wildar_at(0, false);
        let theirs = gained(&s, P2);
        let view = s.view(P1);
        assert_eq!(js(&view.opponent.hand), json!({ "count": 2 }));
        let seen = serde_json::to_string(&view.events).unwrap_or_default();
        assert!(!seen.contains(&theirs[0].id), "the caster reads the opponent's card");
    }

    #[test]
    fn r1882000_the_two_picks_are_independent_and_each_shipped_edition_appears() {
        let mut theirs: Vec<String> = Vec::new();
        let mut mine: Vec<String> = Vec::new();
        let mut differ = false;
        for cursor in 0..SWEEP {
            let s = wildar_at(cursor, false);
            let a = gained(&s, P2)[0].def_id.clone();
            let b = gained(&s, P1)[0].def_id.clone();
            differ |= a != b;
            theirs.push(a);
            mine.push(b);
        }
        for edition in SHIPPED_EDITIONS {
            assert!(theirs.iter().any(|id| id == edition), "{edition} never reached the opponent");
            assert!(mine.iter().any(|id| id == edition), "{edition} never reached its caster");
        }
        assert!(differ, "the two picks never differed");
    }

    #[test]
    fn a_full_hand_burns_its_caster_s_card() {
        let mut hand = vec![json!(ID)];
        hand.extend((0..HAND_CAP).map(|_| json!(ANCHOR)));
        let mut s = scenario(json!({
            "seed": "wildar",
            "p1": { "hand": hand, "library": filler(4), "mana": 10 },
            "p2": { "hand": [ANCHOR], "library": filler(4) },
        }));
        s.play(ID, json!({}));
        let burned: Vec<Value> = s
            .events()
            .iter()
            .filter(|event| event.event_type().as_str() == "burned")
            .map(js)
            .collect();
        assert_eq!(burned.len(), 1);
        assert_eq!(burned[0]["owner"], json!("p1"));
        assert_eq!(s.hand(P1).len() as i32, HAND_CAP);
        assert_eq!(gained(&s, P2).len(), 1);
    }

    #[test]
    fn radiant_gives_the_opponent_one_at_1_and_its_caster_a_radiant_one_at_1() {
        let s = wildar_at(0, true);
        let theirs = gained(&s, P2);
        assert_eq!(theirs.len(), 1);
        assert!(SHIPPED_EDITIONS.contains(&theirs[0].def_id.as_str()));
        assert!(!theirs[0].radiant);
        assert_eq!(cost_of(&s, &theirs[0]), 1, "It costs (1)");
        let mine = gained(&s, P1);
        assert_eq!(mine.len(), 1);
        assert!(SHIPPED_EDITIONS.contains(&mine[0].def_id.as_str()));
        assert!(mine[0].radiant);
        assert_eq!(cost_of(&s, &mine[0]), 1, "It costs (1)");
    }

    /// R767000: through v0.6.7k (issue #4285) both faces' cards cost (0).
    #[test]
    fn r767000_the_script_kept_for_v0_6_7k_prices_both_cards_0() {
        crate::under_kept(ID, script_v0_6_7k(), || {
            for radiant in [false, true] {
                let s = wildar_at(0, radiant);
                assert_eq!(cost_of(&s, &gained(&s, P2)[0]), COST_V0_6_7K, "radiant {radiant}");
                let mine = gained(&s, P1);
                assert_eq!(cost_of(&s, &mine[0]), if radiant { COST_V0_6_7K } else { 4 });
            }
        });
    }

    #[test]
    fn r767000_the_script_kept_for_v0_5_9f_adds_only_its_caster_s_radiant_call_to_chaos() {
        crate::under_kept(ID, script_v0_5_9f(), || {
            let s = wildar_at(0, true);
            assert!(gained(&s, P2).is_empty());
            let mine = gained(&s, P1);
            assert_eq!(mine.len(), 1);
            assert!(mine[0].radiant);
            assert_eq!(cost_of(&s, &mine[0]), 4);
        });
    }
}
