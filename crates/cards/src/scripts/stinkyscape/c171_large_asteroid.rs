//! SPEC §8.10 Stinkyscape 171 Large Asteroid, a card of the held Stinkyscape set (R1420).
//!   Base:    "Can't attack\nCan't be in Defense Position.\nDeath: Gain {resources|random Stinkyscape
//!            Resource|random Stinkyscape Resources}.\nActivate: Take {selfDamage} damage. Tribute
//!            this." (0/8; 3, 4)
//!   Radiant: "… Death: Gain {resources|random Radiant Stinkyscape Resource|…}.\nActivate: Tribute
//!            this." (0/16; 3)
//!
//! A mining node (R1840006): Can't attack is printed, and `StaticFlags.never_defense` keeps it out of
//! Defense Position (§4.1). Activate (§6.2, R384), once a turn: its controller's hero takes `selfDamage`
//! (§4.4), then a Sacrifice of it (§6.3), a death, so its Death runs: `gain_resources` of `resources`
//! random Resources into its controller's tray (R1840000, R1840002). Any other death pays too; an exile
//! or a Bounce nothing. Radiant: the Activate is the Sacrifice alone, and the Resources are Radiant.
//! The numbers are declared (R386), `selfDamage` on the base face only (R1431).

use jackioh_engine::effects::{damage, gain_resources, sacrifice};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-171";

/// The Activate's id.
const MINE: &str = "mine";

/// "Activate: Take {selfDamage} damage. Tribute this.", the damage dropped on the Radiant face.
fn mine(radiant: bool) -> ActivationDecl {
    ActivationDecl {
        id: MINE.to_string(),
        label: if radiant {
            "Tribute this"
        } else {
            "Take damage, then Tribute this"
        }
        .to_string(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(move |ctx| {
            let tribute = sacrifice(json_as(json!({ "target": { "of": "self" } })));
            if radiant {
                vec![tribute]
            } else {
                vec![
                    damage(json_as(
                        json!({ "to": { "of": "selfHero" }, "amount": param(&*ctx, "selfDamage") }),
                    )),
                    tribute,
                ]
            }
        }),
    }
}

fn face(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            never_defense: Some(true),
            ..StaticFlags::default()
        }),
        activations: vec![mine(radiant)],
        death: Some(hook(move |ctx| {
            vec![gain_resources(json_as(
                json!({ "count": param(&*ctx, "resources"), "radiant": radiant }),
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

// S 171 Large Asteroid — SPEC §8.10 row 171, BUILD M12 row S 171: "its Activate deals 4 damage to its
// controller's hero, then tributes it, and its Death gains 3 random Stinkyscape Resources into the tray;
// destroyed in combat or by an effect it pays the same; it never attacks or switches to Defense;
// resources and selfDamage read through `param()`; radiant 0/16, the Activate dealing no damage, Radiant
// Resources".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::{ID, MINE};
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005";
    const BODY: &str = "core-019"; // 9/9
    const ROCK: &str = "core-066"; // 10/10, Indestructible
    const NETHER: &str = "core-088"; // (4) Spell: destroy all permanents
    const SELF_DAMAGE: i32 = 4;
    const RESOURCES: usize = 3;

    fn node(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [NETHER, FILLER],
                "library": filler(4),
                "field": [{ "def": ID, "radiant": radiant }],
            },
            "p2": { "hand": [FILLER], "library": filler(4), "field": [BODY] },
        }))
    }

    fn mined(s: &mut Scenario) {
        s.activate(ID, json!({ "ability": MINE }));
    }

    #[test]
    fn r1840006_activate_damages_then_tributes_and_pays() {
        let mut s = node(false);
        mined(&mut s);
        s.expect_health(P1, 30 - SELF_DAMAGE);
        s.expect_in_zone(ID, "graveyard");
        let tray = s.pile(P1, "tray");
        assert_eq!(tray.len(), RESOURCES);
        assert!(tray.iter().all(|card| !card.radiant));
    }

    #[test]
    fn destroyed_in_combat_or_by_twisting_nether_it_pays() {
        let mut s = node(false);
        s.play(NETHER, json!({}));
        assert_eq!(s.pile(P1, "tray").len(), RESOURCES, "Twisting Nether");

        let mut s = scenario(json!({
            "active": "p2",
            "p1": { "hand": [FILLER], "library": filler(4), "field": [ID] },
            "p2": { "hand": [FILLER], "library": filler(4), "field": [ROCK] },
        }));
        let attacker = s.unit_at(P2, 1);
        let node = s.unit_at(P1, 1);
        s.attack(&attacker, &node);
        s.expect_in_zone(&node, "graveyard");
        assert_eq!(s.pile(P1, "tray").len(), RESOURCES, "in combat");
        assert!(s.pile(P2, "tray").is_empty());
    }

    #[test]
    fn never_attacks_or_switches() {
        let mut s = node(false);
        s.expect_refused(|s| s.attack(ID, "hero"));
        s.expect_refused(|s| s.switch_position(ID));
        let target = s.unit_at(P2, 1);
        s.expect_refused(|s| s.attack(ID, &target));
    }

    #[test]
    fn resources_and_self_damage_read_through_param() {
        let mut s = node(false);
        set_param(s.card_mut(ID), "resources", 3);
        set_param(s.card_mut(ID), "selfDamage", 1);
        mined(&mut s);
        s.expect_health(P1, 29);
        assert_eq!(s.pile(P1, "tray").len(), 3);
    }

    #[test]
    fn radiant_no_damage_radiant_resources() {
        let mut s = node(true);
        mined(&mut s);
        s.expect_health(P1, 30);
        s.expect_in_zone(ID, "graveyard");
        let tray = s.pile(P1, "tray");
        assert_eq!(tray.len(), RESOURCES);
        assert!(tray.iter().all(|card| card.radiant));
    }
}
