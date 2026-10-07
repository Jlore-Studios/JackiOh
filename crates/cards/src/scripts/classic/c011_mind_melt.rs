//! C #11 Mind Melt (SPEC §8.6 row 11, §6.3 Look at a hand, §10.6, §10.8; R65, R81, R177). Spell, cost 1,
//! Common.
//!   Base:    "Look at your opponent's hand. Exile {cards|card|cards} from it."
//!   Radiant: "Look at your opponent's hand. Choose a cost. Exile every card of that cost from it."
//!   Engine:  "Look at a hand (§6.3, §10.8): their hand cards are the options of a `pick` prompt of one
//!            card (§10.6), seen by you alone. Radiant: a `mode` prompt whose options are their hand
//!            grouped by cost (the cost each would be played for now, R65); every card of the chosen
//!            cost is exiled. The opponent sees that a prompt is open, then which cards left their
//!            hand (exile is public). An empty hand opens no prompt. Tunes: cards exiled 1 ↑."
//!
//! LOOKING AT THE HAND is the prompt itself (B5 E17): `chooseFromHand({ of: "enemy" })` offers the
//! opponent's hand cards to you, and `viewFor` shows an open prompt's options to the player it is for
//! alone — the hand's owner reads only that a prompt is open for you (R177). The answer exiles what it
//! picked; exile is public, so from then on both players read those cards. The count is the card's
//! declared number (`param(ctx, "cards")`), so an Upgrade makes it two; a hand shorter than that
//! offers what it holds, and an empty hand asks nothing (`openPrompt` opens no prompt without options).
//!
//! THE RADIANT COST is the one each card would be played for now (R65, `effectiveCost`): the engine's
//! `chooseCostInHand` groups the hand by it into the options of a `number` prompt, and `exileMatching`
//! reads the same cost when it sweeps the hand, so the group chosen is the group exiled. The option
//! captions name the cards of each cost, which only you are shown.

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-011";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![choose_from_hand(json_as(json!({
                "of": "enemy",
                "count": param(&*ctx, "cards"),
                "step": "exile",
                "prompt": "Look at your opponent's hand: exile a card from it",
            })))]
        })),
        resume: IndexMap::from([(
            "exile",
            hook(|ctx| {
                ctx.targets
                    .iter()
                    .enumerate()
                    .map(|(index, _)| exile(json_as(json!({ "target": { "of": "chosen", "index": index } }))))
                    .collect()
            }),
        )]),
        ..Script::default()
    };

    let radiant = Script {
        cry: Some(hook(|_ctx| {
            vec![choose_cost_in_hand(json_as(json!({
                "of": "enemy",
                "step": "cost",
                "prompt": "Look at your opponent's hand: choose a cost",
            })))]
        })),
        resume: IndexMap::from([(
            "cost",
            hook(|ctx| match chosen_number(&*ctx) {
                None => vec![],
                Some(cost) => vec![exile_matching(json_as(json!({ "zones": ["hand"], "player": "enemy", "cost": cost })))],
            }),
        )]),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// C #11 Mind Melt — SPEC §8.6 row 11, BUILD M9 Classic row C 11: "Opens a prompt whose options are
// the opponent's hand cards, readable by you alone (§10.8), and exiles the one chosen; an empty hand
// opens no prompt; the opponent's view shows a prompt open for you and names none of its options
// (R177), then the exiled card (exile is public); once it closes your view names none of their
// remaining hand; the open prompt survives a JSON round trip; radiant: the options are their hand
// grouped by cost (each card's cost to play now, R65) and every card of the chosen cost is exiled;
// its tuned number (cards exiled) reads through `param()` (R386)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MELT: &str = "classic-011";
    const FILLER: &str = "core-005"; // (1) Spell, p1's spare card (§2.5).
    // p2's hand: five distinct definitions no other pile holds, so a def id in a view names exactly one card.
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt
    const SEVEN: &str = "core-025"; // (4) Unit 7/7
    const FELINORS: &str = "core-012"; // (2) Unit 3/4
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const REPLENISH: &str = "core-010"; // (0) Spell
    const BIGOT: &str = "core-002"; // (2) Unit 6/1
    const THEIR_HAND: [&str; 5] = [MENACE, SEVEN, FELINORS, VANILLA, REPLENISH];

    fn must<T>(value: Option<T>, what: &str) -> T {
        match value {
            Some(value) => value,
            None => panic!("the scenario has no {what}"),
        }
    }

    fn open(s: &Scenario) -> PendingChoice {
        must(s.state().pending.clone(), "open prompt")
    }

    use crate::js;

    /// TS `melt(radiantFace, theirHand = THEIR_HAND)`: each hand entry a def id or `{ def, costMod }`.
    fn melt(radiant_face: bool, their_hand: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": MELT, "radiant": radiant_face }, FILLER] },
            "p2": { "hand": their_hand, "library": [FILLER] },
        }))
    }

    fn hand_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.hand(player).into_iter().map(|card| card.def_id).collect()
    }

    /// The JSON round trip of §9.3, then the answer sent through `reduce`.
    fn revive_and_answer(s: &Scenario, selection: Value, nonce: &str) -> ReduceResult {
        let revived: GameState = serde_json::from_value(js(s.state())).unwrap();
        assert_eq!(&revived, s.state());
        let pending = must(revived.pending.clone(), "revived prompt");
        let action: Action = json_as(json!({
            "type": "answer",
            "playerId": "p1",
            "choiceId": pending.id,
            "selection": selection,
            "nonce": nonce,
        }));
        reduce(&revived, &action)
    }

    fn exiles(events: &[GameEvent], id: &str) -> bool {
        events
            .iter()
            .any(|event| matches!(event, GameEvent::Exiled { instance_id, .. } if instance_id == id))
    }

    #[test]
    fn declares_its_one_number_cards_exiled_r386() {
        assert_eq!(
            js(&crate::card_def(MELT).params),
            json!([{ "key": "cards", "base": 1, "radiant": 1, "better": "up", "step": 1, "min": 1 }])
        );
        let CardScripts { base, radiant } = script();
        assert!(base.targets.is_empty());
        assert!(radiant.targets.is_empty());
    }

    mod base {
        use super::*;

        #[test]
        fn e17_opens_a_prompt_for_you_whose_options_are_the_opponent_s_hand_cards_one_to_pick() {
            let mut s = melt(false, json!(THEIR_HAND));
            s.play(MELT, json!({}));
            let pending = open(&s);
            assert_eq!(pending.player_id, P1);
            assert_eq!(pending.kind, PromptKind::Hand);
            assert_eq!(pending.min, 1);
            assert_eq!(pending.max, 1);
            let offered: Vec<String> = pending
                .options
                .iter()
                .map(|option| match &option.selection {
                    Selection::Instance { instance_id } => s.card(instance_id.as_str()).def_id.clone(),
                    _ => "?".to_string(),
                })
                .collect();
            assert_eq!(offered, THEIR_HAND);
            // You read them: your view of the prompt names each card.
            let mine = js(&s.view(P1))["pending"].clone();
            assert!(!mine.is_null(), "p1's view of the prompt");
            assert_eq!(mine["forYou"], json!(true));
            assert!(mine.to_string().contains("Midrange Menace"));
        }

        #[test]
        fn exiles_the_card_you_choose_from_their_hand_and_only_that_one() {
            let mut s = melt(false, json!(THEIR_HAND));
            s.play(MELT, json!({}));
            let menace = s.card(MENACE).clone();
            s.answer(json!(menace.id));
            s.expect_in_zone(&menace, "exile");
            assert_eq!(hand_defs(&s, P2), [SEVEN, FELINORS, VANILLA, REPLENISH]);
            assert!(s.state().pending.is_none());
            s.expect_in_zone(MELT, "graveyard");
            s.expect_events(json!(["promptOpened", "promptAnswered", "exiled"]));
        }

        #[test]
        fn an_empty_hand_opens_no_prompt_and_the_spell_still_resolves() {
            let mut s = melt(false, json!([]));
            s.play(MELT, json!({}));
            assert!(s.state().pending.is_none());
            assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::PromptOpened));
            s.expect_in_zone(MELT, "graveyard");
        }

        #[test]
        fn r177_the_opponent_s_view_shows_a_prompt_open_for_you_and_names_none_of_its_options() {
            let mut s = melt(false, json!(THEIR_HAND));
            s.play(MELT, json!({}));
            let theirs = js(&s.view(P2))["pending"].clone();
            assert!(!theirs.is_null(), "p2's view of the prompt");
            assert_eq!(theirs, json!({ "forYou": false, "pendingFor": "p1" }));
            // p2 knows their own hand, so the check is that the prompt itself carries no option.
            assert!(!theirs.to_string().contains("Menace"));
        }

        #[test]
        fn exile_is_public_both_players_read_the_exiled_card_and_your_view_names_none_of_their_remaining_hand() {
            let mut s = melt(false, json!(THEIR_HAND));
            s.play(MELT, json!({}));
            let menace = s.card(MENACE).clone();
            s.answer(json!(menace.id));
            for viewer in [P1, P2] {
                let exiled: Vec<Value> = s
                    .view(viewer)
                    .events
                    .iter()
                    .filter(|event| event.event_type() == GameEventType::Exiled)
                    .map(|event| js(event))
                    .collect();
                assert_eq!(
                    exiled,
                    vec![json!({ "type": "exiled", "instanceId": menace.id, "defId": MENACE, "owner": "p2" })]
                );
            }
            let mine = serde_json::to_string(&s.view(P1)).unwrap();
            for card in s.hand(P2) {
                assert!(!mine.contains(&format!("\"{}\"", card.id)));
                assert!(!mine.contains(&card.def_id));
            }
        }

        #[test]
        fn sec9_3_the_open_prompt_survives_a_json_round_trip_and_resumes_through_reduce() {
            let mut s = melt(false, json!(THEIR_HAND));
            s.play(MELT, json!({}));
            let seven = s.card(SEVEN).clone();
            let result = revive_and_answer(
                &s,
                json!([{ "pick": "instance", "instanceId": seven.id }]),
                "mind-melt-round-trip",
            );
            assert!(result.error.is_none());
            assert!(result.state.pending.is_none());
            assert!(result.state.work.is_empty());
            assert!(exiles(&result.events, &seven.id));
        }

        #[test]
        fn r386_an_upgrade_of_cards_exiled_makes_it_two_picks_both_exiled() {
            let mut s = melt(false, json!(THEIR_HAND));
            step_param(s.card_mut(MELT), "cards", 1);
            s.play(MELT, json!({}));
            let pending = open(&s);
            assert_eq!(pending.min, 2);
            assert_eq!(pending.max, 2);
            let menace = s.card(MENACE).clone();
            let vanilla = s.card(VANILLA).clone();
            s.answer(json!([menace.id, vanilla.id]));
            s.expect_in_zone(&menace, "exile");
            s.expect_in_zone(&vanilla, "exile");
            assert_eq!(hand_defs(&s, P2), [SEVEN, FELINORS, REPLENISH]);
        }

        #[test]
        fn r386_an_upgraded_count_larger_than_their_hand_picks_the_whole_hand() {
            let mut s = melt(false, json!([MENACE]));
            step_param(s.card_mut(MELT), "cards", 2);
            s.play(MELT, json!({}));
            assert_eq!(open(&s).max, 1);
            let menace = s.card(MENACE).id.clone();
            s.answer(json!(menace));
            assert!(s.hand(P2).is_empty());
        }
    }

    mod radiant {
        use super::*;

        fn mixed_hand() -> Value {
            json!([MENACE, { "def": FELINORS, "costMod": -1 }, VANILLA, REPLENISH, BIGOT])
        }

        #[test]
        fn r65_the_options_are_their_hand_grouped_by_the_cost_each_would_be_played_for_now() {
            // Duplicating Felinors (2) with costMod −1 costs (1) now, so it joins Mr. Vanilla's group.
            let mut s = melt(true, mixed_hand());
            s.play(MELT, json!({}));
            let pending = open(&s);
            assert_eq!(pending.player_id, P1);
            let options: Vec<String> = pending
                .options
                .iter()
                .map(|option| match &option.selection {
                    Selection::Mode { option } => option.clone(),
                    _ => "?".to_string(),
                })
                .collect();
            assert_eq!(options, ["0", "1", "2", "3"]);
        }

        #[test]
        fn exiles_every_card_of_the_chosen_cost_from_their_hand_and_leaves_the_rest() {
            let mut s = melt(true, mixed_hand());
            s.play(MELT, json!({}));
            let felinors = s.card(FELINORS).clone();
            let vanilla = s.card(VANILLA).clone();
            s.answer(json!("1"));
            s.expect_in_zone(&felinors, "exile");
            s.expect_in_zone(&vanilla, "exile");
            assert_eq!(hand_defs(&s, P2), [MENACE, REPLENISH, BIGOT]);
            s.expect_in_zone(MELT, "graveyard");
        }

        #[test]
        fn a_cost_only_one_card_has_exiles_that_card_alone() {
            let mut s = melt(true, json!(THEIR_HAND));
            s.play(MELT, json!({}));
            s.answer(json!("4"));
            s.expect_in_zone(SEVEN, "exile");
            assert_eq!(hand_defs(&s, P2), [MENACE, FELINORS, VANILLA, REPLENISH]);
        }

        #[test]
        fn an_empty_hand_opens_no_prompt() {
            let mut s = melt(true, json!([]));
            s.play(MELT, json!({}));
            assert!(s.state().pending.is_none());
            s.expect_in_zone(MELT, "graveyard");
        }

        #[test]
        fn r177_the_opponent_s_view_shows_a_prompt_open_for_you_and_names_none_of_its_options() {
            let mut s = melt(true, json!(THEIR_HAND));
            s.play(MELT, json!({}));
            assert_eq!(js(&s.view(P2))["pending"], json!({ "forYou": false, "pendingFor": "p1" }));
            // Your view reads the cards of each cost.
            assert!(js(&s.view(P1))["pending"].to_string().contains("Midrange Menace"));
        }

        #[test]
        fn once_it_closes_your_view_names_none_of_their_remaining_hand() {
            let mut s = melt(true, json!(THEIR_HAND));
            s.play(MELT, json!({}));
            s.answer(json!("3"));
            let mine = serde_json::to_string(&s.view(P1)).unwrap();
            for card in s.hand(P2) {
                assert!(!mine.contains(&format!("\"{}\"", card.id)));
                assert!(!mine.contains(&card.def_id));
            }
            assert!(mine.contains(MENACE));
        }

        #[test]
        fn sec9_3_the_open_prompt_survives_a_json_round_trip_and_resumes_through_reduce() {
            let mut s = melt(true, json!(THEIR_HAND));
            s.play(MELT, json!({}));
            let result = revive_and_answer(&s, json!([{ "pick": "mode", "option": "2" }]), "mind-melt-radiant-round-trip");
            assert!(result.error.is_none());
            assert!(result.state.pending.is_none());
            let felinors = s.card(FELINORS).id.clone();
            assert!(exiles(&result.events, &felinors));
        }
    }
}
