//! #42 Eugenics (SPEC §8.2). Spell, cost 2, Common.
//!   Base:    "Exile 7 random cards from your deck. Each card left in your deck has a 30% chance to
//!             become Radiant." (patch v0.1.1: 8 became 7 on both faces)
//!   Radiant: "Lucky 1 at 40%" — §8 Conventions: a cell that changes only a number changes only that
//!            number, and every clause it does not restate is kept. So the radiant face still exiles
//!            7, and only the chance (30% → 40%) changes, with Lucky 1 added (§6.1: one extra roll,
//!            keep the best, i.e. keep the success).
//!   Engine:  "Fewer than 7 → exile all" — R60's "a random pick of N existing cards picks N
//!            different cards, or all of them if fewer exist".
//!
//! NO DICE IN A CARD FILE. Rolling advances `rngCursor`, which is state (CLAUDE.md rules 4 and 5,
//! §10.7), so both halves of this card are single effects that own their own randomness. Neither
//! exists in `packages/engine/src/effects` yet; the signatures below are what the engine must add
//! (see the agent report):
//!
//!   exileRandomFromLibrary({ count, player? })
//!       R60: `count` DIFFERENT cards drawn uniformly from `player`'s library, or the whole library
//!       when it holds fewer; each goes through the Exile verb (§6.3), so the exile counter moves
//!       (R55) and a unit-token card ceases to exist instead of entering the pile (R11).
//!
//!   radiantChance({ zone, player?, chance, lucky? })
//!       "each remaining library card has a 30% chance": ONE independent roll per card still in the
//!       zone, in zone order (top down), through `rng.chance(chance)`. R60 makes the flag the whole
//!       model and an already-Radiant card is skipped rather than rolled, so nothing is un-set and
//!       the roll count is the number of non-Radiant cards. `lucky: n` routes the roll through
//!       `rng.lucky(n, roll, better)` with "a success beats a failure" as the comparator (§6.1,
//!       R32), which is what "Lucky 1 at 40%" means: two rolls per card, keep the success.
//!
//! ORDER MATTERS. The exile runs first and the chance rolls over what is LEFT ("each remaining
//! library card"), so an exiled card is never rolled. Two effects in one list is the right shape for
//! that: §4.5/R59 puts the state check after the whole list, never between two effects of it.
//!
//! (Rust: the two verbs are `effects::exile_random_from_library` and `effects::radiant_chance`, each
//! handed its TS object literal through `json_as`.)

use jackioh_engine::effects::{exile_random_from_library, radiant_chance};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-042";

/// §8.2: "Exile 7 random cards from your deck", on both faces (patch v0.1.1: 8 became 7).
const EXILE_COUNT: i32 = 7;

