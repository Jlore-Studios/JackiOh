// Layered summons (issue #1842): several Units summoned into one unit zone as a §3.2 Stack pile, bottom
// first (`summon_pile`, R1842000), and R1204's lethal guard standing in such a pile, where it guards only
// while it is the top (R1842001). Each through a fixture script (`fixtures/layered_summons.rs`) before
// a card uses it: the Stinkyscape Drone Battleship (S07), Meltdown at the bottom and the Shield on top.

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::layered_summons::{
    BOLT, GUARD_ARMOR, ally, asker, asking_pile, bolt, bottom, guard, lane_pile, middle, mixed_pile, pile,
    register_layered_fixtures, smite, trap, watcher, wipe,
};

/// `p1`'s turn 9, its main phase; each side's setup as given, with plain cards to draw so no turn ends
/// by itself (§2.5).
fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    register_layered_fixtures();
    let mut p1 = p1;
    let mut p2 = p2;
    for side in [&mut p1, &mut p2] {
        if side.get("library").is_none() {
            side["library"] = json!([plain.id, plain.id, plain.id, plain.id]);
        }
    }
    scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }))
}

/// The cards of a unit zone's pile, top first; none when the zone is empty.
fn pile_cards(s: &Scenario, player: PlayerId, lane: i32) -> Vec<CardInstance> {
    pile_at(
        s.state(),
        ZoneSlot {
            player,
            row: Row::Units,
            lane,
        },
    )
    .cloned()
    .unwrap_or_default()
}

/// The definitions of a unit zone's pile, top first.
fn pile_defs(s: &Scenario, player: PlayerId, lane: i32) -> Vec<String> {
    pile_cards(s, player, lane)
        .into_iter()
        .map(|card| card.def_id)
        .collect()
}

/// The instances of a unit zone's pile, top first.
fn pile_ids(s: &Scenario, player: PlayerId, lane: i32) -> Vec<String> {
    pile_cards(s, player, lane)
        .into_iter()
        .map(|card| card.id)
        .collect()
}

/// `ls-pile`'s three as they stand, top first.
fn the_pile() -> Vec<String> {
    vec![guard.id.clone(), middle.id.clone(), bottom.id.clone()]
}

/// The `summoned` events of the last step, as (definition, lane).
fn summoned(events: &[GameEvent]) -> Vec<(String, i32)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Summoned { def_id, lane, .. } => Some((def_id.clone(), *lane)),
            _ => None,
        })
        .collect()
}

/// The `redirected` events of the last step, as (from, to).
fn redirected(events: &[GameEvent]) -> Vec<(String, String)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Redirected { from_id, to_id, .. } => Some((from_id.clone(), to_id.clone())),
            _ => None,
        })
        .collect()
}

fn pick(id: &str) -> Value {
    json!({ "targets": [{ "pick": "instance", "instanceId": id }] })
}

fn in_graveyard(s: &Scenario, id: &str) -> bool {
    s.card(id).zone.z() == ZoneName::Graveyard
}

mod r1842000_a_layered_summon_stacks_its_units_in_one_zone_bottom_first {
    use super::*;

    #[test]
    fn r1842000_the_pile_is_summoned_bottom_first_and_only_its_top_acts() {
        let mut s = board("r1842000-pile", json!({ "hand": [pile.id] }), json!({}));
        s.play(pile.id.as_str(), json!({}));

        assert_eq!(pile_defs(&s, P1, 1), the_pile());
        assert_eq!(
            summoned(s.last_events()),
            vec![
                (bottom.id.clone(), 1),
                (middle.id.clone(), 1),
                (guard.id.clone(), 1)
            ],
            "each layer is its own summon, bottom first"
        );
        let top = s.unit(P1, 1).expect("the pile's top");
        assert_eq!(top.def_id, guard.id);
        // Rule 7: both seats see the top and how many lie beneath, never what they are.
        for unit in [
            s.view(P1).you.units[0].clone(),
            s.view(P2).opponent.units[0].clone(),
        ] {
            let unit = unit.expect("the pile in view");
            assert_eq!(unit.instance_id, top.id);
            assert_eq!(unit.buried, 2);
        }
        let turn = s.state().turn;
        for card in pile_cards(&s, P1, 1) {
            assert_eq!(
                card.summoned_turn,
                Some(turn),
                "{} is summoning sick",
                card.def_id
            );
            assert!(!card.radiant, "{} is base", card.def_id);
        }
        assert!(s.unit(P1, 2).is_none(), "the pile takes one zone");
    }

