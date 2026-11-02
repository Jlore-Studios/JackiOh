// Issue #1848: Stinkyscape's spell verbs, each an engine primitive proved on the fixture scripts of
// `fixtures/spell_verbs.rs`. R1848000 a Freeze that never thaws; R1848001 a Spell's Trample onto
// adjacent Units (and the hero); R1848002 Poisonous for a Spell and "your next Spell this turn";
// R1848003 copies with a fixed X; R1848004 lose or gain N mana next turn; R1848005 the melee;
// R1848006 Inflation; and R1868000 (#1868) an X card's declared target read at the play's X.

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{armored, big_body, indestructible, pacifist, plain, shielded};
use crate::rules::fixtures::spell_verbs::register_spell_verb_fixtures;

/// `active`'s turn 9, its main phase; both sides hold `hand`s, the given fields, and plain cards to
/// draw, so no turn ends by itself (§2.5).
fn board(
    active: &str,
    seed: &str,
    p1_hand: Value,
    p1_field: Value,
    p2_hand: Value,
    p2_field: Value,
) -> Scenario {
    register_spell_verb_fixtures();
    scenario(json!({
        "seed": seed,
        "active": active,
        "p1": { "hand": p1_hand, "field": p1_field, "library": [plain.id, plain.id, plain.id, plain.id] },
        "p2": { "hand": p2_hand, "field": p2_field, "library": [plain.id, plain.id, plain.id, plain.id] },
    }))
}

fn id_at(s: &Scenario, player: PlayerId, lane: i32) -> String {
    s.unit(player, lane)
        .unwrap_or_else(|| panic!("no unit at {player:?}'s lane {lane}"))
        .id
}

fn pick(id: &str) -> Value {
    json!({ "targets": [{ "pick": "instance", "instanceId": id }] })
}

fn hero_health(s: &Scenario, player: PlayerId) -> i32 {
    s.state().players[player].hero.health
}

/// The `(target, amount)` of every `damage` event of the last step, in order.
fn hits(s: &Scenario) -> Vec<(String, i32)> {
    s.last_events()
        .iter()
        .filter_map(|event| match event {
            GameEvent::Damage {
                target_id, amount, ..
            } => Some((target_id.clone(), *amount)),
            _ => None,
        })
        .collect()
}

mod r1848000_freeze_forever_never_thaws {
    use super::*;
    use crate::rules::fixtures::freeze::{chill, grab, hush, recall};
    use crate::rules::fixtures::spell_verbs::absolute_zero;

    fn offers_attack_or_switch(s: &Scenario, id: &str) -> bool {
        let unit = s.card(id).clone();
        legal_actions(s.state(), unit.controller)
            .iter()
            .any(|action| match action {
                ActionBody::Attack { attacker_id, .. } => *attacker_id == unit.id,
                ActionBody::SwitchPosition { instance_id } => *instance_id == unit.id,
                _ => false,
            })
    }

    fn frozen_marks(events: &[GameEvent], id: &str) -> Vec<bool> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Marked {
                    instance_id,
                    mark,
                    added,
                    ..
                } if instance_id == id && mark == FROZEN_MARK.mark => Some(*added),
                _ => None,
            })
            .collect()
    }

    /// Whether `viewer`'s view shows the unit at `owner`'s `lane` Frozen forever.
    fn shown_forever(s: &Scenario, viewer: PlayerId, owner: PlayerId, lane: i32) -> Option<bool> {
        let view = s.view(viewer);
        let side = if owner == viewer {
            &view.you
        } else {
            &view.opponent
        };
        side.units[(lane - 1) as usize]
            .as_ref()
            .and_then(|unit| unit.frozen_forever)
    }

    #[test]
    fn r1848000_a_unit_frozen_forever_never_thaws_and_both_seats_see_it() {
        let mut s = board(
            "p1",
            "r1848000",
            json!([absolute_zero.id]),
            json!([{ "def": plain.id, "lane": 1 }]),
            json!([plain.id]),
            json!([{ "def": plain.id, "lane": 1 }]),
        );
        let unit = id_at(&s, P2, 1);
        s.play(absolute_zero.id.as_str(), pick(&unit));
        assert_eq!(s.card(&unit).frozen, Some(FROZEN_FOREVER));
        assert_eq!(
            frozen_marks(s.last_events(), &unit),
            vec![true],
            "announced as any freeze"
        );
        assert_eq!(shown_forever(&s, P1, P2, 1), Some(true));
        assert_eq!(shown_forever(&s, P2, P2, 1), Some(true));

        // Six turns on, through three of its controller's cleanups, it has not thawed.
        for _ in 0..6 {
            s.end_turn();
            assert_eq!(
                s.card(&unit).frozen,
                Some(FROZEN_FOREVER),
                "turn {}",
                s.state().turn
            );
            assert!(frozen_marks(s.last_events(), &unit).is_empty(), "no thaw");
            if s.state().active == P2 {
                assert!(!offers_attack_or_switch(&s, &unit), "turn {}", s.state().turn);
            }
        }
    }

    #[test]
    fn r1848000_an_ordinary_freeze_never_shortens_it_and_a_forever_freeze_lengthens_one() {
        let mut s = board(
            "p1",
            "r1848000-later",
            json!([chill.id, absolute_zero.id, chill.id]),
            json!([]),
            json!([plain.id]),
            json!([{ "def": plain.id, "lane": 1 }]),
        );
        let unit = id_at(&s, P2, 1);
        s.play(chill.id.as_str(), pick(&unit));
        assert_eq!(
            s.card(&unit).frozen,
            Some(10),
            "an ordinary freeze on turn 9 thaws at turn 10's cleanup"
        );
        assert_eq!(shown_forever(&s, P1, P2, 1), None);
        s.play(absolute_zero.id.as_str(), pick(&unit));
        assert_eq!(s.card(&unit).frozen, Some(FROZEN_FOREVER));
        s.play(chill.id.as_str(), pick(&unit));
        assert_eq!(
            s.card(&unit).frozen,
            Some(FROZEN_FOREVER),
            "the later thaw is kept"
        );
    }

    #[test]
    fn r1848000_a_change_of_control_keeps_it_and_a_vanilla_or_a_bounce_ends_it() {
        let mut s = board(
            "p1",
            "r1848000-ends",
            json!([absolute_zero.id, grab.id, hush.id, absolute_zero.id, recall.id]),
            json!([]),
            json!([plain.id]),
            json!([{ "def": plain.id, "lane": 1 }, { "def": plain.id, "lane": 2 }]),
        );
        let stolen = id_at(&s, P2, 1);
        s.play(absolute_zero.id.as_str(), pick(&stolen));
        s.play(grab.id.as_str(), pick(&stolen));
        assert_eq!(s.card(&stolen).controller, P1);
        assert_eq!(
            s.card(&stolen).frozen,
            Some(FROZEN_FOREVER),
            "no thaw to stamp again on the new side"
        );
        s.play(hush.id.as_str(), pick(&stolen));
        assert_eq!(s.card(&stolen).frozen, None, "a Vanilla removes it");
        assert_eq!(frozen_marks(s.last_events(), &stolen), vec![false]);

        let bounced = id_at(&s, P2, 2);
        s.play(absolute_zero.id.as_str(), pick(&bounced));
        s.play(recall.id.as_str(), pick(&bounced));
        assert_eq!(s.card(&bounced).frozen, None, "leaving the field ends it (R78)");
    }
}

