//! Tradable and the Trade (§6.1, R1838000–R1838003): in their own main phase a player may put a
//! Tradable card from their hand into their deck at a random position and draw a card, at
//! `TRADE_COST` (no mana), up to their Trades a turn (`TRADES_PER_TURN`, or a figure of their own),
//! counted over all their Tradable cards. `legal_actions`, the reducer and the view all read
//! `trade::why_trade_refused`. The trade is hidden like a draw, its draw is a draw for every rule, it
//! is no play, and a Trade left keeps a turn from ending by itself. The fixtures in
//! `fixtures/tradable.rs` stand in for Meditative #53 and #98, which test it again in their own files,
//! and for Stinkyscape #164 and #165 (R1864002–R1864004): an aura lends Tradable to a hand, a card on
//! the field gives Trades a turn, and a trigger on `traded` is handed the card the Trade put in.

use jackioh_engine::testkit::*;

use super::fixtures::harness::{in_hand, put, slot, types_of};
use super::fixtures::prompt_harness::board;
use super::fixtures::tradable::{
    FIXTURE_TRADES, lender, no_draws, plain_unit, register_tradable, tradable_spell, trade_watcher,
    two_trades,
};

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

/// p1's main phase on turn 3 with 4 mana a side, holding a Tradable Spell and a plain Unit; p2 holds
/// a plain Unit. Returns the state and the Spell's and the Unit's ids.
fn holding(seed: &str) -> (GameState, String, String) {
    let mut state = board(seed);
    register_tradable();
    let spell = in_hand(&mut state, &tradable_spell().id, P1, 1).remove(0);
    let unit = in_hand(&mut state, &plain_unit().id, P1, 1).remove(0);
    in_hand(&mut state, &plain_unit().id, P2, 1);
    (state, spell.id, unit.id)
}

fn trade_of(id: &str) -> ActionBody {
    ActionBody::Trade {
        instance_id: id.to_string(),
    }
}

fn trade(state: &GameState, player: PlayerId, id: &str, nonce: &str) -> ReduceResult {
    reduce(state, &Action::new(trade_of(id), player, nonce))
}

fn trades_listed(state: &GameState, player: PlayerId) -> Vec<String> {
    legal_actions(state, player)
        .into_iter()
        .filter_map(|action| match action {
            ActionBody::Trade { instance_id } => Some(instance_id),
            _ => None,
        })
        .collect()
}

fn hand_card<'a>(view: &'a PlayerView, id: &str) -> &'a CardView {
    let HandView::Cards(cards) = &view.you.hand else {
        panic!("the viewer's own hand is a list");
    };
    cards
        .iter()
        .find(|card| card.instance_id == id)
        .unwrap_or_else(|| panic!("{id} in the viewer's hand"))
}

#[test]
fn r1838000_a_trade_shuffles_the_card_in_and_draws_one_at_no_mana() {
    let (state, spell, _unit) = holding("r1838000-trade");
    let (hand, library, mana) = (
        state.players.p1.hand.len(),
        state.players.p1.library.len(),
        state.players.p1.mana.current,
    );
    assert_eq!(
        trades_listed(&state, P1),
        vec![spell.clone()],
        "only the Tradable card is offered"
    );
    let out = trade(&state, P1, &spell, "t1");
    assert_eq!(out.error, None);
    let kinds = types_of(&out.events);
    assert_eq!(
        kinds.iter().take(3).copied().collect::<Vec<_>>(),
        vec![
            GameEventType::Traded,
            GameEventType::ShuffledIn,
            GameEventType::Drawn
        ],
        "the trade, the card going in, the card coming out"
    );
    let side = &out.state.players.p1;
    assert_eq!(
        side.hand.len(),
        hand,
        "one card out of the hand, one drawn into it"
    );
    assert_eq!(side.library.len(), library);
    assert_eq!(
        side.mana.current,
        mana - TRADE_COST,
        "a Trade costs TRADE_COST, none"
    );
    assert_eq!(TRADE_COST, 0);
    assert_eq!(trades_this_turn(&out.state, P1), 1);
    let traded = find_instance(&out.state, &spell).expect("the traded card still exists");
    assert!(
        matches!(
            traded.zone,
            Zone::Library { player: P1 } | Zone::Hand { player: P1 }
        ),
        "into the deck (or drawn straight back): {:?}",
        traded.zone
    );
}

