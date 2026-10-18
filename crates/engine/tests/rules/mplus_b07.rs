// Meditative+ batch 07 (issue #1185): the engine systems M+ #1.1 Base Camp, #21 Harmony Spirit and #24
// Bob are written with, each through a fixture script (`fixtures/mplus_b07.rs`) before a card uses it:
// R1185001's play count raised (`raise_times_played`), R1185002's set stats (`set_stats_all`) and the
// `chinese` rider on `fill_board`, R1185003's card on the field taken into a hand (`take_to_hand`,
// `ownership::take_into_hand`), and R1185004's draw that fills the hand with copies (`draw_priced`'s
// `fillWithCopies`).

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{big_body, plain, stacker};
use crate::rules::fixtures::mplus_b07::{
    RAISE_BY, SET_TO, banner, camp, ghoul, harmony, kidnapper, party, plain_spell, register_b07_fixtures,
    snatch, spark, stoic, summit, xerox,
};

/// `p1`'s turn 9, its main phase; each side's setup as given, with plain cards to draw so no turn ends
/// by itself (§2.5).
fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    register_b07_fixtures();
    let mut p1 = p1;
    let mut p2 = p2;
    for side in [&mut p1, &mut p2] {
        if side.get("library").is_none() {
            side["library"] = json!([plain.id, plain.id, plain.id, plain.id]);
        }
    }
    scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }))
}

fn id_at(s: &Scenario, player: PlayerId, lane: i32) -> String {
    s.unit(player, lane)
        .unwrap_or_else(|| panic!("no unit at {player:?}'s lane {lane}"))
        .id
}

fn pick(id: &str) -> Value {
    json!({ "targets": [{ "pick": "instance", "instanceId": id }] })
}

fn stats_of(s: &Scenario, id: &str) -> (i32, i32, i32) {
    let view = s.stats(id);
    (view.attack, view.max_health, view.health)
}

/// The `numberChanged` events of the last step, as (instance, key, value).
fn numbers_changed(events: &[GameEvent]) -> Vec<(String, String, i32)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::NumberChanged {
                instance_id,
                key,
                value,
                ..
            } => Some((instance_id.clone(), key.clone(), *value)),
            _ => None,
        })
        .collect()
}

fn types_of(events: &[GameEvent]) -> Vec<GameEventType> {
    events.iter().map(GameEvent::event_type).collect()
}

mod r1185002_set_stats_sets_attack_and_health_outright_its_damage_and_buffs_gone {
    use super::*;

    #[test]
    fn r1185002_every_other_unit_of_both_sides_is_set_to_3_3_undamaged_its_buffs_and_tuning_gone() {
        let mut s = board(
            "r1185002-set",
            json!({ "hand": [harmony.id, plain.id], "field": [{ "def": big_body.id, "lane": 1, "damage": 5 }] }),
            json!({ "hand": [plain.id], "field": [{ "def": plain.id, "lane": 2 }] }),
        );
        let mine = id_at(&s, P1, 1);
        let theirs = id_at(&s, P2, 2);
        s.card_mut(mine.as_str()).buffs = AttackHealth { attack: 2, health: 2 };
        s.card_mut(theirs.as_str()).tuning = Some(Tuning {
            attack: Some(4),
            health: Some(1),
            ..Tuning::default()
        });
        assert_eq!(stats_of(&s, &mine), (7, 12, 7));
        assert_eq!(stats_of(&s, &theirs), (7, 4, 4));

        s.play(harmony.id.as_str(), json!({ "zone": 3 }));
        assert_eq!(
            stats_of(&s, &mine),
            (SET_TO, SET_TO, SET_TO),
            "5 damage healed, +2/+2 gone"
        );
        assert_eq!(
            stats_of(&s, &theirs),
            (SET_TO, SET_TO, SET_TO),
            "the tuning's stats gone"
        );
        assert_eq!(s.card(mine.as_str()).damage, 0);
        assert_eq!(s.card(theirs.as_str()).tuning, None);
        // Harmony itself is not "another" Unit.
        let spirit = id_at(&s, P1, 3);
        assert_eq!(stats_of(&s, &spirit), (2, 4, 4));
        // Each Unit set reports its two numbers, as it wears them now; the active side first (R68).
        assert_eq!(
            numbers_changed(s.last_events()),
            vec![
                (mine.clone(), "attack".to_string(), SET_TO),
                (mine.clone(), "health".to_string(), SET_TO),
                (theirs.clone(), "attack".to_string(), SET_TO),
                (theirs.clone(), "health".to_string(), SET_TO),
            ]
        );
    }