mod r1848001_a_spells_trample_carries_its_excess_onto_the_adjacent_units {
    use super::*;
    use crate::rules::fixtures::spell_verbs::{adept, meteor, warded};

    /// P1 holds a Meteor (`radiant` or not) facing P2's `field`; nothing else on the board.
    fn meteor_board(seed: &str, radiant: bool, p1_field: Value, p2_field: Value) -> Scenario {
        board(
            "p1",
            seed,
            json!([{ "def": meteor.id, "radiant": radiant }]),
            p1_field,
            json!([plain.id]),
            p2_field,
        )
    }

    fn three_plains() -> Value {
        json!([
            { "def": plain.id, "lane": 1 },
            { "def": plain.id, "lane": 2 },
            { "def": plain.id, "lane": 3 },
        ])
    }

    #[test]
    fn r1848001_the_excess_goes_onto_both_neighbours_each_up_to_its_health_and_the_rest_is_lost() {
        let mut s = meteor_board("r1848001", false, json!([]), three_plains());
        let (left, target, right) = (id_at(&s, P2, 1), id_at(&s, P2, 2), id_at(&s, P2, 3));
        let before = s.state().rng_cursor;
        s.play(meteor.id.as_str(), pick(&target));
        let hits = hits(&s);
        assert_eq!(hits.len(), 3, "{hits:?}");
        assert_eq!(
            hits[0],
            (target.clone(), 3),
            "the target counts the hit up to its health (R63)"
        );
        let neighbours: Vec<&str> = hits[1..].iter().map(|(id, _)| id.as_str()).collect();
        assert!(neighbours.contains(&left.as_str()) && neighbours.contains(&right.as_str()));
        assert!(hits[1..].iter().all(|(_, amount)| *amount == 3));
        assert_eq!(
            hero_health(&s, P2),
            HERO_HEALTH,
            "the base face sends nothing to the hero"
        );
        assert_eq!(
            s.state().rng_cursor,
            before + 1,
            "one draw picks the first of two neighbours"
        );
        for id in [&left, &target, &right] {
            s.expect_in_zone(id.as_str(), "graveyard");
        }
    }

    #[test]
    fn r1848001_the_radiant_face_sends_what_passes_both_neighbours_on_to_their_hero() {
        let mut s = meteor_board("r1848001-radiant", true, json!([]), three_plains());
        let target = id_at(&s, P2, 2);
        s.play(meteor.id.as_str(), pick(&target));
        assert_eq!(hero_health(&s, P2), HERO_HEALTH - (20 - 9));
        let hero = format!("hero-{}", P2.as_str());
        assert_eq!(hits(&s).last(), Some(&(hero, 11)));
    }

    #[test]
    fn r1848001_the_first_neighbour_is_drawn_at_random_and_takes_the_excess_up_to_its_health() {
        let setup = || {
            meteor_board(
                "r1848001-order",
                false,
                json!([]),
                json!([
                    { "def": big_body.id, "lane": 1 },
                    { "def": plain.id, "lane": 2 },
                    { "def": plain.id, "lane": 3 },
                ]),
            )
        };
        let first_hit = |cursor: u32| -> Vec<(String, i32)> {
            let mut s = setup();
            s.state_mut().rng_cursor = cursor;
            let target = id_at(&s, P2, 2);
            s.play(meteor.id.as_str(), pick(&target));
            hits(&s)
        };
        let probe = setup();
        let (big, right) = (id_at(&probe, P2, 1), id_at(&probe, P2, 3));
        let seed = probe.state().seed.clone();
        let big_first = cursor_where(&seed, |cursor| first_hit(cursor)[1].0 == big);
        let right_first = cursor_where(&seed, |cursor| first_hit(cursor)[1].0 == right);
        // 12 past the target: the 5/10 takes 10 and the 3/3 the 2 left, or the 3/3 takes 3 and the
        // 5/10 the 9 left.
        assert_eq!(first_hit(big_first)[1..], [(big.clone(), 10), (right.clone(), 2)]);
        assert_eq!(first_hit(right_first)[1..], [(right, 3), (big, 9)]);
    }

