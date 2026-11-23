//! Stinkyscape #59 Repeater v5 (SPEC §8.10 row 59). Unit 0/4 → 0/8, Weapon, cost 4, Legendary.
//!   Base:    "Starship Piece
//!            Activate 4: Deal {damage} damage to an enemy Unit, with Pierce." — damage 2 (3 until issue
//!            #4353)
//!   Radiant: the same text — damage 4 (6 until issue #4353)
//!
//! Starship Piece is a catalog keyword the engine plays, and Equip was one through v0.6.7k (R4285001): Equipped onto one of your Units it
//! fuses into it (R1086), which then takes the Activate (R1751000) and is a piece only if it was one
//! (R1852001); played alone, its death fuses a copy into its owner's Dormant Starship (R1852000).
//! R1862000: the Activate's target is a declared enemy Unit (R81), never a hero (#2521). Pierce is
//! `ignoreArmor` (R346). Activate 4 is four uses a turn (R384, R1857003), each hit its own state check;
//! issue #4154 cut it from four to three, which `script_v0_6_6h` keeps for the replays of v0.6.6h and
//! before (R767000), and issue #4353 put it back to four, its three kept by `script_v0_6_8b` for the
//! replays from v0.6.6i through v0.6.8b. The number is the declared `damage` (R386), read through `param`.

use jackioh_engine::effects::{DamageEffectArgs, DamageFlagArgs, TargetSpec, damage};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-059";

/// Activate 4 (R1857003): four uses a turn (issue #4353).
const USES: i32 = 4;

/// R767000: Activate 3 (R1857003): the three uses a turn it had after v0.6.6h through v0.6.8b (issue
/// #4154).
const USES_V0_6_8B: i32 = 3;

/// R767000: Activate 4 (R1857003): the four uses a turn it had through v0.6.6h.
const USES_V0_6_6H: i32 = 4;

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

fn repeater(uses: i32) -> ActivationDecl {
    ActivationDecl {
        id: "repeater".into(),
        label: "Deal damage to an enemy Unit, with Pierce".into(),
        uses: ActivationUses::Count(uses),
        cost: None,
        // "an enemy Unit": enemy Units only, never a hero.
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit"] }))],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(|ctx| shot(ctx)),
    }
}

/// Both faces' scripts, the Activate's `uses` a turn.
fn scripts(uses: i32) -> CardScripts {
    let base = Script {
        activations: vec![repeater(uses)],
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

pub fn script() -> CardScripts {
    scripts(USES)
}

/// R767000: after v0.6.6h through v0.6.8b its Activate had `USES_V0_6_8B` uses a turn; `KEPT_SCRIPTS`
/// folds those replays under it.
pub fn script_v0_6_8b() -> CardScripts {
    scripts(USES_V0_6_8B)
}

/// R767000: through v0.6.6h its Activate had `USES_V0_6_6H` uses a turn; `KEPT_SCRIPTS` folds those
/// replays under it.
pub fn script_v0_6_6h() -> CardScripts {
    scripts(USES_V0_6_6H)
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

    /// The printed `damage`, and the Radiant face's (3 and 6 until issue #4353).
    const DAMAGE: i32 = 2;
    const RADIANT_DAMAGE: i32 = 4;

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

        #[test]
        fn r767000_the_script_kept_for_v0_6_6h_has_four_uses_a_turn() {
            crate::under_kept(ID, script_v0_6_6h(), || {
                fam::rep_uses_a_turn(ID, "s059-v0-6-6e", false, USES_V0_6_6H, DAMAGE);
            });
        }

        /// R767000: after v0.6.6h through v0.6.8b it had three uses a turn (issue #4353 made it four).
        #[test]
        fn r767000_the_script_kept_for_v0_6_8b_has_three_uses_a_turn() {
            crate::under_kept(ID, script_v0_6_8b(), || {
                fam::rep_uses_a_turn(ID, "s059-v0-6-8b", false, USES_V0_6_8B, DAMAGE);
            });
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

        /// R4334000: a launched Starship holding a Radiant copy hits the chosen Unit for the Radiant
        /// `damage` and shows it; one holding a base copy, the base `damage`.
        #[test]
        fn r4334000_a_starship_holding_a_radiant_copy_uses_its_radiant_number() {
            fam::shot_ship_hits(ID, "s059", "repeater", true, RADIANT_DAMAGE);
            fam::shot_ship_hits(ID, "s059", "repeater", false, DAMAGE);
        }
    }
}
