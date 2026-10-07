//! C #15 Nose Hunter (SPEC §8.6 row 15). Unit 3/1 → 6/2, Human, cost 1, Common.
//!   Base:    "Activate ♾️: Discard a random card. Exile the bottom {exile|card|cards} of your opponent's deck."
//!   Radiant: "… Exile the bottom {exile|card|cards} of your opponent's deck and a random card from their hand."
//!
//! R392: the designer's "Discard a random card: Exile …" is an Activate ♾️ ability (balance patch 1;
//! R384) — "cost: effect" is how a card is clicked to do an effect — so the random discard is the
//! ability's cost, paid as it is activated (`cost.discardRandom`), and with an empty hand it cannot be
//! activated at all (`subsystems/activate.ts` refuses it, and so `legalActions` never lists it). The
//! discard is an ordinary discard (§6.3), so C #64 Malzahar's Recycler sees it. "Each opponent" in the
//! designer's text is the multiplayer phrasing Heroic Power uses (R45): with two players, the opponent.
//!
//! Activating is not attacking, so it is usable the turn Nose Hunter arrives and never spends an
//! exertion; nor is it a play, so nothing that counts plays sees it (R384).
//!
//! The exile reads the bottom of the deck as `exileBottomOfLibrary` does (the last element; an empty
//! deck exiles nothing and deals no fatigue, since this is not a draw). The Radiant face's "a random
//! card from their hand" is `exileRandomFromHand` (R60; an empty hand: nothing). No event names a deck
//! position; the exiled cards are public once in exile (§3.2).

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-015";

/// "Exile the bottom {exile} card(s) of your opponent's deck" (Radiant: "… and a random card from their hand").
fn hunt(ctx: &EffectContext<'_>, from_hand: bool) -> Vec<Effect> {
    let bottom = exile_bottom_of_library(json_as(json!({ "player": "enemy", "count": param(ctx, "exile") })));
    if from_hand {
        vec![bottom, exile_random_from_hand(json_as(json!({ "player": "enemy", "count": 1 })))]
    } else {
        vec![bottom]
    }
}

fn ability(from_hand: bool) -> ActivationDecl {
    ActivationDecl {
        id: "hunt".into(),
        label: if from_hand {
            "Discard a random card. Exile the bottom of your opponent's deck and a random card from their hand".into()
        } else {
            "Discard a random card. Exile the bottom of your opponent's deck".into()
        },
        uses: ActivationUses::Unlimited,
        cost: Some(ActivationCost { discard_random: Some(1), ..ActivationCost::default() }),
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(move |ctx| hunt(ctx, from_hand)),
    }
}

pub fn script() -> CardScripts {
    let base = Script { activations: vec![ability(false)], ..Script::default() };

    let radiant = Script { activations: vec![ability(true)], ..Script::default() };
    CardScripts { base, radiant }
}

