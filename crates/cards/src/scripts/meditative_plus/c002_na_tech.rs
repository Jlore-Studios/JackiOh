//! M+ #2 NA Tech (SPEC §8.9 row 2; R1100, R1183000, R1183001): (1) Spell, Epic.
//!
//! Base:    "Fuse the card with the lowest win rate and the card with the highest win rate, and add
//!          it to your hand. It costs ({setCost})."
//! Radiant: "Fuse a Radiant card with the lowest win rate and a Radiant card with the highest win
//!          rate, and add it to your hand. It costs ({setCost})."
//! Engine: M #50 CN Tech's compiled table read from both ends — the leader (`win_rate_leader`,
//! R1100) and the trailer (`win_rate_trailer`, R1183000: the same floor and tie-breaks, the lowest
//! rate, never the leader), neither ever NA Tech itself (R387) — fused trailer first into one
//! transient hand card (`fuse_generated`'s two pools, R1108; R77, R102) at a `costOverride` of
//! setCost (R1183001). An end the table cannot name is a random non-token card of the sets that ship,
//! as CN Tech's fallback, never the other ingredient (`distinct`). The Radiant face fuses both
//! ingredients on their Radiant faces (R469). Tunes: setCost 2 ↓ (Radiant 0).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-002";

fn tech(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let leader = win_rate_leader(ID);
            let mut skipped = vec![ID];
            skipped.extend(leader.as_deref());
            let trailer = win_rate_trailer(&skipped);
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
        base: tech(false),
        radiant: tech(true),
    }
}

// M+ #2 NA Tech — SPEC §8.9 row 2, BUILD M11 row M+ 2: "with a fixture win-rate table, fuses the
// trailer and the leader (both eligible past the games floor, ties as R1100) into one transient card
// (summed stats, union of keywords, both scripts, R77) added to your hand at (2); with an empty table
// two different random non-token cards of the shipped sets stand in; a full hand burns it; setCost
// reads through `param()`; radiant both ingredients Radiant and it costs (0)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// Tempo Timmy, 3/3 Rush and First Strike (Radiant 6/6 Charge and First Strike): the leader.
    const LEADER: &str = "core-011";
    /// Right-house defender, 1/1 Taunt, Divine Shield and Reborn (Radiant 2/2): the trailer.
    const TRAILER: &str = "core-003";
    /// Bigot, between the two.
    const MIDDLE: &str = "core-002";
    const FILLER: &str = "core-005";

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

    /// p1 holds NA Tech (base unless `radiant_face`) and `fillers` fillers; both sides keep cards in
    /// hand so no turn auto-ends.
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

    /// Cast NA Tech; the card it added to p1's hand.
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

    /// The fused card's ingredients, in fusion order, with their Radiant flags.
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
            // A thin row (fewer than 20 games) never trails; 10/40 and 5/20 tie, more games wins.
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
        fn r1183001_with_one_eligible_card_it_is_the_leader_and_a_random_card_the_lowest() {
            register_win_rates(table(vec![(LEADER, 30, 40)]));
            let mut s = casting("na-tech-one", false, 1);
            let card = fused(&mut s);
            let made = ingredients(&s, &card);
            assert_eq!(made[1].0, LEADER);
            assert_ne!(made[0].0, LEADER, "two different cards");
            let stand_in = def_of(Some(s.state()), &made[0].0);
            assert!(!stand_in.token, "a non-token card stands in");
            assert_ne!(stand_in.id, ID, "never NA Tech itself");
        }

        #[test]
        fn r1183001_with_an_empty_table_two_different_random_cards_stand_in() {
            register_win_rates(table(vec![]));
            for n in 0..8 {
                let mut s = casting(&format!("na-tech-empty-{n}"), false, 1);
                let card = fused(&mut s);
                let made = ingredients(&s, &card);
                assert_eq!(made.len(), 2);
                assert_ne!(made[0].0, made[1].0, "two different cards");
                for (id, _) in &made {
                    let def = def_of(Some(s.state()), id);
                    assert!(!def.token, "{id} is no token");
                    assert!(set_ships(def.set), "{id} is of a set that ships");
                    assert_ne!(id, ID);
                }
            }
        }

        #[test]
        fn the_fused_card_is_hidden_from_the_opponent() {
            register_win_rates(the_table());
            let mut s = casting("na-tech-hidden", false, 1);
            let card = fused(&mut s);
            let view = serde_json::to_value(view_for(s.state(), P2)).expect("the view serialises");
            assert_eq!(view["opponent"]["hand"], json!({ "count": 2 }));
            assert!(!view.to_string().contains(&card.id), "the opponent never sees its id");
        }

        #[test]
        fn a_full_hand_burns_it() {
            register_win_rates(the_table());
            // Eleven in hand, ten once NA Tech is cast: the hand is full as the card arrives.
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
            // The Radiant faces summed, Charge from Tempo Timmy's Radiant face.
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
        fn r1101_the_compiled_table_ships_empty_so_the_card_fuses_two_random_cards() {
            crate::register_all();
            clear_overrides();
            assert_eq!(win_rate_trailer(&[ID]), None);
            assert_eq!(win_rate_leader(ID), None);
        }
    }
}
