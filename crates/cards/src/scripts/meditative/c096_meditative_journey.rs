//! M #96 Meditative Journey (SPEC §8.8 row 96; §6.3 Exile, Shuffle into; R11, R80, R113, R311,
//! R1246, R1247): (2) Spell, Rare.
//!
//! Base:    "Choose up to {cards|card|cards} in your hand to go on a journey: exile them and shuffle a
//!          Journey Complete into your deck."
//! Radiant: "Choose up to {cards|card|cards} in your hand to go on a journey: exile them and shuffle a
//!          Radiant Journey Complete into your deck. Draw {draw|card|cards}."
//! Engine: as the Spell resolves, a `pick` prompt of up to `cards` cards of your hand (`choose_pick`,
//! the chooser's alone, §10.8; `cards` tunes, so it is a resolution prompt, R386). Its step exiles each
//! picked card in the order picked, hand order (R221) (a unit-token card ceases to exist instead, R11),
//! then shuffles one Journey Complete (M #96.1) into your deck, Radiant on the Radiant face (R1247), its
//! `memory.journey` the picked ids in that order (R1246); the shuffle-in is public and recorded in your
//! list (R311), and a full deck refuses it with the cards staying in exile (R80). With nothing picked
//! nothing happens.
//! The Radiant face then draws `draw`, after the step (R113), or at once with an empty hand.

use jackioh_engine::effects::choose_pick;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-096";

/// M #96.1 Journey Complete, which reads the same key (R1246).
const JOURNEY_COMPLETE: &str = "meditative-096-1";
const JOURNEY_KEY: &str = "journey";
const STEP: &str = "journey";

