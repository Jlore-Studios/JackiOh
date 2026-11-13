//! M+ #2 NA Tech (SPEC §8.9 row 2; R1100, R1183000, R1183001, R2521007, R3869005): (0) Spell ((1) until issue #3138), Epic.
//!
//! Base:    "Fuse the card with the lowest win rate and the card with the highest win rate, and add
//!          it to your hand. It costs ({setCost})."
//! Radiant: "Fuse a Radiant card with the lowest win rate and a Radiant card with the highest win
//!          rate, and add it to your hand. It costs ({setCost})."
//! Engine: M #50 CN Tech's table, the game's win-rate snapshot (R3049000), read from both ends — the
//! leader (`win_rate_leader`, R1100: the highest Wilson lower bound, R3049001) and the trailer
//! (`win_rate_trailer`, R1183000: the same floor and tie-breaks, the lowest upper bound, never the
//! leader); a game handed none ranks its version's frozen table by the exact rate (R1266000). Neither
//! is ever NA Tech itself (R387); they are fused trailer first into one
//! transient hand card (`fuse_generated`'s two pools, R1108; R77, R102) at a `costOverride` of
//! setCost (R1183001). The Radiant face fuses both ingredients on their Radiant faces (R469). Tunes:
//! setCost 2 ↓ (Radiant 0).
//!
//! The fallback (#3869, R3869005): an end the table cannot name is a random non-token card of the sets
//! that ship, never NA Tech, the end the table names or the other ingredient (`distinct`), so the two
//! ends stay distinct, as R142 had it before #2521. Through v0.6.0g a missing end was a named card,
//! Core #75 Infinite Reserves for the lowest and Core #35 Lunar Eclipse for the highest, unless the
//! other end was that card (R2521007; `script_v0_6_0g`, kept, R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-002";

/// R767000, R2521007: the highest end through v0.6.0g when the table named none: Core #35 Lunar
/// Eclipse, 33/48, the leader of v0.4.0's provisional table, as M #50 CN Tech's.
pub const FALLBACK_LEADER_V0_6_0G: &str = "core-035";
/// R767000, R2521007: the lowest end through v0.6.0g when the table named none: Core #75 Infinite
/// Reserves, 7/43, that table's trailer.
pub const FALLBACK_TRAILER_V0_6_0G: &str = "core-075";

/// R3869005: a missing end is random (`None`); R767000: through v0.6.0g (`kept`) it was `card`, unless
/// the other end was that card already.
fn fallback(kept: bool, card: &str, other_end: Option<&str>) -> Option<String> {
    (kept && other_end != Some(card)).then(|| card.to_string())
}

