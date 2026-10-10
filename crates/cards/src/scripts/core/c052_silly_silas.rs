//! #52 Silly Silas (SPEC §8.3, §3.1's rotation-topology ruling, §3.2, §6.3 Rotate; R4, R11, R12,
//! R14, R33, R78, R81, R88). Unit 4/4 → 8/8, Human, cost 3, Legendary. BUILD wave 3 (rotation).
//!   Base:    "Cry: choose left or right; rotate every card on the field one step around its ring
//!            (3.1); cards crossing sides change control"
//!   Radiant: "Cards that would move to the opponent are bounced to their controller's hand costing 0
//!            instead" — §8 Conventions: the cell restates only what happens to a crossing card, so
//!            the direction choice and the rotation itself are kept.
//!
//! The direction is a play-time choice, not a prompt: R81 names this card ("a declared `direction`
//! pick [travels] in `modes`"), and §10.6 says no Core card opens a `direction` prompt. So the script
//! declares `modes`, `legal_actions` enumerates both, and the answer arrives in `ctx.modes`, which
//! `chosen_options(ctx)` reads like a prompt-mode answer.
//! The rotation is `subsystems/rotation.rs`'s (R14), which the effects barrel keeps out so a card stays
//! pure (CLAUDE.md rule 5); the `rotate` verb calls it with the controller's perspective and the
//! running face's `radiant`, so both faces share one hook. What it already does:
//!   * both rings turn together, read whole before anything is placed (§3.1); a Stack travels whole
//!     (§3.2, R13); a card never leaves the field, so R78's reset never runs and damage travels (R14);
//!   * a crossing card enters its new side summoning sick (R171); `controller` changes, `owner` never
//!     (R12), and a face-down trap is read by its new controller (R33);
//!   * a Locked or Reborn-reserved destination bounces the card to its controller's hand (R14, R88,
//!     R747): the hand cap applies (R4) and a unit token ceases to exist (R11);
//!   * radiant turns an OUTBOUND crossing into that bounce at `cost_override: 0` (R65); cards crossing
//!     onto this side still cross and change control (R14, R171).

use jackioh_engine::prelude::*;

pub const ID: &str = "core-052";

/// R81: the declared direction, enumerated by `legal_actions` and answered in the `play` action.
const DIRECTIONS: [&str; 2] = ["left", "right"];

fn direction_decl() -> ModeDecl {
    ModeDecl {
        kind: PromptKind::Direction,
        options: DIRECTIONS.iter().map(|direction| direction.to_string()).collect(),
    }
}

/// The declared pick, narrowed rather than cast. `chosen_options(ctx)` reads a prompt answer's mode
/// first and `ctx.modes` second, so one reader covers both an ordinary play and an Echo repeat of
/// this Cry. R90 keeps the play legal with the answers that exist, so a play that carried no
/// direction fizzles here instead of guessing a side for the whole board.
fn direction_of(ctx: &EffectContext<'_>) -> Option<&'static str> {
    let chosen = chosen_options(ctx);
    let picked = chosen.first().map(String::as_str);
    DIRECTIONS.iter().copied().find(|direction| Some(*direction) == picked)
}

/// §8: one step around each ring, in the direction the play declared.
fn silas(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    match direction_of(ctx) {
        None => vec![],
        Some(direction) => vec![rotate(json_as(json!({ "direction": direction })))],
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            modes: vec![direction_decl()],
            cry: Some(hook(silas)),
            ..Script::default()
        },
        // The same declaration and the same hook: the radiant difference is `ctx.radiant`, which the
        // rotation subsystem reads for itself (R14's "radiant bounces go to the card's controller's
        // hand", R747).
        radiant: Script {
            modes: vec![direction_decl()],
            cry: Some(hook(silas)),
            ..Script::default()
        },
    }
}

