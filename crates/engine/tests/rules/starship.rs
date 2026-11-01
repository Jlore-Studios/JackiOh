// R1852000–R1852007 (issue #1852): the Starship, Stinkyscape's mechanic ahead of its cards (SPEC §6.1
// Starship Piece and Starship, §6.2 When launched, §6.3 Launch). A Starship Piece that dies fuses a copy
// of itself into its owner's Dormant Starship, summoning one first (`starship::fuse_dead_piece`, the
// death pass's last step); the Dormant Starship's one ability is the engine's launch, Activate: (3),
// whose awakening fires its fused Cries and then its fused "When launched"; it remembers its pieces; and
// a Stack card played onto it is exiled before it resolves. Proved on the fixtures of
// `fixtures/starship.rs`, the Dormant Starship among them under the id the engine summons.

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::types_of;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::starship::*;

/// p1's or p2's turn 9, its main phase, with the given hands and fields; both libraries hold plain
/// cards to draw, so no turn ends by itself (§2.5).
fn board(active: &str, p1_hand: Value, p1_field: Value, p2_hand: Value, p2_field: Value) -> Scenario {
    board_with_backrow(active, p1_hand, p1_field, json!([]), p2_hand, p2_field)
}

/// `board`, with p1's backrow set as given.
fn board_with_backrow(
    active: &str,
    p1_hand: Value,
    p1_field: Value,
    p1_backrow: Value,
    p2_hand: Value,
    p2_field: Value,
) -> Scenario {
    register_starship_fixtures();
    let library = json!([plain.id, plain.id, plain.id, plain.id, plain.id, plain.id]);
    scenario(json!({
        "seed": "r1852000",
        "active": active,
        "p1": { "hand": p1_hand, "field": p1_field, "backrow": p1_backrow, "library": library },
        "p2": { "hand": p2_hand, "field": p2_field, "library": library },
    }))
}

fn id_at(s: &Scenario, player: PlayerId, lane: i32) -> String {
    s.unit(player, lane)
        .unwrap_or_else(|| panic!("no unit at {player:?}'s lane {lane}"))
        .id
}

/// R2249000: the id of the card in `player`'s backrow zone `lane`, where a Dormant Starship stands first.
fn backrow_id(s: &Scenario, player: PlayerId, lane: i32) -> String {
    s.backrow(player, lane)
        .unwrap_or_else(|| panic!("nothing in {player:?}'s backrow lane {lane}"))
        .id
}

/// A countdown Trap in each of a backrow's zones, so a Starship finds none of them open (R2249000).
fn full_backrow() -> Value {
    json!(
        (1..=BACKROW_ZONES)
            .map(|lane| json!({ "def": countdown.id, "lane": lane }))
            .collect::<Vec<_>>()
    )
}

/// R2249000: the Starship `id` as `side` of a view shows it, a Unit in a unit zone or standing in a
/// backrow zone.
fn ship_unit<'a>(side: &'a SideView, id: &str) -> &'a jackioh_engine::wire::UnitView {
    side.units
        .iter()
        .flatten()
        .chain(side.backrow.iter().flatten().filter_map(|card| match card {
            BackrowView::Public(card) => card.unit.as_ref(),
            BackrowView::FaceDown(_) => None,
        }))
        .find(|unit| unit.instance_id == id)
        .unwrap_or_else(|| panic!("the view shows no Starship {id}"))
}

fn pick(id: &str) -> Value {
    json!({ "targets": [{ "pick": "instance", "instanceId": id }] })
}

fn bolt_at(s: &mut Scenario, id: &str) {
    s.play(bolt.id.as_str(), pick(id));
}

fn ship_of(s: &Scenario, player: PlayerId) -> CardInstance {
    dormant_starship_of(s.state(), player)
        .cloned()
        .unwrap_or_else(|| panic!("{player:?} has no Dormant Starship"))
}

fn piece_defs(card: &CardInstance) -> Vec<String> {
    starship_pieces_of(card)
        .into_iter()
        .map(|piece| piece.def_id)
        .collect()
}

fn count_of(events: &[GameEvent], kind: GameEventType) -> usize {
    events.iter().filter(|event| event.event_type() == kind).count()
}

fn keywords_of(s: &Scenario, id: &str) -> Vec<KeywordKind> {
    s.stats(id).keywords.iter().map(Keyword::kind).collect()
}

fn health_of(s: &Scenario, player: PlayerId) -> i32 {
    s.state().players[player].hero.health
}

fn launch_action(id: &str) -> ActionBody {
    ActionBody::Activate {
        instance_id: id.to_string(),
        ability: Some(STARSHIP_LAUNCH_ABILITY.to_string()),
        targets: None,
        modes: None,
        tributes: None,
    }
}

fn offers_launch(s: &Scenario, player: PlayerId, id: &str) -> bool {
    legal_actions(s.state(), player)
        .iter()
        .any(|action| *action == launch_action(id))
}

fn launch(s: &mut Scenario, id: &str) {
    s.activate(id, json!({ "ability": STARSHIP_LAUNCH_ABILITY }));
}

/// p1's Dormant Starship holding the booster (its lane-1 piece, killed by a bolt) and the gunner (its
/// lane-2 piece, killed by a quake that also nicks p2's plain); the Starship stands in lane 1.
fn built_ship(p1_hand: Value, mana: Option<i32>) -> Scenario {
    built_ship_with(p1_hand, mana, json!([]))
}

/// `built_ship`, with p2's backrow set (face-down) as given.
fn built_ship_with(p1_hand: Value, mana: Option<i32>, p2_backrow: Value) -> Scenario {
    register_starship_fixtures();
    let library = json!([plain.id, plain.id, plain.id, plain.id, plain.id, plain.id]);
    let mut hand = vec![json!(bolt.id), json!(quake.id)];
    hand.extend(p1_hand.as_array().cloned().unwrap_or_default());
    let mut p1 = json!({
        "hand": hand,
        "field": [{ "def": booster.id, "lane": 1 }, { "def": gunner.id, "lane": 2 }],
        "library": library,
    });
    if let Some(mana) = mana {
        p1["mana"] = json!(mana);
    }
    let mut s = scenario(json!({
        "seed": "r1852003",
        "active": "p1",
        "p1": p1,
        "p2": {
            "hand": [plain.id],
            "field": [{ "def": plain.id, "lane": 1 }],
            "backrow": p2_backrow,
            "library": library,
        },
    }));
    let booster_id = id_at(&s, P1, 1);
    bolt_at(&mut s, &booster_id);
    s.play(quake.id.as_str(), json!({}));
    s
}

