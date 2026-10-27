//! Stinkyscape #59 Repeater v5 (SPEC §8.10 row 59). Unit 0/8 → 0/16, Weapon, cost 4, Legendary; held (R1420).
//!   Base:    "Starship Piece, Equip
//!            Activate 6: Deal {damage} damage to an enemy, with Pierce. Deals half damage to heroes."
//!            — damage 2
//!   Radiant: the same text — damage 4
//!
//! Starship Piece and Equip are catalog keywords the engine plays: played onto one of your Units it
//! fuses into it (R1086), which then takes the Activate (R1751000) and is a piece only if it was one
//! (R1852001); played alone, its death fuses a copy into its owner's Dormant Starship (R1852000).
//! R1862000: the Activate's target is a declared enemy Unit or the enemy hero (R81), and Pierce is
//! `ignoreArmor` (R346). Activate 6 is six uses a turn (R384, R1857003), each hit its own state check;
//! a hit on a hero is halved, rounded down (R1841001). The number is the
//! declared `damage` (R386), read through `param`.

use jackioh_engine::effects::{DamageEffectArgs, DamageFlagArgs, TargetSpec, damage};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-059";

/// Activate 6 (R1857003): six uses a turn.
const USES: i32 = 6;

/// R1862000: the chosen enemy's hit, through Armor.
fn shot(ctx: &EffectContext<'_>) -> Vec<Effect> {
    vec![damage(DamageEffectArgs {
        to: TargetSpec::Chosen { index: None },
        amount: param(ctx, "damage"),
        flags: DamageFlagArgs {
            ignore_armor: Some(true),
            half_to_heroes: Some(true),
            ..DamageFlagArgs::default()
        },
    })]
}

fn repeater() -> ActivationDecl {
    ActivationDecl {
        id: "repeater".into(),
        label: "Deal damage to an enemy, with Pierce".into(),
        uses: ActivationUses::Count(USES),
        cost: None,
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "enemy", "of": ["unit", "hero"] }),
        )],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(|ctx| shot(ctx)),
    }
}

pub fn script() -> CardScripts {
    let base = Script {
        activations: vec![repeater()],
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Stinkyscape #59 Repeater v5 — SPEC §8.10 row 59, BUILD M12 row S 59: its Activate deals 2 to a
// chosen enemy through Armor (Pierce); on the enemy hero 1 (half, rounded down, R1841001); six uses a
// turn, a seventh refused; equipped onto a Unit, the host takes the Activate; damage reads through
// `param()`; radiant 0/16, 4 damage.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{hits, next_turn_of, weapon_board};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MENACE: &str = "core-019"; // (3) 9/9 Taunt.
    const VANILLA: &str = "core-008"; // (1) 4/4.

    /// The printed `damage`, and the Radiant face's, and its Radiant health.
    const DAMAGE: i32 = 2;
    const RADIANT_DAMAGE: i32 = 4;
    const RADIANT_HEALTH: i32 = 16;

    fn at(target: &str) -> Value {
        json!({ "ability": "repeater", "targets": pick::id(target) })
    }

    fn at_hero() -> Value {
        json!({ "ability": "repeater", "targets": [{ "pick": "hero", "player": "p2" }] })
    }

    fn repeater_on(radiant: bool, p2: Value) -> Scenario {
        weapon_board(
            "s059",
            json!({ "field": [{ "def": ID, "radiant": radiant }] }),
            p2,
        )
    }

    /// One hit on an Armored Menace between two others, whole, and none beside it.
    fn pierces(radiant: bool, amount: i32) {
        let mut s = repeater_on(radiant, json!({ "field": [MENACE, MENACE, MENACE] }));
        let target = s.unit_at(P2, 2).id;
        s.card_mut(target.as_str())
            .granted_keywords
            .push(Keyword::Armor { n: 3 });
        s.activate(ID, at(&target));
        assert_eq!(hits(s.last_events()), vec![(target, amount)], "radiant {radiant}");
    }

    /// R1841001: the hit on the hero is halved, rounded down; a hit halved to 0 is none.
    fn halves_on_the_hero(radiant: bool, amount: i32) {
        let halved = amount / HALF_DAMAGE_TO_HEROES_DIVISOR;
        let mut s = repeater_on(radiant, json!({}));
        s.activate(ID, at_hero());
        let expected = if halved > 0 {
            vec![("hero-p2".to_string(), halved)]
        } else {
            vec![]
        };
        assert_eq!(hits(s.last_events()), expected, "radiant {radiant}");
        s.expect_health(P2, HERO_HEALTH - halved);
    }

    mod base {
        use super::*;

        #[test]
        fn r1862000_hits_the_chosen_enemy_through_armor() {
            pierces(false, DAMAGE);
        }

        #[test]
        fn r1841001_the_enemy_hero_takes_half_rounded_down() {
            halves_on_the_hero(false, DAMAGE);
        }

        #[test]
        fn r1857003_n_uses_a_turn_and_the_next_refused() {
            let mut s = repeater_on(false, json!({}));
            for _ in 0..USES {
                s.activate(ID, at_hero());
            }
            s.expect_health(P2, HERO_HEALTH - USES * (DAMAGE / HALF_DAMAGE_TO_HEROES_DIVISOR));
            s.expect_refused_with(
                |s| s.activate(ID, at_hero()),
                &format!("that ability has been used {USES} times this turn"),
            );
            // No cooldown: its next turn it is back (R1841000).
            next_turn_of(&mut s, P1);
            s.activate(ID, at_hero());
        }

        #[test]
        fn r1086_r1852001_an_equipped_host_takes_the_activate_and_is_no_starship_piece() {
            let mut s = weapon_board(
                "s059-equip",
                json!({ "hand": [ID], "field": [VANILLA] }),
                json!({ "field": [VANILLA] }),
            );
            let host = s.unit_at(P1, 1).id;
            let target = s.unit_at(P2, 1).id;
            s.play(ID, json!({ "zone": 1, "equip": true }));
            let fused = s.card(host.as_str()).clone();
            assert!(!unit_has(s.state(), &fused, KeywordKind::StarshipPiece));
            s.activate(host.as_str(), at(&target));
            assert_eq!(hits(s.last_events()), vec![(target, DAMAGE)]);
        }

        #[test]
        fn damage_reads_through_param() {
            let mut s = repeater_on(false, json!({ "field": [VANILLA] }));
            let amount = crate::upgrade_number(&mut s, ID, "damage");
            let target = s.unit_at(P2, 1).id;
            s.activate(ID, at(&target));
            assert_eq!(hits(s.last_events()), vec![(target, amount)]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1862000_hits_the_chosen_enemy_through_armor_twice_as_hard() {
            pierces(true, RADIANT_DAMAGE);
        }

        #[test]
        fn r1841001_the_enemy_hero_takes_half_of_its_doubled_damage() {
            halves_on_the_hero(true, RADIANT_DAMAGE);
        }

        #[test]
        fn its_health_is_doubled() {
            let s = repeater_on(true, json!({}));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (0, RADIANT_HEALTH));
        }
    }
}
