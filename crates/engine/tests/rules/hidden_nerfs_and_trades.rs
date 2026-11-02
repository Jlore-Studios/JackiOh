// Stinkyscape M11 (issue #1845): the engine primitives of #123 Gokugains and #77 Yuma, each through a
// fixture script (`fixtures/hidden_trades.rs`): R1845000's hidden Nerfs (`degrade_secretly`), which no
// seat reads, R1845001's landing of them as the card is played, sparing its cost, and R1845002's trade
// offer (`trade_offer`), whose two prompts every answerer answers as it answers any other.

use jackioh_engine::effects::{
    TRADE_OFFER_HOOK, TRADE_OFFER_LABEL, TRADE_OFFER_LUCKY_KEY, TuneDirection, TuneRow, applicable_changes,
};
use jackioh_engine::subsystems::ai_policy::choose_action;
use jackioh_engine::subsystems::scorer::concealed_copy;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{big_body, plain, taunter, zero_attack};
use crate::rules::fixtures::hidden_trades::{
    DUD, HEAVY_NERFS, LUCKY_TRADER, SECRET_NERFS, SNEAK, STOIC, TRADER, register_hidden_trade_fixtures,
};

/// `p1`'s turn 9, its main phase (or `active`'s); each side's setup as given, with plain cards to
/// draw and to hold so no turn ends by itself (§2.5).
fn board_on(seed: &str, active: PlayerId, p1: Value, p2: Value) -> Scenario {
    register_hidden_trade_fixtures();
    let mut p1 = p1;
    let mut p2 = p2;
    for side in [&mut p1, &mut p2] {
        if side.get("library").is_none() {
            side["library"] = json!([plain.id, plain.id, plain.id, plain.id, plain.id, plain.id]);
        }
        if side.get("hand").is_none() {
            side["hand"] = json!([plain.id]);
        }
    }
    scenario(json!({ "seed": seed, "active": active.as_str(), "p1": p1, "p2": p2 }))
}

fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    board_on(seed, P1, p1, p2)
}

fn badges(s: &Scenario, viewer: PlayerId, side: PlayerId) -> Vec<String> {
    let view = s.view(viewer);
    let seat = if viewer == side { view.you } else { view.opponent };
    seat.modifiers.into_iter().map(|badge| badge.label).collect()
}

fn held(s: &Scenario, player: PlayerId, def_id: &str) -> String {
    s.hand(player)
        .into_iter()
        .find(|card| card.def_id == def_id)
        .map(|card| card.id)
        .unwrap_or_else(|| panic!("{player:?} holds a {def_id}"))
}

fn scenario_hand_ids(s: &Scenario, player: PlayerId) -> Vec<String> {
    s.hand(player).into_iter().map(|card| card.id).collect()
}

fn offered(s: &Scenario) -> Vec<String> {
    let pending = s.state().pending.as_ref().expect("a prompt is open");
    pending
        .options
        .iter()
        .filter_map(|option| match &option.selection {
            Selection::Instance { instance_id } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

fn sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids
}

fn stolen(events: &[GameEvent]) -> Vec<(String, PlayerId, PlayerId)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Stolen {
                instance_id,
                from,
                to,
                ..
            } => Some((instance_id.clone(), *from, *to)),
            _ => None,
        })
        .collect()
}

fn degraded(events: &[GameEvent], id: &str) -> Vec<(TuningChange, Option<Vec<PlayerId>>)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Degraded {
                instance_id,
                change,
                hidden_from,
                ..
            } if instance_id == id => Some((change.clone(), hidden_from.clone())),
            _ => None,
        })
        .collect()
}

fn hidden_nerfs(s: &Scenario, id: &str) -> Option<i32> {
    s.card(id).hidden_nerfs
}

/// p1 plays the trader into lane 2 (a plain stands in lane 1, so p1's turn never ends by itself) and
/// ends the turn: the offer comes due at p2's turn start.
fn offer_and_pass(s: &mut Scenario) {
    s.play(TRADER.id.as_str(), json!({ "zone": 2 }));
    s.end_turn();
}

mod r1845000_a_hidden_nerf_is_a_count_no_seat_reads {
    use super::*;