/// `id`, taken out of p1's graveyard into their hand (`ZoneName::Hand`) or their library (anything else),
/// as a return or a shuffle takes it: R97 judges a card by where it is now.
fn out_of_the_graveyard(s: &mut Scenario, id: &str, to: ZoneName) {
    let side = &mut s.state_mut().players.p1;
    let at = side
        .graveyard
        .iter()
        .position(|card| card.id == id)
        .unwrap_or_else(|| panic!("{id} is not in p1's graveyard"));
    let mut card = side.graveyard.remove(at);
    if to == ZoneName::Hand {
        card.zone = Zone::Hand { player: P1 };
        side.hand.push(card);
    } else {
        card.zone = Zone::Library { player: P1 };
        side.library.push(card);
    }
}

/// The `starshipFused` and `starshipLaunched` events on `viewer`'s view, as the wire carries them.
fn starship_events(s: &Scenario, viewer: PlayerId) -> Vec<Value> {
    s.view(viewer)
        .events
        .iter()
        .filter(|event| {
            matches!(
                event.event_type(),
                GameEventType::StarshipFused | GameEventType::StarshipLaunched
            )
        })
        .map(|event| serde_json::to_value(event).expect("an event serializes"))
        .collect()
}

mod r1852000_a_piece_that_dies_fuses_into_its_owners_dormant_starship {
    use super::*;

    #[test]
    fn r1852000_r2249000_a_dying_piece_summons_a_dormant_starship_into_the_middle_of_the_backrow_and_fuses_into_it()
     {
        let mut s = board(
            "p1",
            json!([bolt.id]),
            json!([{ "def": plain.id, "lane": 1 }, { "def": booster.id, "lane": 2 }]),
            json!([plain.id]),
            json!([]),
        );
        let piece = id_at(&s, P1, 2);
        assert!(!building_a_ship(s.state(), P1));
        bolt_at(&mut s, &piece);
        assert_eq!(s.card(&piece).zone.z(), ZoneName::Graveyard);
        // The Starship stands in the middle of the backrow (R2249000), Dormant, holding the piece.
        let ship = ship_of(&s, P1);
        assert_eq!(backrow_id(&s, P1, STARSHIP_LANE_ORDER[0]), ship.id);
        assert_eq!(
            ship.slumber,
            Some(Slumber { turns: None }),
            "the Starship is Dormant"
        );
        assert_eq!(piece_defs(&ship), vec![booster.id.clone()]);
        assert!(building_a_ship(s.state(), P1));
        assert!(!building_a_ship(s.state(), P2));
        // It starts 0/0, so its stats are the piece's, and it holds the piece's tag, not its keyword.
        s.expect_stats(ship.id.as_str(), json!({ "attack": 2, "health": 3 }));
        assert!(!keywords_of(&s, &ship.id).contains(&KeywordKind::StarshipPiece));
        assert!(tags_of(s.state(), &ship).contains(&Tag::Human));
        // It is no Unit for the rules that read Units (R1839000).
        assert!(
            active_units_of(s.state(), P1)
                .iter()
                .all(|unit| unit.id != ship.id)
        );
        // Reported in that order: the death, the graveyard, the summon, the Fuse and the fusion.
        let events = types_of(s.last_events());
        let at = |kind: GameEventType| {
            events
                .iter()
                .position(|each| *each == kind)
                .unwrap_or_else(|| panic!("no {kind:?} in {events:?}"))
        };
        assert!(at(GameEventType::Destroyed) < at(GameEventType::EnteredGraveyard));
        assert!(at(GameEventType::EnteredGraveyard) < at(GameEventType::Summoned));
        assert!(at(GameEventType::Summoned) < at(GameEventType::Fused));
        assert!(at(GameEventType::Fused) < at(GameEventType::StarshipFused));
        // Both seats see the Starship and what is fused into it.
        for viewer in [P1, P2] {
            let view = s.view(viewer);
            let side = if viewer == P1 { &view.you } else { &view.opponent };
            let unit = ship_unit(side, &ship.id);
            let memory = unit
                .starship
                .as_ref()
                .expect("the Starship's memory is on the view");
            assert_eq!(memory.pieces.len(), 1, "{viewer:?}");
            assert_eq!(memory.pieces[0].def_id, booster.id, "{viewer:?}");
            assert_eq!(memory.name, None, "{viewer:?}");
        }
    }

