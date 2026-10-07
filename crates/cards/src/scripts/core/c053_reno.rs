//! #53 Reno (SPEC §8.3, §6.3 Heal, §3). Unit 4/6 → 8/12, Human, cost 3, Common.
//!   Base:    "Cry: if your hero is below 30, set it to 30"
//!   Radiant: "60" — §8 Conventions: "a cell that changes only a number changes only that number",
//!            so the radiant face is the same clause with 30 replaced by 60, not a second heal.
//!
//! The Engine cell is `health = max(health, 30)`, which is §6.3's "Heal up to 30" reading of Heal:
//! `heal({ target, upTo })`. On a hero that is `heal_hero_up_to` (engine/src/damage.rs), and it is a
//! floor, never a ceiling — "if (hero.health >= floor) return 0", so a hero at 35 is left at 35 and
//! no `healed` event is emitted for it. §3 gives a hero no maximum health, so nothing caps the 30
//! or the 60 either; a hero already above the floor simply has nothing happen (§6.3, R19).
//!
//! R19 is why this is a legal target at all ("a heal may name any unit or hero"), and the target is
//! named, not chosen: "your hero" is `{ of: "selfHero" }`, the controller's hero (§8 Conventions,
//! "'Your' means the controller"), so there is no prompt and no play-time declaration here (R81).
//!
//! R195, the yellow glow: in hand, Reno glows exactly when its Cry would raise the hero, which is the
//! clause's own "if your hero is below 30" (60 radiant). `below_floor` is that one predicate, read by
//! both the Cry and `condition_met`, so the glow and the heal cannot disagree. Gating the Cry on it
//! changes nothing on the board: `heal_hero_up_to` already leaves a hero at or above the floor alone
//! and emits no `healed` event for it. The condition is about the play, so a Reno on the field never
//! glows.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-053";

/// The floor each face raises the hero to. §8: base 30, radiant 60.
const BASE_FLOOR: i32 = 30;
const RADIANT_FLOOR: i32 = 60;

/// "If your hero is below `floor`": strictly below, read through the engine's `hero_of`.
fn below_floor(state: &GameState, controller: PlayerId, floor: i32) -> bool {
    hero_of(state, controller).health < floor
}

