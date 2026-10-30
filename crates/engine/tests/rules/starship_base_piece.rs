// R2521000 (issue #2521): a dead Starship Piece fuses its base version into its owner's Dormant
// Starship (`starship::piece_copy`): a fresh card of the piece it was printed as — for a fusion, the
// card the others were fused into, its last ingredient, a fused one walked the same way — on its
// Radiant face if it was Radiant, and without its tuning. The Starship's memory and `starshipFused`
// name that base definition. Proved on the fixtures of `fixtures/starship.rs`.

use jackioh_engine::testkit::PlayerId::P1;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::starship::*;

/// p1's turn, its main phase, with the given hand and field; both libraries hold plain cards to draw,
/// so no turn ends by itself (§2.5).
fn board(p1_hand: Value, p1_field: Value) -> Scenario {
    register_starship_fixtures();
    let library = json!([plain.id, plain.id, plain.id, plain.id, plain.id, plain.id]);
    scenario(json!({
        "seed": "r2521000",
        "active": "p1",
        "p1": { "hand": p1_hand, "field": p1_field, "library": library },
        "p2": { "hand": [plain.id], "field": [], "library": library },
    }))
}

fn id_at(s: &Scenario, lane: i32) -> String {
    s.unit(P1, lane)
        .unwrap_or_else(|| panic!("no unit at p1's lane {lane}"))
        .id
}

fn bolt_at(s: &mut Scenario, id: &str) {
    s.play(
        bolt.id.as_str(),
        json!({ "targets": [{ "pick": "instance", "instanceId": id }] }),
    );
}

fn equip_onto_lane_1(s: &mut Scenario, card: &str) {
    s.play(card, json!({ "zone": 1, "equip": true }));
}

fn ship_of(s: &Scenario) -> CardInstance {
    dormant_starship_of(s.state(), P1)
        .cloned()
        .expect("p1 has a Dormant Starship")
}

/// The definitions a fused definition lists, in ingredient order (`CardDef.ingredients`).
fn ingredients_of(s: &Scenario, def_id: &str) -> Vec<String> {
    jackioh_engine::catalog::find_def(Some(s.state()), def_id)
        .and_then(|def| def.ingredients.clone())
        .unwrap_or_default()
        .into_iter()
        .map(|spec| spec.def_id)
        .collect()
}

fn record(def_id: &str, radiant: bool) -> StarshipPieceRecord {
    StarshipPieceRecord {
        def_id: def_id.to_string(),
        radiant: if radiant { Some(true) } else { None },
    }
}