    #[test]
    fn r1848001_one_neighbour_draws_nothing_and_none_sends_the_radiant_excess_to_the_hero() {
        let mut s = meteor_board(
            "r1848001-one",
            false,
            json!([]),
            json!([{ "def": plain.id, "lane": 1 }, { "def": plain.id, "lane": 2 }]),
        );
        let before = s.state().rng_cursor;
        let target = id_at(&s, P2, 1);
        s.play(meteor.id.as_str(), pick(&target));
        assert_eq!(hits(&s).len(), 2);
        assert_eq!(s.state().rng_cursor, before, "a lone neighbour is no draw");

        let mut s = meteor_board(
            "r1848001-none",
            true,
            json!([]),
            json!([{ "def": plain.id, "lane": 3 }]),
        );
        let target = id_at(&s, P2, 3);
        s.play(meteor.id.as_str(), pick(&target));
        assert_eq!(hero_health(&s, P2), HERO_HEALTH - 17);
        let mut s = meteor_board(
            "r1848001-none-base",
            false,
            json!([]),
            json!([{ "def": plain.id, "lane": 3 }]),
        );
        let target = id_at(&s, P2, 3);
        s.play(meteor.id.as_str(), pick(&target));
        assert_eq!(hits(&s).len(), 1);
        assert_eq!(hero_health(&s, P2), HERO_HEALTH);
    }

    #[test]
    fn r1848001_a_neighbour_that_takes_the_hit_whole_stops_it_and_one_immune_to_spells_is_passed_by() {
        // Divine Shield: the neighbour's shield takes the 17, and nothing reaches the hero.
        let mut s = meteor_board(
            "r1848001-shield",
            true,
            json!([]),
            json!([{ "def": plain.id, "lane": 1 }, { "def": shielded.id, "lane": 2 }]),
        );
        let (target, shield) = (id_at(&s, P2, 1), id_at(&s, P2, 2));
        s.play(meteor.id.as_str(), pick(&target));
        assert!(s.last_events().iter().any(
            |event| matches!(event, GameEvent::DivineShieldLost { instance_id } if *instance_id == shield)
        ));
        assert_eq!(hits(&s), vec![(target, 3)]);
        assert_eq!(hero_health(&s, P2), HERO_HEALTH);

        // Immune to Spells: the warded neighbour is not there for the Spell, so the other one, alone, is
        // drawn for nothing and takes the excess.
        let mut s = meteor_board(
            "r1848001-warded",
            false,
            json!([]),
            json!([
                { "def": warded.id, "lane": 1 },
                { "def": plain.id, "lane": 2 },
                { "def": plain.id, "lane": 3 },
            ]),
        );
        let before = s.state().rng_cursor;
        let (ward, target, right) = (id_at(&s, P2, 1), id_at(&s, P2, 2), id_at(&s, P2, 3));
        s.play(meteor.id.as_str(), pick(&target));
        assert_eq!(hits(&s), vec![(target, 3), (right, 3)]);
        assert_eq!(s.card(&ward).damage, 0);
        assert_eq!(s.state().rng_cursor, before);
    }

    #[test]
    fn r1848001_the_excess_is_what_armor_left_past_the_health_and_indestructible_sends_none() {
        // Armor 7 takes 7 of the 15; the 7/7 counts 7 of the 8 left, and its neighbour takes the 1.
        let mut s = meteor_board(
            "r1848001-armor",
            false,
            json!([]),
            json!([{ "def": armored.id, "lane": 1 }, { "def": plain.id, "lane": 2 }]),
        );
        let (target, right) = (id_at(&s, P2, 1), id_at(&s, P2, 2));
        s.play(meteor.id.as_str(), pick(&target));
        assert_eq!(hits(&s), vec![(target, 7), (right, 1)]);

        let mut s = meteor_board(
            "r1848001-indestructible",
            true,
            json!([]),
            json!([{ "def": indestructible.id, "lane": 1 }, { "def": plain.id, "lane": 2 }]),
        );
        let target = id_at(&s, P2, 1);
        s.play(meteor.id.as_str(), pick(&target));
        assert!(
            hits(&s).is_empty(),
            "Indestructible takes nothing, so nothing passes it"
        );
        assert_eq!(hero_health(&s, P2), HERO_HEALTH);
    }

    #[test]
    fn r1848001_spell_damage_raises_the_hit_once_and_never_its_excess() {
        let mut s = meteor_board(
            "r1848001-spell-damage",
            true,
            json!([{ "def": adept.id, "lane": 1 }]),
            json!([{ "def": plain.id, "lane": 5 }]),
        );
        let target = id_at(&s, P2, 5);
        s.play(meteor.id.as_str(), pick(&target));
        // 20 + 2 = 22; the 3/3 counts 3 and the hero takes the 19 left, not 21.
        assert_eq!(hero_health(&s, P2), HERO_HEALTH - 19);
    }
}

mod r1848002_a_spell_with_poisonous_destroys_a_unit_it_damages {
    use super::*;
    use crate::rules::fixtures::spell_verbs::{cloud, ping};

    const LABEL: &str = "Your next Spell this turn has Poisonous";

    fn labels(s: &Scenario, viewer: PlayerId, of: PlayerId) -> Vec<String> {
        let view = s.view(viewer);
        let side = if of == viewer { &view.you } else { &view.opponent };
        side.modifiers
            .iter()
            .map(|modifier| modifier.label.clone())
            .collect()
    }

