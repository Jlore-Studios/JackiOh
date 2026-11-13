//! #52 Silly Silas (SPEC §8.3, §3.1's rotation-topology ruling, §3.2, §6.3 Rotate; R4, R11, R12,
//! R14, R33, R78, R81, R88, R3837000, R3837001, R3837002). Unit 2/2 → 4/4, Human, cost 4, Legendary
//! (issue #3136; it was 4/4 → 8/8). BUILD wave 3 (rotation).
//!   Base:    "Summon this for your opponent. Cry: choose left or right; rotate every card on the field
//!            one step around its ring (3.1); cards crossing sides change control"
//!   Radiant: "Cards that would move to the opponent are bounced to their controller's hand costing 0
//!            instead" — §8 Conventions: the cell restates only what happens to a crossing card, so
//!            the direction choice and the rotation itself are kept. The Radiant face stays with its
//!            player.
//!
//! Issue #3837 took Crater off both faces (issue #3136 had put it on) and gave the base face "Summon
//! this for your opponent", as #55 Lava Golem's base face is (R360) but with no condition: every play of
//! it lands in the opponent's leftmost open unit zone, under their control (R3837000, the
//! `summonForOpponent` flag), or where the play named when they have none (R3837001). It is still the
//! player's play and card, so the Cry that follows is the player's and reads "left" and "right" from the
//! player's seat (R3837002); Silas rotates with the rest and may cross back. `script_v0_6_0d` keeps the
//! scripts through v0.6.0d, both faces staying with their player, for those replays (R767000).
//!
//! The direction is a play-time choice, not a prompt: R81 names this card and §10.6 says no Core card
//! opens a `direction` prompt, so the script declares `modes` and `chosen_options(ctx)` reads `ctx.modes`.
//! The rotation is `subsystems/rotation.rs`'s (R14), which mutates the board and so is not in the
//! effects barrel (CLAUDE.md rule 5); the `rotate` verb (engine/src/effects/rotate.rs) wraps it with
//! the controller's perspective and the running face's `radiant`, so both faces share one hook.
//!   * both rings turn together, read whole before anything is placed (§3.1); a Stack travels whole
//!     (§3.2, R13); a card never leaves the field, so R78's reset never runs and damage travels;
//!   * a crossing card enters summoning sick (R171); `controller` changes and `owner` does not (R12,
//!     R747), and a face-down trap is read by its new controller (R33);
//!   * a Reborn-reserved destination bounces the card to its controller's hand (R88): the hand cap
//!     applies (R4) and a unit token ceases to exist (R11); a Locked one takes the card (R831000, #831);
//!   * radiant turns an OUTBOUND crossing into that bounce at `cost_override: 0` (R65).

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

/// The declared pick, narrowed rather than cast. `chosen_options(ctx)` reads a prompt answer's mode,
/// then `ctx.modes`, so it covers an Echo repeat too. R90 keeps a play to the answers that exist; one
/// carrying no direction fizzles here instead of guessing a side for the whole board.
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

/// Both faces, with the base face summoned for the opponent when `for_opponent` (R3837000).
fn silly_silas(for_opponent: bool) -> CardScripts {
    CardScripts {
        base: Script {
            modes: vec![direction_decl()],
            cry: Some(hook(silas)),
            static_flags: for_opponent.then(|| StaticFlags {
                summon_for_opponent: Some(true),
                ..StaticFlags::default()
            }),
            ..Script::default()
        },
        // The same declaration and hook: the radiant difference is `ctx.radiant`, which the rotation
        // subsystem reads for itself (R14, R747).
        radiant: Script {
            modes: vec![direction_decl()],
            cry: Some(hook(silas)),
            ..Script::default()
        },
    }
}

pub fn script() -> CardScripts {
    silly_silas(true)
}

/// R767000: through v0.6.0d both faces stayed with their player; `KEPT_SCRIPTS` folds those replays
/// under it.
pub fn script_v0_6_0d() -> CardScripts {
    silly_silas(false)
}

