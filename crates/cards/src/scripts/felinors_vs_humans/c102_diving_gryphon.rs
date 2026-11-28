//! SPEC §8.14 FvH #102 Diving Gryphon ((1) Unit, Rare, 3/1 → 6/2), a card of the Felinors vs Humans set
//! (R5235000).
//!   Base:    "Rush / Death: Draw a random Rush card from your deck."
//!   Radiant: "Rush / Death: Draw a random Rush card from your deck. Make it Radiant."
//!
//! Rush is catalog data. Death: one uniform pick of the match rng (R60) among its owner's library cards
//! whose keywords, printed or given, hold Rush (read through the layers, §10.4; R5261006), then
//! `draw_from_library` of it (§6.3 Draw: R55's counter, the hand cap, a cast on draw). The pick is made
//! as the effect applies (`for_each_card`'s read, memoized), never as the hook builds it, so a list a
//! drawn card's question pauses resumes on the same card. A deck with no Rush card draws nothing and
//! fatigues nothing (R129). Radiant: the `radiant` rider makes the drawn card Radiant once it is in the
//! hand; one cast on draw or burned stays as it was (R1061). Built by S11 (#5261).

use jackioh_engine::effects::draw_from_library;
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-102";

/// R5261006: one of the library's Rush cards, picked uniformly by the match rng; a lone one is taken
/// with no draw (R129), none leaves nothing.
fn random_rush_card(ctx: &mut EffectContext<'_>) -> Vec<String> {
    let rush: Vec<CardInstance> = zone_cards(ctx.state, ctx.controller, OffFieldZone::Library)
        .into_iter()
        .filter(|card| has_keyword(&unit_view(ctx.state, card).keywords, KeywordKind::Rush))
        .collect();
    let picked = if rush.len() <= 1 {
        rush.first().cloned()
    } else {
        ctx.rng.pick(&rush).cloned()
    };
    picked.map(|card| vec![card.id]).unwrap_or_default()
}