    #[test]
    fn r1848002_a_spell_that_prints_poisonous_destroys_every_unit_its_hits_damage() {
        let mut s = board(
            "p1",
            "r1848002",
            json!([cloud.id, ping.id]),
            json!([]),
            json!([plain.id]),
            json!([{ "def": big_body.id, "lane": 1 }, { "def": shielded.id, "lane": 2 }]),
        );
        let (big, shield) = (id_at(&s, P2, 1), id_at(&s, P2, 2));
        s.play(cloud.id.as_str(), json!({}));
        s.expect_in_zone(big.as_str(), "graveyard");
        // Divine Shield took the hit, so nothing was dealt and nothing destroys it (§4.4 step 7).
        s.expect_in_zone(shield.as_str(), "field");
        assert_eq!(hero_health(&s, P2), HERO_HEALTH - 1);
        assert_eq!(labels(&s, P1, P1), vec![LABEL.to_string()]);
        assert_eq!(
            labels(&s, P2, P1),
            vec![LABEL.to_string()],
            "public, as the play was"
        );
    }

    #[test]
    fn r1848002_the_next_spell_this_turn_has_poisonous_for_its_resolution_and_spends_the_grant() {
        let mut s = board(
            "p1",
            "r1848002-next",
            json!([cloud.id, plain.id, ping.id, ping.id]),
            json!([]),
            json!([plain.id]),
            json!([{ "def": shielded.id, "lane": 1 }, { "def": shielded.id, "lane": 2 }]),
        );
        let (first, second) = (id_at(&s, P2, 1), id_at(&s, P2, 2));
        s.play(cloud.id.as_str(), json!({}));
        // A Unit is no Spell: the grant waits.
        s.play(plain.id.as_str(), json!({ "zone": 1 }));
        assert_eq!(labels(&s, P1, P1), vec![LABEL.to_string()]);

        let hand = s.hand(P1);
        let pings: Vec<String> = hand
            .iter()
            .filter(|card| card.def_id == ping.id)
            .map(|card| card.id.clone())
            .collect();
        s.play(pings[0].as_str(), pick(&first));
        assert!(s.last_events().iter().any(|event| matches!(
            event,
            GameEvent::KeywordGranted { instance_id, keyword: Keyword::Poisonous, .. } if *instance_id == pings[0]
        )));
        s.expect_in_zone(first.as_str(), "graveyard");
        assert!(labels(&s, P1, P1).is_empty(), "the Spell spent it");
        // R78: the keyword went with the resolution.
        assert!(s.card(&pings[0]).granted_keywords.is_empty());

        s.play(pings[1].as_str(), pick(&second));
        s.expect_in_zone(second.as_str(), "field");
        assert_eq!(s.card(&second).damage, 1);
    }

    #[test]
    fn r1848002_the_grant_ends_with_the_turn() {
        let mut s = board(
            "p1",
            "r1848002-turn",
            json!([cloud.id, ping.id]),
            json!([]),
            json!([plain.id]),
            json!([{ "def": shielded.id, "lane": 1 }]),
        );
        let unit = id_at(&s, P2, 1);
        s.play(cloud.id.as_str(), json!({}));
        s.end_turn();
        assert!(
            labels(&s, P2, P1).is_empty(),
            "\"this turn\" ended at the cleanup"
        );
        s.end_turn();
        let ping_id = s
            .hand(P1)
            .iter()
            .find(|card| card.def_id == ping.id)
            .unwrap()
            .id
            .clone();
        s.play(ping_id.as_str(), pick(&unit));
        s.expect_in_zone(unit.as_str(), "field");
    }
}

mod r1848003_an_x_card_whose_x_is_set_is_played_for_that_x {
    use super::*;
    use crate::rules::fixtures::spell_verbs::{nono, nono_book, not_x_book, ping};

