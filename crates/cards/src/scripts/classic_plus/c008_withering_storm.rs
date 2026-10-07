//! C+ #8 Withering Storm (SPEC §8.7 row 8). (2) Spell, Rare.
//! One Degrade (R386) on each of {cards} different random cards of the opponent's deck, drawn among the
//! cards a Degrade can change (R60; all of them when fewer), then draw {draw}. Radiant: one Degrade on
//! every card of their deck. The changes stay hidden from both players while in the deck (R311).
//!
//! R569: so the count of `degraded` cues never says how many deck cards could change (R440), the pick is
//! padded to {cards} (or the deck's size) with cues on cards no change reaches, which the Degrade leaves
//! alone with the change `none` — the same cue R440 gives every unchangeable card of the Radiant sweep.

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-008";

/// The deck cards the base face reaches, in deck order (R242); read once, so a pause resumes over them (R113).
fn picks(ctx: &mut EffectContext<'_>) -> Vec<CardInstance> {
    let deck = zone_cards(&ctx.state, opponent_of(ctx.controller), OffFieldZone::Library);
    let count = param(&*ctx, "cards").min(deck.len() as i32).max(0) as usize;
    let changeable: Vec<CardInstance> = deck
        .iter()
        .filter(|card| !applicable_changes(&ctx.state, card, TuneDirection::Degrade).is_empty())
        .cloned()
        .collect();
    // R60, R129: N different cards, or all of them (and no draw) when there are no more than N.
    let chosen: Vec<CardInstance> = if changeable.len() <= count {
        changeable.clone()
    } else {
        ctx.rng.shuffle(&changeable).into_iter().take(count).collect()
    };
    let mut ids: IndexSet<String> = chosen.into_iter().map(|card| card.id).collect();
    for card in &deck {
        if ids.len() < count && !changeable.iter().any(|each| each.id == card.id) {
            ids.insert(card.id.clone());
        }
    }
    deck.into_iter().filter(|card| ids.contains(&card.id)).collect()
}

/// `forEachCard`'s `cards`, typed (TS `(ctx) => readonly (CardInstance | string)[]`, ids here).
fn cards_of(f: impl Fn(&mut EffectContext<'_>) -> Vec<String> + Send + Sync + 'static) -> ForEachCardCards {
    Arc::new(f)
}

/// `forEachCard`'s `each`, typed (TS `(instanceId) => Effect`).
fn each_of(f: impl Fn(&str) -> Effect + Send + Sync + 'static) -> ForEachCardEach {
    Arc::new(f)
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|ctx| {
                vec![
                    for_each_card(ForEachCardArgs {
                        cards: cards_of(|now| picks(now).into_iter().map(|card| card.id).collect()),
                        each: each_of(|instance_id| degrade(json_as(json!({ "instanceId": instance_id })))),
                    }),
                    draw(json_as(json!({ "count": param(&*ctx, "draw") }))),
                ]
            })),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|ctx| {
                vec![
                    degrade(json_as(json!({ "scope": { "side": "enemy", "zones": ["library"] } }))),
                    draw(json_as(json!({ "count": param(&*ctx, "draw") }))),
                ]
            })),
            ..Script::default()
        },
    }
}

