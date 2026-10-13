//! #18 Bread and Butter (SPEC §8.1, §5.1, §7, R37, R52, R62): "When any player ends a turn with
//! unspent mana: summon a Bread Token X/X for the trap's controller, X = that player's unspent mana".
//! The radiant cell only says "X = 3 × unspent" (§8 Conventions), so both faces share the trigger,
//! the beneficiary and the token.
//!
//! It fires in R62's end-of-turn trap window (both sides, in R68 order): a trap trigger on `turnEnded`,
//! which `traps.rs` reserves for that window (`TRAP_WINDOW_EVENTS`). "Any player" needs no condition.
//! R52: the token goes to the trap's controller (`player: "self"`), whoever ended the turn. X comes off
//! the event (`turnEnded.unspentMana`), so the trigger reads no state. The token is printed 0/0 and
//! summoned X/X through `statsOverride` (§7, R37). A Field Trap is not consumed (§5.1, §3.2, R33).
//! The R280 preview and the R195/R662 glow read `unspent_mana_of`, which is public (§10.8); R33 hides
//! a face-down trap's preview from the opponent.

use jackioh_engine::effects::summon;
use jackioh_engine::prelude::*;

pub const ID: &str = "core-018";

/// §7: the Bread Token definition; its X/X comes from `statsOverride`, never from the def.
const BREAD_TOKEN: &str = "core-t-bread";

/// The ending player's unspent mana, straight off the event. Any other event type is not this
/// trigger's (`on` already guarantees `turnEnded`), and a negative pool cannot happen, so both read
/// as "nothing unspent".
fn unspent_on(event: &GameEvent) -> i32 {
    match event {
        GameEvent::TurnEnded { unspent_mana, .. } => (*unspent_mana).max(0),
        _ => 0,
    }
}

/// The same on both faces: `multiplier` is §8's X (base) or 3X (radiant), a declared number (R386)
/// tuned on the Radiant face only (R749: the base face prints no number, always 1). The condition is
/// written twice: `traps.rs` filters on `when`, `run_queued_trigger` ignores it and takes an empty
/// effect list as "not met", so either way 0 unspent summons nothing and the trap stays armed.
fn bread_trigger() -> TriggerDef {
    TriggerDef::new("bread-and-butter", &[GameEventType::TurnEnded], |ctx, event| {
        let x = unspent_on(event) * param(&*ctx, "multiplier");
        if x <= 0 {
            return vec![];
        }
        vec![summon(json_as(json!({
            "defId": BREAD_TOKEN,
            // R52: the trap's controller, whoever ended the turn.
            "player": "self",
            // §7, R37: X/X on a card printed 0/0.
            "statsOverride": { "attack": x, "health": x },
            // §7: the radiant face prints "Armor X", the same X; carried now because nothing later knows X.
            "armorOverride": x
        })))]
    })
    .with_when(|_ctx, event| unspent_on(event) > 0)
}

/// R280: the formula as the face prints it, which the preview labels its number with — X on the base
/// face, `multiplier` × the unspent mana on the Radiant one, with the multiplier as it stands.
fn formula(radiant: bool, multiplier: i32) -> String {
    if radiant {
        format!("X = {multiplier} \u{00d7} that player's unspent mana")
    } else {
        "X = that player's unspent mana".to_string()
    }
}

/// R280: the Bread Token's X if the active player ended the turn now. `your_turn` names the active
/// player without the card reading `state.active` (README §1).
fn preview() -> PreviewHook {
    condition_hook(|ctx: ConditionContext<'_>| {
        let active = if ctx.your_turn {
            ctx.controller
        } else {
            opponent_of(ctx.controller)
        };
        let multiplier = param(&ctx, "multiplier");
        vec![PreviewValue {
            label: formula(ctx.radiant, multiplier),
            value: unspent_mana_of(ctx.state, active) * multiplier,
            display: None,
            ids: None,
        }]
    })
}

