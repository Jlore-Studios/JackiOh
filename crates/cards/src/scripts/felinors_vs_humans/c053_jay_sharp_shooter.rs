//! FvH #53 Jay Sharp Shooter (SPEC §8.14 row 53): (1) Unit, Human, Common, 1/4 → 2/8.
//!   Base:    "Activate: Pay (1) to deal {damage} Pierce damage." (3)
//!   Radiant: "Activate: Pay (1) to deal {damage} Pierce damage." (6)
//!
//! Engine: Activate, once a turn (R384), an `ActivationCost` of (1) mana: one declared target, any Unit or
//! hero on either side (R81), `damage` of the declared `damage` with `ignore_armor`, the effect's own
//! Pierce (R346); summoning sickness does not apply to an Activate (§6.2). Its (1) is a price, fixed. The
//! Radiant face differs only in its stats and its declared `damage`, so it runs the same script. Tunes:
//! damage 3 ↑ (Radiant 6).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-053";

/// "Activate" is once a turn (R384).
const USES: i32 = 1;
/// "Pay (1)": the Activate's mana price.
const PRICE: i32 = 1;

fn face() -> Script {
    let shoot = ActivationDecl {
        id: "shoot".to_string(),
        label: "Pay (1) to deal Pierce damage".to_string(),
        uses: ActivationUses::Count(USES),
        cost: Some(ActivationCost {
            mana: Some(PRICE),
            ..ActivationCost::default()
        }),
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "hero"] }))],
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(|ctx| {
            vec![damage(json_as(json!({
                "to": { "of": "chosen" },
                "amount": param(&*ctx, "damage"),
                "ignoreArmor": true,
            })))]
        }),
    };
    Script {
        activations: vec![shoot],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = face();
    // The Radiant face's 2/8 and its 6 damage are catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #53 Jay Sharp Shooter — SPEC §8.14 row 53 (R384, R81, R346).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A textless Spell each seat can always play, so no turn ends by itself (§2.5).
    const SPELL: &str = "stand-in (0) Spell";
    const ARMORED: &str = "stand-in (4) 7/7 Armor 7";
    const FRIEND: &str = "stand-in (1) 4/4";
    /// The heroes' health before any hit (§2.1).
    const HERO: i32 = 30;
    /// Its printed `damage`, base and Radiant.
    const DAMAGE: i32 = 3;
    const RADIANT_DAMAGE: i32 = 6;
    /// What Jay costs to play.
    const COST: i32 = 1;
    /// The mana p1 starts with: Jay's (1) and the Activate's (1).
    const MANA: i32 = COST + PRICE;

    /// p1 holds Jay in hand with 2 mana; p2 an armored 7/7.
    fn shooter(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, SPELL],
                "field": [FRIEND],
                "mana": MANA,
                "library": filler(6),
            },
            "p2": { "hand": [SPELL], "field": [ARMORED], "library": filler(6) },
        }))
    }

    /// Jay played into lane 2 this turn.
    fn played(radiant: bool) -> Scenario {
        let mut s = shooter(radiant);
        s.play(ID, json!({ "zone": 2 }));
        s
    }

    mod base {
        use super::*;

        #[test]
        fn r384_played_this_turn_it_pays_1_and_deals_3_to_the_chosen_hero_once_a_turn() {
            let mut s = played(false);
            s.activate(ID, json!({ "targets": pick::hero(P2) }));
            s.expect_health(P2, HERO - DAMAGE);
            s.expect_mana(P1, MANA - COST - PRICE);
            s.expect_refused(|s| s.activate(ID, json!({ "targets": pick::hero(P2) })));
        }

        #[test]
        fn r346_its_damage_pierces_armor() {
            let mut s = played(false);
            let armored = s.card(ARMORED).clone();
            s.activate(ID, json!({ "targets": pick::instance(&armored) }));
            s.expect_stats(&armored, json!({ "health": 7 - DAMAGE }));
        }

        #[test]
        fn r81_any_unit_or_hero_its_own_sides_included() {
            let mut s = played(false);
            let friend = s.card(FRIEND).clone();
            s.activate(ID, json!({ "targets": pick::instance(&friend) }));
            s.expect_stats(&friend, json!({ "health": 4 - DAMAGE }));
        }

        #[test]
        fn r384_with_no_mana_left_for_its_price_it_is_refused() {
            let mut s = played(false);
            s.state_mut().players[P1].mana.current = 0;
            s.expect_refused(|s| s.activate(ID, json!({ "targets": pick::hero(P2) })));
            s.expect_health(P2, HERO);
        }

        #[test]
        fn r386_its_damage_reads_through_param() {
            let mut s = played(false);
            let jay = s.card(ID).id.clone();
            set_param(s.card_mut(&jay), "damage", DAMAGE + 1);
            s.activate(&jay, json!({ "targets": pick::hero(P2) }));
            s.expect_health(P2, HERO - DAMAGE - 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r346_it_pays_1_and_deals_6_pierce_damage() {
            let mut s = played(true);
            let armored = s.card(ARMORED).clone();
            s.activate(ID, json!({ "targets": pick::instance(&armored) }));
            s.expect_stats(&armored, json!({ "health": 7 - RADIANT_DAMAGE }));
            s.expect_mana(P1, MANA - COST - PRICE);
        }
    }
}
