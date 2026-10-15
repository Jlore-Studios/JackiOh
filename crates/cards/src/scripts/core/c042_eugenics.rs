//! #42 Eugenics (SPEC §8.2). Spell, cost 2, Common.
//!   Base:    "Exile all cards in your deck that cost (2) or less. Each card left in your deck has a 30%
//!             chance to become Radiant." (patch v0.3.9; it exiled 7 random cards before)
//!   Radiant: "Exile all cards in your deck that cost (2) or less. Each card left in your deck becomes
//!             Radiant." (patch v0.3.9: 100%, and no Lucky, since nothing is left to chance)
//!   Engine:  `exile_matching` over the library with `maxCost` 2 (R831014): each card's cost read per R65
//!            as the clause resolves, so an X card out of play counts as 0, an embiggen card at its base
//!            price, and a card's cost changes count; each exile is its own (R135). Then
//!            `radiant_chance` rolls each card left, one independent roll per card in zone order (R60),
//!            an already-Radiant card skipped; a Lucky given to the card rolls each twice and keeps a
//!            success (R1438).
//!
//! ORDER MATTERS. The exile runs first and the chance rolls over what is LEFT ("each card left in your
//! deck"), so an exiled card is never rolled. Two effects in one list is the right shape for that:
//! §4.5/R59 puts the state check after the whole list, never between two effects of it.

pub const ID: &str = "core-042";

// #42 Eugenics (SPEC §8.2, §5.2, §6.1 Lucky, §10.7; R60, R65, R80, R831014).
//
// The must-pass row (BUILD M4-T4 #42): "every deck card that costs (2) or less exiled (R65's cost);
// 30% per remaining card; radiant every remaining card Radiant."

