// The weapon kit (issue #1841, Stinkyscape's phase 0), each primitive through a fixture script:
// R1841000's Activate with an extra turn cooldown (`extraTurnCooldown`), R1841001's damage that deals
// half to heroes (`halfToHeroes`), R1841002's Equip straight into this Unit from a pick (`equip_picks`),
// R1841003's Float lowering Tribute by 1, and R2268000's view of the proto's Tribute (issue #2268).

use jackioh_engine::effects::EQUIP_PICK_STEP;
use jackioh_engine::play_choices::{legal_tribute_sets, tribute_cost_of};
use jackioh_engine::subsystems::activate::{abilities_of, activate_actions_for, activation_views_for};
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::types_of;
use jackioh_engine::work::can_resume;

use crate::rules::fixtures::weapon_kit::{
    BEAM, BEAM_DAMAGE, BLADE, CHEST, LOCKER, PEA, PICKS, PLAIN, PROTO, PROTO_TRIBUTE, RACK, RACK_OPTIONS,
    SHIELD, STATUE, WALL, register_weapon_kit_fixtures,
};

/// `p1`'s turn 9, its main phase; each side's setup as given, with plain cards to draw, and no turn
/// that ends by itself (R345), so each test ends every turn it means to.
fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    register_weapon_kit_fixtures();
    let mut p1 = p1;
    let mut p2 = p2;
    for side in [&mut p1, &mut p2] {
        if side.get("library").is_none() {
            side["library"] = json!([PLAIN, PLAIN, PLAIN, PLAIN]);
        }
    }
    let mut s = scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }));
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

fn id_at(s: &Scenario, player: PlayerId, lane: i32) -> String {
    s.unit(player, lane)
        .unwrap_or_else(|| panic!("no unit at {player:?}'s lane {lane}"))
        .id
}

/// The beam's or the pea's shot at p2's hero.
fn at_hero(ability: &str) -> Value {
    json!({ "ability": ability, "targets": [{ "pick": "hero", "player": "p2" }] })
}

fn at_unit(ability: &str, id: &str) -> Value {
    json!({ "ability": ability, "targets": [{ "pick": "instance", "instanceId": id }] })
}

/// Ends turns until it is `player`'s again, `times` times over.
fn pass_to(s: &mut Scenario, player: PlayerId, times: usize) {
    for _ in 0..times {
        s.end_turn();
        while s.state().active != player {
            s.end_turn();
        }
    }
}

fn attack_health(s: &Scenario, id: &str) -> (i32, i32) {
    let view = s.stats(id);
    (view.attack, view.health)
}

/// The open prompt's options, by the option each names.
fn offered(s: &Scenario) -> Vec<String> {
    let pending = s.state().pending.as_ref().expect("a prompt is open");
    pending
        .options
        .iter()
        .filter_map(|option| match &option.selection {
            Selection::Mode { option } => Some(option.clone()),
            _ => None,
        })
        .collect()
}

mod r1841000_extra_turn_cooldown {
    use super::*;

    #[test]
    fn r1841000_a_cooldown_ability_sits_out_its_users_next_turn_and_comes_back_the_turn_after() {
        let mut s = board("r1841000-cooldown", json!({ "field": [BEAM] }), json!({}));
        let beam = id_at(&s, P1, 1);
        let halved = BEAM_DAMAGE / HALF_DAMAGE_TO_HEROES_DIVISOR;

        s.activate(beam.as_str(), at_hero("beam"));
        s.expect_health(P2, HERO_HEALTH - halved);
        s.expect_refused_with(
            |s| s.activate(beam.as_str(), at_hero("beam")),
            "already been used",
        );

        // p1's next turn: the cooldown, not the use count, refuses it.
        pass_to(&mut s, P1, 1);
        s.expect_refused_with(|s| s.activate(beam.as_str(), at_hero("beam")), "cooling down");
        s.expect_health(P2, HERO_HEALTH - halved);

        // The turn after that, it is back.
        pass_to(&mut s, P1, 1);
        s.activate(beam.as_str(), at_hero("beam"));
        s.expect_health(P2, HERO_HEALTH - 2 * halved);
    }