/// The faces differ only in the chance and in the Lucky they print (§6.1). The Lucky is the card's own
/// (`lucky_on`: printed plus given, R1438), so a base face given Lucky 1 rolls twice a card too.
fn eugenics(chance: f64) -> Script {
    Script {
        // A Spell's script hangs off `cry`: that is its on-resolve hook (§10.9).
        cry: Some(hook(move |ctx| {
            let lucky = ctx.live_self().map_or(0, |me| lucky_on(&*ctx.state, me));
            let mut chance_args = json!({ "zone": "library", "chance": chance });
            if lucky > 0 {
                chance_args["lucky"] = json!(lucky);
            }
            vec![
                exile_random_from_library(json_as(json!({ "count": EXILE_COUNT }))),
                radiant_chance(json_as(chance_args)),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: eugenics(0.3),
        radiant: eugenics(0.4),
    }
}

// #42 Eugenics (SPEC §8.2, §5.2, §6.1 Lucky, §10.7; R60, R80).
//
// The must-pass row (BUILD M4-T4 #42): "7 random exiled (all if fewer); 30% per remaining card;
// radiant two rolls at 40%." (patch v0.1.1: 8 became 7.)
//
// Exact per-seed outcomes are not asserted here, and deliberately so: the two verbs this card needs
// (`exileRandomFromLibrary`, `radiantChance`) are not in the effects library yet, so there is no
// implementation whose draw order an expected number could be read off — a hard-coded count would
// be a guess that locks the engine into whatever the guess was. What IS pinned down instead:
//   * the counts the §8 row states (7, or the whole library when smaller);
//   * that the roll is PER CARD and independent, proved with a 57-card library where "one roll for
//     the zone" or "all or nothing" would land on 0 or 50 and an independent 30% cannot;
//   * that the exiled cards are never rolled;
//   * determinism: the same seed and the same steps give the same pattern of flags (§9.3);
//   * that the radiant face rolls exactly one extra time per remaining card, which is what Lucky 1
//     means (§6.1) and is a count, not a probability.
#[cfg(test)]
mod tests {
    use super::script;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    /// A library of `n` identical non-Radiant, non-token cards. Identity is irrelevant to every clause
    /// of this card — only the counts and the flags are — and #25 4-mana 7/7 has no script at all, so
    /// nothing in the library can react to being exiled. R80 caps a library at 60.
    fn library(n: usize) -> Vec<&'static str> {
        vec!["core-025"; n]
    }

    /// p1 casts Eugenics; #21 Hinder rides along so the turn does not auto-end after it (R82).
    fn cast(library_size: usize, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": ["core-042", "core-021"], "library": library(library_size) },
        }));
        s.play("core-042", json!({}));
        s
    }

    /// The radiant face has to be flagged in hand. TS set `radiant` on the hand card after the build;
    /// the setup entry's own `radiant: true` sets the same flag on the same freshly created hand card.
    fn cast_radiant(library_size: usize, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": "core-042", "radiant": true }, "core-021"],
                "library": library(library_size),
            },
        }));
        s.play("core-042", json!({}));
        s
    }

    /// The effects a face returns, by `kind`. Neither hook reads its context, so any context will do:
    /// TS handed a stub, Rust hands one built at rest over a scenario's state.
    fn effect_kinds(hook: Option<&Hook>) -> Vec<&'static str> {
        let Some(hook) = hook else {
            return vec![];
        };
        let mut state = scenario(json!({})).state().clone();
        let mut events = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut ctx = EffectContext::new(EngineSink::new(&mut state, &mut events, &mut rng), PlayerId::P1);
        hook(&mut ctx).iter().map(|effect| effect.kind).collect()
    }

    fn radiant_flags(s: &Scenario) -> Vec<bool> {
        s.pile(PlayerId::P1, "library")
            .iter()
            .map(|card| card.radiant)
            .collect()
    }

    /// "#42 Eugenics — base"
    mod base {
        use super::*;

        /// "exiles 7 random cards from your deck"
        #[test]
        fn exiles_7_random_cards_from_your_deck() {
            let s = cast(12, "eugenics");

            assert_eq!(s.pile(PlayerId::P1, "exile").len(), 7);
            assert_eq!(s.pile(PlayerId::P1, "library").len(), 5);
        }

        /// "R60 exiles the whole library when it holds fewer than 7 (§8.2 Engine: 'fewer than 7 → exile all')"
        #[test]
        fn r60_exiles_the_whole_library_when_it_holds_fewer_than_7_engine_fewer_than_7_exile_all() {
            let s = cast(5, "eugenics");

            assert_eq!(s.pile(PlayerId::P1, "exile").len(), 5);
            assert_eq!(s.pile(PlayerId::P1, "library").len(), 0);
        }

        /// "an empty library exiles nothing and the spell still counts as played (§8 Conventions)"
        #[test]
        fn an_empty_library_exiles_nothing_and_the_spell_still_counts_as_played() {
            let mut s = cast(0, "eugenics");

            assert_eq!(s.pile(PlayerId::P1, "exile").len(), 0);
            s.expect_events(json!(["cardPlayed"]));
            s.expect_in_zone("core-042", "graveyard");
        }

        /// "rolls 'each remaining library card' independently at 30%, so neither 0 nor all 50 come up"
        #[test]
        fn rolls_each_remaining_library_card_independently_at_30_percent_so_neither_0_nor_all_50_come_up() {
            let s = cast(57, "eugenics");
            let remaining = s.pile(PlayerId::P1, "library");

            assert_eq!(remaining.len(), 50);
            let converted = remaining.iter().filter(|card| card.radiant).count();
            // One roll for the whole zone would give 0 or 50; 50 independent 30% rolls give neither.
            assert!(converted > 0);
            assert!(converted < 50);
        }

        /// "R60 the exiled cards are never rolled: the exile happens first, the chance rolls over what is left"
        #[test]
        fn r60_the_exiled_cards_are_never_rolled_the_exile_happens_first_the_chance_rolls_over_what_is_left()
        {
            let s = cast(57, "eugenics");

            assert_eq!(s.pile(PlayerId::P1, "exile").len(), 7);
            assert!(s.pile(PlayerId::P1, "exile").iter().all(|card| !card.radiant));
        }

        /// "§9.3 is deterministic: the same seed and steps give the same flags"
        #[test]
        fn s9_3_is_deterministic_the_same_seed_and_steps_give_the_same_flags() {
            assert_eq!(
                radiant_flags(&cast(57, "eugenics-determinism")),
                radiant_flags(&cast(57, "eugenics-determinism")),
            );
        }

        /// "§10.9 the base face is one exile effect followed by one chance effect, in that order"
        #[test]
        fn s10_9_the_base_face_is_one_exile_effect_followed_by_one_chance_effect_in_that_order() {
            assert_eq!(
                effect_kinds(script().base.cry.as_ref()),
                vec!["exileRandomFromLibrary", "radiantChance"],
            );
        }
    }

    /// R1438: given Lucky 1, the base face rolls each remaining card twice at 30% and keeps a success:
    /// 50 more draws than the plain base face over the same exile, and more cards come up Radiant.
    #[test]
    fn r1438_given_lucky_1_the_base_face_rolls_twice_and_keeps_the_better() {
        let plain = cast(57, "eugenics-lucky");
        let mut lucky = scenario(json!({
            "seed": "eugenics-lucky",
            "p1": { "hand": ["core-042", "core-021"], "library": library(57) },
        }));
        crate::give_lucky(&mut lucky, "core-042", 1);
        lucky.play("core-042", json!({}));

        assert_eq!(lucky.state().rng_cursor - plain.state().rng_cursor, 50);
        let converted = |s: &Scenario| {
            s.pile(PlayerId::P1, "library")
                .iter()
                .filter(|card| card.radiant)
                .count()
        };
        assert!(converted(&lucky) > converted(&plain));
        assert!(converted(&lucky) < 50);
    }

    /// "#42 Eugenics — radiant"
    mod radiant {
        use super::*;

        /// "§8 Conventions: the unrestated clause is kept, so it still exiles 7"
        #[test]
        fn s8_conventions_the_unrestated_clause_is_kept_so_it_still_exiles_7() {
            let s = cast_radiant(12, "eugenics");

            assert_eq!(s.pile(PlayerId::P1, "exile").len(), 7);
            assert_eq!(s.pile(PlayerId::P1, "library").len(), 5);
        }

        /// "R60 the exiled cards are still never rolled"
        #[test]
        fn r60_the_exiled_cards_are_still_never_rolled() {
            let s = cast_radiant(57, "eugenics");

            assert_eq!(s.pile(PlayerId::P1, "exile").len(), 7);
            assert!(s.pile(PlayerId::P1, "exile").iter().all(|card| !card.radiant));
        }

        /// "rolls per remaining card at 40% with Lucky 1, so neither 0 nor all 50 come up"
        #[test]
        fn rolls_per_remaining_card_at_40_percent_with_lucky_1_so_neither_0_nor_all_50_come_up() {
            let s = cast_radiant(57, "eugenics");
            let remaining = s.pile(PlayerId::P1, "library");

            assert_eq!(remaining.len(), 50);
            let converted = remaining.iter().filter(|card| card.radiant).count();
            assert!(converted > 0);
            assert!(converted < 50);
        }

        /// "§6.1 Lucky 1 is exactly one extra roll per remaining card: 50 more draws than the base face"
        #[test]
        fn s6_1_lucky_1_is_exactly_one_extra_roll_per_remaining_card_50_more_draws_than_the_base_face() {
            // Same seed and same library, so the exile step consumes the same draws in both runs and the
            // whole difference in `rngCursor` is the second roll Lucky 1 takes for each remaining card.
            let plain = cast(57, "eugenics-lucky");
            let lucky = cast_radiant(57, "eugenics-lucky");

            assert_eq!(lucky.state().rng_cursor - plain.state().rng_cursor, 50);
        }

        /// "§9.3 is deterministic: the same seed and steps give the same flags"
        #[test]
        fn s9_3_is_deterministic_the_same_seed_and_steps_give_the_same_flags() {
            assert_eq!(
                radiant_flags(&cast_radiant(57, "eugenics-r-determinism")),
                radiant_flags(&cast_radiant(57, "eugenics-r-determinism")),
            );
        }

        /// "§10.9 the radiant face is the same pair of effects, so only the numbers changed"
        #[test]
        fn s10_9_the_radiant_face_is_the_same_pair_of_effects_so_only_the_numbers_changed() {
            assert_eq!(
                effect_kinds(script().radiant.cry.as_ref()),
                vec!["exileRandomFromLibrary", "radiantChance"],
            );
        }
    }

    /// "#42 Eugenics — R311 the owner's library list"
    mod r311_the_owners_library_list {
        use super::*;

        /// "R311 a card rolled Radiant inside the library is still listed with the face it went in with"
        #[test]
        fn r311_a_card_rolled_radiant_inside_the_library_is_still_listed_with_the_face_it_went_in_with() {
            let s = cast(57, "eugenics");
            let remaining = s.pile(PlayerId::P1, "library");
            assert!(remaining.iter().any(|card| card.radiant));

            // The rolls happened where nobody reads them (R177), so the list shows none of them.
            assert_eq!(
                serde_json::to_value(&s.view(PlayerId::P1).you.own_library).unwrap(),
                json!({
                    "cards": [{ "defId": "core-025", "radiant": false, "count": remaining.len() }],
                    "unknown": 0,
                }),
            );
        }
    }
}