#[test]
fn r1838000_one_trade_a_turn_over_all_tradable_cards_listed_refused_and_viewed_alike() {
    let (mut state, spell, _unit) = holding("r1838000-limit");
    let second = in_hand(&mut state, &tradable_spell().id, P1, 1).remove(0).id;
    assert_eq!(trades_listed(&state, P1).len(), 2);
    let view = view_for(&state, P1);
    assert_eq!(
        view.trades,
        Some(TradesView {
            cost: TRADE_COST,
            left: TRADES_PER_TURN
        })
    );
    assert_eq!(
        hand_card(&view, &second).trade_reason,
        None,
        "a legal Trade carries no reason"
    );

    let traded = trade(&state, P1, &spell, "t1").state;
    assert!(
        trades_listed(&traded, P1).is_empty(),
        "the turn's one Trade is made"
    );
    let refused = trade(&traded, P1, &second, "t2");
    let message = refused.error.expect("the second Trade is refused");
    assert_eq!(message, "you have made this turn's 1 Trade");
    // The view says the same, in the reducer's words, from the same function.
    let view = view_for(&traded, P1);
    assert_eq!(
        hand_card(&view, &second).trade_reason.as_deref(),
        Some(message.as_str())
    );
    assert_eq!(
        view.trades,
        Some(TradesView {
            cost: TRADE_COST,
            left: 0
        })
    );
    assert_eq!(
        why_trade_refused(&traded, P1, &second).map_err(|error| error.message),
        Err(message)
    );
}

#[test]
fn r1838000_the_limit_is_the_players_own_figure_and_resets_each_turn() {
    let (mut state, spell, _unit) = holding("r1838000-figure");
    let second = in_hand(&mut state, &tradable_spell().id, P1, 1).remove(0).id;
    let third = in_hand(&mut state, &tradable_spell().id, P1, 1).remove(0).id;
    state.players.p1.trades_per_turn = Some(2);
    assert_eq!(trades_per_turn_of(&state, P1), 2);
    let once = trade(&state, P1, &spell, "t1").state;
    let twice = trade(&once, P1, &second, "t2");
    assert_eq!(twice.error, None, "a figure of 2 allows a second Trade");
    let thrice = trade(&twice.state, P1, &third, "t3");
    assert_eq!(
        thrice.error.as_deref(),
        Some("you have made this turn's 2 Trades")
    );
    // A count kept for an earlier turn reads as 0.
    let mut later = twice.state.clone();
    later.turn += 2;
    assert_eq!(trades_this_turn(&later, P1), 0);
    assert_eq!(trades_left(&later, P1), 2);
}

#[test]
fn r1838000_refused_off_turn_for_an_untradable_card_and_into_a_full_deck() {
    let (mut state, spell, unit) = holding("r1838000-refused");
    // Not Tradable: neither listed nor taken.
    assert_eq!(
        trade(&state, P1, &unit, "t1").error.as_deref(),
        Some(format!("{} is not Tradable", plain_unit().name).as_str())
    );
    // Not the player's turn: the other seat may not trade.
    let theirs = state.players.p2.hand[0].id.clone();
    assert!(trades_listed(&state, P2).is_empty());
    assert_eq!(
        trade(&state, P2, &theirs, "t2").error.as_deref(),
        Some("it is not your turn")
    );
    // A deck at R80's cap has no room for the card, so no Trade is offered.
    while (state.players.p1.library.len() as i32) < LIBRARY_CAP {
        let card = new_instance(&mut state, &plain_unit().id, P1, Zone::Library { player: P1 });
        state.players.p1.library.push(card);
    }
    assert!(trades_listed(&state, P1).is_empty());
    assert_eq!(
        trade(&state, P1, &spell, "t3").error.as_deref(),
        Some(format!("your deck is full ({LIBRARY_CAP} cards)").as_str())
    );
}

#[test]
fn r1838000_a_card_given_tradable_in_the_hand_is_tradable() {
    let (mut state, _spell, unit) = holding("r1838000-given");
    find_instance_mut(&mut state, &unit)
        .expect("the Unit in hand")
        .granted_keywords
        .push(Keyword::Tradable);
    assert!(trades_listed(&state, P1).contains(&unit));
    assert_eq!(trade(&state, P1, &unit, "t1").error, None);
}

