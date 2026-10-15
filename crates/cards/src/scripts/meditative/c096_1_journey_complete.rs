//! M #96.1 Journey Complete (SPEC §8.8 row 96.1, §7; §6.2 Cast on draw, §6.3 Add to hand, Make Radiant;
//! R40, R58, R70, R746, R766, R1248): (2) Spell, Token, printed Rare.
//!
//! Base:    "Cast on draw: Return the cards that went on this journey from exile to hand. They become
//!          Radiant."
//! Radiant: "Cast on draw: Return the cards that went on this journey from exile to hand. They become
//!          Radiant and cost ({discount}) less."
//! Engine: §6.2 Cast on draw (`static_flags.cast_on_draw`), a cast (R40, R70) under R58's chain cap. It
//! reads the `memory.journey` M #96 wrote on it (R1246) and, for each id in that order whose card is
//! still in an exile pile, `add_to_hand` moves it to its owner's hand (R746), Radiant (§6.3 Make
//! Radiant, R74), keeping its `tuning`; on the Radiant face it costs `discount` less, a price that lands
//! after R766 reset it in exile. The hand cap burns the rest (§2.4). One that remembers nothing — made
//! by any other card, a copy, or one that reached a graveyard or an exile pile (R766) — does nothing
//! (R1248).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-096-1";

/// What M #96 Meditative Journey writes (R1246).
const JOURNEY_KEY: &str = "journey";

