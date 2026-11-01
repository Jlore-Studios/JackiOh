//! SPEC §8.10 Stinkyscape 171 Large Asteroid, a card of the Stinkyscape set.
//!   Base:    "Can't attack\nCan't be in Defense Position.\nDeath: Gain {resources|random Stinkyscape
//!            Resource|random Stinkyscape Resources}.\nActivate: Take {selfDamage} damage. Tribute
//!            this." (0/8; 3, 4)
//!   Radiant: "… Death: Gain {resources|random Radiant Stinkyscape Resource|…}.\nActivate: Tribute
//!            this." (0/16; 3)
//!
//! A mining node (R1840006): Can't attack is printed, and Can't Defend (R2657000) keeps it out of
//! Defense Position (§4.1). Activate (§6.2, R384), once a turn: its controller's hero takes `selfDamage`
//! (§4.4), then a Sacrifice of it (§6.3), a death, so its Death runs: `gain_resources` of `resources`
//! random Resources into its controller's hand (R1840000, R1840002). Any other death pays too; an exile
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
        activations: vec![mine(radiant)],
        death: Some(hook(move |ctx| {
            vec![gain_resources(json_as(
                json!({ "count": param(&*ctx, "resources"), "radiant": radiant }),
            ))]
        })),
        preview: Some(super::c169_tiny_asteroid::pool_preview()),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

/// R767000: through v0.5.9f "Can't be in Defense Position" was the static flag `never_defense` on both
/// faces, before R2657000 made it the keyword Can't Defend; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_9f() -> CardScripts {
    crate::never_defense_kept(script(), true, true)
}

// S 171 Large Asteroid — SPEC §8.10 row 171, BUILD M12 row S 171.
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
        let in_hand = crate::resources_in_hand(&s, P1);
        assert_eq!(in_hand.len(), RESOURCES);
        assert!(in_hand.iter().all(|card| !card.radiant));
    }

    #[test]
    fn destroyed_in_combat_or_by_twisting_nether_it_pays() {
        let mut s = node(false);
        s.play(NETHER, json!({}));
        assert_eq!(crate::resources_in_hand(&s, P1).len(), RESOURCES, "Twisting Nether");

        let mut s = scenario(json!({
            "active": "p2",
            "p1": { "hand": [FILLER], "library": filler(4), "field": [ID] },
            "p2": { "hand": [FILLER], "library": filler(4), "field": [ROCK] },
        }));
        let attacker = s.unit_at(P2, 1);
        let node = s.unit_at(P1, 1);
        s.attack(&attacker, &node);
        s.expect_in_zone(&node, "graveyard");
        assert_eq!(crate::resources_in_hand(&s, P1).len(), RESOURCES, "in combat");
        assert!(crate::resources_in_hand(&s, P2).is_empty());
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
        assert_eq!(crate::resources_in_hand(&s, P1).len(), 3);
    }

    #[test]
    fn radiant_no_damage_radiant_resources() {
        let mut s = node(true);
        mined(&mut s);
        s.expect_health(P1, 30);
        s.expect_in_zone(ID, "graveyard");
        let in_hand = crate::resources_in_hand(&s, P1);
        assert_eq!(in_hand.len(), RESOURCES);
        assert!(in_hand.iter().all(|card| card.radiant));
    }

    /// R2476000, R1855002: with no rider it previews nothing; with an Anomaly board's rider it names the
    /// pool after "random Stinkyscape Resource" (Radiant: "random Radiant Stinkyscape Resource"), to
    /// both seats, and its Death's Resources come from that pool alone.
    #[test]
    fn r2476000_it_names_the_resource_pool_its_rider_holds() {
        for radiant in [false, true] {
            let mut s = node(radiant);
            let rock = s.unit_at(P1, 1).id;
            assert!(crate::preview_seen(&s, P1, &rock).is_empty(), "radiant {radiant}: no rider");
            let pool = ["stinkyscape-t-narcor", "stinkyscape-t-red-narcor"];
            s.card_mut(&rock).memory.insert(jackioh_engine::effects::RESOURCE_POOL_KEY.to_string(), json!(pool));
            let label = if radiant { "random Radiant Stinkyscape Resource" } else { "random Stinkyscape Resource" };
            let named = vec![(label.to_string(), 2, Some("Narcor or Red Narcor".to_string()))];
            assert_eq!(crate::preview_seen(&s, P1, &rock), named, "radiant {radiant}");
            assert_eq!(crate::preview_seen(&s, P2, &rock), named, "radiant {radiant}");
            mined(&mut s);
            let gained = crate::resources_in_hand(&s, P1);
            assert!(!gained.is_empty(), "radiant {radiant}");
            assert!(gained.iter().all(|card| pool.contains(&card.def_id.as_str())), "radiant {radiant}");
        }
    }
}