    #[test]
    fn r1845000_a_secret_nerf_marks_one_random_enemy_hand_unit_and_says_nothing() {
        let mut s = board(
            "r1845000-secret",
            json!({ "hand": [SNEAK.id, plain.id] }),
            json!({ "hand": [big_body.id, DUD.id, plain.id] }),
        );
        let body = held(&s, P2, &big_body.id);
        let dud = held(&s, P2, &DUD.id);
        let other = held(&s, P2, &plain.id);
        let owners_view = serde_json::to_value(s.view(P2).you.hand).expect("a view serializes");
        let offerers_view = serde_json::to_value(s.view(P1).opponent.hand).expect("a view serializes");
        s.play(SNEAK.id.as_str(), json!({ "zone": 1 }));

        let mut counts = vec![hidden_nerfs(&s, &body), hidden_nerfs(&s, &other)];
        counts.sort();
        assert_eq!(
            counts,
            vec![None, Some(SECRET_NERFS)],
            "one random Unit takes the three"
        );
        assert_eq!(hidden_nerfs(&s, &dud), None, "a Spell is no Unit");
        assert!(
            !s.last_events()
                .iter()
                .any(|event| matches!(event, GameEvent::Degraded { .. } | GameEvent::Upgraded { .. })),
            "nothing is reported"
        );
        assert_eq!(
            serde_json::to_value(s.view(P2).you.hand).expect("a view serializes"),
            owners_view,
            "the owner's own hand reads as it did"
        );
        assert_eq!(
            serde_json::to_value(s.view(P1).opponent.hand).expect("a view serializes"),
            offerers_view
        );
    }

    #[test]
    fn r1845000_a_secret_nerf_passes_by_an_immutable_card_and_a_hand_without_units() {
        let mut s = board(
            "r1845000-stoic",
            json!({ "hand": [SNEAK.id, plain.id] }),
            json!({ "hand": [STOIC.id, big_body.id] }),
        );
        let stoic = held(&s, P2, &STOIC.id);
        let body = held(&s, P2, &big_body.id);
        s.play(SNEAK.id.as_str(), json!({ "zone": 1 }));
        assert_eq!(hidden_nerfs(&s, &body), Some(SECRET_NERFS));
        assert_eq!(
            hidden_nerfs(&s, &stoic),
            None,
            "no Nerf reaches an Immutable card (R23)"
        );

        let mut s = board(
            "r1845000-no-units",
            json!({ "hand": [SNEAK.id, plain.id] }),
            json!({ "hand": [DUD.id, DUD.id] }),
        );
        s.play(SNEAK.id.as_str(), json!({ "zone": 1 }));
        assert!(s.hand(P2).iter().all(|card| card.hidden_nerfs.is_none()));
    }

    #[test]
    fn r1845000_a_concealed_copy_holds_no_hidden_nerf() {
        let mut s = board(
            "r1845000-concealed",
            json!({ "hand": [big_body.id, plain.id] }),
            json!({ "hand": [big_body.id, plain.id] }),
        );
        for player in [P1, P2] {
            let id = held(&s, player, &big_body.id);
            s.card_mut(id.as_str()).hidden_nerfs = Some(SECRET_NERFS);
            let top = s.state().players[player].library[0].id.clone();
            s.card_mut(top.as_str()).hidden_nerfs = Some(SECRET_NERFS);
        }
        let copy = concealed_copy(s.state(), P1);
        for player in [P1, P2] {
            let side = &copy.players[player];
            assert!(
                side.hand
                    .iter()
                    .chain(side.library.iter())
                    .all(|card| card.hidden_nerfs.is_none()),
                "{player:?}'s cards hold no count in the copy"
            );
        }
        // The game itself keeps them.
        assert_eq!(hidden_nerfs(&s, &held(&s, P1, &big_body.id)), Some(SECRET_NERFS));
    }
}

mod r1845001_hidden_nerfs_land_as_the_card_is_played {
    use super::*;

    #[test]
    fn r1845001_hidden_nerfs_land_as_the_card_is_played_and_spare_its_cost() {
        let mut s = board_on(
            "r1845001-land",
            P2,
            json!({}),
            json!({ "hand": [big_body.id, plain.id] }),
        );
        let body = held(&s, P2, &big_body.id);
        s.card_mut(body.as_str()).hidden_nerfs = Some(HEAVY_NERFS);
        let mana = s.state().players[P2].mana.current;
        s.play(body.as_str(), json!({ "zone": 1 }));

        assert_eq!(
            mana - s.state().players[P2].mana.current,
            3,
            "it costs what it printed"
        );
        assert_eq!(s.card(body.as_str()).cost_mod, 0);
        assert_eq!(hidden_nerfs(&s, &body), None, "the count is spent");
        let nerfs = degraded(s.last_events(), &body);
        assert_eq!(nerfs.len(), HEAVY_NERFS as usize, "every Nerf is reported (R440)");
        assert!(
            nerfs
                .iter()
                .all(|(_, hidden_from)| *hidden_from == Some(vec![P1])),
            "as a hand card's, unread by the other seat (R177)"
        );
        assert!(
            !nerfs
                .iter()
                .any(|(change, _)| matches!(change, TuningChange::Cost { .. }))
        );
        let stats = s.stats(body.as_str());
        assert_eq!((stats.attack, stats.health), (0, 1));
    }

