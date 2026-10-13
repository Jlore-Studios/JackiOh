//! #65 Masochism Mask (SPEC §8.3, §6.2, §10.6, R18, R62).
//!
//! Base: "Start of turn: choose one: exile the bottom card of your library, lose 3 health, or summon
//! a Spikey Pillow". Radiant: "Choose twice from: nothing, exile bottom, lose 3, summon Spikey
//! Pillow" — a restated clause, so it replaces the base's single pick (§8 Conventions); the
//! Quickdraw tag and the start-of-turn timing are kept.
//!
//! §8.3's Engine cell: "Two sequential pending choices; 'lose' is not damage". §6.2: Quickdraw is
//! the `quickdraw` static flag, which `setup.rs` reads. §10.6, §10.1: a choice made during
//! resolution is a `PendingChoice`, so each face's `start_of_turn` opens one mode prompt whose answer
//! re-enters a named `resume` step, and the radiant first step applies its pick, then opens the
//! second prompt. The prompt effect is LAST in every list, correct under `apply_resumable` (parks a
//! tail) and `resolve::apply_effects` (does not; `turn.rs` runs start-of-turn hooks with it).
//! R18: "lose 3 health" is not damage: `lose_health` skips the §4.4 pipeline. R62: start-of-turn
//! triggers run before the draw, so "the bottom card" is the bottom before this turn's draw.

pub const ID: &str = "core-065";

