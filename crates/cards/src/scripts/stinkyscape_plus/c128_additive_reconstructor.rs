//! SPEC §8.12 S+ #128 Additive Reconstructor (§2.4, §5.1; R62, R65, R382, R1420, R3535000, R3563000,
//! R3565001): (1) Field Spell, Mindustrial, Rare, a card of the Stinkyscape+ set.
//!   Base:    "Start of turn: Add a random (0) or (1) cost Automa card to your hand."
//!   Radiant: "Start of turn: Add a random Radiant (0) or (1) cost Automa card to your hand."
//!
//! Engine: at the start of its controller's turn (R62), before their draw, one `addRandomFromCatalog` from
//! the Stinkyscape+ cards carrying any of the six Automa tags (R3565001), tokens out (§5.1, R382), the pool
//! naming its set so that it reaches them while the set is held (R1420), whose cost, read out of play
//! (R65), is 0 or 1: Dagger, Flare, Crawler, Mace and Horizon. No prompt opens: the card is drawn from the
//! match rng. The Radiant face adds it Radiant. A full hand burns it (§2.4).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-128";

/// "(0) or (1) cost": the cheapest and the dearest cost the pool takes.
const MIN_COST: i32 = 0;
const MAX_COST: i32 = 1;

/// R3565001: "Automa" is any of the six Automa tags.
const AUTOMA: [Tag; 6] = [
    Tag::AutomaAlpha,
    Tag::AutomaBeta,
    Tag::AutomaGamma,
    Tag::AutomaDelta,
    Tag::AutomaOmega,
    Tag::AutomaZeta,
];

/// R3565001: "a random (0) or (1) cost Automa card", the held set's.
fn automa_cards() -> CatalogQueryArgs {
    json_as(json!({
        "set": SetName::StinkyscapePlus,
        "anyTags": AUTOMA,
        "costRange": { "min": MIN_COST, "max": MAX_COST },
    }))
}

fn reconstructor(radiant: bool) -> Script {
    Script {
        start_of_turn: Some(hook(move |_ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": automa_cards(),
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: reconstructor(false),
        radiant: reconstructor(true),
    }
}

// S+ #128 Additive Reconstructor — SPEC §8.12 row S+ 128 (R3565001; R62, R65, R1420, §2.4).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, weapon_board};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";
    /// Dagger, Mace, Flare, Horizon and Crawler: the Automa cards that cost (0) or (1).
    const POOL: [&str; 5] = [
        "stinkyscapeplus-110",
        "stinkyscapeplus-111",
        "stinkyscapeplus-115",
        "stinkyscapeplus-116",
        "stinkyscapeplus-120",
    ];

    /// The Field Spell already standing in p1's backrow, one card in p1's hand.
    fn reconstructing(radiant: bool, seed: &str) -> Scenario {
        weapon_board(
            seed,
            json!({
                "hand": [FILLER],
                "library": filler(6),
                "backrow": [{ "def": ID, "radiant": radiant }],
            }),
            json!({ "hand": [FILLER], "library": filler(6) }),
        )
    }

    /// The Stinkyscape+ cards p1 gained in the last step (the turn's draw reaches the hand too, as a Core
    /// filler).
    fn gained(s: &Scenario) -> Vec<CardInstance> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand { player: P1, instance_id, .. } => Some(s.card(instance_id).clone()),
                _ => None,
            })
            .filter(|card| crate::card_def(&card.def_id).set == SetName::StinkyscapePlus)
            .collect()
    }

    /// p1's next turn start, from their main phase.
    fn next_turn(s: &mut Scenario) {
        s.end_turn().end_turn();
    }

    fn check_automa(card: &CardInstance, radiant: bool) {
        let def = crate::card_def(&card.def_id);
        assert_eq!(def.set, SetName::StinkyscapePlus, "{}", card.def_id);
        assert!(def.tags.iter().any(|tag| AUTOMA.contains(tag)), "{}", card.def_id);
        assert!(matches!(def.cost, CardCost::Fixed(0 | 1)), "{}", card.def_id);
        assert!(!def.is_token(), "{}", card.def_id);
        assert_eq!(card.zone.z(), ZoneName::Hand);
        assert_eq!(card.radiant, radiant, "{}", card.def_id);
    }

    #[test]
    fn r3565001_the_pool_is_dagger_mace_flare_horizon_crawler() {
        // The scenario registers the catalog, which the pool reads.
        let _registered = reconstructing(false, "s128-pool");
        let mut found: Vec<String> =
            crate::query::pool(ID, &automa_cards()).iter().map(|def| def.id.clone()).collect();
        found.sort();
        let mut expected: Vec<String> = POOL.iter().map(|id| id.to_string()).collect();
        expected.sort();
        assert_eq!(found, expected);
    }

    #[test]
    fn nothing_at_the_opponents_start_of_turn() {
        for radiant in [false, true] {
            let mut s = reconstructing(radiant, "s128-opponent");
            s.end_turn();
            assert!(gained(&s).is_empty(), "radiant {radiant}");
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r3565001_start_of_turn_adds_one_automa_card_costing_0_or_1() {
            let mut kinds: IndexSet<String> = IndexSet::new();
            for i in 0..40 {
                let mut s = reconstructing(false, &format!("s128-{i}"));
                next_turn(&mut s);
                assert!(s.state().pending.is_none(), "no prompt opens");
                let cards = gained(&s);
                assert_eq!(cards.len(), 1, "one card");
                check_automa(&cards[0], false);
                assert!(POOL.contains(&cards[0].def_id.as_str()), "{}", cards[0].def_id);
                kinds.insert(cards[0].def_id.clone());
                // Before the turn's draw (R62).
                let events = s.last_events();
                let added = events.iter().position(|e| matches!(e, GameEvent::AddedToHand { player: P1, .. }));
                let drawn = events.iter().rposition(|e| matches!(e, GameEvent::Drawn { .. }));
                assert!(added.expect("the add") < drawn.expect("the turn's draw"));
            }
            assert!(kinds.len() >= 3, "the cards vary: {kinds:?}");
        }

        #[test]
        fn s2_4_a_full_hand_burns_it() {
            let mut s = weapon_board(
                "s128-full",
                json!({
                    "hand": vec![FILLER; HAND_CAP as usize],
                    "library": filler(6),
                    "backrow": [ID],
                }),
                json!({ "hand": [FILLER], "library": filler(6) }),
            );
            next_turn(&mut s);
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            assert!(gained(&s).is_empty());
            assert!(s.last_events().iter().any(
                |event| matches!(event, GameEvent::Burned { def_id, .. } if POOL.contains(&def_id.as_str()))
            ));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3565001_radiant_adds_a_radiant_one() {
            for i in 0..20 {
                let mut s = reconstructing(true, &format!("s128-r{i}"));
                next_turn(&mut s);
                let cards = gained(&s);
                assert_eq!(cards.len(), 1, "one card");
                check_automa(&cards[0], true);
                assert!(POOL.contains(&cards[0].def_id.as_str()), "{}", cards[0].def_id);
            }
        }
    }
}
