//! Stinkyscape #50 Beam v3 (SPEC §8.10 row 50). Unit 0/2 → 0/4, Weapon, cost 2, Rare.
//!   Base:    "Starship Piece, Equip
//!            Activate: Deal {damage} damage to an enemy Unit, with Cleave and Pierce. Has an extra turn
//!            cooldown." — damage 5
//!   Radiant: the same text — damage 10
//!
//! Starship Piece and Equip are catalog keywords the engine plays: played onto one of your Units it
//! fuses into it (R1086), which then takes the Activate (R1751000) and is a piece only if it was one
//! (R1852001); played alone, its death fuses a copy into its owner's Dormant Starship (R1852000).
//! R1862000: the Activate's target is a declared enemy Unit (R81), never a hero (#2521). Cleave is the
//! same hit on each Unit beside the target, read before the first hit; Pierce is `ignoreArmor` (R346).
//! Once a turn (R384), with an extra turn cooldown (R1841000); the number is the declared `damage`
//! (R386), read through `param`. Through v0.5.11 the enemy hero was a target too, its hit halved
//! (R1841001), which `script_v0_5_11` keeps (R767000).

use jackioh_engine::effects::{
    BoardScope, DamageEffectArgs, DamageFlagArgs, TargetSpec, adjacent_to_aimed, damage,
};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-050";

/// R1862000: the chosen enemy Unit's hit, then the same hit on each Unit beside it, read before the
/// first.
fn shot(ctx: &EffectContext<'_>) -> Vec<Effect> {
    let amount = param(ctx, "damage");
    let hit = |to: TargetSpec| {
        damage(DamageEffectArgs {
            to,
            amount,
            flags: DamageFlagArgs {
                ignore_armor: Some(true),
                ..DamageFlagArgs::default()
            },
        })
    };
    let chosen = TargetSpec::Chosen { index: None };
    let beside = adjacent_to_aimed(ctx, &chosen, &BoardScope::default(), TargetAim::Harm);
    let mut hits = vec![hit(chosen)];
    hits.extend(
        beside
            .into_iter()
            .map(|unit| hit(TargetSpec::Instance { instance_id: unit.id })),
    );
    hits
}