#[test]
fn r1838001_the_other_player_learns_that_a_card_was_traded_and_not_which() {
    let (state, spell, _unit) = holding("r1838001-hidden");
    let out = trade(&state, P1, &spell, "t1");
    let drawn_id = out
        .events
        .iter()
        .find_map(|event| match event {
            GameEvent::Drawn { instance_id, .. } => Some(instance_id.clone()),
            _ => None,
        })
        .expect("a card was drawn");
    let theirs = view_for(&out.state, P2);
    let traded = theirs
        .events
        .iter()
        .find(|event| event.event_type() == GameEventType::Traded);
    assert_eq!(
        traded,
        Some(&GameEvent::Traded { player: P1 }),
        "public: a player, no card"
    );
    for event in &theirs.events {
        let json = serde_json::to_value(event).expect("an event serializes");
        match event {
            GameEvent::ShuffledIn { .. } | GameEvent::Drawn { .. } => {
                assert_eq!(json["instanceId"], json!(HIDDEN_ID), "{json}");
                assert_eq!(json["defId"], json!(HIDDEN_ID), "{json}");
            }
            _ => {}
        }
        let text = json.to_string();
        assert!(
            !text.contains(&format!("\"{spell}\"")),
            "the traded card is never named to p2: {text}"
        );
    }
    assert_eq!(
        theirs.trades, None,
        "the other player's Trades are not theirs to see"
    );
    // The trader reads the card they drew, is shown what went in (R311), and never its slot.
    let mine = view_for(&out.state, P1);
    if drawn_id != spell {
        assert_eq!(hand_card(&mine, &drawn_id).instance_id, drawn_id);
    }
    let shuffled = mine
        .events
        .iter()
        .find(|event| event.event_type() == GameEventType::ShuffledIn)
        .map(|event| serde_json::to_value(event).expect("an event serializes"))
        .expect("the shuffle-in");
    assert_eq!(
        shuffled["position"],
        json!(HIDDEN_POSITION),
        "the slot is hidden from both (R97)"
    );
    let known = find_instance(&out.state, &spell).and_then(|card| card.known_as.clone());
    assert!(known.is_some(), "the owner was shown the card going in");
}

#[test]
fn r1838002_the_trades_draw_is_a_draw_and_the_trade_no_play() {
    let (state, spell, _unit) = holding("r1838002-draw");
    let out = trade(&state, P1, &spell, "t1");
    // Counted as the turn's draw, as any draw is (R457), and as no play.
    assert_eq!(draws_this_turn(&out.state, P1), 1);
    assert!(out.events.iter().any(|event| matches!(
        event,
        GameEvent::Drawn {
            turn_draw: Some(1),
            ..
        }
    )));
    assert_eq!(out.state.players.p1.turn_log.cards_played, 0);
    assert_eq!(out.state.counters.played, state.counters.played);
    assert!(!types_of(&out.events).contains(&GameEventType::CardPlayed));
    assert!(!types_of(&out.events).contains(&GameEventType::Discarded));

    // A draw limit stops the trade's draw as it stops any (R457): the card stays in the deck.
    let (mut limited, spell, _unit) = holding("r1838002-limited");
    put(&mut limited, &no_draws().id, slot(P1, Row::Backrow, 1), json!({}));
    let hand = limited.players.p1.hand.len();
    let out = trade(&limited, P1, &spell, "t1");
    assert_eq!(out.error, None, "a draw limit does not refuse the Trade");
    assert!(types_of(&out.events).contains(&GameEventType::DrawLimited));
    assert!(!types_of(&out.events).contains(&GameEventType::Drawn));
    assert_eq!(out.state.players.p1.hand.len(), hand - 1);
    assert!(matches!(
        find_instance(&out.state, &spell).map(|card| card.zone.clone()),
        Some(Zone::Library { player: P1 })
    ));
}

/// p1 with no mana, holding only the Tradable Spell, no unit: nothing to do but end the turn, trade
/// or concede.
fn only_a_trade(seed: &str) -> (GameState, String) {
    let (mut state, spell, unit) = holding(seed);
    state.players.p1.mana.current = 0;
    state.players.p1.hand.retain(|card| card.id != unit);
    (state, spell)
}

