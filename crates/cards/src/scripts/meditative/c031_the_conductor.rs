//! M #31 The Conductor (SPEC §8.8 row 31): (3) Unit, Human, Rare, 7/9 → 14/18.
//!
//! Base:    "Deft
//!           Cry: Draw every Created card in your deck."
//! Radiant: "Deft
//!           Cry: Shuffle a random Prime card into your deck. Then draw every Created card in your
//!           deck."
//! Engine:
//! - **Deft** is the §6.1 keyword (catalog data).
//! - **The Cry** reads, as it begins, the ids of the Created cards in its controller's library,
//!   top down (`query::created_in_library`, MD-B6), and returns one named `draw_from_library`
//!   per id. Each is a real §2.4 draw (MD-B7, R944): a `drawn` event and R55's counter, R4's hand
//!   cap burning the overflow, a cast-on-draw card cast with its top-of-deck replacement draw
//!   (R58), R457's draw limit. An id gone from the library by its turn fizzles, and a card that
//!   enters during the draws is not drawn (`for_each_card` reads the list once).
//! - **The Radiant face** first shuffles in a random Prime card (`shuffle_random_from_catalog`
///!   over `{ tags: [Prime] }` — every Prime card is a token, so Prime joins the pool's token tags,
///!   R1421). That card is Created, so the same Cry draws it.

use jackioh_engine::effects::{ForEachCardArgs, for_each_card, shuffle_random_from_catalog};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-031";

/// One named draw per Created card in the controller's library, top down (MD-B7, R944).
fn draw_created() -> Effect {
    for_each_card(ForEachCardArgs {
        cards: Arc::new(|ctx: &mut EffectContext<'_>| -> Vec<String> {
            created_in_library(ctx.state, ctx.controller)
        }),
        each: Arc::new(|instance_id: &str| {
            draw_from_library(json_as(json!({ "instanceId": instance_id })))
        }),
    })
}

fn conductor(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            if !radiant {
                return vec![draw_created()];
            }
            vec![
                shuffle_random_from_catalog(json_as(json!({
                    "query": { "tags": ["Prime"] },
                    "count": 1,
                }))),
                draw_created(),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: conductor(false),
        radiant: conductor(true),
    }
}