// #52 Silly Silas (SPEC §8.3, §3.1's rotation-topology ruling, §3.2, §6.3 Rotate; R4, R11, R12,
// R14, R33, R78, R81, R88). BUILD M4-T4 row 52 (R831000). The ring, from p1's seat (§3.1): p1 lanes 1→5, then
// p2 lanes 5→1, back to p1 lane 1; "right" is one step forward along it, "left" one back.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

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

    /// The `summoned` event that landed Silas: its side and lane.
    fn landed(s: &Scenario) -> (PlayerId, i32) {
        s.last_events()
            .iter()
            .find_map(|event| match event {
                GameEvent::Summoned { player, def_id, lane, .. } if def_id == "core-052" => Some((*player, *lane)),
                _ => None,
            })
            .expect("Silas was summoned")
    }

    /// Issue #3837 took Crater off both faces (issue #3136 had put it on): no zone is Locked as he
    /// arrives, and nothing is Locked after the Rotate.
    #[test]
    fn issue_3837_neither_face_has_crater_and_no_zone_is_locked() {
        for radiant in [false, true] {
            let def = crate::card_def("core-052");
            let face = if radiant { &def.radiant } else { &def.base };
            assert!(face.keywords.is_empty(), "radiant {radiant}");
            let mut s = scenario(json!({ "p1": { "hand": [{ "def": "core-052", "radiant": radiant }] } }));

            s.play("core-052", json!({ "zone": 1, "modes": ["right"] }));

            assert_eq!(s.state().players.p1.locks.units, vec![false; 5]);
            assert_eq!(s.state().players.p2.locks.units, vec![false; 5]);
            assert!(
                !s.last_events()
                    .iter()
                    .any(|event| event.event_type() == GameEventType::Locked)
            );
        }
    }

    mod n52_silly_silas_summoned_for_the_opponent_r3837000 {
        use super::*;

        #[test]
        fn r3837000_the_base_face_lands_in_the_opponents_leftmost_open_unit_zone_under_their_control() {
            let mut s = scenario(json!({
                "p1": { "hand": ["core-052"] },
                "p2": { "field": [{ "def": "core-t-felinor", "lane": 1 }] },
            }));
            let felinor = s.unit_at(P2, 1).id;

            s.play("core-052", json!({ "zone": 3, "modes": ["left"] }));

            // Not p1's lane 3, nor p2's lane 3 as #55 would take (R360): p2's leftmost open zone.
            assert_eq!(landed(&s), (P2, 2));
            // Then the Cry: "left" is one step back along p1's ring, so p2's lanes step up by one.
            assert_eq!(slot_of(s.card("core-052")), "p2 units 3");
            assert_eq!(s.card("core-052").controller, P2);
            assert_eq!(s.card("core-052").owner, P1);
            assert_eq!(slot_of(s.card(&felinor)), "p2 units 2");
            assert!(s.state().players.p1.units.iter().all(Option::is_none));
            // It is still p1's play (R360).
            assert!(
                s.last_events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::CardPlayed { player, .. } if *player == P1))
            );
        }

        #[test]
        fn r3837000_only_a_play_hands_him_over_a_summoned_silas_stays_with_its_summoner() {
            let mut s = scenario(json!({ "p1": { "hand": [] } }));
            crate::run_as(
                &mut s,
                jackioh_engine::effects::summon(json_as(json!({ "defId": "core-052" }))),
                P1,
            );
            assert_eq!(slot_of(s.card("core-052")), "p1 units 1");
            assert_eq!(s.card("core-052").controller, P1);
        }

        #[test]
        fn r3837000_the_radiant_face_stays_with_its_player() {
            let mut s = scenario(json!({ "p1": { "hand": [{ "def": "core-052", "radiant": true }] } }));

            s.play("core-052", json!({ "zone": 1, "modes": ["right"] }));

            assert_eq!(landed(&s), (P1, 1));
            assert_eq!(slot_of(s.card("core-052")), "p1 units 2");
            assert_eq!(s.card("core-052").controller, P1);
        }

        #[test]
        fn r3837001_with_no_open_unit_zone_on_the_opponents_side_it_lands_where_the_play_named() {
            let field: Vec<Value> = (1..=5)
                .map(|lane| json!({ "def": "stand-in (1) 1/1", "lane": lane }))
                .collect();
            let mut s = scenario(json!({ "p1": { "hand": ["core-052"] }, "p2": { "field": field } }));

            s.play("core-052", json!({ "zone": 3, "modes": ["right"] }));

            assert_eq!(landed(&s), (P1, 3));
            assert_eq!(slot_of(s.card("core-052")), "p1 units 4");
            assert_eq!(s.card("core-052").controller, P1);
        }

        #[test]
        fn r3837002_the_cry_rotates_after_the_landing_from_the_players_seat() {
            // "right" from p2's lane 1 is one step forward along p1's ring: home, to p1's lane 1.
            let mut right = scenario(json!({ "p1": { "hand": ["core-052"] } }));
            right.play("core-052", json!({ "zone": 1, "modes": ["right"] }));
            assert_eq!(landed(&right), (P2, 1));
            assert_eq!(slot_of(right.card("core-052")), "p1 units 1");
            assert_eq!(right.card("core-052").controller, P1);
            let kinds: Vec<GameEventType> = right.last_events().iter().map(|event| event.event_type()).collect();
            let summoned = kinds.iter().position(|kind| *kind == GameEventType::Summoned);
            let rotated = kinds.iter().position(|kind| *kind == GameEventType::Rotated);
            assert!(summoned.is_some() && rotated.is_some());
            assert!(summoned < rotated);

            // "left" is one step back: on along p2's row, still theirs.
            let mut left = scenario(json!({ "p1": { "hand": ["core-052"] } }));
            left.play("core-052", json!({ "zone": 1, "modes": ["left"] }));
            assert_eq!(landed(&left), (P2, 1));
            assert_eq!(slot_of(left.card("core-052")), "p2 units 2");
            assert_eq!(left.card("core-052").controller, P2);
        }

        #[test]
        fn r767000_the_kept_v0_6_0d_script_stays_with_its_player() {
            crate::under_kept(super::super::ID, super::super::script_v0_6_0d(), || {
                let mut s = scenario(json!({ "p1": { "hand": ["core-052"] } }));
                s.play("core-052", json!({ "zone": 1, "modes": ["right"] }));
                assert_eq!(landed(&s), (P1, 1));
                assert_eq!(slot_of(s.card("core-052")), "p1 units 2");
                assert_eq!(s.card("core-052").controller, P1);
            });
        }
    }

    mod n52_silly_silas_base_rotating_right {
        use super::*;

        #[test]
        fn r14_turns_both_rings_one_step_and_silas_rotates_with_them() {
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
            let reno = s.unit_at(P1, 5).id;
            let felinor = s.unit_at(P2, 1).id;
            let farm = backrow_at(&s, P1, 1).id;
            let stimmy = backrow_at(&s, P2, 1).id;

            s.play("core-052", json!({ "zone": 1, "modes": ["right"] }));

            // The Engine cell's "Silas rotates too": he is on the field for his own Cry (§10.5 step 4),
            // in p2's leftmost open zone, lane 2 (R3837000), and steps on to their lane 1.
            assert_eq!(s.unit_at(P2, 1).def_id, "core-052");
            assert_eq!(slot_of(s.card(&reno)), "p2 units 5");
            assert_eq!(slot_of(s.card(&felinor)), "p1 units 1");
            assert_eq!(slot_of(s.card(&farm)), "p1 backrow 2");
            assert_eq!(slot_of(s.card(&stimmy)), "p1 backrow 1");
            s.expect_events(json!(["cardPlayed", "rotated", "controlChanged"]));
        }

        #[test]
        fn r12_a_card_that_crosses_the_center_line_changes_controller_and_never_owner() {
            let mut s = scenario(json!({
                "p1": { "hand": ["core-052"], "field": [{ "def": "core-053", "lane": 5 }] },
                "p2": { "field": [{ "def": "core-t-felinor", "lane": 1 }] },
            }));
            let reno = s.unit_at(P1, 5).id;
            let felinor = s.unit_at(P2, 1).id;

            s.play("core-052", json!({ "zone": 1, "modes": ["right"] }));

            assert_eq!(s.card(&reno).controller, P2);
            assert_eq!(s.card(&reno).owner, P1);
            assert_eq!(s.card(&felinor).controller, P1);
            assert_eq!(s.card(&felinor).owner, P2);
            // Summoned for p2 into their lane 2 (R3837000), Silas steps to their lane 1 and stays theirs.
            assert_eq!(slot_of(s.card("core-052")), "p2 units 1");
            assert_eq!(s.card("core-052").controller, P2);
            assert_eq!(s.card("core-052").owner, P1);
        }

        #[test]
        fn r14_r78_damage_and_every_other_part_of_the_instance_travel_with_the_card() {
            let mut s = scenario(json!({
                "p1": { "hand": ["core-052"], "field": [{ "def": "core-053", "lane": 5, "damage": 2, "position": "DEF" }] },
            }));
            let reno = s.unit_at(P1, 5).id;

            s.play("core-052", json!({ "zone": 1, "modes": ["right"] }));

            // A rotation never takes a card off the field, so R78's reset never runs: 4/6 with 2 damage.
            s.expect_stats(&reno, json!({ "attack": 4, "health": 4, "maxHealth": 6 }));
            assert_eq!(s.card(&reno).damage, 2);
            assert_eq!(s.card(&reno).position, Some(Position::Def));
            assert_eq!(slot_of(s.card(&reno)), "p2 units 5");
        }

        #[test]
        fn r33_a_face_down_trap_that_crosses_is_read_by_its_new_controller_alone() {
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
            let mut s = scenario(json!({
                "p1": { "hand": ["core-052"], "field": [{ "def": "core-053", "lane": 1 }] },
                "p2": { "field": [{ "def": "core-t-felinor", "lane": 5 }] },
            }));
            let reno = s.unit_at(P1, 1).id;
            let felinor = s.unit_at(P2, 5).id;

            s.play("core-052", json!({ "zone": 3, "modes": ["left"] }));

            // Summoned into p2's lane 1 (R3837000), Silas steps back along the ring to their lane 2.
            assert_eq!(s.unit_at(P2, 2).def_id, "core-052");
            assert_eq!(slot_of(s.card(&reno)), "p2 units 1");
            assert_eq!(s.card(&reno).controller, P2);
            assert_eq!(slot_of(s.card(&felinor)), "p1 units 5");
            assert_eq!(s.card(&felinor).controller, P1);
        }

        #[test]
        fn r11_a_unit_token_that_crosses_stays_on_the_field_it_changed_control_it_did_not_leave() {
            let mut s = scenario(json!({
                "p1": { "hand": ["core-052"] },
                "p2": { "field": [{ "def": "core-t-felinor", "lane": 5 }] },
            }));
            let felinor = s.unit_at(P2, 5).id;

            s.play("core-052", json!({ "zone": 3, "modes": ["left"] }));

            s.expect_in_zone(&felinor, "field");
            assert_eq!(s.card(&felinor).controller, P1);
        }
    }

    mod n52_silly_silas_a_locked_destination_r14_r831000 {
        use super::*;

        #[test]
        fn r14_r831000_the_card_rotates_into_the_locked_zone_its_damage_and_position_kept() {
            let mut s = scenario(json!({
                "p1": { "hand": ["core-052"], "field": [{ "def": "core-053", "lane": 5, "damage": 2, "position": "DEF" }] },
            }));
            let reno = s.unit_at(P1, 5).id;
            // §3.2 Lock, the destination of p1 lane 5 under a rightward rotation.
            lock_unit_zone(&mut s, P2, 5);

            s.play("core-052", json!({ "zone": 1, "modes": ["right"] }));

            assert_eq!(s.unit_at(P2, 5).id, reno);
            assert_eq!(s.card(&reno).controller, P2);
            assert_eq!(s.card(&reno).owner, P1);
            assert_eq!(s.card(&reno).damage, 2);
            s.expect_events(json!(["rotated", "controlChanged"]));
            // Summoned into p2's leftmost open zone, lane 1 (their lane 5 is Locked), Silas crosses home.
            assert_eq!(s.unit_at(P1, 1).def_id, "core-052");
        }

        #[test]
        fn r831000_a_unit_token_rotates_into_a_locked_zone_too() {
            let mut s = scenario(json!({
                "p1": { "hand": ["core-052"], "field": [{ "def": "core-t-felinor", "lane": 5 }] },
            }));
            let felinor = s.unit_at(P1, 5).id;
            lock_unit_zone(&mut s, P2, 5);

            s.play("core-052", json!({ "zone": 1, "modes": ["right"] }));

            s.expect_in_zone(&felinor, "field");
            assert_eq!(s.unit_at(P2, 5).id, felinor);
        }
    }

    mod n52_silly_silas_radiant {
        use super::*;

        #[test]
        fn r14_every_card_that_would_move_to_the_opponent_is_bounced_to_its_controller_s_hand_costing_0_instead_and_a_card_crossing_to_its_side_still_crosses()
         {
            let mut s = scenario(json!({
                "p1": { "hand": ["core-052"], "field": [{ "def": "core-053", "lane": 5, "damage": 2 }] },
                "p2": { "field": [{ "def": "core-019", "lane": 1 }] },
            }));
            // HARNESS GAP (reported): `SideSetup.hand` takes no `{ def, radiant }` form, and a radiant Cry
            // only fires if the card is PLAYED, so the flag goes on the hand instance (§5.2).
            s.card_mut("core-052").radiant = true;
            let reno = s.unit_at(P1, 5).id;
            let menace = s.unit_at(P2, 1).id;

            s.play("core-052", json!({ "zone": 1, "modes": ["right"] }));

            // "Cards that would move to the opponent": Reno would move from p1's lane 5 to p2's, so it goes
            // home instead, at cost 0 (R747: the CONTROLLER's hand, R65).
            s.expect_in_zone(&reno, "hand");
            assert_eq!(s.card(&reno).owner, P1);
            assert_eq!(s.card(&reno).cost_override, Some(0));
            // Midrange Menace moves to Silas's controller, not to the opponent, so the base clause holds:
            // it crosses and changes control, still p2's and at its printed cost (§8 Conventions, R14, R171).
            assert_eq!(s.unit_at(P1, 1).id, menace);
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
            let mut s = scenario(json!({
                "p1": { "hand": ["core-052"], "field": [{ "def": "core-053", "lane": 3 }] },
            }));
            s.card_mut("core-052").radiant = true;

            s.play("core-052", json!({ "zone": 1, "modes": ["right"] }));

            // Neither card crosses, so the radiant clause never applies and both simply step one lane.
            assert_eq!(s.unit_at(P1, 2).def_id, "core-052");
            assert_eq!(s.unit_at(P1, 4).def_id, "core-053");
            s.expect_stats("core-052", json!({ "attack": 4, "maxHealth": 4 }));
        }

        #[test]
        fn r11_a_crossing_unit_token_is_bounced_so_it_ceases_to_exist_s3_2() {
            let mut s = scenario(json!({
                "p1": { "hand": ["core-052"], "field": [{ "def": "core-t-felinor", "lane": 5 }] },
            }));
            s.card_mut("core-052").radiant = true;
            let felinor = s.unit_at(P1, 5).id;

            s.play("core-052", json!({ "zone": 1, "modes": ["right"] }));

            s.expect_in_zone(&felinor, "gone");
        }

        #[test]
        fn r14_the_direction_is_still_declared_leftward_crossings_bounce_the_same_way() {
            let mut s = scenario(json!({
                "p1": { "hand": ["core-052"], "field": [{ "def": "core-053", "lane": 1 }] },
            }));
            s.card_mut("core-052").radiant = true;
            let reno = s.unit_at(P1, 1).id;

            s.play("core-052", json!({ "zone": 3, "modes": ["left"] }));

            s.expect_in_zone(&reno, "hand");
            assert_eq!(s.card(&reno).cost_override, Some(0));
        }
    }
}