/// The Cry; `kept`, the script through v0.6.0g.
fn tech(radiant: bool, kept: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let named_leader = win_rate_leader(&*ctx.state, ID);
            let mut skipped = vec![ID];
            skipped.extend(named_leader.as_deref());
            let named_trailer = win_rate_trailer(&*ctx.state, &skipped);
            // R3869005: a missing end is random; R767000: through v0.6.0g, its hard-coded card.
            let leader = named_leader.clone().or_else(|| fallback(kept, FALLBACK_LEADER_V0_6_0G, named_trailer.as_deref()));
            let trailer = named_trailer.or_else(|| fallback(kept, FALLBACK_TRAILER_V0_6_0G, named_leader.as_deref()));
            // R1183001: a named end is its own pool; a missing one is any card but the named ones.
            let named: Vec<&str> = [trailer.as_deref(), leader.as_deref()].into_iter().flatten().collect();
            let pool = |end: &Option<String>| match end {
                Some(id) => json!({ "defId": id }),
                None => json!({ "excludeDefId": named }),
            };
            vec![fuse_generated(json_as(json!({
                "count": 2,
                "pools": [pool(&trailer), pool(&leader)],
                "distinct": true,
                "radiant": radiant,
                "costOverride": param(ctx, "setCost"),
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: tech(false, false),
        radiant: tech(true, false),
    }
}

/// R767000: through v0.6.0g a missing end was Lunar Eclipse or Infinite Reserves (R2521007);
/// `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_0g() -> CardScripts {
    CardScripts {
        base: tech(false, true),
        radiant: tech(true, true),
    }
}

// M+ #2 NA Tech — SPEC §8.9 row 2, BUILD M11 row M+ 2 (R1100, R77, R3869005).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::js;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const LEADER: &str = "core-011";
    const TRAILER: &str = "core-003";
    const MIDDLE: &str = "core-002";
    const FILLER: &str = "core-005";
    /// Core #35 Lunar Eclipse and Core #75 Infinite Reserves: v0.4.0's table's two ends.
    const LUNAR_ECLIPSE: &str = FALLBACK_LEADER_V0_6_0G;
    const INFINITE_RESERVES: &str = FALLBACK_TRAILER_V0_6_0G;

    /// R3869005: whether `id` may stand in for a missing end: a non-token card of a set that ships,
    /// never NA Tech.
    fn a_random_stand_in(state: &GameState, id: &str) -> bool {
        let def = def_of(Some(state), id);
        !def.token && set_ships(def.set) && id != ID
    }

    fn table(cards: Vec<(&str, i32, i32)>) -> WinRateTable {
        WinRateTable {
            patch: "v0.3.X".to_string(),
            source: WinRateSource::Provisional,
            cards: cards
                .into_iter()
                .map(|(id, wins, games)| WinRateRow {
                    id: id.to_string(),
                    wins,
                    games,
                })
                .collect(),
        }
    }

    fn the_table() -> WinRateTable {
        table(vec![(LEADER, 30, 40), (MIDDLE, 20, 40), (TRAILER, 10, 40)])
    }

    fn casting(seed: &str, radiant_face: bool, fillers: usize) -> Scenario {
        crate::register_all();
        let mut hand = vec![json!({ "def": ID, "radiant": radiant_face })];
        hand.extend(vec![json!(FILLER); fillers]);
        scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "library": [FILLER, FILLER, FILLER] },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
        }))
    }

    fn fused(s: &mut Scenario) -> CardInstance {
        s.play(ID, json!({}));
        let added: Vec<CardInstance> = s
            .hand(P1)
            .into_iter()
            .filter(|card| card.def_id != FILLER)
            .collect();
        assert_eq!(added.len(), 1, "one fused card in hand");
        added[0].clone()
    }

    fn ingredients(s: &Scenario, card: &CardInstance) -> Vec<(String, bool)> {
        def_of(Some(s.state()), &card.def_id)
            .ingredients
            .clone()
            .expect("a fused definition lists its ingredients")
            .into_iter()
            .map(|ingredient| (ingredient.def_id, ingredient.radiant == Some(true)))
            .collect()
    }

    mod mp2_na_tech {
        use super::*;

        /// Issue #3138: it costs (0), from (1), on both faces: played with no mana at all, and the fused
        /// card is still added at its own price (the `setCost`, (2) and (0)).
        #[test]
        fn issue_3138_it_costs_0_and_plays_for_no_mana_on_both_faces() {
            for (radiant, set_cost) in [(false, 2), (true, 0)] {
                register_win_rates(the_table());
                let mut s = casting("na-tech-free", radiant, 1);
                s.state_mut().players.p1.mana.current = 0;
                let card = fused(&mut s);
                s.expect_mana(P1, 0);
                assert_eq!(card.cost_override, Some(set_cost), "radiant {radiant}");
            }
        }

        #[test]
        fn r1183001_fuses_the_trailer_and_the_leader_into_one_card_at_2() {
            register_win_rates(the_table());
            let mut s = casting("na-tech", false, 1);
            let card = fused(&mut s);
            assert_eq!(
                ingredients(&s, &card),
                vec![(TRAILER.to_string(), false), (LEADER.to_string(), false)]
            );
            assert_eq!(card.cost_override, Some(2), "it costs (2)");
            assert!(!card.radiant);
        }

        #[test]
        fn r1183001_summed_stats_and_united_keywords_on_a_transient_card() {
            register_win_rates(the_table());
            let mut s = casting("na-tech-stats", false, 1);
            let card = fused(&mut s);
            let def = def_of(Some(s.state()), &card.def_id).clone();
            assert!(
                s.state().transient_defs.contains_key(&def.id),
                "a transient definition (R77)"
            );
            assert!(registered_catalog().get(&def.id).is_none(), "in no pool");
            let (leader, trailer) = (crate::card_def(LEADER), crate::card_def(TRAILER));
            let summed = |a: Option<i32>, b: Option<i32>| Some(a.unwrap_or(0) + b.unwrap_or(0));
            assert_eq!(
                (def.base.attack, def.base.health),
                (
                    summed(leader.base.attack, trailer.base.attack),
                    summed(leader.base.health, trailer.base.health)
                ),
                "summed stats"
            );
            let kinds: Vec<KeywordKind> = def.base.keywords.iter().map(Keyword::kind).collect();
            for kind in [
                KeywordKind::Taunt,
                KeywordKind::DivineShield,
                KeywordKind::Reborn,
                KeywordKind::Rush,
                KeywordKind::FirstStrike,
            ] {
                assert!(kinds.contains(&kind), "{kind:?} from one ingredient or the other");
            }
        }

        #[test]
        fn r1183000_the_lowest_passes_over_thin_rows_and_breaks_ties_as_r1100() {
            register_win_rates(table(vec![
                (LEADER, 30, 40),
                ("core-008", 0, 19),
                (MIDDLE, 5, 20),
                (TRAILER, 10, 40),
            ]));
            let mut s = casting("na-tech-ties", false, 1);
            let card = fused(&mut s);
            assert_eq!(ingredients(&s, &card)[0].0, TRAILER);
        }

        #[test]
        fn r3869005_with_one_eligible_card_it_is_the_leader_and_the_lowest_a_random_card() {
            register_win_rates(table(vec![(LEADER, 30, 40)]));
            for n in 0..4 {
                let mut s = casting(&format!("na-tech-one-{n}"), false, 1);
                let card = fused(&mut s);
                let made = ingredients(&s, &card);
                assert_eq!(made.len(), 2);
                assert_eq!(made[1], (LEADER.to_string(), false), "the table's leader");
                assert_ne!(made[0].0, LEADER, "two different cards");
                assert!(a_random_stand_in(s.state(), &made[0].0), "{}", made[0].0);
            }
        }

        #[test]
        fn r3869005_with_an_empty_table_two_different_random_cards() {
            register_win_rates(table(vec![]));
            let mut seen: Vec<String> = Vec::new();
            for n in 0..6 {
                let mut s = casting(&format!("na-tech-empty-{n}"), false, 1);
                let card = fused(&mut s);
                let made = ingredients(&s, &card);
                assert_eq!(made.len(), 2);
                assert_ne!(made[0].0, made[1].0, "two different cards");
                for (id, radiant) in &made {
                    assert!(a_random_stand_in(s.state(), id), "{id}");
                    assert!(!radiant);
                }
                assert_eq!(card.cost_override, Some(2));
                seen.extend(made.into_iter().map(|(id, _)| id));
            }
            seen.sort();
            seen.dedup();
            assert!(seen.len() > 2, "not the same two on every seed: {seen:?}");
        }

        #[test]
        fn r767000_r2521007_the_script_kept_for_v0_6_0g_fuses_infinite_reserves_and_lunar_eclipse() {
            register_win_rates(table(vec![]));
            crate::under_kept(ID, script_v0_6_0g(), || {
                let mut s = casting("na-tech-kept", false, 1);
                let card = fused(&mut s);
                assert_eq!(
                    ingredients(&s, &card),
                    vec![(INFINITE_RESERVES.to_string(), false), (LUNAR_ECLIPSE.to_string(), false)],
                    "the same two on every seed through v0.6.0g"
                );
            });
            clear_overrides();
        }

        #[test]
        fn r77_the_fused_card_runs_both_scripts() {
            const CRY: &str = "core-015";
            const END_OF_TURN: &str = "core-013";
            register_win_rates(table(vec![(END_OF_TURN, 30, 40), (CRY, 10, 40)]));
            let mut s = casting("na-tech-scripts", false, 1);
            let card = fused(&mut s);
            assert_eq!(
                ingredients(&s, &card),
                vec![(CRY.to_string(), false), (END_OF_TURN.to_string(), false)]
            );
            let enemy_health = s.state().players.p2.hero.health;
            s.play(&card, json!({}));
            let row: Vec<String> = (1..=5)
                .filter_map(|lane| s.unit(P1, lane).map(|unit| unit.def_id))
                .collect();
            assert_eq!(row, vec![card.def_id.clone(), "core-t-rush".to_string()], "the trailer's Cry");
            s.end_turn();
            assert_eq!(
                s.state().players.p2.hero.health,
                enemy_health - 2,
                "the leader's end of turn"
            );
        }

        #[test]
        fn the_fused_card_is_hidden_from_the_opponent() {
            register_win_rates(the_table());
            let mut s = casting("na-tech-hidden", false, 1);
            let card = fused(&mut s);
            let view = js(&view_for(s.state(), P2));
            assert_eq!(view["opponent"]["hand"], json!({ "count": 2 }));
            assert!(!view.to_string().contains(&card.id), "the opponent never sees its id");
        }

        #[test]
        fn a_full_hand_burns_it() {
            register_win_rates(the_table());
            let mut s = casting("na-tech-burn", false, HAND_CAP as usize);
            s.play(ID, json!({}));
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            let burned: Vec<&CardInstance> = s
                .state()
                .players
                .p1
                .graveyard
                .iter()
                .filter(|card| card.def_id != ID)
                .collect();
            assert_eq!(burned.len(), 1, "the fused card burned");
        }

        #[test]
        fn r386_its_set_cost_reads_through_param() {
            register_win_rates(the_table());
            let mut s = casting("na-tech-param", false, 1);
            step_param(s.card_mut(ID), "setCost", 1);
            let card = fused(&mut s);
            assert_eq!(card.cost_override, Some(3));
        }

        #[test]
        fn the_radiant_face_fuses_both_ingredients_radiant_and_it_costs_0() {
            register_win_rates(the_table());
            let mut s = casting("na-tech-radiant", true, 1);
            let card = fused(&mut s);
            assert_eq!(
                ingredients(&s, &card),
                vec![(TRAILER.to_string(), true), (LEADER.to_string(), true)]
            );
            assert!(card.radiant, "every ingredient Radiant: the card is (R469)");
            assert_eq!(card.cost_override, Some(0));
            let def = def_of(Some(s.state()), &card.def_id).clone();
            let (leader, trailer) = (crate::card_def(LEADER), crate::card_def(TRAILER));
            let summed = |a: Option<i32>, b: Option<i32>| Some(a.unwrap_or(0) + b.unwrap_or(0));
            assert_eq!(
                (def.radiant.attack, def.radiant.health),
                (
                    summed(leader.radiant.attack, trailer.radiant.attack),
                    summed(leader.radiant.health, trailer.radiant.health)
                )
            );
            assert!(has_keyword(&def.radiant.keywords, KeywordKind::Charge));
        }

        #[test]
        fn r3869005_the_radiant_face_fuses_two_random_cards_radiant() {
            register_win_rates(table(vec![]));
            let mut s = casting("na-tech-empty-radiant", true, 1);
            let card = fused(&mut s);
            let made = ingredients(&s, &card);
            assert_eq!(made.len(), 2);
            for (id, radiant) in &made {
                assert!(a_random_stand_in(s.state(), id), "{id}");
                assert!(radiant, "{id} is Radiant");
            }
            assert!(card.radiant);
            assert_eq!(card.cost_override, Some(0));
        }

        #[test]
        fn r3049000_fuses_the_ends_of_the_snapshot_the_game_was_handed() {
            crate::register_all();
            register_win_rates(table(vec![(MIDDLE, 30, 40), (FILLER, 10, 40)]));
            let mut s = scenario(json!({
                "seed": "na-tech-snapshot",
                "p1": { "hand": [{ "def": ID }, FILLER], "library": [FILLER, FILLER, FILLER] },
                "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
                "winRates": the_table(),
            }));
            let card = fused(&mut s);
            assert_eq!(
                ingredients(&s, &card),
                vec![(TRAILER.to_string(), false), (LEADER.to_string(), false)],
                "the game's own snapshot, never the table a state with none reads"
            );
            clear_overrides();
        }

        #[test]
        fn r3049001_the_lowest_upper_bound_trails() {
            crate::register_all();
            // 2 wins in 20 games is the worse rate, 50 in 250 the surer one.
            let rows = || table(vec![(LEADER, 30, 40), (MIDDLE, 2, 20), (TRAILER, 50, 250)]);
            let mut s = scenario(json!({
                "seed": "na-tech-bound",
                "p1": { "hand": [{ "def": ID }, FILLER], "library": [FILLER, FILLER, FILLER] },
                "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
                "winRates": rows(),
            }));
            let card = fused(&mut s);
            let made: Vec<String> = ingredients(&s, &card).into_iter().map(|(id, _)| id).collect();
            assert_eq!(made, vec![TRAILER.to_string(), LEADER.to_string()]);
            // A game handed none ranks the table it reads by the rate, as its version did (R1183000).
            register_win_rates(rows());
            let mut s = casting("na-tech-bound", false, 1);
            let card = fused(&mut s);
            assert_eq!(ingredients(&s, &card)[0].0, MIDDLE);
            clear_overrides();
        }

        #[test]
        fn r1266000_r3049001_a_game_on_v0_4_0_still_fuses_its_frozen_tables_ends_by_the_rate() {
            crate::register_all();
            clear_overrides();
            let shipped = crate::shipped_catalog("v0.4.0").expect("v0.4.0 is carried");
            jackioh_engine::with_shipped(Some(shipped), || {
                let mut s = casting("na-tech-v0-4-0", false, 1);
                assert_eq!(s.state().win_rates, None, "a game recorded before #3049");
                // v0.4.0's table, ranked by the rate: Lunar Eclipse (33/48) leads and Infinite
                // Reserves (7/43) trails. Ranked by the bound, Fig of Life's 92 in 146 would lead.
                assert_eq!(
                    leader_of(shipped.win_rates(), WinRateRanking::Bound, &[ID]).as_deref(),
                    Some("core-047")
                );
                assert_eq!(win_rate_leader(s.state(), ID).as_deref(), Some(LUNAR_ECLIPSE));
                assert_eq!(
                    win_rate_trailer(s.state(), &[ID, LUNAR_ECLIPSE]).as_deref(),
                    Some(INFINITE_RESERVES)
                );
                let card = fused(&mut s);
                assert_eq!(
                    ingredients(&s, &card),
                    vec![(INFINITE_RESERVES.to_string(), false), (LUNAR_ECLIPSE.to_string(), false)]
                );
            });
        }

        #[test]
        fn r1101_r3049000_r3869005_handed_no_snapshot_it_fuses_two_random_cards() {
            crate::register_all();
            clear_overrides();
            assert!(registered_win_rates().cards.is_empty(), "no table ships with the build");
            let mut s = casting("na-tech-compiled", false, 1);
            assert_eq!(s.state().win_rates, None, "the scenario was handed none");
            let card = fused(&mut s);
            let made = ingredients(&s, &card);
            assert_eq!(made.len(), 2);
            assert_ne!(made[0].0, made[1].0);
            assert!(made.iter().all(|(id, _)| a_random_stand_in(s.state(), id)), "{made:?}");
        }
    }
}
