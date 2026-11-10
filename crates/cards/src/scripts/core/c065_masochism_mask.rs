//! #65 Masochism Mask (SPEC §8.3, §6.2, §10.6, R18, R62, R3139001).
//!
//! Base: "Start of turn: choose one: exile the bottom card of your library, lose 3 health, or summon
//! a Spikey Pillow. After you take {downsides} downsides from this, make your hand Radiant". Radiant:
//! "Choose twice from: nothing, exile bottom, lose 3, summon Spikey Pillow" — a restated clause, so it
//! replaces the base's single pick (§8 Conventions); the Quickdraw tag, the start-of-turn timing and the
//! last sentence are kept on both faces.
//!
//! A downside is a pick of the Mask's own Start-of-turn Choose that is not "nothing": exile the bottom card,
//! lose health, summon a Spikey Pillow (R3139001). A count in the card's memory (`remember`, key
//! `DOWNSIDES`) rises by one per downside, after the downside's own effect; at the `downsides`th the whole
//! hand becomes Radiant (`make_hand_radiant`, the Call to Chaos effect, §6.3 Make Radiant) and the count
//! restarts at 0, so it repeats every `downsides`. The Radiant face's two picks count one by one and its
//! "nothing" counts none; the opponent's turn asks nothing, so counts none. The count lives on the
//! instance, so it goes when the Mask leaves the field (R78). `script_v0_5_11o` keeps the Mask without it.
//!
//! §8.3's Engine cell: "Two sequential pending choices; 'lose' is not damage". §6.2: Quickdraw is
//! the `quickdraw` static flag. §10.6, §10.1: each prompt is a `PendingChoice` re-entering a named
//! `resume` step. The prompt effect is LAST in every list, correct under `apply_resumable` (parks a
//! tail) and `resolve::apply_effects` (does not; `turn.rs` runs start-of-turn hooks with it).
//! R18: "lose 3 health" is not damage: `lose_health` skips the §4.4 pipeline. R62: start-of-turn
//! triggers run before the draw, so "the bottom card" is the bottom before this turn's draw.

use jackioh_engine::effects::{choose_mode, chosen_options, exile_bottom_of_library, lose_health, remember, summon};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-065";

/// #65.1, the token this card defines (§7, §8.3).
const SPIKEY_PILLOW: &str = "core-065-1";

// The option names in §8.3's order: prompt options, not declared modes (R81), so they reach the
// client through `state.pending.options`.
const EXILE_BOTTOM: &str = "exile bottom";
const SUMMON_PILLOW: &str = "summon Spikey Pillow";
const NOTHING: &str = "nothing";

/// The Mask's memory of the downsides taken since the hand last became Radiant (R3139001).
const DOWNSIDES: &str = "downsides";

/// "lose 3": the declared number `loss` (R386) as the card stands when the prompt opens.
fn lose_option(ctx: &EffectContext<'_>) -> String {
    format!("lose {}", param(ctx, "loss"))
}

fn base_options(ctx: &EffectContext<'_>) -> Vec<String> {
    vec![EXILE_BOTTOM.to_string(), lose_option(ctx), SUMMON_PILLOW.to_string()]
}

fn radiant_options(ctx: &EffectContext<'_>) -> Vec<String> {
    vec![NOTHING.to_string(), EXILE_BOTTOM.to_string(), lose_option(ctx), SUMMON_PILLOW.to_string()]
}

/// The steps of the `resume` table (§10.6); the radiant face uses both.
const STEP_FIRST: &str = "firstPick";
const STEP_SECOND: &str = "secondPick";

/// What one answered option does. "nothing" and an answer that named no option are both the empty
/// list: the trigger still fired and the card stays on the field.
fn effects_for(ctx: &EffectContext<'_>, option: Option<&str>) -> Vec<Effect> {
    match option {
        Some(EXILE_BOTTOM) => vec![exile_bottom_of_library(json_as(json!({ "player": "self" })))],
        Some(SUMMON_PILLOW) => vec![summon(json_as(json!({ "defId": SPIKEY_PILLOW })))],
        // R18: lost health, not damage.
        Some(picked) if picked == lose_option(ctx) => {
            vec![lose_health(json_as(json!({ "player": "self", "amount": param(ctx, "loss") })))]
        }
        _ => vec![],
    }
}