    #[test]
    fn r1841000_the_list_the_refusal_and_the_view_agree_while_it_cools_down() {
        let mut s = board("r1841000-agree", json!({ "field": [BEAM] }), json!({}));
        let beam = id_at(&s, P1, 1);
        s.activate(beam.as_str(), at_hero("beam"));

        pass_to(&mut s, P1, 1);
        let card = s.card(beam.as_str()).clone();
        assert!(activate_actions_for(s.state(), P1, &card).is_empty());
        assert!(!legal_actions(s.state(), P1).iter().any(
            |action| matches!(action, ActionBody::Activate { instance_id, .. } if *instance_id == beam)
        ));
        let views = activation_views_for(s.state(), P1, &card).expect("p1 sees the beam's ability");
        assert_eq!(views.len(), 1);
        assert!(!views[0].usable);
        assert_eq!(
            views[0].reason.as_deref(),
            Some("that ability is cooling down this turn")
        );

        pass_to(&mut s, P1, 1);
        let card = s.card(beam.as_str()).clone();
        // One action per target: the enemy hero and the enemy units (p2 has none).
        assert_eq!(activate_actions_for(s.state(), P1, &card).len(), 1);
        let views = activation_views_for(s.state(), P1, &card).expect("p1 sees the beam's ability");
        assert!(views[0].usable);
        assert_eq!(views[0].reason, None);
    }

    #[test]
    fn r1841000_an_extra_turn_is_its_users_next_turn() {
        let mut s = board("r1841000-extra", json!({ "field": [BEAM] }), json!({}));
        let beam = id_at(&s, P1, 1);
        s.activate(beam.as_str(), at_hero("beam"));
        s.state_mut().players[P1].extra_turns = Some(1);

        // p1's extra turn follows at once, and is the turn the ability sits out.
        s.end_turn();
        assert_eq!(s.state().active, P1);
        s.expect_refused_with(|s| s.activate(beam.as_str(), at_hero("beam")), "cooling down");

        pass_to(&mut s, P1, 1);
        s.activate(beam.as_str(), at_hero("beam"));
    }

    #[test]
    fn r1841000_an_ability_without_the_rider_is_used_again_next_turn() {
        let mut s = board("r1841000-none", json!({ "field": [PEA] }), json!({}));
        let pea = id_at(&s, P1, 1);
        s.activate(pea.as_str(), at_hero("pea"));
        pass_to(&mut s, P1, 1);
        s.activate(pea.as_str(), at_hero("pea"));
        assert!(
            s.card(pea.as_str()).memory.get("cooldowns").is_none(),
            "no cooldown is kept without the rider"
        );
    }
}

mod r1841001_half_damage_to_heroes {
    use super::*;

    #[test]
    fn r1841001_a_hit_on_a_hero_is_halved_rounded_down_and_a_unit_takes_it_whole() {
        let mut s = board("r1841001-hero", json!({ "field": [BEAM] }), json!({}));
        let beam = id_at(&s, P1, 1);
        s.activate(beam.as_str(), at_hero("beam"));
        // 7 halved is 3, rounded down.
        s.expect_health(P2, HERO_HEALTH - 3);

        let mut s = board(
            "r1841001-unit",
            json!({ "field": [BEAM] }),
            json!({ "field": [WALL] }),
        );
        let beam = id_at(&s, P1, 1);
        let wall = id_at(&s, P2, 1);
        s.activate(beam.as_str(), at_unit("beam", &wall));
        assert_eq!(attack_health(&s, &wall), (0, 8 - BEAM_DAMAGE));
        s.expect_health(P2, HERO_HEALTH);
    }

    #[test]
    fn r1841001_a_hit_halved_to_zero_is_no_hit() {
        let mut s = board("r1841001-zero", json!({ "field": [PEA] }), json!({}));
        let pea = id_at(&s, P1, 1);
        s.activate(pea.as_str(), at_hero("pea"));
        s.expect_health(P2, HERO_HEALTH);
        let types = types_of(s.last_events());
        assert!(!types.contains(&GameEventType::Damage), "{types:?}");
        assert!(!types.contains(&GameEventType::DamageAbsorbed), "{types:?}");
    }

