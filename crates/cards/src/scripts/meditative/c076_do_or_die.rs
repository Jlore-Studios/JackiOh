//! M #76 Do or Die (SPEC §8.8 row 76): (1) Spell, Common.
//!
//! Base:    "Mark {marks|random card|random cards} in your opponent's hand. At the start of your next
//!          turn, steal each marked card still in their hand."
//! Radiant: "Mark every card in your opponent's hand. At the start of your next turn, steal each
//!          marked card still in their hand."
//! Engine:
//! - **The marks:** as it resolves, `marks` different cards of the opponent's hand are drawn from
//!   the match rng (R60; all of them when fewer); the Radiant face takes every card it holds then,
//!   and a card they draw later is not marked. With an empty hand nothing is scheduled.
//! - **The watch:** one `delay` at the start of this player's next turn (§2.2, R62), its `handWatch`
//!   the picked cards and its `mark` the steal's purple (ME-HANDMARK, R1140, R1141). A watched card
//!   is dropped as its stay in that hand ends (played, discarded, shuffled away, stolen by something
//!   else), and the entry with it once none is left, so a card that comes back later is unmarked.
//! - **The steal:** the step reads the cards still watched (`HAND_WATCH_KEY`) and takes them off the
//!   field into this player's hand as theirs, in hand order (`give_from_hand` by `ids`, §6.3 Steal,
//!   R12), the hand cap burning the rest into this player's graveyard (§2.4, R4). A unit-token card
//!   stays in a hand, so it lives on in theirs (R1145).
//! - **Hidden information:** the hand's owner sees the mark on each marked card; this player sees
//!   only how many (`SideView.handMarked`, R1141) and meets each card as it reaches their hand (R97).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-076";

/// The step the delayed steal re-enters at the start of this player's next turn (§10.6, R62).
const STEAL_STEP: &str = "steal";

/// R437, R1141: the pending steal's mark, purple as #50 K-Pop Fanatic's.
fn steal_mark() -> Value {
    json!({ "mark": "steal", "color": "purple" })
}

/// The cards the watch still holds as it comes due, in the order they were picked.
fn still_watched(ctx: &EffectContext<'_>) -> Vec<String> {
    ctx.data
        .get(HAND_WATCH_KEY)
        .and_then(Value::as_array)
        .map(|ids| ids.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default()
}

/// R12, R1140: each watched card still in their hand becomes this player's, in hand order.
fn steal_step(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let ids = still_watched(ctx);
    if ids.is_empty() {
        return vec![];
    }
    vec![give_from_hand(json_as(json!({ "from": "enemy", "ids": ids })))]
}

/// `every` is the Radiant face: every card of their hand, not `marks` of them.
fn do_or_die(every: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let hand: Vec<String> = zone_cards(ctx.state, opponent_of(ctx.controller), OffFieldZone::Hand)
                .into_iter()
                .map(|card| card.id)
                .collect();
            if hand.is_empty() {
                return vec![];
            }
            let picked: Vec<String> = if every {
                hand
            } else {
                let marks = param(&*ctx, "marks").max(0) as usize;
                ctx.rng.shuffle(&hand).into_iter().take(marks).collect()
            };
            vec![delay(json_as(json!({
                "at": { "phase": "start", "player": "self" },
                "step": STEAL_STEP,
                "hook": RESUME_HOOK,
                "handWatch": picked,
                "mark": steal_mark(),
            })))]
        })),
        resume: IndexMap::from([(STEAL_STEP, hook(steal_step))]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: do_or_die(false),
        radiant: do_or_die(true),
    }
}