    #[test]
    fn r1852000_two_pieces_dying_at_once_fuse_into_one_starship_in_lane_order() {
        let mut s = board(
            "p1",
            json!([quake.id]),
            json!([{ "def": gunner.id, "lane": 1 }, { "def": plain.id, "lane": 2 }, { "def": gunner.id, "lane": 3 }]),
            json!([plain.id]),
            json!([]),
        );
        s.play(quake.id.as_str(), json!({}));
        let ship = ship_of(&s, P1);
        assert_eq!(
            backrow_id(&s, P1, STARSHIP_LANE_ORDER[0]),
            ship.id,
            "the middle of the backrow (R2249000)"
        );
        assert!(s.unit(P1, 3).is_none(), "one Starship, not two");
        assert_eq!(piece_defs(&ship), vec![gunner.id.clone(), gunner.id.clone()]);
        s.expect_stats(ship.id.as_str(), json!({ "attack": 2, "health": 2 }));
        assert_eq!(count_of(s.last_events(), GameEventType::StarshipFused), 2);
        let summoned_ships = s
            .last_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Summoned { def_id, .. } if def_id == DORMANT_STARSHIP_DEF_ID))
            .count();
        assert_eq!(summoned_ships, 1);
    }

    #[test]
    fn r1852000_r2249000_a_piece_killed_while_both_its_owners_rows_are_full_stacks_the_starship_onto_lane_3_locked_or_not()
     {
        let mut s = board_with_backrow(
            "p1",
            json!([bolt.id]),
            json!([
                { "def": plain.id, "lane": 1 },
                { "def": plain.id, "lane": 2 },
                { "def": plain.id, "lane": 3 },
                { "def": plain.id, "lane": 4 },
                { "def": phoenix.id, "lane": 5 },
            ]),
            full_backrow(),
            json!([plain.id]),
            json!([]),
        );
        let beneath = id_at(&s, P1, 3);
        let slot = ZoneSlot {
            player: P1,
            row: Row::Units,
            lane: STARSHIP_FULL_ROW_LANE,
        };
        lock_zone(s.state_mut(), slot);
        let phoenix_id = id_at(&s, P1, 5);
        bolt_at(&mut s, &phoenix_id);
        // Reborn brought the piece back to its zone first, so the row is full again.
        assert_eq!(id_at(&s, P1, 5), phoenix_id);
        // The Starship tops lane 3, Locked as it is, with the plain beneath it.
        let ship = ship_of(&s, P1);
        assert_eq!(id_at(&s, P1, STARSHIP_FULL_ROW_LANE), ship.id);
        assert!(is_locked(s.state(), slot));
        let pile = pile_at(s.state(), slot).expect("lane 3's pile").clone();
        assert_eq!(pile.len(), 2);
        assert_eq!(pile[1].id, beneath, "the plain lies beneath the Starship");
        assert_eq!(piece_defs(&ship), vec![phoenix.id.clone()]);
        s.expect_stats(ship.id.as_str(), json!({ "attack": 1, "health": 1 }));
    }

    #[test]
    fn r1852000_a_discard_fuses_nothing() {
        let mut s = board(
            "p1",
            json!([dump.id, booster.id, gunner.id]),
            json!([]),
            json!([plain.id]),
            json!([]),
        );
        s.play(dump.id.as_str(), json!({}));
        let graveyard: Vec<String> = s
            .pile(P1, "graveyard")
            .into_iter()
            .map(|card| card.def_id)
            .collect();
        assert!(graveyard.contains(&booster.id) && graveyard.contains(&gunner.id));
        assert!(!building_a_ship(s.state(), P1), "no piece died, so no Starship");
        assert_eq!(count_of(s.last_events(), GameEventType::StarshipFused), 0);
        assert!((1..=5).all(|lane| s.unit(P1, lane).is_none()));
    }

    #[test]
    fn r1852000_one_starship_at_a_time_it_is_no_token_and_after_a_launch_the_next_piece_starts_a_new_one() {
        let mut s = built_ship(json!([bolt.id, bolt.id, booster.id]), None);
        let first = ship_of(&s, P1);
        assert_eq!(piece_defs(&first), vec![booster.id.clone(), gunner.id.clone()]);
        assert!(s.unit(P1, 2).is_none(), "the gunner fused into the one Starship");
        launch(&mut s, &first.id);
        let enemy = id_at(&s, P2, 1);
        s.answer(json!([{ "pick": "instance", "instanceId": enemy }]));
        // The launched Starship stands awake; the next piece to die starts a new one.
        s.play(booster.id.as_str(), json!({ "zone": 2 }));
        let piece = id_at(&s, P1, 2);
        bolt_at(&mut s, &piece);
        let second = ship_of(&s, P1);
        assert_ne!(second.id, first.id);
        assert_eq!(piece_defs(&second), vec![booster.id.clone()]);
        assert!(is_launched_starship(s.card(&first.id)));
        // The launched Starship is no token: killed, it reaches its owner's graveyard.
        bolt_at(&mut s, &first.id);
        assert_eq!(s.card(&first.id).zone.z(), ZoneName::Graveyard);
    }

    #[test]
    fn r1852000_a_piece_returned_to_a_hand_is_hidden_on_its_starship_fused_event() {
        let mut s = board(
            "p1",
            json!([bolt.id]),
            json!([{ "def": booster.id, "lane": 1 }]),
            json!([plain.id]),
            json!([]),
        );
        let piece = id_at(&s, P1, 1);
        bolt_at(&mut s, &piece);
        let ship = ship_of(&s, P1);
        out_of_the_graveyard(&mut s, &piece, ZoneName::Hand);
        // The other seat reads the fusion, the Starship and its count, but not the piece now in a hand.
        assert_eq!(
            starship_events(&s, P2)[0],
            json!({
                "type": "starshipFused",
                "player": "p1",
                "instanceId": ship.id,
                "pieceId": HIDDEN_ID,
                "defId": HIDDEN_ID,
                "pieces": 1,
            })
        );
        // Its owner reads its own hand.
        let own = &starship_events(&s, P1)[0];
        assert_eq!(own["pieceId"], json!(piece));
        assert_eq!(own["defId"], json!(booster.id));
        assert_eq!(
            create_invariant_monitor(s.state()).hidden(s.state()),
            Vec::<String>::new()
        );
    }

    #[test]
    fn r1852005_a_piece_shuffled_into_a_library_is_still_named_by_the_starships_memory() {
        let mut s = board(
            "p1",
            json!([bolt.id]),
            json!([{ "def": booster.id, "lane": 1 }]),
            json!([plain.id]),
            json!([]),
        );
        let piece = id_at(&s, P1, 1);
        bolt_at(&mut s, &piece);
        let ship = ship_of(&s, P1);
        out_of_the_graveyard(&mut s, &piece, ZoneName::Library);
        // The log keeps the last NONCE_HISTORY actions (`reduce::remember_nonce`): once the fusion has
        // left it, only the Starship's memory names the piece's definition.
        s.state_mut().applied.clear();
        let view = s.view(P2);
        let unit = ship_unit(&view.opponent, &ship.id);
        assert_eq!(unit.instance_id, ship.id);
        let memory = unit
            .starship
            .as_ref()
            .expect("the Starship's memory is on the view");
        assert_eq!(memory.pieces[0].def_id, booster.id);
        // The piece died face-up, so its definition is no secret, wherever the card went since.
        assert_eq!(
            create_invariant_monitor(s.state()).hidden(s.state()),
            Vec::<String>::new()
        );
    }
}

mod r1852001_equip_does_not_pass_the_keyword_on {
    use super::*;

