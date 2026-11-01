//! SPEC §8.10 Stinkyscape 97 Asteroid Field, a card of the Stinkyscape set.
//!   Base:    "Cast on draw: Deal {damage} damage to both heroes and all Units. Gain {resources|random
//!            Stinkyscape Resource|random Stinkyscape Resources}." (2, 1; 2, 2 until #2657)
//!   Radiant: "Cast on draw: Deal {damage} damage to all enemies. Gain {resources|random Radiant
//!            Stinkyscape Resource|random Radiant Stinkyscape Resources}." (2, 1; 2, 2 until #2657)
//!
//! Cast on draw (§6.2, `StaticFlags.cast_on_draw`): drawn, it is cast free and counted as played (R70),
//! its owner then drawing again (R58); setup never deals it (R635). Its cast is one `damage_all` hit on
//! every Unit of both sides and then each hero, one state check after them all (R59), its caster's Spell
//! Damage raising each (§4.4 step 0); then `gain_resources` of `resources` random Resources into the
//! hand, a full hand burning them (R2250000). Radiant: the enemy Units and the enemy hero only, and
//! Radiant Resources. The numbers are the declared `damage` and `resources` (R386).

use jackioh_engine::effects::{damage_all, gain_resources};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-097";

fn face(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(move |ctx| {
            vec![
                damage_all(json_as(json!({
                    "side": if radiant { "enemy" } else { "any" },
                    "amount": param(&*ctx, "damage"),
                    "heroes": true,
                }))),
                gain_resources(json_as(
                    json!({ "count": param(&*ctx, "resources"), "radiant": radiant }),
                )),
            ]
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

// S 97 Asteroid Field — SPEC §8.10 row 97, BUILD M12 row S 97 (R635).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005";
    const BODY: &str = "core-019"; // 9/9

    fn field_on_top(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [FILLER],
                "library": [{ "def": ID, "radiant": radiant }, FILLER, FILLER],
                "field": [BODY],
            },
            "p2": { "hand": [FILLER], "library": filler(4), "field": [BODY] },
        }))
    }

    #[test]
    fn drawn_it_hits_both_heroes_and_all_units_gains_1_and_draws() {
        let mut s = field_on_top(false);
        let (own, enemy) = (s.unit_at(P1, 1), s.unit_at(P2, 1));
        s.start_turn();
        s.expect_health(P1, 28).expect_health(P2, 28);
        s.expect_stats(&own, json!({ "health": 7 }));
        s.expect_stats(&enemy, json!({ "health": 7 }));
        let in_hand = crate::resources_in_hand(&s, P1);
        assert_eq!(in_hand.len(), 1);
        assert!(in_hand.iter().all(|card| !card.radiant));
        s.expect_in_zone(ID, "graveyard");
        assert_eq!(
            s.hand(P1).len(),
            3,
            "the draw repeated (R58), beside the one Resource (R2250000)"
        );
    }

    #[test]
    fn damage_and_resources_read_through_param() {
        let mut s = field_on_top(false);
        set_param(s.card_mut(ID), "damage", 3);
        set_param(s.card_mut(ID), "resources", 1);
        s.start_turn();
        s.expect_health(P1, 27).expect_health(P2, 27);
        assert_eq!(crate::resources_in_hand(&s, P1).len(), 1);
    }

    #[test]
    fn r635_setup_never_deals_it() {
        crate::register_all();
        let rest = [
            "core-001", "core-002", "core-003", "core-004", "core-005", "core-006", "core-007", "core-008",
            "core-009", "core-010", "core-011", "core-012", "core-013", "core-014", "core-016", "core-017",
            "core-019", "core-020", "core-025", "core-032", "core-036", "core-043", "core-044", "core-053",
        ];
        let mut mine = vec![ID.to_string()];
        mine.extend(rest.iter().map(|id| id.to_string()));
        let mut theirs = vec!["core-026".to_string()];
        theirs.extend(rest.iter().map(|id| id.to_string()));
        for n in 0..50 {
            let options: CreateGameOptions = json_as(json!({
                "seed": format!("asteroid-deal-{n}"),
                "decks": [mine, theirs],
            }));
            let begun = begin_game(&create_game(&options));
            assert!(begun.state.players.p1.hand.iter().all(|card| card.def_id != ID));
            assert!(
                begun
                    .state
                    .players
                    .p1
                    .hand
                    .iter()
                    .all(|card| crate::card_def(&card.def_id).resource.is_none()),
                "{n}: setup cast none"
            );
            assert!(
                begun
                    .state
                    .players
                    .p1
                    .library
                    .iter()
                    .any(|card| card.def_id == ID)
            );
        }
    }

    #[test]
    fn radiant_only_enemies_are_hit() {
        let mut s = field_on_top(true);
        let (own, enemy) = (s.unit_at(P1, 1), s.unit_at(P2, 1));
        s.start_turn();
        s.expect_health(P1, 30).expect_health(P2, 28);
        s.expect_stats(&own, json!({ "health": 9 }));
        s.expect_stats(&enemy, json!({ "health": 7 }));
        let in_hand = crate::resources_in_hand(&s, P1);
        assert_eq!(in_hand.len(), 1);
        assert!(in_hand.iter().all(|card| card.radiant));
    }
}
