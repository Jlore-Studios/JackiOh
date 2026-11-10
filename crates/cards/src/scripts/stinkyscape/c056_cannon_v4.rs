//! Stinkyscape #56 Cannon v4 (SPEC §8.10 row 56). Unit 0/3 → 0/6, Weapon, cost 3, Epic.
//!   Base:    "Starship Piece, Equip
//!            Activate: Deal {damage} damage to an enemy Unit." — damage 7
//!   Radiant: the same text — damage 14
//!
//! Starship Piece and Equip are catalog keywords the engine plays: played onto one of your Units it
//! fuses into it (R1086), which then takes the Activate (R1751000) and is a piece only if it was one
//! (R1852001); played alone, its death fuses a copy into its owner's Dormant Starship (R1852000).
//! R1862000: the Activate's target is a declared enemy Unit (R81), never a hero: since #2521 the enemy
//! hero is no legal target, and the old "deals half damage to heroes" is gone. Once a turn (R384). The
//! number is the declared `damage` (R386), read through `param`.
//! Through v0.5.11 the enemy hero was a target too, its hit halved (R1841001), which `script_v0_5_11`
//! keeps (R767000).

use jackioh_engine::effects::{DamageEffectArgs, DamageFlagArgs, TargetSpec, damage};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-056";

/// R1862000: the chosen enemy Unit's hit.
fn shot(ctx: &EffectContext<'_>) -> Vec<Effect> {
    vec![damage(DamageEffectArgs {
        to: TargetSpec::Chosen { index: None },
        amount: param(ctx, "damage"),
        flags: DamageFlagArgs::default(),
    })]
}

fn cannon() -> ActivationDecl {
    ActivationDecl {
        id: "cannon".into(),
        label: "Deal damage to an enemy Unit".into(),
        uses: ActivationUses::Count(1),
        cost: None,
        // "an enemy Unit": enemy Units only, never a hero.
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit"] }))],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(|ctx| shot(ctx)),
    }
}

pub fn script() -> CardScripts {
    let base = Script {
        activations: vec![cannon()],
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

/// R767000: the chosen enemy's hit as v0.5.11 ran it: the enemy hero a target too, its hit halved,
/// rounded down (R1841001).
fn shot_v0_5_11(ctx: &EffectContext<'_>) -> Vec<Effect> {
    vec![damage(DamageEffectArgs {
        to: TargetSpec::Chosen { index: None },
        amount: param(ctx, "damage"),
        flags: DamageFlagArgs {
            half_to_heroes: Some(true),
            ..DamageFlagArgs::default()
        },
    })]
}

/// R767000: the Activate as v0.5.11 ran it, aimed at an enemy Unit or the enemy hero.
fn cannon_v0_5_11() -> ActivationDecl {
    ActivationDecl {
        id: "cannon".into(),
        label: "Deal damage to an enemy".into(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "enemy", "of": ["unit", "hero"] }),
        )],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(|ctx| shot_v0_5_11(ctx)),
    }
}

/// R767000: through v0.5.11 its Activate could aim at the enemy hero, for half; `KEPT_SCRIPTS` folds
/// those replays under it.
pub fn script_v0_5_11() -> CardScripts {
    let base = Script {
        activations: vec![cannon_v0_5_11()],
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Stinkyscape #56 Cannon v4 — SPEC §8.10 row 56, BUILD M12 row S 56.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::stinkyscape_family as fam;

    /// The printed `damage`, and the Radiant face's.
    const DAMAGE: i32 = 7;
    const RADIANT_DAMAGE: i32 = 14;

    mod base {
        use super::*;

        #[test]
        fn r767000_the_script_kept_for_v0_5_11_halves_its_hit_on_the_enemy_hero() {
            fam::can_kept_halves_hero(ID, "s056", DAMAGE, script_v0_5_11);
        }

        #[test]
        fn r1862000_hits_the_chosen_enemy_unit() {
            fam::can_hits(ID, "s056", false, DAMAGE);
        }

        #[test]
        fn r1862000_the_enemy_hero_is_never_offered_and_is_refused() {
            fam::can_never_hero(ID, "s056", false);
        }

        #[test]
        fn r384_once_a_turn_a_second_use_refused() {
            fam::can_once_a_turn(ID, "s056", DAMAGE);
        }

        #[test]
        fn r1086_r1852001_an_equipped_host_takes_the_activate_and_is_no_starship_piece() {
            fam::can_equip_takes_activate(ID, "s056", DAMAGE);
        }

        #[test]
        fn damage_reads_through_param() {
            fam::can_param_reads(ID, "s056");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1862000_hits_the_chosen_enemy_unit_twice_as_hard() {
            fam::can_hits(ID, "s056", true, RADIANT_DAMAGE);
        }

        #[test]
        fn r1862000_the_enemy_hero_is_never_offered_and_is_refused() {
            fam::can_never_hero(ID, "s056", true);
        }
    }
}
