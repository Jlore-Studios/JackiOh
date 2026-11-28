//! FvH #86 Living Poison Potion (SPEC §8.14 row 86): (1) Unit, Potion, Rare, 4/2 → 8/4.
//!   Base:    "Activate: Tribute this. Toxic {toxic} all enemy Units." (2)
//!   Radiant: "Activate: Toxic {toxic} all enemy Units." (2)
//!
//! M+ #11 Living Mana Potion's shape. Activate (R384), tributing itself as its cost on the base face (C #84),
//! paid first: `toxic_all` of `toxic` on every enemy Unit, no hero, ticking at the start of each poisoned
//! Unit's controller's turn past Armor (R3540001). Radiant: no cost, once a turn, the potion staying.
//! Tunes: toxic 2 ↑.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-086";

/// "Activate" is once a turn (R384).
const USES: i32 = 1;

fn potion(radiant: bool) -> Script {
    Script {
        activations: vec![ActivationDecl {
            id: "toxic".to_string(),
            label: if radiant {
                "Toxic all enemy Units".to_string()
            } else {
                "Tribute this, then Toxic all enemy Units".to_string()
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
                vec![toxic_all(json_as(json!({ "amount": param(&*ctx, "toxic"), "side": "enemy" })))]
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

// FvH #86 Living Poison Potion — SPEC §8.14 row 86 (R384, R3540001).
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
    const ARMORED: &str = "stand-in (4) 7/7 Armor 7";
    /// Its printed `toxic`.
    const TOXIC: i32 = 2;

    fn brewed(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [SPELL],
                "field": [{ "def": ID, "radiant": radiant }, FRIEND],
                "library": filler(6),
            },
            "p2": { "hand": [SPELL], "field": [ENEMY, ARMORED], "library": filler(6) },
        }))
    }

    /// Every enemy Unit carries `toxic`; the enemy hero and p1's side carry none.
    fn enemy_units_toxic(s: &Scenario, toxic: i32) {
        for unit in [ENEMY, ARMORED] {
            assert_eq!(toxic_on(s.card(unit)), toxic, "{unit}");
        }
        assert_eq!(hero_toxic_of(s.state(), P2), 0, "no hero");
        assert_eq!(toxic_on(s.card(FRIEND)), 0);
        assert_eq!(hero_toxic_of(s.state(), P1), 0);
    }

    mod base {
        use super::*;

        #[test]
        fn r384_its_activate_tributes_it_first_then_applies_toxic_2_to_every_enemy_unit_and_no_hero() {
            let mut s = brewed(false);
            let potion = s.card(ID).clone();
            s.activate(ID, json!({}));
            s.expect_in_zone(&potion, "graveyard");
            enemy_units_toxic(&s, TOXIC);
        }

        #[test]
        fn r3540001_the_toxic_ticks_past_armor_at_the_start_of_their_controllers_turn_and_stays() {
            let mut s = brewed(false);
            s.activate(ID, json!({}));
            s.end_turn();
            s.expect_stats(ENEMY, json!({ "health": 5 - TOXIC }));
            s.expect_stats(ARMORED, json!({ "health": 7 - TOXIC }));
            enemy_units_toxic(&s, TOXIC);
        }

        #[test]
        fn r386_its_toxic_reads_through_param() {
            let mut s = brewed(false);
            let potion = s.card(ID).id.clone();
            set_param(s.card_mut(&potion), "toxic", TOXIC + 1);
            s.activate(&potion, json!({}));
            enemy_units_toxic(&s, TOXIC + 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r384_it_applies_toxic_2_to_every_enemy_unit_without_a_tribute_and_stays_once_a_turn() {
            let mut s = brewed(true);
            s.activate(ID, json!({}));
            s.expect_in_zone(ID, "field");
            enemy_units_toxic(&s, TOXIC);
            s.expect_refused(|s| s.activate(ID, json!({})));
            enemy_units_toxic(&s, TOXIC);
        }
    }
}