/// One pick, answered: its effect, then, for a downside, the count and, at the `downsides`th, the whole
/// hand made Radiant and the count back to 0 (R3139001). A downside is exactly a pick with an effect:
/// "nothing" and a pick that named no option have none and count none.
fn after_pick(ctx: &EffectContext<'_>, option: Option<&str>, counting: bool) -> Vec<Effect> {
    let mut effects = effects_for(ctx, option);
    if !counting || effects.is_empty() {
        return effects;
    }
    let taken = recalled(ctx, DOWNSIDES).and_then(|stored| stored.as_i64()).map_or(0, |n| n as i32) + 1;
    if taken >= param(ctx, "downsides") {
        effects.push(subsystems::make_hand_radiant());
        effects.push(remember(json_as(json!({ "key": DOWNSIDES, "value": 0 }))));
    } else {
        effects.push(remember(json_as(json!({ "key": DOWNSIDES, "value": taken }))));
    }
    effects
}

/// The answered pick: §10.6 delivers a mode selection in `ctx.targets`, which is what this reads.
fn pick_of(ctx: &EffectContext) -> Option<String> {
    chosen_options(ctx).into_iter().next()
}

/// `counting`: whether the downsides are counted toward the Radiant hand, since issue #3139.
fn base(counting: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        start_of_turn: Some(hook(|ctx| {
            vec![choose_mode(json_as(json!({
                "options": base_options(ctx),
                "step": STEP_FIRST,
                "prompt": "Masochism Mask: choose one",
            })))]
        })),
        resume: IndexMap::from([(
            STEP_FIRST,
            hook(move |ctx| {
                let picked = pick_of(ctx);
                after_pick(ctx, picked.as_deref(), counting)
            }),
        )]),
        ..Script::default()
    }
}

fn radiant(counting: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        start_of_turn: Some(hook(|ctx| {
            vec![choose_mode(json_as(json!({
                "options": radiant_options(ctx),
                "step": STEP_FIRST,
                "prompt": "Masochism Mask: choose twice (1 of 2)",
            })))]
        })),
        resume: IndexMap::from([
            // The first pick resolves, then the second prompt opens; the same option may be picked
            // again, as the "picked twice" check in `prompts.rs` is per-prompt.
            (
                STEP_FIRST,
                hook(move |ctx| {
                    let picked = pick_of(ctx);
                    let mut effects = after_pick(ctx, picked.as_deref(), counting);
                    effects.push(choose_mode(json_as(json!({
                        "options": radiant_options(ctx),
                        "step": STEP_SECOND,
                        "prompt": "Masochism Mask: choose twice (2 of 2)",
                    }))));
                    effects
                }),
            ),
            (
                STEP_SECOND,
                hook(move |ctx| {
                    let picked = pick_of(ctx);
                    after_pick(ctx, picked.as_deref(), counting)
                }),
            ),
        ]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: base(true),
        radiant: radiant(true),
    }
}

/// R767000: the script v0.5.11r and the versions before it ran, with no count of downsides; `KEPT_SCRIPTS`
/// folds those replays under it.
pub fn script_v0_5_11o() -> CardScripts {
    CardScripts {
        base: base(false),
        radiant: radiant(false),
    }
}

// #65 Masochism Mask — SPEC §8.3, §10.6, BUILD M4-T4 row 65, R3139001.
#[cfg(test)]
mod tests {
    use super::{script, script_v0_5_11o};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;

    const MASK: &str = "core-065"; // Masochism Mask: a Field Spell, Quickdraw
    const PILLOW: &str = "core-065-1"; // Spikey Pillow: #65.1, the token the third option summons
    const MENACE: &str = "core-019"; // Midrange Menace: library filler: the top card, so the start-of-turn draw is known
    const TIMMY: &str = "core-011";
    const POSTDOC: &str = "core-061"; // Prejudiced Postdoc: the bottom card of the library in the exile test

