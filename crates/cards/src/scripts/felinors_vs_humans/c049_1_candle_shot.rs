//! FvH #49.1 Candle Shot (SPEC §8.14 row 49.1): (1) Field Spell, Token (printed Common), the token FvH #49
//! Candle Master summons.
//!   Base:    "Pierce, Durability 3\nActivate: Deal {damage} damage to an enemy." (1)
//!   Radiant: "Pierce, Durability 3\nActivate: Deal {damage} damage to an enemy." (2; FV2 #49.1, the list
//!            prints no Radiant face)
//!
//! Engine: Activate, once a turn (R384): one declared enemy, Unit or hero (R81), `damage` of the declared
//! `damage`, its printed Pierce read off the source (R346). Durability 3 is the engine's keyword
//! (R5236000): at 0, after the use resolves, it goes to its owner's graveyard by a plain move, no Death
//! (R5236002), and stays there as a Spell token does (R11). `dealsDamage` (R1748001); its 3 is a keyword
//! count (R1748003). The Radiant face differs only in its declared `damage`, so it runs the same script.
//! Tunes: damage 1 ↑ (Radiant 2).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-049-1";

/// "Activate" is once a turn (R384).
const USES: i32 = 1;

fn face() -> Script {
    let shot = ActivationDecl {
        id: "shot".to_string(),
        label: "Deal damage to an enemy".to_string(),
        uses: ActivationUses::Count(USES),
        cost: None,
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit", "hero"] }))],
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(|ctx| {
            vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": param(&*ctx, "damage") })))]
        }),
    };
    Script {
        activations: vec![shot],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = face();
    // The Radiant face's 2 damage is its declared number: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #49.1 Candle Shot — SPEC §8.14 row 49.1 (R384, R81, R346, R5236000, R5236002, R5236003, R11, R386).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, run_as, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A textless Spell each seat can always play, so no turn ends by itself (§2.5).
    const SPELL: &str = "stand-in (1) Spell";
    const ARMORED: &str = "stand-in (4) 7/7 Armor 7";
    const FRIEND: &str = "stand-in (1) 4/4";
    /// The heroes' health before any hit (§2.1).
    const HERO: i32 = 30;
    /// Its printed `damage`, base and Radiant.
    const DAMAGE: i32 = 1;
    const RADIANT_DAMAGE: i32 = 2;
    /// Its printed Durability.
    const DURABILITY: i32 = 3;

    fn lit(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [SPELL],
                "field": [FRIEND],
                "backrow": [{ "def": ID, "radiant": radiant, "faceUp": true }],
                "library": filler(6),
            },
            "p2": { "hand": [SPELL], "field": [ARMORED], "library": filler(6) },
        }))
    }

    fn at_hero(s: &mut Scenario) -> &mut Scenario {
        s.activate(ID, json!({ "targets": pick::hero(P2) }))
    }

    mod base {
        use super::*;

        #[test]
        fn r81_its_activate_deals_1_to_the_chosen_enemy_hero_once_a_turn() {
            let mut s = lit(false);
            at_hero(&mut s);
            s.expect_health(P2, HERO - DAMAGE);
            s.expect_refused(at_hero);
            s.expect_health(P2, HERO - DAMAGE);
        }

        #[test]
        fn r346_its_printed_pierce_skips_the_enemy_units_armor() {
            let mut s = lit(false);
            let armored = s.card(ARMORED).clone();
            s.activate(ID, json!({ "targets": pick::instance(&armored) }));
            s.expect_stats(&armored, json!({ "health": 7 - DAMAGE }));
        }

        #[test]
        fn r81_a_friendly_unit_or_hero_is_no_target() {
            let mut s = lit(false);
            let friend = s.card(FRIEND).clone();
            s.expect_refused(|s| s.activate(ID, json!({ "targets": pick::instance(&friend) })));
            s.expect_refused(|s| s.activate(ID, json!({ "targets": pick::hero(P1) })));
            s.expect_health(P1, HERO);
        }

        #[test]
        fn r5236002_r11_three_uses_spend_its_durability_3_and_it_stays_in_the_graveyard() {
            let mut s = lit(false);
            let shot = s.card(ID).clone();
            for left in (0..DURABILITY).rev() {
                at_hero(&mut s);
                if left > 0 {
                    assert_eq!(durability_left(s.state(), s.card(&shot.id)), Some(left));
                    run_as(&mut s, refresh_activates(), P1);
                }
            }
            s.expect_health(P2, HERO - DURABILITY * DAMAGE);
            s.expect_in_zone(&shot, "graveyard");
            assert_eq!(s.count("destroyed"), 0);
        }

        #[test]
        fn r386_its_damage_reads_through_param() {
            let mut s = lit(false);
            let shot = s.card(ID).id.clone();
            set_param(s.card_mut(&shot), "damage", DAMAGE + 2);
            at_hero(&mut s);
            s.expect_health(P2, HERO - DAMAGE - 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r81_its_activate_deals_2_to_the_chosen_enemy() {
            let mut s = lit(true);
            at_hero(&mut s);
            s.expect_health(P2, HERO - RADIANT_DAMAGE);
            assert_eq!(durability_left(s.state(), s.card(ID)), Some(DURABILITY - 1));
        }

        #[test]
        fn r346_its_printed_pierce_skips_armor_too() {
            let mut s = lit(true);
            let armored = s.card(ARMORED).clone();
            s.activate(ID, json!({ "targets": pick::instance(&armored) }));
            s.expect_stats(&armored, json!({ "health": 7 - RADIANT_DAMAGE }));
        }
    }
}
