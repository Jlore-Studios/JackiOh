//! #65.1 Spikey Pillow (SPEC §8.3, §4.1, §10.4, §7; the token #65 summons).
//!
//! Base 0/2: "Cannot be in Defense Position. Aura: your units have −2 attack".
//! Radiant 0/4: "Aura: your non-Spikey-Pillow units have −2 attack" — a restated clause, so it
//! replaces the base aura while the Defense-Position ban is kept (§8 Conventions, §5.2).
//!
//! §8.3's Engine cell: "Position validator flag; aura floors attack at 0". The ban is the
//! `never_defense` flag, enforced by `reduce.rs` and `combat.rs` (§4.1, §3, R20, #48 5pek Controller).
//! §10.4 layer 5: the aura is computed on read, so the −2 goes the instant the Pillow leaves, and
//! `layers.rs` floors attack at 0. `applies` reads INSTANCE fields only, never `unit_view`, or the
//! layers would recurse. "Your units" is the controller's (§8 Conventions), and a card dormant under
//! a Stack pile is not on the field for this (R13).

use jackioh_engine::prelude::*;

pub const ID: &str = "core-065-1";

/// −2 attack to the controller's units in the unit row. `spares_own_kind` is the radiant text: every
/// unit that is not a Spikey Pillow, this one included. The test is against `ID`, never `self.def_id`:
/// a Pillow #85 fused with another card carries this text (R102) but is named "A + Spikey Pillow", and
/// its aura still spares every Spikey Pillow while draining the fused card itself.
fn attack_drain_aura(spares_own_kind: bool) -> AuraHook {
    aura_hook(move |args| {
        // "−2 attack": the declared number `drain` (R386), less being better for the Pillow's controller.
        let drain = param(&args, "drain");
        let controller = args.self_.controller;
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| {
                unit.controller == controller
                    && matches!(unit.zone, Zone::Field { row: Row::Units, .. })
                    && !(spares_own_kind && unit.def_id == ID)
            }),
            mod_: StatMod {
                attack: Some(-drain),
                ..StatMod::default()
            },
        }]
    })
}

fn never_defense() -> StaticFlags {
    StaticFlags {
        never_defense: Some(true),
        ..StaticFlags::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            static_flags: Some(never_defense()),
            aura: Some(attack_drain_aura(false)),
            ..Script::default()
        },
        radiant: Script {
            static_flags: Some(never_defense()),
            aura: Some(attack_drain_aura(true)),
            ..Script::default()
        },
    }
}

