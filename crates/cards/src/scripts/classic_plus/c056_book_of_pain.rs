//! C+ #56 Book of Pain (SPEC §8.7 row 56). (1) Spell, Book, Epic.
//!   Base:    "Your opponent discards {discards|card|cards}." — discards 2
//!   Radiant: the same text, discards 4.
//!   Engine:  "Random from their hand (R682): no prompt opens; fewer cards → all they have, none →
//!            nothing; a discarded unit-token card ceases to exist (R11). Tunes: discards 2 ↑."
//!
//! No prompt opens (R682), so the caster never waits on the opponent. The discard lands as the Cry
//! reaches it.

use jackioh_engine::effects::discard_random;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-056";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![discard_random(json_as(json!({ "count": param(&*ctx, "discards"), "player": "enemy" })))]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's 4 is its declared `discards`, which `param` reads.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// C+ #56 Book of Pain — SPEC §8.7 row 56, BUILD M9 Classic+ row C+ 56: "The opponent discards 2 cards
// at random (R682), with no prompt; the opponent's remaining hand stays hidden (§10.6) while the
// discards are public; fewer cards, all of them; an empty hand, nothing; the count reads through
// `param()`; radiant 4".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const BOOK: &str = "classicplus-056";
    const FILLER: &str = "core-005";
    const A: &str = "core-008"; // Mr. Vanilla
    const B: &str = "core-011"; // Tempo Timmy
    const C: &str = "core-020"; // Pointmaster
    const RUSH: &str = "core-t-rush"; // a unit-token card (R11)

    /// `_harness.ts` registers every card on import; the engine's testkit cannot, so this does.
    fn scenario(opts: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(opts)
    }

    /// TS `pain({ radiant?, theirs? })`: `None` is TS's `[A, B, C]`.
    fn pain(radiant: bool, theirs: Option<&[&str]>) -> Scenario {
        let theirs: Vec<&str> = theirs.map_or_else(|| vec![A, B, C], |cards| cards.to_vec());
        scenario(json!({
            "p1": { "hand": [{ "def": BOOK, "radiant": radiant }, FILLER] },
            "p2": { "hand": theirs },
        }))
    }

    fn grave_defs(s: &Scenario) -> Vec<String> {
        s.pile(PlayerId::P2, "graveyard").into_iter().map(|card| card.def_id).collect()
    }

    #[test]
    fn is_a_1_spell_book_both_faces_run_one_script() {
        crate::register_all();
        assert_eq!(crate::card_def(super::ID).id, BOOK);
        let scripts = super::script();
        // TS `expect(radiant).toBe(base)`: the one Cry hook.
        assert!(std::sync::Arc::ptr_eq(
            scripts.radiant.cry.as_ref().expect("a Cry"),
            scripts.base.cry.as_ref().expect("a Cry"),
        ));
    }

    mod base {
        use super::*;

        #[test]
        fn r682_no_prompt_opens_2_random_cards_of_theirs_go_at_once() {
            let mut s = pain(false, None);
            s.play(BOOK, json!({}));
            assert_eq!(s.state().active, PlayerId::P1);
            assert!(s.state().pending.is_none());
            assert_eq!(s.hand(PlayerId::P2).len(), 1);
            let grave = grave_defs(&s);
            assert_eq!(grave.len(), 2);
            for def_id in &grave {
                assert!([A, B, C].contains(&def_id.as_str()));
            }
            s.expect_in_zone(BOOK, "graveyard");
        }

        #[test]
        fn s10_6_r682_the_opponents_remaining_hand_stays_hidden_while_the_discards_are_public() {
            let mut s = pain(false, None);
            s.play(BOOK, json!({}));
            assert!(s.view(PlayerId::P1).pending.is_none());
            assert!(s.view(PlayerId::P2).pending.is_none());
            let mine = serde_json::to_string(&s.view(PlayerId::P1)).unwrap();
            for card in s.hand(PlayerId::P2) {
                assert!(!mine.contains(&format!("\"{}\"", card.id)));
                assert!(!mine.contains(&card.def_id));
            }
            let seen = s
                .view(PlayerId::P1)
                .events
                .iter()
                .filter(|event| matches!(event, GameEvent::Discarded { .. }))
                .count();
            assert_eq!(seen, 2);
            for card in s.pile(PlayerId::P2, "graveyard") {
                assert!(mine.contains(&card.id));
            }
        }

        #[test]
        fn r682_the_random_discards_come_from_the_match_rng_the_same_game_discards_the_same_cards() {
            let mut first = pain(false, None);
            first.play(BOOK, json!({}));
            let mut second = pain(false, None);
            second.play(BOOK, json!({}));
            let ids = |s: &Scenario| -> Vec<String> {
                s.events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::Discarded { instance_id, .. } => Some(instance_id.clone()),
                        _ => None,
                    })
                    .collect()
            };
            assert_eq!(ids(&first), ids(&second));
        }

        #[test]
        fn fewer_cards_than_asked_they_discard_all_they_have() {
            let mut s = pain(false, Some(&[A]));
            s.play(BOOK, json!({}));
            assert!(s.state().pending.is_none());
            assert_eq!(s.hand(PlayerId::P2), Vec::<CardInstance>::new());
            assert_eq!(grave_defs(&s), vec![A.to_string()]);
        }

        #[test]
        fn an_empty_hand_opens_no_prompt_and_the_spell_still_resolves() {
            let mut s = pain(false, Some(&[]));
            s.play(BOOK, json!({}));
            assert!(s.state().pending.is_none());
            assert!(!s.last_events().iter().any(|event| matches!(event, GameEvent::PromptOpened { .. })));
            s.expect_in_zone(BOOK, "graveyard");
        }

        #[test]
        fn r11_a_discarded_unit_token_card_ceases_to_exist() {
            let mut s = pain(false, Some(&[RUSH, A]));
            let token = s
                .hand(PlayerId::P2)
                .into_iter()
                .find(|card| card.def_id == RUSH)
                .map(|card| card.id)
                .unwrap_or_default();
            s.play(BOOK, json!({}));
            assert!(s.state().pending.is_none());
            s.expect_in_zone(&token, "gone");
            assert_eq!(s.hand(PlayerId::P2), Vec::<CardInstance>::new());
        }

        #[test]
        fn r113_nothing_pauses_on_the_opponent_no_prompt_no_work_owed() {
            let mut s = pain(false, None);
            s.play(BOOK, json!({}));
            assert!(s.state().pending.is_none());
            assert!(s.state().work.is_empty());
        }

        #[test]
        fn r386_an_upgrade_asks_for_3_a_degrade_for_1() {
            let mut up = pain(false, None);
            step_param(up.card_mut(BOOK), "discards", 1);
            up.play(BOOK, json!({}));
            assert!(up.state().pending.is_none());
            assert_eq!(up.pile(PlayerId::P2, "graveyard").len(), 3);
            assert_eq!(up.hand(PlayerId::P2).len(), 0);

            let mut down = pain(false, None);
            step_param(down.card_mut(BOOK), "discards", -1);
            down.play(BOOK, json!({}));
            assert!(down.state().pending.is_none());
            assert_eq!(down.pile(PlayerId::P2, "graveyard").len(), 1);
            assert_eq!(down.hand(PlayerId::P2).len(), 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r682_the_opponent_discards_4_at_random() {
            let mut s = pain(true, Some(&[A, B, C, A, B]));
            s.play(BOOK, json!({}));
            assert!(s.state().pending.is_none());
            assert_eq!(s.hand(PlayerId::P2).len(), 1);
            assert_eq!(s.pile(PlayerId::P2, "graveyard").len(), 4);
        }

        #[test]
        fn with_3_cards_they_discard_all_3() {
            let mut s = pain(true, None);
            s.play(BOOK, json!({}));
            assert!(s.state().pending.is_none());
            assert_eq!(s.hand(PlayerId::P2), Vec::<CardInstance>::new());
            assert_eq!(s.pile(PlayerId::P2, "graveyard").len(), 3);
        }
    }
}
