//! SPEC §8.10 Stinkyscape 149 Wildar (R60, R97, R380, R1420, R1882000). (3) Spell, Legendary.
//!   Base:    "Add a random Call to Chaos to your opponent's hand. It costs (0). Then add a random Radiant
//!            Call to Chaos to your hand."
//!   Radiant: "Add a random Radiant Call to Chaos to your hand."
//!
//! Two independent picks (R60, R1882000) from the Call to Chaos pool, the cards tagged Call to Chaos of
//! the sets that ship (R380; the set's own two join it as it ships, R1420): the first a fresh card of the
//! opponent's, owned by them, at `costOverride` 0, its edition hidden from its caster (R97); then the
//! second its caster's, Radiant (§5.2), at its printed cost. A full hand burns its card (§2.4). The
//! Radiant face drops the opponent's.

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-149";

/// "It costs (0)": the opponent's Call to Chaos (`costOverride`, a price, R1748003).
const OPPONENT_COST: i32 = 0;

/// The Call to Chaos pool (`Tag::CallToChaos`), with the riders a pick takes.
fn add_call_to_chaos(riders: Value) -> Effect {
    let mut args = json!({ "query": { "tags": [Tag::CallToChaos] } });
    if let (Value::Object(into), Value::Object(more)) = (&mut args, riders) {
        into.extend(more);
    }
    add_random_from_catalog(json_as(args))
}

/// Then "add a random Radiant Call to Chaos to your hand".
fn add_own() -> Effect {
    add_call_to_chaos(json!({ "radiant": true }))
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|_ctx| {
            vec![
                add_call_to_chaos(json!({ "player": "enemy", "costOverride": OPPONENT_COST })),
                add_own(),
            ]
        })),
        ..Script::default()
    };

    let radiant = Script {
        cry: Some(hook(|_ctx| vec![add_own()])),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, js, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// The Call to Chaos editions of the sets that ship, in catalog order: the set's own two joined
    /// them as it shipped (#1884, R1420).
    const SHIPPED_EDITIONS: [&str; 5] =
        ["core-095", "classicplus-073", "meditative-095", "stinkyscape-150", "stinkyscape-151"];
    /// A (1) Unit that keeps p1's turn from ending by itself.
    const ANCHOR: &str = "core-008";
    /// How many cursors the sweeps walk.
    const SWEEP: u32 = 60;

    /// p1 plays Wildar on `radiant`'s face with the rng at `cursor`; p2 holds one card.
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

    /// The cards of `player`'s hand that are no `ANCHOR`.
    fn gained(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        s.hand(player).into_iter().filter(|card| card.def_id != ANCHOR).collect()
    }

    fn cost_of(s: &Scenario, card: &CardInstance) -> i32 {
        jackioh_engine::mana::effective_cost(s.state(), card, Default::default())
    }

    #[test]
    fn r1882000_the_opponent_gains_a_free_call_to_chaos_and_its_caster_a_radiant_one() {
        let mut s = wildar_at(0, false);
        let theirs = gained(&s, P2);
        assert_eq!(theirs.len(), 1);
        assert!(SHIPPED_EDITIONS.contains(&theirs[0].def_id.as_str()));
        assert_eq!(theirs[0].owner, P2);
        assert!(!theirs[0].radiant);
        assert_eq!(cost_of(&s, &theirs[0]), OPPONENT_COST);
        let mine = gained(&s, P1);
        assert_eq!(mine.len(), 1);
        assert!(SHIPPED_EDITIONS.contains(&mine[0].def_id.as_str()));
        assert!(mine[0].radiant);
        // Its printed (4), no override.
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
        // The hand at its cap once Wildar has left it.
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
    fn radiant_adds_only_its_caster_s_radiant_call_to_chaos() {
        let s = wildar_at(0, true);
        assert!(gained(&s, P2).is_empty());
        let mine = gained(&s, P1);
        assert_eq!(mine.len(), 1);
        assert!(SHIPPED_EDITIONS.contains(&mine[0].def_id.as_str()));
        assert!(mine[0].radiant);
        assert_eq!(cost_of(&s, &mine[0]), 4);
    }
}