    #[test]
    fn r1852001_a_piece_equipped_onto_a_unit_that_is_no_piece_leaves_it_no_piece() {
        let mut s = board(
            "p1",
            json!([plate.id, bolt.id]),
            json!([{ "def": plain.id, "lane": 1 }]),
            json!([plain.id]),
            json!([]),
        );
        let host = id_at(&s, P1, 1);
        s.play(plate.id.as_str(), json!({ "zone": 1, "equip": true }));
        assert_eq!(id_at(&s, P1, 1), host, "the host is the kept card");
        assert!(keywords_of(&s, &host).contains(&KeywordKind::Equip));
        assert!(
            !keywords_of(&s, &host).contains(&KeywordKind::StarshipPiece),
            "Equip does not pass Starship Piece on"
        );
        s.expect_stats(host.as_str(), json!({ "attack": 4, "health": 5 }));
        bolt_at(&mut s, &host);
        assert_eq!(s.card(&host).zone.z(), ZoneName::Graveyard);
        assert!(!building_a_ship(s.state(), P1), "it died as no piece");
    }

    #[test]
    fn r1852001_a_unit_equipped_onto_a_piece_keeps_it_a_piece() {
        let mut s = board(
            "p1",
            json!([clamp.id, bolt.id]),
            json!([{ "def": booster.id, "lane": 1 }]),
            json!([plain.id]),
            json!([]),
        );
        let host = id_at(&s, P1, 1);
        s.play(clamp.id.as_str(), json!({ "zone": 1, "equip": true }));
        assert!(keywords_of(&s, &host).contains(&KeywordKind::StarshipPiece));
        bolt_at(&mut s, &host);
        // R2521000: the booster alone fuses, its base version, never the clamp equipped onto it.
        let ship = ship_of(&s, P1);
        s.expect_stats(ship.id.as_str(), json!({ "attack": 2, "health": 3 }));
        assert!(!keywords_of(&s, &ship.id).contains(&KeywordKind::StarshipPiece));
    }
}

/// R1878001: an Embiggened version's lent Starship Piece follows R1852001 through a Fuse: the card kept
/// keeps its own lend, as a grant, and an ingredient's reaches no other card.
mod r1878001_a_lent_piece_through_a_fuse {
    use super::*;

    #[test]
    fn r1878001_a_lent_piece_equipped_onto_a_host_played_embiggened_leaves_it_no_piece() {
        let mut s = board(
            "p1",
            json!([brute.id, lender.id, bolt.id]),
            json!([]),
            json!([plain.id]),
            json!([]),
        );
        s.play(brute.id.as_str(), json!({ "zone": 1, "embiggen": true }));
        let host = id_at(&s, P1, 1);
        assert!(!keywords_of(&s, &host).contains(&KeywordKind::StarshipPiece));
        s.play(
            lender.id.as_str(),
            json!({ "zone": 1, "equip": true, "embiggen": true }),
        );
        assert_eq!(id_at(&s, P1, 1), host, "the host is the kept card");
        assert!(
            !keywords_of(&s, &host).contains(&KeywordKind::StarshipPiece),
            "the lend does not reach the host, though it too was played Embiggened"
        );
        bolt_at(&mut s, &host);
        assert!(!building_a_ship(s.state(), P1), "it died as no piece");
    }

    #[test]
    fn r1878001_a_card_that_lent_itself_the_piece_keeps_it_as_a_grant_when_a_card_is_equipped_onto_it() {
        let mut s = board(
            "p1",
            json!([lender.id, clamp.id, bolt.id]),
            json!([]),
            json!([plain.id]),
            json!([]),
        );
        s.play(lender.id.as_str(), json!({ "zone": 1, "embiggen": true }));
        let host = id_at(&s, P1, 1);
        assert!(keywords_of(&s, &host).contains(&KeywordKind::StarshipPiece));
        s.play(clamp.id.as_str(), json!({ "zone": 1, "equip": true }));
        assert_eq!(id_at(&s, P1, 1), host, "the lender is the kept card");
        assert!(keywords_of(&s, &host).contains(&KeywordKind::StarshipPiece));
        assert!(s.card(&host).granted_keywords.contains(&Keyword::StarshipPiece));
        bolt_at(&mut s, &host);
        assert_eq!(piece_defs(&ship_of(&s, P1)).len(), 1, "it died as a piece");
    }
}

mod r1852002_a_stolen_piece {
    use super::*;

    #[test]
    fn r1852002_r2618000_a_stolen_piece_that_dies_fuses_into_its_controllers_starship_not_its_owners() {
        let mut s = board(
            "p2",
            json!([plain.id]),
            json!([{ "def": booster.id, "lane": 1 }]),
            json!([grab.id, bolt.id]),
            json!([]),
        );
        let piece = id_at(&s, P1, 1);
        s.play(grab.id.as_str(), json!({}));
        assert_eq!(s.card(&piece).controller, P2);
        assert_eq!(s.card(&piece).owner, P1);
        s.play(bolt.id.as_str(), pick(&piece));
        // R2618000: it died on p2's side, into p2's graveyard as p2's own card.
        assert_eq!(s.card(&piece).zone, Zone::Graveyard { player: P2 });
        assert_eq!(s.card(&piece).owner, P2);
        assert!(!building_a_ship(s.state(), P1), "its old owner builds no ship");
        let ship = ship_of(&s, P2);
        assert_eq!(
            backrow_id(&s, P2, STARSHIP_LANE_ORDER[0]),
            ship.id,
            "its controller's, on its controller's side"
        );
        assert_eq!(ship.controller, P2);
        assert_eq!(piece_defs(&ship), vec![booster.id.clone()]);
        assert!(s.last_events().iter().any(|event| matches!(
            event,
            GameEvent::StarshipFused { player: P2, piece_id, .. } if *piece_id == piece
        )));
    }
}

mod r1852003_the_launch {
    use super::*;

