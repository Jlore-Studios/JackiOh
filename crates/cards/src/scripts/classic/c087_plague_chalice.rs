//! C #87 Plague Chalice (SPEC §8.6 row 87). (X) Field Spell, Epic.
//!   Base:    "This enters with X Plague Counters on it.
//!             Aura: Counter every card played whose cost equals the number of Plague Counters on this."
//!   Radiant: "… Counter every card your opponent plays whose cost equals the number of Plague Counters on this."
//!   Engine:  "X is at least 1 (R348), and on the field it costs the X it was played for (R396). Counter
//!            (§6.3), in §10.5's announce window, on every announce whose cost paid equals the current count
//!            (both players'; Radiant: the opponent's). "Cost" is the cost paid, as #60 Bear Honeypot reads it
//!            (R56), so a free cast (R70) is countered only at a count of 0. … It is not on the field during
//!            its own announce, so it never counters itself. Tunes: none."
//!
//! "Enters with X" is one placement of X on itself as its Cry (`placePlague`). The Aura answers each
//! `cardAnnounced` whose `costPaid` equals the tokens on it now — the count moves as tokens are placed and
//! removed (C #78) — by countering that play to its owner's graveyard (`counterPlay`).
//!
//! Patch v0.2.7 (#126, R667): `wouldCounter` is the same match asked ahead of any play, so the engine
//! can warn the viewer off a hand card the Chalice would counter (`counteredOnPlay`). Both halves call
//! `counters`, so the warning and the counter cannot disagree.

use jackioh_engine::effects::{counter_play, place_plague};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-087";

/// The match: a play `player` makes paying `cost_paid`, against this Chalice's count and its face's reach.
fn counters(self_: &CardInstance, controller: PlayerId, player: PlayerId, cost_paid: i32, opponent_only: bool) -> bool {
    if opponent_only && player == controller {
        return false;
    }
    cost_paid == plague_on(self_)
}

/// The announced play's instance id when the Chalice, as it stands now, counters it. TS read the live
/// `ctx.self`, so its count is the one on the card at this moment (`live_self`).
fn matches(ctx: &EffectContext<'_>, event: &GameEvent, opponent_only: bool) -> Option<String> {
    let GameEvent::CardAnnounced { player, instance_id, cost_paid, .. } = event else {
        return None;
    };
    let me = ctx.live_self()?;
    if counters(me, ctx.controller, *player, *cost_paid, opponent_only) {
        Some(instance_id.clone())
    } else {
        None
    }
}