fn journey(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut effects = vec![choose_pick(json_as(json!({
                "step": STEP,
                "from": [{ "zone": "hand" }],
                "max": param(&*ctx, "cards"),
                "prompt": "Choose cards in your hand to go on a journey",
            })))];
            if radiant {
                effects.push(draw(json_as(json!({ "count": param(&*ctx, "draw") }))));
            }
            effects
        })),
        resume: IndexMap::from([(
            STEP,
            hook(move |ctx| {
                let picked: Vec<String> = ctx
                    .targets
                    .iter()
                    .filter_map(|selection| match selection {
                        Selection::Instance { instance_id } => Some(instance_id.clone()),
                        _ => None,
                    })
                    .collect();
                if picked.is_empty() {
                    return Vec::new();
                }
                let mut effects: Vec<Effect> = picked
                    .iter()
                    .map(|id| {
                        exile(json_as(
                            json!({ "target": { "of": "instance", "instanceId": id } }),
                        ))
                    })
                    .collect();
                effects.push(shuffle_into(json_as(json!({
                    "defId": JOURNEY_COMPLETE,
                    "count": 1,
                    "radiant": radiant,
                    "memory": { JOURNEY_KEY: picked },
                }))));
                effects
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: journey(false),
        radiant: journey(true),
    }
}

// M #96 Meditative Journey — SPEC §8.8 row 96, BUILD M10 row M 96: "A `pick` prompt of up to 2 hand cards
// as it resolves; the picks are exiled and one Journey Complete remembering their ids in the order picked
// (`shuffle_into { memory }`) is shuffled into your deck, publicly (R1246); picking none exiles nothing and
// shuffles nothing; a full deck refuses the token and the cards stay in exile; the memory survives a JSON
// round trip and a replay; cards reads through `param()`; radiant up to 5, a Radiant Journey Complete
// (R1247), then draw 1, a draw even when none is picked".
#[cfg(test)]
mod tests {
    use super::{ID, JOURNEY_COMPLETE, JOURNEY_KEY};
    use crate::js;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-005"; // (1) Spell, Stockpile: a spare card that stays in hand.
    const POINTMASTER: &str = "core-020"; // (2) Unit 7/1
    const MENACE: &str = "core-019"; // (3) Unit 9/9
    const HIT_JOB: &str = "core-016"; // (3) Spell
    const SEVEN: &str = "core-025"; // (4) Unit 7/7
    const BIG_FELINOR: &str = "core-043"; // (4) Unit
    const RUSH_TOKEN: &str = "core-t-rush"; // (1) Unit, token

    /// p1 holding the Journey on the given face, then `hand` (each def once), over `library`; 30 mana and a
    /// spare card on each side, so no turn ends by itself.
    fn holding(seed: &str, radiant: bool, hand: &[&str], library: Value) -> Scenario {
        let mut cards = vec![json!({ "def": ID, "radiant": radiant })];
        cards.extend(hand.iter().map(|def| json!(def)));
        crate::scenario(json!({
            "seed": seed,
            "p1": { "hand": cards, "library": library, "mana": 30 },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }))
    }

    fn open(s: &Scenario) -> &PendingChoice {
        s.state()
            .pending
            .as_ref()
            .unwrap_or_else(|| panic!("expected an open prompt"))
    }

    /// The instance id of the one card of `def` in p1's hand.
    fn id_of(s: &Scenario, def: &str) -> String {
        s.hand(P1)
            .into_iter()
            .find(|card| card.def_id == def)
            .unwrap_or_else(|| panic!("no {def} in p1's hand"))
            .id
    }

    fn ids(cards: Vec<CardInstance>) -> Vec<String> {
        cards.into_iter().map(|card| card.id).collect()
    }

    /// The Journey Completes in p1's library.
    fn completes(s: &Scenario) -> Vec<CardInstance> {
        s.pile(P1, "library")
            .into_iter()
            .filter(|card| card.def_id == JOURNEY_COMPLETE)
            .collect()
    }

    fn shuffled_in(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| event.event_type() == GameEventType::ShuffledIn)
            .count()
    }

    mod m96_meditative_journey {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r1246_picked_cards_are_exiled_and_one_journey_complete_remembers_them_in_order() {
                let mut s = holding(
                    "journey-order",
                    false,
                    &[POINTMASTER, MENACE, SEVEN, FILLER],
                    json!([FILLER, FILLER, FILLER]),
                );
                let pointmaster = id_of(&s, POINTMASTER);
                let seven = id_of(&s, SEVEN);
                s.play(ID, json!({}));
                let pending = open(&s);
                assert_eq!(pending.player_id, P1);
                assert_eq!(js(&pending.kind), json!("pick"));
                // R221: the picks are taken in the order the prompt offered them (the hand's), whatever
                // order the answer lists them in, so this listing still means Pointmaster, then the 7/7.
                s.answer(json!([seven, pointmaster]));

                assert_eq!(ids(s.pile(P1, "exile")), vec![pointmaster.clone(), seven.clone()]);
                let library = s.pile(P1, "library");
                assert_eq!(library.len(), 4);
                let completes = completes(&s);
                assert_eq!(completes.len(), 1);
                assert!(!completes[0].radiant);
                assert_eq!(
                    completes[0].memory.get(JOURNEY_KEY),
                    Some(&json!([pointmaster, seven]))
                );
                assert_eq!(
                    s.hand(P1)
                        .iter()
                        .map(|card| card.def_id.as_str())
                        .collect::<Vec<_>>(),
                    vec![MENACE, FILLER]
                );
                s.expect_in_zone(ID, "graveyard");

                // R311: the shuffle-in is open, so p1's list names it; never where it went.
                assert_eq!(
                    js(&s.view(P1).you.own_library),
                    json!({
                        "cards": [
                            { "defId": FILLER, "radiant": false, "count": 3 },
                            { "defId": JOURNEY_COMPLETE, "radiant": false, "count": 1 },
                        ],
                        "unknown": 0,
                    })
                );
                // p2 watched it resolve: the picks lie in p1's public exile and the deck grew by one.
                let theirs = s.view(P2);
                assert_eq!(
                    theirs
                        .opponent
                        .exile
                        .iter()
                        .map(|card| card.instance_id.clone())
                        .collect::<Vec<_>>(),
                    vec![pointmaster, seven]
                );
                assert_eq!(theirs.opponent.library_count, 4);
                assert!(theirs.events.iter().any(|event| matches!(
                    event,
                    GameEvent::ShuffledIn { player, .. } if *player == P1
                )));
            }

            #[test]
            fn r1246_picking_none_shuffles_nothing() {
                let mut s = holding(
                    "journey-none",
                    false,
                    &[POINTMASTER, MENACE, FILLER],
                    json!([FILLER, FILLER]),
                );
                let library = ids(s.pile(P1, "library"));
                s.play(ID, json!({}));
                assert_eq!(js(&open(&s).kind), json!("pick"));
                s.answer(json!([]));

                assert!(s.state().pending.is_none());
                assert!(s.pile(P1, "exile").is_empty());
                assert_eq!(ids(s.pile(P1, "library")), library);
                assert_eq!(shuffled_in(&s), 0);
                assert_eq!(s.hand(P1).len(), 3);
                s.expect_in_zone(ID, "graveyard");
            }

            #[test]
            fn r1246_offers_at_most_two() {
                let mut s = holding(
                    "journey-max",
                    false,
                    &[POINTMASTER, MENACE, SEVEN, FILLER],
                    json!([FILLER]),
                );
                let three = json!([id_of(&s, POINTMASTER), id_of(&s, MENACE), id_of(&s, SEVEN)]);
                s.play(ID, json!({}));
                let pending = open(&s);
                assert_eq!((pending.min, pending.max), (0, 2));
                // Every card of the hand is offered; the Journey itself is resolving, not in hand.
                assert_eq!(pending.options.len(), 4);

                s.expect_refused(move |s| s.answer(three));
                assert_eq!(js(&open(&s).kind), json!("pick"));
                assert!(s.pile(P1, "exile").is_empty());
                assert!(completes(&s).is_empty());
            }

            #[test]
            fn r11_a_unit_token_picked_ceases_to_exist() {
                let mut s = holding(
                    "journey-token",
                    false,
                    &[RUSH_TOKEN, MENACE, FILLER],
                    json!([FILLER]),
                );
                let token = id_of(&s, RUSH_TOKEN);
                let menace = id_of(&s, MENACE);
                s.play(ID, json!({}));
                s.answer(json!([token, menace]));

                s.expect_in_zone(token.as_str(), "gone");
                assert_eq!(ids(s.pile(P1, "exile")), vec![menace.clone()]);
                let completes = completes(&s);
                assert_eq!(completes.len(), 1);
                assert_eq!(
                    completes[0].memory.get(JOURNEY_KEY),
                    Some(&json!([token, menace]))
                );
            }

            #[test]
            fn r80_a_full_deck_refuses_it_and_the_cards_stay_in_exile() {
                let full: Vec<&str> = (0..LIBRARY_CAP).map(|_| FILLER).collect();
                let mut s = holding("journey-full", false, &[POINTMASTER, MENACE, FILLER], json!(full));
                let pointmaster = id_of(&s, POINTMASTER);
                let menace = id_of(&s, MENACE);
                s.play(ID, json!({}));
                s.answer(json!([pointmaster, menace]));

                assert_eq!(s.pile(P1, "library").len(), LIBRARY_CAP as usize);
                assert!(completes(&s).is_empty());
                assert_eq!(shuffled_in(&s), 0);
                assert!(s.events().iter().any(|event| matches!(
                    event,
                    GameEvent::LibraryOverflow { player, def_id, outcome: LibraryOverflowOutcome::NotCreated, .. }
                        if *player == P1 && def_id == JOURNEY_COMPLETE
                )));
                assert_eq!(ids(s.pile(P1, "exile")), vec![pointmaster, menace]);
            }

            #[test]
            fn cards_reads_through_param() {
                let mut s = holding(
                    "journey-param",
                    false,
                    &[POINTMASTER, MENACE, SEVEN, HIT_JOB, FILLER],
                    json!([FILLER]),
                );
                set_param(s.card_mut(ID), "cards", 3);
                let picks = vec![id_of(&s, POINTMASTER), id_of(&s, MENACE), id_of(&s, SEVEN)];
                s.play(ID, json!({}));
                assert_eq!(open(&s).max, 3);
                s.answer(json!(picks));

                assert_eq!(ids(s.pile(P1, "exile")), picks);
                let completes = completes(&s);
                assert_eq!(completes.len(), 1);
                assert_eq!(completes[0].memory.get(JOURNEY_KEY), Some(&json!(picks)));
            }

            #[test]
            fn r1246_the_memory_survives_a_json_round_trip() {
                let mut s = holding(
                    "journey-json",
                    false,
                    &[POINTMASTER, MENACE, FILLER],
                    json!([FILLER]),
                );
                let picks = vec![id_of(&s, POINTMASTER), id_of(&s, MENACE)];
                s.play(ID, json!({}));
                s.answer(json!(picks));

                let revived: GameState =
                    serde_json::from_str(&serde_json::to_string(s.state()).expect("the state serialises"))
                        .expect("the state parses back");
                assert_eq!(&revived, s.state());
                assert_eq!(hash_state(&revived), hash_state(s.state()));
                let remembered: Vec<&Value> = revived.players[P1]
                    .library
                    .iter()
                    .filter_map(|card| card.memory.get(JOURNEY_KEY))
                    .collect();
                assert_eq!(remembered, vec![&json!(picks)]);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r1247_up_to_five_then_a_radiant_journey_complete_then_draw_one() {
                let picked = [POINTMASTER, MENACE, HIT_JOB, SEVEN, BIG_FELINOR];
                let mut hand = picked.to_vec();
                hand.push(FILLER);
                let mut s = holding("journey-radiant", true, &hand, json!([]));
                let picks: Vec<String> = picked.iter().map(|def| id_of(&s, def)).collect();
                s.play(ID, json!({}));
                let pending = open(&s);
                assert_eq!((pending.min, pending.max), (0, 5));
                assert_eq!(pending.options.len(), 6);
                s.answer(json!(picks));

                // The deck held nothing but the Radiant Journey Complete, so the draw after the step
                // (R113) drew it, it was cast, and every picked card came back Radiant, a (1) cheaper.
                s.expect_events(json!(["exiled", "shuffledIn", "drawn", "addedToHand"]));
                assert!(s.events().iter().any(|event| matches!(
                    event,
                    GameEvent::Drawn { player, def_id, .. } if *player == P1 && def_id == JOURNEY_COMPLETE
                )));
                assert!(s.pile(P1, "library").is_empty());
                assert!(s.pile(P1, "exile").is_empty());
                let complete = s
                    .pile(P1, "graveyard")
                    .into_iter()
                    .find(|card| card.def_id == JOURNEY_COMPLETE)
                    .unwrap_or_else(|| panic!("the Journey Complete, cast, in the graveyard"));
                assert!(complete.radiant);
                // §2.4: a cast-on-draw card's draw repeats, here onto the empty deck.
                assert_eq!(s.state().players[P1].fatigue_count, 1);

                for (def, id) in picked.iter().zip(&picks) {
                    s.expect_in_zone(id.as_str(), "hand");
                    let card = s.card(id.as_str());
                    assert!(card.radiant, "{def}");
                    let printed = match crate::card_def(def).cost {
                        CardCost::Fixed(cost) => cost,
                        other => panic!("{def} costs {other:?}"),
                    };
                    assert_eq!(
                        effective_cost(s.state(), card, Default::default()),
                        printed - 1,
                        "{def}"
                    );
                }
                assert_eq!(s.hand(P1).len(), 6);
            }

            #[test]
            fn r1247_draws_even_when_none_is_picked() {
                let mut s = holding(
                    "journey-radiant-none",
                    true,
                    &[POINTMASTER, FILLER],
                    json!([MENACE]),
                );
                let menace = s.card(MENACE).id.clone();
                s.play(ID, json!({}));
                assert_eq!(js(&open(&s).kind), json!("pick"));
                // The draw waits behind the pick's step (R113): nothing is drawn while the prompt is open.
                assert_eq!(s.pile(P1, "library").len(), 1);
                s.answer(json!([]));

                assert_eq!(shuffled_in(&s), 0);
                assert!(s.pile(P1, "exile").is_empty());
                assert!(s.pile(P1, "library").is_empty());
                s.expect_in_zone(menace.as_str(), "hand");
                assert_eq!(s.hand(P1).len(), 3);
            }
        }
    }
}
