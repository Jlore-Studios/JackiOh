//! R979000 (issue #979): a card in the viewer's own hand that cannot be played says why
//! (`CardView.play_reason`, §10.8), as a unit that cannot attack says why (`UnitView.attackReason`,
//! R849000). The view carries the first refusal in a fixed order — ban, price, zone, Tribute,
//! choices — in the engine's own words, each read off the function `legal_actions` and §10.5 step 1
//! already call. The client reads the field and works nothing out (CLAUDE.md rule 7).
//!
//! Fixtures are prefixed `pr-` and indexed from 4700 so they cannot collide (BUILD §0). The ban,
//! price, zone and Tribute cases reuse the play pipeline B fixtures (docs/classic-sets.md B5);
//! the required-target case needs its own declaration (`required`, R703). R2975000 (issue #2975)
//! ends the order with the choices' own costs, served by `pr-pair` (4701) and pipeline A's Ghost.

use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{in_hand, put, slot};
use crate::rules::fixtures::play_pipeline_b::{
    five_unit, grave_spell, grave_unit, lobbyist, pb_playing, plays_of, three_spell, titan_two,
};

/// A Spell with one needed unit pick (R703): unanswerable on an empty board.
fn needed() -> CardDef {
    json_as(json!({
        "id": "pr-needed",
        "index": "4700",
        "name": "needed (playReason)",
        "set": "Core",
        "type": "Spell",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": "needed" },
        "radiant": { "keywords": [], "text": "needed" },
    }))
}

/// R2975000: a Spell with two needed enemy unit picks (R703), two Ghosts' discards owed at once.
fn pair() -> CardDef {
    json_as(json!({
        "id": "pr-pair",
        "index": "4701",
        "name": "pair (playReason)",
        "set": "Core",
        "type": "Spell",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": "pair" },
        "radiant": { "keywords": [], "text": "pair" },
    }))
}

fn pair_face() -> Script {
    Script {
        targets: json_as(json!([
            { "kind": "target", "min": 2, "max": 2, "filter": { "side": "enemy", "of": ["unit"] }, "required": true },
        ])),
        cry: Some(hook(|_ctx| {
            vec![effects::damage(json_as(
                json!({ "to": { "of": "chosen" }, "amount": 1 }),
            ))]
        })),
        ..Script::default()
    }
}

fn scripts() -> IndexMap<String, CardScripts> {
    IndexMap::from([(
        needed().id.clone(),
        CardScripts {
            base: Script {
                targets: json_as(json!([
                    { "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit"] }, "required": true },
                ])),
                cry: Some(hook(|_ctx| {
                    vec![effects::damage(json_as(
                        json!({ "to": { "of": "chosen" }, "amount": 1 }),
                    ))]
                })),
                ..Script::default()
            },
            radiant: Script {
                targets: json_as(json!([
                    { "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit"] }, "required": true },
                ])),
                cry: Some(hook(|_ctx| {
                    vec![effects::damage(json_as(
                        json!({ "to": { "of": "chosen" }, "amount": 1 }),
                    ))]
                })),
                ..Script::default()
            },
        },
    )])
}

/// Past the mulligans, in p1's main phase at 4 mana, with pipeline B and this file's fixtures
/// registered.
fn playing(seed: &str) -> GameState {
    let state = pb_playing(seed);
    let mut catalog = registered_catalog().clone();
    catalog.insert(needed().id.clone(), needed());
    catalog.insert(pair().id.clone(), pair());
    register_catalog(catalog);
    let mut all = registered_scripts().clone();
    all.extend(scripts());
    all.insert(
        pair().id.clone(),
        CardScripts {
            base: pair_face(),
            radiant: pair_face(),
        },
    );
    register_scripts(all);
    state
}

fn only<T: Clone>(items: &[T]) -> T {
    items.first().cloned().expect("expected at least one item")
}

fn hand_card(state: &mut GameState, def_id: &str, player: PlayerId) -> CardInstance {
    only(&in_hand(state, def_id, player, 1))
}

