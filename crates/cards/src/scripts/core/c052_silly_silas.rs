//! #52 Silly Silas (SPEC §8.3, §3.1's rotation-topology ruling, §3.2, §6.3 Rotate; R4, R11, R12,
//! R14, R33, R78, R81, R88). Unit 4/4 → 8/8, Human, cost 3, Legendary. BUILD wave 3 (rotation).
//!   Base:    "Cry: choose left or right; rotate every card on the field one step around its ring
//!            (3.1); cards crossing sides change control"
//!   Radiant: "Cards that would move to the opponent are bounced to their controller's hand costing 0
//!            instead" — §8 Conventions: the cell restates only what happens to a crossing card, so
//!            the direction choice and the rotation itself are kept.
//!
//! THE DIRECTION IS A PLAY-TIME CHOICE, NOT A PROMPT. R81 names this card: "a declared `direction`
//! pick [travels] in `modes`, so … Silly Silas's direction [is] chosen with the play and never
//! pause[s] resolution", and §10.6 adds that no Core card ever opens a `direction` prompt. So the
//! script DECLARES `modes: [{ kind: "direction", options: ["left", "right"] }]`, `legal_actions`
//! enumerates both, and the answer arrives in `ctx.modes` — which is the second half of
//! `chosen_options(ctx)` (engine/src/effects/choose.rs), the one reader for a declared mode and a
//! prompt-mode answer alike.
//!
//! THE ROTATION IS THE SUBSYSTEM'S, NOT THIS FILE'S. `subsystems/rotation.rs` implements R14 in full
//! and is deliberately not in the effects barrel, so the card cannot call it and stay pure
//! (CLAUDE.md rule 5: `rotate_rings` takes an `EngineSink` and mutates the board). What it already
//! does, so that nothing here needs re-reading or re-deciding:
//!   * two rings, `ring_order`/`ring_neighbor` (§3.1): the rotating player's lanes 1→5, then the
//!     opponent's 5→1, and back; "right" is one step forward along that order, "left" one back;
//!     the unit ring and the backrow ring turn together, and "left"/"right" are read from the
//!     rotating player's seat, which is why the wrapper must pass `perspective: ctx.controller`;
//!   * the whole board is read before anything is placed, so one rotation is atomic and a full ring
//!     keeps every card;
//!   * a Stack pile travels whole and keeps its top card on top (§3.2, R13);
//!   * Silas is already on the field when his Cry resolves (§10.5 step 4 precedes step 5), so he is
//!     in the snapshot and rotates with everything else — the Engine cell's "Silas rotates too";
//!   * a card never leaves the field, so R78's reset never runs: damage, buffs, granted keywords,
//!     counters and position all travel with it (R14), and so do exertion and `summoned_turn` for a
//!     card that moves along its own side;
//!   * a card that crosses the centre line has entered its new controller's side (R171): it takes
//!     this turn as its `summoned_turn`, so it is summoning sick there, and a fresh exertion;
//!   * `controller` changes only when the destination is on the other side of the centre line, and
//!     `owner` changes only when a bounce takes the card to its controller's hand as theirs
//!     (R747) and never on a crossing (R12), so a crossed card still leaves to its owner's other
//!     piles later; a face-down trap that crosses is read by its new controller alone, which follows from
//!     `controller` and is why `face_up` is untouched (R33);
//!   * a Locked or Reborn-reserved destination bounces the card to its controller's hand instead
//!     (R14, R88, R747), where the hand cap applies (R4) and a unit token ceases to exist on the way (R11);
//!   * `radiant: true` replaces an OUTBOUND crossing — a card leaving this player's side for the
//!     opponent's, "cards that would move to the opponent" — with that same bounce at
//!     `cost_override: 0`. The opponent's cards crossing onto this side are not moving "to the
//!     opponent", so the base clause holds for them and they cross and change control (R14, R65,
//!     R171): the radiant face loses nothing and still takes what the rotation brings over.
//!
//! BASE AND RADIANT SHARE ONE HOOK. The only difference between the faces is `ctx.radiant`, which
//! `rotate_rings` already honours through its own `radiant` argument — so the wrapper passes
//! `radiant: ctx.radiant` and there is exactly one rotation implementation in the game.
//!
//! THE WRAPPER IS `rotate` (engine/src/effects/rotate.rs), the effects barrel's Rotate verb. Its
//! `apply(ctx)` is one call to `rotate_rings` — an `EffectContext` already satisfies `EngineSink`
//! (`state`, `events`, `rng`) — and its defaults are the controller's perspective and the running
//! face's `radiant`, so `rotate({ direction })` is the whole call and no rotation logic moves.

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
// R14, R33, R78, R81, R88).
// BUILD M4-T4 row 52: "Rotate both rings either direction, control changes on crossing, damage
// travels, Silas moves too; Locked destination bounces; radiant bounces crossing cards to their
// owner's hand at cost 0 (R14)".
//
// The ring, from p1's seat (§3.1): p1 lanes 1→5, then p2 lanes 5→1, and back to p1 lane 1. So
// "right" is one step forward along that order — p1 lane 5 becomes p2 lane 5 and p2 lane 1 becomes
// p1 lane 1 — and "left" is one step back.
//
// The zone is locked with the engine's own `lock_zone`, the very function the §6.3 Lock effect calls
// (`effects/counters.rs`), because `SideSetup` has no `locks` option — reported as a harness gap.
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

    /// TS `lockZone(s.state, { player, row: "units", lane })`.
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