fn gryphon(radiant: bool) -> Script {
    Script {
        death: Some(hook(move |_ctx| {
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(random_rush_card),
                each: Arc::new(move |id| {
                    draw_from_library(json_as(json!({ "instanceId": id, "radiant": radiant })))
                }),
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: gryphon(false),
        radiant: gryphon(true),
    }
}

// FvH #102 Diving Gryphon — SPEC §8.14 row 102 (R5261006).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself
    const RUSHER: &str = "stand-in (1) 3/3 Rush";
    const PLAIN: &str = "stand-in (1) 2/2";
    /// What the Gryphon attacks: it strikes back for 1, which kills the 3/1 (Radiant 6/2 lives).
    const WALL: &str = "stand-in (1) 1/20";
    /// What the Radiant Gryphon attacks: it strikes back for 2.
    const BIG_WALL: &str = "stand-in (1) 2/20";

    fn board(radiant: bool, library: Value) -> Scenario {
        scenario(json!({
            "seed": "diving-gryphon",
            "p1": {
                "field": [{ "def": ID, "lane": 1, "radiant": radiant }],
                "hand": [FILLER],
                "library": library,
            },
            "p2": { "hand": [FILLER], "field": [WALL, BIG_WALL], "library": crate::filler(4) },
        }))
    }

    /// The Gryphon attacks the wall that kills it.
    fn dives(s: &mut Scenario, radiant: bool) {
        let gryphon = s.unit_at(P1, 1);
        let wall = if radiant { s.unit_at(P2, 2) } else { s.unit_at(P2, 1) };
        s.attack(&gryphon, &wall);
        s.expect_in_zone(&gryphon, "graveyard");
    }

    fn drawn(s: &Scenario, def_id: &str) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id == def_id).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn rush_reaches_play_it_attacks_a_unit_the_turn_it_is_played() {
            let mut s = scenario(json!({
                "p1": { "hand": [ID, FILLER], "library": crate::filler(4) },
                "p2": { "hand": [FILLER], "field": [PLAIN], "library": crate::filler(4) },
            }));
            s.play(ID, json!({ "zone": 1 }));
            let gryphon = s.unit_at(P1, 1);
            let enemy = s.unit_at(P2, 1);
            s.attack(&gryphon, &enemy);
            s.expect_in_zone(&enemy, "graveyard");
        }

        #[test]
        fn r5261006_its_death_draws_the_rush_card_from_your_deck() {
            let mut s = board(false, json!([FILLER, RUSHER, PLAIN, FILLER]));
            dives(&mut s, false);
            let got = drawn(&s, RUSHER);
            assert_eq!(got.len(), 1, "the Rush card is drawn");
            assert!(!got[0].radiant);
            let library = crate::def_ids(&s.pile(P1, "library"));
            assert_eq!(library, vec![FILLER, PLAIN, FILLER], "the other cards stay where they were");
        }

        #[test]
        fn r5261006_a_rush_card_is_one_of_the_deck_s_rush_cards_at_random() {
            for seed in 0..6 {
                let mut s = scenario(json!({
                    "seed": format!("diving-gryphon-{seed}"),
                    "p1": {
                        "field": [{ "def": ID, "lane": 1 }],
                        "hand": [FILLER],
                        "library": [RUSHER, PLAIN, { "def": RUSHER, "radiant": true }, FILLER],
                    },
                    "p2": { "hand": [FILLER], "field": [WALL], "library": crate::filler(4) },
                }));
                dives(&mut s, false);
                let library = s.pile(P1, "library");
                assert_eq!(library.len(), 3, "one card is drawn");
                let left: Vec<&CardInstance> = library.iter().filter(|card| card.def_id == RUSHER).collect();
                assert_eq!(left.len(), 1, "one of the two Rush cards is drawn");
                assert_eq!(drawn(&s, RUSHER).len(), 1);
            }
        }

        #[test]
        fn r5261006_a_given_rush_counts() {
            let mut s = scenario(json!({
                "seed": "diving-gryphon-given",
                "p1": {
                    "field": [{ "def": ID, "lane": 1 }],
                    "hand": [FILLER],
                    "library": [FILLER, PLAIN, FILLER],
                },
                "p2": { "hand": [FILLER], "field": [WALL], "library": crate::filler(4) },
            }));
            let plain = must(
                s.pile(P1, "library").into_iter().find(|card| card.def_id == PLAIN),
                "the plain card",
            );
            s.card_mut(&plain).granted_keywords.push(Keyword::Rush);
            dives(&mut s, false);
            s.expect_in_zone(&plain, "hand");
        }

        #[test]
        fn r5261006_with_no_rush_card_in_your_deck_nothing_is_drawn() {
            let mut s = board(false, json!([FILLER, PLAIN]));
            let hand = s.hand(P1).len();
            dives(&mut s, false);
            assert_eq!(s.hand(P1).len(), hand, "nothing drawn");
            assert_eq!(s.pile(P1, "library").len(), 2, "the deck keeps its cards");
            assert_eq!(s.count(GameEventType::Fatigue), 0, "and nothing fatigues");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn rush_reaches_play_it_attacks_a_unit_the_turn_it_is_played() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": ID, "radiant": true }, FILLER], "library": crate::filler(4) },
                "p2": { "hand": [FILLER], "field": [PLAIN], "library": crate::filler(4) },
            }));
            s.play(ID, json!({ "zone": 1 }));
            let gryphon = s.unit_at(P1, 1);
            let enemy = s.unit_at(P2, 1);
            s.attack(&gryphon, &enemy);
            s.expect_in_zone(&enemy, "graveyard");
        }

        #[test]
        fn r5261006_its_death_draws_the_rush_card_and_makes_it_radiant() {
            let mut s = board(true, json!([FILLER, RUSHER, PLAIN]));
            dives(&mut s, true);
            let got = drawn(&s, RUSHER);
            assert_eq!(got.len(), 1, "the Rush card is drawn");
            assert!(got[0].radiant, "and made Radiant");
            assert!(s.hand(P1).iter().filter(|card| card.def_id != RUSHER).all(|card| !card.radiant));
        }
    }
}