/// What `viewer` reads as this hand card's `playReason`.
fn reason_of(state: &GameState, viewer: PlayerId, instance_id: &str) -> Option<String> {
    let HandView::Cards(hand) = view_for(state, viewer).you.hand else {
        panic!("{viewer:?} reads its own hand as cards");
    };
    hand.iter()
        .find(|card| card.instance_id == instance_id)
        .unwrap_or_else(|| panic!("{viewer:?} holds no card {instance_id}"))
        .play_reason
        .clone()
}

fn has_no_plays(state: &GameState, player: PlayerId, instance_id: &str) -> bool {
    plays_of(state, instance_id, player).is_empty()
}

mod r979000_an_unplayable_hand_card_says_why {
    use super::*;

    #[test]
    fn r979000_banned_card_says_the_ban() {
        let mut state = playing("r979000-ban");
        put(
            &mut state,
            &lobbyist().id,
            slot(PlayerId::P2, Row::Units, 1),
            json_as(json!({ "radiant": true })),
        );
        let three = hand_card(&mut state, &three_spell().id, PlayerId::P1);

        assert!(has_no_plays(&state, PlayerId::P1, &three.id));
        assert_eq!(
            reason_of(&state, PlayerId::P1, &three.id),
            Some("you can't play (3)+ Cost cards now".to_string())
        );
    }

    #[test]
    fn r979000_unaffordable_card_says_its_price() {
        let mut state = playing("r979000-price");
        state.players.p1.mana.current = 0;
        let five = hand_card(&mut state, &five_unit().id, PlayerId::P1);

        assert!(has_no_plays(&state, PlayerId::P1, &five.id));
        assert_eq!(
            reason_of(&state, PlayerId::P1, &five.id),
            Some("five-unit (pipeline B) costs 5, more than your mana".to_string())
        );
    }

    #[test]
    fn r979000_full_row_says_no_free_zone() {
        let mut state = playing("r979000-zone");
        for lane in 1..=5 {
            put(
                &mut state,
                &grave_unit().id,
                slot(PlayerId::P1, Row::Units, lane),
                json!({}),
            );
        }
        let sixth = hand_card(&mut state, &grave_unit().id, PlayerId::P1);

        assert!(has_no_plays(&state, PlayerId::P1, &sixth.id));
        assert_eq!(
            reason_of(&state, PlayerId::P1, &sixth.id),
            Some("no free units zone".to_string())
        );
    }

