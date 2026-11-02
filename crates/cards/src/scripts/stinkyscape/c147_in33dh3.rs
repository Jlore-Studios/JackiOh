//! S #147 IN33DH3 (SPEC §8.10 row 147): (0) Unit, Catalyst, Human, Legendary, 1/2 → 2/4, a card of
//! the Stinkyscape set.
//!   Base:    "Cry: For the rest of your turn, an AI helps you make the Pareto Optimal decisions.
//!             Death: Shuffle an IN33DH3 Prime into your deck."
//!   Radiant: "Cry: For the rest of your turn, an AI helps you make the Pareto Optimal decisions.
//!             Death: Shuffle a Radiant IN33DH3 Prime into your deck."
//! Engine: Cry (R1): the Pareto hint (R1844001), on its caster's own turn only, until that turn's
//!   cleanup: their own view marks each hand card the judge rates best to play now, ties sharing it,
//!   plays only. Death: a fresh S #147.1 IN33DH3 Prime shuffled into its owner's deck, C+ #38's shape
//!   (R1421). Radiant: a Radiant Prime.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-147";

/// S #147.1 IN33DH3 Prime, the Catalyst's Prime.
const PRIME: &str = "stinkyscape-147-1";

fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(|_ctx| vec![pareto_hint()])),
        death: Some(hook(move |_ctx| {
            vec![shuffle_into(json_as(json!({ "defId": PRIME, "count": 1, "radiant": radiant })))]
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

// S #147 IN33DH3 — SPEC §8.10 row 147, BUILD M12 row S 147 (R1844001).
#[cfg(test)]
mod tests {
    use jackioh_engine::effects::summon;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const IN33DH3: &str = "stinkyscape-147";
    const PRIME: &str = "stinkyscape-147-1";
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const MENACE: &str = "core-019"; // 9/9 Taunt.
    const FILLER: &str = "core-005";

    fn marked(s: &Scenario) -> Vec<String> {
        let HandView::Cards(hand) = s.view(P1).you.hand else {
            panic!("p1's own hand is full cards");
        };
        hand.into_iter()
            .filter(|card| card.pareto_optimal == Some(true))
            .map(|card| card.instance_id)
            .collect()
    }

    fn hinting() -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [IN33DH3, VANILLA, VANILLA], "library": filler(6) },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }));
        assert!(marked(&s).is_empty(), "no hint before the Cry");
        s.play(IN33DH3, json!({ "zone": 1 }));
        s
    }

    fn by_hand(s: &mut Scenario, effects: Vec<Effect>) {
        let mut rng = create_rng(&s.state().seed, s.state().rng_cursor);
        let mut events: Vec<GameEvent> = Vec::new();
        {
            let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
            {
                let mut ctx = make_context(
                    &mut sink,
                    None,
                    HookOptions {
                        controller: Some(P1),
                        ..HookOptions::default()
                    },
                );
                apply_effects(&effects, &mut ctx);
            }
            settle(&mut sink, SettleOptions::default());
        }
        s.state_mut().rng_cursor = rng.cursor();
    }

    fn dies(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "hand": [FILLER],
                "field": [{ "def": IN33DH3, "radiant": radiant }],
                "library": filler(12),
            },
            "p2": { "hand": [FILLER], "field": [MENACE], "library": filler(6) },
        }));
        s.attack(IN33DH3, MENACE);
        s.expect_in_zone(IN33DH3, "graveyard");
        s
    }

    fn primes(s: &Scenario) -> Vec<bool> {
        s.pile(P1, "library")
            .iter()
            .filter(|card| card.def_id == PRIME)
            .map(|card| card.radiant)
            .collect()
    }

    #[test]
    fn r1844001_its_cry_marks_the_cards_the_judge_rates_best_ties_sharing_it() {
        let s = hinting();
        assert_eq!(s.state().players[P1].pareto_hint, Some(s.state().turn));
        let vanillas: Vec<String> = s.hand(P1).into_iter().map(|card| card.id).collect();
        assert_eq!(vanillas.len(), 2);
        assert_eq!(marked(&s), vanillas, "two equal plays share the mark");
        let theirs = serde_json::to_string(&s.view(P2)).expect("a view serializes");
        assert!(!theirs.contains("paretoOptimal"), "never on the opponent's view");
    }

    #[test]
    fn r1844001_the_hint_is_gone_at_the_cleanup() {
        let mut s = hinting();
        assert!(!marked(&s).is_empty());
        s.end_turn();
        assert_eq!(s.state().players[P1].pareto_hint, None);
        s.end_turn();
        assert_eq!(s.state().active, P1);
        assert!(marked(&s).is_empty(), "the rest of that turn only");
    }

    #[test]
    fn r1_summoned_not_played_it_hints_nothing() {
        let mut s = scenario(json!({
            "p1": { "hand": [VANILLA, VANILLA], "library": filler(6) },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }));
        by_hand(&mut s, vec![summon(json_as(json!({ "defId": IN33DH3 })))]);
        s.expect_in_zone(IN33DH3, "field");
        assert_eq!(s.state().players[P1].pareto_hint, None);
        assert!(marked(&s).is_empty());
        s.play(VANILLA, json!({ "zone": 2 }));
        assert!(marked(&s).is_empty(), "standing on the field it has no aura");
    }

    #[test]
    fn r1421_its_death_shuffles_an_in33dh3_prime_into_your_deck() {
        let s = dies(false);
        assert_eq!(primes(&s), vec![false]);
        assert_eq!(s.pile(P1, "library").len(), 13);
    }

    #[test]
    fn radiant_2_4_and_its_death_shuffles_a_radiant_prime() {
        let s = scenario(json!({
            "p1": { "hand": [FILLER], "field": [{ "def": IN33DH3, "radiant": true }], "library": filler(6) },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }));
        let stats = s.stats(IN33DH3);
        assert_eq!((stats.attack, stats.health), (2, 4));
        assert_eq!(primes(&dies(true)), vec![true]);
    }
}