    fn nonos(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1)
            .into_iter()
            .filter(|card| card.def_id == nono.id)
            .collect()
    }

    fn plays_of(s: &Scenario, id: &str) -> Vec<Option<i32>> {
        legal_actions(s.state(), P1)
            .iter()
            .filter_map(|action| match action {
                ActionBody::Play { instance_id, x, .. } if instance_id == id => Some(*x),
                _ => None,
            })
            .collect()
    }

    fn book_board(seed: &str) -> Scenario {
        let mut s = board(
            "p1",
            seed,
            json!([nono_book.id]),
            json!([]),
            json!([plain.id]),
            json!([]),
        );
        s.play(nono_book.id.as_str(), json!({}));
        s
    }

    #[test]
    fn r1848003_the_copies_cost_their_set_x_and_say_so_on_the_owners_view() {
        let s = book_board("r1848003");
        let made = nonos(&s);
        assert_eq!(
            made.iter().map(|card| card.fixed_x).collect::<Vec<_>>(),
            vec![Some(1), Some(2), Some(3), Some(4)]
        );
        let view = s.view(P1);
        let HandView::Cards(hand) = &view.you.hand else {
            panic!("the owner reads their hand");
        };
        for card in &made {
            let shown = hand.iter().find(|shown| shown.instance_id == card.id).unwrap();
            assert_eq!(shown.fixed_x, card.fixed_x);
            assert_eq!(Some(shown.cost), card.fixed_x, "the price is the set X");
            assert_eq!(
                cost_now(s.state(), card),
                card.fixed_x.unwrap(),
                "R396 reads the same price"
            );
        }
    }

    #[test]
    fn r1848003_legal_actions_offer_each_one_play_with_no_x_and_only_while_it_is_affordable() {
        let s = book_board("r1848003-legal");
        assert_eq!(s.state().players[P1].mana.current, 3);
        for card in nonos(&s) {
            let expected: Vec<Option<i32>> = if card.fixed_x.unwrap() <= 3 {
                vec![None]
            } else {
                vec![]
            };
            assert_eq!(plays_of(&s, &card.id), expected, "X {:?}", card.fixed_x);
        }
    }

    #[test]
    fn r1848003_the_play_is_for_its_set_x_and_one_naming_another_is_refused() {
        let mut s = book_board("r1848003-play");
        let made = nonos(&s);
        let (two, three) = (made[1].id.clone(), made[2].id.clone());
        s.expect_refused_with(|s| s.play(two.as_str(), json!({ "x": 1 })), "X is set at 2");
        // A modifier is no price for an X card (R65): the set X is still the price.
        let turn = s.state().turn;
        s.state_mut().players[P1].mods.push(json_as(json!({
            "id": "m-r1848003",
            "expiry": { "until": "thisTurn", "turn": turn },
            "kind": "costDiscount",
            "amount": 2,
        })));
        assert_eq!(effective_cost(s.state(), s.card(&three), Default::default()), 3);

        s.play(three.as_str(), json!({ "x": 3 }));
        assert_eq!(s.state().players[P1].mana.current, 0);
        assert_eq!(hero_health(&s, P2), HERO_HEALTH - 3, "its text reads X = 3");
        assert!(s.last_events().iter().any(|event| matches!(
            event,
            GameEvent::CardPlayed { instance_id, x: Some(3), .. } if *instance_id == three
        )));
        // R766: the set X is a price, and the graveyard takes it off.
        s.expect_in_zone(three.as_str(), "graveyard");
        assert_eq!(s.card(&three).fixed_x, None);
    }

    #[test]
    fn r1848003_a_card_with_no_x_takes_no_set_x() {
        let mut s = board(
            "p1",
            "r1848003-not-x",
            json!([not_x_book.id]),
            json!([]),
            json!([plain.id]),
            json!([]),
        );
        s.play(not_x_book.id.as_str(), json!({}));
        let made = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == ping.id)
            .unwrap();
        assert_eq!(made.fixed_x, None);
    }
}

mod r1848004_lose_or_gain_n_mana_next_turn_is_the_next_refreshs_rider {
    use super::*;
    use crate::rules::fixtures::spell_verbs::{blackout, core_system, stable, wild_system};

    fn refresh_label(s: &Scenario) -> Option<String> {
        s.view(P1)
            .you
            .modifiers
            .iter()
            .find(|modifier| modifier.id == "nextTurnMana")
            .map(|modifier| modifier.label.clone())
    }

    /// P1's mana as their next turn begins. Each board leaves a (0) Spell in P1's hand, so a turn with
    /// nothing else to do is not ended for them (R82).
    fn next_refresh(s: &mut Scenario) -> i32 {
        s.end_turn();
        s.end_turn();
        assert_eq!(s.state().active, P1);
        s.state().players[P1].mana.current
    }

    #[test]
    fn r1848004_losing_n_takes_n_off_the_next_refresh_and_never_below_0() {
        let mut s = board(
            "p1",
            "r1848004",
            json!([core_system.id, stable.id]),
            json!([]),
            json!([plain.id]),
            json!([]),
        );
        s.play(core_system.id.as_str(), json!({}));
        assert_eq!(refresh_label(&s).as_deref(), Some("Next refresh −2 mana"));
        assert_eq!(
            s.state().players[P1].mana.current,
            4,
            "this turn's mana is untouched"
        );
        assert_eq!(next_refresh(&mut s), 2);
        assert_eq!(refresh_label(&s), None, "the refresh spends it");
        assert_eq!(next_refresh(&mut s), 4, "one refresh only");

        let mut s = board(
            "p1",
            "r1848004-floor",
            json!([blackout.id, stable.id]),
            json!([]),
            json!([plain.id]),
            json!([]),
        );
        s.play(blackout.id.as_str(), json!({}));
        assert_eq!(next_refresh(&mut s), 0);
    }

    #[test]
    fn r1848004_a_gain_and_a_loss_sum_on_the_one_rider() {
        let mut s = board(
            "p1",
            "r1848004-sum",
            json!([wild_system.id, core_system.id, stable.id]),
            json!([]),
            json!([plain.id]),
            json!([]),
        );
        s.play(wild_system.id.as_str(), json!({}));
        assert_eq!(refresh_label(&s).as_deref(), Some("Next refresh +1 mana"));
        s.play(core_system.id.as_str(), json!({}));
        assert_eq!(refresh_label(&s).as_deref(), Some("Next refresh −1 mana"));
        assert_eq!(next_refresh(&mut s), 3);
    }

    #[test]
    fn r1848004_a_lost_refresh_spends_the_rider_with_it() {
        let mut s = board(
            "p1",
            "r1848004-lost",
            json!([wild_system.id, stable.id]),
            json!([]),
            json!([plain.id]),
            json!([]),
        );
        s.play(wild_system.id.as_str(), json!({}));
        let through = s.state().players[P1].turns_started + 1;
        s.state_mut().players[P1].lost_refresh_through = Some(through);
        assert_eq!(next_refresh(&mut s), 0, "R844: a lost refresh gives 0");
        assert_eq!(s.state().players[P1].mana.next_turn_mod, 0);
        assert_eq!(next_refresh(&mut s), 4);
    }
}

mod r1848005_the_melee_makes_forced_attacks_between_random_pairs_until_one_unit_is_left {
    use super::*;
    use crate::rules::fixtures::spell_verbs::{asker, flyer, untouchable, wild_system};