    #[test]
    fn r979000_tribute_shortfall_says_the_tribute() {
        let mut state = playing("r979000-tribute");
        put(
            &mut state,
            &grave_unit().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let big = hand_card(&mut state, &titan_two().id, PlayerId::P1);

        assert!(has_no_plays(&state, PlayerId::P1, &big.id));
        assert_eq!(
            reason_of(&state, PlayerId::P1, &big.id),
            Some("titan-two (pipeline B) needs Tribute 2".to_string())
        );
    }

    #[test]
    fn r979000_required_target_missing_says_so() {
        let mut state = playing("r979000-target");
        let zap = hand_card(&mut state, &needed().id, PlayerId::P1);

        assert!(has_no_plays(&state, PlayerId::P1, &zap.id));
        assert_eq!(
            reason_of(&state, PlayerId::P1, &zap.id),
            Some("needed (playReason) cannot be played without 1 legal target".to_string())
        );
    }

    #[test]
    fn r979000_the_first_refusal_in_the_order_is_said() {
        // A banned card the purse cannot meet either says the ban, not its price.
        let mut state = playing("r979000-order-ban");
        put(
            &mut state,
            &lobbyist().id,
            slot(PlayerId::P2, Row::Units, 1),
            json_as(json!({ "radiant": true })),
        );
        state.players.p1.mana.current = 0;
        let three = hand_card(&mut state, &three_spell().id, PlayerId::P1);

        assert!(has_no_plays(&state, PlayerId::P1, &three.id));
        assert_eq!(
            reason_of(&state, PlayerId::P1, &three.id),
            Some("you can't play (3)+ Cost cards now".to_string())
        );

        // A unit the purse cannot meet on a full row says its price, not the row.
        let mut state = playing("r979000-order-price");
        for lane in 1..=5 {
            put(
                &mut state,
                &grave_unit().id,
                slot(PlayerId::P1, Row::Units, lane),
                json!({}),
            );
        }
        state.players.p1.mana.current = 1;
        let sixth = hand_card(&mut state, &grave_unit().id, PlayerId::P1);

        assert!(has_no_plays(&state, PlayerId::P1, &sixth.id));
        assert_eq!(
            reason_of(&state, PlayerId::P1, &sixth.id),
            Some("grave-unit (pipeline B) costs 2, more than your mana".to_string())
        );
    }

    #[test]
    fn r979000_playable_opponent_and_inactive_hands_carry_no_reason() {
        let mut state = playing("r979000-absent");
        put(
            &mut state,
            &lobbyist().id,
            slot(PlayerId::P2, Row::Units, 1),
            json_as(json!({ "radiant": true })),
        );
        let three = hand_card(&mut state, &three_spell().id, PlayerId::P1);
        let cantrip = hand_card(&mut state, &grave_spell().id, PlayerId::P1);

        // A playable card says nothing, though the ban stops its neighbour.
        assert!(!has_no_plays(&state, PlayerId::P1, &cantrip.id));
        assert_eq!(reason_of(&state, PlayerId::P1, &cantrip.id), None);
        assert_eq!(
            reason_of(&state, PlayerId::P1, &three.id),
            Some("you can't play (3)+ Cost cards now".to_string())
        );

        // The opponent's hand is a count, with nothing on it (R97).
        assert_eq!(
            view_for(&state, PlayerId::P1).opponent.hand,
            HandView::Count {
                count: state.players.p2.hand.len() as i32
            }
        );

        // The viewer's own hand on the other player's turn says nothing.
        state.active = PlayerId::P2;
        assert_eq!(reason_of(&state, PlayerId::P1, &three.id), None);

        // Post-game hands, revealed to both seats (R434), say nothing either.
        state.active = PlayerId::P1;
        state.result = Some(GameResult {
            winner: Winner::P1,
            reason: GameOverReason::HeroDeath,
        });
        let HandView::Cards(their) = view_for(&state, PlayerId::P2).opponent.hand else {
            panic!("p2 reads p1's revealed hand as cards");
        };
        assert!(
            their.iter().all(|card| card.play_reason.is_none()),
            "a revealed hand carries no playReason"
        );
    }
}

mod r979000_r2975000_every_listed_choice_refused_says_why {
    use super::*;
    use crate::rules::fixtures::play_pipeline_a::{PA, register_play_a};

    /// p1 holds `def_id` and `others` cards beside it; `ghosts` enemy Ghosts (Classic #89's fixture,
    /// 2 discards to target, R450) are the only units.
    fn ghosts_and(
        seed: &str,
        def_id: &str,
        ghosts: i32,
        others: i32,
    ) -> (GameState, CardInstance, Vec<CardInstance>) {
        let mut state = playing(seed);
        register_play_a();
        state.players.p1.hand.clear();
        let placed = (1..=ghosts)
            .map(|lane| {
                put(
                    &mut state,
                    &PA.ghost.id,
                    slot(PlayerId::P2, Row::Units, lane),
                    json!({}),
                )
            })
            .collect();
        let card = hand_card(&mut state, def_id, PlayerId::P1);
        in_hand(&mut state, &grave_spell().id, PlayerId::P1, others);
        (state, card, placed)
    }

    fn set_reason(view: &mut PlayerView, instance_id: &str, reason: Option<&str>) {
        let HandView::Cards(cards) = &mut view.you.hand else {
            panic!("p1 reads its own hand as cards")
        };
        for card in cards.iter_mut().filter(|card| card.instance_id == instance_id) {
            card.play_reason = reason.map(str::to_string);
        }
    }