// C #15 Nose Hunter — SPEC §8.6 row 15, BUILD M9 Classic row C 15: "Activate Infinity (R392,
// R384): its cost, discarding a random card of yours, is paid as it activates, so with an empty hand
// it can't activate and `legalActions` doesn't list it; then exile the bottom card of the opponent's
// deck (an empty deck: nothing); usable the turn it is played (no sickness, no exertion), only in your
// main phase; as many activations a turn as its cost can be paid; not a play (R384); the discard is a
// discard (C #64 sees it); no event carries a deck position; radiant 6/2: also exile a random card from
// their hand (empty: nothing), public once in exile; its tuned number (exiled) reads through `param()`
// (R386)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    /// Card tests run on the real catalog and scripts (TS: the vitest globalSetup's `registerAll`).
    fn scenario(setup: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(setup)
    }

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const NOSE: &str = "classic-015";
    const FILLER: &str = "core-005"; // (1) Spell Stockpile: a card to discard, or to keep a hand from auto-ending (§2.5).
    const VANILLA: &str = "core-008"; // (1) 4/4 Unit.
    const TIMMY: &str = "core-011"; // (1) 3/3.
    const MENACE: &str = "core-019"; // (3) 9/9.
    const POINTMASTER: &str = "core-020"; // (2) 7/1.

    /// p2's deck, top first: the bottom card is Pointmaster, so "the bottom card" is visible by def.
    const DECK: [&str; 3] = [VANILLA, TIMMY, POINTMASTER];

    /// TS `{ ...base, ...extra }` on a setup object: `extra`'s keys replace `base`'s.
    fn merged(base: Value, extra: Value) -> Value {
        let mut out = base;
        if let (Some(map), Value::Object(extra)) = (out.as_object_mut(), extra) {
            for (key, value) in extra {
                map.insert(key, value);
            }
        }
        out
    }

    /// TS `setup(p1, p2 = {})`.
    fn setup(p1: Value, p2: Value) -> Scenario {
        scenario(json!({
            "p1": merged(json!({ "library": [FILLER, FILLER] }), p1),
            "p2": merged(json!({ "hand": [FILLER], "library": DECK }), p2),
        }))
    }

    /// The legal `activate` (or legacy `activatePower`) actions naming this card.
    fn activations(s: &Scenario, player: PlayerId, instance_id: &str) -> Vec<Value> {
        legal_actions(s.state(), player)
            .iter()
            .map(|action| serde_json::to_value(action).unwrap())
            .filter(|action| {
                (action["type"] == "activate" || action["type"] == "activatePower") && action["instanceId"] == instance_id
            })
            .collect()
    }

    fn defs_of(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.def_id.clone()).collect()
    }

    /// TS `stepParam(s.card(ref), key, steps)` on the live card.
    fn step(s: &mut Scenario, card: &str, key: &str, steps: i32) {
        let id = s.card(card).id.clone();
        match find_instance_mut(s.state_mut(), &id) {
            Some(live) => step_param(live, key, steps),
            None => panic!("no card {card} to tune"),
        }
    }

    /// The def ids a viewer reads in an exile pile of their view (`you` or `opponent`).
    fn exile_seen(s: &Scenario, viewer: PlayerId, side: &str) -> Vec<String> {
        let view = serde_json::to_value(s.view(viewer)).unwrap();
        view[side]["exile"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|card| card.get("defId").and_then(Value::as_str).unwrap_or("").to_string())
            .collect()
    }

    #[test]
    fn declares_one_activate_infinity_ability_on_each_face_whose_cost_is_a_random_discard() {
        assert_eq!(crate::card_def(ID).id, NOSE);
        let CardScripts { base, radiant } = script();
        for face in [&base, &radiant] {
            assert_eq!(face.activations.len(), 1);
            assert_eq!(face.activations.first().map(|decl| decl.uses), Some(ActivationUses::Unlimited));
            assert_eq!(
                face.activations.first().map(|decl| serde_json::to_value(decl.cost).unwrap()),
                Some(json!({ "discardRandom": 1 }))
            );
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r392_activating_discards_a_random_card_of_yours_and_exiles_the_bottom_card_of_the_opponent_s_deck() {
            let mut s = setup(json!({ "hand": [FILLER], "field": [NOSE] }), json!({}));
            let (Some(discarded), Some(bottom)) =
                (s.hand(P1).first().cloned(), s.pile(P2, "library").get(2).cloned())
            else {
                panic!("fixture");
            };

            s.activate(NOSE, json!({}));

            s.expect_in_zone(&discarded, "graveyard");
            s.expect_in_zone(&bottom, "exile");
            assert_eq!(defs_of(&s.pile(P2, "library")), [VANILLA, TIMMY]);
            assert_eq!(defs_of(&s.pile(P2, "exile")), [POINTMASTER]);
            s.expect_events(json!(["activated", "discarded", "exiled"]));
        }

        #[test]
        fn r392_the_discard_is_its_cost_with_an_empty_hand_it_can_t_be_activated_and_legalactions_doesn_t_list_it() {
            let mut s = setup(json!({ "hand": [], "field": [VANILLA, NOSE] }), json!({}));
            let nose = s.card(NOSE).clone();

            assert!(activations(&s, P1, &nose.id).is_empty());
            s.expect_refused(|s| s.activate(&nose, json!({})));
            assert!(s.pile(P2, "exile").is_empty());
        }

        #[test]
        fn r392_with_a_card_in_hand_it_is_listed_and_the_random_discard_is_drawn_from_the_match_rng() {
            let mut s = setup(json!({ "hand": [FILLER, VANILLA, TIMMY], "field": [NOSE] }), json!({}));
            let nose = s.card(NOSE).clone();
            assert!(!activations(&s, P1, &nose.id).is_empty());

            s.activate(&nose, json!({}));

            assert_eq!(s.hand(P1).len(), 2);
            assert_eq!(s.pile(P1, "graveyard").len(), 1);
            let discards = s.events().iter().filter(|event| event.event_type() == GameEventType::Discarded).count();
            assert_eq!(discards, 1);
        }

        #[test]
        fn an_empty_deck_exiles_nothing_and_deals_no_fatigue_the_discard_is_still_paid() {
            let mut s = setup(json!({ "hand": [FILLER], "field": [NOSE] }), json!({ "library": [] }));

            s.activate(NOSE, json!({}));

            assert_eq!(s.pile(P1, "graveyard").len(), 1);
            assert!(s.pile(P2, "exile").is_empty());
            s.expect_health(P2, 30);
            assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::Fatigue));
        }

        #[test]
        fn r384_usable_the_turn_it_is_played_activating_needs_no_readiness_and_spends_no_exertion() {
            let mut s = setup(json!({ "hand": [NOSE, FILLER, FILLER] }), json!({}));
            s.play(NOSE, json!({ "zone": 1 }));

            s.activate(NOSE, json!({}));

            assert_eq!(s.pile(P2, "exile").len(), 1);
            // The same unit may still switch position this turn: activating spent no exertion.
            let nose = s.card(NOSE).clone();
            assert!(!nose.exertion.attacked);
            assert!(!nose.exertion.switched);
        }

        #[test]
        fn r384_a_unit_that_activated_may_still_attack_that_turn() {
            let mut s = setup(json!({ "hand": [FILLER, FILLER], "field": [NOSE] }), json!({}));

            s.activate(NOSE, json!({})).attack(NOSE, "hero");

            s.expect_health(P2, 27);
        }

        #[test]
        fn activate_infinity_a_second_activation_that_turn_works_too_each_paying_its_own_discard() {
            let mut s = setup(json!({ "hand": [FILLER, FILLER, FILLER], "field": [NOSE] }), json!({}));
            let nose = s.card(NOSE).clone();

            s.activate(&nose, json!({}));
            assert!(!activations(&s, P1, &nose.id).is_empty());
            s.activate(&nose, json!({}));
            assert_eq!(s.pile(P2, "exile").len(), 2);
            assert_eq!(s.hand(P1).len(), 1);
        }

        #[test]
        fn r384_only_its_controller_in_their_own_main_phase_on_the_opponent_s_turn_it_can_t_be_activated() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [FILLER], "field": [NOSE] },
                "p2": { "hand": [FILLER], "library": DECK },
            }));
            let nose = s.card(NOSE).clone();

            assert!(activations(&s, P1, &nose.id).is_empty());
            s.expect_refused(|s| s.activate(&nose, json!({})));
            assert!(s.pile(P2, "exile").is_empty());
        }

        #[test]
        fn r13_r384_dormant_under_a_stack_pile_it_does_not_act_so_it_can_t_be_activated() {
            let mut s = setup(
                json!({ "hand": [FILLER], "field": [{ "def": NOSE, "lane": 1 }, { "def": VANILLA, "stack": true }] }),
                json!({}),
            );
            let nose = s.card(NOSE).clone();

            assert!(activations(&s, P1, &nose.id).is_empty());
            s.expect_refused(|s| s.activate(&nose, json!({})));
            assert_eq!(s.hand(P1).len(), 1);
            assert!(s.pile(P2, "exile").is_empty());
        }

        #[test]
        fn r384_activating_is_not_a_play_no_cardplayed_and_the_turn_s_play_count_does_not_move() {
            let mut s = setup(json!({ "hand": [FILLER, FILLER], "field": [NOSE] }), json!({}));
            let played = s.state().players.p1.turn_log.cards_played;

            s.activate(NOSE, json!({}));

            assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::CardPlayed));
            assert_eq!(s.state().players.p1.turn_log.cards_played, played);
        }

        #[test]
        fn the_cost_is_a_discard_sec6_3_the_card_goes_hand_to_graveyard_with_a_discarded_event_naming_it() {
            let mut s = setup(json!({ "hand": [VANILLA], "field": [NOSE] }), json!({}));
            let Some(card) = s.hand(P1).first().cloned() else {
                panic!("fixture");
            };

            s.activate(NOSE, json!({}));

            let discard = s.events().iter().find(|event| event.event_type() == GameEventType::Discarded).cloned();
            assert_eq!(
                discard,
                Some(GameEvent::Discarded { instance_id: card.id.clone(), def_id: VANILLA.to_string(), owner: P1 })
            );
            assert!(s
                .events()
                .iter()
                .any(|event| matches!(event, GameEvent::EnteredGraveyard { instance_id, .. } if *instance_id == card.id)));
        }

        #[test]
        fn no_event_carries_a_deck_position_the_exiled_card_is_public_in_both_views() {
            let mut s = setup(json!({ "hand": [FILLER], "field": [NOSE] }), json!({}));

            s.activate(NOSE, json!({}));

            for event in s.last_events() {
                let keys = serde_json::to_value(event).unwrap();
                assert!(keys.get("position").is_none());
                assert!(keys.get("index").is_none());
            }
            assert_eq!(exile_seen(&s, P1, "opponent"), [POINTMASTER]);
            assert_eq!(exile_seen(&s, P2, "you"), [POINTMASTER]);
        }

        #[test]
        fn r386_an_upgrade_of_its_number_exiles_the_bottom_2_cards_a_degrade_never_takes_it_below_1() {
            let mut up = setup(json!({ "hand": [FILLER], "field": [NOSE] }), json!({}));
            step(&mut up, NOSE, "exile", 1);
            up.activate(NOSE, json!({}));
            assert_eq!(defs_of(&up.pile(P2, "exile")), [POINTMASTER, TIMMY]);
            assert_eq!(defs_of(&up.pile(P2, "library")), [VANILLA]);

            let mut down = setup(json!({ "hand": [FILLER], "field": [NOSE] }), json!({}));
            step(&mut down, NOSE, "exile", -1);
            down.activate(NOSE, json!({}));
            assert_eq!(defs_of(&down.pile(P2, "exile")), [POINTMASTER]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r275_the_radiant_face_is_a_6_2() {
            let mut s = setup(json!({ "hand": [FILLER], "field": [{ "def": NOSE, "radiant": true }] }), json!({}));
            s.expect_stats(NOSE, json!({ "attack": 6, "health": 2, "maxHealth": 2 }));
        }

        #[test]
        fn r392_also_exiles_a_random_card_from_the_opponent_s_hand_public_once_in_exile() {
            let mut s = setup(
                json!({ "hand": [FILLER], "field": [{ "def": NOSE, "radiant": true }] }),
                json!({ "hand": [MENACE] }),
            );
            let menace = s.card(MENACE).clone();

            s.activate(NOSE, json!({}));

            s.expect_in_zone(&menace, "exile");
            let mut exiled = defs_of(&s.pile(P2, "exile"));
            exiled.sort();
            let mut expected = vec![MENACE.to_string(), POINTMASTER.to_string()];
            expected.sort();
            assert_eq!(exiled, expected);
            assert!(s.hand(P2).is_empty());
            assert!(exile_seen(&s, P1, "opponent").contains(&MENACE.to_string()));
        }

        #[test]
        fn r60_the_hand_card_is_picked_at_random_from_their_hand_one_card_only() {
            let mut s = setup(
                json!({ "hand": [FILLER], "field": [{ "def": NOSE, "radiant": true }] }),
                json!({ "hand": [MENACE, VANILLA, TIMMY] }),
            );

            s.activate(NOSE, json!({}));

            assert_eq!(s.hand(P2).len(), 2);
            assert_eq!(s.pile(P2, "exile").len(), 2);
        }

        #[test]
        fn an_empty_opponent_s_hand_only_the_deck_s_bottom_card_is_exiled() {
            let mut s = setup(json!({ "hand": [FILLER], "field": [{ "def": NOSE, "radiant": true }] }), json!({ "hand": [] }));

            s.activate(NOSE, json!({}));

            assert_eq!(defs_of(&s.pile(P2, "exile")), [POINTMASTER]);
        }

        #[test]
        fn r392_the_radiant_face_still_costs_a_random_discard_an_empty_hand_can_t_activate_it() {
            let mut s = setup(
                json!({ "hand": [], "field": [VANILLA, { "def": NOSE, "radiant": true }] }),
                json!({ "hand": [MENACE] }),
            );
            let nose = s.card(NOSE).clone();

            assert!(activations(&s, P1, &nose.id).is_empty());
            s.expect_refused(|s| s.activate(&nose, json!({})));
            assert_eq!(s.hand(P2).len(), 1);
        }

        #[test]
        fn r386_an_upgrade_moves_the_deck_exile_to_2_and_leaves_the_hand_exile_at_one_card() {
            let mut s = setup(
                json!({ "hand": [FILLER], "field": [{ "def": NOSE, "radiant": true }] }),
                json!({ "hand": [MENACE, VANILLA] }),
            );
            step(&mut s, NOSE, "exile", 1);

            s.activate(NOSE, json!({}));

            assert_eq!(s.pile(P2, "library").len(), 1);
            assert_eq!(s.hand(P2).len(), 1);
            assert_eq!(s.pile(P2, "exile").len(), 3);
        }
    }
}