    #[test]
    fn r1852003_activate_3_launches_it_its_cries_fire_with_their_prompts_then_its_when_launched() {
        let mut s = built_ship(json!([]), None);
        let ship = ship_of(&s, P1);
        let enemy = id_at(&s, P2, 1);
        // Its one ability is the launch, offered, and named with its price on its controller's view.
        assert!(offers_launch(&s, P1, &ship.id));
        let view = s.view(P1);
        let unit = ship_unit(&view.you, &ship.id);
        let abilities = unit.activations.clone().expect("the launch is on the view");
        assert_eq!(abilities.len(), 1);
        assert_eq!(abilities[0].ability, STARSHIP_LAUNCH_ABILITY);
        assert_eq!(abilities[0].label, format!("Launch ({STARSHIP_LAUNCH_COST})"));
        assert!(abilities[0].usable);
        let mana = s.state().players.p1.mana.current;
        let health = health_of(&s, P2);
        launch(&mut s, &ship.id);
        assert_eq!(s.state().players.p1.mana.current, mana - STARSHIP_LAUNCH_COST);
        // Awake, it took the Starship keyword and a name, and the launch is reported after the awakening.
        let types = types_of(s.last_events());
        let awakened = types
            .iter()
            .position(|kind| *kind == GameEventType::Awakened)
            .expect("awakened");
        let launched = types
            .iter()
            .position(|kind| *kind == GameEventType::StarshipLaunched)
            .expect("starshipLaunched");
        assert!(awakened < launched);
        let card = s.card(&ship.id).clone();
        assert_eq!(card.slumber, None);
        assert!(keywords_of(&s, &ship.id).contains(&KeywordKind::Starship));
        assert!(
            card.starship
                .as_ref()
                .and_then(|memory| memory.name.clone())
                .is_some()
        );
        // The gunner's fused Cry asks its controller for its target, and the When launched waits for it.
        let pending = s.state().pending.clone().expect("the Cry's target prompt");
        assert_eq!(pending.player_id, P1);
        assert_eq!(pending.kind, PromptKind::Target);
        assert_eq!(health_of(&s, P2), health, "the When launched has not run yet");
        let damage_before = s.card(&enemy).damage;
        s.answer(json!([{ "pick": "instance", "instanceId": enemy }]));
        assert_eq!(s.card(&enemy).damage, damage_before + CRY_DAMAGE);
        assert_eq!(
            health_of(&s, P2),
            health - LAUNCHED_DAMAGE,
            "then the When launched"
        );
        let answer_events = types_of(s.last_events());
        let cry_hit = answer_events
            .iter()
            .position(|kind| *kind == GameEventType::Damage)
            .expect("the Cry's hit");
        assert!(
            answer_events
                .iter()
                .rposition(|kind| *kind == GameEventType::Damage)
                .is_some_and(|last| last > cry_hit),
            "the When launched hits after the Cry: {answer_events:?}"
        );
        assert!(!offers_launch(&s, P1, &ship.id));
        assert!(
            active_units_of(s.state(), P1)
                .iter()
                .any(|unit| unit.id == ship.id)
        );
    }

    #[test]
    fn r1852003_legal_actions_the_refusal_and_the_view_agree_on_a_launch_it_cannot_pay_for() {
        // A plain in hand keeps p1's turn from ending by itself (R82) once the launch is out of reach.
        let s = built_ship(json!([plain.id]), Some(2));
        let ship = ship_of(&s, P1);
        let mana = s.state().players.p1.mana.current;
        assert!(mana < STARSHIP_LAUNCH_COST, "{mana}");
        assert!(!offers_launch(&s, P1, &ship.id));
        let reason = format!("that ability costs {STARSHIP_LAUNCH_COST}, more than your mana");
        let result = reduce(
            s.state(),
            &ActionInput {
                body: launch_action(&ship.id),
                player_id: P1,
            }
            .with_nonce("too-dear"),
        );
        assert_eq!(result.error.as_deref(), Some(reason.as_str()));
        let view = s.view(P1);
        let unit = ship_unit(&view.you, &ship.id);
        let ability = &unit.activations.as_ref().expect("the launch is on the view")[0];
        assert!(!ability.usable);
        assert_eq!(ability.reason.as_deref(), Some(reason.as_str()));
        // Nor may the other seat launch it, and it is offered nothing else.
        assert!(!offers_launch(&s, P2, &ship.id));
    }

    #[test]
    fn r1852003_an_effect_that_awakens_it_launches_it_for_nothing() {
        let mut s = built_ship(json!([ignition.id]), None);
        let ship = ship_of(&s, P1);
        let mana = s.state().players.p1.mana.current;
        s.play(ignition.id.as_str(), json!({}));
        assert_eq!(s.state().players.p1.mana.current, mana);
        assert!(is_launched_starship(s.card(&ship.id)));
        assert_eq!(count_of(s.last_events(), GameEventType::StarshipLaunched), 1);
        assert!(
            s.state().pending.is_some(),
            "its Cry asks as it would after the Activate"
        );
    }

    #[test]
    fn r1852003_the_timeout_and_the_random_policy_answer_a_launchs_prompt() {
        let mut s = built_ship(json!([]), None);
        let ship = ship_of(&s, P1);
        launch(&mut s, &ship.id);
        let paused = s.state().clone();
        assert!(paused.pending.is_some());
        // The random policy answers it with an action the reducer takes.
        let mut rng = create_rng("r1852003-policy", 0);
        let action = subsystems::ai_policy::choose_action(&paused, P1, &mut rng).expect("an answer");
        assert!(matches!(action, ActionBody::Answer { .. }), "{action:?}");
        let answered = reduce(
            &paused,
            &ActionInput {
                body: action,
                player_id: P1,
            }
            .with_nonce("policy"),
        );
        assert_eq!(answered.error, None);
        assert!(answered.state.pending.is_none());
        // And the timeout answers it, then ends the turn.
        let timed_out = reduce(
            &paused,
            &ActionInput {
                body: ActionBody::Timeout,
                player_id: P1,
            }
            .with_nonce("timeout"),
        );
        assert_eq!(timed_out.error, None);
        assert!(timed_out.state.pending.is_none());
        assert!(
            timed_out
                .events
                .iter()
                .any(|event| event.event_type() == GameEventType::Damage),
            "the Cry ran with the drawn target"
        );
    }