    #[test]
    fn r1185002_an_aura_still_applies_on_top_and_an_immutable_unit_is_untouched() {
        let mut s = board(
            "r1185002-aura",
            json!({
                "hand": [harmony.id, plain.id],
                "field": [{ "def": banner.id, "lane": 1 }, { "def": stoic.id, "lane": 2 }, { "def": big_body.id, "lane": 3 }],
            }),
            json!({ "hand": [plain.id], "field": [{ "def": plain.id, "lane": 1 }] }),
        );
        let flag = id_at(&s, P1, 1);
        let immutable = id_at(&s, P1, 2);
        let body = id_at(&s, P1, 3);
        s.play(harmony.id.as_str(), json!({ "zone": 4 }));
        // The banner is set too, and its aura lifts the others it reaches after the set (§10.4).
        assert_eq!(stats_of(&s, &flag), (SET_TO, SET_TO, SET_TO));
        assert_eq!(stats_of(&s, &body), (SET_TO + 1, SET_TO + 1, SET_TO + 1));
        assert_eq!(
            stats_of(&s, &immutable),
            (6, 6, 6),
            "R23: Immutable, it keeps its 5/5 (+1/+1)"
        );
        assert_eq!(s.card(immutable.as_str()).stats_override, None);
        assert_eq!(
            stats_of(&s, &id_at(&s, P1, 4)),
            (3, 5, 5),
            "the spirit has the banner's +1/+1 only"
        );
    }

    #[test]
    fn r1185002_a_radiant_token_whose_face_doubles_its_x_x_is_set_to_3_3_too() {
        let mut s = board(
            "r1185002-ghoul",
            json!({ "hand": [harmony.id, plain.id] }),
            json!({
                "hand": [plain.id],
                "field": [{ "def": ghoul.id, "lane": 1, "radiant": true, "statsOverride": { "attack": 2, "health": 2 } }],
            }),
        );
        let token = id_at(&s, P2, 1);
        assert_eq!(
            stats_of(&s, &token),
            (4, 4, 4),
            "R349: a Radiant Ghoul wears its X/X doubled"
        );
        s.play(harmony.id.as_str(), json!({ "zone": 1 }));
        assert_eq!(stats_of(&s, &token), (SET_TO, SET_TO, SET_TO));
    }

    #[test]
    fn r1185002_a_dormant_card_under_a_pile_is_not_on_the_field_for_it() {
        let mut s = board(
            "r1185002-pile",
            json!({ "hand": [harmony.id, plain.id] }),
            json!({
                "hand": [plain.id],
                "field": [{ "def": big_body.id, "lane": 1 }, { "def": stacker.id, "lane": 1, "stack": true }],
            }),
        );
        let top = id_at(&s, P2, 1);
        let beneath = s.state().players.p2.units[0].as_ref().expect("the pile")[1].clone();
        s.play(harmony.id.as_str(), json!({ "zone": 1 }));
        assert_eq!(stats_of(&s, &top), (SET_TO, SET_TO, SET_TO));
        assert_eq!(
            s.card(beneath.id.as_str()).stats_override,
            None,
            "R13: dormant, untouched"
        );
    }

    #[test]
    fn r1185002_a_set_unit_taken_off_the_field_is_its_printed_card_again() {
        let mut s = board(
            "r1185002-reset",
            json!({ "hand": [harmony.id, snatch.id, plain.id] }),
            json!({ "hand": [plain.id], "field": [{ "def": big_body.id, "lane": 1 }] }),
        );
        let body = id_at(&s, P2, 1);
        s.play(harmony.id.as_str(), json!({ "zone": 1 }));
        assert_eq!(
            s.card(body.as_str()).stats_override,
            Some(AttackHealth {
                attack: SET_TO,
                health: SET_TO
            })
        );
        s.play(snatch.id.as_str(), pick(&body));
        let taken = s.card(body.as_str()).clone();
        assert_eq!(taken.zone, Zone::Hand { player: P1 });
        assert_eq!(
            taken.stats_override, None,
            "R78's reset takes the set off with the card leaving"
        );
    }

