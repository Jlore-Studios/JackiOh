//! SPEC §8.12 S+ #90 Meltdown (Unit 2/12 → 4/28, Mindustrial, Legendary, cost 3), a card of the Stinkyscape+ set (R1420).
//!   Base:    "Flexible, Ranged, Can't Defend, Windfury
//!            When this attacks, gain +{attack} attack and take {damage} damage." — attack 2, damage 2
//!   Radiant: "Flexible, Ranged, Windfury
//!            When this attacks, gain +{attack} attack and take {damage} damage." — attack 4, damage 2
//!
//! Engine: the keywords are catalog data (Flexible, R3543000: it attacks from the backrow as a Unit). R3559002:
//! "When this attacks" is the `whenAttacks` hook, not the brief's `afterAttack` (R2396000, R2285000): on each
//! attack, declared or forced, Meltdown first gains a lasting +`attack` attack, then takes `damage` damage from
//! itself, then strikes with the raised attack; one the damage kills does not fight (R220). Both faces share the
//! script; both numbers are the declared ones (R386), read through `param`. R4783023: a freeze heals nothing.
//!
//! Issue #4783 (Stinkyscape+ Balance Changes VIII) took "When this is Frozen, fully heal it" off both faces.
//! Through v0.6.13h (`script_v0_6_13h`, R767000) it was a trigger on the Frozen mark landing on it
//! (R3550002), answering every freeze that landed, a re-freeze included, and healing it to full.

use jackioh_engine::effects::{buff, damage, heal};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-090";

/// R3550002: the id of the Frozen trigger (through v0.6.13h).
const THAW: &str = "meltdown-thaw";

/// The face: today's when `through_v0_6_13h` is false, else the one v0.6.13h and before ran.
fn meltdown(through_v0_6_13h: bool) -> Script {
    let mut script = Script {
        // R3559002: gain first, then take the damage, both before the strike.
        when_attacks: Some(hook(|ctx| {
            vec![
                buff(json_as(json!({ "target": { "of": "self" }, "attack": param(&*ctx, "attack") }))),
                damage(json_as(json!({ "to": { "of": "self" }, "amount": param(&*ctx, "damage") }))),
            ]
        })),
        ..Script::default()
    };
    if through_v0_6_13h {
        // R3550002 (through v0.6.13h): every freeze that lands on it heals it to full.
        script.triggers = vec![
            TriggerDef::new(THAW, &[GameEventType::Marked], |_ctx, _event| {
                vec![heal(json_as(json!({ "target": { "of": "self" }, "toFull": true })))]
            })
            .with_when(|ctx, event| {
                ctx.self_
                    .as_ref()
                    .is_some_and(|card| is_frozen_event(event, card.id.as_str()))
            }),
        ];
    }
    script
}

pub fn script() -> CardScripts {
    CardScripts {
        base: meltdown(false),
        radiant: meltdown(false),
    }
}

/// R767000: what it ran through v0.6.13h, before issue #4783: "When this is Frozen, fully heal it"
/// (R3550002).
pub fn script_v0_6_13h() -> CardScripts {
    CardScripts {
        base: meltdown(true),
        radiant: meltdown(true),
    }
}

