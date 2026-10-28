//! SPEC §8.10 Stinkyscape 75 Badger, a card of the held Stinkyscape set (R1420).
//!   Base:    "Starship Piece\nWhenever this attacks, gain a random Stinkyscape Resource." (2/3)
//!   Radiant: "Starship Piece\nWhenever this attacks, gain a random Radiant Stinkyscape Resource." (4/6)
//!
//! Starship Piece is the engine's printed keyword: its death fuses a copy into its owner's Dormant
//! Starship (R1852000). "Whenever this attacks" is the engine's `afterAttack` hook, Core #32 Prem
//! Panther's: it runs once each combat this card attacked in, declared or forced (R53), is over and
//! the state check after it has run (R59), for the player who controlled it there, even when it died
//! in it (R426); it never runs for a defending Badger, nor for an attack called off before it fought
//! (R44, Core #96 My Pawn). It is `gain_resources` of one random Resource into the tray (R1840000,
//! R1840002), a ninth sold at once (R1840001). Radiant: a Radiant Resource.

use jackioh_engine::effects::gain_resources;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-075";

/// "Gain a random Stinkyscape Resource": one, printed in the words.
const RESOURCES: i32 = 1;

fn face(radiant: bool) -> Script {
    Script {
        after_attack: Some(hook(move |_ctx| {
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
// Resource into its controller's tray, defending none; a forced attack gains one; it dies and fuses into
// a Dormant Starship; radiant 4/6, Radiant Resources".
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
        let tray = s.pile(P1, "tray");
        assert!(tray.iter().all(|card| card_def(&card.def_id).resource.is_some()));
        tray
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
    fn r426_dying_in_the_combat_it_still_gains() {
        let mut s = badger(
            false,
            json!({ "hand": [FILLER], "library": filler(4), "field": [BODY] }),
        );
        let target = s.unit_at(P2, 1);
        s.attack(ID, &target);
        s.expect_in_zone(ID, "graveyard");
        assert_eq!(gained(&s).len(), 1);
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
        assert!(s.pile(P2, "tray").is_empty());
    }

    #[test]
    fn r1852000_its_death_builds_a_ship() {
        let mut s = scenario(json!({
            "p1": { "hand": [NETHER, FILLER], "library": filler(4), "field": [ID] },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(NETHER, json!({}));
        assert!(building_a_ship(s.state(), P1));
    }

    #[test]
    fn radiant_radiant_resources() {
        let mut s = badger(true, json!({ "hand": [FILLER], "library": filler(4) }));
        s.attack(ID, "hero");
        let tray = gained(&s);
        assert_eq!(tray.len(), 1);
        assert!(tray[0].radiant);
    }
}