    #[test]
    fn r1185002_a_fill_in_chinese_summons_every_token_in_chinese() {
        let mut s = board(
            "r1185002-chinese",
            json!({ "hand": [party.id, plain.id], "field": [{ "def": plain.id, "lane": 1 }] }),
            json!({ "hand": [plain.id] }),
        );
        let standing = id_at(&s, P1, 1);
        s.play(party.id.as_str(), json!({}));
        for lane in 2..=5 {
            let token = s.unit(P1, lane).expect("a Rush Token");
            assert_eq!(token.def_id, "fx-token-rush");
            assert_eq!(token.chinese, Some(true), "lane {lane}");
        }
        assert_eq!(
            s.card(standing.as_str()).chinese,
            None,
            "the Unit already there is not translated"
        );
    }
}

mod r1185003_a_unit_on_the_field_is_taken_into_your_hand_as_yours {
    use super::*;

    #[test]
    fn r1185003_it_leaves_the_field_for_the_takers_hand_reset_and_owned_by_them_with_no_death() {
        let mut s = board(
            "r1185003-take",
            json!({ "hand": [kidnapper.id, plain.id] }),
            json!({ "hand": [plain.id], "field": [{ "def": big_body.id, "lane": 2, "damage": 4 }] }),
        );
        let body = id_at(&s, P2, 2);
        s.card_mut(body.as_str()).buffs = AttackHealth { attack: 1, health: 1 };
        s.play(
            kidnapper.id.as_str(),
            json!({ "zone": 1, "targets": [{ "pick": "instance", "instanceId": body }] }),
        );
        let taken = s.card(body.as_str()).clone();
        assert_eq!(taken.zone, Zone::Hand { player: P1 });
        assert_eq!((taken.owner, taken.controller), (P1, P1));
        assert_eq!(
            (taken.damage, taken.buffs),
            (0, AttackHealth { attack: 0, health: 0 })
        );
        assert!(s.unit(P2, 2).is_none());
        let events = types_of(s.last_events());
        assert!(events.contains(&GameEventType::Stolen));
        assert!(events.contains(&GameEventType::AddedToHand));
        assert!(!events.contains(&GameEventType::Destroyed), "no death");
        // It is p1's card from now on: its graveyard is p1's.
        s.play(body.as_str(), json!({ "zone": 2 }));
        assert_eq!(s.unit(P1, 2).expect("played").id, body);
    }

    #[test]
    fn r1185003_a_unit_token_taken_ceases_to_exist() {
        let mut s = board(
            "r1185003-token",
            json!({ "hand": [kidnapper.id, plain.id] }),
            json!({ "hand": [plain.id], "field": [{ "def": "fx-token-rush", "lane": 1 }] }),
        );
        let token = s.unit(P2, 1).expect("the token");
        let hand_before = s.hand(P1).len();
        s.play(
            kidnapper.id.as_str(),
            json!({ "zone": 1, "targets": [{ "pick": "instance", "instanceId": token.id }] }),
        );
        s.expect_in_zone(&token, "gone");
        assert_eq!(
            s.hand(P1).len(),
            hand_before - 1,
            "only the kidnapper left the hand"
        );
        assert!(!types_of(s.last_events()).contains(&GameEventType::AddedToHand));
        assert!(s.pile(P1, "graveyard").is_empty() && s.pile(P2, "graveyard").is_empty());
    }

    #[test]
    fn r1185003_a_full_hand_burns_it_into_the_takers_graveyard() {
        let mut s = board(
            "r1185003-full",
            json!({ "hand": [kidnapper.id, plain.id, plain.id, plain.id, plain.id, plain.id, plain.id, plain.id, plain.id, plain.id] }),
            json!({ "hand": [plain.id], "field": [{ "def": big_body.id, "lane": 1 }] }),
        );
        let extra = new_instance(s.state_mut(), plain.id.as_str(), P1, Zone::Hand { player: P1 });
        s.state_mut().players.p1.hand.push(extra);
        let body = id_at(&s, P2, 1);
        s.play(
            kidnapper.id.as_str(),
            json!({ "zone": 1, "targets": [{ "pick": "instance", "instanceId": body }] }),
        );
        let burned = s.card(body.as_str()).clone();
        assert_eq!(
            burned.zone,
            Zone::Graveyard { player: P1 },
            "§2.4: burned, and p1 owns it now"
        );
        assert!(types_of(s.last_events()).contains(&GameEventType::Burned));
    }