    /// The forced attacks of `events`, as `(attacker, target)`.
    fn forced(events: &[GameEvent]) -> Vec<(String, String)> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::AttackDeclared {
                    attacker_id,
                    target_id,
                    forced: true,
                    ..
                } => Some((attacker_id.clone(), target_id.clone())),
                _ => None,
            })
            .collect()
    }

    fn melee_board(seed: &str, p1_field: Value, p2_field: Value) -> Scenario {
        board(
            "p1",
            seed,
            json!([wild_system.id]),
            p1_field,
            json!([plain.id]),
            p2_field,
        )
    }

    #[test]
    fn r1848005_allies_fight_each_other_and_it_ends_when_one_unit_is_left() {
        let mut s = melee_board(
            "r1848005",
            json!([{ "def": plain.id, "lane": 1 }, { "def": big_body.id, "lane": 2 }]),
            json!([]),
        );
        let (small, big) = (id_at(&s, P1, 1), id_at(&s, P1, 2));
        s.play(wild_system.id.as_str(), json!({}));
        let attacks = forced(s.last_events());
        assert_eq!(attacks.len(), 1, "{attacks:?}");
        assert!(attacks[0] == (small.clone(), big.clone()) || attacks[0] == (big.clone(), small.clone()));
        s.expect_in_zone(small.as_str(), "graveyard");
        s.expect_in_zone(big.as_str(), "field");
        assert_eq!(hero_health(&s, P1), HERO_HEALTH, "a hero is never a target");
    }

    #[test]
    fn r1848005_it_makes_at_most_twenty_attacks() {
        let mut s = melee_board(
            "r1848005-cap",
            json!([{ "def": indestructible.id, "lane": 1 }]),
            json!([{ "def": indestructible.id, "lane": 1 }]),
        );
        s.play(wild_system.id.as_str(), json!({}));
        assert_eq!(forced(s.last_events()).len(), MELEE_MAX_ATTACKS as usize);
        assert!(s.unit(P1, 1).is_some() && s.unit(P2, 1).is_some());
    }

    #[test]
    fn r1848005_only_a_unit_with_a_target_it_may_attack_is_drawn_to_attack() {
        // Can't be attacked: the 1/1 is never a target, so it is the only attacker, and it falls.
        let mut s = melee_board(
            "r1848005-untouchable",
            json!([{ "def": plain.id, "lane": 1 }]),
            json!([{ "def": untouchable.id, "lane": 1 }]),
        );
        let (fighter, aloof) = (id_at(&s, P1, 1), id_at(&s, P2, 1));
        s.play(wild_system.id.as_str(), json!({}));
        assert_eq!(forced(s.last_events()), vec![(aloof.clone(), fighter)]);
        s.expect_in_zone(aloof.as_str(), "graveyard");

        // Can't attack bars a declared attack (§4.2 step 1), which a forced one waives (R53): the
        // 1/1 attacks the 3/3 as readily as it is attacked.
        let mut s = melee_board(
            "r1848005-pacifist",
            json!([{ "def": plain.id, "lane": 1 }]),
            json!([{ "def": pacifist.id, "lane": 1 }]),
        );
        s.play(wild_system.id.as_str(), json!({}));
        assert_eq!(forced(s.last_events()).len(), 1);

        // Flying: the 3/3 may not attack it in Attack Position, so the flyer attacks and falls.
        let mut s = melee_board(
            "r1848005-flyer",
            json!([{ "def": plain.id, "lane": 1 }]),
            json!([{ "def": flyer.id, "lane": 1 }]),
        );
        let (ground, air) = (id_at(&s, P1, 1), id_at(&s, P2, 1));
        s.play(wild_system.id.as_str(), json!({}));
        assert_eq!(forced(s.last_events()), vec![(air.clone(), ground.clone())]);
        s.expect_in_zone(air.as_str(), "graveyard");
        s.expect_in_zone(ground.as_str(), "field");
    }

    #[test]
    fn r1848005_with_no_pair_to_fight_nothing_happens_and_nothing_is_drawn() {
        for (seed, p1_field, p2_field) in [
            (
                "r1848005-none",
                json!([{ "def": untouchable.id, "lane": 1 }]),
                json!([{ "def": untouchable.id, "lane": 1 }]),
            ),
            ("r1848005-one", json!([{ "def": plain.id, "lane": 1 }]), json!([])),
        ] {
            let mut s = melee_board(seed, p1_field, p2_field);
            let before = s.state().rng_cursor;
            s.play(wild_system.id.as_str(), json!({}));
            assert!(forced(s.last_events()).is_empty(), "{seed}");
            assert_eq!(s.state().rng_cursor, before, "{seed}");
        }
    }

    #[test]
    fn r1848005_a_death_that_asks_owes_the_rest_of_the_melee_behind_its_answer() {
        let setup = |seed: &str| {
            melee_board(
                seed,
                json!([{ "def": asker.id, "lane": 1 }, { "def": indestructible.id, "lane": 2 }]),
                json!([{ "def": indestructible.id, "lane": 1 }]),
            )
        };
        let seed = seed_where("r1848005-asks", |seed| {
            let mut s = setup(seed);
            s.play(wild_system.id.as_str(), json!({}));
            s.state().pending.is_some() && !forced(s.last_events()).is_empty()
        });
        let mut s = setup(&seed);
        let from = s.events().len();
        s.play(wild_system.id.as_str(), json!({}));
        let before_answer = forced(&s.events()[from..]).len();
        assert!(
            before_answer < MELEE_MAX_ATTACKS as usize,
            "the melee waits for the answer"
        );
        s.answer(json!([{ "pick": "none" }]));
        assert_eq!(
            forced(&s.events()[from..]).len(),
            MELEE_MAX_ATTACKS as usize,
            "the answer's drain makes the attacks it still owed"
        );
        assert!(s.state().pending.is_none());
        assert!(s.state().work.is_empty());
    }
}

