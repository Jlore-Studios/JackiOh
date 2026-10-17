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
    assert_eq!(fixture.network.features_version, FEATURES_VERSION);
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

/// R1010002: with the switch on, a 1v1 finalist takes the network's score and a Duos duel's keeps
/// the hand eval's (D9, R964000).
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
        let learned = finalist_score_with(&solo, PlayerId::P1, NextSwing::Seat, EvalKind::Learned, network);
        let features = features_of(&solo, PlayerId::P1);
        assert_eq!(learned, network.points(hand, &features));
        assert_ne!(learned, hand, "the network moves this position");

        let duos_hand = evaluate(&duos, PlayerId::P1, NextSwing::Seat, &AI_EVAL);
        assert_eq!(
            finalist_score_with(&duos, PlayerId::P1, NextSwing::Seat, EvalKind::Learned, network),
            duos_hand
        );
    });
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
            finalist_score_with(&state, PlayerId::P1, next, EvalKind::Hand, &LEARNED),
            hand
        );
        if LEARNED.keys.is_empty()
            && LEARNED.hand_b == 0.0
            && LEARNED.out_w.iter().all(|w| *w == 0.0)
            && LEARNED.out_b == 0.0
        {
            assert_eq!(
                finalist_score_with(&state, PlayerId::P1, next, EvalKind::Learned, &LEARNED),
                hand
            );
        }
    }
    assert_eq!(finalist_score(&state, PlayerId::P1, NextSwing::Enemy), {
        match AI_EVAL_KIND {
            EvalKind::Hand => evaluate(&state, PlayerId::P1, NextSwing::Enemy, &AI_EVAL),
            EvalKind::Learned => finalist_score_with(
                &state,
                PlayerId::P1,
                NextSwing::Enemy,
                EvalKind::Learned,
                &LEARNED,
            ),
        }
    });
}