    #[test]
    fn r1852003_a_piece_on_the_field_and_a_launched_starship_do_nothing_when_another_starship_launches() {
        // Each side's booster dies into a Starship; p1 also has a live booster on the field.
        let mut s = board(
            "p1",
            json!([bolt.id, booster.id]),
            json!([{ "def": booster.id, "lane": 1 }]),
            json!([bolt.id]),
            json!([{ "def": booster.id, "lane": 1 }]),
        );
        let mine = id_at(&s, P1, 1);
        bolt_at(&mut s, &mine);
        s.play(booster.id.as_str(), json!({ "zone": 2 }));
        let p2_health = health_of(&s, P2);
        let ship = ship_of(&s, P1);
        launch(&mut s, &ship.id);
        assert_eq!(
            health_of(&s, P2),
            p2_health - LAUNCHED_DAMAGE,
            "only the booster fused into the Starship answers its launch"
        );
        // p2 launches theirs: p1's live booster and p1's launched Starship answer nothing.
        s.end_turn();
        let theirs = id_at(&s, P2, 1);
        bolt_at(&mut s, &theirs);
        let p1_health = health_of(&s, P1);
        let p2_health = health_of(&s, P2);
        let ship = ship_of(&s, P2);
        launch(&mut s, &ship.id);
        assert_eq!(health_of(&s, P1), p1_health - LAUNCHED_DAMAGE);
        assert_eq!(
            health_of(&s, P2),
            p2_health,
            "no card of p1's answered p2's launch"
        );
    }

    #[test]
    fn r1852003_a_trap_reveals_when_either_player_launches_a_starship() {
        let mut s = built_ship_with(json!([]), None, json!([{ "def": countdown.id, "lane": 1 }]));
        let ship = ship_of(&s, P1);
        let health = health_of(&s, P1);
        let before = count_of(s.events(), GameEventType::TrapFired);
        launch(&mut s, &ship.id);
        if s.state().pending.is_some() {
            let enemy = id_at(&s, P2, 1);
            s.answer(json!([{ "pick": "instance", "instanceId": enemy }]));
        }
        assert_eq!(
            count_of(s.events(), GameEventType::TrapFired),
            before + 1,
            "the opponent's launch trap fired"
        );
        assert_eq!(health_of(&s, P1), health - TRAP_DAMAGE);
    }

    #[test]
    fn r1852003_r2259000_a_starship_summoned_and_launched_in_one_turn_is_sick_and_the_monitor_agrees() {
        let mut s = board(
            "p1",
            json!([bolt.id]),
            json!([{ "def": booster.id, "lane": 1 }]),
            json!([plain.id]),
            json!([]),
        );
        let mut monitor = create_invariant_monitor(s.state());
        let piece = id_at(&s, P1, 1);
        let from = s.events().len();
        bolt_at(&mut s, &piece);
        let ship = ship_of(&s, P1);
        assert_eq!(s.card(&ship.id).summoned_turn, Some(s.state().turn));
        assert_eq!(
            monitor.after(&s.events()[from..], s.state()),
            Vec::<String>::new()
        );
        let from = s.events().len();
        launch(&mut s, &ship.id);
        // R2259000: its awakening makes it sick for the rest of the turn, and the monitor's shadow reads
        // it so (I4).
        assert_eq!(s.card(&ship.id).summoned_turn, Some(s.state().turn));
        assert_eq!(
            monitor.after(&s.events()[from..], s.state()),
            Vec::<String>::new()
        );
        let attack = ActionBody::Attack {
            attacker_id: ship.id.clone(),
            target_id: "hero-p2".to_string(),
        };
        assert!(!legal_actions(s.state(), P1).contains(&attack));
        // I1 agrees no attack of it is offered; the scenario's own cards arrived with no event, which I3
        // reports.
        let found: Vec<String> = monitor
            .before(s.state(), P1, &ActionBody::EndTurn)
            .into_iter()
            .filter(|finding| !finding.starts_with("I3"))
            .collect();
        assert_eq!(found, Vec::<String>::new());
    }

    #[test]
    fn r1852003_a_starship_bounced_into_a_hand_is_hidden_on_its_fused_and_launched_events() {
        let mut s = board(
            "p1",
            json!([bolt.id, recall.id]),
            json!([{ "def": booster.id, "lane": 1 }]),
            json!([plain.id]),
            json!([]),
        );
        let piece = id_at(&s, P1, 1);
        bolt_at(&mut s, &piece);
        let ship = ship_of(&s, P1);
        launch(&mut s, &ship.id);
        s.play(recall.id.as_str(), json!({}));
        assert_eq!(s.card(&ship.id).zone.z(), ZoneName::Hand);
        // The other seat reads both events, but not the Starship now in p1's hand.
        let theirs = starship_events(&s, P2);
        assert_eq!(theirs.len(), 2, "{theirs:?}");
        assert!(
            theirs.iter().all(|event| event["instanceId"] == json!(HIDDEN_ID)),
            "{theirs:?}"
        );
        assert_eq!(theirs[1]["defId"], json!(HIDDEN_ID));
        // Its owner reads its own hand.
        assert!(
            starship_events(&s, P1)
                .iter()
                .all(|event| event["instanceId"] == json!(ship.id))
        );
        assert_eq!(
            create_invariant_monitor(s.state()).hidden(s.state()),
            Vec::<String>::new()
        );
    }

    #[test]
    fn r1852003_a_launch_names_its_pieces_after_the_starship_died_and_its_piece_went_back_to_a_library() {
        let mut s = board(
            "p1",
            json!([bolt.id, bolt.id]),
            json!([{ "def": booster.id, "lane": 1 }]),
            json!([plain.id]),
            json!([]),
        );
        let piece = id_at(&s, P1, 1);
        bolt_at(&mut s, &piece);
        let ship = ship_of(&s, P1);
        launch(&mut s, &ship.id);
        let launched_at = s.state().applied.len() - 1;
        bolt_at(&mut s, &ship.id);
        assert_eq!(s.card(&ship.id).zone.z(), ZoneName::Graveyard);
        assert!(
            s.card(&ship.id).starship.is_none(),
            "R78 takes the memory off with the card"
        );
        out_of_the_graveyard(&mut s, &piece, ZoneName::Library);
        // The log keeps the last NONCE_HISTORY actions (`reduce::remember_nonce`): once the fusion has
        // left it, only the launch names the piece's definition.
        s.state_mut().applied.drain(..launched_at);
        let launched: Vec<Value> = starship_events(&s, P2)
            .into_iter()
            .filter(|event| event["type"] == json!("starshipLaunched"))
            .collect();
        assert_eq!(launched.len(), 1, "{launched:?}");
        assert_eq!(launched[0]["pieces"], json!([booster.id]));
        assert_eq!(
            create_invariant_monitor(s.state()).hidden(s.state()),
            Vec::<String>::new()
        );
    }
}

mod r1852004_the_name {
    use super::*;

