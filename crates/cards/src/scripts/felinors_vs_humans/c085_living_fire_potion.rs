//! FvH #85 Living Fire Potion (SPEC §8.14 row 85): (1) Unit, Potion, Rare, 4/2 → 8/4.
//!   Base:    "Activate: Tribute this. Burn {burn} all enemies." (2)
//!   Radiant: "Activate: Burn {burn} all enemies." (2)
//!
//! M+ #11 Living Mana Potion's shape. Activate (R384), tributing itself as its cost on the base face (C #84),
//! paid first, so the Burn is the potion's last act: `burn_all` of `burn` on every enemy Unit and the enemy
//! hero (Hearthstone's "all enemies"), ticking once at the end of a turn (R4154000). Radiant: no cost, once
//! a turn, the potion staying. Tunes: burn 2 ↑.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-085";

/// "Activate" is once a turn (R384).
const USES: i32 = 1;

fn potion(radiant: bool) -> Script {
    Script {
        activations: vec![ActivationDecl {
            id: "burn".to_string(),
            label: if radiant {
                "Burn all enemies".to_string()
            } else {
                "Tribute this, then Burn all enemies".to_string()
            },
            uses: ActivationUses::Count(USES),
            cost: (!radiant).then(|| ActivationCost {
                tribute_self: Some(true),
                ..ActivationCost::default()
            }),
            targets: Vec::new(),
            modes: Vec::new(),
            can_activate: None,
            has: None,
            run: hook(|ctx| {
                vec![burn_all(json_as(json!({
                    "amount": param(&*ctx, "burn"),
                    "heroes": true,
                    "side": "enemy",
                })))]
            }),
        }],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: potion(false),
        radiant: potion(true),
    }
}

// FvH #85 Living Fire Potion — SPEC §8.14 row 85 (R384, R4154000).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A textless Spell each seat can always play, so no turn ends by itself (§2.5).
    const SPELL: &str = "stand-in (0) Spell";
    const FRIEND: &str = "stand-in (1) 4/4";
    const ENEMY: &str = "stand-in (2) 5/5";
    const OTHER_ENEMY: &str = "stand-in (3) 6/6";
    /// The heroes' health before any hit (§2.1).
    const HERO: i32 = 30;
    /// Its printed `burn`.
    const BURN: i32 = 2;

    fn brewed(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [SPELL],
                "field": [{ "def": ID, "radiant": radiant }, FRIEND],
                "library": filler(6),
            },
            "p2": { "hand": [SPELL], "field": [ENEMY, OTHER_ENEMY], "library": filler(6) },
        }))
    }

    /// Every enemy Unit and the enemy hero carry `burn`; p1's side carries none.
    fn enemies_burn(s: &Scenario, burn: i32) {
        for unit in [ENEMY, OTHER_ENEMY] {
            assert_eq!(burn_on(s.card(unit)), burn, "{unit}");
        }
        assert_eq!(hero_burn_of(s.state(), P2), burn);
        assert_eq!(burn_on(s.card(FRIEND)), 0);
        assert_eq!(hero_burn_of(s.state(), P1), 0);
    }

    mod base {
        use super::*;

        #[test]
        fn r384_its_activate_tributes_it_first_then_burns_2_every_enemy_unit_and_the_enemy_hero() {
            let mut s = brewed(false);
            let potion = s.card(ID).clone();
            s.activate(ID, json!({}));
            s.expect_in_zone(&potion, "graveyard");
            enemies_burn(&s, BURN);
        }

        #[test]
        fn r4154000_the_burn_ticks_once_at_the_end_of_the_turn_and_is_gone() {
            let mut s = brewed(false);
            s.activate(ID, json!({}));
            s.end_turn();
            s.expect_stats(ENEMY, json!({ "health": 5 - BURN }));
            s.expect_stats(OTHER_ENEMY, json!({ "health": 6 - BURN }));
            s.expect_health(P2, HERO - BURN);
            enemies_burn(&s, 0);
        }

        #[test]
        fn r386_its_burn_reads_through_param() {
            let mut s = brewed(false);
            let potion = s.card(ID).id.clone();
            set_param(s.card_mut(&potion), "burn", BURN + 1);
            s.activate(&potion, json!({}));
            enemies_burn(&s, BURN + 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r384_it_burns_every_enemy_without_a_tribute_and_stays_once_a_turn() {
            let mut s = brewed(true);
            s.activate(ID, json!({}));
            s.expect_in_zone(ID, "field");
            enemies_burn(&s, BURN);
            s.expect_refused(|s| s.activate(ID, json!({})));
            enemies_burn(&s, BURN);
        }
    }
}
