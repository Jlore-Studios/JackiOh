//! SPEC §8.10 Stinkyscape 75 Badger, a card of the Stinkyscape set.
//!   Base:    "When this attacks, gain a random Stinkyscape Resource." (2/3)
//!   Radiant: "When this attacks, gain a random Radiant Stinkyscape Resource." (4/6)
//!
//! Since #2521 it is no Starship Piece: its death fuses nothing (R1852000 reads the keyword).
//! "When this attacks" is the engine's `whenAttacks` hook, Classic #45 Nature Titan's (R2396000:
//! "When", not "Whenever", which resolves after the attack): it runs for the player who controls this
//! card before any damage of each attack it makes,
//! declared or forced (R53, R2285000), so a Badger that dies in the combat has already gained; it never
//! runs for a defending Badger, nor for an attack called off in its trap window (R44, Core #96 My
//! Pawn). It is `gain_resources` of one random Resource into the hand (R1840000, R1840002), a ninth
//! sold at once (R1840001). Radiant: a Radiant Resource.

use jackioh_engine::effects::gain_resources;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-075";

/// "Gain a random Stinkyscape Resource": one, printed in the words.
const RESOURCES: i32 = 1;

fn face(radiant: bool) -> Script {
    Script {
        when_attacks: Some(hook(move |_ctx| {
            vec![gain_resources(json_as(
                json!({ "count": RESOURCES, "radiant": radiant }),
            ))]
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

// S 75 Badger — SPEC §8.10 row 75, BUILD M12 row S 75: "attacking a Unit or the hero gains one random
// Resource into its controller's hand, defending none; a forced attack gains one; no Starship Piece, so
// its death fuses nothing (#2521); radiant 4/6, Radiant Resources".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{card_def, filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005";
    const VANILLA: &str = "core-008";
    const BODY: &str = "core-019"; // 9/9
    const MOTHS: &str = "core-009"; // Start of turn: every enemy unit attacks this.
    const PAWN: &str = "core-096"; // Trap: cancels an attack lethal to its controller's hero.
    const NETHER: &str = "core-088"; // (4) Spell: destroy all permanents

    fn badger(radiant: bool, p2: Value) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [FILLER],
                "library": filler(4),
                "field": [{ "def": ID, "radiant": radiant }],
            },
            "p2": p2,
        }))
    }

    fn gained(s: &Scenario) -> Vec<CardInstance> {
        let in_hand = crate::resources_in_hand(s, P1);
        assert!(in_hand.iter().all(|card| card_def(&card.def_id).resource.is_some()));
        in_hand
    }

    #[test]
    fn attacking_the_hero_or_a_unit_gains_one() {
        let mut s = badger(false, json!({ "hand": [FILLER], "library": filler(4) }));
        s.attack(ID, "hero");
        assert_eq!(gained(&s).len(), 1, "the hero");
        assert!(!gained(&s)[0].radiant);

        let mut s = badger(
            false,
            json!({ "hand": [FILLER], "library": filler(4), "field": [VANILLA] }),
        );
        let target = s.unit_at(P2, 1);
        s.attack(ID, &target);
        assert_eq!(gained(&s).len(), 1, "a Unit");
    }

    #[test]
    fn r53_a_forced_attack_gains_one() {
        let mut s = badger(
            false,
            json!({ "hand": [FILLER], "library": filler(4), "field": [MOTHS] }),
        );
        s.end_turn();
        assert_eq!(
            gained(&s).len(),
            1,
            "forced to attack Moths at p2's start of turn"
        );
    }

    #[test]
    fn r2285000_it_gains_before_the_damage_so_dying_in_the_combat_it_has_gained() {
        let mut s = badger(
            false,
            json!({ "hand": [FILLER], "library": filler(4), "field": [BODY] }),
        );
        let target = s.unit_at(P2, 1);
        s.attack(ID, &target);
        s.expect_in_zone(ID, "graveyard");
        assert_eq!(gained(&s).len(), 1);
        s.expect_events(json!(["attackDeclared", "addedToHand", "damage", "destroyed"]));
    }

    #[test]
    fn r44_an_attack_my_pawn_calls_off_gains_nothing() {
        let mut s = badger(
            false,
            json!({ "hand": [FILLER], "library": filler(4), "health": 2, "backrow": [{ "def": PAWN, "faceUp": false }] }),
        );
        s.attack(ID, "hero");
        s.expect_health(P2, 2);
        assert!(
            gained(&s).is_empty(),
            "the attack was called off before it fought"
        );
    }

    #[test]
    fn defending_gains_nothing() {
        let mut s = scenario(json!({
            "active": "p2",
            "p1": { "hand": [FILLER], "library": filler(4), "field": [ID] },
            "p2": { "hand": [FILLER], "library": filler(4), "field": [VANILLA] },
        }));
        let attacker = s.unit_at(P2, 1);
        let defender = s.unit_at(P1, 1);
        s.attack(&attacker, &defender);
        assert!(gained(&s).is_empty());
        assert!(crate::resources_in_hand(&s, P2).is_empty());
    }

    #[test]
    fn its_death_fuses_nothing_it_is_no_starship_piece() {
        let mut s = scenario(json!({
            "p1": { "hand": [NETHER, FILLER], "library": filler(4), "field": [ID] },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        assert!(!has_keyword(&s.stats(ID).keywords, KeywordKind::StarshipPiece));
        s.play(NETHER, json!({}));
        s.expect_in_zone(ID, "graveyard");
        assert!(!building_a_ship(s.state(), P1), "no piece died, so no Dormant Starship");
        assert!(!s.events().iter().any(|event| matches!(event, GameEvent::StarshipFused { .. })));
    }

    #[test]
    fn radiant_radiant_resources() {
        let mut s = badger(true, json!({ "hand": [FILLER], "library": filler(4) }));
        s.attack(ID, "hero");
        let in_hand = gained(&s);
        assert_eq!(in_hand.len(), 1);
        assert!(in_hand[0].radiant);
    }
}
