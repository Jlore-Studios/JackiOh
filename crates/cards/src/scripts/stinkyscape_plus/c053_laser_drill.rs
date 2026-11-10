//! SPEC §8.12 S+ #53 Laser Drill (§2.4, §5.1, §6.1 GIGA, R62, R1420, R1840002, R3535000, R3546000, R3549001):
//! (4) Field Spell, Mindustrial, Epic, a card of the held Stinkyscape+ set.
//!   Base:    "GIGA
//!            End of turn: Add three random Minerals (that cost 0) or Resources to your hand."
//!   Radiant: "GIGA
//!            End of turn: Add three random Radiant Minerals (that cost 0) or Radiant Resources to your hand."
//!
//! GIGA is catalog data, a keyword the engine reads (R3546000). At the end of its controller's turn (R62)
//! it makes `DRILL_FINDS` finds (R3549001), each its own flip (`MINERAL_CHANCE`) between a Mineral and a
//! Resource, as #54's: a Mineral is a random card of the Stinkyscape+ cards tagged Mineral (#44 to #50),
//! the pool naming its set so that it reaches them while the set is held (R1420), and it costs (0)
//! (`costOverride`, "that cost 0"); a Resource is `gainResources` (R1840002), which cost (0) already. The
//! Radiant face adds all Radiant. The list prints GIGA twice on the Radiant face; it is one keyword
//! (R3549001). A full hand burns what does not fit (§2.4).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-053";

/// "three random Minerals or Resources".
const DRILL_FINDS: i32 = 3;
/// R3549001: the chance that a find is a Mineral rather than a Resource.
const MINERAL_CHANCE: f64 = 0.5;
/// "Minerals (that cost 0)".
const MINERAL_COST: i32 = 0;

/// R3549001: "a random Mineral": the held set's, tagged Mineral.
fn minerals() -> CatalogQueryArgs {
    json_as(json!({ "set": SetName::StinkyscapePlus, "tags": [Tag::Mineral] }))
}

/// R3549001: one find, a Mineral at cost (0) or a Resource by the flip.
fn find(ctx: &mut EffectContext<'_>, radiant: bool) -> Effect {
    if ctx.rng.chance(MINERAL_CHANCE) {
        add_random_from_catalog(json_as(json!({
            "query": minerals(),
            "costOverride": MINERAL_COST,
            "radiant": radiant,
        })))
    } else {
        gain_resources(json_as(json!({ "count": 1, "radiant": radiant })))
    }
}

fn drill(radiant: bool) -> Script {
    Script {
        end_of_turn: Some(hook(move |ctx| (0..DRILL_FINDS).map(|_| find(ctx, radiant)).collect())),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: drill(false),
        radiant: drill(true),
    }
}

// S+ #53 Laser Drill — SPEC §8.12 row S+ 53 (R3549001; R62, R1840002, R3546000, §2.4).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, weapon_board};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";

    fn drilling(radiant: bool, seed: &str, hand: usize) -> Scenario {
        weapon_board(
            seed,
            json!({
                "hand": vec![FILLER; hand],
                "library": filler(6),
                "backrow": [{ "def": ID, "radiant": radiant }],
            }),
            json!({ "hand": [FILLER], "library": filler(6) }),
        )
    }

    /// The Minerals and Resources p1 gained in the last step, in order (a turn's draw is neither).
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

    /// Plays `games` first end of turns and returns what each gained.
    fn rounds(radiant: bool, games: usize) -> Vec<(Scenario, Vec<CardInstance>)> {
        (0..games)
            .map(|i| {
                let mut s = drilling(radiant, &format!("s53-{radiant}-{i}"), 1);
                s.end_turn();
                let cards = gained(&s);
                (s, cards)
            })
            .collect()
    }

    #[test]
    fn r3546000_it_prints_giga_once_on_each_face() {
        let def = crate::card_def(ID);
        for face in [&def.base, &def.radiant] {
            assert_eq!(face.keywords.iter().filter(|keyword| **keyword == Keyword::Giga).count(), 1);
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r3549001_end_of_turn_adds_three_minerals_or_resources() {
            let mut minerals = 0;
            let mut resources = 0;
            for (s, cards) in rounds(false, 40) {
                assert_eq!(cards.len(), DRILL_FINDS as usize);
                for card in &cards {
                    assert_eq!(card.zone.z(), ZoneName::Hand);
                    assert!(!card.radiant, "{}", card.def_id);
                    if is_mineral(card) {
                        assert_eq!(card.cost_override, Some(MINERAL_COST), "a Mineral that costs 0");
                        assert_eq!(effective_cost(s.state(), card, Default::default()), 0);
                        minerals += 1;
                    } else {
                        assert!(is_resource(&s, card), "{}", card.def_id);
                        resources += 1;
                    }
                }
            }
            assert!(minerals > 20 && resources > 20, "{minerals} Minerals, {resources} Resources");
        }

        #[test]
        fn r3549001_each_find_is_its_own_flip() {
            let kinds: IndexSet<Vec<bool>> = rounds(false, 60)
                .iter()
                .map(|(_, cards)| cards.iter().map(is_mineral).collect())
                .collect();
            assert!(kinds.len() > 4, "mixed hauls, not three of one kind: {kinds:?}");
        }

        #[test]
        fn r3549001_only_at_the_end_of_its_controllers_turn() {
            let mut s = drilling(false, "s53-turns", 1);
            s.end_turn();
            assert_eq!(gained(&s).len(), DRILL_FINDS as usize);
            s.end_turn();
            assert!(gained(&s).is_empty(), "the opponent's end of turn adds nothing");
        }

        #[test]
        fn s2_4_a_full_hand_burns_what_does_not_fit() {
            let mut s = drilling(false, "s53-full", HAND_CAP as usize - 1);
            s.end_turn();
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            let burned = s.last_events().iter().filter(|event| matches!(event, GameEvent::Burned { .. })).count();
            assert_eq!(burned, DRILL_FINDS as usize - 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3549001_all_three_are_radiant() {
            let mut minerals = 0;
            let mut resources = 0;
            for (s, cards) in rounds(true, 40) {
                assert_eq!(cards.len(), DRILL_FINDS as usize);
                for card in &cards {
                    assert!(card.radiant, "{}", card.def_id);
                    if is_mineral(card) {
                        assert_eq!(effective_cost(s.state(), card, Default::default()), 0);
                        minerals += 1;
                    } else {
                        assert!(is_resource(&s, card), "{}", card.def_id);
                        resources += 1;
                    }
                }
            }
            assert!(minerals > 20 && resources > 20, "{minerals} Minerals, {resources} Resources");
        }
    }
}