    #[test]
    fn r979000_r2975000_two_ghosts_too_dear_together_say_the_discard_as_the_reducer_does() {
        let (state, pair_card, ghosts) = ghosts_and("r2975000-pair", &pair().id, 2, 3);
        let said = "targeting that costs 4 discards, and you hold 3 cards";
        assert!(has_no_plays(&state, PlayerId::P1, &pair_card.id));
        assert_eq!(
            reason_of(&state, PlayerId::P1, &pair_card.id),
            Some(said.to_string())
        );
        let targets: Vec<Value> = ghosts
            .iter()
            .map(|ghost| json!({ "pick": "instance", "instanceId": ghost.id }))
            .collect();
        let body = json!({ "type": "play", "instanceId": pair_card.id, "targets": targets });
        let refused = reduce(
            &state,
            &Action::new(json_as(body), PlayerId::P1, "r2975000-pair-play".to_string()),
        );
        assert_eq!(refused.error.as_deref(), Some(said));
    }

    #[test]
    fn r979000_r2975000_one_ghost_too_few_cards_pay_for_is_no_legal_target() {
        // R450: the Ghost is no legal target at all, so step 1 refuses the missing target, not the cost.
        let (state, zap, _) = ghosts_and("r2975000-one", &needed().id, 1, 1);
        assert!(has_no_plays(&state, PlayerId::P1, &zap.id));
        assert_eq!(
            reason_of(&state, PlayerId::P1, &zap.id),
            Some("needed (playReason) cannot be played without 1 legal target".to_string())
        );
    }

    #[test]
    fn r2975000_ghosts_the_hand_can_pay_for_list_the_play_and_say_nothing() {
        let (state, pair_card, _) = ghosts_and("r2975000-paid", &pair().id, 2, 4);
        assert!(!has_no_plays(&state, PlayerId::P1, &pair_card.id));
        assert_eq!(reason_of(&state, PlayerId::P1, &pair_card.id), None);
    }

    #[test]
    fn r2975000_i13_reports_a_card_that_says_nothing_and_passes_one_that_says_why() {
        let (state, pair_card, _) = ghosts_and("r2975000-i13", &pair().id, 2, 3);
        let view = view_for(&state, PlayerId::P1);
        let legal = legal_actions(&state, PlayerId::P1);
        assert_eq!(
            play_reason_violations(&state, PlayerId::P1, &view, &legal),
            Vec::<String>::new()
        );
        // `hidden` runs I13 beside I6 (a hand-built state may trip I6 on its own, so filter).
        let i13: Vec<String> = create_invariant_monitor(&state)
            .hidden(&state)
            .into_iter()
            .filter(|finding| finding.starts_with("I13"))
            .collect();
        assert_eq!(i13, Vec::<String>::new());

        let mut silent = view.clone();
        set_reason(&mut silent, &pair_card.id, None);
        let found = play_reason_violations(&state, PlayerId::P1, &silent, &legal);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].starts_with("I13 silent card")
                && found[0].contains(&pair_card.id)
                && found[0].contains("R979000"),
            "{found:?}"
        );

        let filler = state
            .players
            .p1
            .hand
            .iter()
            .find(|card| card.def_id == grave_spell().id)
            .expect("a filler")
            .clone();
        let mut stray = view.clone();
        set_reason(&mut stray, &filler.id, Some("stray"));
        let found = play_reason_violations(&state, PlayerId::P1, &stray, &legal);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].starts_with("I13 stray reason") && found[0].contains(&filler.id),
            "{found:?}"
        );

        // Off the viewer's turn nothing is asked (R979000's window).
        let mut theirs = state.clone();
        theirs.active = PlayerId::P2;
        assert_eq!(
            play_reason_violations(
                &theirs,
                PlayerId::P1,
                &silent,
                &legal_actions(&theirs, PlayerId::P1)
            ),
            Vec::<String>::new()
        );
    }
}