    #[test]
    fn r1852004_a_tagged_name_needs_its_tag_and_is_preferred_and_the_others_suit_any_ship() {
        assert_eq!(launch_names(&[Tag::Human]), vec!["Frontier's Promise"]);
        assert_eq!(
            launch_names(&[Tag::Felinor, Tag::Human]),
            vec!["The Ninth Life", "Frontier's Promise"]
        );
        assert_eq!(launch_names(&[]), vec!["Ember of Departure", "The Silent Choir"]);
        assert_eq!(
            launch_names(&[Tag::Fruit]),
            vec!["Ember of Departure", "The Silent Choir"],
            "a tag no name has is any ship's"
        );
        // The booster is Human, so the Starship holding it launches as Frontier's Promise.
        let mut s = built_ship(json!([]), None);
        let ship = ship_of(&s, P1);
        launch(&mut s, &ship.id);
        let name = s
            .card(&ship.id)
            .starship
            .as_ref()
            .and_then(|memory| memory.name.clone());
        assert_eq!(name.as_deref(), Some("Frontier's Promise"));
        assert!(s.last_events().iter().any(|event| matches!(
            event,
            GameEvent::StarshipLaunched { name, .. } if name == "Frontier's Promise"
        )));
    }

    #[test]
    fn r1852004_a_starship_with_no_tagged_piece_takes_a_name_for_any_ship() {
        let mut s = board(
            "p1",
            json!([quake.id]),
            json!([{ "def": gunner.id, "lane": 1 }]),
            json!([plain.id]),
            json!([]),
        );
        s.play(quake.id.as_str(), json!({}));
        let ship = ship_of(&s, P1);
        launch(&mut s, &ship.id);
        let name = s
            .card(&ship.id)
            .starship
            .as_ref()
            .and_then(|memory| memory.name.clone());
        assert!(
            matches!(name.as_deref(), Some("Ember of Departure" | "The Silent Choir")),
            "{name:?}"
        );
    }
}

mod r1852005_the_memory_and_building_a_ship {
    use super::*;

    #[test]
    fn r1852005_the_starship_remembers_its_pieces_and_a_card_reads_them_after_the_launch() {
        let mut s = built_ship(json!([salvage.id]), None);
        let ship = ship_of(&s, P1);
        assert!(building_a_ship(s.state(), P1));
        launch(&mut s, &ship.id);
        let enemy = id_at(&s, P2, 1);
        s.answer(json!([{ "pick": "instance", "instanceId": enemy }]));
        assert!(
            !building_a_ship(s.state(), P1),
            "a launched Starship is no ship being built"
        );
        assert_eq!(
            launched_starship_of(s.state(), P1).map(|card| card.id.clone()),
            Some(ship.id.clone())
        );
        let held_before = s.hand(P1).len();
        s.play(salvage.id.as_str(), json!({}));
        let hand: Vec<String> = s.hand(P1).into_iter().map(|card| card.def_id).collect();
        assert_eq!(hand.len(), held_before - 1 + 2);
        assert!(hand.contains(&booster.id) && hand.contains(&gunner.id));
        // Both seats read the memory and the name on the view.
        for viewer in [P1, P2] {
            let view = s.view(viewer);
            let side = if viewer == P1 { &view.you } else { &view.opponent };
            let unit = ship_unit(side, &ship.id);
            let memory = unit.starship.as_ref().expect("its memory");
            assert_eq!(memory.pieces.len(), 2, "{viewer:?}");
            assert!(memory.name.is_some(), "{viewer:?}");
        }
    }
}

mod r1852006_the_launch_price {
    use super::*;

    #[test]
    fn r1852006_your_starship_launch_costs_0_prices_the_next_launch_and_is_spent_by_it() {
        let mut s = built_ship(json!([liftoff.id, booster.id, bolt.id]), None);
        let ship = ship_of(&s, P1);
        s.play(liftoff.id.as_str(), json!({}));
        assert_eq!(launch_cost(s.state(), P1), 0);
        let view = s.view(P1);
        assert!(
            view.you
                .modifiers
                .iter()
                .any(|badge| badge.label == "Your Starship launch costs (0)")
        );
        let unit = ship_unit(&view.you, &ship.id);
        assert_eq!(
            unit.activations.as_ref().expect("the launch")[0].label,
            "Launch (0)"
        );
        let mana = s.state().players.p1.mana.current;
        launch(&mut s, &ship.id);
        assert_eq!(s.state().players.p1.mana.current, mana, "the launch cost nothing");
        assert_eq!(
            launch_cost(s.state(), P1),
            STARSHIP_LAUNCH_COST,
            "and spent the price"
        );
        assert!(
            !s.state()
                .players
                .p1
                .mods
                .iter()
                .any(|modifier| matches!(modifier.kind, ModifierKind::StarshipLaunchCost { .. }))
        );
    }

    #[test]
    fn r1852006_a_launch_by_an_effect_spends_no_price() {
        let mut s = built_ship(json!([liftoff.id, ignition.id]), None);
        s.play(liftoff.id.as_str(), json!({}));
        s.play(ignition.id.as_str(), json!({}));
        assert_eq!(launch_cost(s.state(), P1), 0, "still held for the next launch");
    }
}

mod r1852007_a_stack_card_onto_the_dormant_starship {
    use super::*;

    #[test]
    fn r1852007_a_stack_card_played_onto_a_dormant_starship_is_exiled_before_it_resolves() {
        // A full backrow puts the Starship in the unit row's middle (R2249000), where a Stack card lands.
        let mut s = board_with_backrow(
            "p1",
            json!([bolt.id, lander.id]),
            json!([{ "def": booster.id, "lane": 1 }]),
            full_backrow(),
            json!([plain.id]),
            json!([]),
        );
        let lane = STARSHIP_LANE_ORDER[0];
        let piece = id_at(&s, P1, 1);
        bolt_at(&mut s, &piece);
        let ship = ship_of(&s, P1);
        let health = health_of(&s, P2);
        assert_eq!(id_at(&s, P1, lane), ship.id);
        s.play(lander.id.as_str(), json!({ "zone": lane }));
        // It was played, and exiled before its Cry.
        assert!(
            s.last_events()
                .iter()
                .any(|event| event.event_type() == GameEventType::CardPlayed)
        );
        assert!(
            s.last_events()
                .iter()
                .any(|event| event.event_type() == GameEventType::Exiled)
        );
        let lander_card = s
            .pile(P1, "exile")
            .into_iter()
            .find(|card| card.def_id == lander.id)
            .expect("the lander is in exile");
        assert_eq!(lander_card.zone.z(), ZoneName::Exile);
        assert_eq!(health_of(&s, P2), health, "its Cry never resolved");
        assert_eq!(id_at(&s, P1, lane), ship.id);
        assert!(is_dormant_starship(s.card(&ship.id)));
    }
}

