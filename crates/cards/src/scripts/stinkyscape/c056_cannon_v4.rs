//! Stinkyscape #56 Cannon v4 (SPEC §8.10 row 56). Unit 0/6 → 0/12, Weapon, cost 3, Epic.
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

// Stinkyscape #56 Cannon v4 — SPEC §8.10 row 56, BUILD M12 row S 56: its Activate deals 7 to a chosen
// enemy Unit; the enemy hero is never offered as its target, and aiming at it is refused; once a turn,
// a second use refused; equipped onto a Unit, the host takes the Activate; damage reads through
// `param()`; radiant 0/12, 14 damage.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{hits, js, next_turn_of, weapon_board};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MENACE: &str = "core-019"; // (3) 9/9 Taunt.
    const VANILLA: &str = "core-008"; // (1) 4/4.

    /// The printed `damage`, and the Radiant face's, and its Radiant health.
    const DAMAGE: i32 = 7;
    const RADIANT_DAMAGE: i32 = 14;
    const RADIANT_HEALTH: i32 = 12;
    /// The Armor on the target: it has no Pierce, so the Armor takes its part.
    const ARMOR: i32 = 3;

    fn at(target: &str) -> Value {
        json!({ "ability": "cannon", "targets": pick::id(target) })
    }

    fn at_hero() -> Value {
        json!({ "ability": "cannon", "targets": [{ "pick": "hero", "player": "p2" }] })
    }

    fn cannon_on(radiant: bool, p2: Value) -> Scenario {
        weapon_board(
            "s056",
            json!({ "field": [{ "def": ID, "radiant": radiant }] }),
            p2,
        )
    }

    /// The targets `legal_actions` offers p1 for the Cannon's Activate, one per action.
    fn offered(s: &Scenario) -> Vec<Value> {
        let cannon = s.card(ID).id.clone();
        legal_actions(s.state(), P1)
            .iter()
            .map(js)
            .filter(|action| action["type"] == "activate" && action["instanceId"] == cannon.as_str())
            .map(|action| action["targets"].clone())
            .collect()
    }

    /// One hit on an Armored Menace between two others, the Armor taking its part, and none beside it.
    fn hits_the_unit(radiant: bool, amount: i32) {
        let mut s = cannon_on(radiant, json!({ "field": [MENACE, MENACE, MENACE] }));
        let target = s.unit_at(P2, 2).id;
        s.card_mut(target.as_str())
            .granted_keywords
            .push(Keyword::Armor { n: ARMOR });
        s.activate(ID, at(&target));
        assert_eq!(
            hits(s.last_events()),
            vec![(target, amount - ARMOR)],
            "radiant {radiant}"
        );
        s.expect_health(P2, HERO_HEALTH);
    }

    /// "an enemy Unit": the enemy Unit is the one target offered, the enemy hero never is, and aiming
    /// at the hero is refused, leaving it untouched.
    fn never_the_hero(radiant: bool) {
        let mut s = cannon_on(radiant, json!({ "field": [VANILLA] }));
        let unit = s.unit_at(P2, 1).id;
        assert_eq!(offered(&s), vec![pick::id(&unit)], "radiant {radiant}");
        s.expect_refused(|s| s.activate(ID, at_hero()));
        s.expect_health(P2, HERO_HEALTH);
    }

    mod base {
        use super::*;

        /// R767000: the script kept for the replays of v0.5.11 and before may aim at the enemy hero,
        /// whose hit is halved, rounded down (R1841001).
        #[test]
        fn r767000_the_script_kept_for_v0_5_11_halves_its_hit_on_the_enemy_hero() {
            crate::under_kept(ID, script_v0_5_11(), || {
                let halved = DAMAGE / HALF_DAMAGE_TO_HEROES_DIVISOR;
                let mut s = cannon_on(false, json!({ "field": [VANILLA] }));
                s.activate(ID, at_hero());
                assert_eq!(hits(s.last_events()), vec![("hero-p2".to_string(), halved)]);
                s.expect_health(P2, HERO_HEALTH - halved);
            });
        }

        #[test]
        fn r1862000_hits_the_chosen_enemy_unit() {
            hits_the_unit(false, DAMAGE);
        }

        #[test]
        fn r1862000_the_enemy_hero_is_never_offered_and_is_refused() {
            never_the_hero(false);
        }

        #[test]
        fn r384_once_a_turn_a_second_use_refused() {
            let mut s = cannon_on(false, json!({ "field": [MENACE, MENACE] }));
            let [first, second] = [1, 2].map(|lane| s.unit_at(P2, lane).id);
            s.activate(ID, at(&first));
            s.expect_refused_with(|s| s.activate(ID, at(&second)), "already been used");
            next_turn_of(&mut s, P1);
            s.activate(ID, at(&second));
            assert_eq!(hits(s.last_events()), vec![(second, DAMAGE)]);
        }

        #[test]
        fn r1086_r1852001_an_equipped_host_takes_the_activate_and_is_no_starship_piece() {
            let mut s = weapon_board(
                "s056-equip",
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
            let mut s = cannon_on(false, json!({ "field": [MENACE] }));
            let amount = crate::upgrade_number(&mut s, ID, "damage");
            let target = s.unit_at(P2, 1).id;
            s.activate(ID, at(&target));
            assert_eq!(hits(s.last_events()), vec![(target, amount)]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1862000_hits_the_chosen_enemy_unit_twice_as_hard() {
            hits_the_unit(true, RADIANT_DAMAGE);
        }

        #[test]
        fn r1862000_the_enemy_hero_is_never_offered_and_is_refused() {
            never_the_hero(true);
        }

        #[test]
        fn its_health_is_doubled() {
            let s = cannon_on(true, json!({}));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (0, RADIANT_HEALTH));
        }
    }
}