    #[test]
    fn r1845001_a_card_only_its_cost_could_nerf_takes_them_as_nothing() {
        let mut s = board_on(
            "r1845001-dud",
            P2,
            json!({}),
            json!({ "hand": [DUD.id, plain.id] }),
        );
        let dud = held(&s, P2, &DUD.id);
        // A Nerf of the ordinary kind could only raise its cost.
        assert_eq!(
            applicable_changes(s.state(), s.card(dud.as_str()), TuneDirection::Degrade),
            vec![TuneRow::Cost]
        );
        s.card_mut(dud.as_str()).hidden_nerfs = Some(SECRET_NERFS);
        let mana = s.state().players[P2].mana.current;
        s.play(dud.as_str(), json!({}));

        assert_eq!(s.state().players[P2].mana.current, mana, "a (0) is still free");
        let nerfs = degraded(s.last_events(), &dud);
        assert_eq!(
            nerfs,
            vec![(TuningChange::None, Some(vec![P1])); SECRET_NERFS as usize],
            "each is cued with nothing changed (R440)"
        );
        assert_eq!(s.card(dud.as_str()).cost_mod, 0);
    }

    #[test]
    fn r1845001_a_replaced_play_lands_none_its_card_having_ceased_to_exist() {
        let mut s = board_on(
            "r1845001-replaced",
            P2,
            json!({}),
            json!({ "hand": [big_body.id, plain.id] }),
        );
        let body = held(&s, P2, &big_body.id);
        s.card_mut(body.as_str()).hidden_nerfs = Some(HEAVY_NERFS);
        // C #23 Devil's Pact's rider (R449): the next card played this turn is replaced by a plain.
        let turn = s.state().turn;
        s.state_mut().players[P2].mods.push(PlayerModifier {
            id: "m-pact".to_string(),
            expiry: ModifierExpiry::ThisTurn { turn },
            kind: ModifierKind::ReplacePlays {
                def_id: plain.id.clone(),
                radiant: false,
            },
        });
        s.play(body.as_str(), json!({ "zone": 1 }));

        let played = s.unit(P2, 1).expect("the replacement stands");
        assert_eq!(played.def_id, plain.id);
        assert_eq!(played.hidden_nerfs, None);
        assert!(
            !s.last_events()
                .iter()
                .any(|event| matches!(event, GameEvent::Degraded { .. })),
            "the replaced card's count went with it"
        );
        assert!(
            find_instance(s.state(), &body).is_none(),
            "it has ceased to exist"
        );
        assert_eq!(played.tuning, None);
    }

    #[test]
    fn r1845001_r1845002_a_traded_card_keeps_its_hidden_nerfs_until_its_new_owner_plays_it() {
        let mut s = board(
            "r1845001-traded",
            json!({ "hand": [TRADER.id, plain.id], "field": [plain.id] }),
            json!({ "hand": [big_body.id, DUD.id] }),
        );
        let body = held(&s, P2, &big_body.id);
        s.card_mut(body.as_str()).hidden_nerfs = Some(HEAVY_NERFS);
        offer_and_pass(&mut s);
        let given = held(&s, P1, &plain.id);
        s.answer(pick::id(&given));
        s.answer(pick::id(&body));

        let card = s.card(body.as_str());
        assert_eq!(card.zone, Zone::Hand { player: P1 });
        assert_eq!(card.owner, P1);
        assert_eq!(
            card.hidden_nerfs,
            Some(HEAVY_NERFS),
            "kept through the change of hands"
        );
        assert_eq!(card.tuning, None, "and still not landed");

        s.end_turn();
        assert_eq!(s.state().active, P1);
        s.play(body.as_str(), json!({ "zone": 3 }));
        let stats = s.stats(body.as_str());
        assert_eq!((stats.attack, stats.health), (0, 1));
        let nerfs = degraded(s.last_events(), &body);
        assert_eq!(nerfs.len(), HEAVY_NERFS as usize);
        assert!(
            nerfs
                .iter()
                .all(|(_, hidden_from)| *hidden_from == Some(vec![P2]))
        );
    }
}