#[test]
fn r1838003_a_trade_left_keeps_the_turn_from_ending_by_itself() {
    let (state, spell) = only_a_trade("r1838003-auto");
    let poke = Action::new(ActionBody::SetAutoEndTurn { enabled: true }, P1, "p0");
    let kept = reduce(&state, &poke);
    assert_eq!(kept.error, None);
    assert_eq!(kept.state.active, P1, "a Trade left is something to do (R82)");
    assert!(!types_of(&kept.events).contains(&GameEventType::TurnAutoEnded));
    // Once it is made and nothing is left, the turn ends by itself.
    let traded = trade(&kept.state, P1, &spell, "t1");
    assert_eq!(traded.error, None);
    assert!(types_of(&traded.events).contains(&GameEventType::TurnAutoEnded));
    assert_eq!(traded.state.active, P2);
}

#[test]
fn r1838003_the_random_policy_may_trade_and_the_timeout_never_does() {
    let (state, spell) = only_a_trade("r1838003-policy");
    let offered = jackioh_engine::subsystems::ai_policy::policy_actions(&state, P1, Default::default());
    assert!(
        offered.contains(&trade_of(&spell)),
        "the random policy draws from the Trades too"
    );
    let timed_out = reduce(&state, &Action::new(ActionBody::Timeout, P1, "srv-t"));
    assert_eq!(timed_out.error, None);
    assert!(!types_of(&timed_out.events).contains(&GameEventType::Traded));
    assert_eq!(
        timed_out.state.active, P2,
        "the clock ends the turn and trades nothing"
    );
}

#[test]
fn r1864002_an_aura_lends_tradable_to_its_controllers_hand_while_it_acts() {
    let (mut state, _spell, unit) = holding("r1864002-lent");
    let theirs = state.players.p2.hand[0].id.clone();
    let is_tradable_now = |state: &GameState, id: &str| {
        let card = find_instance(state, id).expect("the card in a hand");
        is_tradable(state, card)
    };
    assert!(
        !is_tradable_now(&state, &unit),
        "the plain Unit prints no Tradable"
    );

    // A lender on p2's side lends p1's hand nothing.
    let mut across = state.clone();
    put(&mut across, &lender().id, slot(P2, Row::Backrow, 1), json!({}));
    assert!(!is_tradable_now(&across, &unit));
    assert!(
        is_tradable_now(&across, &theirs),
        "it lends its own controller's hand"
    );

    put(&mut state, &lender().id, slot(P1, Row::Backrow, 1), json!({}));
    assert!(is_tradable_now(&state, &unit), "lent by the aura while it acts");
    assert!(trades_listed(&state, P1).contains(&unit));
    assert!(
        !is_tradable_now(&state, &theirs),
        "the other player's hand is not lent it"
    );
    assert_eq!(trade(&state, P1, &unit, "t1").error, None);
}

#[test]
fn r1864003_a_trigger_on_traded_is_handed_the_card_the_trade_put_into_the_deck() {
    let (mut state, spell, _unit) = holding("r1864003-handed");
    put(
        &mut state,
        &trade_watcher().id,
        slot(P1, Row::Backrow, 1),
        json!({}),
    );
    let out = trade(&state, P1, &spell, "t1");
    assert_eq!(out.error, None);
    assert!(
        out.state.players.p1.exile.iter().any(|card| card.id == spell),
        "the watcher exiled the traded card"
    );
    assert!(out.events.iter().any(|event| matches!(
        event,
        GameEvent::Exiled { instance_id, .. } if *instance_id == spell
    )));
    // The id rode on the queue entry alone: the public `traded` still names no card (R1838001).
    assert!(out.events.contains(&GameEvent::Traded { player: P1 }));
}