// S+ #90 Meltdown — SPEC §8.12 row 90 (R3559002, R4783023; R220, R386, R2285000, R2396000, R3543000).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon_board;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A target nothing here brings down.
    const TOUGH: &str = "stand-in (1) 0/60";
    const FREEZE_RAY: &str = "stinkyscape-001"; // Freeze Ray: freeze a Unit and the Units beside it

    /// The printed numbers, base and Radiant: attack 2 and 4, the stats, the damage it takes.
    const ATTACK: i32 = 2;
    const RADIANT_ATTACK: i32 = 4;
    const STARTING_ATTACK: i32 = 2;
    const RADIANT_STARTING_ATTACK: i32 = 4;
    const SELF_DAMAGE: i32 = 2;
    const HEALTH: i32 = 12;
    const RADIANT_HEALTH: i32 = 28;
    /// Windfury's attacks a turn.
    const ATTACKS: i32 = 2;

    /// Meltdown in p1's unit row, a TOUGH target for p2.
    fn board(radiant: bool, seed: &str) -> Scenario {
        weapon_board(
            seed,
            json!({ "field": [{ "def": ID, "radiant": radiant }], "hand": [FREEZE_RAY, FREEZE_RAY] }),
            json!({ "field": [TOUGH] }),
        )
    }

    /// The damage Meltdown of `radiant`'s face carries once Freeze Ray has frozen it with 6 on it.
    fn frozen_damage(radiant: bool, seed: &str) -> i32 {
        let mut s = board(radiant, seed);
        s.card_mut(ID).damage = 6;
        let id = s.card(ID).id.clone();
        s.play(FREEZE_RAY, json!({ "targets": pick::id(id.as_str()) }));
        assert!(s.card(ID).frozen.is_some(), "Freeze Ray froze it");
        s.card(ID).damage
    }

    mod base {
        use super::*;

        #[test]
        fn r3559002_each_attack_gains_two_then_takes_two_before_striking() {
            let mut s = board(false, "s090-base");
            let target = s.unit_at(P2, 1).id;
            // One attack: +2 attack, 2 damage taken, and the strike lands with the raised attack.
            s.attack(ID, target.as_str());
            assert_eq!(s.card(target.as_str()).damage, STARTING_ATTACK + ATTACK);
            assert_eq!(s.card(ID).damage, SELF_DAMAGE);
            assert_eq!(s.stats(ID).attack, STARTING_ATTACK + ATTACK);
            // The second (Windfury) attack does it again from where it stands.
            s.attack(ID, target.as_str());
            assert_eq!(
                s.card(target.as_str()).damage,
                (STARTING_ATTACK + ATTACK) + (STARTING_ATTACK + 2 * ATTACK)
            );
            assert_eq!(s.card(ID).damage, ATTACKS * SELF_DAMAGE);
            assert_eq!(s.stats(ID).attack, STARTING_ATTACK + ATTACKS * ATTACK);
            s.expect_refused(|s| s.attack(ID, target.as_str()));
        }

        #[test]
        fn r3559002_the_gain_is_lasting_into_the_next_turn() {
            let mut s = board(false, "s090-lasting");
            let target = s.unit_at(P2, 1).id;
            s.attack(ID, target.as_str());
            crate::next_turn_of(&mut s, P1);
            assert_eq!(s.stats(ID).attack, STARTING_ATTACK + ATTACK, "the buff stays");
        }

        #[test]
        fn r3559002_it_strikes_the_hero_the_same_way() {
            let mut s = board(false, "s090-hero");
            s.attack(ID, "hero");
            s.expect_health(P2, HERO_HEALTH - (STARTING_ATTACK + ATTACK));
            assert_eq!(s.card(ID).damage, SELF_DAMAGE);
        }

        #[test]
        fn r220_r3559002_a_meltdown_its_own_damage_kills_does_not_fight() {
            let mut s = board(false, "s090-dies");
            let target = s.unit_at(P2, 1).id;
            s.card_mut(ID).damage = HEALTH - SELF_DAMAGE;
            s.attack(ID, target.as_str());
            assert_eq!(s.card(target.as_str()).damage, 0, "it died before its strike");
            s.expect_in_zone(ID, "graveyard");
        }

        #[test]
        fn r4783023_a_freeze_heals_nothing() {
            assert_eq!(frozen_damage(false, "s090-thaw"), 6);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3559002_each_attack_gains_four_then_takes_two_before_striking() {
            let mut s = board(true, "s090-radiant");
            let target = s.unit_at(P2, 1).id;
            s.attack(ID, target.as_str());
            assert_eq!(s.card(target.as_str()).damage, RADIANT_STARTING_ATTACK + RADIANT_ATTACK);
            assert_eq!(s.card(ID).damage, SELF_DAMAGE);
            assert_eq!(s.stats(ID).attack, RADIANT_STARTING_ATTACK + RADIANT_ATTACK);
        }

        #[test]
        fn r4783023_a_freeze_heals_nothing() {
            assert_eq!(frozen_damage(true, "s090-radiant-thaw"), 6);
        }

        #[test]
        fn its_health_is_doubled_and_more() {
            let base = board(false, "s090-base-health");
            let radiant = board(true, "s090-radiant-health");
            assert_eq!(base.stats(ID).health, HEALTH);
            assert_eq!(radiant.stats(ID).health, RADIANT_HEALTH);
        }
    }

    /// R767000: the script kept for the replays of v0.6.13h and before (issue #4783).
    mod kept_v0_6_13h {
        use super::*;

        #[test]
        fn r767000_r3550002_a_freeze_healed_it_to_full_on_both_faces() {
            crate::under_kept(ID, script_v0_6_13h(), || {
                for radiant in [false, true] {
                    assert_eq!(frozen_damage(radiant, "s090-kept-thaw"), 0, "radiant {radiant}");
                }
            });
        }

        #[test]
        fn r767000_r3550002_every_freeze_that_landed_healed_it_again() {
            crate::under_kept(ID, script_v0_6_13h(), || {
                let mut s = board(false, "s090-kept-rethaw");
                let id = s.card(ID).id.clone();
                s.card_mut(ID).damage = 6;
                s.play(FREEZE_RAY, json!({ "targets": pick::id(id.as_str()) }));
                assert_eq!(s.card(ID).damage, 0);
                // Frozen already, a second Freeze Ray lands again and heals it again.
                s.card_mut(ID).damage = 5;
                s.play(FREEZE_RAY, json!({ "targets": pick::id(id.as_str()) }));
                assert_eq!(s.card(ID).damage, 0, "a re-freeze heals it too");
            });
        }
    }
}