mod r1845002_a_trade_offer {
    use super::*;

    #[test]
    fn r1845002_the_offer_waits_for_the_opponents_next_turn_with_a_badge() {
        let mut s = board(
            "r1845002-waits",
            json!({ "hand": [TRADER.id, plain.id], "field": [plain.id] }),
            json!({ "hand": [DUD.id, plain.id] }),
        );
        s.play(TRADER.id.as_str(), json!({ "zone": 2 }));
        assert_eq!(s.state().delayed.len(), 1);
        assert_eq!(s.state().delayed[0].resume.hook, TRADE_OFFER_HOOK);
        assert!(
            s.state().pending.is_none(),
            "nothing is asked on the offerer's turn"
        );
        for viewer in [P1, P2] {
            assert!(
                badges(&s, viewer, P1)
                    .iter()
                    .any(|label| label.starts_with(TRADE_OFFER_LABEL)),
                "{viewer:?} sees the offer waiting on p1's side"
            );
        }
        assert!(stolen(s.events()).is_empty());
    }

    #[test]
    fn r1845002_the_controller_gives_a_card_then_the_opponent_discovers_one_of_theirs_back() {
        let mut s = board(
            "r1845002-trade",
            json!({ "hand": [TRADER.id, plain.id, big_body.id], "field": [plain.id] }),
            json!({ "hand": [DUD.id, plain.id, STOIC.id] }),
        );
        let theirs = scenario_hand_ids(&s, P2);
        offer_and_pass(&mut s);
        let from = s.events().len();

        let pending = s.state().pending.clone().expect("the offerer is asked");
        assert_eq!(pending.player_id, P1, "on p2's turn (R1279000)");
        assert_eq!(s.state().active, P2);
        assert_eq!(pending.kind, PromptKind::Hand);
        assert_eq!(offered(&s), scenario_hand_ids(&s, P1));
        let body = held(&s, P1, &big_body.id);
        s.answer(pick::id(&body));
        let given = s.card(body.as_str());
        assert_eq!(given.zone, Zone::Hand { player: P2 });
        assert_eq!((given.owner, given.controller), (P2, P2));

        let pending = s.state().pending.clone().expect("the opponent Discovers");
        assert_eq!(pending.player_id, P2);
        assert_eq!(pending.kind, PromptKind::Discover);
        assert_eq!(
            sorted(offered(&s)),
            sorted(theirs.clone()),
            "their own cards, the given one aside"
        );
        let stoic = held(&s, P2, &STOIC.id);
        s.answer(pick::id(&stoic));
        let taken = s.card(stoic.as_str());
        assert_eq!(taken.zone, Zone::Hand { player: P1 });
        assert_eq!((taken.owner, taken.controller), (P1, P1));

        assert_eq!(
            stolen(&s.events()[from..]),
            vec![(body.clone(), P1, P2), (stoic.clone(), P2, P1)]
        );
        assert!(s.state().pending.is_none());
        assert_eq!(s.state().active, P2);
        assert!(
            s.last_events()
                .iter()
                .any(|event| matches!(event, GameEvent::Drawn { player: P2, .. })),
            "the turn goes on to its draw"
        );
        assert_eq!(s.hand(P2).len(), theirs.len() + 1);
    }

    #[test]
    fn r1845002_no_trade_when_either_hand_is_empty() {
        // The offerer's hand is empty once the trader is played.
        let mut s = board(
            "r1845002-empty-offerer",
            json!({ "hand": [TRADER.id], "field": [plain.id] }),
            json!({ "hand": [DUD.id, plain.id] }),
        );
        offer_and_pass(&mut s);
        assert!(s.state().pending.is_none());
        assert!(stolen(s.events()).is_empty());
        assert!(s.hand(P1).is_empty());

        // The opponent's is empty as their turn starts, before its draw.
        let mut s = board(
            "r1845002-empty-taker",
            json!({ "hand": [TRADER.id, plain.id], "field": [plain.id] }),
            json!({ "hand": [] }),
        );
        let mine = scenario_hand_ids(&s, P1);
        offer_and_pass(&mut s);
        assert!(s.state().pending.is_none());
        assert!(stolen(s.events()).is_empty());
        assert_eq!(scenario_hand_ids(&s, P1), mine[1..].to_vec());
    }

