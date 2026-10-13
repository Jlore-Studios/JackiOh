//! #23 Reoccurring Dream (SPEC §8.2, R60, R68, §5.1).
//!
//! Base: "30% chance a random card in your hand becomes Radiant. End of turn: returns from the GY to
//! your hand". Radiant restates the roll only — "Lucky 1 (two rolls, keep the success) at 40%" — so
//! the return-from-the-graveyard clause is kept (§8 Conventions).
//!
//! The pick is `setRadiantRandom`, which is R60's random pick: it draws from the non-Radiant cards of
//! the zone and does nothing when none are left. The spell itself is in `resolving` while its script
//! runs (§10.5), so it can never pick itself.
//!
//! Both rolls go through the seeded `ctx.rng` (CLAUDE.md rule 4), the base one roll at 0.3 and the
//! radiant `lucky(1, …)` — two rolls at 0.4, keeping a success, which is §6.1's Lucky X read on a
//! yes/no roll. The X is the face's printed Lucky as it always was (0, or 1 on the Radiant face; a
//! Nerf or a Buff never reaches it, D14) plus any Lucky given (`given_lucky_on`, R1438), so a base
//! face given Lucky 1 rolls twice at 0.3, and a Radiant face given Lucky 1 three times at 0.4. No
//! effect verb gates on a probability, so the hook does the roll and returns either
//! the effect or nothing; see the report for the `chanceOf` verb this wants. With an empty hand the
//! effect has nothing to do, so it rolls nothing (R129, R60). A hand that is all Radiant is rolled
//! like any other: whether the hand holds a base-face card is the hand's (§9.1), so neither the roll
//! nor the cue may hang on it — a success over it cues the pick it could not make on a Radiant card
//! (R177's `cueUnpicked`), and the hand's size, which decides the roll, is public (R177).
//!
//! §5.1: "Spells with 'End of turn: add this back to your hand' are flagged
//! `returnToHandAtEndOfTurn` when played and return from the graveyard at the end of that turn, as
//! graveyard triggers (R68)". No effect verb sets that flag yet (reported), so the hook gates on the
//! flag OR this turn's play log, which is the same set of cards for a spell played normally: a copy
//! that reached the graveyard by being discarded or milled is not in the log and stays there.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-023";

const BASE_CHANCE: f64 = 0.3;
const RADIANT_CHANCE: f64 = 0.4;

/// R60: one random non-Radiant card in the caster's hand becomes Radiant.
fn make_one_radiant() -> Vec<Effect> {
    vec![set_radiant_random(json_as(
        json!({ "zones": "hand", "count": 1 }),
    ))]
}

/// R129: the roll is taken only when the pick has a hand to look in. The hand's size is public; which
/// of its cards are Radiant is not (§9.1), so an all-Radiant hand is rolled too and its success is
/// cued on a Radiant card (R177).
fn any_to_make_radiant(ctx: &EffectContext<'_>) -> bool {
    !zone_cards(ctx.state, ctx.controller, OffFieldZone::Hand).is_empty()
}

/// §6.1: the Lucky the Radiant face prints, fixed (D14: tuning never reached this roll).
const RADIANT_LUCKY: i32 = 1;

/// §6.1, R1438: the Lucky the Dream rolls with: the face's printed number plus any Lucky given.
fn lucky(ctx: &EffectContext<'_>, printed: i32) -> i32 {
    printed + ctx.live_self().map_or(0, given_lucky_on)
}

/// §5.1 and R68: at the end of the turn it was played on, the spell goes from the graveyard back to
/// its owner's hand, and a full hand burns it (§2.4, R4) — both of which `bounce` does.
fn returns_to_hand(ctx: &EffectContext<'_>) -> bool {
    // TS read the live `ctx.self`: the card as it stands now.
    let Some(self_) = ctx.live_self() else {
        return false;
    };
    if self_.return_to_hand_at_end_of_turn == Some(true) {
        return true;
    }
    was_played_this_turn(ctx.state, self_.controller, self_)
}