    #[test]
    fn r1185003_the_card_beneath_a_taken_pile_top_resumes() {
        let mut s = board(
            "r1185003-pile",
            json!({ "hand": [kidnapper.id, plain.id] }),
            json!({
                "hand": [plain.id],
                "field": [{ "def": big_body.id, "lane": 1 }, { "def": stacker.id, "lane": 1, "stack": true }],
            }),
        );
        let top = id_at(&s, P2, 1);
        s.play(
            kidnapper.id.as_str(),
            json!({ "zone": 1, "targets": [{ "pick": "instance", "instanceId": top }] }),
        );
        assert_eq!(s.card(top.as_str()).zone, Zone::Hand { player: P1 });
        assert_eq!(
            s.unit(P2, 1).expect("the card beneath").def_id,
            big_body.id,
            "R13: it resumes on top"
        );
    }

    #[test]
    fn r1185003_a_rider_prices_the_card_once_it_is_in_the_hand() {
        let mut s = board(
            "r1185003-rider",
            json!({ "hand": [snatch.id, plain.id] }),
            json!({ "hand": [plain.id], "field": [{ "def": big_body.id, "lane": 1 }] }),
        );
        let body = id_at(&s, P2, 1);
        s.play(snatch.id.as_str(), pick(&body));
        assert_eq!(s.card(body.as_str()).cost_override, Some(0));
    }
}

mod r1185001_a_play_count_raised_on_every_copy_you_own {
    use super::*;

    fn summit_entry() -> Value {
        json!(summit.id)
    }

    #[test]
    fn r1185001_every_copy_the_caster_owns_in_every_zone_gains_the_plays_and_no_other_card_does() {
        let mut s = board(
            "r1185001-raise",
            json!({
                "hand": [camp.id, summit_entry(), plain_spell.id],
                "library": [summit_entry(), plain.id, plain.id, plain.id],
                "graveyard": [summit_entry()],
                "exile": [summit_entry()],
            }),
            json!({ "hand": [summit_entry(), plain.id] }),
        );
        s.play(camp.id.as_str(), json!({}));
        let mine: Vec<CardInstance> = [
            s.hand(P1),
            s.pile(P1, "library"),
            s.pile(P1, "graveyard"),
            s.pile(P1, "exile"),
        ]
        .concat()
        .into_iter()
        .filter(|card| card.def_id == summit.id)
        .collect();
        assert_eq!(mine.len(), 4);
        for card in &mine {
            assert_eq!(times_played_of(card), RAISE_BY, "{} in {:?}", card.id, card.zone);
        }
        let theirs = s
            .hand(P2)
            .into_iter()
            .find(|card| card.def_id == summit.id)
            .expect("p2's summit");
        assert_eq!(theirs.times_played, None, "a copy p2 owns is untouched");
        let other = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == plain_spell.id)
            .expect("the plain spell");
        assert_eq!(
            other.times_played, None,
            "a card that counts no plays is passed by"
        );
        let reported: Vec<i32> = numbers_changed(s.last_events())
            .into_iter()
            .filter(|(_, key, _)| key == "timesPlayed")
            .map(|(_, _, value)| value)
            .collect();
        assert_eq!(reported, vec![RAISE_BY; 4]);
    }

    #[test]
    fn r1185001_the_count_adds_to_the_plays_it_had_and_its_next_play_counts_on_top() {
        let mut s = board(
            "r1185001-adds",
            json!({ "hand": [camp.id, summit_entry(), plain.id] }),
            json!({ "hand": [plain.id] }),
        );
        let held = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == summit.id)
            .expect("the summit");
        s.card_mut(held.id.as_str()).times_played = Some(3);
        s.play(camp.id.as_str(), json!({}));
        assert_eq!(times_played_of(s.card(held.id.as_str())), 3 + RAISE_BY);
        s.play(held.id.as_str(), json!({}));
        assert_eq!(
            times_played_of(s.card(held.id.as_str())),
            3 + RAISE_BY + 1,
            "R429: the play counts too"
        );
    }

    #[test]
    fn r1185001_the_report_is_hidden_where_the_card_is() {
        let mut s = board(
            "r1185001-hidden",
            json!({
                "hand": [camp.id, summit_entry(), plain.id],
                "library": [summit_entry(), plain.id, plain.id],
                "graveyard": [summit_entry()],
            }),
            json!({ "hand": [plain.id] }),
        );
        let in_hand = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == summit.id)
            .expect("hand")
            .id;
        let in_deck = s
            .pile(P1, "library")
            .into_iter()
            .find(|card| card.def_id == summit.id)
            .expect("deck")
            .id;
        let in_graveyard = s.pile(P1, "graveyard")[0].id.clone();
        s.play(camp.id.as_str(), json!({}));
        let seen = |viewer: PlayerId| -> Vec<String> {
            view_for(s.state(), viewer)
                .events
                .iter()
                .filter_map(|event| match event {
                    GameEvent::NumberChanged { instance_id, .. } => Some(instance_id.clone()),
                    _ => None,
                })
                .collect()
        };
        let mine = seen(P1);
        assert!(mine.contains(&in_hand) && mine.contains(&in_graveyard));
        assert!(!mine.contains(&in_deck), "R97: a deck card is nobody's to read");
        let theirs = seen(P2);
        assert!(theirs.contains(&in_graveyard));
        assert!(!theirs.contains(&in_hand) && !theirs.contains(&in_deck));
    }
}