fn beam() -> ActivationDecl {
    ActivationDecl {
        id: "beam".into(),
        label: "Deal damage to an enemy Unit, with Cleave and Pierce".into(),
        uses: ActivationUses::Count(1),
        cost: Some(ActivationCost {
            extra_turn_cooldown: Some(true),
            ..ActivationCost::default()
        }),
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
        activations: vec![beam()],
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

/// R767000: the chosen enemy's hit, then the same hit on each Unit beside it, read before the first,
/// as v0.5.11 ran it: the enemy hero a target too, its hit halved, rounded down (R1841001).
fn shot_v0_5_11(ctx: &EffectContext<'_>) -> Vec<Effect> {
    let amount = param(ctx, "damage");
    let hit = |to: TargetSpec| {
        damage(DamageEffectArgs {
            to,
            amount,
            flags: DamageFlagArgs {
                ignore_armor: Some(true),
                half_to_heroes: Some(true),
                ..DamageFlagArgs::default()
            },
        })
    };
    let chosen = TargetSpec::Chosen { index: None };
    let beside = adjacent_to_aimed(ctx, &chosen, &BoardScope::default(), TargetAim::Harm);
    let mut hits = vec![hit(chosen)];
    hits.extend(
        beside
            .into_iter()
            .map(|unit| hit(TargetSpec::Instance { instance_id: unit.id })),
    );
    hits
}

/// R767000: the Activate as v0.5.11 ran it, aimed at an enemy Unit or the enemy hero.
fn beam_v0_5_11() -> ActivationDecl {
    ActivationDecl {
        id: "beam".into(),
        label: "Deal damage to an enemy, with Cleave and Pierce".into(),
        uses: ActivationUses::Count(1),
        cost: Some(ActivationCost {
            extra_turn_cooldown: Some(true),
            ..ActivationCost::default()
        }),
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
        activations: vec![beam_v0_5_11()],
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Stinkyscape #50 Beam v3 — SPEC §8.10 row 50, BUILD M12 row S 50 (R1841000, R78).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::stinkyscape_family as fam;
    use crate::{built_starship, launch_starship, weapon_board};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    /// The printed `damage`, and the Radiant face's.
    const DAMAGE: i32 = 5;
    const RADIANT_DAMAGE: i32 = 10;
    const VANILLA: &str = "core-008"; // (1) 4/4.

    /// R3291000: an ability's id, and the card and face its `printed_on` names, if any.
    type Sourced = (String, Option<(String, Option<bool>)>);

    /// R3291000: each ability p1's view lists on `id`, with the card (and face) its `printed_on` names.
    fn sources(s: &Scenario, id: &str) -> Vec<Sourced> {
        jackioh_engine::subsystems::activate::activation_views_for(s.state(), P1, s.card(id))
            .expect("the card lists its abilities")
            .into_iter()
            .map(|view| {
                (
                    view.ability,
                    view.printed_on.map(|card| (card.def_id, card.radiant)),
                )
            })
            .collect()
    }

    /// A Vanilla of p1's with this card Equipped (R1086), and what its abilities name.
    fn equipped_sources() -> Vec<Sourced> {
        let mut s = weapon_board(
            "s050-r3291000-equip",
            json!({ "hand": [ID], "field": [VANILLA] }),
            json!({ "field": [VANILLA] }),
        );
        let host = s.unit_at(P1, 1).id;
        s.play(ID, json!({ "zone": 1, "equip": true }));
        sources(&s, &host)
    }

    mod base {
        use super::*;

        #[test]
        fn r767000_the_script_kept_for_v0_5_11_halves_its_hit_on_the_enemy_hero() {
            fam::beam_kept_halves_hero(ID, "s050", DAMAGE, script_v0_5_11);
        }

        #[test]
        fn r1862000_hits_the_chosen_unit_and_each_unit_beside_it_through_armor() {
            fam::beam_cleaves(ID, "s050", false, DAMAGE);
        }

        #[test]
        fn r1862000_its_target_is_an_enemy_never_a_unit_of_yours() {
            fam::beam_own_unit_refused(ID, "s050");
        }

        #[test]
        fn r1862000_the_enemy_hero_is_never_offered_and_is_refused() {
            fam::beam_never_hero(ID, "s050", false);
        }

        #[test]
        fn r1841000_once_a_turn_then_cooling_down_then_back() {
            fam::beam_cooldown_cycle(ID, "s050", DAMAGE);
        }

        #[test]
        fn r1841000_r78_a_bounce_clears_the_cooldown() {
            fam::beam_bounce_clears(ID, "s050", DAMAGE);
        }

        #[test]
        fn r1086_r1852001_an_equipped_host_takes_the_activate_and_is_no_starship_piece() {
            fam::beam_equip_takes_activate(ID, "s050", DAMAGE);
        }

        /// R3291000: the Activate picker wears the art of the card that prints the ability, so a
        /// launched Starship's beam names this card, the piece whose copy fused in.
        #[test]
        fn r3291000_a_launched_starship_names_beam_v3_on_its_beam() {
            let (mut s, ship) = built_starship(
                "s050-r3291000-ship",
                json!([{ "def": ID, "lane": 2 }]),
                json!({}),
                json!({ "field": [VANILLA] }),
            );
            launch_starship(&mut s, &ship);
            assert_eq!(
                sources(&s, &ship),
                vec![("beam".to_string(), Some((ID.to_string(), None)))]
            );
        }

        /// R3291000: so does a Unit it is Equipped to, whose own art its other abilities would wear.
        #[test]
        fn r3291000_an_equipped_host_names_beam_v3_on_its_beam() {
            assert_eq!(
                equipped_sources(),
                vec![("beam".to_string(), Some((ID.to_string(), None)))]
            );
        }

        #[test]
        fn damage_reads_through_param() {
            fam::beam_param_reads(ID, "s050");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1862000_hits_the_chosen_unit_and_each_unit_beside_it_through_armor_twice_as_hard() {
            fam::beam_cleaves(ID, "s050", true, RADIANT_DAMAGE);
        }

        #[test]
        fn r1862000_the_enemy_hero_is_never_offered_and_is_refused() {
            fam::beam_never_hero(ID, "s050", true);
        }
    }
}
