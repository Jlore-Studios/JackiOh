//! SPEC §8.12 S+ #52 Airblast Drill (§2.4, §5.1, §6.1 GIGA, §6.3 Tribute; R62, R101, R1420, R1840002, R3535000,
//! R3546000, R3549001): (4) Field Spell, Mindustrial, Legendary, a card of the held Stinkyscape+ set.
//!   Base:    "Tribute 1, GIGA
//!            End of turn: Fill your hand with random Minerals (that cost 0) and Resources."
//!   Radiant: "Tribute 1, GIGA
//!            End of turn: Fill your hand with random Radiant Minerals (that cost 0) and Radiant Resources."
//!
//! Tribute 1 is the play validator's (§6.3, R101), read off `staticFlags.tribute`: one Unit of its player's
//! is destroyed to pay it, and the zones it frees count for the GIGA anchor (R3546000). GIGA is catalog
//! data, a keyword the engine reads: the card stands in four zones. At the end of its controller's turn
//! (R62) it fills the room in their hand (`room_in`, "fill your hand", as Stinkyscape #45's Cry does) with
//! that many finds (R3549001), each its own flip (`MINERAL_CHANCE`) between a Mineral and a Resource, as
//! #53's: a Mineral is a random card of the Stinkyscape+ cards tagged Mineral (#44 to #50), the pool naming
//! its set so that it reaches them while the set is held (R1420), and it costs (0) (`costOverride`, "that
//! cost 0"); a Resource is `gainResources` (R1840002). The Radiant face fills with Radiant cards. A full
//! hand adds nothing and draws nothing from the rng.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-052";

/// "Tribute 1" (R101).
const TRIBUTE: i32 = 1;
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
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE),
            ..StaticFlags::default()
        }),
        end_of_turn: Some(hook(move |ctx| {
            let room = room_in(ctx.state, ctx.controller, OffFieldZone::Hand);
            (0..room).map(|_| find(ctx, radiant)).collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: drill(false),
        radiant: drill(true),
    }
}

// S+ #52 Airblast Drill — SPEC §8.12 row S+ 52 (R3549001; R62, R101, R1840002, R3546000, §2.4).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::play_choices::tribute_cost_of;
    use jackioh_engine::testkit::*;
    use jackioh_engine::zones::is_empty;

    use crate::{filler, weapon_board};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";

    /// The drill already standing in p1's backrow, `hand` Stockpiles in p1's hand.
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

    /// The drill in p1's hand, a Unit on the field to pay its Tribute.
    fn to_play(radiant: bool, seed: &str) -> Scenario {
        weapon_board(
            seed,
            json!({
                "hand": [{ "def": ID, "radiant": radiant }],
                "library": filler(6),
                "field": [VANILLA],
                "mana": 10,
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

    /// Every card p1 gained is a Mineral that costs 0 or a Resource.
    fn check_finds(s: &Scenario, radiant: bool) -> (usize, usize) {
        let (mut minerals, mut resources) = (0, 0);
        for card in gained(s) {
            assert_eq!(card.zone.z(), ZoneName::Hand);
            assert_eq!(card.radiant, radiant, "{}", card.def_id);
            if is_mineral(&card) {
                assert_eq!(effective_cost(s.state(), &card, Default::default()), 0, "{}", card.def_id);
                minerals += 1;
            } else {
                assert!(is_resource(s, &card), "{}", card.def_id);
                resources += 1;
            }
        }
        (minerals, resources)
    }

    #[test]
    fn r101_tribute_1_is_required() {
        let mut s = to_play(false, "s52-tribute");
        assert_eq!(tribute_cost_of(s.state(), s.card(ID)), TRIBUTE);
        // No tribute is refused; one Unit pays it.
        s.expect_refused(|s| s.play(ID, json!({ "zone": 1 })));
        let unit = s.unit_at(P1, 1).id;
        s.play(ID, json!({ "zone": 1, "tributes": [unit.as_str()] }));
        s.expect_in_zone(ID, "field");
        s.expect_in_zone(unit.as_str(), "graveyard");
    }

    #[test]
    fn r3546000_giga_fills_four_zones_the_tribute_frees_one_of_them() {
        let mut s = to_play(false, "s52-giga");
        let unit = s.unit_at(P1, 1).id;
        s.play(ID, json!({ "zone": 1, "tributes": [unit.as_str()] }));
        assert_eq!(s.card(ID).giga, Some(true));
        for (row, lane) in [(Row::Units, 1), (Row::Units, 2), (Row::Backrow, 1), (Row::Backrow, 2)] {
            assert!(
                !is_empty(s.state(), ZoneSlot { player: P1, row, lane }),
                "{row} {lane} is filled by the drill"
            );
        }
        assert!(is_empty(s.state(), ZoneSlot { player: P1, row: Row::Units, lane: 3 }));
    }

    mod base {
        use super::*;

        #[test]
        fn r3549001_end_of_turn_fills_the_hand_with_minerals_that_cost_0_and_resources() {
            let mut minerals = 0;
            let mut resources = 0;
            for i in 0..30 {
                let mut s = drilling(false, &format!("s52-fill-{i}"), 3);
                s.end_turn();
                assert_eq!(s.hand(P1).len(), HAND_CAP as usize, "the hand is full");
                assert_eq!(gained(&s).len(), HAND_CAP as usize - 3);
                let (m, r) = check_finds(&s, false);
                minerals += m;
                resources += r;
            }
            assert!(minerals > 30 && resources > 30, "{minerals} Minerals, {resources} Resources");
        }

        #[test]
        fn r3549001_an_empty_hand_fills_to_the_cap_and_nothing_burns() {
            let mut s = drilling(false, "s52-empty", 0);
            s.end_turn();
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            assert!(!s.last_events().iter().any(|event| matches!(event, GameEvent::Burned { .. })));
        }

        #[test]
        fn a_full_hand_gains_nothing() {
            let mut s = drilling(false, "s52-full", HAND_CAP as usize);
            s.end_turn();
            assert!(gained(&s).is_empty());
            assert!(!s.last_events().iter().any(|event| matches!(event, GameEvent::Burned { .. })));
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
        }

        #[test]
        fn only_at_the_end_of_its_controllers_turn() {
            let mut s = drilling(false, "s52-turns", 3);
            s.end_turn();
            assert!(!gained(&s).is_empty());
            s.end_turn();
            assert!(gained(&s).is_empty(), "the opponent's end of turn adds nothing");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3549001_it_fills_the_hand_with_radiant_cards() {
            let mut minerals = 0;
            let mut resources = 0;
            for i in 0..30 {
                let mut s = drilling(true, &format!("s52-rfill-{i}"), 3);
                s.end_turn();
                assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
                let (m, r) = check_finds(&s, true);
                minerals += m;
                resources += r;
            }
            assert!(minerals > 30 && resources > 30, "{minerals} Minerals, {resources} Resources");
        }

        #[test]
        fn r101_it_needs_tribute_1_too() {
            let mut s = to_play(true, "s52-rtribute");
            assert_eq!(tribute_cost_of(s.state(), s.card(ID)), TRIBUTE);
            s.expect_refused(|s| s.play(ID, json!({ "zone": 1 })));
        }
    }
}