mod r1185004_draw_1_then_fill_the_hand_with_copies_of_it {
    use super::*;

    #[test]
    fn r1185004_the_drawn_card_and_its_copies_fill_the_hand() {
        let mut s = board(
            "r1185004-fill",
            json!({ "hand": [xerox.id, plain.id, plain.id], "library": [big_body.id, plain.id, plain.id] }),
            json!({ "hand": [plain.id] }),
        );
        s.play(xerox.id.as_str(), json!({}));
        let hand = s.hand(P1);
        assert_eq!(hand.len() as i32, hand_cap_of(s.state(), P1));
        let bodies: Vec<&CardInstance> = hand.iter().filter(|card| card.def_id == big_body.id).collect();
        assert_eq!(
            bodies.len(),
            hand.len() - 2,
            "the drawn card and its copies, beside the two kept"
        );
        assert!(
            s.pile(P1, "graveyard")
                .iter()
                .all(|card| card.def_id != big_body.id),
            "nothing burned"
        );
    }

    #[test]
    fn r1185004_copies_carry_the_drawn_cards_radiant_face() {
        let mut s = board(
            "r1185004-radiant",
            json!({ "hand": [xerox.id, plain.id], "library": [{ "def": big_body.id, "radiant": true }, plain.id] }),
            json!({ "hand": [plain.id] }),
        );
        s.play(xerox.id.as_str(), json!({}));
        let bodies: Vec<CardInstance> = s
            .hand(P1)
            .into_iter()
            .filter(|card| card.def_id == big_body.id)
            .collect();
        assert_eq!(bodies.len(), 9);
        assert!(
            bodies.iter().all(|card| card.radiant),
            "R57, R1242: a copy keeps the Radiant face"
        );
    }

    #[test]
    fn r1185004_a_card_cast_on_draw_is_copied_by_nothing() {
        let mut s = board(
            "r1185004-cast",
            json!({ "hand": [xerox.id, plain.id], "library": [spark.id, big_body.id, plain.id] }),
            json!({ "hand": [plain.id] }),
        );
        let health = s.state().players.p2.hero.health;
        s.play(xerox.id.as_str(), json!({}));
        assert_eq!(
            s.state().players.p2.hero.health,
            health - 1,
            "the spark was cast (R58)"
        );
        // R596: the draw's own card never reached the hand, so nothing is copied; the card its chain
        // brought next is that repeat's, not this draw's.
        let bodies = s
            .hand(P1)
            .iter()
            .filter(|card| card.def_id == big_body.id)
            .count();
        assert!(
            bodies <= 1 && s.hand(P1).len() <= 2,
            "no copies: {:?}",
            s.hand(P1)
        );
    }

    #[test]
    fn r1185004_an_empty_deck_deals_fatigue_and_copies_nothing() {
        let mut s = board(
            "r1185004-empty",
            json!({ "hand": [xerox.id, plain.id], "library": [] }),
            json!({ "hand": [plain.id] }),
        );
        s.play(xerox.id.as_str(), json!({}));
        assert_eq!(s.hand(P1).len(), 1);
    }
}
