//! SPEC §8.12 S+ #58 Phase Conveyer (§2.4, §5.1, §6.1 Flexible; R62, R1420, R1840002, R3535000, R3543000,
//! R3549002): (1) Unit, Mindustrial, Epic, 0/3 → 0/6, a card of the held Stinkyscape+ set.
//!   Base:    "Flexible
//!            Start of turn: Add a Mineral and Resource to your hand."
//!   Radiant: "Flexible
//!            Start of turn: Add a Radiant Mineral and a Radiant Resource to your hand."
//!
//! Flexible is catalog data, a keyword the engine reads (R3543000). At the start of its controller's turn
//! (R62), before their draw, it adds two cards in the text's order (R3549002): a random Mineral, a card of
//! the Stinkyscape+ cards tagged Mineral (#44 to #50), tokens out (§5.1), the pool naming its set so that
//! it reaches them while the set is held (R1420), at its printed price (the text prints no (0) as the
//! drills' do); then a Resource, `gainResources` (R1840002). No prompt opens: both are drawn from the
//! match rng. The Radiant face adds both Radiant. A full hand burns what does not fit (§2.4).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-058";

/// R3549002: "a Mineral": the held set's, tagged Mineral.
fn minerals() -> CatalogQueryArgs {
    json_as(json!({ "set": SetName::StinkyscapePlus, "tags": [Tag::Mineral] }))
}

fn conveyer(radiant: bool) -> Script {
    Script {
        start_of_turn: Some(hook(move |_ctx| {
            vec![
                add_random_from_catalog(json_as(json!({ "query": minerals(), "radiant": radiant }))),
                gain_resources(json_as(json!({ "count": 1, "radiant": radiant }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: conveyer(false),
        radiant: conveyer(true),
    }
}

// S+ #58 Phase Conveyer — SPEC §8.12 row S+ 58 (R3549002; R62, R1840002, R3543000, §2.4).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, weapon_board};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";

    fn conveying(radiant: bool, seed: &str, hand: usize) -> Scenario {
        weapon_board(
            seed,
            json!({
                "hand": vec![FILLER; hand],
                "library": filler(6),
                "field": [{ "def": ID, "radiant": radiant }],
            }),
            json!({ "hand": [FILLER], "library": filler(6) }),
        )
    }

    /// The Minerals and Resources p1 gained in the last step, in order (the turn's draw is neither).
    fn gained(s: &Scenario) -> Vec<CardInstance> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand { player: P1, instance_id, .. } => Some(s.card(instance_id).clone()),
                _ => None,
            })
            .filter(|card| is_mineral(card) || is_resource(s, card))
            .collect()
    }

    fn is_mineral(card: &CardInstance) -> bool {
        crate::card_def(&card.def_id).tags.contains(&Tag::Mineral)
    }

    fn is_resource(s: &Scenario, card: &CardInstance) -> bool {
        crate::resources_in_hand(s, P1).iter().any(|held| held.id == card.id)
    }

    /// p1's next turn start, from their main phase.
    fn next_turn(s: &mut Scenario) {
        s.end_turn().end_turn();
    }

    #[test]
    fn it_is_a_flexible_0_3_that_becomes_0_6() {
        let s = conveying(false, "s58-stats", 1);
        let def = crate::card_def(ID);
        assert!(def.base.keywords.contains(&Keyword::Flexible) && def.radiant.keywords.contains(&Keyword::Flexible));
        let unit = s.unit_at(P1, 1).id;
        let stats = s.stats(unit.as_str());
        assert_eq!((stats.attack, stats.health), (0, 3));
        let r = conveying(true, "s58-rstats", 1);
        let unit = r.unit_at(P1, 1).id;
        let stats = r.stats(unit.as_str());
        assert_eq!((stats.attack, stats.health), (0, 6));
    }

    mod base {
        use super::*;

        #[test]
        fn r3549002_start_of_turn_adds_one_mineral_and_one_resource() {
            let mut kinds: IndexSet<String> = IndexSet::new();
            for i in 0..60 {
                let mut s = conveying(false, &format!("s58-{i}"), 1);
                next_turn(&mut s);
                assert!(s.state().pending.is_none(), "no prompt opens");
                let cards = gained(&s);
                assert_eq!(cards.len(), 2, "a Mineral and a Resource");
                let [mineral, resource] = [&cards[0], &cards[1]];
                assert!(is_mineral(mineral), "the Mineral first: {}", mineral.def_id);
                assert!(is_resource(&s, resource), "then the Resource: {}", resource.def_id);
                for card in &cards {
                    assert_eq!(card.zone.z(), ZoneName::Hand);
                    assert!(!card.radiant);
                    assert_eq!(card.cost_mod, 0);
                    assert!(card.cost_override.is_none(), "the printed price");
                }
                kinds.insert(mineral.def_id.clone());
                // Before the turn's draw.
                let events = s.last_events();
                let added = events.iter().position(|e| matches!(e, GameEvent::AddedToHand { player: P1, .. }));
                let drawn = events.iter().rposition(|e| matches!(e, GameEvent::Drawn { .. }));
                assert!(added.expect("the adds") < drawn.expect("the turn's draw"));
            }
            assert!(kinds.len() > 3, "the Minerals vary: {kinds:?}");
        }

        #[test]
        fn r3549002_the_mineral_is_the_held_sets_and_costs_its_printed_price() {
            let mut s = conveying(false, "s58-price", 1);
            next_turn(&mut s);
            let mineral = gained(&s).remove(0);
            let def = crate::card_def(&mineral.def_id);
            assert_eq!(def.set, SetName::StinkyscapePlus);
            assert_eq!(effective_cost(s.state(), &mineral, Default::default()), 1);
        }

        #[test]
        fn nothing_at_the_opponents_start_of_turn() {
            let mut s = conveying(false, "s58-opp", 1);
            s.end_turn();
            assert_eq!(s.state().active, PlayerId::P2);
            assert!(gained(&s).is_empty());
        }

        #[test]
        fn s2_4_a_full_hand_burns_what_does_not_fit() {
            let mut s = conveying(false, "s58-full", HAND_CAP as usize - 1);
            next_turn(&mut s);
            let burned = s.last_events().iter().filter(|e| matches!(e, GameEvent::Burned { .. })).count();
            assert!(burned >= 1, "two gains and the draw cannot all fit");
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3549002_it_adds_a_radiant_mineral_and_a_radiant_resource() {
            for i in 0..30 {
                let mut s = conveying(true, &format!("s58-r{i}"), 1);
                next_turn(&mut s);
                assert!(s.state().pending.is_none(), "no prompt opens");
                let cards = gained(&s);
                assert_eq!(cards.len(), 2);
                assert!(is_mineral(&cards[0]) && is_resource(&s, &cards[1]));
                for card in &cards {
                    assert!(card.radiant, "{}", card.def_id);
                    assert_eq!(card.cost_mod, 0);
                    assert!(card.cost_override.is_none());
                }
            }
        }
    }
}