    #[test]
    fn r1845002_a_full_hand_burns_the_given_card_and_the_trade_goes_on() {
        let mut s = board(
            "r1845002-full",
            json!({ "hand": [TRADER.id, big_body.id], "field": [plain.id] }),
            json!({ "hand": vec![plain.id.clone(); HAND_CAP as usize] }),
        );
        offer_and_pass(&mut s);
        let body = held(&s, P1, &big_body.id);
        s.answer(pick::id(&body));
        let burned = s.card(body.as_str());
        assert_eq!(
            burned.zone,
            Zone::Graveyard { player: P2 },
            "its new owner's (R317)"
        );
        assert!(
            s.last_events()
                .iter()
                .any(|event| matches!(event, GameEvent::Burned { instance_id, .. } if *instance_id == body))
        );

        let pending = s.state().pending.clone().expect("the Discover still opens");
        assert_eq!(pending.kind, PromptKind::Discover);
        assert_eq!(offered(&s).len(), DISCOVER_OPTIONS as usize);
        let back = offered(&s)[0].clone();
        s.answer(pick::id(&back));
        assert_eq!(s.card(back.as_str()).zone, Zone::Hand { player: P1 });
    }

    #[test]
    fn r1845002_legal_actions_the_refusal_and_the_view_agree_on_both_prompts() {
        let mut s = board(
            "r1845002-agree",
            json!({ "hand": [TRADER.id, plain.id, big_body.id], "field": [plain.id] }),
            json!({ "hand": [DUD.id, plain.id, STOIC.id] }),
        );
        let theirs = scenario_hand_ids(&s, P2);
        offer_and_pass(&mut s);
        let body = held(&s, P1, &big_body.id);
        // At the give, a card of the other hand is not on offer; at the Discover, the given card is not.
        for (holder, not_offered, answer) in [
            (P1, theirs[0].clone(), body.clone()),
            (P2, body.clone(), theirs[2].clone()),
        ] {
            let pending = s.state().pending.clone().expect("a prompt is open");
            assert_eq!(pending.player_id, holder);
            let mut answers: Vec<Vec<Selection>> = legal_actions(s.state(), holder)
                .into_iter()
                .filter_map(|action| match action {
                    ActionBody::Answer { selection, .. } => Some(selection),
                    _ => None,
                })
                .collect();
            answers.sort_by_key(|selection| format!("{selection:?}"));
            let mut options: Vec<Vec<Selection>> = pending
                .options
                .iter()
                .map(|option| vec![option.selection.clone()])
                .collect();
            options.sort_by_key(|selection| format!("{selection:?}"));
            assert_eq!(answers, options, "one answer per option");
            let other = opponent_of(holder);
            assert!(
                !legal_actions(s.state(), other)
                    .iter()
                    .any(|action| matches!(action, ActionBody::Answer { .. }))
            );
            match s.view(holder).pending {
                Some(PendingView::ForYou(view)) => {
                    assert_eq!(view.kind, pending.kind);
                    assert_eq!(view.options.len(), pending.options.len());
                }
                other => panic!("the holder sees its prompt, not {other:?}"),
            }
            assert_eq!(
                s.view(other).pending,
                Some(PendingView::Elsewhere(PendingElsewhereView {
                    for_you: false,
                    pending_for: holder,
                }))
            );
            let choice_id = pending.id.clone();
            s.expect_refused(|s| {
                s.act(
                    ActionBody::Answer {
                        choice_id,
                        selection: vec![Selection::Instance {
                            instance_id: not_offered,
                        }],
                    },
                    holder,
                    "not offered",
                )
            });
            s.answer(pick::id(&answer));
        }
        assert!(s.state().pending.is_none());
        assert_eq!(stolen(s.events()).len(), 2);
    }

    #[test]
    fn r1845002_the_random_policy_and_the_timeout_answer_both_prompts() {
        let setup = || {
            let mut s = board(
                "r1845002-answerers",
                json!({ "hand": [TRADER.id, plain.id, big_body.id], "field": [plain.id] }),
                json!({ "hand": [DUD.id, plain.id, STOIC.id] }),
            );
            offer_and_pass(&mut s);
            s
        };

        let mut s = setup();
        for holder in [P1, P2] {
            assert_eq!(
                s.state().pending.as_ref().map(|pending| pending.player_id),
                Some(holder)
            );
            let body = choose_action(s.state(), holder, &mut create_rng("r1845002-policy", 0))
                .expect("the policy answers");
            assert!(matches!(body, ActionBody::Answer { .. }));
            s.act(body, holder, "the random policy's answer");
        }
        assert!(s.state().pending.is_none());
        assert_eq!(stolen(s.events()).len(), 2);

        let mut s = setup();
        s.act(ActionBody::Timeout, P1, "the offerer's clock runs out");
        assert_eq!(
            s.state().pending.as_ref().map(|pending| pending.player_id),
            Some(P2)
        );
        s.act(ActionBody::Timeout, P2, "the opponent's clock runs out");
        assert!(s.state().pending.is_none());
        assert_eq!(stolen(s.events()).len(), 2);
    }
}