// #65 Masochism Mask — SPEC §8.3, §10.6, BUILD M4-T4 row 65.
//
// Must-pass: "In opening hand (Quickdraw); start-of-turn mode prompt; 'lose 3' ignores armor;
// radiant two picks including 'nothing'."
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const MASK: &str = "core-065"; // Field Spell, 2, Quickdraw
    const PILLOW: &str = "core-065-1"; // #65.1, the token the third option summons
    const MENACE: &str = "core-019"; // library filler: the top card, so the start-of-turn draw is known
    const TIMMY: &str = "core-011";
    const POSTDOC: &str = "core-061"; // the bottom card of the library in the exile test

    // The prompt options as §8.3's two cells word them. They travel to the client in
    // `state.pending.options` (a prompt, not a declared mode, R81), so this is where they are pinned.
    const EXILE_BOTTOM: &str = "exile bottom";
    const LOSE_THREE: &str = "lose 3";
    const SUMMON_PILLOW: &str = "summon Spikey Pillow";
    const NOTHING: &str = "nothing";

    /// A library whose top is #19 and whose BOTTOM is #61 (`library[0]` is the top).
    const LIBRARY: [&str; 3] = [MENACE, TIMMY, POSTDOC];

    fn unit_at(s: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
        match s.unit(player, lane) {
            Some(found) => found,
            None => panic!("expected a unit in {player} lane {lane}, found none"),
        }
    }

    fn option_labels(s: &Scenario) -> Vec<String> {
        s.state()
            .pending
            .as_ref()
            .map(|pending| pending.options.iter().map(|option| option.label.clone()).collect())
            .unwrap_or_default()
    }

    fn mask_scenario(face_radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "backrow": [{ "def": MASK, "radiant": face_radiant }], "library": LIBRARY, "armor": 5 },
            "p2": { "hand": [MENACE], "field": [MENACE] },
        }))
    }

    #[test]
    fn r386_an_upgrade_offers_lose_2_and_a_degrade_lose_4_and_the_answer_loses_that() {
        for (upgrade, loss) in [(true, 2), (false, 4)] {
            crate::register_all();
            let mut s = mask_scenario(false);
            let moved = if upgrade {
                crate::upgrade_number(&mut s, MASK, "loss")
            } else {
                crate::degrade_number(&mut s, MASK, "loss")
            };
            assert_eq!(moved, loss);
            s.start_turn();
            let lose = format!("lose {loss}");
            assert_eq!(option_labels(&s), [EXILE_BOTTOM.to_string(), lose.clone(), SUMMON_PILLOW.to_string()]);
            s.answer(json!(lose));
            s.expect_health(P1, 30 - loss);
        }
    }

    mod masochism_mask {
        use super::*;

        #[test]
        fn s6_2_quickdraw_both_faces_carry_the_flag_that_starts_the_card_in_the_opening_hand() {
            // `setup.rs` reads the flag and swaps one opening draw for it; that placement is the engine's
            // own setup test. What this card owes is the flag, on both faces.
            crate::register_all();
            let faces = crate::scripts_of()[ID].clone();
            assert_eq!(faces.base.static_flags.and_then(|flags| flags.quickdraw), Some(true));
            assert_eq!(faces.radiant.static_flags.and_then(|flags| flags.quickdraw), Some(true));
        }

        #[test]
        fn s10_6_the_start_of_your_turn_opens_a_mode_prompt_with_the_three_options() {
            crate::register_all();
            let mut s = mask_scenario(false);
            s.start_turn();

            let pending = s.state().pending.clone();
            assert!(pending.is_some());
            assert_eq!(pending.as_ref().map(|p| p.kind), Some(PromptKind::Mode));
            assert_eq!(pending.as_ref().map(|p| p.player_id), Some(P1));
            assert_eq!(pending.as_ref().map(|p| p.min), Some(1));
            assert_eq!(pending.as_ref().map(|p| p.max), Some(1));
            assert_eq!(option_labels(&s), [EXILE_BOTTOM, LOSE_THREE, SUMMON_PILLOW]);
            s.expect_events(json!(["turnStarted", "promptOpened"]));
        }

        #[test]
        fn s6_2_it_does_not_ask_on_the_opponent_s_turn() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [MASK], "library": LIBRARY },
                "p2": { "library": LIBRARY, "field": [MENACE] },
            }));
            s.start_turn();

            assert!(s.state().pending.is_none());
        }

        #[test]
        fn r18_losing_3_health_is_not_damage_so_armor_does_not_reduce_it() {
            crate::register_all();
            let mut s = mask_scenario(false);
            s.start_turn().answer(json!(LOSE_THREE));

            s.expect_health(P1, 27);
            // The Armor is still there: nothing was spent on a non-damage loss.
            assert_eq!(s.state().players.p1.hero.armor, 5);
            s.expect_events(json!(["promptAnswered", "healthLost"]));
            assert!(!s.events().iter().any(|event| event.event_type().as_str() == "damaged"));
        }

        #[test]
        fn exiles_the_bottom_card_of_your_library() {
            crate::register_all();
            let mut s = mask_scenario(false);
            s.start_turn().answer(json!(EXILE_BOTTOM));

            s.expect_in_zone(POSTDOC, "exile");
            // The draw took the top card, so the two are never the same card.
            s.expect_in_zone(MENACE, "hand");
            assert!(!s.pile(P1, "library").iter().any(|card| card.def_id == POSTDOC));
        }

        #[test]
        fn summons_a_spikey_pillow() {
            crate::register_all();
            let mut s = mask_scenario(false);
            s.start_turn().answer(json!(SUMMON_PILLOW));

            let pillow = unit_at(&s, P1, 1);
            assert_eq!(pillow.def_id, PILLOW);
            s.expect_stats(&pillow, json!({ "attack": 0, "maxHealth": 2 }));
        }

        #[test]
        fn s10_6_radiant_asks_twice_with_nothing_among_the_four_options() {
            crate::register_all();
            let mut s = mask_scenario(true);
            s.start_turn();

            assert_eq!(option_labels(&s), [NOTHING, EXILE_BOTTOM, LOSE_THREE, SUMMON_PILLOW]);

            s.answer(json!(NOTHING));

            // The first step opened the second prompt, so a second pick is pending.
            assert!(s.state().pending.is_some());
            assert_eq!(option_labels(&s), [NOTHING, EXILE_BOTTOM, LOSE_THREE, SUMMON_PILLOW]);

            s.answer(json!(NOTHING));

            assert!(s.state().pending.is_none());
            s.expect_health(P1, 30);
            assert!(s.unit(P1, 1).is_none());
            s.expect_in_zone(POSTDOC, "library");
        }

        #[test]
        fn s10_6_radiant_may_pick_the_same_option_twice_two_spikey_pillows() {
            crate::register_all();
            let mut s = mask_scenario(true);
            s.start_turn().answer(json!(SUMMON_PILLOW)).answer(json!(SUMMON_PILLOW));

            assert!(s.state().pending.is_none());
            assert_eq!(unit_at(&s, P1, 1).def_id, PILLOW);
            assert_eq!(unit_at(&s, P1, 2).def_id, PILLOW);
        }

        #[test]
        fn s10_6_radiant_may_pick_the_same_option_twice_6_health() {
            crate::register_all();
            let mut s = mask_scenario(true);
            s.start_turn().answer(json!(LOSE_THREE)).answer(json!(LOSE_THREE));

            s.expect_health(P1, 24);
            assert_eq!(s.state().players.p1.hero.armor, 5);
        }

        #[test]
        fn s10_6_radiant_resumes_the_chain_the_first_pick_resolves_before_the_second_prompt_opens() {
            crate::register_all();
            let mut s = mask_scenario(true);
            s.start_turn().answer(json!(LOSE_THREE));

            // The step applied its own effect and then opened the next prompt, in that order.
            s.expect_health(P1, 27);
            assert!(s.state().pending.is_some());

            s.answer(json!(SUMMON_PILLOW));

            s.expect_health(P1, 27);
            assert_eq!(unit_at(&s, P1, 1).def_id, PILLOW);
            s.expect_events(json!(["promptOpened", "healthLost", "promptOpened", "summoned"]));
        }

        #[test]
        fn s10_6_radiant_s_two_picks_may_differ_in_the_other_order_too() {
            crate::register_all();
            let mut s = mask_scenario(true);
            s.start_turn().answer(json!(EXILE_BOTTOM)).answer(json!(LOSE_THREE));

            s.expect_in_zone(POSTDOC, "exile");
            s.expect_health(P1, 27);
        }
    }
}