    #[test]
    fn r1841001_armor_takes_the_halved_hit_and_reports_it_absorbed() {
        let mut s = board(
            "r1841001-armor",
            json!({ "field": [BEAM] }),
            json!({ "armor": 3 }),
        );
        let beam = id_at(&s, P1, 1);
        s.activate(beam.as_str(), at_hero("beam"));
        // Halved first (3), then Armor 3 takes it whole: halving after Armor would have dealt 2.
        s.expect_health(P2, HERO_HEALTH);
        let absorbed: Vec<i32> = s
            .last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::DamageAbsorbed { absorbed, .. } => Some(*absorbed),
                _ => None,
            })
            .collect();
        assert_eq!(absorbed, vec![3]);
        assert!(!types_of(s.last_events()).contains(&GameEventType::Damage));
    }
}

mod r1841002_equip_from_a_pick {
    use super::*;

    #[test]
    fn r1841002_a_discover_equips_each_pick_into_this_unit_and_asks_again_for_twice() {
        let mut s = board("r1841002-discover", json!({ "hand": [RACK, PLAIN] }), json!({}));
        let hand_before = s.hand(P1).len();
        s.play(RACK, json!({ "zone": 3 }));
        let rack = id_at(&s, P1, 3);

        let pending = s.state().pending.clone().expect("the rack's Discover is open");
        assert_eq!(pending.kind, PromptKind::Discover);
        assert_eq!(pending.player_id, P1);
        assert_eq!(pending.resume.step, EQUIP_PICK_STEP);
        assert!(can_resume(s.state(), &pending.resume));
        let mut first = offered(&s);
        first.sort();
        assert_eq!(first.len(), RACK_OPTIONS as usize);
        assert_eq!(first, vec![BLADE.to_string(), SHIELD.to_string()]);

        s.answer(json!(BLADE));
        assert_eq!(
            types_of(s.last_events())
                .iter()
                .filter(|kind| **kind == GameEventType::Fused)
                .count(),
            1
        );
        // The second pick's pool leaves out the card the first fused into the rack (R387).
        assert_eq!(offered(&s), vec![SHIELD.to_string()]);

        s.answer(json!(SHIELD));
        assert!(s.state().pending.is_none());
        let fused = s.card(rack.as_str()).clone();
        assert_eq!(fused.zone.z(), ZoneName::Field, "the rack is the kept instance");
        assert_eq!(attack_health(&s, &rack), (2 + 1, 2 + 1 + 3));
        assert!(
            abilities_of(s.state(), &fused)
                .iter()
                .any(|ability| ability.id == "blade")
        );
        // Never through a hand, and no play: the blade's Cry never ran and nothing counted it.
        assert_eq!(s.hand(P1).len(), hand_before - 1);
        s.expect_health(P2, HERO_HEALTH);
        assert_eq!(s.state().players[P1].turn_log.cards_played, 1);
    }

    #[test]
    fn r1841002_a_radiant_pick_goes_in_on_its_radiant_face() {
        let mut s = board(
            "r1841002-radiant",
            json!({ "hand": [{ "def": RACK, "radiant": true }] }),
            json!({}),
        );
        s.play(RACK, json!({ "zone": 3 }));
        let rack = id_at(&s, P1, 3);
        s.answer(json!(SHIELD));
        s.answer(json!(BLADE));
        // The Radiant rack's 4/4, a Radiant shield's 0/6 and a Radiant blade's 2/2.
        assert_eq!(attack_health(&s, &rack), (4 + 2, 4 + 6 + 2));
    }

    #[test]
    fn r1841002_a_random_pick_equips_without_a_prompt_each_pick_a_different_card() {
        let mut s = board("r1841002-random", json!({ "hand": [CHEST] }), json!({}));
        s.play(CHEST, json!({ "zone": 3 }));
        let chest = id_at(&s, P1, 3);
        assert!(s.state().pending.is_none());
        // Both picks, on their Radiant faces: a blade's 2/2 and a shield's 0/6, in either order.
        assert_eq!(attack_health(&s, &chest), (2 + 2, 2 + 2 + 6));
        assert_eq!(
            types_of(s.last_events())
                .iter()
                .filter(|kind| **kind == GameEventType::Fused)
                .count(),
            PICKS as usize
        );
    }