    #[test]
    fn r1842000_the_pile_takes_the_leftmost_open_zone_and_a_full_row_summons_nothing() {
        let mut s = board(
            "r1842000-leftmost",
            json!({ "hand": [pile.id], "field": [plain.id, plain.id] }),
            json!({}),
        );
        s.play(pile.id.as_str(), json!({}));
        assert_eq!(pile_defs(&s, P1, 3), the_pile());

        let mut s = board(
            "r1842000-full",
            json!({ "hand": [pile.id], "field": [plain.id, plain.id, plain.id, plain.id, plain.id] }),
            json!({}),
        );
        s.play(pile.id.as_str(), json!({}));
        assert!(summoned(s.last_events()).is_empty(), "a full row summons nothing");
        for lane in 1..=5 {
            assert_eq!(pile_defs(&s, P1, lane), vec![plain.id.clone()], "lane {lane}");
        }
    }

    #[test]
    fn r1842000_a_named_lane_takes_the_pile_and_an_occupied_one_summons_nothing() {
        let mut s = board("r1842000-named", json!({ "hand": [lane_pile.id] }), json!({}));
        s.play(lane_pile.id.as_str(), json!({}));
        assert_eq!(pile_defs(&s, P1, 3), the_pile());
        assert!(s.unit(P1, 1).is_none());

        // R688: a named summon enters a Locked zone.
        let mut s = board(
            "r1842000-named-locked",
            json!({ "hand": [lane_pile.id] }),
            json!({}),
        );
        lock_zone(
            s.state_mut(),
            ZoneSlot {
                player: P1,
                row: Row::Units,
                lane: 3,
            },
        );
        s.play(lane_pile.id.as_str(), json!({}));
        assert_eq!(pile_defs(&s, P1, 3), the_pile());

        let mut s = board(
            "r1842000-named-taken",
            json!({ "hand": [lane_pile.id], "field": [{ "def": plain.id, "lane": 3 }] }),
            json!({}),
        );
        s.play(lane_pile.id.as_str(), json!({}));
        assert!(
            summoned(s.last_events()).is_empty(),
            "an occupied lane takes no pile"
        );
        assert_eq!(pile_defs(&s, P1, 3), vec![plain.id.clone()]);

        // R64: a zone held for a Reborn return takes nothing either.
        let mut s = board(
            "r1842000-named-reserved",
            json!({ "hand": [lane_pile.id] }),
            json!({}),
        );
        reserve_zone(
            s.state_mut(),
            ZoneSlot {
                player: P1,
                row: Row::Units,
                lane: 3,
            },
        );
        s.play(lane_pile.id.as_str(), json!({}));
        assert!(
            summoned(s.last_events()).is_empty(),
            "a reserved lane takes no pile"
        );
        assert!(pile_defs(&s, P1, 3).is_empty());
    }

    #[test]
    fn r1842000_with_no_open_zone_the_pile_enters_the_leftmost_empty_locked_one() {
        let mut s = board(
            "r1842000-locked",
            json!({ "hand": [pile.id], "field": [plain.id, plain.id, plain.id, plain.id] }),
            json!({}),
        );
        lock_zone(
            s.state_mut(),
            ZoneSlot {
                player: P1,
                row: Row::Units,
                lane: 5,
            },
        );
        s.play(pile.id.as_str(), json!({}));
        assert_eq!(pile_defs(&s, P1, 5), the_pile());
    }

    #[test]
    fn r1842000_the_radiant_face_makes_every_layer_radiant() {
        let mut s = board(
            "r1842000-radiant",
            json!({ "hand": [{ "def": pile.id, "radiant": true }] }),
            json!({}),
        );
        s.play(pile.id.as_str(), json!({}));
        let cards = pile_cards(&s, P1, 1);
        assert_eq!(cards.len(), 3);
        assert!(cards.iter().all(|card| card.radiant), "every layer is Radiant");
    }