    // The prompt options as §8.3 words them, pinned where the client reads them (R81).
    const EXILE_BOTTOM: &str = "exile bottom";
    const LOSE_THREE: &str = "lose 3";
    const SUMMON_PILLOW: &str = "summon Spikey Pillow";
    const NOTHING: &str = "nothing";

    /// A library whose top is #19 and whose BOTTOM is #61 (`library[0]` is the top).
    const LIBRARY: [&str; 3] = [MENACE, TIMMY, POSTDOC];

    fn option_labels(s: &Scenario) -> Vec<String> {
        s.state()
            .pending
            .as_ref()
            .map(|pending| pending.options.iter().map(|option| option.label.clone()).collect())
            .unwrap_or_default()
    }

    /// p1's hand, one flag per card: whether it is Radiant.
    fn radiance(s: &Scenario) -> Vec<bool> {
        s.hand(P1).iter().map(|card| card.radiant).collect()
    }

    /// p1's hand as the Mask's trigger saw it: without the card the turn's draw added after it (R62, the
    /// start-of-turn triggers run before the draw), the last one.
    fn held_before_the_draw(s: &Scenario) -> Vec<bool> {
        let mut flags = radiance(s);
        flags.pop();
        flags
    }

    /// The downsides the Mask has counted since the hand last became Radiant (0 before the first).
    fn counted(s: &Scenario) -> i64 {
        s.card(MASK).memory.get("downsides").and_then(Value::as_i64).unwrap_or(0)
    }