fn end_of_turn() -> Hook {
    hook(|ctx| {
        if returns_to_hand(ctx) {
            vec![bounce(json_as(json!({ "target": { "of": "self" } })))]
        } else {
            vec![]
        }
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            if !any_to_make_radiant(ctx) {
                return vec![];
            }
            // R987: the controller's Luck rolls extra times beside the card's own Lucky (R1438; with
            // neither, `lucky(0)` draws once, as before).
            let lucky = lucky(ctx, 0) + luck_of(&*ctx.state, ctx.controller);
            let hit = ctx.rng.lucky(lucky, |rng| rng.chance(BASE_CHANCE), |a, b| a || b);
            if hit { make_one_radiant() } else { vec![] }
        })),
        end_of_turn: Some(end_of_turn()),
        ..Script::default()
    };

    let radiant = Script {
        // Lucky 1: roll twice and keep the success (§6.1), each roll at 40%.
        cry: Some(hook(|ctx| {
            if !any_to_make_radiant(ctx) {
                return vec![];
            }
            // R987: the controller's Luck rolls extra times beside the card's own Lucky (R1438).
            let lucky = lucky(ctx, RADIANT_LUCKY) + luck_of(&*ctx.state, ctx.controller);
            let hit = ctx
                .rng
                .lucky(lucky, |rng| rng.chance(RADIANT_CHANCE), |a, b| a || b);
            if hit { make_one_radiant() } else { vec![] }
        })),
        end_of_turn: Some(end_of_turn()),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// #23 Reoccurring Dream — SPEC §8.2, BUILD M4-T4 row 23: "Seeded 30% roll on a non-Radiant hand
// card (R60); returns to hand at end of turn; hand full → burned; radiant two rolls at 40% keeping
// a success".
//
// Every roll goes through the seeded rng, so each fixture pins its seed and asserts the outcome
// that seed produces. The gate itself is asserted across one fixed seed list used by both faces,
// which is also how "Lucky 1 at 40%" is proved to be strictly luckier than "30%": at the same seed
// and the same rng cursor a base hit (roll < 0.3) is always a radiant hit (roll < 0.4), and the
// radiant face rolls a second time on top of that, so its hit set is a strict superset of the base
// one. Whether any individual seed hits is not asserted: that would pin the rng's internals, not
// the card.
#[cfg(test)]
mod tests {
    use super::script;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const DREAM: &str = "core-023"; // cost 1.
    const OTHERS: [&str; 4] = ["core-002", "core-003", "core-004", "core-011"];

    /// Ten other cards: playing the Dream leaves the hand exactly at HAND_CAP.
    const FULL_HAND: [&str; 10] = [
        "core-002", "core-003", "core-004", "core-005", "core-008", "core-010", "core-011", "core-012",
        "core-013", "core-016",
    ];

    const SEEDS: [&str; 12] = [
        "dream-01", "dream-02", "dream-03", "dream-04", "dream-05", "dream-06", "dream-07", "dream-08",
        "dream-09", "dream-10", "dream-11", "dream-12",
    ];

    fn dream_in(seed: &str, is_radiant: bool, others: &[&str]) -> Scenario {
        let mut hand = vec![json!({ "def": DREAM, "radiant": is_radiant })];
        hand.extend(others.iter().map(|def| json!(def)));
        scenario(json!({
            "seed": seed,
            "p1": { "hand": hand },
            "p2": { "hand": ["core-005"] },
        }))
    }

    /// Plays the Dream and counts the Radiant cards left in the hand.
    fn radiants_after_playing(seed: &str, is_radiant: bool) -> usize {
        let mut s = dream_in(seed, is_radiant, &OTHERS);
        s.play(DREAM, json!({}));
        s.hand(P1).iter().filter(|card| card.radiant).count()
    }

    mod n23_reoccurring_dream {
        use super::*;

        /// Plays the Dream with(out) a Feng Shui in the caster's backrow and counts the Radiant
        /// cards left in the hand.
        fn radiants_with(seed: &str, is_radiant: bool, judge: bool) -> usize {
            let mut hand = vec![json!({ "def": DREAM, "radiant": is_radiant })];
            hand.extend(OTHERS.iter().map(|def| json!(def)));
            let mut p1 = json!({ "hand": hand });
            if judge {
                p1["backrow"] = json!(["meditative-040"]);
            }
            let mut s = scenario(json!({
                "seed": seed,
                "p1": p1,
                "p2": { "hand": ["core-005"] },
            }));
            s.play(DREAM, json!({}));
            s.hand(P1).iter().filter(|card| card.radiant).count()
        }

        #[test]
        fn r987_feng_shui_s_luck_adds_a_roll() {
            crate::register_all();
            let _open = preview_sets(&[SetName::Meditative]);
            for radiant in [false, true] {
                let bare: Vec<usize> = SEEDS
                    .iter()
                    .map(|seed| radiants_with(seed, radiant, false))
                    .collect();
                let judged: Vec<usize> = SEEDS
                    .iter()
                    .map(|seed| radiants_with(seed, radiant, true))
                    .collect();
                // One more roll per seed keeps the success: every bare hit is a judged hit, and at
                // least one bare miss becomes a hit.
                assert!(bare.iter().zip(&judged).all(|(hit, judged)| judged >= hit));
                assert!(judged.iter().sum::<usize>() > bare.iter().sum::<usize>());
            }
        }

        mod base {
            use super::*;

            #[test]
            fn r60_the_30_roll_gates_it_and_a_hit_makes_exactly_one_hand_card_radiant() {
                crate::register_all();
                let results: Vec<usize> = SEEDS
                    .iter()
                    .map(|seed| radiants_after_playing(seed, false))
                    .collect();

                // R60: one random card, never two, and nothing at all on a miss.
                assert!(results.iter().all(|count| *count == 0 || *count == 1));
                assert!(results.contains(&1));
                assert!(results.contains(&0));
            }

            #[test]
            fn the_roll_is_seeded_one_seed_always_replays_to_the_same_outcome() {
                crate::register_all();
                for seed in &SEEDS[..4] {
                    assert_eq!(
                        radiants_after_playing(seed, false),
                        radiants_after_playing(seed, false)
                    );
                }
            }

            #[test]
            fn r60_a_hand_whose_other_cards_are_all_radiant_is_left_alone() {
                crate::register_all();
                // R60: "a random 'becomes Radiant' pick chooses among non-Radiant cards and does nothing if
                // none are left" — so this must hold at every seed, hit or miss.
                for seed in &SEEDS[..4] {
                    let mut hand = vec![json!({ "def": DREAM })];
                    hand.extend(OTHERS.iter().map(|def| json!({ "def": def, "radiant": true })));
                    let mut s = scenario(json!({
                        "seed": seed,
                        "p1": { "hand": hand },
                        "p2": { "hand": ["core-005"] },
                    }));
                    s.play(DREAM, json!({}));

                    assert_eq!(s.hand(P1).len(), OTHERS.len());
                    assert!(s.hand(P1).iter().all(|card| card.radiant));
                }
            }

            #[test]
            fn s5_1_r68_at_the_end_of_the_turn_it_returns_from_the_graveyard_to_your_hand() {
                crate::register_all();
                let mut s = dream_in("dream-return", false, &OTHERS);
                s.play(DREAM, json!({}));
                let dream = s.card(DREAM).clone();
                s.expect_in_zone(&dream, "graveyard");

                s.end_turn();

                s.expect_in_zone(&dream, "hand");
                assert!(!s.pile(P1, "graveyard").iter().any(|card| card.def_id == DREAM));
            }

            #[test]
            fn r4_a_full_hand_burns_the_returning_card_instead() {
                crate::register_all();
                let mut hand = vec![DREAM];
                hand.extend(FULL_HAND);
                let mut s = scenario(json!({
                    "seed": "dream-burn",
                    // Eleven cards: playing the Dream leaves the hand at HAND_CAP.
                    "p1": { "hand": hand },
                    "p2": { "hand": ["core-005"] },
                }));
                s.play(DREAM, json!({}));
                let dream = s.card(DREAM).clone();
                assert_eq!(s.hand(P1).len(), FULL_HAND.len());

                s.end_turn();

                s.expect_in_zone(&dream, "graveyard");
                s.expect_events(json!(["burned"]));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn lucky_1_at_40_hits_wherever_the_base_30_hits_and_at_more_seeds_besides() {
                crate::register_all();
                let base_hits: Vec<&str> = SEEDS
                    .iter()
                    .copied()
                    .filter(|seed| radiants_after_playing(seed, false) == 1)
                    .collect();
                let radiant_hits: Vec<&str> = SEEDS
                    .iter()
                    .copied()
                    .filter(|seed| radiants_after_playing(seed, true) == 1)
                    .collect();

                assert!(!base_hits.is_empty());
                for seed in &base_hits {
                    assert!(radiant_hits.contains(seed));
                }
                // Two rolls at 40% beat one at 30%: 64% against 30% over enough seeds.
                assert!(radiant_hits.len() > base_hits.len());
                // R60: still one card at a time.
                assert!(SEEDS.iter().all(|seed| radiants_after_playing(seed, true) <= 1));
            }

            #[test]
            fn the_return_from_the_graveyard_is_kept_radiant_flag_and_all_s8_conventions_r74() {
                crate::register_all();
                let mut s = dream_in("dream-radiant-return", true, &OTHERS);
                s.play(DREAM, json!({}));
                let dream = s.card(DREAM).clone();

                s.end_turn();

                s.expect_in_zone(&dream, "hand");
                assert_eq!(
                    s.hand(P1)
                        .iter()
                        .find(|card| card.id == dream.id)
                        .map(|card| card.radiant),
                    Some(true)
                );
            }
        }

        /// R1438: given Lucky 1, the base face rolls twice at 30% and keeps a success, so it hits
        /// wherever the plain base face hits and at more seeds besides, one card at a time. Where the
        /// two come out the same, the lucky cast took exactly one more draw: its second roll.
        #[test]
        fn r1438_given_lucky_1_the_base_face_rolls_twice_and_keeps_the_better() {
            crate::register_all();
            let cast = |seed: &str, lucky: bool| -> (usize, u32) {
                let mut s = dream_in(seed, false, &OTHERS);
                if lucky {
                    crate::give_lucky(&mut s, DREAM, 1);
                }
                let before = s.state().rng_cursor;
                s.play(DREAM, json!({}));
                (
                    s.hand(P1).iter().filter(|card| card.radiant).count(),
                    s.state().rng_cursor - before,
                )
            };
            let base_hits: Vec<&str> = SEEDS
                .iter()
                .copied()
                .filter(|seed| cast(seed, false).0 == 1)
                .collect();
            let lucky_hits: Vec<&str> = SEEDS
                .iter()
                .copied()
                .filter(|seed| cast(seed, true).0 == 1)
                .collect();

            for seed in &base_hits {
                assert!(lucky_hits.contains(seed));
            }
            assert!(lucky_hits.len() > base_hits.len());
            let mut alike = 0;
            for seed in &SEEDS {
                let ((plain, plain_draws), (lucky, lucky_draws)) = (cast(seed, false), cast(seed, true));
                assert!(lucky <= 1);
                if plain == lucky {
                    alike += 1;
                    assert_eq!(lucky_draws, plain_draws + 1);
                }
            }
            assert!(alike > 0);
        }

        #[test]
        fn neither_face_declares_a_play_time_choice_r60_s_pick_is_random_not_chosen() {
            let scripts = script();
            assert!(scripts.base.targets.is_empty());
            assert!(scripts.base.modes.is_empty());
            assert!(scripts.radiant.targets.is_empty());
            assert!(scripts.radiant.modes.is_empty());
            assert!(scripts.base.end_of_turn.is_some());
            assert!(scripts.radiant.end_of_turn.is_some());
        }
    }
}