/// R662: "ends a turn with unspent mana", asked of the turn as it stands; the same on both faces.
fn condition_met() -> ConditionHook {
    condition_hook(|ctx: ConditionContext<'_>| {
        if ctx.zone != ConditionZone::Field {
            return false;
        }
        let active = if ctx.your_turn {
            ctx.controller
        } else {
            opponent_of(ctx.controller)
        };
        unspent_mana_of(ctx.state, active) > 0
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        triggers: vec![bread_trigger()],
        preview: Some(preview()),
        condition_met: Some(condition_met()),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// #18 Bread and Butter — SPEC §8.1 row 18, BUILD M4-T4 row 18.
//
// Must-pass (M4-T4): "Fires in the trap window at either player's end with unspent mana (R62), token
// to trap controller (R52), X = unspent, 0 → nothing, stays; radiant 3X".
//
// R62: the trap watches `turnEnded`, which `traps.rs` reserves for the end-of-turn trap window (R68
// order). R52 is why the opponent's-turn test asserts an EMPTY board on the ending player's side. §7,
// R37: the token is printed 0/0, so every assertion reads the def id AND the stats. The R280 preview
// is proved in tests/cross/preview.rs; R662's glow (`conditionMet`) at the end of this file.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const BREAD_TOKEN: &str = "core-t-bread";

    fn unit_at(s: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
        match s.unit(player, lane) {
            Some(found) => found,
            None => panic!("no unit in {player} unit lane {lane}"),
        }
    }

    fn backrow_at(s: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
        match s.backrow(player, lane) {
            Some(found) => found,
            None => panic!("no card in {player} backrow lane {lane}"),
        }
    }

    /// §7, R37: a Bread Token of exactly X/X, not the printed 0/0.
    fn expect_bread(s: &mut Scenario, token: &CardInstance, x: i32) {
        assert_eq!(token.def_id, BREAD_TOKEN);
        s.expect_stats(token, json!({ "attack": x, "health": x, "maxHealth": x }));
    }

    mod n18_bread_and_butter_base {
        use super::*;

        #[test]
        fn r62_fires_in_the_end_of_turn_trap_window_of_its_controller_s_own_turn() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "backrow": ["core-018"], "mana": 2, "library": ["core-010"] },
                "p2": { "field": ["core-012"], "library": ["core-010"] }
            }));

            s.end_turn();

            let token = unit_at(&s, PlayerId::P1, 1);
            expect_bread(&mut s, &token, 2);
            s.expect_events(json!(["turnEnded", "trapFired", "summoned"]));
        }

        #[test]
        fn r62_r52_fires_at_the_opponent_s_end_of_turn_and_the_token_goes_to_the_trap_s_controller() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "backrow": ["core-018"], "field": ["core-012"], "library": ["core-010"] },
                "p2": { "mana": 3, "library": ["core-010"] }
            }));

            s.end_turn();

            // R64: lane 1 is taken, so the token takes the leftmost free zone.
            let token = unit_at(&s, PlayerId::P1, 2);
            expect_bread(&mut s, &token, 3);
            // R52: p2 ended the turn with the unspent mana, and gets nothing for it.
            for lane in 1..=5 {
                assert!(s.unit(PlayerId::P2, lane).is_none());
            }
        }

        #[test]
        fn x_is_the_ending_player_s_unspent_mana() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "backrow": ["core-018"], "mana": 4, "library": ["core-010"] },
                "p2": { "field": ["core-012"], "library": ["core-010"] }
            }));

            s.end_turn();

            let token = unit_at(&s, PlayerId::P1, 1);
            expect_bread(&mut s, &token, 4);
        }

        #[test]
        fn a_turn_ended_with_no_unspent_mana_summons_nothing_and_leaves_the_trap_armed_and_face_down() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "backrow": ["core-018"], "field": ["core-012"], "mana": 0, "library": ["core-010"] },
                "p2": { "field": ["core-020"], "library": ["core-010"] }
            }));
            let trap = backrow_at(&s, PlayerId::P1, 1);

            s.end_turn();

            for lane in 2..=5 {
                assert!(s.unit(PlayerId::P1, lane).is_none());
            }
            s.expect_in_zone(&trap, "field");
            // Nothing fired, so §5.1's reveal never happened and the opponent still sees a bare marker.
            assert_ne!(s.card(&trap).face_up, Some(true));
            let seen = serde_json::to_value(&s.view(PlayerId::P2).opponent.backrow[0]).unwrap_or(Value::Null);
            assert_eq!(seen, json!({ "faceDown": true, "cost": 1 }));
        }

        #[test]
        fn s5_1_a_field_trap_is_not_consumed_it_stays_on_the_field_and_pays_out_again() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "backrow": ["core-018"], "mana": 1, "library": ["core-010", "core-011"] },
                "p2": { "field": ["core-012"], "library": ["core-010", "core-011"] }
            }));
            let trap = backrow_at(&s, PlayerId::P1, 1);

            s.end_turn(); // p1 ends with 1 unspent

            let first = unit_at(&s, PlayerId::P1, 1);
            expect_bread(&mut s, &first, 1);
            s.expect_in_zone(&trap, "field");
            // R33: a Field Trap that has fired is face-up to both players from then on.
            assert_eq!(s.card(&trap).face_up, Some(true));

            s.end_turn(); // p2 ends, its mana refreshed to MAX_MANA and unspent

            let second = unit_at(&s, PlayerId::P1, 2);
            expect_bread(&mut s, &second, 4);
            s.expect_in_zone(&trap, "field");
            assert!(!s.pile(PlayerId::P1, "graveyard").iter().any(|card| card.id == trap.id));
        }
    }

    mod n18_bread_and_butter_radiant {
        use super::*;

        #[test]
        fn summons_a_3x_3x_bread_token() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "backrow": [{ "def": "core-018", "radiant": true }], "mana": 2, "library": ["core-010"] },
                "p2": { "field": ["core-012"], "library": ["core-010"] }
            }));

            s.end_turn();

            let token = unit_at(&s, PlayerId::P1, 1);
            expect_bread(&mut s, &token, 6);
        }

        #[test]
        fn r386_an_upgrade_makes_it_4x_and_a_degrade_2x_and_the_preview_reads_the_moved_multiplier() {
            for (upgrade, multiplier) in [(true, 4), (false, 2)] {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "backrow": [{ "def": "core-018", "radiant": true }], "mana": 2, "library": ["core-010"] },
                    "p2": { "field": ["core-012"], "library": ["core-010"] }
                }));
                let moved = if upgrade {
                    crate::upgrade_number(&mut s, "core-018", "multiplier")
                } else {
                    crate::degrade_number(&mut s, "core-018", "multiplier")
                };
                assert_eq!(moved, multiplier);
                let shown = serde_json::to_value(&s.view(PlayerId::P1).you.backrow[0]).unwrap_or_default();
                assert_eq!(
                    shown["preview"][0]["label"],
                    json!(format!("X = {multiplier} \u{00d7} that player's unspent mana"))
                );
                assert_eq!(shown["preview"][0]["value"], json!(2 * multiplier));

                s.end_turn();

                let token = unit_at(&s, PlayerId::P1, 1);
                expect_bread(&mut s, &token, 2 * multiplier);
            }
        }

        #[test]
        fn r749_the_base_face_s_multiplier_of_1_is_never_tuned() {
            crate::register_all();
            let s = scenario(json!({ "p1": { "backrow": ["core-018"] } }));
            assert!(!crate::can_upgrade_number(&s, "core-018", "multiplier"));
            assert!(!crate::can_degrade_number(&s, "core-018", "multiplier"));
        }

        #[test]
        fn r52_the_3x_token_still_goes_to_the_trap_s_controller_at_the_opponent_s_end_of_turn() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [{ "def": "core-018", "radiant": true }], "field": ["core-012"], "library": ["core-010"] },
                "p2": { "mana": 1, "library": ["core-010"] }
            }));

            s.end_turn();

            let token = unit_at(&s, PlayerId::P1, 2);
            expect_bread(&mut s, &token, 3);
            for lane in 1..=5 {
                assert!(s.unit(PlayerId::P2, lane).is_none());
            }
        }

        #[test]
        fn t3_0_is_still_nothing_no_token_at_0_unspent_mana() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "backrow": [{ "def": "core-018", "radiant": true }],
                    "field": ["core-012"],
                    "mana": 0,
                    "library": ["core-010"]
                },
                "p2": { "field": ["core-020"], "library": ["core-010"] }
            }));
            let trap = backrow_at(&s, PlayerId::P1, 1);

            s.end_turn();

            for lane in 2..=5 {
                assert!(s.unit(PlayerId::P1, lane).is_none());
            }
            s.expect_in_zone(&trap, "field");
        }
    }

    mod n18_bread_and_butter_glows_while_a_turn_would_end_with_unspent_mana_r662 {
        use super::*;

        fn face_of(radiant: bool) -> &'static str {
            if radiant { "radiant" } else { "base" }
        }

        /// R662 <face>: with mana left it glows for its controller only, and ending the turn pays out.
        fn glows_with_mana_left(radiant: bool) {
            crate::register_all();
            let face = face_of(radiant);
            let mut s = scenario(json!({
                "seed": format!("r662-018-{face}-on"),
                "p1": { "backrow": [{ "def": "core-018", "radiant": radiant }], "mana": 2, "library": ["core-010"] },
                "p2": { "library": ["core-010"] }
            }));
            assert!(backrow_glows(&s, 1, PlayerId::P1));
            assert!(!opponent_sees_glow(&s, 1, PlayerId::P1));

            s.end_turn();
            assert_eq!(
                s.unit(PlayerId::P1, 1).map(|unit| unit.def_id),
                Some(BREAD_TOKEN.to_string())
            );
        }

        /// R662 <face>: with no mana left it does not glow, and ending the turn pays nothing.
        fn no_glow_without_mana(radiant: bool) {
            crate::register_all();
            let face = face_of(radiant);
            let mut s = scenario(json!({
                "seed": format!("r662-018-{face}-off"),
                "p1": { "backrow": [{ "def": "core-018", "radiant": radiant }], "mana": 0, "library": ["core-010"] },
                "p2": { "library": ["core-010"] }
            }));
            assert!(!backrow_glows(&s, 1, PlayerId::P1));

            s.end_turn();
            assert!(s.unit(PlayerId::P1, 1).is_none());
        }

        /// R662 <face>: on the opponent's turn it reads the opponent's mana.
        fn reads_the_opponent_s_mana(radiant: bool) {
            crate::register_all();
            let face = face_of(radiant);
            let on = scenario(json!({
                "seed": format!("r662-018-{face}-theirs"),
                "active": "p2",
                "p1": { "backrow": [{ "def": "core-018", "radiant": radiant }], "mana": 0 },
                "p2": { "mana": 3 }
            }));
            assert!(backrow_glows(&on, 1, PlayerId::P1));
            let off = scenario(json!({
                "seed": format!("r662-018-{face}-theirs-0"),
                "active": "p2",
                "p1": { "backrow": [{ "def": "core-018", "radiant": radiant }], "mana": 3 },
                "p2": { "mana": 0 }
            }));
            assert!(!backrow_glows(&off, 1, PlayerId::P1));
        }

        #[test]
        fn r662_base_with_mana_left_it_glows_for_its_controller_only_and_ending_the_turn_pays_out() {
            glows_with_mana_left(false);
        }

        #[test]
        fn r662_base_with_no_mana_left_it_does_not_glow_and_ending_the_turn_pays_nothing() {
            no_glow_without_mana(false);
        }

        #[test]
        fn r662_base_on_the_opponent_s_turn_it_reads_the_opponent_s_mana() {
            reads_the_opponent_s_mana(false);
        }

        #[test]
        fn r662_radiant_with_mana_left_it_glows_for_its_controller_only_and_ending_the_turn_pays_out() {
            glows_with_mana_left(true);
        }

        #[test]
        fn r662_radiant_with_no_mana_left_it_does_not_glow_and_ending_the_turn_pays_nothing() {
            no_glow_without_mana(true);
        }

        #[test]
        fn r662_radiant_on_the_opponent_s_turn_it_reads_the_opponent_s_mana() {
            reads_the_opponent_s_mana(true);
        }

        #[test]
        fn r662_in_hand_it_never_glows_its_condition_is_the_field_s() {
            crate::register_all();
            let s = scenario(json!({ "seed": "r662-018-hand", "p1": { "hand": ["core-018", "core-010"], "mana": 3 } }));
            let id = s.card("core-018").id.clone();
            assert!(!hand_glows(&s, &id, PlayerId::P1));
        }
    }
}