/// The `def_id` of the `starshipFused` the last action reported for the dead piece `piece`.
fn fused_def_of(s: &Scenario, piece: &str) -> String {
    s.last_events()
        .iter()
        .find_map(|event| match event {
            GameEvent::StarshipFused { piece_id, def_id, .. } if piece_id == piece => Some(def_id.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no starshipFused names {piece}"))
}

fn keywords_of(s: &Scenario, id: &str) -> Vec<KeywordKind> {
    s.stats(id).keywords.iter().map(Keyword::kind).collect()
}

mod r2521000_a_dead_piece_fuses_its_base_version {
    use super::*;

    #[test]
    fn r2521000_a_piece_with_a_card_equipped_onto_it_fuses_its_own_definition_alone() {
        let mut s = board(
            json!([clamp.id, bolt.id]),
            json!([{ "def": booster.id, "lane": 1 }]),
        );
        let host = id_at(&s, 1);
        equip_onto_lane_1(&mut s, clamp.id.as_str());
        let fused = s.card(&host).def_id.clone();
        assert_eq!(
            ingredients_of(&s, &fused),
            vec![clamp.id.clone(), booster.id.clone()],
            "the host is the fusion's last ingredient"
        );
        s.expect_stats(host.as_str(), json!({ "attack": 3, "health": 4 }));
        bolt_at(&mut s, &host);
        assert_eq!(s.card(&host).zone.z(), ZoneName::Graveyard);
        let ship = ship_of(&s);
        // The memory and the event name the booster, not the fusion.
        assert_eq!(starship_pieces_of(&ship), vec![record(&booster.id, false)]);
        assert_eq!(fused_def_of(&s, &host), booster.id);
        // The Starship gains the booster's 2/3 alone: none of the clamp's stats, nor its Equip.
        s.expect_stats(ship.id.as_str(), json!({ "attack": 2, "health": 3 }));
        assert!(!keywords_of(&s, &ship.id).contains(&KeywordKind::Equip));
        assert!(!ingredients_of(&s, &ship.def_id).contains(&clamp.id));
        assert!(ingredients_of(&s, &ship.def_id).contains(&booster.id));
    }

    #[test]
    fn r2521000_r1852001_of_two_pieces_fused_the_kept_one_the_last_ingredient_is_the_base() {
        let mut s = board(
            json!([plate.id, bolt.id]),
            json!([{ "def": booster.id, "lane": 1 }]),
        );
        let host = id_at(&s, 1);
        equip_onto_lane_1(&mut s, plate.id.as_str());
        // Both print Starship Piece; the booster, kept, is walked first.
        let fused = s.card(&host).def_id.clone();
        assert_eq!(
            ingredients_of(&s, &fused),
            vec![plate.id.clone(), booster.id.clone()]
        );
        bolt_at(&mut s, &host);
        let ship = ship_of(&s);
        assert_eq!(starship_pieces_of(&ship), vec![record(&booster.id, false)]);
        assert_eq!(fused_def_of(&s, &host), booster.id);
        s.expect_stats(ship.id.as_str(), json!({ "attack": 2, "health": 3 }));
    }

    #[test]
    fn r2521000_a_fusion_of_a_fusion_is_walked_into_its_fused_ingredient() {
        let mut s = board(
            json!([clamp.id, clamp.id, bolt.id]),
            json!([{ "def": booster.id, "lane": 1 }]),
        );
        let host = id_at(&s, 1);
        equip_onto_lane_1(&mut s, clamp.id.as_str());
        let first = s.card(&host).def_id.clone();
        equip_onto_lane_1(&mut s, clamp.id.as_str());
        let second = s.card(&host).def_id.clone();
        assert_ne!(first, second);
        // The second fusion's last ingredient is the first fusion, whose last is the booster.
        assert_eq!(ingredients_of(&s, &second), vec![clamp.id.clone(), first.clone()]);
        s.expect_stats(host.as_str(), json!({ "attack": 4, "health": 5 }));
        bolt_at(&mut s, &host);
        let ship = ship_of(&s);
        assert_eq!(starship_pieces_of(&ship), vec![record(&booster.id, false)]);
        assert_eq!(fused_def_of(&s, &host), booster.id);
        s.expect_stats(ship.id.as_str(), json!({ "attack": 2, "health": 3 }));
    }

    #[test]
    fn r2521000_r1878001_a_kept_card_that_lent_itself_the_piece_fuses_itself() {
        let mut s = board(json!([lender.id, plate.id, bolt.id]), json!([]));
        s.play(lender.id.as_str(), json!({ "zone": 1, "embiggen": true }));
        let host = id_at(&s, 1);
        equip_onto_lane_1(&mut s, plate.id.as_str());
        assert_eq!(id_at(&s, 1), host, "the lender is the kept card");
        assert!(keywords_of(&s, &host).contains(&KeywordKind::StarshipPiece));
        bolt_at(&mut s, &host);
        // The kept card is the fusion's last ingredient, so the lender fuses, never the plate on it.
        let ship = ship_of(&s);
        assert_eq!(starship_pieces_of(&ship), vec![record(&lender.id, false)]);
        assert_eq!(fused_def_of(&s, &host), lender.id);
        s.expect_stats(ship.id.as_str(), json!({ "attack": 1, "health": 1 }));
    }

    #[test]
    fn r2521000_r1852000_a_fusion_none_of_whose_ingredients_prints_the_keyword_fuses_its_kept_card() {
        let mut s = board(json!([lender.id, clamp.id, bolt.id]), json!([]));
        s.play(lender.id.as_str(), json!({ "zone": 1, "embiggen": true }));
        let host = id_at(&s, 1);
        equip_onto_lane_1(&mut s, clamp.id.as_str());
        assert!(keywords_of(&s, &host).contains(&KeywordKind::StarshipPiece));
        bolt_at(&mut s, &host);
        // Neither the lender nor the clamp prints it: the base is still the kept card, the lender.
        let ship = ship_of(&s);
        assert_eq!(starship_pieces_of(&ship), vec![record(&lender.id, false)]);
        assert_eq!(fused_def_of(&s, &host), lender.id);
    }

    #[test]
    fn r2521000_a_radiant_piece_fuses_on_its_radiant_face() {
        let mut s = board(
            json!([bolt.id]),
            json!([{ "def": booster.id, "lane": 1, "radiant": true, "damage": 2 }]),
        );
        let piece = id_at(&s, 1);
        s.expect_stats(piece.as_str(), json!({ "attack": 4, "health": 4 }));
        bolt_at(&mut s, &piece);
        let ship = ship_of(&s);
        assert_eq!(starship_pieces_of(&ship), vec![record(&booster.id, true)]);
        assert_eq!(fused_def_of(&s, &piece), booster.id);
        // The booster's Radiant 4/6, undamaged: the copy is fresh.
        s.expect_stats(ship.id.as_str(), json!({ "attack": 4, "health": 6 }));
    }

    #[test]
    fn r2521000_a_tuned_piece_fuses_with_its_printed_numbers_and_no_tuning() {
        let mut s = board(json!([bolt.id]), json!([{ "def": booster.id, "lane": 1 }]));
        let piece = id_at(&s, 1);
        s.card_mut(&piece).tuning = Some(Tuning {
            attack: Some(1),
            health: Some(1),
            ..Tuning::default()
        });
        s.expect_stats(piece.as_str(), json!({ "attack": 3, "health": 4 }));
        bolt_at(&mut s, &piece);
        let ship = ship_of(&s);
        assert_eq!(starship_pieces_of(&ship), vec![record(&booster.id, false)]);
        // The Upgrade was the dead card's: the Starship holds the booster's printed 2/3 and no tuning.
        s.expect_stats(ship.id.as_str(), json!({ "attack": 2, "health": 3 }));
        assert_eq!(s.card(&ship.id).tuning, None);
    }
}
