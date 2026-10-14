//! #83 Transmogulate (SPEC §8.4 row 83): Spell, cost 2, Legendary.
//!   Base:    "Replace every card in your hand, deck, board, graveyard and exile with a random
//!            Legendary."
//!   Radiant: "... with a random Radiant Legendary." — the same five zones with `radiant: true` on
//!            every replacement (§8 Conventions).
//!
//! R35 is the whole card; the hand is an "other zone" like the rest (R365). The pool is §5.1's one
//! query with this card's index excluded, never listed by hand. A board card takes a same-type
//! Legendary in place (Field Trap counts as Trap, so it becomes #85); Immutable cards stay (R23).
//! Cards under a Stack are dormant, so "your board" is each pile's top (R13, §3.2). The pool holds no
//! tokens, and a unit token cannot sit in a graveyard or exile (R11), so no replacement vanishes on
//! arrival and thins a zone. Picks draw from `ctx.rng` while the list is built (CLAUDE.md rule 4,
//! §9.3): deterministic, since the hook runs once and opens no prompt.

pub const ID: &str = "core-083";

// #83 Transmogulate (SPEC §8.4 row 83; R11, R23, R35, R365, R380). BUILD M4-T4's row: zone counts kept,
// same-type Legendaries in place, a Field Trap becoming Unlicensed Experimentation (R35), the hand
// replaced too (R365), radiant giving radiant cards; the pool is every set's non-token Legendaries
// but #83 (R380). A Spell reaches the graveyard after its script (§10.5), so that zone ends one larger.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TRANSMOGULATE: &str = "core-083";

    use crate::js;

    fn pool_of(args: Value) -> Vec<String> {
        crate::query::pool(TRANSMOGULATE, &json_as(args))
            .into_iter()
            .map(|def| def.id.clone())
            .collect()
    }

    /// R35's pool: every non-token Legendary except #83, of every set (R380); in Core the six R35
    /// names, #52, #85, #87, #92, #93, #95.
    fn pool_ids() -> Vec<String> {
        pool_of(json!({ "rarity": "Legendary" }))
    }

    const CORE_SIX: [&str; 6] = ["core-052", "core-085", "core-087", "core-092", "core-093", "core-095"];

    /// The Legendary Units in the pool, every set's (Core #52 Silly Silas and #92 Felinor Fiender among them).
    fn legendary_units() -> Vec<String> {
        pool_of(json!({ "rarity": "Legendary", "type": "Unit" }))
    }

    /// The Legendary Field Spells: Core #93 Combo-Index, Classic #4, #7, Classic+ #78.
    fn legendary_field_spells() -> Vec<String> {
        pool_of(json!({ "rarity": "Legendary", "type": "Field Spell" }))
    }

    /// The Legendary Trap this seed draws: #85 Unlicensed Experimentation, since a Field Trap counts as a
    /// Trap. There are two Legendary Traps of any set (#85 and Classic #9 Income Tax); the
    /// "transmogulate-1" seed draws #85 for both trap slots.
    const LEGENDARY_TRAP: &str = "core-085";

    /// Board fixtures: #11 Tempo Timmy (Unit), a Radiant #19 Midrange Menace (Immutable Unit), #73
    ///  (Field Spell), #41 Sheepish (Trap), #18 Bread and Butter (Field Trap).
    const IMMUTABLE: &str = "core-019";

    fn board(radiant_face: bool) -> Scenario {
        let mut s = scenario(json!({
            // Pinned: this seed draws #85 for both trap slots and no replacement casts (a Spell Tyrant
            // draw cascades under other seeds).
            "seed": "transmogulate-1",
            "p1": {
                "hand": [TRANSMOGULATE, "core-056"],
                "field": ["core-011", { "def": IMMUTABLE, "radiant": true }],
                "backrow": [
                    { "def": "core-073", "lane": 1 },
                    { "def": "core-041", "lane": 2 },
                    { "def": "core-018", "lane": 3 },
                ],
                "library": ["core-020", "core-025"],
                "graveyard": ["core-056"],
                "exile": ["core-011"],
            },
            "p2": { "field": ["core-011"], "library": ["core-020"] },
        }));
        // Stands in for a missing `{ def, radiant }` form on SideSetup.hand (harness request).
        if radiant_face {
            let id = s.card(TRANSMOGULATE).id.clone();
            find_instance_mut(s.state_mut(), &id).expect("Transmogulate in hand").radiant = true;
        }
        s
    }

    /// The def id of the unit acting in a lane, or "" for an empty lane.
    fn unit_def(s: &Scenario, player: PlayerId, lane: i32) -> String {
        s.unit(player, lane).map(|card| card.def_id.clone()).unwrap_or_default()
    }

    /// The def id of the backrow card in a lane, or "" for an empty lane.
    fn backrow_def(s: &Scenario, player: PlayerId, lane: i32) -> String {
        s.backrow(player, lane).map(|card| card.def_id.clone()).unwrap_or_default()
    }

    mod n83_transmogulate_base {
        use super::*;

        #[test]
        fn r35_board_cards_are_replaced_in_place_by_a_legendary_of_the_same_type() {
            crate::register_all();
            let mut s = board(false);
            s.play(TRANSMOGULATE, json!({}));

            // A Unit becomes a Legendary Unit, in its own lane.
            assert!(legendary_units().contains(&unit_def(&s, P1, 1)));
            // A Field Spell becomes the Legendary Field Spell; a Trap becomes the Legendary Trap.
            assert!(legendary_field_spells().contains(&backrow_def(&s, P1, 1)));
            assert_eq!(backrow_def(&s, P1, 2), LEGENDARY_TRAP);
            s.expect_events(json!(["cardPlayed", "transformed"]));
        }

        #[test]
        fn r35_field_trap_counts_as_trap_a_field_trap_becomes_unlicensed_experimentation() {
            crate::register_all();
            let mut s = board(false);
            s.play(TRANSMOGULATE, json!({}));

            assert_eq!(backrow_def(&s, P1, 3), LEGENDARY_TRAP);
        }

        #[test]
        fn r23_an_immutable_board_card_stays_since_this_is_a_transform() {
            crate::register_all();
            let mut s = board(false);
            let immutable = s.card(IMMUTABLE).id.clone();
            s.play(TRANSMOGULATE, json!({}));

            assert_eq!(s.unit(P1, 2).map(|card| card.id.clone()), Some(immutable));
            assert_eq!(unit_def(&s, P1, 2), IMMUTABLE);
        }

        #[test]
        fn r35_r365_same_counts_per_zone_in_hand_library_graveyard_and_exile() {
            crate::register_all();
            let mut s = board(false);
            let hand = s.hand(Some(P1)).len() - 1;
            let library = s.pile(P1, "library").len();
            let graveyard = s.pile(P1, "graveyard").len();
            let exile = s.pile(P1, "exile").len();
            s.play(TRANSMOGULATE, json!({}));

            assert_eq!(s.hand(Some(P1)).len(), hand);
            assert_eq!(s.pile(P1, "library").len(), library);
            assert_eq!(s.pile(P1, "exile").len(), exile);
            // Plus Transmogulate itself, which was resolving while the replacements happened.
            assert_eq!(s.pile(P1, "graveyard").len(), graveyard + 1);
            assert!(s
                .pile(P1, "graveyard")
                .iter()
                .any(|card| card.def_id == TRANSMOGULATE));
        }

        #[test]
        fn r35_r380_the_pool_is_every_set_s_non_token_legendaries_core_s_n52_n85_n87_n92_n93_n95_among_them_never_transmogulate_itself(
        ) {
            crate::register_all();
            let pool = pool_ids();
            for id in CORE_SIX {
                assert!(pool.contains(&id.to_string()));
            }
            assert!(!pool.contains(&TRANSMOGULATE.to_string()));
            assert!(pool.iter().any(|id| id.starts_with("classic")));
            let mut s = board(false);
            let spell = s.card(TRANSMOGULATE).id.clone();
            s.play(TRANSMOGULATE, json!({}));

            let mut replaced: Vec<CardInstance> = Vec::new();
            replaced.extend(s.hand(Some(P1)));
            replaced.extend(s.pile(P1, "library"));
            replaced.extend(s.pile(P1, "exile"));
            replaced.extend(s.pile(P1, "graveyard").into_iter().filter(|card| card.id != spell));
            for lane in 1..=5 {
                if let Some(unit) = s.unit(P1, lane) {
                    replaced.push(unit.clone());
                }
                if let Some(back) = s.backrow(P1, lane) {
                    replaced.push(back.clone());
                }
            }

            for card in &replaced {
                // The Immutable Menace is the one card R23 left alone.
                if card.def_id == IMMUTABLE {
                    continue;
                }
                assert!(pool.contains(&card.def_id), "{} is not in R35's pool", card.def_id);
            }
        }

        #[test]
        fn r35_replaced_cards_cease_to_exist_they_are_in_no_pile_at_all() {
            crate::register_all();
            let mut s = board(false);
            let old_unit = s.card("core-011").id.clone();
            let old_library: Vec<String> = s.pile(P1, "library").into_iter().map(|card| card.id).collect();
            let old_graveyard: Vec<String> = s.pile(P1, "graveyard").into_iter().map(|card| card.id).collect();
            s.play(TRANSMOGULATE, json!({}));

            s.expect_in_zone(old_unit.as_str(), "gone");
            for id in old_library.iter().chain(old_graveyard.iter()) {
                s.expect_in_zone(id.as_str(), "gone");
            }
        }

        #[test]
        fn r365_your_hand_is_replaced_too_the_card_in_it_ceases_to_exist_and_a_pool_card_takes_its_place() {
            crate::register_all();
            let mut s = board(false);
            let spare = s
                .hand(Some(P1))
                .into_iter()
                .find(|card| card.def_id == "core-056")
                .map(|card| card.id)
                .unwrap_or_default();
            s.play(TRANSMOGULATE, json!({}));

            s.expect_in_zone(spare.as_str(), "gone");
            assert_eq!(s.hand(Some(P1)).len(), 1);
            let first = s.hand(Some(P1)).first().cloned();
            assert!(pool_ids().contains(&first.as_ref().map(|card| card.def_id.clone()).unwrap_or_default()));
            assert_eq!(first.map(|card| card.radiant), Some(false));
        }

        #[test]
        fn r365_the_opponent_never_learns_what_the_hand_became_r177() {
            crate::register_all();
            let mut s = board(false);
            s.play(TRANSMOGULATE, json!({}));

            assert_eq!(js(&s.view(Some(P2)).opponent.hand), json!({ "count": 1 }));
            let seen = serde_json::to_string(&s.view(Some(P2))).expect("serialises");
            let held = s
                .hand(Some(P1))
                .first()
                .map(|card| card.id.clone())
                .unwrap_or_else(|| "no-card".to_string());
            assert!(!seen.contains(&held));
        }

        #[test]
        fn s8_your_library_and_board_the_opponent_keeps_everything() {
            crate::register_all();
            let mut s = board(false);
            s.play(TRANSMOGULATE, json!({}));

            assert_eq!(unit_def(&s, P2, 1), "core-011");
            assert_eq!(
                s.pile(P2, "library").into_iter().map(|card| card.def_id).collect::<Vec<_>>(),
                vec!["core-020".to_string()]
            );
        }

        #[test]
        fn base_gives_non_radiant_cards() {
            crate::register_all();
            let mut s = board(false);
            s.play(TRANSMOGULATE, json!({}));

            assert_eq!(
                s.pile(P1, "library").into_iter().map(|card| card.radiant).collect::<Vec<_>>(),
                vec![false, false]
            );
            assert_eq!(s.unit(P1, 1).map(|card| card.radiant), Some(false));
            assert_eq!(s.backrow(P1, 1).map(|card| card.radiant), Some(false));
        }

        #[test]
        fn r13_a_card_dormant_under_a_stack_is_not_on_your_board_only_the_top_of_the_pile_is_replaced() {
            crate::register_all();
            // #92 Felinor Fiender (Stack) on top of #43 Big Felinor in lane 1; the Big Felinor is dormant.
            let mut s = scenario(json!({
                "seed": "transmogulate-stack",
                "p1": {
                    "hand": [TRANSMOGULATE],
                    "field": ["core-043", { "def": "core-092", "stack": true }],
                    "library": ["core-020"],
                },
                "p2": { "field": ["core-011"], "library": ["core-020"] },
            }));
            let buried = s.card("core-043").clone();
            let top = s.card("core-092").clone();
            assert_eq!(
                s.unit(P1, 1).map(|card| card.id.clone()),
                Some(top.id.clone()),
                "the Fiender is on top of lane 1"
            );

            s.play(TRANSMOGULATE, json!({}));

            // The acting top card was replaced in place by a Legendary Unit.
            let transformed: Vec<String> = s
                .events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Transformed { instance_id, .. } => Some(instance_id.clone()),
                    _ => None,
                })
                .collect();
            assert!(transformed.contains(&top.id));
            assert!(legendary_units().contains(&unit_def(&s, P1, 1)));
            // The dormant card is untouched: still on the field beneath it, still a Big Felinor.
            assert!(!transformed.contains(&buried.id));
            s.expect_in_zone(&buried, "field");
            assert_eq!(s.card(&buried).def_id, "core-043");
        }
    }

    mod n83_transmogulate_radiant {
        use super::*;

        #[test]
        fn random_radiant_legendaries_every_replacement_is_radiant_in_every_zone() {
            crate::register_all();
            let mut s = board(true);
            let spell = s.card(TRANSMOGULATE).id.clone();
            s.play(TRANSMOGULATE, json!({}));

            assert_eq!(s.unit(P1, 1).map(|card| card.radiant), Some(true));
            assert_eq!(s.backrow(P1, 1).map(|card| card.radiant), Some(true));
            assert_eq!(s.backrow(P1, 2).map(|card| card.radiant), Some(true));
            assert_eq!(s.backrow(P1, 3).map(|card| card.radiant), Some(true));
            assert_eq!(
                s.pile(P1, "library").into_iter().map(|card| card.radiant).collect::<Vec<_>>(),
                vec![true, true]
            );
            assert_eq!(
                s.pile(P1, "exile").into_iter().map(|card| card.radiant).collect::<Vec<_>>(),
                vec![true]
            );
            assert_eq!(
                s.hand(Some(P1)).into_iter().map(|card| card.radiant).collect::<Vec<_>>(),
                vec![true]
            );
            assert_eq!(
                s.pile(P1, "graveyard")
                    .into_iter()
                    .filter(|card| card.id != spell)
                    .map(|card| card.radiant)
                    .collect::<Vec<_>>(),
                vec![true]
            );
        }

        #[test]
        fn the_radiant_face_keeps_r35_s_pool_and_its_same_type_board_rule() {
            crate::register_all();
            let mut s = board(true);
            s.play(TRANSMOGULATE, json!({}));

            assert!(legendary_units().contains(&unit_def(&s, P1, 1)));
            assert!(legendary_field_spells().contains(&backrow_def(&s, P1, 1)));
            assert_eq!(backrow_def(&s, P1, 3), LEGENDARY_TRAP);
            let pool = pool_ids();
            for card in s.pile(P1, "library") {
                assert!(pool.contains(&card.def_id));
            }
        }

        #[test]
        fn r23_an_immutable_board_card_still_stays_on_the_radiant_face() {
            crate::register_all();
            let mut s = board(true);
            s.play(TRANSMOGULATE, json!({}));

            let immutable = s.unit(P1, 2);
            assert_eq!(immutable.as_ref().map(|card| card.def_id.as_str()), Some(IMMUTABLE));
            // A refused Transform changes nothing about the card: it is the Menace it was.
            assert_eq!(immutable.map(|card| card.radiant), Some(true));
        }
    }

    mod n83_transmogulate_r312_the_owner_s_library_list {
        use super::*;

        #[test]
        fn r312_every_library_replacement_is_a_card_its_owner_was_never_shown_so_the_list_counts_them_unknown() {
            crate::register_all();
            let mut s = board(false);
            // Before: p1's own two cards, by printed cost (R310): Pointmaster (2), then the 7/7 (4).
            assert_eq!(
                js(&s.view(Some(P1)).you.own_library),
                json!({
                    "cards": [
                        { "defId": "core-020", "radiant": false, "count": 1 },
                        { "defId": "core-025", "radiant": false, "count": 1 },
                    ],
                    "unknown": 0,
                })
            );

            s.play(TRANSMOGULATE, json!({}));

            // The same count, none of it named: the Legendaries it rolled stay unread, even by p1.
            assert_eq!(js(&s.view(Some(P1)).you.own_library), json!({ "cards": [], "unknown": 2 }));
            let mine = serde_json::to_string(&s.view(Some(P1))).expect("serialises");
            for card in s.pile(P1, "library") {
                assert!(!mine.contains(&format!("\"{}\"", card.id)));
            }
            // p2's library is untouched and still fully known to p2.
            assert_eq!(
                js(&s.view(Some(P2)).you.own_library),
                json!({ "cards": [{ "defId": "core-020", "radiant": false, "count": 1 }], "unknown": 0 })
            );
        }
    }
}