#[test]
fn r1864004_a_card_acting_on_the_field_gives_trades_a_turn_the_most_holding() {
    let (mut state, spell, _unit) = holding("r1864004-given");
    let second = in_hand(&mut state, &tradable_spell().id, P1, 1).remove(0).id;
    let third = in_hand(&mut state, &tradable_spell().id, P1, 1).remove(0).id;
    put(&mut state, &two_trades().id, slot(P1, Row::Backrow, 1), json!({}));
    assert_eq!(trades_per_turn_of(&state, P1), FIXTURE_TRADES);
    assert_eq!(
        trades_per_turn_of(&state, P2),
        TRADES_PER_TURN,
        "it gives only its controller"
    );

    // A second one changes nothing: the most holds, never a sum.
    let mut doubled = state.clone();
    put(
        &mut doubled,
        &two_trades().id,
        slot(P1, Row::Backrow, 2),
        json!({}),
    );
    assert_eq!(trades_per_turn_of(&doubled, P1), FIXTURE_TRADES);

    // A larger figure of the player's own wins.
    let mut own = state.clone();
    own.players.p1.trades_per_turn = Some(FIXTURE_TRADES + 1);
    assert_eq!(trades_per_turn_of(&own, P1), FIXTURE_TRADES + 1);

    let once = trade(&state, P1, &spell, "t1").state;
    let twice = trade(&once, P1, &second, "t2");
    assert_eq!(twice.error, None, "two Trades a turn");
    let thrice = trade(&twice.state, P1, &third, "t3");
    assert_eq!(
        thrice.error.as_deref(),
        Some(format!("you have made this turn's {FIXTURE_TRADES} Trades").as_str())
    );
}

#[test]
fn r2574000_view_marks_every_hand_card_of_the_viewer_passed_as_tradable() {
    let (mut state, spell, unit) = holding("r2574000-view");
    let theirs = state.players.p2.hand[0].id.clone();

    let view_plain = view_for(&state, P1);
    assert_eq!(
        hand_card(&view_plain, &spell).tradable,
        Some(true),
        "a card printing Tradable has tradable: true without the aura"
    );
    assert_eq!(
        hand_card(&view_plain, &unit).tradable,
        None,
        "a plain unit has no tradable key"
    );

    let mut state_given = state.clone();
    find_instance_mut(&mut state_given, &unit)
        .expect("the Unit in hand")
        .granted_keywords
        .push(Keyword::Tradable);
    let view_given = view_for(&state_given, P1);
    assert_eq!(
        hand_card(&view_given, &unit).tradable,
        Some(true),
        "a card given Tradable has tradable: true"
    );

    put(&mut state, &lender().id, slot(P1, Row::Backrow, 1), json!({}));
    let view_aura = view_for(&state, P1);
    let c_spell = hand_card(&view_aura, &spell);
    let c_unit = hand_card(&view_aura, &unit);
    assert_eq!(c_spell.tradable, Some(true));
    assert_eq!(
        c_unit.tradable,
        Some(true),
        "aura-lent card in viewer hand has tradable: true"
    );
    assert!(
        !c_unit
            .keywords
            .as_ref()
            .is_some_and(|list| list.iter().any(|k| k.kind() == KeywordKind::Tradable)),
        "the hand card's keywords still lack Tradable (R1864002)"
    );

    let view_p2 = view_for(&state, P2);
    assert_eq!(
        hand_card(&view_p2, &theirs).tradable,
        None,
        "opponent's hand card without aura has no tradable key"
    );
    match &view_aura.opponent.hand {
        HandView::Count { .. } => {}
        HandView::Cards(_) => panic!("opponent hand during live game must be HandView::Count"),
    }

    let mut state_removed = state.clone();
    state_removed.players.p1.backrow[0] = None;
    let view_removed = view_for(&state_removed, P1);
    assert_eq!(
        hand_card(&view_removed, &unit).tradable,
        None,
        "with aura gone, card that only aura made Tradable has no tradable key"
    );
    assert_eq!(
        hand_card(&view_removed, &spell).tradable,
        Some(true),
        "printed Tradable still has tradable: true"
    );

    let mut finished = state.clone();
    finished.result = Some(GameResult {
        winner: Winner::P1,
        reason: GameOverReason::HeroDeath,
    });
    let view_finished = view_for(&finished, P1);
    let HandView::Cards(own_cards) = &view_finished.you.hand else {
        panic!("finished game you.hand must be HandView::Cards");
    };
    for card in own_cards {
        assert_eq!(
            card.tradable, None,
            "post-game viewer hand cards never carry tradable"
        );
    }
    let HandView::Cards(opp_cards) = &view_finished.opponent.hand else {
        panic!("finished game opponent.hand must be HandView::Cards");
    };
    for card in opp_cards {
        assert_eq!(
            card.tradable, None,
            "post-game opponent hand cards never carry tradable"
        );
    }
}