mod r1848006_inflation_rolls_each_card_of_a_hand_and_a_deck_and_raises_its_price {
    use super::*;
    use crate::rules::fixtures::spell_verbs::{hyperinflation, nono, sanctions, stable, stimmy};
    use jackioh_engine::prelude::{inflation_chance_of, next_inflation_chance};

    /// The `costMod` of every card of `player`'s hand and library, in order, a Stimmycheck left out.
    fn mods(s: &Scenario, player: PlayerId) -> Vec<i32> {
        let side = &s.state().players[player];
        side.hand
            .iter()
            .filter(|card| card.def_id != stimmy.id)
            .chain(side.library.iter())
            .map(|card| card.cost_mod)
            .collect()
    }

    /// What a roll at `chance` adds to each of `count` cards from `cursor`, `lucky` extra draws each,
    /// kept as `own` says (a miss on the roller's own cards, a hit on the opponent's).
    fn expected(seed: &str, cursor: u32, count: usize, chance: i32, lucky: i32, own: bool) -> Vec<i32> {
        let mut rng = Rng::new(seed, cursor);
        (0..count)
            .map(|_| {
                let hit = rng.lucky(
                    lucky,
                    |rng| rng.int(INFLATION_ROLL_SIDES) < chance,
                    |kept, next| if own { kept && next } else { kept || next },
                );
                if hit { INFLATION_COST } else { 0 }
            })
            .collect()
    }

    fn plus(was: &[i32], rolls: &[i32]) -> Vec<i32> {
        was.iter()
            .zip(rolls.iter())
            .map(|(was, roll)| was + roll)
            .collect()
    }

    fn stimmy_board(seed: &str, radiant: bool) -> Scenario {
        board(
            "p1",
            seed,
            json!([{ "def": stimmy.id, "radiant": radiant }, plain.id, plain.id, plain.id]),
            json!([]),
            json!([plain.id]),
            json!([]),
        )
    }

    #[test]
    fn r1848006_one_roll_per_card_hand_then_deck_and_a_hit_costs_1_more() {
        let mut s = stimmy_board("r1848006", false);
        let seed = s.state().seed.clone();
        let before = s.state().rng_cursor;
        let events_before = s.events().len();
        s.play(stimmy.id.as_str(), json!({}));
        // Three cards in hand and four in the deck roll; the copy the Stimmycheck adds comes after.
        assert_eq!(s.state().rng_cursor, before + 7);
        assert_eq!(
            mods(&s, P1),
            expected(&seed, before, 7, INFLATION_CHANCE, 0, true)
        );
        // R440: nobody reads both a hand and a deck, so nothing announces the rolls.
        assert!(
            !s.events()[events_before..]
                .iter()
                .any(|event| matches!(event, GameEvent::CostChanged { .. }))
        );
    }

    #[test]
    fn r1848006_the_copy_carries_a_chance_10_higher_and_rolls_at_it() {
        let mut s = stimmy_board("r1848006-copy", false);
        s.play(stimmy.id.as_str(), json!({}));
        let copy = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == stimmy.id)
            .unwrap();
        let grown = INFLATION_CHANCE + INFLATION_CHANCE_STEP;
        assert_eq!(inflation_chance_of(Some(&copy), INFLATION_CHANCE), grown);

        let seed = s.state().seed.clone();
        let was = mods(&s, P1);
        let before = s.state().rng_cursor;
        s.play(copy.id.as_str(), json!({}));
        assert_eq!(
            mods(&s, P1),
            plus(&was, &expected(&seed, before, 7, grown, 0, true))
        );
        let next = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == stimmy.id)
            .unwrap();
        assert_eq!(
            inflation_chance_of(Some(&next), INFLATION_CHANCE),
            grown + INFLATION_CHANCE_STEP
        );
        assert_eq!(
            next_inflation_chance(95),
            INFLATION_ROLL_SIDES,
            "never past a certain hit"
        );
    }

    #[test]
    fn r1848006_lucky_rolls_again_and_keeps_a_miss_on_your_own_cards_and_a_hit_on_the_enemys() {
        let mut s = stimmy_board("r1848006-lucky", true);
        let seed = s.state().seed.clone();
        let before = s.state().rng_cursor;
        s.play(stimmy.id.as_str(), json!({}));
        assert_eq!(s.state().rng_cursor, before + 14, "Lucky 1: two draws per card");
        assert_eq!(
            mods(&s, P1),
            expected(&seed, before, 7, INFLATION_CHANCE, 1, true)
        );

        let mut s = board(
            "p1",
            "r1848006-enemy",
            json!([{ "def": sanctions.id, "radiant": true }]),
            json!([]),
            json!([plain.id, plain.id]),
            json!([]),
        );
        let seed = s.state().seed.clone();
        let before = s.state().rng_cursor;
        s.play(sanctions.id.as_str(), json!({}));
        assert_eq!(
            mods(&s, P2),
            expected(&seed, before, 6, INFLATION_CHANCE, 1, false)
        );
        assert!(
            mods(&s, P1).iter().all(|cost_mod| *cost_mod == 0),
            "only the named player's cards"
        );
    }

    #[test]
    fn r1848006_certain_and_impossible_chances_roll_nothing_and_an_x_card_is_passed_by() {
        let mut s = board(
            "p1",
            "r1848006-certain",
            json!([hyperinflation.id, stable.id, plain.id, nono.id]),
            json!([]),
            json!([plain.id]),
            json!([]),
        );
        let before = s.state().rng_cursor;
        s.play(stable.id.as_str(), json!({}));
        assert_eq!(s.state().rng_cursor, before);
        assert!(mods(&s, P1).iter().all(|cost_mod| *cost_mod == 0));

        s.play(hyperinflation.id.as_str(), json!({}));
        assert_eq!(s.state().rng_cursor, before);
        let plain_card = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == plain.id)
            .unwrap();
        let x_card = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == nono.id)
            .unwrap();
        assert_eq!(plain_card.cost_mod, INFLATION_COST);
        assert_eq!(x_card.cost_mod, 0, "R65: no price change reaches an X card");
        assert!(
            s.state().players[P1]
                .library
                .iter()
                .all(|card| card.cost_mod == INFLATION_COST)
        );
        assert_eq!(
            effective_cost(s.state(), &plain_card, Default::default()),
            1 + INFLATION_COST
        );
    }
}