    #[test]
    fn r1842000_a_card_that_is_no_unit_is_passed_over() {
        let mut s = board("r1842000-mixed", json!({ "hand": [mixed_pile.id] }), json!({}));
        s.play(mixed_pile.id.as_str(), json!({}));
        assert_eq!(pile_defs(&s, P1, 1), vec![guard.id.clone(), bottom.id.clone()]);
        assert!(s.backrow(P1, 1).is_none(), "the Trap is set nowhere");
        assert!(
            !summoned(s.last_events())
                .iter()
                .any(|(def_id, _)| *def_id == trap.id),
            "the Trap is not summoned"
        );
    }

    #[test]
    fn r1842000_an_all_effect_reaches_only_the_top_and_each_layer_resumes_in_turn() {
        let mut s = board(
            "r1842000-wipe",
            json!({ "hand": [pile.id] }),
            json!({ "hand": [wipe.id, wipe.id] }),
        );
        s.play(pile.id.as_str(), json!({}));
        let ids = pile_ids(&s, P1, 1);
        s.end_turn();

        s.play(wipe.id.as_str(), json!({}));
        assert!(in_graveyard(&s, &ids[0]), "the top was destroyed");
        assert_eq!(pile_defs(&s, P1, 1), vec![middle.id.clone(), bottom.id.clone()]);
        assert_eq!(s.card(ids[1].as_str()).damage, 0, "the middle resumes untouched");

        s.play(wipe.id.as_str(), json!({}));
        assert!(in_graveyard(&s, &ids[1]));
        assert_eq!(pile_defs(&s, P1, 1), vec![bottom.id.clone()]);
    }

    #[test]
    fn r1842000_a_reaction_to_a_lower_layer_s_summon_finds_it_dormant() {
        let mut s = board(
            "r1842000-watch",
            json!({ "hand": [pile.id], "field": [watcher.id] }),
            json!({}),
        );
        s.play(pile.id.as_str(), json!({}));
        assert_eq!(pile_defs(&s, P1, 2), the_pile());
        assert_eq!(summoned(s.last_events()).len(), 3);
        let cards = pile_cards(&s, P1, 2);
        assert_eq!(
            cards[0].buffs,
            AttackHealth { attack: 1, health: 1 },
            "the top took its buff"
        );
        for card in &cards[1..] {
            assert_eq!(
                card.buffs.attack, 0,
                "{} was dormant when its buff came",
                card.def_id
            );
        }
    }

    #[test]
    fn r1842000_a_question_a_layer_asks_on_arrival_pauses_the_rest_onto_the_same_zone() {
        let mut s = board("r1842000-ask", json!({ "hand": [asking_pile.id] }), json!({}));
        s.play(asking_pile.id.as_str(), json!({}));
        assert!(s.state().pending.is_some(), "the asker asks as it arrives (R151)");
        assert_eq!(pile_defs(&s, P1, 1), vec![asker.id.clone()]);
        // §9.3: the paused pile is data.
        let round: GameState =
            serde_json::from_str(&serde_json::to_string(s.state()).expect("serialises")).expect("parses");
        assert_eq!(&round, s.state());

        s.answer(json!("left"));
        assert!(s.state().pending.is_none());
        assert_eq!(
            pile_defs(&s, P1, 1),
            vec![guard.id.clone(), middle.id.clone(), asker.id.clone()],
            "the rest resumes onto the asker's zone (R113)"
        );
        assert!(s.unit(P1, 2).is_none());
    }
}

mod r1842001_a_lethal_guard_in_a_pile_guards_only_while_it_is_the_top {
    use super::*;

