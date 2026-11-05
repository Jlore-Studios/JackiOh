//! S #45 Drone Commander (SPEC §8.10 row 45; §2.4, R1, R60, R380, R3142002): (4) Unit, Human, Epic,
//! 8/12 → 16/24, a card of the Stinkyscape set.
//!   Base:    "Cry: Fill your hand with random Drone Core cards."
//!   Radiant: "Cry: Fill your hand with random Radiant Drone Core cards."
//! Engine: Cry (R1; the list prints no trigger, the plan's fix ST8). It counts the room in its
//!   controller's hand as the Cry resolves (`room_in`: the Commander is on the field by then, not in the
//!   hand) and adds that many cards picked from S #41's Drone Core pool (the non-token cards tagged Drone
//!   Core, R380), one draw of the match rng per card (R60), each at its printed price: nothing is
//!   discounted. A full hand adds nothing and draws nothing from the rng, as Meditative #77 Bulk Booster
//!   and Stinkyscape #96 Ocean Tsunami fill one. Declares no numbers: the Cry's six cards and one mana
//!   off were `cards` and `discount` through v0.5.11o (`script_v0_5_11o`, kept, R767000). The pool names
//!   no set, so it reaches the set's Drone Cores because the set ships (R1420). Radiant: added Radiant.

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-045";

fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let room = room_in(ctx.state, ctx.controller, OffFieldZone::Hand);
            if room == 0 {
                return vec![];
            }
            vec![add_random_from_catalog(json_as(json!({
                "query": { "tags": ["Drone Core"] },
                "count": room,
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

/// One face as it was through v0.5.11o: `cards` of them, each `discount` less.
fn face_v0_5_11o(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "tags": ["Drone Core"] },
                "count": param(&*ctx, "cards"),
                "radiant": radiant,
                "costMod": -param(&*ctx, "discount"),
            })))]
        })),
        ..Script::default()
    }
}

/// R767000: through v0.5.11o the Cry added `cards` (6) random Drone Core cards, each costing `discount`
/// (1) less; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_11o() -> CardScripts {
    CardScripts {
        base: face_v0_5_11o(false),
        radiant: face_v0_5_11o(true),
    }
}

// S #45 Drone Commander — SPEC §8.10 row 45, BUILD M12 row S 45 (R129, R767000, R1857000).
#[cfg(test)]
mod tests {
    use jackioh_engine::effects::summon;
    use jackioh_engine::testkit::*;

    use super::script_v0_5_11o;
    use crate::{filler, scenario, under_kept_as};

    const P1: PlayerId = PlayerId::P1;

    const COMMANDER: &str = "stinkyscape-045";
    const FILLER: &str = "core-005";

    /// The Drone Core cards in p1's hand (S #27 to #36): whether Radiant, and the price changes they carry.
    fn cores(s: &Scenario) -> Vec<(bool, i32, Option<i32>)> {
        s.hand(P1)
            .into_iter()
            .filter(|card| {
                (27..=36)
                    .map(|n| format!("stinkyscape-{n:03}"))
                    .any(|tier| tier == card.def_id)
            })
            .map(|card| (card.radiant, card.cost_mod, card.cost_override))
            .collect()
    }

    fn setup(radiant: bool, hand: usize) -> Scenario {
        let mut cards = vec![json!({ "def": COMMANDER, "radiant": radiant })];
        cards.extend((0..hand).map(|_| json!(FILLER)));
        scenario(json!({
            "seed": "drone-commander",
            "p1": { "hand": cards, "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    fn burned(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Burned { .. }))
            .count()
    }

    #[test]
    fn it_fills_the_hand_with_drone_core_cards_at_full_price() {
        for held in [0, 1, 4] {
            let mut s = setup(false, held);
            s.play(COMMANDER, json!({}));
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize, "{held} kept");
            assert_eq!(cores(&s), vec![(false, 0, None); HAND_CAP as usize - held], "{held} kept, none discounted");
            assert_eq!(burned(&s), 0);
        }
    }

    #[test]
    fn a_full_hand_adds_none_and_takes_no_random_number() {
        // Ten cards behind it: the hand is full as the Cry resolves (R129).
        let mut s = setup(false, HAND_CAP as usize);
        let cursor = s.state().rng_cursor;
        s.play(COMMANDER, json!({}));
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
        assert!(cores(&s).is_empty());
        assert_eq!(burned(&s), 0);
        assert_eq!(s.state().rng_cursor, cursor, "no random number");
    }

    #[test]
    fn it_declares_no_numbers() {
        assert!(crate::card_def(COMMANDER).params.as_ref().is_none_or(|params| params.is_empty()));
    }

    /// Held, its pool of the set's own Drone Cores was empty; since the release (#1884) the set ships,
    /// so they arrive with no preview.
    #[test]
    fn r1857000_since_the_release_the_drone_cores_arrive_without_a_preview() {
        let mut s = setup(false, 1);
        s.play(COMMANDER, json!({}));
        assert!(!cores(&s).is_empty());
    }

    #[test]
    fn radiant_16_24_fills_the_hand_with_radiant_ones() {
        let mut s = setup(true, 1);
        s.play(COMMANDER, json!({}));
        let commander = s.unit(P1, 1).expect("the Commander");
        let stats = s.stats(commander.id.as_str());
        assert_eq!((stats.attack, stats.health), (16, 24));
        assert_eq!(cores(&s), vec![(true, 0, None); HAND_CAP as usize - 1]);
    }

    #[test]
    fn summoned_it_has_no_cry() {
        let mut s = setup(false, 1);
        crate::run_as(&mut s, summon(json_as(json!({ "defId": COMMANDER }))), P1);
        assert!(cores(&s).is_empty());
    }

    /// The definition through v0.5.11o: `cards` 6 and `discount` 1 on both faces.
    fn old_definition(def: &mut CardDef) {
        def.params = Some(vec![
            json_as(json!({ "key": "cards", "base": 6, "radiant": 6, "better": "up", "step": 2, "min": 1, "cry": true })),
            json_as(json!({ "key": "discount", "base": 1, "radiant": 1, "better": "up", "step": 1, "min": 1, "cry": true })),
        ]);
    }

    /// R767000: through v0.5.11o the Cry added 6 random Drone Core cards, each costing 1 less.
    #[test]
    fn r767000_the_kept_script_adds_six_each_costing_one_less() {
        under_kept_as(COMMANDER, script_v0_5_11o(), old_definition, || {
            let mut s = setup(false, 1);
            s.play(COMMANDER, json!({}));
            assert_eq!(cores(&s), vec![(false, -1, None); 6]);
            assert_eq!(burned(&s), 0);

            let mut s = setup(false, 6);
            s.play(COMMANDER, json!({}));
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            assert_eq!(cores(&s).len(), 4);
            assert_eq!(burned(&s), 2);

            let mut s = setup(true, 1);
            s.play(COMMANDER, json!({}));
            assert_eq!(cores(&s), vec![(true, -1, None); 6]);
        });
    }
}