mod r1868000_an_x_cards_declared_target_reads_the_x_of_its_play {
    use super::*;
    use crate::rules::fixtures::spell_verbs::{nono_x, riser, stable};

    /// Each play of `id` that `legal_actions` lists, as its X and the ids it targets.
    fn plays_of(s: &Scenario, id: &str) -> Vec<(Option<i32>, Vec<String>)> {
        legal_actions(s.state(), P1)
            .iter()
            .filter_map(|action| match action {
                ActionBody::Play {
                    instance_id,
                    x,
                    targets,
                    ..
                } if instance_id == id => Some((
                    *x,
                    targets
                        .iter()
                        .flatten()
                        .filter_map(|selection| match selection {
                            Selection::Instance { instance_id } => Some(instance_id.clone()),
                            _ => None,
                        })
                        .collect(),
                )),
                _ => None,
            })
            .collect()
    }

    /// The ids X `x`'s plays target, in listing order.
    fn targets_at(plays: &[(Option<i32>, Vec<String>)], x: Option<i32>) -> Vec<String> {
        plays
            .iter()
            .filter(|(at, _)| *at == x)
            .flat_map(|(_, targets)| targets.clone())
            .collect()
    }

    fn nono_board(seed: &str, p2_field: Value) -> Scenario {
        board(
            "p1",
            seed,
            json!([nono_x.id, stable.id]),
            json!([]),
            json!([plain.id]),
            p2_field,
        )
    }

    fn plain_and_big(seed: &str) -> Scenario {
        nono_board(
            seed,
            json!([{ "def": plain.id, "lane": 1 }, { "def": big_body.id, "lane": 2 }]),
        )
    }

    #[test]
    fn r1868000_legal_actions_pair_each_x_with_the_units_below_four_times_it() {
        let s = plain_and_big("r1868000-legal");
        let (three, five) = (id_at(&s, P2, 1), id_at(&s, P2, 2));
        let plays = plays_of(&s, &s.card(nono_x.id.as_str()).id);
        // A 3-Attack Unit is below 4 × 1; a 5-Attack one is not, and is below 4 × 2 and up.
        assert_eq!(targets_at(&plays, Some(1)), vec![three.clone()]);
        for x in 2..=4 {
            assert_eq!(
                targets_at(&plays, Some(x)),
                vec![three.clone(), five.clone()],
                "X {x}"
            );
        }
        assert!(targets_at(&plays, Some(5)).is_empty(), "4 mana pays no X of 5");
    }

    #[test]
    fn r1868000_the_reducer_refuses_a_pick_its_x_does_not_allow() {
        let mut s = plain_and_big("r1868000-refuse");
        let five = id_at(&s, P2, 2);
        let at_x = |x: i32| json!({ "x": x, "targets": pick::id(&five) });
        let one = at_x(1);
        s.expect_refused_with(|s| s.play(nono_x.id.as_str(), one), "not a legal target");
        s.expect_in_zone(nono_x.id.as_str(), "hand");

        s.play(nono_x.id.as_str(), at_x(2));
        s.expect_in_zone(five.as_str(), "graveyard");
    }

    #[test]
    fn r1868000_an_x_with_no_unit_below_its_limit_is_not_offered_and_is_refused() {
        let mut s = nono_board("r1868000-needed", json!([{ "def": big_body.id, "lane": 1 }]));
        let plays = plays_of(&s, &s.card(nono_x.id.as_str()).id);
        // R703: the pick is needed, so an X whose limit no Unit is below is no play.
        assert!(plays.iter().all(|(x, _)| *x != Some(1)), "{plays:?}");
        assert_eq!(targets_at(&plays, Some(2)), vec![id_at(&s, P2, 1)]);
        s.expect_refused_with(
            |s| s.play(nono_x.id.as_str(), json!({ "x": 1 })),
            "cannot be played without",
        );
    }

    #[test]
    fn r1868000_a_set_x_is_the_plays_x() {
        let mut s = plain_and_big("r1868000-set");
        let three = id_at(&s, P2, 1);
        s.card_mut(nono_x.id.as_str()).fixed_x = Some(1);
        let plays = plays_of(&s, &s.card(nono_x.id.as_str()).id);
        assert_eq!(plays, vec![(None, vec![three])]);
    }

    #[test]
    fn r1868000_r1867005_a_target_raised_past_it_after_the_pick_is_still_destroyed() {
        let mut s = nono_board("r1868000-raised", json!([{ "def": riser.id, "lane": 1 }]));
        let risen = id_at(&s, P2, 1);
        s.play(nono_x.id.as_str(), json!({ "x": 1, "targets": pick::id(&risen) }));
        // §10.5 step 4: its play raised it to 5 Attack before the Spell resolved, and the pick stands.
        assert!(s.last_events().iter().any(|event| matches!(
            event,
            GameEvent::Buffed { instance_id, attack: 2, .. } if *instance_id == risen
        )));
        s.expect_in_zone(risen.as_str(), "graveyard");
    }
}