// C+ #8 Withering Storm — SPEC §8.7 row 8, BUILD M9 Classic+ row C+ 8: "Degrades 4 different random
// cards in the opponent's deck (R60), drawn among the cards a Degrade can change (an Immutable card, or
// one no change reaches, is never picked), one draw from R386's menu each, then you draw 1; a deck with
// 3 or fewer such cards degrades them all, one with none degrades nothing and draws no random number
// (R129), and you still draw; the changes are `tuning` and leave the deck with the card; `degraded`
// events stay unread by both players while the cards are in the deck (R177, R311) and the opponent's
// library list does not change; card count and draw read through `param()`; radiant degrades every card
// in the opponent's deck once".
//
// R569 (with R440): the pick is drawn among the cards a Degrade can change, and the count of `degraded`
// cues never says how many of the deck's cards could — it is padded with `none` cues on cards no change
// reaches, up to {cards} (or the deck's size).
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const STORM: &str = "classicplus-008";
    const VANILLA: &str = "core-008"; // (1) 4/4: a Degrade can change its cost or its stats.
    const NETHER: &str = "core-088"; // (4) Spell, no keywords, no numbers: no Degrade reaches it.
    const HINDER: &str = "core-021"; // Cast on draw: … Discard 1 at random with no prompt (R682).
    const FILLER: &str = "core-010";
    const STOCKPILE: &str = "core-005";

    /// Radiant Midrange Menace: Immutable.
    fn immutable() -> Value {
        json!({ "def": "core-019", "radiant": true })
    }

    fn setup(deck: Value, radiant_face: bool, seed: Option<&str>, p1_library: Value) -> Scenario {
        crate::register_all();
        let mut options = json!({
            "p1": { "hand": [{ "def": STORM, "radiant": radiant_face }, FILLER], "library": p1_library },
            "p2": { "hand": [FILLER], "library": deck },
        });
        if let Some(seed) = seed {
            options["seed"] = json!(seed);
        }
        scenario(options)
    }

    /// The TS default `p1Library`.
    fn two_stockpiles() -> Value {
        json!([STOCKPILE, STOCKPILE])
    }

    fn vanillas(n: usize) -> Value {
        json!(vec![VANILLA; n])
    }

    fn cues(events: &[GameEvent]) -> Vec<GameEvent> {
        events.iter().filter(|event| matches!(event, GameEvent::Degraded { .. })).cloned().collect()
    }

    fn changed(events: &[GameEvent]) -> Vec<GameEvent> {
        cues(events)
            .into_iter()
            .filter(|event| !matches!(event, GameEvent::Degraded { change: TuningChange::None, .. }))
            .collect()
    }

    fn cue_id(event: &GameEvent) -> String {
        match event {
            GameEvent::Degraded { instance_id, .. } => instance_id.clone(),
            _ => String::new(),
        }
    }

    /// Whether a Degrade left anything on the card (its cost or its tuning).
    fn was_changed(card: &CardInstance) -> bool {
        card.cost_mod != 0 || card.tuning.is_some()
    }

    #[test]
    fn is_a_2_spell_declaring_cards_4_and_draw_1_on_both_faces() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.type_, CardType::Spell);
        assert_eq!(def.cost, CardCost::Fixed(2));
        assert_eq!(
            def.params.unwrap_or_default().iter().map(|p| (p.key.clone(), p.base, p.radiant)).collect::<Vec<_>>(),
            vec![("cards".to_string(), 4, 4), ("draw".to_string(), 1, 1)]
        );
        let scripts = script();
        assert!(scripts.base.cry.is_some());
        assert!(scripts.radiant.cry.is_some());
    }

    mod base {
        use super::*;

        #[test]
        fn r60_r386_degrades_4_different_cards_of_the_opponent_s_deck_one_change_each_then_you_draw_1() {
            crate::register_all();
            let mut s = setup(vanillas(8), false, None, two_stockpiles());
            let hand = s.hand(P1).len();

            s.play(STORM, json!({}));

            let done = changed(s.events());
            assert_eq!(done.len(), 4);
            assert_eq!(cues(s.events()).len(), 4);
            assert_eq!(done.iter().map(cue_id).collect::<IndexSet<_>>().len(), 4);
            let deck = s.pile(P2, "library");
            let mut tuned: Vec<String> = deck.iter().filter(|card| was_changed(card)).map(|card| card.id.clone()).collect();
            tuned.sort();
            let mut named: Vec<String> = done.iter().map(cue_id).collect();
            named.sort();
            assert_eq!(tuned, named);
            assert_eq!(s.hand(P1).len(), hand - 1 + 1);
            s.expect_events(json!(["degraded", "drawn"]));
        }

        #[test]
        fn r569_r60_picks_only_among_cards_a_degrade_can_change_immutable_and_unreachable_cards_are_never_picked() {
            crate::register_all();
            for seed in 1..=10 {
                let deck = json!([
                    immutable(), VANILLA, NETHER, VANILLA, VANILLA, immutable(), VANILLA, NETHER, VANILLA, VANILLA
                ]);
                let mut s = setup(deck, false, Some(&format!("storm-{seed}")), two_stockpiles());

                s.play(STORM, json!({}));

                let deck = s.pile(P2, "library");
                assert!(!deck.iter().filter(|card| card.def_id != VANILLA).any(was_changed));
                assert_eq!(deck.iter().filter(|card| was_changed(card)).count(), 4);
                assert_eq!(cues(s.events()).len(), 4);
            }
        }

        #[test]
        fn r569_r440_a_deck_with_3_changeable_cards_degrades_all_3_and_a_none_cue_keeps_the_count_at_4() {
            crate::register_all();
            let mut s = setup(json!([immutable(), VANILLA, NETHER, VANILLA, VANILLA]), false, None, two_stockpiles());

            s.play(STORM, json!({}));

            assert_eq!(changed(s.events()).len(), 3);
            assert_eq!(cues(s.events()).len(), 4);
            assert!(s.pile(P2, "library").iter().filter(|card| card.def_id == VANILLA).all(was_changed));
        }

        #[test]
        fn r569_r129_a_deck_no_degrade_can_change_is_left_alone_with_no_random_number_drawn_and_you_still_draw() {
            crate::register_all();
            let mut s = setup(json!([immutable(), NETHER, immutable()]), false, None, two_stockpiles());
            let cursor = s.state().rng_cursor;
            let hand = s.hand(P1).len();

            s.play(STORM, json!({}));

            assert_eq!(changed(s.events()).len(), 0);
            assert_eq!(cues(s.events()).len(), 3); // R440: one cue per card it reached, `none` on each
            assert!(!s.pile(P2, "library").iter().any(was_changed));
            assert_eq!(s.state().rng_cursor, cursor);
            assert_eq!(s.hand(P1).len(), hand);
        }

        #[test]
        fn an_empty_deck_nothing_to_degrade_and_you_still_draw() {
            crate::register_all();
            let mut s = setup(json!([]), false, None, two_stockpiles());
            let hand = s.hand(P1).len();

            s.play(STORM, json!({}));

            assert_eq!(cues(s.events()).len(), 0);
            assert_eq!(s.hand(P1).len(), hand);
        }

        #[test]
        fn r386_the_changes_are_tuning_they_leave_the_deck_with_the_card_which_shows_them_once_its_owner_draws_it() {
            crate::register_all();
            let mut s = setup(vanillas(4), false, None, two_stockpiles());
            s.play(STORM, json!({}));
            let top = s.pile(P2, "library").first().cloned().expect("a deck card");
            assert!(was_changed(&top));

            s.end_turn(); // p2's start of turn draws the top card

            let drawn = s.card(&top.id).clone();
            assert_eq!(drawn.zone.z(), ZoneName::Hand);
            assert!(was_changed(&drawn));
            let HandView::Cards(shown) = s.view(P2).you.hand else {
                panic!("p2 reads its own hand");
            };
            let card = shown.iter().find(|each| each.instance_id == top.id);
            let numbers = card.map(|card| (Some(card.cost), card.attack, card.health)).unwrap_or((None, None, None));
            assert_ne!(numbers, (Some(1), Some(4), Some(4)));
        }

        #[test]
        fn r177_r311_the_degraded_cues_stay_unread_by_both_players_and_the_opponent_s_library_list_does_not_change() {
            crate::register_all();
            let mut s = setup(vanillas(5), false, None, two_stockpiles());
            let list_before = serde_json::to_string(&s.view(P2).you.own_library).unwrap();

            s.play(STORM, json!({}));

            for viewer in [P1, P2] {
                let seen = cues(&s.view(viewer).events);
                assert_eq!(seen.len(), 4);
                for cue in &seen {
                    let GameEvent::Degraded { instance_id, def_id, .. } = cue else {
                        panic!("a degraded cue");
                    };
                    assert_eq!((instance_id.as_str(), def_id.as_str()), ("hidden", "hidden"));
                }
                assert!(!serde_json::to_string(&seen).unwrap().contains(VANILLA));
            }
            assert_eq!(serde_json::to_string(&s.view(P2).you.own_library).unwrap(), list_before);
        }

        #[test]
        fn r682_the_draw_s_cast_asks_nothing_and_the_degrades_are_still_made_once() {
            crate::register_all();
            let mut s = setup(vanillas(5), false, None, json!([HINDER, STOCKPILE]));

            s.play(STORM, json!({}));

            // Base Hinder's discard is random (R682): the draw's cast opens no prompt.
            assert!(s.state().pending.is_none());
            let as_json = serde_json::to_value(s.state()).unwrap();
            let revived: GameState = serde_json::from_value(as_json.clone()).unwrap();
            assert_eq!(serde_json::to_value(&revived).unwrap(), as_json);
            assert!(s.state().work.is_empty());

            // Hinder was cast (its victim the one card held) and the repeat drew Stockpile: the Degrades
            // were made once, and answering nothing makes none again.
            assert_eq!(s.card(HINDER).zone.z(), ZoneName::Graveyard);
            assert_eq!(s.card(STOCKPILE).zone.z(), ZoneName::Hand);
            assert_eq!(changed(s.events()).len(), 4);
            assert_eq!(s.state().players.p2.library.iter().filter(|card| was_changed(card)).count(), 4);
        }

        #[test]
        fn r386_card_count_and_draw_read_through_param_an_upgrade_of_each_moves_what_resolves() {
            crate::register_all();
            let mut s = setup(vanillas(8), false, None, two_stockpiles());
            step_param(s.card_mut(STORM), "cards", 1);
            s.play(STORM, json!({}));
            assert_eq!(changed(s.events()).len(), 5);

            let mut t = setup(json!([VANILLA]), false, None, json!([STOCKPILE, STOCKPILE, STOCKPILE]));
            let hand = t.hand(P1).len();
            step_param(t.card_mut(STORM), "draw", 1);
            t.play(STORM, json!({}));
            assert_eq!(t.hand(P1).len(), hand - 1 + 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn degrades_every_card_of_the_opponent_s_deck_once_leaving_immutable_and_unreachable_cards_alone() {
            crate::register_all();
            let deck = json!([VANILLA, immutable(), VANILLA, NETHER, VANILLA, VANILLA, VANILLA, VANILLA]);
            let mut s = setup(deck, true, None, two_stockpiles());
            let hand = s.hand(P1).len();

            s.play(STORM, json!({}));

            let deck = s.pile(P2, "library");
            assert!(deck.iter().filter(|card| card.def_id == VANILLA).all(was_changed));
            assert!(!deck.iter().filter(|card| card.def_id != VANILLA).any(was_changed));
            assert_eq!(changed(s.events()).len(), 6);
            assert_eq!(cues(s.events()).len(), 8);
            assert_eq!(s.hand(P1).len(), hand);
        }
    }
}
