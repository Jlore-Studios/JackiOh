//! The learned evaluation's inference (#1010, `src/learned.rs`): the Rust gives what `training/ml`'s
//! reference computes, and the switch and the null network change nothing.

use jackioh_ai::*;
use jackioh_engine::testkit::*;
use serde::Deserialize;

/// A seeded random network over the training fixture's positions, with the outputs
/// `training/ml/jackioh_ml/export.py`'s `reference` computes for 40 of them
/// (`python -m jackioh_ml.export --random 1010 --positions tests/fixtures/positions.jsonl`).
const TINY: &str = include_str!("../fixtures/learned-tiny.json");

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LayerJson {
    w: Vec<Vec<f64>>,
    b: Vec<f64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OutJson {
    w: Vec<f64>,
    b: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NetworkJson {
    features_version: u32,
    catalog: String,
    mean: Vec<f64>,
    scale: Vec<f64>,
    keys: Vec<String>,
    width: usize,
    dense_in: Vec<Vec<f64>>,
    sparse_in: Vec<Vec<f64>>,
    bias: Vec<f64>,
    head: Vec<LayerJson>,
    out: OutJson,
    #[serde(default)]
    linear: Vec<f64>,
    hand_a: f64,
    hand_b: f64,
    hand_unit: f64,
}

#[derive(Deserialize)]
struct Case {
    dense: Vec<i32>,
    sparse: Vec<(String, i32)>,
    hand: f64,
    correction: f64,
    points: f64,
}

#[derive(Deserialize)]
struct Fixture {
    network: NetworkJson,
    cases: Vec<Case>,
}

/// The fixture's network, owned, and the flat slices a `Network` borrows.
struct Owned {
    json: NetworkJson,
    keys: Vec<String>,
    dense_in: Vec<f64>,
    sparse_in: Vec<f64>,
    head_w: Vec<Vec<f64>>,
}

impl Owned {
    fn of(json: NetworkJson) -> Owned {
        Owned {
            keys: json.keys.clone(),
            dense_in: json.dense_in.iter().flatten().copied().collect(),
            sparse_in: json.sparse_in.iter().flatten().copied().collect(),
            head_w: json
                .head
                .iter()
                .map(|layer| layer.w.iter().flatten().copied().collect())
                .collect(),
            json,
        }
    }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 * a.abs().max(b.abs()).max(1.0)
}

/// Lends the fixture's network to `run`, with the fixture's cases.
fn with_tiny<R>(run: impl FnOnce(&Network<'_>, &[Case]) -> R) -> R {
    let fixture: Fixture = serde_json::from_str(TINY).expect("the fixture parses");
    assert!((FEATURES_READ_SINCE..=FEATURES_VERSION).contains(&fixture.network.features_version));
    let owned = Owned::of(fixture.network);
    let keys: Vec<&str> = owned.keys.iter().map(String::as_str).collect();
    let mut inputs = owned.json.width;
    let layers: Vec<Layer<'_>> = owned
        .json
        .head
        .iter()
        .zip(&owned.head_w)
        .map(|(layer, w)| {
            let made = Layer {
                inputs,
                w: w.as_slice(),
                b: layer.b.as_slice(),
            };
            inputs = layer.b.len();
            made
        })
        .collect();
    let network = Network {
        features_version: owned.json.features_version,
        catalog: &owned.json.catalog,
        width: owned.json.width,
        mean: &owned.json.mean,
        scale: &owned.json.scale,
        keys: &keys,
        dense_in: &owned.dense_in,
        sparse_in: &owned.sparse_in,
        bias: &owned.json.bias,
        head: &layers,
        out_w: &owned.json.out.w,
        out_b: owned.json.out.b,
        linear: &owned.json.linear,
        hand_a: owned.json.hand_a,
        hand_b: owned.json.hand_b,
        hand_unit: owned.json.hand_unit,
    };
    run(&network, &fixture.cases)
}

/// R1010001: a committed tiny network gives, on committed positions, the outputs training/ml's
/// reference computes.
#[test]
fn r1010001_a_tiny_network_gives_the_outputs_the_python_reference_computes() {
    with_tiny(|network, cases| {
        assert_eq!(cases.len(), 40);
        let mut moved = 0;
        for (at, case) in cases.iter().enumerate() {
            let features = Features {
                dense: case.dense.clone(),
                sparse: case.sparse.clone(),
            };
            let correction = network.correction(&features);
            assert!(
                close(correction, case.correction),
                "case {at}: {correction} against {}",
                case.correction
            );
            let points = network.points(case.hand, &features);
            assert!(
                close(points, case.points),
                "case {at}: {points} against {}",
                case.points
            );
            if correction != 0.0 {
                moved += 1;
            }
        }
        assert!(moved > 30, "the network moves most positions");

        // Features of another layout get no correction, never a wrong one.
        let short = Features {
            dense: cases[0].dense[1..].to_vec(),
            sparse: Vec::new(),
        };
        assert_eq!(network.correction(&short), 0.0);
    });
}

/// R1012001: a finalist takes the network's correction at `LEARNED_CORRECTION_SHARE`: its score lies
/// between the hand eval's and the network's whole one.
#[test]
fn r1012001_a_finalist_takes_the_correction_at_its_share() {
    let solo = state();
    with_tiny(|network, _| {
        for next in [NextSwing::Seat, NextSwing::Enemy] {
            let hand = evaluate(&solo, PlayerId::P1, next, &AI_EVAL);
            let points = network.points(hand, &features_of(&solo, PlayerId::P1));
            let finalist = finalist_score_with(
                &solo,
                PlayerId::P1,
                next,
                EvalKind::Learned,
                &Learned::Float(*network),
                Prices::NONE,
            );
            assert_ne!(points, hand, "the network moves this position");
            assert_eq!(finalist, hand + LEARNED_CORRECTION_SHARE * (points - hand));
            let (low, high) = if points < hand {
                (points, hand)
            } else {
                (hand, points)
            };
            assert!(
                low <= finalist && finalist <= high,
                "{finalist} outside [{low}, {high}]"
            );
        }
    });
}

/// R1012002: the learned evaluation reaches as far into the search as `LearnedReach` says, each step
/// taking in the one before it: a complete line takes a finalist's score from `Lines` on, an open
/// line from `Beam` on, and with the switch off every one keeps the hand eval.
#[test]
fn r1012002_the_reach_says_which_of_the_beam_s_scores_the_network_gives() {
    let solo = state();
    with_tiny(|network, _| {
        let network = Learned::Float(*network);
        let next = NextSwing::Enemy;
        let hand = evaluate(&solo, PlayerId::P1, next, &AI_EVAL);
        let learned = finalist_score_with(
            &solo,
            PlayerId::P1,
            next,
            EvalKind::Learned,
            &network,
            Prices::NONE,
        );
        assert_ne!(learned, hand, "the network moves this position");
        for (reach, line, open) in [
            (LearnedReach::Finalists, hand, hand),
            (LearnedReach::Lines, learned, hand),
            (LearnedReach::Beam, learned, learned),
        ] {
            let kind = EvalKind::Learned;
            assert_eq!(
                line_score_with(&solo, PlayerId::P1, next, reach, kind, &network, Prices::NONE),
                line,
                "{reach:?}: a complete line"
            );
            assert_eq!(
                open_score_with(&solo, PlayerId::P1, next, reach, kind, &network, Prices::NONE),
                open,
                "{reach:?}: an open line"
            );
            for score in [line_score_with, open_score_with] {
                assert_eq!(
                    score(
                        &solo,
                        PlayerId::P1,
                        next,
                        reach,
                        EvalKind::Hand,
                        &network,
                        Prices::NONE
                    ),
                    hand
                );
            }
        }
    });
}

/// R1010002: with the switch on, a 1v1 finalist takes the network's score (at its share, R1012001)
/// and a Duos duel's keeps the hand eval's (D9, R964000).
#[test]
fn r1010002_a_duos_duel_keeps_the_hand_eval() {
    let solo = state();
    let mut duos = solo.clone();
    for seat in [PlayerId::P1, PlayerId::P2] {
        duos.players[seat].duos = Some(DuosLife {
            pool: 10,
            pool_cap: 10,
            spent: false,
        });
    }
    with_tiny(|network, _| {
        let hand = evaluate(&solo, PlayerId::P1, NextSwing::Seat, &AI_EVAL);
        let learned = finalist_score_with(
            &solo,
            PlayerId::P1,
            NextSwing::Seat,
            EvalKind::Learned,
            &Learned::Float(*network),
            Prices::NONE,
        );
        let features = features_of(&solo, PlayerId::P1);
        let points = network.points(hand, &features);
        assert_eq!(learned, hand + LEARNED_CORRECTION_SHARE * (points - hand));
        assert_ne!(learned, hand, "the network moves this position");

        let duos_hand = evaluate(&duos, PlayerId::P1, NextSwing::Seat, &AI_EVAL);
        assert_eq!(
            finalist_score_with(
                &duos,
                PlayerId::P1,
                NextSwing::Seat,
                EvalKind::Learned,
                &Learned::Float(*network),
                Prices::NONE,
            ),
            duos_hand
        );
    });
}

/// R1012003: with the switch on, the tutorial's opponent (`AI_TUTORIAL`, R290) keeps the hand eval;
/// the other seat, and a seat at any practice tier, still take the network's correction.
#[test]
fn r1012003_the_tutorials_opponent_keeps_the_hand_eval_and_every_tier_takes_the_network() {
    let solo = state();
    let mut tutorial = solo.clone();
    tutorial.players[PlayerId::P1].handicap = Some(AI_TUTORIAL);
    with_tiny(|network, _| {
        let network = Learned::Float(*network);
        for next in [NextSwing::Enemy, NextSwing::Seat] {
            let hand = evaluate(&tutorial, PlayerId::P1, next, &AI_EVAL);
            assert_eq!(
                finalist_score_with(
                    &tutorial,
                    PlayerId::P1,
                    next,
                    EvalKind::Learned,
                    &network,
                    Prices::NONE
                ),
                hand
            );
            let other = finalist_score_with(
                &tutorial,
                PlayerId::P2,
                next,
                EvalKind::Learned,
                &network,
                Prices::NONE,
            );
            assert_eq!(
                other,
                finalist_score_with(
                    &solo,
                    PlayerId::P2,
                    next,
                    EvalKind::Learned,
                    &network,
                    Prices::NONE
                )
            );
        }
        for tier in [AI_DIFFICULTY.medium, AI_DIFFICULTY.hard] {
            let mut practice = solo.clone();
            practice.players[PlayerId::P1].handicap = Some(tier);
            assert_ne!(
                finalist_score_with(
                    &practice,
                    PlayerId::P1,
                    NextSwing::Seat,
                    EvalKind::Learned,
                    &network,
                    Prices::NONE,
                ),
                evaluate(&practice, PlayerId::P1, NextSwing::Seat, &AI_EVAL),
                "a tier's handicap takes the network's correction"
            );
        }
    });
}

/// R1012003 with R2201000's floor and R2201001's pool hold: neither is a resource, so a seat under
/// AI_TUTORIAL that also carries either is still the tutorial's opponent and keeps the hand eval, and
/// the tiers carrying them still take the network's correction.
#[test]
fn r1012003_r2201000_the_tutorials_opponent_keeps_the_hand_eval_with_a_floor_or_a_pool_hold() {
    let solo = state();
    let lesson_fields = |handicap: Handicap| Handicap {
        hero_floor: Some(TUTORIAL_HERO_FLOOR),
        pool_hold: Some(true),
        ..handicap
    };
    assert!(is_tutorial_opponent(Some(AI_TUTORIAL)));
    assert!(is_tutorial_opponent(Some(lesson_fields(AI_TUTORIAL))));
    assert!(!is_tutorial_opponent(None));
    assert!(!is_tutorial_opponent(Some(TUTORIAL_HUMAN)));
    for tier in [AI_DIFFICULTY.easy, AI_DIFFICULTY.medium, AI_DIFFICULTY.hard] {
        assert!(!is_tutorial_opponent(Some(tier)));
        assert!(!is_tutorial_opponent(Some(lesson_fields(tier))));
    }
    let mut tutorial = solo.clone();
    tutorial.players[PlayerId::P1].handicap = Some(lesson_fields(AI_TUTORIAL));
    with_tiny(|network, _| {
        let network = Learned::Float(*network);
        for next in [NextSwing::Enemy, NextSwing::Seat] {
            assert_eq!(
                finalist_score_with(
                    &tutorial,
                    PlayerId::P1,
                    next,
                    EvalKind::Learned,
                    &network,
                    Prices::NONE
                ),
                evaluate(&tutorial, PlayerId::P1, next, &AI_EVAL)
            );
        }
    });
}

/// R1012003 with R2533000's bar and R2533001's copies: a lesson's opponent with them, and one recorded
/// before them (AI_TUTORIAL without either, as R2433000's lessons read now), are both the tutorial's
/// opponent.
#[test]
fn r1012003_r2533000_the_tutorial_opponent_is_read_with_or_without_its_bar_and_copies() {
    assert_eq!(AI_TUTORIAL.tutorial_cards_only, Some(true));
    assert!(is_tutorial_opponent(Some(AI_TUTORIAL)));
    assert!(is_tutorial_opponent(Some(Handicap {
        tutorial_cards_only: None,
        deck_copies: None,
        ..AI_TUTORIAL
    })));
    assert!(!is_tutorial_opponent(Some(Handicap {
        tutorial_cards_only: Some(true),
        ..HUMAN_HANDICAP
    })));
}

fn state() -> GameState {
    jackioh_cards::register_all();
    scenario(json!({
        "seed": "learned-1010",
        "p1": { "hand": ["core-008", "core-035"], "field": ["core-011"], "library": ["core-020", "core-053"] },
        "p2": { "hand": ["core-002"], "field": ["core-019"], "backrow": [{ "def": "core-041", "faceUp": false }],
                "library": ["core-005"], "health": 20 },
    }))
    .state()
    .clone()
}

/// The switch off, or on with the null network, scores a finalist exactly as the hand eval does.
#[test]
fn r1010000_the_hand_switch_and_the_null_network_score_as_the_hand_eval() {
    let state = state();
    for next in [NextSwing::Enemy, NextSwing::Seat] {
        let hand = evaluate(&state, PlayerId::P1, next, &AI_EVAL);
        assert_eq!(
            finalist_score_with(&state, PlayerId::P1, next, EvalKind::Hand, &LEARNED, Prices::NONE),
            hand
        );
        let null = match LEARNED {
            Learned::Float(net) => {
                net.keys.is_empty()
                    && net.hand_b == 0.0
                    && net.out_w.iter().all(|w| *w == 0.0)
                    && net.out_b == 0.0
            }
            Learned::Int(net) => {
                net.keys.is_empty()
                    && net.hand_b == 0.0
                    && net.out_w.iter().all(|w| *w == 0)
                    && net.out_b == 0
            }
        };
        if null {
            assert_eq!(
                finalist_score_with(
                    &state,
                    PlayerId::P1,
                    next,
                    EvalKind::Learned,
                    &LEARNED,
                    Prices::NONE
                ),
                hand
            );
        }
    }
    assert_eq!(
        finalist_score(&state, PlayerId::P1, NextSwing::Enemy, Prices::NONE),
        {
            match AI_EVAL_KIND {
                EvalKind::Hand => evaluate(&state, PlayerId::P1, NextSwing::Enemy, &AI_EVAL),
                EvalKind::Learned => finalist_score_with(
                    &state,
                    PlayerId::P1,
                    NextSwing::Enemy,
                    EvalKind::Learned,
                    &LEARNED,
                    Prices::NONE,
                ),
            }
        }
    );
}

/// The integer network's parity fixture (`training/ml/jackioh_ml/quantize.py --parity`): a seeded
/// random network, quantized, and 40 positions with the corrections the Python integer reference
/// computes, as the shortest text that reads back as each double.
const PARITY: &str = include_str!("../fixtures/learned-parity.json");

/// R1011000: the integer network gives, natively, exactly the Python reference's corrections, bit
/// for bit; the web's tests hold the WASM build to the same fixture.
#[test]
fn r1011000_the_integer_network_gives_the_reference_bit_for_bit() {
    let fixture: Value = serde_json::from_str(PARITY).expect("the fixture parses");
    let answers = parity_answers(&fixture).expect("the fixture's network reads");
    let cases = fixture["cases"].as_array().expect("cases");
    assert_eq!(answers.len(), 40);
    assert_eq!(answers.len(), cases.len());
    let mut moved = 0;
    for (at, (answer, case)) in answers.iter().zip(cases).enumerate() {
        let expected: f64 = case["expected"]
            .as_str()
            .expect("text")
            .parse()
            .expect("a number");
        assert_eq!(
            answer.to_bits(),
            expected.to_bits(),
            "case {at}: {answer} against {expected}"
        );
        moved += usize::from(*answer != 0.0);
    }
    assert!(moved > 30, "the network moves most positions");
}

/// R3038003: a position `holds_starship` reads for `seat`: p1 and p2 each with `hand`, `field` and, for p2,
/// `theirs` on the field, in a board with no other mechanic of the set.
fn starship_board(mine: Value, hand: Value, theirs: Value) -> GameState {
    jackioh_cards::register_all();
    scenario(json!({
        "seed": "learned-3038",
        "p1": { "hand": hand, "field": mine, "library": ["core-020", "core-053"] },
        "p2": { "hand": ["core-002"], "field": theirs, "library": ["core-005"] },
    }))
    .state()
    .clone()
}

/// R3038003: with the root's fact set (`Prices.starship_root`) a finalist keeps the hand eval for either
/// seat, as a Duos duel does (R1010002), though the network moves the same position when it is not.
#[test]
fn r3038003_a_finalist_of_a_decision_that_holds_a_starship_keeps_the_hand_eval() {
    let solo = state();
    assert!(!holds_starship(&solo, PlayerId::P1), "no Starship on this board");
    let rooted = Prices {
        starship_root: true,
        ..Prices::default()
    };
    assert_ne!(&rooted, Prices::NONE);
    with_tiny(|network, _| {
        let network = Learned::Float(*network);
        for next in [NextSwing::Seat, NextSwing::Enemy] {
            let hand = evaluate(&solo, PlayerId::P1, next, &AI_EVAL);
            let score =
                |seat, prices| finalist_score_with(&solo, seat, next, EvalKind::Learned, &network, prices);
            assert_ne!(
                score(PlayerId::P1, Prices::NONE),
                hand,
                "the network moves this position when the fact is not set"
            );
            assert_eq!(score(PlayerId::P1, &rooted), hand);
            assert_eq!(
                score(PlayerId::P2, &rooted),
                evaluate(&solo, PlayerId::P2, next, &AI_EVAL)
            );
        }
    });
}

/// R3038003: the fact is a hull on either side or a piece on either field or in the seat's own hand, and
/// it reads nothing the seat cannot see: the other hand's piece is no fact for it.
#[test]
fn r3038003_a_piece_on_either_field_or_in_the_seats_own_hand_is_a_starship_the_root_holds() {
    const VANILLA: &str = "core-008";
    const AFTER_BURNER: &str = "stinkyscape-090";
    const HIT_JOB: &str = "core-016";
    let (p1, p2) = (PlayerId::P1, PlayerId::P2);

    let bare = starship_board(json!([VANILLA]), json!([VANILLA]), json!([VANILLA]));
    assert!(!holds_starship(&bare, p1) && !holds_starship(&bare, p2));

    let theirs = starship_board(json!([VANILLA]), json!([VANILLA]), json!([AFTER_BURNER]));
    assert!(holds_starship(&theirs, p1), "the enemy's piece on its field");
    assert!(holds_starship(&theirs, p2));
    let mine = starship_board(json!([AFTER_BURNER]), json!([VANILLA]), json!([VANILLA]));
    assert!(holds_starship(&mine, p1) && holds_starship(&mine, p2));

    let in_hand = starship_board(json!([VANILLA]), json!([AFTER_BURNER]), json!([VANILLA]));
    assert!(holds_starship(&in_hand, p1), "the seat's own hand");
    assert!(
        !holds_starship(&in_hand, p2),
        "the other seat cannot see that hand, so it is no fact for it"
    );

    // A piece that dies fuses into a Dormant Starship: the hull holds it with no piece left to read.
    jackioh_cards::register_all();
    let mut s = scenario(json!({
        "seed": "learned-3038-hull",
        "p1": { "hand": [HIT_JOB], "field": [], "library": ["core-020", "core-053"], "mana": 3 },
        "p2": { "hand": ["core-002"], "field": [{ "def": AFTER_BURNER, "lane": 1 }], "library": ["core-005"] },
    }));
    let piece = s.unit_at(p2, 1).id;
    s.play(HIT_JOB, json!({ "targets": pick::id(&piece) }));
    let after = s.state();
    assert!(dormant_starship_of(after, p2).is_some());
    assert!(
        active_units_of(after, p2).is_empty(),
        "no piece is left on the field"
    );
    assert!(holds_starship(after, p1) && holds_starship(after, p2), "the hull");
}

/// One Unit on p1's board and nothing else.
fn solo_board(seed: &str, unit: &str) -> GameState {
    jackioh_cards::register_all();
    scenario(json!({ "seed": seed, "p1": { "field": [unit] }, "p2": {} }))
        .state()
        .clone()
}

/// `features` less every key that does not name a card by id: what a network fitted before
/// `FEATURES_VERSION` 3 could read of it (R3112000).
fn ids_only(features: &Features) -> Features {
    Features {
        dense: features.dense.clone(),
        sparse: features
            .sparse
            .iter()
            .filter(|(key, _)| card_id_of_key(key).is_some())
            .cloned()
            .collect(),
    }
}

/// R3112000: a network with rows for one card and what it is, and none for another card's id, still
/// moves on the other card through the keys of what the two share.
#[test]
fn r3112000_a_card_with_no_id_row_still_moves_the_evaluation_through_what_it_is() {
    let known = solo_board("learned-3112-known", "core-011");
    let fresh = solo_board("learned-3112-fresh", "core-008");
    let trained = features_of(&known, PlayerId::P1);
    let keys: Vec<&str> = trained.sparse.iter().map(|(key, _)| key.as_str()).collect();
    assert!(keys.contains(&"board:own:core-011"), "{keys:?}");
    assert!(!keys.contains(&"board:own:core-008"), "{keys:?}");
    assert!(keys.windows(2).all(|pair| pair[0] < pair[1]), "keys sorted");

    let zeros = vec![0.0; dense_len()];
    let ones = vec![1.0; dense_len()];
    let sparse_in = vec![0.25; keys.len()];
    let network = Network {
        features_version: FEATURES_VERSION,
        catalog: "",
        width: 1,
        mean: &zeros,
        scale: &ones,
        keys: &keys,
        dense_in: &zeros,
        sparse_in: &sparse_in,
        bias: &[0.0],
        head: &[],
        out_w: &[1.0],
        out_b: 0.0,
        linear: &[],
        hand_a: 1.0,
        hand_b: 0.0,
        hand_unit: 100.0,
    };

    let seen = features_of(&fresh, PlayerId::P1);
    assert!(
        seen.sparse.iter().any(|(key, _)| key == "board.type:own:Unit"),
        "{:?}",
        seen.sparse
    );
    assert_eq!(network.correction(&ids_only(&seen)), 0.0, "its id has no row");
    assert!(
        network.correction(&seen) > 0.0,
        "what it shares with core-011 moves it"
    );
    assert_ne!(
        finalist_score_with(
            &fresh,
            PlayerId::P1,
            NextSwing::Seat,
            EvalKind::Learned,
            &Learned::Float(network),
            Prices::NONE,
        ),
        evaluate(&fresh, PlayerId::P1, NextSwing::Seat, &AI_EVAL)
    );
}

/// R3112001: a network fitted before version 3 has rows only for keys that name a card by id, so this
/// build's features give it exactly the correction their id keys alone give it: it reads them as it
/// was fitted. The tiny fixture is such a network, and so is the shipped one until a refit.
#[test]
fn r3112001_a_network_fitted_before_version_3_reads_this_build_s_features_as_it_was_fitted() {
    let solo = state();
    let features = features_of(&solo, PlayerId::P1);
    let ids = ids_only(&features);
    assert!(
        ids.sparse.len() < features.sparse.len(),
        "the features hold the keys of what each card is: {:?}",
        features.sparse
    );
    with_tiny(|network, _| {
        assert!(network.features_version < FEATURES_VERSION);
        assert!(
            network.keys.iter().all(|key| card_id_of_key(key).is_some()),
            "{:?}",
            network.keys
        );
        let correction = network.correction(&features);
        assert_ne!(correction, 0.0, "the network moves this position");
        assert_eq!(correction.to_bits(), network.correction(&ids).to_bits());
    });
    if LEARNED.features_version() < FEATURES_VERSION {
        assert!(
            LEARNED.keys().iter().all(|key| card_id_of_key(key).is_some()),
            "a network fitted on version {} names only card ids",
            LEARNED.features_version()
        );
        assert_eq!(
            LEARNED.correction(&features).to_bits(),
            LEARNED.correction(&ids).to_bits()
        );
    }
}

/// R3112002: `unknown_cards` counts each copy a position names once (a Radiant card in hand by its
/// `hand` key alone, and no attribute key), and of them the ones whose id the network has no row
/// for; `known_card_ids` names each id a network's keys name, once, sorted.
#[test]
fn r3112002_unknown_cards_counts_each_copy_once_and_the_ids_with_no_row() {
    let sparse: Vec<(String, i32)> = [
        ("backrow:enemy:core-041", 1),
        ("board.cost:own:1", 2),
        ("board:own:core-011", 2),
        ("hand:own:core-008", 1),
        ("hand_radiant:own:core-008", 1),
    ]
    .iter()
    .map(|(key, count)| ((*key).to_string(), *count))
    .collect();
    assert_eq!(
        unknown_cards(&sparse, &["core-011"]),
        UnknownCards { named: 4, unknown: 2 }
    );
    assert_eq!(unknown_cards(&sparse, &[]), UnknownCards { named: 4, unknown: 4 });
    with_tiny(|network, _| {
        let known = Learned::Float(*network).known_card_ids();
        assert!(!known.is_empty());
        assert!(
            known.windows(2).all(|pair| pair[0] < pair[1]),
            "sorted, each once"
        );
        for key in network.keys {
            let (_, id) = card_id_of_key(key).expect("an id key");
            assert!(known.binary_search(&id).is_ok(), "{id}");
        }
    });
}