#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const PLAIN: &str = "core-025"; // (4) 4-mana 7/7, no script: it stays.
    const MENACE: &str = "core-019"; // (3) Midrange Menace: it stays.
    const VANILLA: &str = "core-008"; // (1) Mr. Vanilla: exiled.
    const STOCKPILE: &str = "core-005"; // (1) Stockpile: exiled.

    /// `n` copies of one card. R80 caps a library at 60.
    fn copies(n: usize, card: &'static str) -> Vec<Value> {
        vec![json!(card); n]
    }

    /// 50 cards that stay and 7 that cost (2) or less, shuffled together.
    fn mixed() -> Vec<Value> {
        let mut library = copies(50, PLAIN);
        library.extend(copies(4, VANILLA));
        library.extend(copies(3, STOCKPILE));
        library
    }

    /// p1 casts Eugenics (Radiant on `radiant`) over `library`; #21 Hinder rides along so the turn does
    /// not auto-end after it (R82).
    fn cast_face(radiant: bool, library: Vec<Value>, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": "core-042", "radiant": radiant }, "core-021"], "library": library },
        }));
        s.play("core-042", json!({}));
        s
    }

    fn cast(library: Vec<Value>, seed: &str) -> Scenario {
        cast_face(false, library, seed)
    }

    /// The effects a face returns, by `kind`. Neither hook reads its context, so any context will do:
    /// one built at rest over a scenario's state.
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

    fn def_ids(s: &Scenario, pile: &str) -> Vec<String> {
        let mut ids: Vec<String> = s.pile(PlayerId::P1, pile).into_iter().map(|card| card.def_id).collect();
        ids.sort();
        ids
    }

    /// "#42 Eugenics — base"
    mod base {
        use super::*;

        #[test]
        fn r831014_exiles_every_card_in_your_deck_that_costs_2_or_less_and_keeps_the_rest() {
            let mut library = copies(4, PLAIN);
            library.extend(copies(2, MENACE));
            library.extend(copies(3, VANILLA));
            library.extend(copies(2, STOCKPILE));
            let s = cast(library, "eugenics");

            assert_eq!(def_ids(&s, "exile"), vec![STOCKPILE, STOCKPILE, VANILLA, VANILLA, VANILLA]);
            assert_eq!(def_ids(&s, "library"), vec![MENACE, MENACE, PLAIN, PLAIN, PLAIN, PLAIN]);
        }

        #[test]
        fn r831014_r65_an_x_card_counts_as_0_an_embiggen_card_at_its_base_price_and_a_cost_change_counts() {
            let library = vec![
                json!("core-024"),                           // Efficiency Dividend, X: 0 out of play.
                json!("core-056"),                           // Jilliax, (2) or paid (3): its (2).
                json!("core-046"),                           // Suppressive Aura, (2) or paid (4): its (2).
                json!({ "def": MENACE, "costMod": -1 }),     // (3) made (2): exiled.
                json!({ "def": VANILLA, "costMod": 2 }),     // (1) made (3): kept.
                json!(MENACE),                               // (3): kept.
            ];
            let s = cast(library, "eugenics-r65");

            assert_eq!(def_ids(&s, "exile"), vec!["core-019", "core-024", "core-046", "core-056"]);
            assert_eq!(def_ids(&s, "library"), vec![VANILLA, MENACE]);
        }

        #[test]
        fn an_empty_library_exiles_nothing_and_the_spell_still_counts_as_played() {
            let mut s = cast(vec![], "eugenics");

            assert_eq!(s.pile(PlayerId::P1, "exile").len(), 0);
            s.expect_events(json!(["cardPlayed"]));
            s.expect_in_zone("core-042", "graveyard");
        }

        #[test]
        fn rolls_each_remaining_library_card_independently_at_30_percent_so_neither_0_nor_all_50_come_up() {
            let s = cast(mixed(), "eugenics");
            let remaining = s.pile(PlayerId::P1, "library");

            assert_eq!(remaining.len(), 50);
            let converted = remaining.iter().filter(|card| card.radiant).count();
            // One roll for the whole zone would give 0 or 50; 50 independent 30% rolls give neither.
            assert!(converted > 0);
            assert!(converted < 50);
        }

        #[test]
        fn r60_r831014_the_exiled_cards_are_never_rolled_the_exile_happens_first_the_chance_rolls_over_what_is_left() {
            let s = cast(mixed(), "eugenics");

            assert_eq!(s.pile(PlayerId::P1, "exile").len(), 7);
            assert!(s.pile(PlayerId::P1, "exile").iter().all(|card| !card.radiant));
        }

        #[test]
        fn s9_3_is_deterministic_the_same_seed_and_steps_give_the_same_flags() {
            assert_eq!(
                radiant_flags(&cast(mixed(), "eugenics-determinism")),
                radiant_flags(&cast(mixed(), "eugenics-determinism")),
            );
        }

        #[test]
        fn s10_9_the_base_face_is_one_exile_effect_followed_by_one_chance_effect_in_that_order() {
            assert_eq!(
                effect_kinds(crate::scripts_of()[ID].clone().base.cry.as_ref()),
                vec!["exileMatching", "radiantChance"],
            );
        }
    }

    /// R1438: given Lucky 1, the base face rolls each remaining card twice at 30% and keeps a success:
    /// 50 more draws than the plain base face over the same exile, and more cards come up Radiant.
    #[test]
    fn r1438_given_lucky_1_the_base_face_rolls_twice_and_keeps_the_better() {
        let plain = cast(mixed(), "eugenics-lucky");
        let mut lucky = scenario(json!({
            "seed": "eugenics-lucky",
            "p1": { "hand": ["core-042", "core-021"], "library": mixed() },
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

        #[test]
        fn r831014_the_radiant_face_exiles_the_same_cards_and_every_card_left_becomes_radiant() {
            let s = cast_face(true, mixed(), "eugenics");

            assert_eq!(s.pile(PlayerId::P1, "exile").len(), 7);
            assert!(s.pile(PlayerId::P1, "exile").iter().all(|card| !card.radiant));
            assert_eq!(radiant_flags(&s), vec![true; 50]);
        }

        #[test]
        fn r831014_it_prints_no_lucky_since_patch_v0_3_9() {
            crate::register_all();
            let def = crate::card_def(ID);
            assert!(def.radiant.keywords.is_empty());
            assert!(def.base.keywords.is_empty());
        }

        #[test]
        fn s10_9_the_radiant_face_is_the_same_pair_of_effects_so_only_the_chance_changed() {
            assert_eq!(
                effect_kinds(crate::scripts_of()[ID].clone().radiant.cry.as_ref()),
                vec!["exileMatching", "radiantChance"],
            );
        }
    }

    /// "#42 Eugenics — R311 the owner's library list"
    mod r311_the_owners_library_list {
        use super::*;

        /// "R311 a card rolled Radiant inside the library is still listed with the face it went in with"
        #[test]
        fn r311_a_card_rolled_radiant_inside_the_library_is_still_listed_with_the_face_it_went_in_with() {
            let s = cast(copies(57, PLAIN), "eugenics");
            let remaining = s.pile(PlayerId::P1, "library");
            assert!(remaining.iter().any(|card| card.radiant));

            // The rolls happened where nobody reads them (R177), so the list shows none of them.
            assert_eq!(
                serde_json::to_value(&s.view(PlayerId::P1).you.own_library).unwrap(),
                json!({
                    "cards": [{ "defId": PLAIN, "radiant": false, "count": remaining.len() }],
                    "unknown": 0,
                }),
            );
        }
    }
}
