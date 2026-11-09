//! Stinkyscape #59 Repeater v5 (SPEC §8.10 row 59). Unit 0/4 → 0/8, Weapon, cost 4, Legendary.
//!   Base:    "Starship Piece, Equip
//!            Activate 4: Deal {damage} damage to an enemy Unit, with Pierce." — damage 3
//!   Radiant: the same text — damage 6
//!
//! Starship Piece and Equip are catalog keywords the engine plays: played onto one of your Units it
//! fuses into it (R1086), which then takes the Activate (R1751000) and is a piece only if it was one
//! (R1852001); played alone, its death fuses a copy into its owner's Dormant Starship (R1852000).
//! R1862000: the Activate's target is a declared enemy Unit (R81), never a hero (#2521). Pierce is
//! `ignoreArmor` (R346). Activate 4 is four uses a turn (R384, R1857003), each hit its own state check.
//! The number is the declared `damage` (R386), read through `param`.

use jackioh_engine::effects::{DamageEffectArgs, DamageFlagArgs, TargetSpec, damage};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-059";

/// Activate 4 (R1857003): four uses a turn.
const USES: i32 = 4;

/// R767000: Activate 6 (R1857003): the six uses a turn it had through v0.5.11.
const USES_V0_5_11: i32 = 6;

/// R1862000: the chosen enemy Unit's hit, through Armor.
fn shot(ctx: &EffectContext<'_>) -> Vec<Effect> {
    vec![damage(DamageEffectArgs {
        to: TargetSpec::Chosen { index: None },
        amount: param(ctx, "damage"),
        flags: DamageFlagArgs {
            ignore_armor: Some(true),
            ..DamageFlagArgs::default()
        },
    })]
}

fn repeater() -> ActivationDecl {
    ActivationDecl {
        id: "repeater".into(),
        label: "Deal damage to an enemy Unit, with Pierce".into(),
        uses: ActivationUses::Count(USES),
        cost: None,
        // "an enemy Unit": enemy Units only, never a hero.
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "enemy", "of": ["unit"] }),
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

/// R767000: the chosen enemy's hit, through Armor, as v0.5.11 ran it: the enemy hero a target too,
/// its hit halved, rounded down (R1841001).
fn shot_v0_5_11(ctx: &EffectContext<'_>) -> Vec<Effect> {
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

/// R767000: the Activate as v0.5.11 ran it.
fn repeater_v0_5_11() -> ActivationDecl {
    ActivationDecl {
        id: "repeater".into(),
        label: "Deal damage to an enemy, with Pierce".into(),
        uses: ActivationUses::Count(USES_V0_5_11),
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

/// R767000: through v0.5.11 its Activate could aim at the enemy hero, for half, `USES_V0_5_11` times
/// a turn; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_11() -> CardScripts {
    let base = Script {
        activations: vec![repeater_v0_5_11()],
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Stinkyscape #59 Repeater v5 — SPEC §8.10 row 59, BUILD M12 row S 59.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::stinkyscape_family as fam;

    /// The printed `damage`, and the Radiant face's.
    const DAMAGE: i32 = 3;
    const RADIANT_DAMAGE: i32 = 6;

    mod base {
        use super::*;

        #[test]
        fn r1862000_hits_the_chosen_enemy_unit_through_armor() {
            fam::rep_pierces(ID, "s059", false, DAMAGE);
        }

        #[test]
        fn r1862000_the_enemy_hero_is_never_offered_and_is_refused() {
            fam::rep_never_hero(ID, "s059", false);
        }

        #[test]
        fn r1857003_n_uses_a_turn_the_last_lands_and_the_next_is_refused() {
            fam::rep_uses_a_turn(ID, "s059", false, USES, DAMAGE);
        }

        #[test]
        fn r1086_r1852001_an_equipped_host_takes_the_activate_and_is_no_starship_piece() {
            fam::rep_equip_takes_activate(ID, "s059", DAMAGE);
        }

        #[test]
        fn damage_reads_through_param() {
            fam::rep_param_reads(ID, "s059");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r767000_the_script_kept_for_v0_5_11_hits_the_enemy_hero_for_half_its_old_uses_a_turn() {
            fam::rep_kept_halves_hero(ID, "s059", RADIANT_DAMAGE, USES_V0_5_11, script_v0_5_11);
        }

        #[test]
        fn r1862000_hits_the_chosen_enemy_unit_through_armor_twice_as_hard() {
            fam::rep_pierces(ID, "s059", true, RADIANT_DAMAGE);
        }

        #[test]
        fn r1862000_the_enemy_hero_is_never_offered_and_is_refused() {
            fam::rep_never_hero(ID, "s059", true);
        }

        #[test]
        fn r1857003_n_uses_a_turn_the_last_lands_and_the_next_is_refused() {
            fam::rep_uses_a_turn(ID, "s059", true, USES, RADIANT_DAMAGE);
        }
    }
}