    #[test]
    fn r1842001_the_guard_on_top_of_a_pile_catches_a_lethal_hit_on_a_friendly_ally() {
        let mut s = board(
            "r1842001-ally",
            json!({ "hand": [pile.id], "field": [ally.id] }),
            json!({ "hand": [bolt.id] }),
        );
        s.play(pile.id.as_str(), json!({}));
        let ally_id = s.unit(P1, 1).expect("the ally").id;
        let guard_id = s.unit(P1, 2).expect("the guard").id;
        s.end_turn();

        s.play(bolt.id.as_str(), pick(&ally_id));
        assert_eq!(
            redirected(s.last_events()),
            vec![(ally_id.clone(), guard_id.clone())]
        );
        assert_eq!(s.card(ally_id.as_str()).damage, 0);
        assert_eq!(s.card(guard_id.as_str()).damage, BOLT - GUARD_ARMOR);
    }

    #[test]
    fn r1842001_the_guard_on_top_catches_a_lethal_hit_on_its_hero_and_the_layer_beneath_resumes() {
        let mut s = board(
            "r1842001-hero",
            json!({ "hand": [pile.id], "health": 20 }),
            json!({ "hand": [smite.id] }),
        );
        s.play(pile.id.as_str(), json!({}));
        let ids = pile_ids(&s, P1, 1);
        s.end_turn();

        s.play(
            smite.id.as_str(),
            json!({ "targets": [{ "pick": "hero", "player": "p1" }] }),
        );
        assert_eq!(
            redirected(s.last_events()),
            vec![(String::from("hero-p1"), ids[0].clone())]
        );
        assert_eq!(s.state().players[P1].hero.health, 20);
        assert!(in_graveyard(&s, &ids[0]), "the guard took the hit and died");
        assert_eq!(pile_defs(&s, P1, 1), vec![middle.id.clone(), bottom.id.clone()]);
        assert_eq!(s.card(ids[1].as_str()).damage, 0, "the middle resumes untouched");
    }

    #[test]
    fn r1842001_a_lethal_hit_on_the_guard_itself_kills_it_and_leaves_the_layer_beneath_untouched() {
        let mut s = board(
            "r1842001-self",
            json!({ "hand": [pile.id] }),
            json!({ "hand": [smite.id] }),
        );
        s.play(pile.id.as_str(), json!({}));
        let ids = pile_ids(&s, P1, 1);
        let health = s.state().players[P1].hero.health;
        s.end_turn();

        s.play(smite.id.as_str(), pick(&ids[0]));
        assert!(
            redirected(s.last_events()).is_empty(),
            "R460: never its own to catch"
        );
        assert!(in_graveyard(&s, &ids[0]));
        assert_eq!(s.unit(P1, 1).expect("the middle").id, ids[1]);
        assert_eq!(s.card(ids[1].as_str()).damage, 0);
        assert_eq!(s.state().players[P1].hero.health, health);
    }

    #[test]
    fn r1842001_a_guard_beneath_the_top_catches_nothing_until_it_resumes() {
        let mut s = board(
            "r1842001-beneath",
            json!({
                "field": [
                    { "def": ally.id, "lane": 1 },
                    { "def": guard.id, "lane": 2 },
                    { "def": middle.id, "stack": true },
                    { "def": ally.id, "lane": 3 },
                ],
            }),
            json!({ "hand": [bolt.id, bolt.id, bolt.id] }),
        );
        let first = s.unit(P1, 1).expect("the first ally").id;
        let ids = pile_ids(&s, P1, 2);
        assert_eq!(pile_defs(&s, P1, 2), vec![middle.id.clone(), guard.id.clone()]);
        let last = s.unit(P1, 3).expect("the second ally").id;
        s.end_turn();

        s.play(bolt.id.as_str(), pick(&first));
        assert!(
            redirected(s.last_events()).is_empty(),
            "a dormant guard catches nothing"
        );
        assert!(in_graveyard(&s, &first));
        assert_eq!(s.card(ids[1].as_str()).damage, 0);

        s.play(bolt.id.as_str(), pick(&ids[0]));
        assert!(
            redirected(s.last_events()).is_empty(),
            "nor a hit on the card above it"
        );
        assert_eq!(s.unit(P1, 2).expect("the guard").id, ids[1], "the guard resumes");

        s.play(bolt.id.as_str(), pick(&last));
        assert_eq!(redirected(s.last_events()), vec![(last.clone(), ids[1].clone())]);
        assert_eq!(s.card(ids[1].as_str()).damage, BOLT - GUARD_ARMOR);
    }
}
