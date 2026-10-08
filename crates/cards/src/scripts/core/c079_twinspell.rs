//! #79 Twinspell (SPEC §8.3, R30, R70, §2.2, §3.2, §6.2 Echo, §10.5 step 6).
//!
//! Base: "The next Spell you play gains Echo +1"; the radiant cell is "Echo +2", a cell that changes
//! only a number (§8 Conventions). The text has no "Cry:" (compare #73's "… Cry: draw 1"), and R169
//! says #79 installs its modifier "without a Cry": it is the Field Spell's lasting effect (§5.1,
//! R209), which the permanent has for as long as it stands on the field, however it got there — a
//! summon fires no Cry (§6.2), yet a Twinspell #22's Death summons still grants its Echo, and #85
//! fusing Twinspell onto another Field Spell hands the text to that permanent's controller.
//!
//! So the card is one static flag, `echoGrant`, and the engine does the rest
//! (`modifiers.installLastingModifiers`): an `echoNextSpell` rider on the player whose side the card
//! stands on, owned by the card (`sourceId`), moved with it when control changes and ended when it
//! leaves (R209); §10.5 step 4 has the next Spell take it and sends this card to its owner's
//! graveyard (R30, R178). It is not turn-scoped: §2.2 says so in as many words ("Twinspell's pending
//! Echo is not turn-scoped and survives cleanup"), so the rider is `{ until: "used" }`.
//!
//! §6.2's Echo row: "Play resolves, then the same instance re-resolves X times with fresh mode/target
//! prompts; Twinspell grants Echo +1 to the next spell." §10.5 step 6 is where that happens, and R70
//! adds that a CAST spell uses Twinspell's Echo even though it never uses a cost discount.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-079";

/// The two faces differ only in how many extra resolutions the next spell gets. The engine reads the
/// amount off the face the card wears when a Spell takes it, so a Twinspell made Radiant on the field
/// (#49 radiant) grants "Echo +2" from then on (§5.2, R209). The card declares the grant as `echoGain`
/// (R386), and the engine reads the flag through `params::declared_or`, so a Degrade or an Upgrade
/// moves it.
fn twinspell(amount: i32) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            echo_grant: Some(amount),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: twinspell(1),
        radiant: twinspell(2),
    }
}