/// R1857001: the Dormant Starship is a card, not a token (R1852000), and still in no pool and no deck,
/// as R674 keeps Glitch out of every pool: only a Starship Piece's death summons it. A pool that names
/// it by id reaches it. Proved on this file's fixture hull, a shipped Core card under the engine's id.
mod r1857001_the_dormant_starship_is_in_no_pool_and_no_deck {
    use super::*;

    fn pool(args: Value) -> Vec<String> {
        query(&json_as(args)).iter().map(|def| def.id.clone()).collect()
    }

    #[test]
    fn r1857001_the_dormant_starship_is_in_no_pool_and_no_deck() {
        register_starship_fixtures();
        assert_eq!(hull.id, DORMANT_STARSHIP_DEF_ID);
        assert!(!hull.is_token(), "it is no token (R1852000)");
        for args in [
            json!({}),
            json!({ "type": "Unit" }),
            json!({ "set": "Core", "type": "Unit" }),
            json!({ "cost": 3 }),
            json!({ "withTokens": true }),
            json!({ "everySet": true }),
            json!({ "excludeDefId": booster.id }),
        ] {
            assert!(!pool(args.clone()).contains(&hull.id), "{args}");
        }
        // The pieces are cards like any other.
        assert!(pool(json!({ "type": "Unit" })).contains(&booster.id));
        assert_eq!(pool(json!({ "defId": [hull.id] })), vec![hull.id.clone()]);
        assert!(made_by_a_rule(&hull));
        assert!(!made_by_a_rule(&booster));
        assert!(!deckable(&hull));
        assert!(deckable(&booster));
    }
}

mod r2249000_a_dormant_starship_arrives_in_the_middle_of_the_backrow {
    use super::*;

    /// The zone p1's Dormant Starship stands in, and whether it tops a pile.
    fn ship_zone(s: &Scenario) -> (Row, i32) {
        let ship = ship_of(s, P1);
        match ship.zone {
            Zone::Field { row, lane, .. } => (row, lane),
            other => panic!("the Starship is not on the field: {other:?}"),
        }
    }

    /// p1 with a booster in unit lane 1, `units` more plain Units and `backrow` countdown Traps in the
    /// lanes given; the booster is bolted, so a Starship is summoned.
    fn summoned_with(units: &[i32], backrow: &[i32]) -> Scenario {
        let mut field = vec![json!({ "def": booster.id, "lane": 1 })];
        field.extend(units.iter().map(|lane| json!({ "def": plain.id, "lane": lane })));
        let traps: Vec<Value> = backrow
            .iter()
            .map(|lane| json!({ "def": countdown.id, "lane": lane }))
            .collect();
        register_starship_fixtures();
        let library = json!([plain.id, plain.id, plain.id, plain.id, plain.id, plain.id]);
        let mut s = scenario(json!({
            "seed": "r2249000",
            "active": "p1",
            "p1": { "hand": [bolt.id], "field": field, "backrow": traps, "library": library },
            "p2": { "hand": [plain.id], "library": library },
        }));
        let piece = id_at(&s, P1, 1);
        bolt_at(&mut s, &piece);
        s
    }

    #[test]
    fn r2249000_it_stands_in_backrow_lane_3_first_then_2_4_1_5() {
        let s = summoned_with(&[], &[]);
        assert_eq!(ship_zone(&s), (Row::Backrow, 3));
        assert_eq!(STARSHIP_LANE_ORDER, [3, 2, 4, 1, 5]);
        for (taken, expected) in [
            (vec![3], 2),
            (vec![3, 2], 4),
            (vec![3, 2, 4], 1),
            (vec![3, 2, 4, 1], 5),
        ] {
            let s = summoned_with(&[], &taken);
            assert_eq!(ship_zone(&s), (Row::Backrow, expected), "{taken:?} taken");
        }
    }

    #[test]
    fn r2249000_r1081000_standing_in_the_backrow_it_is_its_owners_dormant_starship_and_launches() {
        let mut s = summoned_with(&[], &[]);
        let ship = ship_of(&s, P1);
        assert!(stands_in_backrow(s.state(), &ship));
        assert!(building_a_ship(s.state(), P1));
        assert_eq!(piece_defs(&ship), vec![booster.id.clone()]);
        // Both seats see it, the Unit standing in that backrow zone, with its pieces.
        for viewer in [P1, P2] {
            let view = s.view(viewer);
            let side = if viewer == P1 { &view.you } else { &view.opponent };
            let unit = side.backrow[2]
                .as_ref()
                .and_then(|card| match card {
                    BackrowView::Public(card) => card.unit.clone(),
                    _ => None,
                })
                .expect("backrow lane 3 shows the Starship");
            assert_eq!(unit.starship.expect("its memory").pieces.len(), 1, "{viewer:?}");
        }
        assert!(offers_launch(&s, P1, &ship.id));
        launch(&mut s, &ship.id);
        assert!(is_launched_starship(s.card(&ship.id)));
        assert!(
            active_units_of(s.state(), P1)
                .iter()
                .any(|unit| unit.id == ship.id),
            "launched, it is a Unit for every rule"
        );
    }

    #[test]
    fn r2249000_with_the_backrow_full_it_takes_the_unit_row_middle_first_and_with_both_full_tops_lane_3() {
        let backrow = [1, 2, 3, 4, 5];
        let s = summoned_with(&[], &backrow);
        assert_eq!(ship_zone(&s), (Row::Units, 3));
        let s = summoned_with(&[3], &backrow);
        assert_eq!(ship_zone(&s), (Row::Units, 2));
        let s = summoned_with(&[2, 3, 4, 5], &backrow);
        // The bolted booster's own zone, lane 1, is the one left.
        assert_eq!(ship_zone(&s), (Row::Units, 1));
    }
}