    /// A Mask with `taken` downsides already counted, a bare hand to make Radiant and a deep library.
    fn counting(face_radiant: bool, taken: i64) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "hand": [MENACE, TIMMY],
                "backrow": [{ "def": MASK, "radiant": face_radiant }],
                "library": crate::filler(30),
                "armor": 5,
            },
            "p2": { "hand": [MENACE], "field": [MENACE] },
        }));
        s.card_mut(MASK).memory.insert("downsides".to_string(), json!(taken));
        s
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
            // The placement is `setup.rs`'s own test; what this card owes is the flag, on both faces.
            crate::register_all();
            let faces = script();
            assert_eq!(faces.base.static_flags.and_then(|flags| flags.quickdraw), Some(true));
            assert_eq!(faces.radiant.static_flags.and_then(|flags| flags.quickdraw), Some(true));
        }

        #[test]
        fn s10_6_the_start_of_your_turn_opens_a_mode_prompt_with_the_three_options() {
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
            let mut s = mask_scenario(false);
            s.start_turn().answer(json!(LOSE_THREE));

            s.expect_health(P1, 27);
            assert_eq!(s.state().players.p1.hero.armor, 5);
            s.expect_events(json!(["promptAnswered", "healthLost"]));
            assert!(!s.events().iter().any(|event| event.event_type().as_str() == "damaged"));
        }

        #[test]
        fn exiles_the_bottom_card_of_your_library() {
            let mut s = mask_scenario(false);
            s.start_turn().answer(json!(EXILE_BOTTOM));

            s.expect_in_zone(POSTDOC, "exile");
            // The draw took the top card, so the two are never the same card.
            s.expect_in_zone(MENACE, "hand");
            assert!(!s.pile(P1, "library").iter().any(|card| card.def_id == POSTDOC));
        }

        #[test]
        fn summons_a_spikey_pillow() {
            let mut s = mask_scenario(false);
            s.start_turn().answer(json!(SUMMON_PILLOW));

            let pillow = s.unit_at(P1, 1);
            assert_eq!(pillow.def_id, PILLOW);
            s.expect_stats(&pillow, json!({ "attack": 0, "maxHealth": 2 }));
        }

        #[test]
        fn s10_6_radiant_asks_twice_with_nothing_among_the_four_options() {
            let mut s = mask_scenario(true);
            s.start_turn();

            assert_eq!(option_labels(&s), [NOTHING, EXILE_BOTTOM, LOSE_THREE, SUMMON_PILLOW]);

            s.answer(json!(NOTHING));

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
            let mut s = mask_scenario(true);
            s.start_turn().answer(json!(SUMMON_PILLOW)).answer(json!(SUMMON_PILLOW));

            assert!(s.state().pending.is_none());
            assert_eq!(s.unit_at(P1, 1).def_id, PILLOW);
            assert_eq!(s.unit_at(P1, 2).def_id, PILLOW);
        }

        #[test]
        fn s10_6_radiant_may_pick_the_same_option_twice_6_health() {
            let mut s = mask_scenario(true);
            s.start_turn().answer(json!(LOSE_THREE)).answer(json!(LOSE_THREE));

            s.expect_health(P1, 24);
            assert_eq!(s.state().players.p1.hero.armor, 5);
        }

        #[test]
        fn s10_6_radiant_resumes_the_chain_the_first_pick_resolves_before_the_second_prompt_opens() {
            let mut s = mask_scenario(true);
            s.start_turn().answer(json!(LOSE_THREE));

            s.expect_health(P1, 27);
            assert!(s.state().pending.is_some());

            s.answer(json!(SUMMON_PILLOW));

            s.expect_health(P1, 27);
            assert_eq!(s.unit_at(P1, 1).def_id, PILLOW);
            s.expect_events(json!(["promptOpened", "healthLost", "promptOpened", "summoned"]));
        }

        #[test]
        fn s10_6_radiant_s_two_picks_may_differ_in_the_other_order_too() {
            let mut s = mask_scenario(true);
            s.start_turn().answer(json!(EXILE_BOTTOM)).answer(json!(LOSE_THREE));

            s.expect_in_zone(POSTDOC, "exile");
            s.expect_health(P1, 27);
        }

        // Issue #3139, R3139001: "After you take {downsides} downsides from this, make your hand Radiant."

        #[test]
        fn r3139001_the_sixth_downside_makes_the_whole_hand_radiant_and_the_count_restarts() {
            let mut s = counting(false, 0);
            for n in 1..=5 {
                s.start_turn().answer(json!(LOSE_THREE));
                assert_eq!(counted(&s), n, "after downside {n}");
                assert!(!radiance(&s).iter().any(|&radiant| radiant), "after downside {n}: no card is Radiant yet");
            }

            s.start_turn().answer(json!(LOSE_THREE));

            // The 6th: every card in the hand, the five drawn since the start included, and the count back to 0.
            // The turn's own draw comes after the trigger (R62), so that card is plain.
            assert_eq!(radiance(&s).len(), 2 + 6);
            assert!(held_before_the_draw(&s).iter().all(|&radiant| radiant), "{:?}", radiance(&s));
            assert_eq!(radiance(&s).iter().filter(|&&radiant| !radiant).count(), 1);
            assert_eq!(counted(&s), 0);
            s.expect_health(P1, 30 - 6 * 3);
            // The count starts over: the cards drawn after it are plain (the hand holds 10 at most, §2.4).
            for n in 1..=2 {
                s.start_turn().answer(json!(EXILE_BOTTOM));
                assert_eq!(counted(&s), n);
            }
            assert_eq!(radiance(&s).len(), 10);
            assert_eq!(radiance(&s).iter().filter(|&&radiant| !radiant).count(), 1 + 2, "the cards drawn since are plain");
            // It repeats every 6: four more downsides take it to the next 6th and the hand is Radiant again.
            for _ in 0..3 {
                s.start_turn().answer(json!(EXILE_BOTTOM));
            }
            assert_eq!(counted(&s), 5);
            s.start_turn().answer(json!(SUMMON_PILLOW));
            assert!(held_before_the_draw(&s).iter().all(|&radiant| radiant), "{:?}", radiance(&s));
            assert_eq!(counted(&s), 0);
        }

        /// Every pick that is not "nothing" is a downside: the exile, the lost health and the Pillow count alike.
        #[test]
        fn r3139001_each_of_the_three_downsides_counts_one() {
            for pick in [EXILE_BOTTOM, LOSE_THREE, SUMMON_PILLOW] {
                let mut s = counting(false, 4);
                s.start_turn().answer(json!(pick));
                assert_eq!(counted(&s), 5, "{pick}");
                assert!(!radiance(&s).iter().any(|&radiant| radiant), "{pick}");
                s.start_turn().answer(json!(pick));
                assert_eq!(counted(&s), 0, "{pick}");
                assert!(held_before_the_draw(&s).iter().all(|&radiant| radiant), "{pick}");
            }
        }

        /// The downside's own effect resolves too, in the pick that makes the hand Radiant.
        #[test]
        fn r3139001_the_sixth_downside_still_does_what_it_says() {
            let mut s = counting(false, 5);
            s.start_turn().answer(json!(LOSE_THREE));
            s.expect_health(P1, 27);
            assert!(held_before_the_draw(&s).iter().all(|&radiant| radiant));
        }

        #[test]
        fn r3139001_the_radiant_face_s_nothing_counts_none() {
            let mut s = counting(true, 0);
            s.start_turn().answer(json!(NOTHING)).answer(json!(NOTHING));
            assert_eq!(counted(&s), 0);

            // One downside and one nothing is one.
            s.start_turn().answer(json!(NOTHING)).answer(json!(LOSE_THREE));
            assert_eq!(counted(&s), 1);
            s.start_turn().answer(json!(SUMMON_PILLOW)).answer(json!(NOTHING));
            assert_eq!(counted(&s), 2);
            assert!(!radiance(&s).iter().any(|&radiant| radiant));
        }

        /// The Radiant face's double pick counts each pick: three turns of two reach the 6th.
        #[test]
        fn r3139001_the_radiant_face_counts_each_of_its_two_picks_so_three_turns_make_the_hand_radiant() {
            let mut s = counting(true, 0);
            for turn in 1..=2 {
                s.start_turn().answer(json!(LOSE_THREE)).answer(json!(EXILE_BOTTOM));
                assert_eq!(counted(&s), 2 * turn);
                assert!(!radiance(&s).iter().any(|&radiant| radiant));
            }

            s.start_turn().answer(json!(SUMMON_PILLOW)).answer(json!(EXILE_BOTTOM));

            assert_eq!(counted(&s), 0);
            assert!(held_before_the_draw(&s).iter().all(|&radiant| radiant));
        }

        /// The count crosses the threshold mid-turn: the first pick is the 6th, the second starts a new count.
        #[test]
        fn r3139001_the_second_pick_of_a_turn_starts_the_next_count_when_the_first_was_the_6th() {
            let mut s = counting(true, 5);

            s.start_turn().answer(json!(LOSE_THREE));
            assert_eq!(counted(&s), 0);
            assert!(radiance(&s).iter().all(|&radiant| radiant));
            s.answer(json!(EXILE_BOTTOM));

            assert_eq!(counted(&s), 1);
        }

        /// The Mask only asks on its controller's turn, so the opponent's turn counts none.
        #[test]
        fn r3139001_the_opponent_s_turn_counts_none() {
            let mut s = counting(false, 3);
            s.end_turn();
            assert_eq!(s.state().active, PlayerId::P2);
            s.start_turn();

            assert!(s.state().pending.is_none());
            assert_eq!(counted(&s), 3);
            assert!(!radiance(&s).iter().any(|&radiant| radiant));
        }

        /// A Degrade or Buff moves the declared 6: an Upgrade makes it the 5th.
        #[test]
        fn r386_r3139001_the_downsides_are_a_declared_number_an_upgrade_makes_it_5() {
            let mut s = counting(false, 4);
            assert_eq!(crate::upgrade_number(&mut s, MASK, "downsides"), 5);

            s.start_turn().answer(json!(LOSE_THREE));

            assert_eq!(counted(&s), 0);
            assert!(held_before_the_draw(&s).iter().all(|&radiant| radiant));
        }

        /// R767000: through v0.5.11r the Mask counted nothing and made no hand Radiant.
        #[test]
        fn r767000_the_script_kept_for_v0_5_11o_counts_no_downsides() {
            crate::under_kept(MASK, script_v0_5_11o(), || {
                let mut s = counting(false, 5);
                s.start_turn().answer(json!(LOSE_THREE));
                assert!(!radiance(&s).iter().any(|&radiant| radiant));
                assert_eq!(counted(&s), 5, "the count it never wrote stays as it was");
            });
        }
    }
}