// #79 Twinspell — SPEC §8.3, R30, §2.2, §6.2 Echo, §10.5 step 6.
//
// BUILD M4-T4: "Next spell echoes once (radiant twice); consumed to GY on use (R30); survives
// cleanup".
//
// The echo count is read through The Coin (§7), whose "Gain 1 mana this turn." makes each resolution
// a number: one extra resolution is +2 instead of +1, two extra is +3. (It was read through #78
// /fullsend's "gain 4 mana" until patch v0.1.1 made that a Refresh, which stops at max.)
// The "fresh prompts per repeat" half of §10.5 step 6 is read through #82 KY's Trial, whose every
// resolution opens a Discover (§10.6). It was read through #80 Zao Gao's chosen discard until patch
// v0.1.1 made that discard random (R354), leaving it no prompt to reopen.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const TWINSPELL: &str = "core-079";
    const COIN: &str = "core-t-coin";
    /// #82 KY's Trial, cost 1: "Discover among 3 distinct random numbers 1–100" (R247).
    const TRIAL: &str = "core-082";

    const LIBRARY: [&str; 4] = ["core-008", "core-008", "core-008", "core-008"];

    use crate::js;

    use crate::matches_object;

    /// Harness gap (reported): no `mods()` accessor, so the test reads `state` — B1.7 covers `src` only.
    fn echo_riders(s: &Scenario) -> Vec<Value> {
        s.state()
            .players
            .p1
            .mods
            .iter()
            .map(js)
            .filter(|modifier| modifier["kind"] == "echoNextSpell")
            .collect()
    }

    /// The first `echoNextSpell` rider, as TS's `mods.find(...)` reads it.
    fn first_echo_rider(s: &Scenario) -> Value {
        echo_riders(s).into_iter().next().expect("an echoNextSpell modifier")
    }

    /// Mana 4, so Twinspell (2) and The Coin (0) both fit on one turn.
    fn with_coin(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": TWINSPELL, "radiant": radiant }, COIN], "mana": 4, "library": LIBRARY },
            "p2": { "hand": ["core-005"], "field": ["core-019"], "library": LIBRARY }
        }))
    }

    /// Mana 4: Twinspell (2) and KY's Trial (1), with a spare card so nothing auto-ends.
    fn with_trial(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": TWINSPELL, "radiant": radiant }, TRIAL, "core-005"], "library": LIBRARY },
            "p2": { "hand": ["core-005"], "field": ["core-019"], "library": LIBRARY }
        }))
    }

    /// Answer the open Discover with its first option, and return the index it named.
    fn discover_first(s: &mut Scenario) -> String {
        let option = s
            .state()
            .pending
            .as_ref()
            .and_then(|pending| pending.options.first())
            .cloned();
        let Some(option) = option else {
            panic!("no Discover is open");
        };
        s.answer(json!([option.selection]));
        option.label
    }

    /// `s.state.pending?.kind`.
    fn pending_kind(s: &Scenario) -> Option<PromptKind> {
        s.state().pending.as_ref().map(|pending| pending.kind)
    }

    #[test]
    fn r386_an_upgrade_grants_echo_2_and_a_radiant_degrade_echo_1() {
        // The Coin's "gain 1 mana" once per resolution: 2 left after the Twinspell, plus 1 + the echoes.
        for (radiant, upgrade, echo) in [(false, true, 2), (true, false, 1)] {
            crate::register_all();
            let mut s = with_coin(radiant);
            let moved = if upgrade {
                crate::upgrade_number(&mut s, TWINSPELL, "echoGain")
            } else {
                crate::degrade_number(&mut s, TWINSPELL, "echoGain")
            };
            assert_eq!(moved, echo);
            s.play(TWINSPELL, json!({}));
            s.play(COIN, json!({}));
            s.expect_mana(P1, 2 + 1 + echo);
        }
    }

    mod n79_twinspell_base {
        use super::*;

        #[test]
        fn r30_installs_an_until_used_echo_1_rider_naming_its_own_instance() {
            crate::register_all();
            let mut s = with_coin(false);
            let twin = s.card(TWINSPELL).clone();

            s.play(TWINSPELL, json!({}));

            // A Field Spell with a Cry is ordinary (#73): it enters the backrow and the Cry fires there.
            s.expect_in_zone(&twin, "field");
            assert_eq!(echo_riders(&s).len(), 1);
            let rider = first_echo_rider(&s);
            assert!(
                matches_object(
                    &rider,
                    &json!({
                        "kind": "echoNextSpell",
                        "amount": 1,
                        // §2.2: not turn-scoped, so it waits for a spell rather than for cleanup.
                        "expiry": { "until": "used" },
                        // R30: the engine needs to know which card to bury when the rider is consumed.
                        "sourceId": twin.id
                    })
                ),
                "{rider}"
            );
        }

        #[test]
        fn s2_2_the_pending_echo_survives_end_of_turn_cleanup() {
            crate::register_all();
            let mut s = with_coin(false);
            s.play(TWINSPELL, json!({}));

            s.end_turn();

            // §2.2 says so in as many words: "Twinspell's pending Echo is not turn-scoped and survives
            // cleanup." BUILD M4-T2's acceptance row says the same of a "this turn" discount's opposite.
            assert_eq!(echo_riders(&s).len(), 1);
            s.expect_in_zone(TWINSPELL, "field");
        }

        #[test]
        fn s10_5_step_6_the_next_spell_resolves_one_extra_time() {
            crate::register_all();
            let mut s = with_coin(false);
            s.play(TWINSPELL, json!({})); // 4 − 2 = 2

            s.play(COIN, json!({})); // 0, then 1 mana per resolution

            // Two resolutions of "gain 1 mana": 2 + 2 = 4 rather than 2 + 1 = 3.
            s.expect_mana(P1, 4);
        }

        #[test]
        fn r30_twinspell_is_consumed_to_the_graveyard_when_it_applies_and_the_rider_is_gone() {
            crate::register_all();
            let mut s = with_coin(false);
            let twin = s.card(TWINSPELL).clone();
            s.play(TWINSPELL, json!({}));

            s.play(COIN, json!({}));

            s.expect_in_zone(&twin, "graveyard");
            assert_eq!(echo_riders(&s).len(), 0);
        }

        #[test]
        fn r30_only_the_next_spell_echoes_a_second_spell_resolves_once() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [TWINSPELL, COIN, COIN], "mana": 4, "library": LIBRARY },
                "p2": { "hand": ["core-005"], "field": ["core-019"], "library": LIBRARY }
            }));
            s.play(TWINSPELL, json!({})); // 4 − 2 = 2
            let coins: Vec<CardInstance> = s.hand(P1).into_iter().filter(|card| card.def_id == COIN).collect();
            let (Some(first), Some(second)) = (coins.first().cloned(), coins.get(1).cloned()) else {
                panic!("setup: two Coins in hand");
            };
            s.play(&first, json!({})); // +1 per resolution, twice: 4
            s.expect_mana(P1, 4);

            s.play(&second, json!({})); // +1 once: 5

            s.expect_mana(P1, 5);
            // The same fact without the arithmetic: the rider was spent on the first spell, so this one
            // carries no Echo and has exactly one resolution to show for itself (§10.5 step 6).
            assert_eq!(
                s.last_events()
                    .iter()
                    .filter(|event| event.event_type() == GameEventType::CardResolved)
                    .count(),
                1
            );
            assert_eq!(echo_riders(&s).len(), 0);
        }

        #[test]
        fn s10_5_step_6_an_echoed_prompting_spell_reopens_its_prompt_on_the_repeat() {
            crate::register_all();
            let mut s = with_trial(false);
            s.play(TWINSPELL, json!({}));
            let before = s.hand(P1).len();

            s.play(TRIAL, json!({}));

            // First resolution's Discover (§10.6).
            assert_eq!(pending_kind(&s), Some(PromptKind::Discover));
            discover_first(&mut s);
            // The repeat asks again: a prompt of its own, not the first one's answer carried over.
            assert_eq!(pending_kind(&s), Some(PromptKind::Discover));
            discover_first(&mut s);

            assert!(s.state().pending.is_none());
            // Two resolutions × one card each, for the Trial that left the hand.
            assert_eq!(s.hand(P1).len(), before - 1 + 2);
        }
    }

    mod n79_twinspell_radiant {
        use super::*;

        #[test]
        fn r30_radiant_installs_echo_2() {
            crate::register_all();
            let mut s = with_coin(true);
            let twin = s.card(TWINSPELL).clone();

            s.play(TWINSPELL, json!({}));

            let rider = first_echo_rider(&s);
            assert!(
                matches_object(
                    &rider,
                    &json!({
                        "kind": "echoNextSpell",
                        "amount": 2,
                        "expiry": { "until": "used" },
                        "sourceId": twin.id
                    })
                ),
                "{rider}"
            );
        }

        #[test]
        fn s10_5_step_6_radiant_makes_the_next_spell_resolve_twice_more() {
            crate::register_all();
            let mut s = with_coin(true);
            s.play(TWINSPELL, json!({})); // 4 − 2 = 2

            s.play(COIN, json!({})); // three resolutions of +1

            s.expect_mana(P1, 5);
        }

        #[test]
        fn r30_radiant_is_consumed_to_the_graveyard_on_use_and_survives_cleanup_until_then() {
            crate::register_all();
            let mut s = with_coin(true);
            let twin = s.card(TWINSPELL).clone();
            s.play(TWINSPELL, json!({}));

            s.end_turn();
            assert_eq!(echo_riders(&s).len(), 1);
            s.expect_in_zone(&twin, "field");

            // Back around to p1's turn, where the waiting rider is spent.
            s.end_turn();
            s.play(COIN, json!({}));

            s.expect_in_zone(&twin, "graveyard");
            assert_eq!(echo_riders(&s).len(), 0);
        }

        #[test]
        fn s10_5_step_6_radiant_reopens_a_prompting_spell_s_prompt_on_both_repeats() {
            crate::register_all();
            let mut s = with_trial(true);
            s.play(TWINSPELL, json!({}));
            let before = s.hand(P1).len();

            s.play(TRIAL, json!({}));

            for _resolution in 0..3 {
                assert_eq!(pending_kind(&s), Some(PromptKind::Discover));
                discover_first(&mut s);
            }

            assert!(s.state().pending.is_none());
            assert_eq!(s.hand(P1).len(), before - 1 + 3);
        }
    }
}
