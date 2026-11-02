//! S #147.2 Wardrum Blob (SPEC §7 row 147.2, §8.10 row 147.2): (2) Field Spell, Token (printed Legendary),
//! a token of the Stinkyscape set.
//!   Base:    "Aura: An AI always helps you make the Pareto Optimal decisions.
//!             Start of turn: Do something helpful."
//!   Radiant: "Aura: An AI always helps you make the Pareto Optimal decisions.
//!             Start of turn: Do something REALLY helpful."
//! Engine: `hintsPlays`, the Pareto hint (R1844001) on each of its controller's turns while it stands,
//!   storing nothing. Start of turn (R62), its controller's: the helpful cast (R1844002), the perfect
//!   pool's top card for its caster cast free as a new card (R70), counted as a play, the same state
//!   casting the same card. Radiant: "REALLY helpful" is the pool ranked by its Radiant faces and the
//!   pick cast Radiant, one cast.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-147-2";

fn face(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            hints_plays: Some(true),
            ..StaticFlags::default()
        }),
        start_of_turn: Some(hook(move |_ctx| {
            vec![helpful_cast(HelpfulCastArgs {
                radiant: Some(radiant),
            })]
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

// S #147.2 Wardrum Blob — SPEC §7 row 147.2, BUILD M12 row S 147.2 (R1844001, R1844002).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BLOB: &str = "stinkyscape-147-2";
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FILLER: &str = "core-005";

    fn fielded(seed: &str, radiant: bool, active: &str, hand: Value) -> Scenario {
        scenario(json!({
            "seed": seed,
            "active": active,
            "p1": {
                "hand": hand,
                "backrow": [{ "def": BLOB, "radiant": radiant, "faceUp": true }],
                "library": filler(6),
            },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }))
    }

    fn marked(s: &Scenario) -> Vec<String> {
        let HandView::Cards(hand) = s.view(P1).you.hand else {
            panic!("p1's own hand is full cards");
        };
        hand.into_iter()
            .filter(|card| card.pareto_optimal == Some(true))
            .map(|card| card.instance_id)
            .collect()
    }

    fn plays(events: &[GameEvent]) -> Vec<(String, String, i32)> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::CardPlayed {
                    player: P1,
                    instance_id,
                    def_id,
                    cost_paid,
                    ..
                } => Some((instance_id.clone(), def_id.clone(), *cost_paid)),
                _ => None,
            })
            .collect()
    }

    fn helped(seed: &str, radiant: bool) -> (Scenario, Vec<(String, String, i32)>) {
        let mut s = fielded(seed, radiant, "p2", json!([VANILLA]));
        s.end_turn();
        assert_eq!(s.state().active, P1);
        let played = plays(s.last_events());
        (s, played)
    }

    #[test]
    fn r1844001_while_it_stands_your_view_marks_the_optimal_hand_cards_on_your_turns() {
        let mut s = fielded("blob-hint", false, "p1", json!([VANILLA, VANILLA]));
        let vanillas: Vec<String> = s.hand(P1).into_iter().map(|card| card.id).collect();
        assert_eq!(marked(&s), vanillas, "two equal plays share the mark");
        assert_eq!(s.state().players[P1].pareto_hint, None, "the aura stores nothing");
        let theirs = serde_json::to_string(&s.view(P2)).expect("a view serializes");
        assert!(!theirs.contains("paretoOptimal"), "never on the opponent's view");
        s.end_turn();
        assert!(
            subsystems::pareto::pareto_hint_ids(s.state(), P1).is_empty(),
            "on p1's own turns only"
        );
    }

    #[test]
    fn r1844002_as_your_turn_begins_it_casts_the_perfect_pools_top_card_free() {
        let (s, played) = helped("blob-help", false);
        let [(instance_id, def_id, cost_paid)] = played.as_slice() else {
            panic!("one helpful cast, got {played:?}");
        };
        assert_eq!(*cost_paid, 0, "free");
        let def = crate::card_def(def_id);
        assert!(SHIPPED_SETS.contains(&def.set), "from a set that ships: {def_id}");
        assert!(!def.token, "never a token: {def_id}");
        assert_ne!(def_id, BLOB);
        assert!(!s.card(instance_id.as_str()).radiant);
        assert!(cards_played_this_turn(s.state(), P1) >= 1, "counted as a play");
    }

    #[test]
    fn r1844002_not_at_the_opponents_start_of_turn() {
        let mut s = fielded("blob-theirs", false, "p1", json!([VANILLA]));
        s.end_turn();
        assert_eq!(s.state().active, P2);
        assert!(plays(s.last_events()).is_empty());
    }

    #[test]
    fn r1844002_the_same_state_casts_the_same_card() {
        let (_, first) = helped("blob-same", false);
        let (_, second) = helped("blob-same", false);
        assert_eq!(first, second);
    }

    #[test]
    fn radiant_the_pick_is_cast_radiant() {
        let (s, played) = helped("blob-radiant", true);
        let [(instance_id, _, cost_paid)] = played.as_slice() else {
            panic!("one helpful cast, got {played:?}");
        };
        assert_eq!(*cost_paid, 0);
        assert!(s.card(instance_id.as_str()).radiant, "REALLY helpful: the pick Radiant");
    }
}