// #65.1 Spikey Pillow — SPEC §8.3, §4.1, §10.4, BUILD M4-T4 row 65.1.
//
// Must-pass: "Cannot switch to DEF; your units −2 attack floored at 0; radiant excludes other
// Pillows."
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const PILLOW: &str = "core-065-1"; // Unit, Token, 0/2 → 0/4
    const TIMMY: &str = "core-011"; // Unit, 3/3
    const FELINOR: &str = "core-t-felinor"; // Unit, 1/1 — the floor case
    const SURGERY: &str = "core-063"; // #63, +3/+3: the only way to give a Pillow attack to drain
    const FRIEND: &str = "core-062"; // #62, fills the board: a unit that arrives after the aura

    fn unit_at(s: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
        match s.unit(player, lane) {
            Some(found) => found,
            None => panic!("expected a unit in {player} lane {lane}, found none"),
        }
    }

    fn sel(card: &CardInstance) -> Value {
        json!({ "pick": "instance", "instanceId": card.id })
    }

    #[test]
    fn r386_an_upgrade_drains_1_attack_and_a_degrade_3() {
        for (upgrade, drain) in [(true, 1), (false, 3)] {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "field": [PILLOW, TIMMY] } }));
            let pillow = unit_at(&s, P1, 1);
            let moved = if upgrade {
                crate::upgrade_number(&mut s, &pillow.id, "drain")
            } else {
                crate::degrade_number(&mut s, &pillow.id, "drain")
            };
            assert_eq!(moved, drain);
            let timmy = unit_at(&s, P1, 2);
            s.expect_stats(&timmy, json!({ "attack": 3 - drain, "maxHealth": 3 }));
        }
    }

    mod spikey_pillow {
        use super::*;

        #[test]
        fn s10_4_your_units_have_2_attack() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "field": [PILLOW, TIMMY] } }));

            let timmy = unit_at(&s, P1, 2);
            s.expect_stats(&timmy, json!({ "attack": 1, "maxHealth": 3 }));
        }

        #[test]
        fn s10_4_attack_floors_at_0_and_max_health_is_untouched() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "field": [PILLOW, FELINOR] } }));

            // 1 − 2 would be −1; `layers.rs` clamps the total at 0.
            let felinor = unit_at(&s, P1, 2);
            s.expect_stats(&felinor, json!({ "attack": 0, "maxHealth": 1, "health": 1 }));
            // The Pillow's own 0 attack is likewise floored, not negative.
            let pillow = unit_at(&s, P1, 1);
            s.expect_stats(&pillow, json!({ "attack": 0, "maxHealth": 2, "health": 2 }));
        }

        #[test]
        fn s10_4_the_aura_reaches_the_controller_s_units_only() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "field": [PILLOW] }, "p2": { "field": [TIMMY] } }));

            let timmy = unit_at(&s, P2, 1);
            s.expect_stats(&timmy, json!({ "attack": 3, "maxHealth": 3 }));
        }

        #[test]
        fn s10_4_the_aura_is_computed_on_read_so_a_unit_that_arrives_later_is_drained_too() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "field": [PILLOW], "hand": [FRIEND], "mana": 4 } }));

            s.play(FRIEND, json!({}));

            for lane in [2, 3, 4, 5] {
                let token = unit_at(&s, P1, lane);
                assert_eq!(token.def_id, FELINOR);
                s.expect_stats(&token, json!({ "attack": 0, "maxHealth": 1 }));
            }
        }

        #[test]
        fn the_base_aura_drains_the_pillow_itself() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "field": [PILLOW], "hand": [SURGERY], "mana": 4 } }));
            let pillow = unit_at(&s, P1, 1);

            s.play(SURGERY, json!({ "targets": [sel(&pillow)] }));

            // 0/2 buffed to 3/5, then its own "your units" aura takes 2 off.
            s.expect_stats(&pillow, json!({ "attack": 1, "maxHealth": 5 }));
        }

        #[test]
        fn radiant_spares_spikey_pillows_itself_included() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "field": [{ "def": PILLOW, "radiant": true }], "hand": [SURGERY], "mana": 4 },
            }));
            let pillow = unit_at(&s, P1, 1);

            s.play(SURGERY, json!({ "targets": [sel(&pillow)] }));

            // 0/4 buffed to 3/7; "your non-Spikey-Pillow units" excludes it, so nothing is drained.
            s.expect_stats(&pillow, json!({ "attack": 3, "maxHealth": 7 }));
        }

        #[test]
        fn radiant_excludes_another_spikey_pillow() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "field": [{ "def": PILLOW, "radiant": true }, { "def": PILLOW, "radiant": true }],
                    "hand": [SURGERY],
                    "mana": 4,
                },
            }));
            let second = unit_at(&s, P1, 2);

            s.play(SURGERY, json!({ "targets": [sel(&second)] }));

            s.expect_stats(&second, json!({ "attack": 3, "maxHealth": 7 }));
        }

        #[test]
        fn the_base_aura_does_not_exclude_another_spikey_pillow() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "field": [PILLOW, { "def": PILLOW, "radiant": true }], "hand": [SURGERY], "mana": 4 },
            }));
            let second = unit_at(&s, P1, 2);

            s.play(SURGERY, json!({ "targets": [sel(&second)] }));

            // Lane 1 is on its base face — "your units" — so the buffed radiant Pillow loses 2.
            s.expect_stats(&second, json!({ "attack": 1, "maxHealth": 7 }));
        }

        #[test]
        fn radiant_still_drains_your_other_units() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "field": [{ "def": PILLOW, "radiant": true }, TIMMY] } }));

            let timmy = unit_at(&s, P1, 2);
            s.expect_stats(&timmy, json!({ "attack": 1, "maxHealth": 3 }));
        }

        #[test]
        fn s4_1_it_can_never_switch_to_defense_position() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "field": [PILLOW, TIMMY] } }));
            let pillow = unit_at(&s, P1, 1);

            s.expect_refused_with(|s| s.switch_position(&pillow), "Defense Position");
            assert_eq!(s.card(&pillow).position.unwrap_or(Position::Atk), Position::Atk);

            // The flag is the Pillow's alone: an ordinary unit beside it still switches.
            let timmy = unit_at(&s, P1, 2);
            s.switch_position(&timmy);
            let timmy = unit_at(&s, P1, 2);
            assert_eq!(s.card(&timmy).position, Some(Position::Def));
        }

        #[test]
        fn s4_1_the_radiant_face_cannot_switch_to_defense_either() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "field": [{ "def": PILLOW, "radiant": true }] } }));
            let pillow = unit_at(&s, P1, 1);

            s.expect_refused_with(|s| s.switch_position(&pillow), "Defense Position");
            s.expect_stats(&pillow, json!({ "attack": 0, "maxHealth": 4 }));
        }
    }
}