    #[test]
    fn r1841002_a_pool_the_unit_already_holds_whole_equips_nothing_more() {
        let mut s = board("r1841002-empty", json!({ "hand": [LOCKER] }), json!({}));
        s.play(LOCKER, json!({ "zone": 3 }));
        let locker = id_at(&s, P1, 3);
        // One shield, on its base face; the second pick's pool is empty (R129).
        assert_eq!(attack_health(&s, &locker), (2, 2 + 3));
        assert_eq!(
            types_of(s.last_events())
                .iter()
                .filter(|kind| **kind == GameEventType::Fused)
                .count(),
            1
        );
    }

    #[test]
    fn r1841002_an_immutable_host_takes_no_pick_and_opens_no_prompt() {
        let mut s = board("r1841002-immutable", json!({ "hand": [STATUE] }), json!({}));
        let cursor = s.state().rng_cursor;
        s.play(STATUE, json!({ "zone": 3 }));
        let statue = id_at(&s, P1, 3);
        assert!(s.state().pending.is_none());
        assert_eq!(attack_health(&s, &statue), (2, 2));
        assert!(!types_of(s.last_events()).contains(&GameEventType::Fused));
        assert_eq!(
            s.state().rng_cursor,
            cursor,
            "nothing was offered, so nothing was drawn"
        );
    }
}

mod r1841003_float_lowers_tribute {
    use super::*;

    /// p1 with a proto in hand and one in the library, three plain units on the field.
    fn proto_board(seed: &str) -> (Scenario, String, String) {
        let s = board(
            seed,
            json!({
                "hand": [PROTO],
                "library": [PROTO, PLAIN, PLAIN, PLAIN, PLAIN],
                "field": [PLAIN, PLAIN, PLAIN],
            }),
            json!({}),
        );
        let in_hand = s.pile(P1, "hand")[0].id.clone();
        let in_deck = s
            .pile(P1, "library")
            .iter()
            .find(|card| card.def_id == PROTO)
            .expect("a proto in the library")
            .id
            .clone();
        (s, in_hand, in_deck)
    }

    fn tribute(s: &Scenario, id: &str) -> i32 {
        tribute_cost_of(s.state(), s.card(id))
    }

    #[test]
    fn r1841003_each_float_of_its_owner_takes_one_off_in_hand_and_deck() {
        let (mut s, in_hand, in_deck) = proto_board("r1841003-float");
        assert_eq!(tribute(&s, &in_hand), PROTO_TRIBUTE);
        assert_eq!(tribute(&s, &in_deck), PROTO_TRIBUTE);

        // p1 ends turn 9 with mana unspent: a Float, whatever its size, takes 1 off.
        s.end_turn();
        assert_eq!(tribute(&s, &in_hand), PROTO_TRIBUTE - 1);
        assert_eq!(tribute(&s, &in_deck), PROTO_TRIBUTE - 1);

        // p2's Float lowers nothing of p1's.
        s.end_turn();
        assert_eq!(s.state().active, P1);
        assert_eq!(tribute(&s, &in_hand), PROTO_TRIBUTE - 1);
        assert_eq!(tribute(&s, &in_deck), PROTO_TRIBUTE - 1);

        // The play's Tribute is the lowered one: pairs of the three units.
        let proto = s.card(in_hand.as_str()).clone();
        let sets = legal_tribute_sets(s.state(), P1, &proto);
        assert_eq!(sets.len(), 3);
        assert!(sets.iter().all(|set| set.len() == 2));
        let first = id_at(&s, P1, 1);
        let second = id_at(&s, P1, 2);
        let third = id_at(&s, P1, 3);
        s.play(
            in_hand.as_str(),
            json!({ "zone": 4, "tributes": [first.as_str(), second.as_str()] }),
        );
        s.expect_in_zone(in_hand.as_str(), "field");
        s.expect_in_zone(third.as_str(), "field");
        s.expect_in_zone(first.as_str(), "graveyard");
    }