// M #31 The Conductor — SPEC §8.8 row 31, BUILD M10 row M 31.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const CONDUCTOR: &str = "meditative-031";
    const CURSE: &str = "meditative-028-1"; // Ancient Curse: cast on draw, take 7.
    const VANILLA: &str = "core-008"; // (1) 4/4 Human.
    const HIT_JOB: &str = "core-016"; // Destroy target unit.
    const RECYCLE: &str = "classic-030"; // Shuffle your graveyard into your deck, then draw 1.
    const PALANTIR: &str = "classic-004"; // Aura: your opponent can't draw more than 1 each turn.
    const FILLER: &str = "core-005";

    fn library() -> Value {
        json!([FILLER, FILLER, FILLER, FILLER, FILLER, FILLER])
    }

    /// p1 holds the Conductor (Radiant on `radiant`) over `library`; p2 holds fillers.
    fn conducting(seed: &str, radiant: bool, library: Value) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": {
                "mana": 10,
                "hand": [{ "def": CONDUCTOR, "radiant": radiant }, FILLER],
                "library": library,
            },
            "p2": { "hand": [FILLER], "library": self::library() },
        }))
    }

    fn def_ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.def_id.clone()).collect()
    }

    #[test]
    fn r944_draws_every_created_card_top_down_and_leaves_dealt_cards() {
        let mut s = conducting(
            "conductor-draw",
            false,
            json!([{ "def": VANILLA, "created": true }, FILLER, { "def": VANILLA, "created": true }]),
        );
        s.play(CONDUCTOR, json!({}));
        // Both Created cards drawn, top down; the dealt card stays.
        let hand = def_ids(&s.hand(P1));
        assert_eq!(hand[hand.len() - 2..], [VANILLA.to_string(), VANILLA.to_string()]);
        assert_eq!(s.pile(P1, "library").len(), 1, "the dealt card stays");
        assert_eq!(s.pile(P1, "library")[0].def_id, FILLER);
        // The drawn cards keep their mark.
        for card in s.hand(P1).into_iter().skip(1) {
            if card.def_id == VANILLA {
                assert_eq!(card.created, Some(true), "drawn cards stay Created");
            }
        }
    }

    #[test]
    fn r944_a_cast_on_draw_curse_is_cast_with_its_replacement_draw() {
        let mut s = conducting(
            "conductor-curse",
            false,
            json!([{ "def": CURSE, "created": true }, FILLER]),
        );
        let health_before = s.state().players[P1].hero.health;
        s.play(CONDUCTOR, json!({}));
        // The Curse cast on draw (take 7), then the top-of-deck replacement draw.
        assert_eq!(s.state().players[P1].hero.health, health_before - 7, "the Curse is cast");
        assert!(s.pile(P1, "library").is_empty(), "the replacement draw took the top");
        assert_eq!(s.hand(P1).len(), 2, "Conductor played, replacement in hand");
        assert_eq!(s.hand(P1)[1].def_id, FILLER, "the replacement draw is the top card");
    }

    #[test]
    fn r944_the_hand_cap_burns_the_overflow_and_the_draw_limit_stops_the_rest() {
        // Hand cap: p1 holds 9 fillers plus the Conductor (10, at the cap once it is played …).
        crate::register_all();
        let mut hand = vec![json!({ "def": CONDUCTOR })];
        for _ in 0..9 {
            hand.push(json!(FILLER));
        }
        let mut s = scenario(json!({
            "seed": "conductor-cap",
            "p1": {
                "mana": 10,
                "hand": hand,
                "library": [{ "def": VANILLA, "created": true }, { "def": VANILLA, "created": true }],
            },
            "p2": { "hand": [FILLER], "library": self::library() },
        }));
        s.play(CONDUCTOR, json!({}));
        // Nine in hand after the play, two draws: the first lands (10), the second burns.
        assert_eq!(s.hand(P1).len(), 10, "the cap holds");
        let burned: Vec<_> = s.pile(P1, "graveyard").into_iter().filter(|card| card.def_id == VANILLA).collect();
        assert_eq!(burned.len(), 1, "the overflow burns");
        assert!(s.hand(P1).into_iter().any(|card| card.def_id == VANILLA), "the first draw lands");

        // Draw limit: Palantir on p2's field lets p1 draw 1 this turn.
        let mut s = conducting(
            "conductor-limit",
            false,
            json!([{ "def": VANILLA, "created": true }, { "def": VANILLA, "created": true }, { "def": VANILLA, "created": true }]),
        );
        {
            // Palantir is a Field Spell: it acts from p2's backrow.
            let state = s.state_mut();
            let lane = first_free_zone(state, P2, Row::Backrow).expect("room");
            let mut card = new_instance(state, PALANTIR, P2, zone_of(lane));
            assert!(place_on_field(state, &mut card, lane, Default::default()));
        }
        let mark = s.events().len();
        s.play(CONDUCTOR, json!({}));
        assert_eq!(s.pile(P1, "library").len(), 2, "the limit stops the rest");
        assert!(
            s.events()[mark..].iter().any(|event| matches!(event, GameEvent::DrawLimited { .. })),
            "the limit is reported"
        );
    }

    #[test]
    fn r943_a_drawn_card_shuffled_back_keeps_its_mark() {
        // Conductor draws a Created Vanilla; Hit Job destroys it; Recycle shuffles it back.
        crate::register_all();
        let mut s = scenario(json!({
            "seed": "conductor-keep",
            "p1": {
                "mana": 10,
                "hand": [{ "def": CONDUCTOR }, { "def": HIT_JOB }, { "def": RECYCLE }, FILLER],
                "library": [{ "def": VANILLA, "created": true }, FILLER, FILLER],
            },
            "p2": { "hand": [FILLER], "library": self::library() },
        }));
        s.play(CONDUCTOR, json!({}));
        let drawn = s.hand(P1).into_iter().find(|card| card.def_id == VANILLA).expect("drawn");
        assert_eq!(drawn.created, Some(true), "the draw keeps the instance");
        let drawn_id = drawn.id.clone();
        s.play(VANILLA, json!({}));
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": drawn_id }] }));
        s.play(RECYCLE, json!({}));
        // Recycle also draws 1, so the Vanilla is in the hand or back in the library — marked either way.
        let state = s.state();
        let in_hand = state.players[P1].hand.iter().any(|card| card.id == drawn_id && card.created == Some(true));
        let in_library = created_in_library(state, P1).contains(&drawn_id);
        assert!(in_hand || in_library, "the shuffled-back card keeps its mark");
        if in_library {
            let list = own_library_view(state, P1);
            assert!(
                list.cards.iter().any(|entry| entry.created == Some(true)),
                "the owner's list marks it"
            );
        }
    }

    #[test]
    fn r944_radiant_shuffles_in_a_prime_card_and_draws_it() {
        let _preview = preview_sets(&[SetName::Meditative]);
        let mut s = conducting("conductor-prime", true, json!([FILLER, FILLER]));
        let mark = s.events().len();
        s.play(CONDUCTOR, json!({}));
        // The shuffled-in Prime is Created, so the same Cry draws it.
        let hand = s.hand(P1);
        let prime = hand.iter().find(|card| tags_of(s.state(), card).contains(&Tag::Prime));
        assert!(prime.is_some(), "a Prime card was shuffled in and drawn");
        assert_eq!(prime.expect("drawn").created, Some(true), "it is Created");
        let _ = mark;
    }
}
