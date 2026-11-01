//! Stinkyscape #55 Repeater v4 (SPEC §8.10 row 55). Unit 0/6 → 0/12, Weapon, cost 3, Epic.
//!   Base:    "Starship Piece, Equip
//!            Activate 3: Deal {damage} damage to an enemy Unit, with Pierce." — damage 2
//!   Radiant: the same text — damage 4
//!
//! Starship Piece and Equip are catalog keywords the engine plays: played onto one of your Units it
//! fuses into it (R1086), which then takes the Activate (R1751000) and is a piece only if it was one
//! (R1852001); played alone, its death fuses a copy into its owner's Dormant Starship (R1852000).
//! R1862000: the Activate's target is a declared enemy Unit (R81), never a hero (#2521). Pierce is
//! `ignoreArmor` (R346). Activate 3 is three uses a turn (R384, R1857003), each hit its own state check.
//! The number is the declared `damage` (R386), read through `param`.

use jackioh_engine::effects::{DamageEffectArgs, DamageFlagArgs, TargetSpec, damage};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-055";

/// Activate 3 (R1857003): three uses a turn.
const USES: i32 = 3;

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
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit"] }))],
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

// Stinkyscape #55 Repeater v4 — SPEC §8.10 row 55, BUILD M12 row S 55: its Activate deals 2 through
// Armor (Pierce) to a chosen enemy Unit, never the enemy hero; three uses a turn; radiant 0/12, 4 damage.
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
    const DAMAGE: i32 = 2;
    const RADIANT_DAMAGE: i32 = 4;
    const RADIANT_HEALTH: i32 = 12;

    fn at(target: &str) -> Value {
        json!({ "ability": "repeater", "targets": pick::id(target) })
    }

    fn at_hero() -> Value {
        json!({ "ability": "repeater", "targets": [{ "pick": "hero", "player": "p2" }] })
    }

    fn repeater_on(radiant: bool, p2: Value) -> Scenario {
        weapon_board(
            "s055",
            json!({ "field": [{ "def": ID, "radiant": radiant }] }),
            p2,
        )
    }

    /// The targets `legal_actions` offers p1 for the Repeater's Activate, one per action.
    fn offered(s: &Scenario) -> Vec<Value> {
        let repeater = s.card(ID).id.clone();
        legal_actions(s.state(), P1)
            .iter()
            .map(js)
            .filter(|action| action["type"] == "activate" && action["instanceId"] == repeater.as_str())
            .map(|action| action["targets"].clone())
            .collect()
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
        s.expect_health(P2, HERO_HEALTH);
    }

    /// "an enemy Unit": the enemy Unit is the one target offered, the enemy hero never is, and aiming
    /// at the hero is refused, leaving it untouched.
    fn never_the_hero(radiant: bool) {
        let mut s = repeater_on(radiant, json!({ "field": [VANILLA] }));
        let unit = s.unit_at(P2, 1).id;
        assert_eq!(offered(&s), vec![pick::id(&unit)], "radiant {radiant}");
        s.expect_refused(|s| s.activate(ID, at_hero()));
        s.expect_health(P2, HERO_HEALTH);
    }

    /// R1857003: USES uses a turn, one per Vanilla in lanes 1 to USES, the last of them landing;
    /// the next, at the Vanilla in lane USES + 1, is refused; the turn after, it is back (no cooldown).
    fn uses_a_turn(radiant: bool, amount: i32) {
        let row = json!({ "field": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA] });
        let mut s = repeater_on(radiant, row);
        for lane in 1..USES {
            let target = s.unit_at(P2, lane).id;
            s.activate(ID, at(&target));
        }
        let last = s.unit_at(P2, USES).id;
        s.activate(ID, at(&last));
        assert_eq!(hits(s.last_events()), vec![(last, amount)], "use {USES} lands");
        let next = s.unit_at(P2, USES + 1).id;
        s.expect_refused_with(
            |s| s.activate(ID, at(&next)),
            &format!("that ability has been used {USES} times this turn"),
        );
        next_turn_of(&mut s, P1);
        s.activate(ID, at(&next));
        assert_eq!(hits(s.last_events()), vec![(next, amount)], "radiant {radiant}");
    }

    mod base {
        use super::*;

        #[test]
        fn r1862000_hits_the_chosen_enemy_unit_through_armor() {
            pierces(false, DAMAGE);
        }

        #[test]
        fn r1862000_the_enemy_hero_is_never_offered_and_is_refused() {
            never_the_hero(false);
        }

        #[test]
        fn r1857003_n_uses_a_turn_the_last_lands_and_the_next_is_refused() {
            uses_a_turn(false, DAMAGE);
        }

        #[test]
        fn r1086_r1852001_an_equipped_host_takes_the_activate_and_is_no_starship_piece() {
            let mut s = weapon_board(
                "s055-equip",
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
        fn r767000_the_script_kept_for_v0_5_11_hits_the_enemy_hero_for_half_its_old_uses_a_turn() {
            crate::under_kept(ID, script_v0_5_11(), || {
                let halved = RADIANT_DAMAGE / HALF_DAMAGE_TO_HEROES_DIVISOR;
                let mut s = repeater_on(true, json!({ "field": [VANILLA] }));
                for _ in 0..USES_V0_5_11 {
                    s.activate(ID, at_hero());
                    assert_eq!(hits(s.last_events()), vec![("hero-p2".to_string(), halved)]);
                }
                s.expect_health(P2, HERO_HEALTH - USES_V0_5_11 * halved);
                s.expect_refused(|s| s.activate(ID, at_hero()));
            });
        }

        #[test]
        fn r1862000_hits_the_chosen_enemy_unit_through_armor_twice_as_hard() {
            pierces(true, RADIANT_DAMAGE);
        }

        #[test]
        fn r1862000_the_enemy_hero_is_never_offered_and_is_refused() {
            never_the_hero(true);
        }

        #[test]
        fn r1857003_n_uses_a_turn_the_last_lands_and_the_next_is_refused() {
            uses_a_turn(true, RADIANT_DAMAGE);
        }

        #[test]
        fn its_health_is_doubled() {
            let s = repeater_on(true, json!({}));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (0, RADIANT_HEALTH));
        }
    }
}