/// The two faces differ only in the number, so one builder writes both (§8 Conventions).
fn reno(floor: i32) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            if below_floor(ctx.state, ctx.controller, floor) {
                vec![heal(json_as(json!({ "target": { "of": "selfHero" }, "upTo": floor })))]
            } else {
                vec![]
            }
        })),
        // R195: hand only — the glow asks whether playing Reno now would set the hero to the floor.
        condition_met: Some(condition_hook(move |c| {
            c.zone == ConditionZone::Hand && below_floor(c.state, c.controller, floor)
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: reno(BASE_FLOOR),
        radiant: reno(RADIANT_FLOOR),
    }
}

// #53 Reno (SPEC §8.3, §6.3 Heal, §3, R19; BUILD M4-T4 row 53: "12 → 30; 35 stays 35; radiant 60").
//
// R195's yellow glow (`condition_met`): both answers of this card's hook, checked against the branch
// its resolution then takes, are in condition_active.rs with the other hooked cards (README §5).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    /// The number of `healed` events in the most recent step.
    fn heals_in_last_step(s: &Scenario) -> usize {
        s.last_events().iter().filter(|event| event.event_type().as_str() == "healed").count()
    }

    mod reno_base {
        use super::*;

        #[test]
        fn s6_3_raises_a_hero_at_12_to_30() {
            let mut s = scenario(json!({ "p1": { "hand": ["core-053"], "health": 12 } }));

            s.play("core-053", json!({}));
            s.expect_health(PlayerId::P1, 30);
            s.expect_events(json!(["cardPlayed", "summoned", "healed"]));
        }

        #[test]
        fn s6_3_heal_up_to_30_is_a_floor_not_a_ceiling_a_hero_at_35_stays_35() {
            let mut s = scenario(json!({ "p1": { "hand": ["core-053"], "health": 35 } }));

            s.play("core-053", json!({}));
            s.expect_health(PlayerId::P1, 35);
            // Nothing was healed, so §3 never gave the hero a maximum to be pulled down to.
            assert_eq!(heals_in_last_step(&s), 0);
        }

        #[test]
        fn a_hero_exactly_at_30_is_untouched_and_29_goes_up_by_1() {
            let mut at_30 = scenario(json!({ "p1": { "hand": ["core-053"], "health": 30 } }));
            at_30.play("core-053", json!({}));
            at_30.expect_health(PlayerId::P1, 30);
            let mut at_29 = scenario(json!({ "p1": { "hand": ["core-053"], "health": 29 } }));
            at_29.play("core-053", json!({}));
            at_29.expect_health(PlayerId::P1, 30);
        }

        #[test]
        fn the_cry_is_gated_on_below_30_the_branch_its_yellow_glow_reports_at_exactly_30_no_heal_is_emitted() {
            let mut s = scenario(json!({ "p1": { "hand": ["core-053"], "health": 30 } }));
            let view = s.view(Some(PlayerId::P1));
            let HandView::Cards(hand) = &view.you.hand else {
                panic!("the viewer's own hand is a card list");
            };
            assert_eq!(hand.len(), 1);
            assert_ne!(hand[0].condition_active, Some(true));

            s.play("core-053", json!({}));
            s.expect_health(PlayerId::P1, 30);
            assert_eq!(heals_in_last_step(&s), 0);
        }

        #[test]
        fn s8_conventions_your_means_the_controller_the_enemy_hero_is_not_raised() {
            let mut s = scenario(json!({ "p1": { "hand": ["core-053"], "health": 12 }, "p2": { "health": 12 } }));

            s.play("core-053", json!({}));
            s.expect_health(PlayerId::P1, 30);
            s.expect_health(PlayerId::P2, 12);
        }

        #[test]
        fn the_unit_still_enters_the_field_with_its_printed_4_6_s10_4_layer_1() {
            let mut s = scenario(json!({ "p1": { "hand": ["core-053"], "health": 12 } }));

            s.play("core-053", json!({}));
            s.expect_in_zone("core-053", "field");
            s.expect_stats("core-053", json!({ "attack": 4, "maxHealth": 6 }));
        }
    }

    // The radiant cases: TS wrote the flag onto the hand card after setup (`s.card(…).radiant = true`,
    // a "HARNESS GAP" note from before the harness took `{ def, radiant }` in a hand). The scenario's
    // hand entry `{ "def": …, "radiant": true }` builds the same state — the harness creates the hand
    // card and then sets the flag, as the TS line did — without a mutable card reference.
    mod reno_radiant {
        use super::*;

        fn radiant_reno_in_hand(health: i32) -> Scenario {
            scenario(json!({ "p1": { "hand": [{ "def": "core-053", "radiant": true }], "health": health } }))
        }

        #[test]
        fn s8_conventions_only_the_number_changes_so_35_becomes_60() {
            let mut s = radiant_reno_in_hand(35);

            s.play("core-053", json!({}));
            s.expect_health(PlayerId::P1, 60);
        }

        #[test]
        fn raises_a_hero_at_12_straight_to_60_in_one_heal() {
            let mut s = radiant_reno_in_hand(12);

            s.play("core-053", json!({}));
            s.expect_health(PlayerId::P1, 60);
            assert_eq!(heals_in_last_step(&s), 1);
        }

        #[test]
        fn r19_a_hero_already_above_60_keeps_its_health_70_stays_70() {
            let mut s = radiant_reno_in_hand(70);

            s.play("core-053", json!({}));
            s.expect_health(PlayerId::P1, 70);
            assert_eq!(heals_in_last_step(&s), 0);
        }

        #[test]
        fn the_radiant_unit_enters_as_8_12_s5_2() {
            let mut s = radiant_reno_in_hand(12);

            s.play("core-053", json!({}));
            s.expect_stats("core-053", json!({ "attack": 8, "maxHealth": 12 }));
        }
    }
}