// M #76 Do or Die — SPEC §8.8 row 76, BUILD M10 row M 76: "Marks two different random cards of the
// opponent's hand (all of them if fewer); at the start of your next turn each still in that hand
// becomes yours, in your hand (R12), the hand cap burning the rest into your graveyard; a marked card
// they play, discard or lose meanwhile is not taken, and one that returns to their hand later is
// unmarked (R1140); the owner sees the marks, you see only the count (R1141); an empty hand schedules
// nothing; marks reads through `param()`; radiant every card in their hand as it resolves, not cards
// drawn later".
//
// Every case crosses a turn boundary, so both sides keep a card in hand and a deck: R82 ends a turn
// with nothing left in it, and an empty deck would add fatigue.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008"; // (1) Unit 4/4: a card p2 can play.
    const FILLER: &str = "core-016"; // Hit Job: inert in a hand.
    const RUSH_TOKEN: &str = "core-t-rush"; // A unit-token card, as #75 Infinite Reserves makes one.

    fn library() -> Value {
        json!([FILLER, FILLER, FILLER, FILLER])
    }

    /// p1 holds Do or Die (Radiant on `radiant`) and a filler; p2 holds `theirs`.
    fn casting(seed: &str, radiant: bool, theirs: Value) -> Scenario {
        crate::scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": library() },
            "p2": { "hand": theirs, "library": library() },
        }))
    }

    fn ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.id.clone()).collect()
    }

    /// The cards the one delayed entry watches.
    fn watched(s: &Scenario) -> Vec<String> {
        assert_eq!(s.state().delayed.len(), 1, "one delayed steal");
        s.state().delayed[0].hand_watch.clone().expect("a hand watch")
    }

    /// The marks on each card of `seat`'s own hand, by id, as that seat's view shows them.
    fn own_hand_marks(s: &Scenario, seat: PlayerId) -> Vec<(String, bool)> {
        match s.view(seat).you.hand {
            HandView::Cards(cards) => cards
                .into_iter()
                .map(|card| (card.instance_id.clone(), card.marks.is_some_and(|marks| !marks.is_empty())))
                .collect(),
            HandView::Count { .. } => panic!("a seat's own hand is its cards"),
        }
    }

    /// Hand the turn over until `player` is the active one.
    fn until_active(s: &mut Scenario, player: PlayerId) {
        for _ in 0..2 {
            if s.state().active != player {
                s.end_turn();
            }
        }
        assert_eq!(s.state().active, player);
    }

    mod m76_do_or_die {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn marks_two_different_random_cards() {
                for n in 0..8 {
                    let mut s = casting(&format!("do-or-die-{n}"), false, json!([VANILLA, FILLER, VANILLA, FILLER]));
                    let theirs = ids(&s.hand(P2));
                    s.play(ID, json!({}));
                    let picked = watched(&s);
                    assert_eq!(picked.len(), 2);
                    assert_ne!(picked[0], picked[1]);
                    assert!(picked.iter().all(|id| theirs.contains(id)));
                    // R1141: p2 sees the two marks on its own cards.
                    let marked: Vec<String> = own_hand_marks(&s, P2)
                        .into_iter()
                        .filter(|(_, marked)| *marked)
                        .map(|(id, _)| id)
                        .collect();
                    assert_eq!(marked.len(), 2);
                    assert!(picked.iter().all(|id| marked.contains(id)));
                }
            }

            #[test]
            fn marks_reads_through_param() {
                let mut s = casting("do-or-die-param", false, json!([VANILLA, FILLER, VANILLA]));
                set_param(s.card_mut(ID), "marks", 1);
                s.play(ID, json!({}));
                assert_eq!(watched(&s).len(), 1);
            }

            #[test]
            fn r1140_fewer_than_two_marks_all() {
                let mut s = casting("do-or-die-one", false, json!([VANILLA]));
                let theirs = ids(&s.hand(P2));
                s.play(ID, json!({}));
                assert_eq!(watched(&s), theirs);
            }

            #[test]
            fn r1140_an_empty_hand_schedules_nothing() {
                let mut s = casting("do-or-die-empty", false, json!([]));
                s.play(ID, json!({}));
                assert!(s.state().delayed.is_empty());
                assert_eq!(s.view(P1).opponent.hand_marked, None);
            }

            #[test]
            fn r62_next_start_each_marked_card_still_held_becomes_yours() {
                let mut s = casting("do-or-die-steal", false, json!([VANILLA, FILLER]));
                let theirs = ids(&s.hand(P2));
                s.play(ID, json!({}));
                s.end_turn();
                // p2's own start of turn does not run it: the steal is p1's.
                assert_eq!(s.state().active, P2);
                assert_eq!(watched(&s), theirs);
                s.end_turn();
                assert_eq!(s.state().active, P1);
                assert!(s.state().delayed.is_empty());
                let mine = ids(&s.hand(P1));
                for id in &theirs {
                    assert!(mine.contains(id), "{id} reached p1's hand");
                    // R12: the stolen card is p1's now, owner and controller.
                    assert_eq!(s.card(id.as_str()).owner, P1);
                    assert_eq!(s.card(id.as_str()).controller, P1);
                }
                // In hand order, after the filler p1 kept and before the card p1 drew (R62: the delayed
                // stage comes before the draw).
                let at: Vec<usize> = theirs
                    .iter()
                    .map(|id| mine.iter().position(|held| held == id).expect("held"))
                    .collect();
                assert_eq!(at, vec![1, 2]);
                // The marks went with the watch.
                assert_eq!(s.view(P1).opponent.hand_marked, None);
                assert!(s.state().marks.is_none());
            }

            #[test]
            fn r1140_a_marked_card_they_play_is_not_taken() {
                let mut s = casting("do-or-die-played", false, json!([VANILLA, FILLER]));
                let theirs = ids(&s.hand(P2));
                s.play(ID, json!({}));
                until_active(&mut s, P2);
                let played = theirs[0].clone();
                s.play(played.as_str(), json!({}));
                // Played out of the hand, it is no longer watched or marked.
                assert_eq!(watched(&s), vec![theirs[1].clone()]);
                assert_eq!(s.view(P1).opponent.hand_marked, Some(1));
                until_active(&mut s, P1);
                assert!(ids(&s.hand(P1)).contains(&theirs[1]));
                assert!(!ids(&s.hand(P1)).contains(&played));
                assert_eq!(s.card(played.as_str()).controller, P2);
                s.expect_in_zone(played.as_str(), "field");
            }

            #[test]
            fn r1140_a_marked_card_that_leaves_and_comes_back_is_a_new_stay() {
                let mut s = casting("do-or-die-back", false, json!([VANILLA]));
                let theirs = ids(&s.hand(P2));
                s.play(ID, json!({}));
                assert_eq!(watched(&s), theirs);
                // Out of the hand and back again: the one watched card is gone, so is the steal.
                let mut card = s.card(theirs[0].as_str()).clone();
                let _ = move_to_zone(s.state_mut(), &mut card, OffFieldZone::Graveyard, Default::default());
                let _ = move_to_zone(s.state_mut(), &mut card, OffFieldZone::Hand, Default::default());
                assert!(s.state().delayed.is_empty());
                assert_eq!(s.view(P1).opponent.hand_marked, None);
                until_active(&mut s, P1);
                assert_eq!(s.card(theirs[0].as_str()).owner, P2);
            }

            #[test]
            fn r1141_owner_sees_marks_caster_only_the_count() {
                let mut s = casting("do-or-die-seen", false, json!([VANILLA, FILLER, VANILLA]));
                let theirs = ids(&s.hand(P2));
                s.play(ID, json!({}));
                let picked = watched(&s);
                // p1 reads a count, never which.
                let mine = s.view(P1);
                assert_eq!(mine.opponent.hand_marked, Some(2));
                assert_eq!(mine.opponent.hand, HandView::Count { count: 3 });
                let marked_seen: Vec<Value> = mine
                    .events
                    .iter()
                    .filter(|event| event.event_type() == GameEventType::Marked)
                    .map(crate::js)
                    .collect();
                assert_eq!(marked_seen.len(), 2);
                for event in &marked_seen {
                    assert_eq!(event["instanceId"], json!(HIDDEN_ID));
                    assert!(!theirs.iter().any(|id| event.to_string().contains(&format!("\"{id}\""))));
                }
                // p2 reads the marks on its own cards, and the events that made them.
                let theirs_view = s.view(P2);
                assert_eq!(theirs_view.opponent.hand_marked, None);
                assert_eq!(theirs_view.you.hand_marked, None);
                let named: Vec<Value> = theirs_view
                    .events
                    .iter()
                    .filter(|event| event.event_type() == GameEventType::Marked)
                    .map(|event| crate::js(event)["instanceId"].clone())
                    .collect();
                assert_eq!(named, picked.iter().map(|id| json!(id)).collect::<Vec<Value>>());
            }

            #[test]
            fn r4_your_cap_burns_the_overflow() {
                // Nine cards behind Do or Die: the first stolen card fills the hand to 10, the second burns.
                let mut hand = vec![json!(ID)];
                hand.extend((0..9).map(|_| json!(FILLER)));
                let mut s = crate::scenario(json!({
                    "seed": "do-or-die-cap",
                    "p1": { "hand": hand, "library": library() },
                    "p2": { "hand": [VANILLA, FILLER], "library": library() },
                }));
                let theirs = ids(&s.hand(P2));
                s.play(ID, json!({}));
                until_active(&mut s, P1);
                let mine = ids(&s.hand(P1));
                assert_eq!(mine.len() as i32, HAND_CAP);
                assert!(mine.contains(&theirs[0]));
                // The second burned into p1's graveyard, as p1's card (§2.4).
                assert!(ids(&s.pile(P1, "graveyard")).contains(&theirs[1]));
                assert_eq!(s.card(theirs[1].as_str()).owner, P1);
            }

            #[test]
            fn r1145_a_taken_unit_token_card_lives_on() {
                let mut s = casting("do-or-die-token", false, json!([RUSH_TOKEN, FILLER]));
                let token = s.hand(P2)[0].clone();
                s.play(ID, json!({}));
                until_active(&mut s, P1);
                // A hand to a hand: it stays in a hand, so it does not cease to exist (R11).
                assert!(ids(&s.hand(P1)).contains(&token.id));
                assert_eq!(s.card(&token).owner, P1);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn radiant_marks_every_card_not_later_draws() {
                let mut s = casting("do-or-die-radiant", true, json!([VANILLA, FILLER, VANILLA]));
                let theirs = ids(&s.hand(P2));
                s.play(ID, json!({}));
                assert_eq!(watched(&s), theirs);
                assert_eq!(s.view(P1).opponent.hand_marked, Some(3));
                until_active(&mut s, P2);
                // p2's draw is a new card, never marked.
                assert_eq!(s.view(P1).opponent.hand_marked, Some(3));
                assert_eq!(s.hand(P2).len(), 4);
                until_active(&mut s, P1);
                let mine = ids(&s.hand(P1));
                assert!(theirs.iter().all(|id| mine.contains(id)));
                // The card p2 drew stays with p2.
                assert_eq!(s.hand(P2).len(), 1);
                assert!(!theirs.contains(&s.hand(P2)[0].id));
            }

            #[test]
            fn r1140_an_empty_hand_schedules_nothing_on_the_radiant_face_too() {
                let mut s = casting("do-or-die-radiant-empty", true, json!([]));
                s.play(ID, json!({}));
                assert!(s.state().delayed.is_empty());
            }
        }
    }
}