fn chalice(opponent_only: bool) -> Script {
    let aura = TriggerDef::new("plague-chalice", &[GameEventType::CardAnnounced], move |ctx, event| {
        match matches(ctx, event, opponent_only) {
            None => vec![],
            Some(instance_id) => vec![counter_play(json_as(json!({
                "target": { "of": "instance", "instanceId": instance_id }
            })))],
        }
    })
    .with_when(move |ctx, event| matches(ctx, event, opponent_only).is_some());
    Script {
        cry: Some(hook(|ctx| {
            vec![place_plague(json_as(json!({ "target": { "of": "self" }, "amount": ctx.x })))]
        })),
        triggers: vec![aura],
        would_counter: Some(would_counter_hook(move |args| {
            counters(args.self_, args.controller, args.player, args.cost_paid, opponent_only)
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: chalice(false),
        radiant: chalice(true),
    }
}

// C #87 Plague Chalice — SPEC §8.6 row 87, BUILD M9 Classic row C 87: "X chosen with the play, at least 1
// (R348); it enters with X Plague Counters; Aura: every card either player plays whose cost paid equals its
// current token count is countered in the announce window (§10.5), treated as never played as C #17's
// is; a free cast is countered only at a count of 0 (R70); the count moves (C #78 removes tokens,
// placements add them); it isn't on the field during its own announce and never counters itself; a set
// trap's cost is public (R351), so countering it reveals only what the graveyard then shows; leaving the
// field ends it; radiant: only the opponent's plays; no tuned numbers".
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;
    use std::collections::BTreeSet;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const CHALICE: &str = "classic-087";
    const BRINGER: &str = "classic-076"; // (2) Unit: Rush; Cry: place 2 Plague Counters (two placements). Draw 1.
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt.
    const MUTATE: &str = "classic-078"; // (1) Field Spell: Activate ♾️: remove a Plague Counter from a permanent …
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const POINTMASTER: &str = "core-020"; // (2) Unit 7/1 First Strike.
    const HINDER: &str = "core-021"; // (0) Spell: Cast on draw: your opponent has 1 less mana next turn. Discard 1.
    const PAWN: &str = "core-096"; // (1) Trap: answers only an attack that would be lethal.
    const COLLATERAL: &str = "core-034"; // (4) Spell: Exile target permanent and a random card from your opponent's deck.
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).
    const FILLER: &str = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
    const X: &str = "core-019"; // library filler.

    use crate::scenario;

    fn def() -> CardDef {
        crate::register_all();
        crate::card_def(ID)
    }

    use crate::js;

    fn must<T>(value: Option<T>, what: &str) -> T {
        match value {
            Some(found) => found,
            None => panic!("missing: {what}"),
        }
    }

    fn lib(n: usize) -> Vec<&'static str> {
        vec![X; n]
    }

    fn library_then(first: &str, n: usize) -> Vec<&str> {
        let mut library = vec![first];
        library.extend(lib(n));
        library
    }

    fn countered(events: &[GameEvent]) -> Vec<String> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Countered { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn in_hand(s: &Scenario, player: PlayerId, def_id: &str) -> CardInstance {
        must(s.hand(player).into_iter().find(|card| card.def_id == def_id), def_id)
    }

    /// A Chalice standing with `tokens` on it (a setup Chalice arrived with no X: R396's 0).
    fn standing(tokens: i32, radiant_face: bool, active: PlayerId) -> Scenario {
        let counters = if tokens > 0 { json!({ "plague": tokens }) } else { json!({}) };
        scenario(json!({
            "active": active,
            "p1": {
                "hand": [VANILLA, POINTMASTER, ANCHOR, FILLER],
                "backrow": [{ "def": CHALICE, "radiant": radiant_face, "counters": counters }],
                "library": lib(4)
            },
            "p2": { "hand": [VANILLA, POINTMASTER, ANCHOR, FILLER], "library": lib(4) }
        }))
    }

    /// is an X-cost Field Spell with no declared numbers; base counters both players, Radiant the opponent's
    #[test]
    fn is_an_x_cost_field_spell_with_no_declared_numbers_base_counters_both_players_radiant_the_opponent_s() {
        assert_eq!(def().id, CHALICE);
        assert_eq!(js(&def().cost), json!("X"));
        assert!(def().params.is_none());
        let scripts = script();
        for face in [&scripts.base, &scripts.radiant] {
            let on: Vec<Vec<&str>> =
                face.triggers.iter().map(|trigger| trigger.on.iter().map(|kind| kind.as_str()).collect()).collect();
            assert_eq!(on, vec![vec!["cardAnnounced"]]);
        }
    }

    /// base
    mod base {
        use super::*;

        /// R348 X is chosen with the play, at least 1 and at most your mana
        #[test]
        fn r348_x_is_chosen_with_the_play_at_least_1_and_at_most_your_mana() {
            let mut s = scenario(json!({ "p1": { "hand": [CHALICE, ANCHOR] }, "p2": { "hand": [ANCHOR] } }));
            let chalice = s.card(CHALICE).clone();
            let xs: BTreeSet<Option<i64>> = legal_actions(s.state(), P1)
                .iter()
                .map(js)
                .filter(|action| action["type"] == json!("play") && action["instanceId"] == json!(chalice.id))
                .map(|play| play["x"].as_i64())
                .collect();

            assert_eq!(xs, BTreeSet::from([Some(1), Some(2), Some(3), Some(4)]));
            s.expect_refused_with(|s| s.play(&chalice, json!({ "x": 0 })), "at least 1");
            s.expect_refused(|s| s.play(&chalice, json!({})));
        }

        /// it enters with X Plague Counters on it, one placement of X
        #[test]
        fn it_enters_with_x_plague_counters_on_it_one_placement_of_x() {
            let mut s = scenario(json!({ "p1": { "hand": [CHALICE, ANCHOR] }, "p2": { "hand": [ANCHOR] } }));

            s.play(CHALICE, json!({ "x": 3 }));

            let chalice = s.card(CHALICE).clone();
            assert_eq!(chalice.counters.plague, Some(3));
            let changes: Vec<Value> = s
                .events()
                .iter()
                .filter(|event| matches!(event, GameEvent::CounterChanged { .. }))
                .map(js)
                .collect();
            assert_eq!(
                json!(changes),
                json!([{ "type": "counterChanged", "instanceId": chalice.id, "counter": "plague", "value": 3, "placed": 3 }])
            );
            s.expect_mana(P1, 1);
        }

        /// it never counters itself: it is not on the field during its own announce
        #[test]
        fn it_never_counters_itself_it_is_not_on_the_field_during_its_own_announce() {
            let mut s = scenario(json!({ "p1": { "hand": [CHALICE, ANCHOR] }, "p2": { "hand": [ANCHOR] } }));

            s.play(CHALICE, json!({ "x": 1 }));

            assert!(countered(s.events()).is_empty());
            s.expect_in_zone(CHALICE, "field");
        }

        /// §10.5 your own play whose cost paid equals the count is countered, treated as never played
        #[test]
        fn s10_5_your_own_play_whose_cost_paid_equals_the_count_is_countered_treated_as_never_played() {
            let mut s = standing(1, false, P1);
            let vanilla = s.card(VANILLA).clone();
            let played_before = s.state().players.p1.turn_log.cards_played;

            s.play(&vanilla, json!({}));

            assert_eq!(countered(s.events()), vec![vanilla.id.clone()]);
            s.expect_in_zone(&vanilla, "graveyard");
            assert!(!s.events().iter().any(
                |event| matches!(event, GameEvent::CardPlayed { instance_id, .. } if *instance_id == vanilla.id)
            ));
            assert_eq!(s.state().players.p1.turn_log.cards_played, played_before);
            // The mana paid stays spent.
            s.expect_mana(P1, 3);
        }

        /// the opponent's play at the count is countered too, and a play of another cost resolves
        #[test]
        fn the_opponent_s_play_at_the_count_is_countered_too_and_a_play_of_another_cost_resolves() {
            let mut s = standing(1, false, P2);
            let their_vanilla = in_hand(&s, P2, VANILLA);
            let their_pointmaster = in_hand(&s, P2, POINTMASTER);

            s.play(&their_vanilla, json!({}));
            s.play(&their_pointmaster, json!({}));

            assert_eq!(countered(s.events()), vec![their_vanilla.id.clone()]);
            s.expect_in_zone(&their_pointmaster, "field");
        }

        /// a second Chalice played for the same X is countered by the first
        #[test]
        fn a_second_chalice_played_for_the_same_x_is_countered_by_the_first() {
            let mut s = scenario(json!({ "p1": { "hand": [CHALICE, CHALICE, ANCHOR], "mana": 10 }, "p2": { "hand": [ANCHOR] } }));
            let chalices: Vec<CardInstance> = s.hand(P1).into_iter().filter(|card| card.def_id == CHALICE).collect();
            let (Some(first), Some(second)) = (chalices.first().cloned(), chalices.get(1).cloned()) else {
                panic!("two Chalices in hand");
            };

            s.play(&first, json!({ "x": 2 }));
            s.play(&second, json!({ "x": 2 }));

            assert_eq!(countered(s.events()), vec![second.id.clone()]);
            s.expect_in_zone(&second, "graveyard");
        }

        /// R70 a free cast is countered only at a count of 0: a cast-on-draw Hinder at 1 resolves
        #[test]
        fn r70_a_free_cast_is_countered_only_at_a_count_of_0_a_cast_on_draw_hinder_at_1_resolves() {
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "backrow": [{ "def": CHALICE, "counters": { "plague": 1 } }], "library": lib(3) },
                "p2": { "hand": [ANCHOR, FILLER], "library": library_then(HINDER, 3) }
            }));

            s.end_turn(); // p2 draws Hinder and casts it, paying 0; its discard is random (R682), no prompt
            assert!(s.state().pending.is_none());

            assert!(countered(s.events()).is_empty());
            assert!(s.events().iter().any(|event| matches!(event, GameEvent::Discarded { .. })));
            assert!(s.events().iter().any(
                |event| matches!(event, GameEvent::CardPlayed { def_id, .. } if def_id.as_str() == HINDER)
            ));
        }

        /// R70 at a count of 0 a free cast is countered: Hinder cast on draw does nothing
        #[test]
        fn r70_at_a_count_of_0_a_free_cast_is_countered_hinder_cast_on_draw_does_nothing() {
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "backrow": [CHALICE], "library": lib(3) },
                "p2": { "hand": [ANCHOR, FILLER], "library": library_then(HINDER, 3) }
            }));
            let hinder = must(s.pile(P2, "library").into_iter().next(), "Hinder on top");

            s.end_turn(); // Countered as announced: no prompt ever opens (R682 asks nothing either).

            assert_eq!(countered(s.events()), vec![hinder.id.clone()]);
            assert!(s.state().pending.is_none());
            s.expect_in_zone(&hinder, "graveyard");
            // Countered before it resolved: nothing was discarded and no refresh rider was set.
            assert!(!s.events().iter().any(|event| matches!(event, GameEvent::Discarded { .. })));
            assert!(!s.events().iter().any(
                |event| matches!(event, GameEvent::CardPlayed { instance_id, .. } if *instance_id == hinder.id)
            ));
        }

        /// at a count of 0 a (0) Cost play is countered too
        #[test]
        fn at_a_count_of_0_a_0_cost_play_is_countered_too() {
            let mut s = standing(0, false, P1);
            let anchor = s.card(ANCHOR).clone();

            s.play(&anchor, json!({}));

            assert_eq!(countered(s.events()), vec![anchor.id.clone()]);
        }

        /// the count moves up: a Plague Bringer's two placements make it 3, and a (3) Cost play is then countered
        #[test]
        fn the_count_moves_up_a_plague_bringer_s_two_placements_make_it_3_and_a_3_cost_play_is_then_countered() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [BRINGER, MENACE, ANCHOR],
                    "backrow": [{ "def": CHALICE, "counters": { "plague": 1 } }],
                    "library": lib(2),
                    "mana": 10
                },
                "p2": { "hand": [ANCHOR] }
            }));
            let chalice = s.card(CHALICE).clone();
            let menace = in_hand(&s, P1, MENACE);

            // The (2) Bringer resolves at a count of 1, and both its placements go on the Chalice: one
            // prompt names the single target for the whole effect (R689).
            s.play(BRINGER, json!({}));
            s.answer(json!(chalice.id));
            assert_eq!(s.card(&chalice).counters.plague, Some(3));
            s.play(&menace, json!({}));

            assert_eq!(countered(s.events()), vec![menace.id.clone()]);
        }

        /// the count moves: C #78 Mutate Spell removes a token, and a (1) Cost play then resolves while a (0) is countered
        #[test]
        fn the_count_moves_c_n78_mutate_spell_removes_a_token_and_a_1_cost_play_then_resolves_while_a_0_is_countered() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [VANILLA, ANCHOR, FILLER],
                    "backrow": [{ "def": CHALICE, "counters": { "plague": 1 } }, { "def": MUTATE, "lane": 2 }],
                    "library": lib(4)
                },
                "p2": { "hand": [ANCHOR], "health": 30 }
            }));
            let vanilla = s.card(VANILLA).clone();
            let anchor = s.card(ANCHOR).clone();

            let chalice = s.card(CHALICE).id.clone();
            s.activate(MUTATE, json!({ "targets": [{ "pick": "instance", "instanceId": chalice }] }));
            assert_eq!(s.card(CHALICE).counters.plague, None);
            s.play(&vanilla, json!({}));
            s.play(&anchor, json!({}));

            s.expect_in_zone(&vanilla, "field");
            assert_eq!(countered(s.events()), vec![anchor.id.clone()]);
        }

        /// R351 a set trap's cost is public: one set at the count is countered into the graveyard, which then shows it
        #[test]
        fn r351_a_set_trap_s_cost_is_public_one_set_at_the_count_is_countered_into_the_graveyard_which_then_shows_it() {
            let mut t = scenario(json!({
                "active": "p2",
                "p1": { "hand": [ANCHOR], "backrow": [{ "def": CHALICE, "counters": { "plague": 1 } }] },
                "p2": { "hand": [PAWN, ANCHOR] }
            }));
            let trap = t.card(PAWN).clone();

            t.play(&trap, json!({}));

            assert_eq!(countered(t.events()), vec![trap.id.clone()]);
            let graveyard: Vec<String> = t.pile(P2, "graveyard").into_iter().map(|card| card.def_id).collect();
            assert_eq!(graveyard, [PAWN]);
            // p1 reads it only now that the graveyard shows it.
            let opponent = js(&t.view(P1))["opponent"].clone();
            assert!(serde_json::to_string(&opponent).expect("serialisable").contains(PAWN));
        }

        /// leaving the field ends it: exiled, it counters nothing more
        #[test]
        fn leaving_the_field_ends_it_exiled_it_counters_nothing_more() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [ANCHOR], "backrow": [{ "def": CHALICE, "counters": { "plague": 1 } }], "library": lib(2) },
                "p2": { "hand": [COLLATERAL, VANILLA, ANCHOR], "library": lib(2), "mana": 10 }
            }));

            let chalice = s.card(CHALICE).id.clone();
            s.play(COLLATERAL, json!({ "targets": [{ "pick": "instance", "instanceId": chalice }] }));
            s.play(VANILLA, json!({}));

            assert!(countered(s.events()).is_empty());
            s.expect_in_zone(VANILLA, "field");
        }
    }

    /// radiant
    mod radiant {
        use super::*;

        /// counters the opponent's play at the count
        #[test]
        fn counters_the_opponent_s_play_at_the_count() {
            let mut s = standing(1, true, P2);
            let their_vanilla = in_hand(&s, P2, VANILLA);

            s.play(&their_vanilla, json!({}));

            assert_eq!(countered(s.events()), vec![their_vanilla.id.clone()]);
        }

        /// lets your own play at the count resolve
        #[test]
        fn lets_your_own_play_at_the_count_resolve() {
            let mut s = standing(1, true, P1);
            let vanilla = s.card(VANILLA).clone();

            s.play(&vanilla, json!({}));

            assert!(countered(s.events()).is_empty());
            s.expect_in_zone(&vanilla, "field");
        }

        /// it enters with X tokens too
        #[test]
        fn it_enters_with_x_tokens_too() {
            let mut s = scenario(json!({ "p1": { "hand": [{ "def": CHALICE, "radiant": true }, ANCHOR] }, "p2": { "hand": [ANCHOR] } }));
            s.play(CHALICE, json!({ "x": 2 }));
            assert_eq!(s.card(CHALICE).counters.plague, Some(2));
        }
    }

    /// C #87 Plague Chalice: R667 the warning on the viewer's hand (patch v0.2.7)
    mod r667_the_warning_on_the_viewer_s_hand_patch_v0_2_7 {
        use super::*;

        /// The hand cards `player`'s own view marks `counteredOnPlay`, by definition.
        fn warned(s: &Scenario, player: PlayerId) -> Vec<String> {
            let view = js(&s.view(player));
            match view["you"]["hand"].as_array() {
                Some(hand) => hand
                    .iter()
                    .filter(|card| card["counteredOnPlay"] == json!(true))
                    .filter_map(|card| card["defId"].as_str().map(String::from))
                    .collect(),
                None => vec![],
            }
        }

        /// R667 base: each player's own hand card whose cost equals the count is marked, and only those
        #[test]
        fn r667_base_each_player_s_own_hand_card_whose_cost_equals_the_count_is_marked_and_only_those() {
            let s = standing(2, false, P1);
            // VANILLA (1), POINTMASTER (2), ANCHOR (0), FILLER (1): only the (2) meets a count of 2, on both seats.
            assert_eq!(warned(&s, P1), [POINTMASTER]);
            assert_eq!(warned(&s, P2), [POINTMASTER]);
            assert_eq!(warned(&standing(0, false, P1), P1), [ANCHOR]);
            assert!(warned(&standing(5, false, P1), P1).is_empty());
        }

        /// R667 R97 the mark rides only the viewer's own hand: the opponent's is a count
        #[test]
        fn r667_r97_the_mark_rides_only_the_viewer_s_own_hand_the_opponent_s_is_a_count() {
            let s = standing(2, false, P1);
            let view = js(&s.view(P1));
            assert_eq!(view["opponent"]["hand"], json!({ "count": 4 }));
            for card in view["you"]["hand"].as_array().cloned().unwrap_or_default() {
                assert_eq!(card["counteredOnPlay"] == json!(true), card["defId"] == json!(POINTMASTER));
            }
        }

        /// R667 radiant: only its controller's opponent is warned
        #[test]
        fn r667_radiant_only_its_controller_s_opponent_is_warned() {
            let s = standing(2, true, P1);
            assert!(warned(&s, P1).is_empty());
            assert_eq!(warned(&s, P2), [POINTMASTER]);
        }

        /// R667 the warning and the counter agree: the marked card is countered, an unmarked one resolves
        #[test]
        fn r667_the_warning_and_the_counter_agree_the_marked_card_is_countered_an_unmarked_one_resolves() {
            let mut s = standing(1, false, P1);
            assert_eq!(warned(&s, P1), [VANILLA, FILLER]);
            let vanilla = s.card(VANILLA).clone();
            s.play(&vanilla, json!({}));
            assert_eq!(countered(s.last_events()), vec![vanilla.id.clone()]);

            let mut t = standing(1, false, P1);
            assert!(!warned(&t, P1).contains(&POINTMASTER.to_string()));
            let pointmaster = t.card(POINTMASTER).clone();
            t.play(&pointmaster, json!({}));
            assert!(countered(t.last_events()).is_empty());
        }

        /// R667 the mark moves with the count: a token removed takes it off the (2) and puts it on the (1)s
        #[test]
        fn r667_the_mark_moves_with_the_count_a_token_removed_takes_it_off_the_2_and_puts_it_on_the_1_s() {
            let mut s = standing(2, false, P1);
            assert_eq!(warned(&s, P1), [POINTMASTER]);
            let chalice = s.card(CHALICE).id.clone();
            must(find_instance_mut(s.state_mut(), &chalice), "the Chalice").counters.plague = Some(1);
            assert_eq!(warned(&s, P1), [VANILLA, FILLER]);
        }

        /// R667 an X card is marked only when every X it could be played for is countered
        #[test]
        fn r667_an_x_card_is_marked_only_when_every_x_it_could_be_played_for_is_countered() {
            // A second Chalice in hand: X runs 1 to the mana, so one countered X leaves the others to play.
            let mut s = scenario(json!({
                "p1": { "hand": [CHALICE], "backrow": [{ "def": CHALICE, "counters": { "plague": 1 } }], "mana": 3 },
                "p2": { "hand": [ANCHOR] }
            }));
            assert!(warned(&s, P1).is_empty());
            // With 1 mana its only X is 1, which the count meets.
            s.state_mut().players.p1.mana.current = 1;
            assert_eq!(warned(&s, P1), [CHALICE]);
        }

        /// R667 leaving the field ends the warning: exiled, it marks nothing more
        #[test]
        fn r667_leaving_the_field_ends_the_warning_exiled_it_marks_nothing_more() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [ANCHOR], "backrow": [{ "def": CHALICE, "counters": { "plague": 1 } }], "library": lib(2) },
                "p2": { "hand": [COLLATERAL, VANILLA, ANCHOR], "library": lib(2), "mana": 10 }
            }));
            assert_eq!(warned(&s, P2), [VANILLA]);

            let chalice = s.card(CHALICE).id.clone();
            s.play(COLLATERAL, json!({ "targets": [{ "pick": "instance", "instanceId": chalice }] }));

            assert!(warned(&s, P2).is_empty());
        }

        /// R667 wouldCounter is the trigger's own match, on both faces
        #[test]
        fn r667_wouldcounter_is_the_trigger_s_own_match_on_both_faces() {
            let s = standing(3, false, P1);
            let self_card = s.card(CHALICE).clone();
            let scripts = script();
            let ask = |face: &Script, player: PlayerId, cost_paid: i32| -> Option<bool> {
                face.would_counter.as_ref().map(|would_counter| {
                    would_counter(WouldCounterArgs {
                        state: s.state(),
                        self_: &self_card,
                        controller: P1,
                        player,
                        cost_paid,
                    })
                })
            };
            assert_eq!(
                [ask(&scripts.base, P1, 3), ask(&scripts.base, P2, 3), ask(&scripts.base, P1, 2)],
                [Some(true), Some(true), Some(false)]
            );
            assert_eq!(
                [ask(&scripts.radiant, P1, 3), ask(&scripts.radiant, P2, 3), ask(&scripts.radiant, P2, 2)],
                [Some(false), Some(true), Some(false)]
            );
        }
    }
}