fn journey_complete(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(move |ctx| {
            let went: Vec<String> = ctx
                .live_self()
                .and_then(|me| me.memory.get(JOURNEY_KEY))
                .and_then(Value::as_array)
                .map(|ids| ids.iter().filter_map(Value::as_str).map(str::to_string).collect())
                .unwrap_or_default();
            let discount = param(&*ctx, "discount");
            went.into_iter()
                .filter(|id| {
                    find_instance(ctx.state, id).is_some_and(|card| card.zone.z() == ZoneName::Exile)
                })
                .map(|id| {
                    let mut args = json!({
                        "instance": { "of": "instance", "instanceId": id },
                        "radiant": true,
                    });
                    if radiant {
                        args["costMod"] = json!(-discount);
                    }
                    add_to_hand(json_as(args))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: journey_complete(false),
        radiant: journey_complete(true),
    }
}

// M #96.1 Journey Complete — SPEC §8.8 row 96.1, R1248: "cast on draw, it returns every card its
// `memory.journey` names that is still in an exile pile, in that order, to its owner's hand, Radiant;
// the hand cap burns the rest; one that left exile stays gone; one that remembers nothing does nothing;
// radiant: each costs `discount` (1) less than printed, after R766's reset in exile".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const SEVEN: &str = "core-025"; // (4) Unit 7/7, no text.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const FILLER: &str = "core-005"; // (1) Spell, never played here.

    /// p1 with `exile` in their exile pile, `hand` in hand and a Journey Complete (on `radiant`'s face)
    /// on top of their library, so their next draw casts it; the draw repeats onto a FILLER (§2.4).
    fn journey(radiant: bool, hand: Value, exile: Value) -> Scenario {
        crate::scenario(json!({
            "p1": {
                "hand": hand,
                "exile": exile,
                "library": [{ "def": ID, "radiant": radiant }, FILLER, FILLER],
            },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }))
    }

    fn ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.id.clone()).collect()
    }

    /// What M #96 writes on the Journey Complete it shuffles in (R1246).
    fn remember(s: &mut Scenario, went: &[String]) {
        s.card_mut(ID).memory.insert(JOURNEY_KEY.to_string(), json!(went));
    }

    /// The ids in p1's hand that are among `of`, in hand order.
    fn held_of(s: &Scenario, of: &[String]) -> Vec<String> {
        ids(&s.hand(P1))
            .into_iter()
            .filter(|id| of.contains(id))
            .collect()
    }

    fn cost(s: &Scenario, id: &str) -> i32 {
        effective_cost(s.state(), s.card(id), Default::default())
    }

    fn printed(s: &Scenario, id: &str) -> i32 {
        printed_cost(s.state(), s.card(id))
    }

    /// Moves p1's hand card `id` to their exile pile the engine's way, so R766 takes its price.
    fn exile_from_hand(s: &mut Scenario, id: &str) {
        let mut card = s.card(id).clone();
        move_to_zone(s.state_mut(), &mut card, OffFieldZone::Exile, Default::default());
        s.expect_in_zone(id, "exile");
    }

    mod base {
        use super::*;

        #[test]
        fn is_cast_on_draw() {
            let scripts = script();
            for face in [&scripts.base, &scripts.radiant] {
                assert_eq!(
                    face.static_flags.as_ref().and_then(|flags| flags.cast_on_draw),
                    Some(true)
                );
            }

            let mut s = journey(false, json!([FILLER]), json!([]));
            let me = s.card(ID).id.clone();
            s.start_turn();

            // Cast as it was drawn: it never sat in the hand and has landed in the graveyard, and the
            // draw repeated onto the FILLER beneath it (§2.4).
            assert!(s.hand(P1).iter().all(|card| card.id != me));
            s.expect_in_zone(me.as_str(), "graveyard");
            assert_eq!(s.hand(P1).len(), 2);
            assert!(s.events().iter().any(|event| matches!(
                event,
                GameEvent::Drawn { instance_id, .. } if *instance_id == me
            )));
        }

        #[test]
        fn r1248_returns_every_remembered_card_from_exile_radiant_in_order() {
            let mut s = journey(false, json!([FILLER]), json!([SEVEN, HIT_JOB]));
            let exiled = ids(&s.pile(P1, "exile"));
            // Remembered the other way round from how they lie in exile: the memory's order rules.
            let went = vec![exiled[1].clone(), exiled[0].clone()];
            remember(&mut s, &went);
            assert!(s.pile(P1, "exile").iter().all(|card| !card.radiant));

            s.start_turn();

            assert_eq!(held_of(&s, &went), went);
            assert!(s.pile(P1, "exile").is_empty());
            for id in &went {
                let card = s.card(id.as_str());
                assert!(card.radiant, "{id} came back Radiant");
                assert_eq!(card.owner, P1);
                // The base face gives no discount: the printed cost.
                assert_eq!(cost(&s, id), printed(&s, id));
            }
            let added: Vec<String> = s
                .events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::AddedToHand { instance_id, .. } if went.contains(instance_id) => {
                        Some(instance_id.clone())
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(added, went);
        }

        #[test]
        fn r1248_one_that_left_exile_stays_gone() {
            let mut s = journey(false, json!([FILLER]), json!([SEVEN, HIT_JOB]));
            let went = ids(&s.pile(P1, "exile"));
            remember(&mut s, &went);
            // The 7/7 leaves exile for the graveyard before the Journey Complete is drawn.
            let mut left = s.card(went[0].as_str()).clone();
            move_to_zone(
                s.state_mut(),
                &mut left,
                OffFieldZone::Graveyard,
                Default::default(),
            );
            s.expect_in_zone(went[0].as_str(), "graveyard");

            s.start_turn();

            s.expect_in_zone(went[0].as_str(), "graveyard");
            assert!(!s.card(went[0].as_str()).radiant);
            assert_eq!(held_of(&s, &went), vec![went[1].clone()]);
            assert!(s.card(went[1].as_str()).radiant);
        }

        #[test]
        fn r1248_one_that_remembers_nothing_does_nothing() {
            let mut s = journey(false, json!([FILLER]), json!([SEVEN, HIT_JOB]));
            let exiled = ids(&s.pile(P1, "exile"));
            let me = s.card(ID).id.clone();
            assert!(s.card(me.as_str()).memory.is_empty());

            s.start_turn();

            // It was cast and nothing moved: the exile pile is as it was, the hand only has the repeat.
            s.expect_in_zone(me.as_str(), "graveyard");
            assert_eq!(ids(&s.pile(P1, "exile")), exiled);
            assert!(s.pile(P1, "exile").iter().all(|card| !card.radiant));
            assert_eq!(s.hand(P1).len(), 2);
            assert!(held_of(&s, &exiled).is_empty());
        }

        #[test]
        fn r1248_returns_all_five() {
            let mut s = journey(
                false,
                json!([FILLER]),
                json!([SEVEN, HIT_JOB, SEVEN, HIT_JOB, SEVEN]),
            );
            let went = ids(&s.pile(P1, "exile"));
            remember(&mut s, &went);

            s.start_turn();

            assert_eq!(held_of(&s, &went), went);
            assert!(s.pile(P1, "exile").is_empty());
            assert!(went.iter().all(|id| s.card(id.as_str()).radiant));
            // The FILLER held, the five, and the repeat of the draw.
            assert_eq!(s.hand(P1).len(), 7);
        }

        #[test]
        fn s2_4_the_cap_burns_the_rest() {
            crate::register_all();
            let cap = HAND_CAP as usize;
            let mut s = journey(
                false,
                json!(vec![FILLER; cap - 1]),
                json!([SEVEN, HIT_JOB, SEVEN]),
            );
            assert_eq!(hand_cap_of(s.state(), P1), HAND_CAP);
            let went = ids(&s.pile(P1, "exile"));
            remember(&mut s, &went);

            s.start_turn();

            // The first fills the hand; the other two are burned to the graveyard (§2.4, R4).
            assert_eq!(s.hand(P1).len(), cap);
            assert_eq!(held_of(&s, &went), vec![went[0].clone()]);
            for id in &went[1..] {
                s.expect_in_zone(id.as_str(), "graveyard");
                assert!(s.events().iter().any(|event| matches!(
                    event,
                    GameEvent::Burned { instance_id, .. } if instance_id == id
                )));
            }
            assert!(s.pile(P1, "exile").is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1248_they_cost_one_less_than_printed() {
            let mut s = journey(
                true,
                json!([FILLER, { "def": SEVEN, "costMod": 2 }, { "def": HIT_JOB, "costMod": -1 }]),
                json!([]),
            );
            let hand = ids(&s.hand(P1));
            let went = vec![hand[1].clone(), hand[2].clone()];
            assert_eq!((cost(&s, &went[0]), cost(&s, &went[1])), (6, 2));
            // Exiled from the hand, the way M #96 exiles them: R766 takes both prices.
            for id in &went {
                exile_from_hand(&mut s, id);
                assert_eq!(s.card(id.as_str()).cost_mod, 0);
            }
            remember(&mut s, &went);

            s.start_turn();

            assert_eq!(held_of(&s, &went), went);
            for id in &went {
                assert!(s.card(id.as_str()).radiant);
                assert_eq!(cost(&s, id), printed(&s, id) - 1);
            }
            assert_eq!((cost(&s, &went[0]), cost(&s, &went[1])), (3, 2));
        }

        #[test]
        fn discount_reads_through_param() {
            let mut s = journey(true, json!([FILLER]), json!([SEVEN]));
            let went = ids(&s.pile(P1, "exile"));
            remember(&mut s, &went);
            set_param(s.card_mut(ID), "discount", 3);

            s.start_turn();

            assert_eq!(held_of(&s, &went), went);
            assert_eq!(cost(&s, &went[0]), 1);
        }
    }
}
