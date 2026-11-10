//! SPEC §8.12 S+ #54 Pneumatic Drill (§2.4, §5.1, R62, R1420, R1840002, R3535000, R3549001): (2) Field Spell,
//! Mindustrial, Rare, a card of the held Stinkyscape+ set.
//!   Base:    "End of turn: Add a random Minerals or Resources to your hand."
//!   Radiant: "End of turn: Add a random Radiant Minerals or Radiant Resources to your hand."
//!
//! Engine: at the end of its controller's turn (R62) one find (R3549001): a fair flip (`MINERAL_CHANCE`)
//! between a Mineral and a Resource, one draw of the match rng. A Mineral is a random card of the
//! Stinkyscape+ cards tagged Mineral (#44 to #50), tokens out (§5.1), at its printed price, the pool naming
//! its set so that it reaches them while the set is held (R1420); a Resource is `gainResources` (R1840002),
//! rolled by its own rarity odds. The Radiant face adds both Radiant, the Mineral still at its printed
//! price (the text prints no discount). A full hand burns the find (§2.4). "A random Minerals or
//! Resources" prints no number besides "a" (R1748003).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-054";

/// R3549001: the chance that a find is a Mineral rather than a Resource.
const MINERAL_CHANCE: f64 = 0.5;

/// R3549001: "a random Mineral": the held set's, tagged Mineral.
fn minerals() -> CatalogQueryArgs {
    json_as(json!({ "set": SetName::StinkyscapePlus, "tags": [Tag::Mineral] }))
}

/// R3549001: one find, a Mineral or a Resource by the flip.
fn find(ctx: &mut EffectContext<'_>, radiant: bool) -> Effect {
    if ctx.rng.chance(MINERAL_CHANCE) {
        add_random_from_catalog(json_as(json!({ "query": minerals(), "radiant": radiant })))
    } else {
        gain_resources(json_as(json!({ "count": 1, "radiant": radiant })))
    }
}

fn drill(radiant: bool) -> Script {
    Script {
        end_of_turn: Some(hook(move |ctx| vec![find(ctx, radiant)])),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: drill(false),
        radiant: drill(true),
    }
}

// S+ #54 Pneumatic Drill — SPEC §8.12 row S+ 54 (R3549001; R62, R1840002, §2.4).
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

    /// The finds over `seeds` games' first end of turn.
    fn finds(radiant: bool, seeds: usize) -> Vec<(Scenario, CardInstance)> {
        (0..seeds)
            .map(|i| {
                let mut s = drilling(radiant, &format!("s54-{radiant}-{i}"), 1);
                s.end_turn();
                let card = gained(&s).pop().expect("one find");
                (s, card)
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn adds_exactly_one_mineral_or_resource() {
            let mut minerals = 0;
            let mut resources = 0;
            for (s, card) in finds(false, 60) {
                assert_eq!(card.zone.z(), ZoneName::Hand);
                assert!(!card.radiant);
                assert_eq!(card.cost_mod, 0);
                assert!(card.cost_override.is_none(), "a Mineral at its printed price");
                if is_mineral(&card) {
                    minerals += 1;
                } else {
                    assert!(is_resource(&s, &card), "{}", card.def_id);
                    resources += 1;
                }
            }
            assert!(minerals > 10 && resources > 10, "a fair flip: {minerals} Minerals, {resources} Resources");
        }

        #[test]
        fn r3549001_one_find_a_turn_and_never_on_the_opponents() {
            let mut s = drilling(false, "s54-turns", 1);
            s.end_turn();
            assert_eq!(gained(&s).len(), 1, "its controller's end of turn");
            s.end_turn();
            assert!(gained(&s).is_empty(), "the opponent's end of turn adds nothing");
            s.end_turn();
            assert_eq!(gained(&s).len(), 1);
        }

        #[test]
        fn r3549001_the_mineral_pool_is_s_plus_44_to_50() {
            let _registered = drilling(false, "s54-registered", 1);
            let pool: Vec<String> = crate::query::query(&minerals()).iter().map(|def| def.index.clone()).collect();
            assert_eq!(pool, ["44", "45", "46", "47", "48", "49", "50"]);
            let mut seen: IndexSet<String> = IndexSet::new();
            for (_, card) in finds(false, 150) {
                if is_mineral(&card) {
                    let def = crate::card_def(&card.def_id);
                    assert_eq!(def.set, SetName::StinkyscapePlus);
                    seen.insert(def.index.clone());
                }
            }
            let mut found: Vec<String> = seen.into_iter().collect();
            found.sort();
            assert_eq!(found, pool);
        }

        #[test]
        fn s2_4_a_full_hand_burns_the_find() {
            let mut s = drilling(false, "s54-full", HAND_CAP as usize);
            s.end_turn();
            assert!(gained(&s).is_empty());
            let burned = s.last_events().iter().filter(|event| matches!(event, GameEvent::Burned { .. })).count();
            assert_eq!(burned, 1);
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3549001_the_find_is_radiant_a_mineral_still_at_its_printed_price() {
            let mut minerals = 0;
            let mut resources = 0;
            for (s, card) in finds(true, 60) {
                assert!(card.radiant, "{}", card.def_id);
                assert_eq!(card.cost_mod, 0);
                assert!(card.cost_override.is_none());
                if is_mineral(&card) {
                    minerals += 1;
                } else {
                    assert!(is_resource(&s, &card), "{}", card.def_id);
                    resources += 1;
                }
            }
            assert!(minerals > 10 && resources > 10, "{minerals} Minerals, {resources} Resources");
        }
    }
}