    #[test]
    fn r1841003_the_tribute_never_goes_below_zero() {
        let (mut s, in_hand, _) = proto_board("r1841003-floor");
        pass_to(&mut s, P1, 4);
        assert_eq!(tribute(&s, &in_hand), 0);
        let first = id_at(&s, P1, 1);
        s.expect_refused_with(
            |s| {
                s.play(
                    in_hand.as_str(),
                    json!({ "zone": 4, "tributes": [first.as_str()] }),
                )
            },
            "needs no Tribute",
        );
        s.play(in_hand.as_str(), json!({ "zone": 4 }));
        s.expect_in_zone(in_hand.as_str(), "field");
        s.expect_in_zone(first.as_str(), "field");
    }
}

mod r2268000_tribute_in_the_view {
    use super::*;

    /// The id of the first card of `def_id` in p1's hand.
    fn held(s: &Scenario, def_id: &str) -> String {
        s.pile(P1, "hand")
            .iter()
            .find(|card| card.def_id == def_id)
            .expect("in p1's hand")
            .id
            .clone()
    }

    /// p1's hand card `id` as `viewer` sees it: a count for p2 until the game ends (R434), so `None`.
    fn shown(s: &Scenario, viewer: PlayerId, id: &str) -> Option<CardView> {
        let view = s.view(viewer);
        let hand = if viewer == P1 {
            view.you.hand
        } else {
            view.opponent.hand
        };
        match hand {
            HandView::Cards(cards) => cards.into_iter().find(|card| card.instance_id == id),
            HandView::Count { .. } => None,
        }
    }

    #[test]
    fn r2268000_the_owners_view_carries_the_tribute_its_play_asks_each_float_taking_one_off() {
        let mut s = board(
            "r2268000-float",
            json!({ "hand": [PROTO, PLAIN], "field": [PLAIN, PLAIN, PLAIN] }),
            json!({}),
        );
        let proto = held(&s, PROTO);
        let plain = held(&s, PLAIN);

        for left in (1..=PROTO_TRIBUTE).rev() {
            assert_eq!(tribute_cost_of(s.state(), s.card(proto.as_str())), left);
            let card = shown(&s, P1, &proto).expect("in its owner's hand");
            assert_eq!(card.tribute, Some(left));
            // p1 Floats, p2 ends: one off.
            pass_to(&mut s, P1, 1);
        }

        assert_eq!(tribute_cost_of(s.state(), s.card(proto.as_str())), 0);
        let card = shown(&s, P1, &proto).expect("in its owner's hand");
        assert_eq!(card.tribute, None, "absent at 0, never 0");
        let card = shown(&s, P1, &plain).expect("in its owner's hand");
        assert_eq!(card.tribute, None, "a card that asks no Tribute shows none");
    }

    #[test]
    fn r2268000_neither_the_opponent_nor_the_field_reads_a_tribute() {
        let mut s = board(
            "r2268000-hidden",
            json!({ "hand": [PROTO, PROTO], "field": [PLAIN, PLAIN, PLAIN] }),
            json!({}),
        );
        let hand = s.pile(P1, "hand");
        let played = hand[0].id.clone();
        let kept = hand[1].id.clone();
        for id in [&played, &kept] {
            let card = shown(&s, P1, id).expect("in its owner's hand");
            assert_eq!(card.tribute, Some(PROTO_TRIBUTE));
        }
        assert!(matches!(s.view(P2).opponent.hand, HandView::Count { .. }));
        assert_eq!(shown(&s, P2, &kept), None);

        let tributes: Vec<String> = (1..=PROTO_TRIBUTE).map(|lane| id_at(&s, P1, lane)).collect();
        s.play(played.as_str(), json!({ "zone": 4, "tributes": tributes }));
        s.expect_in_zone(played.as_str(), "field");
        for viewer in [P1, P2] {
            let view = s.view(viewer);
            let units =
                serde_json::to_string(&[view.you.units, view.opponent.units]).expect("a view is JSON");
            assert!(
                units.contains(PROTO),
                "the proto is on the field, seen by {viewer:?}"
            );
            assert!(
                !units.contains("\"tribute\""),
                "no unit carries a tribute: {units}"
            );
        }
        let card = shown(&s, P1, &kept).expect("in its owner's hand");
        assert_eq!(card.tribute, Some(PROTO_TRIBUTE));

        s.act(ActionBody::Concede, P2, "p2 concedes");
        let card = shown(&s, P2, &kept).expect("revealed at the end (R434)");
        assert_eq!(card.tribute, None);
    }
}