mod r1877001_a_lucky_trade_offer {
    use super::*;

    /// R65's cost of a hand card, as the offer compares them.
    fn cost_of(s: &Scenario, id: &str) -> i32 {
        jackioh_engine::mana::effective_cost(s.state(), s.card(id), Default::default())
    }

    /// The offer `trader` makes on `seed`: p1 gives its plain card, and the opponent's Discover opens
    /// over six cards of costs 0 to 3. Returns the cheapest option's cost and the rng cursor after.
    fn discover_after(trader: &str, seed: &str) -> (i32, u32, Scenario) {
        let mut s = board(
            seed,
            json!({ "hand": [trader, plain.id], "field": [plain.id] }),
            json!({ "hand": [DUD.id, plain.id, big_body.id, zero_attack.id, taunter.id, DUD.id] }),
        );
        s.play(trader, json!({ "zone": 2 }));
        s.end_turn();
        let mine = held(&s, P1, &plain.id);
        s.answer(pick::id(&mine));
        let pending = s.state().pending.clone().expect("the opponent Discovers");
        assert_eq!(pending.kind, PromptKind::Discover);
        assert_eq!(offered(&s).len(), DISCOVER_OPTIONS as usize);
        let cheapest = offered(&s)
            .iter()
            .map(|id| cost_of(&s, id))
            .min()
            .expect("options");
        (cheapest, s.state().rng_cursor, s)
    }

    #[test]
    fn r1877001_a_card_with_no_lucky_makes_the_offer_as_before_with_nothing_carried() {
        let mut s = board(
            "r1877001-plain",
            json!({ "hand": [TRADER.id, plain.id], "field": [plain.id] }),
            json!({ "hand": [DUD.id, plain.id] }),
        );
        s.play(TRADER.id.as_str(), json!({ "zone": 2 }));
        assert!(
            !s.state().delayed[0]
                .resume
                .data
                .contains_key(TRADE_OFFER_LUCKY_KEY)
        );

        let mut s = board(
            "r1877001-lucky",
            json!({ "hand": [LUCKY_TRADER.id, plain.id], "field": [plain.id] }),
            json!({ "hand": [DUD.id, plain.id] }),
        );
        s.play(LUCKY_TRADER.id.as_str(), json!({ "zone": 2 }));
        assert_eq!(
            s.state().delayed[0].resume.data.get(TRADE_OFFER_LUCKY_KEY),
            Some(&json!(1)),
            "the card's Lucky, carried whatever becomes of it (R76)"
        );
    }

    #[test]
    fn r1877001_lucky_1_draws_the_options_twice_and_keeps_the_set_whose_cheapest_card_costs_most() {
        let mut better = 0;
        for n in 0..16 {
            let seed = format!("r1877001-{n}");
            let (plain_min, plain_cursor, _) = discover_after(&TRADER.id, &seed);
            let (lucky_min, lucky_cursor, _) = discover_after(&LUCKY_TRADER.id, &seed);
            // The same first draw, then one more: the kept set is never worse for the offerer.
            assert!(lucky_min >= plain_min, "{seed}: {lucky_min} against {plain_min}");
            assert!(lucky_cursor > plain_cursor, "{seed}: Lucky draws again");
            if lucky_min > plain_min {
                better += 1;
            }
        }
        assert!(better > 0, "a second draw sometimes offers a dearer set");
    }

    #[test]
    fn r1877001_a_hand_no_bigger_than_the_options_is_drawn_once() {
        let small = |trader: &str| {
            let mut s = board(
                "r1877001-small",
                json!({ "hand": [trader, plain.id], "field": [plain.id] }),
                json!({ "hand": [DUD.id, big_body.id, taunter.id] }),
            );
            s.play(trader, json!({ "zone": 2 }));
            s.end_turn();
            let mine = held(&s, P1, &plain.id);
            s.answer(pick::id(&mine));
            (sorted(offered(&s)), s.state().rng_cursor)
        };
        assert_eq!(small(&TRADER.id), small(&LUCKY_TRADER.id));
    }
}