// #52 Silly Silas (SPEC §8.3, §3.1's rotation-topology ruling, §3.2, §6.3 Rotate; R4, R11, R12,
// R14, R33, R78, R81, R88). BUILD M4-T4 row 52: "Rotate both rings either direction, control changes
// on crossing, damage travels, Silas moves too; Locked destination bounces; radiant bounces crossing
// cards to their owner's hand at cost 0 (R14)". "Right" is one step forward on p1's ring (§3.1).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    fn unit_at(s: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
        s.unit(player, lane)
            .unwrap_or_else(|| panic!("no unit in {}'s lane {lane}", player.as_str()))
            .clone()
    }

    fn backrow_at(s: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
        s.backrow(player, lane)
            .unwrap_or_else(|| panic!("no backrow card in {}'s lane {lane}", player.as_str()))
            .clone()
    }

    /// Where a card sits now, as the one thing a rotation test is about.
    fn slot_of(card: &CardInstance) -> String {
        match &card.zone {
            Zone::Field { player, row, lane } => format!("{} {} {}", player.as_str(), row.as_str(), lane),
            other => other.z().as_str().to_string(),
        }
    }

    /// Locks a unit zone with the engine's own `lock_zone`: `SideSetup` has no `locks` option.
    fn lock_unit_zone(s: &mut Scenario, player: PlayerId, lane: i32) {
        lock_zone(
            s.state_mut(),
            ZoneSlot {
                player,
                row: Row::Units,
                lane,
            },
        );
    }

    mod n52_silly_silas_base_rotating_right {
        use super::*;

        #[test]
        fn r14_turns_both_rings_one_step_and_silas_rotates_with_them() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": ["core-052"],
                    "field": [{ "def": "core-053", "lane": 5, "damage": 2, "position": "DEF" }],
                    "backrow": [{ "def": "core-058", "lane": 1 }],
                },
                "p2": {
                    "field": [{ "def": "core-t-felinor", "lane": 1 }],
                    "backrow": [{ "def": "core-071", "lane": 1 }],
                },
            }));
            let reno = unit_at(&s, P1, 5).id;
            let felinor = unit_at(&s, P2, 1).id;
            let farm = backrow_at(&s, P1, 1).id;
            let stimmy = backrow_at(&s, P2, 1).id;

            s.play("core-052", json!({ "zone": 1, "modes": ["right"] }));

            // The Engine cell's "Silas rotates too": he is on the field for his own Cry (§10.5 step 4).
            assert_eq!(unit_at(&s, P1, 2).def_id, "core-052");
            // The unit ring: p1 lane 5 → p2 lane 5, p2 lane 1 → p1 lane 1.
            assert_eq!(slot_of(s.card(&reno)), "p2 units 5");
            assert_eq!(slot_of(s.card(&felinor)), "p1 units 1");
            // The backrow ring turns with it: p1 lane 1 → p1 lane 2, p2 lane 1 → p1 lane 1.
            assert_eq!(slot_of(s.card(&farm)), "p1 backrow 2");
            assert_eq!(slot_of(s.card(&stimmy)), "p1 backrow 1");
            s.expect_events(json!(["cardPlayed", "rotated", "controlChanged"]));
        }

        #[test]
        fn r12_a_card_that_crosses_the_centre_line_changes_controller_and_never_owner() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-052"], "field": [{ "def": "core-053", "lane": 5 }] },
                "p2": { "field": [{ "def": "core-t-felinor", "lane": 1 }] },
            }));
            let reno = unit_at(&s, P1, 5).id;
            let felinor = unit_at(&s, P2, 1).id;

            s.play("core-052", json!({ "zone": 1, "modes": ["right"] }));

            assert_eq!(s.card(&reno).controller, P2);
            assert_eq!(s.card(&reno).owner, P1);
            assert_eq!(s.card(&felinor).controller, P1);
            assert_eq!(s.card(&felinor).owner, P2);
            // Silas did not cross, so nothing about him changed but the lane.
            assert_eq!(s.card("core-052").controller, P1);
        }

        #[test]
        fn r14_r78_damage_and_every_other_part_of_the_instance_travel_with_the_card() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-052"], "field": [{ "def": "core-053", "lane": 5, "damage": 2, "position": "DEF" }] },
            }));
            let reno = unit_at(&s, P1, 5).id;

            s.play("core-052", json!({ "zone": 1, "modes": ["right"] }));

            // A rotation never takes a card off the field, so R78's reset never runs: 4/6 with 2 damage.
            s.expect_stats(&reno, json!({ "attack": 4, "health": 4, "maxHealth": 6 }));
            assert_eq!(s.card(&reno).damage, 2);
            assert_eq!(s.card(&reno).position, Some(Position::Def));
            assert_eq!(slot_of(s.card(&reno)), "p2 units 5");
        }

        #[test]
        fn r33_a_face_down_trap_that_crosses_is_read_by_its_new_controller_alone() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-052"] },
                "p2": { "backrow": [{ "def": "core-071", "lane": 1, "faceUp": false }] },
            }));
            let stimmy = backrow_at(&s, P2, 1).id;

            s.play("core-052", json!({ "zone": 1, "modes": ["right"] }));

            // Identity follows `controller` and nothing else, so the flip is not a flip: `face_up` is
            // untouched and p1 now sees the card because p1 controls it.
            assert_eq!(s.card(&stimmy).controller, P1);
            assert_eq!(s.card(&stimmy).owner, P2);
            assert_ne!(s.card(&stimmy).face_up, Some(true));
            assert_eq!(slot_of(s.card(&stimmy)), "p1 backrow 1");
        }
    }

    mod n52_silly_silas_base_rotating_left {
        use super::*;

        #[test]
        fn r81_the_declared_direction_turns_the_ring_the_other_way() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-052"], "field": [{ "def": "core-053", "lane": 1 }] },
                "p2": { "field": [{ "def": "core-t-felinor", "lane": 5 }] },
            }));
            let reno = unit_at(&s, P1, 1).id;
            let felinor = unit_at(&s, P2, 5).id;

            s.play("core-052", json!({ "zone": 3, "modes": ["left"] }));

            // One step back along the ring: p1 lane 3 → p1 lane 2, p1 lane 1 → p2 lane 1 (crossing),
            // p2 lane 5 → p1 lane 5 (crossing the other way).
            assert_eq!(unit_at(&s, P1, 2).def_id, "core-052");
            assert_eq!(slot_of(s.card(&reno)), "p2 units 1");
            assert_eq!(s.card(&reno).controller, P2);
            assert_eq!(slot_of(s.card(&felinor)), "p1 units 5");
            assert_eq!(s.card(&felinor).controller, P1);
        }

        #[test]
        fn r11_a_unit_token_that_crosses_stays_on_the_field_it_changed_control_it_did_not_leave() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-052"] },
                "p2": { "field": [{ "def": "core-t-felinor", "lane": 5 }] },
            }));
            let felinor = unit_at(&s, P2, 5).id;

            s.play("core-052", json!({ "zone": 3, "modes": ["left"] }));

            s.expect_in_zone(&felinor, "field");
            assert_eq!(s.card(&felinor).controller, P1);
        }
    }

    mod n52_silly_silas_a_locked_destination_r14_r88 {
        use super::*;

        #[test]
        fn r14_bounces_the_card_to_its_controller_s_hand_instead_reset_per_r78() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-052"], "field": [{ "def": "core-053", "lane": 5, "damage": 2, "position": "DEF" }] },
            }));
            let reno = unit_at(&s, P1, 5).id;
            // §3.2 Lock, the destination of p1 lane 5 under a rightward rotation.
            lock_unit_zone(&mut s, P2, 5);

            s.play("core-052", json!({ "zone": 1, "modes": ["right"] }));

            s.expect_in_zone(&reno, "hand");
            assert_eq!(s.card(&reno).owner, P1);
            // R78: leaving the field resets damage, position and controller; the base bounce is not free.
            assert_eq!(s.card(&reno).damage, 0);
            assert!(s.card(&reno).cost_override.is_none());
            s.expect_events(json!(["rotated", "bounced"]));
            // Silas still rotated: the bounce is one zone's outcome, not a cancelled rotation.
            assert_eq!(unit_at(&s, P1, 2).def_id, "core-052");
        }

        #[test]
        fn r11_r88_a_unit_token_bounced_off_a_locked_destination_ceases_to_exist() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-052"], "field": [{ "def": "core-t-felinor", "lane": 5 }] },
            }));
            let felinor = unit_at(&s, P1, 5).id;
            lock_unit_zone(&mut s, P2, 5);

            s.play("core-052", json!({ "zone": 1, "modes": ["right"] }));

            s.expect_in_zone(&felinor, "gone");
        }
    }

    mod n52_silly_silas_radiant {
        use super::*;

        #[test]
        fn r14_every_card_that_would_move_to_the_opponent_is_bounced_to_its_controller_s_hand_costing_0_instead_and_a_card_crossing_to_its_side_still_crosses()
         {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-052"], "field": [{ "def": "core-053", "lane": 5, "damage": 2 }] },
                "p2": { "field": [{ "def": "core-019", "lane": 1 }] },
            }));
            // HARNESS GAP (reported): `SideSetup.hand` takes no `{ def, radiant }` form, and a radiant Cry
            // only fires if the card is PLAYED, so the flag goes on the hand instance (§5.2).
            s.card_mut("core-052").radiant = true;
            let reno = unit_at(&s, P1, 5).id;
            let menace = unit_at(&s, P2, 1).id;

            s.play("core-052", json!({ "zone": 1, "modes": ["right"] }));

            // "Cards that would move to the opponent": Reno would move from p1's lane 5 to p2's, so it goes
            // home instead, at cost 0 (R747: the CONTROLLER's hand, R65).
            s.expect_in_zone(&reno, "hand");
            assert_eq!(s.card(&reno).owner, P1);
            assert_eq!(s.card(&reno).cost_override, Some(0));
            // Midrange Menace moves from p2's lane 1 to p1's — to Silas's controller, not to the opponent —
            // so the radiant clause leaves it alone and the base clause holds: it crosses and changes
            // control, still owned by p2 and at its printed cost (§8 Conventions, R14, R171).
            assert_eq!(unit_at(&s, P1, 1).id, menace);
            assert_eq!(s.card(&menace).controller, P1);
            assert_eq!(s.card(&menace).owner, P2);
            assert!(s.card(&menace).cost_override.is_none());
            let changed: Vec<String> = s
                .last_events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::ControlChanged { instance_id, .. } => Some(instance_id.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(changed, vec![menace.clone()]);
        }

        #[test]
        fn s8_conventions_keep_the_rest_the_rotation_still_happens_and_silas_still_moves() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-052"], "field": [{ "def": "core-053", "lane": 3 }] },
            }));
            s.card_mut("core-052").radiant = true;

            s.play("core-052", json!({ "zone": 1, "modes": ["right"] }));

            // Neither card crosses, so the radiant clause never applies and both simply step one lane.
            assert_eq!(unit_at(&s, P1, 2).def_id, "core-052");
            assert_eq!(unit_at(&s, P1, 4).def_id, "core-053");
            s.expect_stats("core-052", json!({ "attack": 8, "maxHealth": 8 }));
        }

        #[test]
        fn r11_a_crossing_unit_token_is_bounced_so_it_ceases_to_exist_s3_2() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-052"], "field": [{ "def": "core-t-felinor", "lane": 5 }] },
            }));
            s.card_mut("core-052").radiant = true;
            let felinor = unit_at(&s, P1, 5).id;

            s.play("core-052", json!({ "zone": 1, "modes": ["right"] }));

            s.expect_in_zone(&felinor, "gone");
        }

        #[test]
        fn r14_the_direction_is_still_declared_leftward_crossings_bounce_the_same_way() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-052"], "field": [{ "def": "core-053", "lane": 1 }] },
            }));
            s.card_mut("core-052").radiant = true;
            let reno = unit_at(&s, P1, 1).id;

            s.play("core-052", json!({ "zone": 3, "modes": ["left"] }));

            s.expect_in_zone(&reno, "hand");
            assert_eq!(s.card(&reno).cost_override, Some(0));
        }
    }
}
