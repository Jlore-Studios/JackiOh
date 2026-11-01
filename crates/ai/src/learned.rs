//! The learned evaluation in the search (#466, #1010; `docs/learned-eval.md` D4, D6, D7, D9): a small
//! network over `features`, fitted by `training/ml`, that corrects the hand eval where the search scores.
//! With `EvalKind::Learned`, `finalist_score` scores finalists (`LEARNED_REACH` R1012002, `share` R1012001).
//! Skip connection maps back to eval points, preserving constants and ladder bot temperature (R1452).
//! Duos (R964000) and tutorial opponents (R1012003) retain hand eval.
//! Sparse keys missing from the table default to dense stats alone (#915). Integer weights used in #1011.

use jackioh_engine::{AI_TUTORIAL, GameState, Handicap, PlayerId};

use crate::config::{AI_EVAL, AI_EVAL_KIND, EvalKind, LEARNED_CORRECTION_SHARE, LEARNED_REACH, LearnedReach};
use crate::evaluate::{NextSwing, Prices, evaluate_priced};
use crate::features::{Features, features_of};

/// One dense layer of the head: `out = clip(w · input + b)`, `w` row-major `outputs × inputs`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer<'a> {
    pub inputs: usize,
    pub w: &'a [f64],
    pub b: &'a [f64],
}

/// A fitted network, borrowed: `LEARNED` holds the compiled-in one, and a test can lend one it read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Network<'a> {
    pub features_version: u32,
    pub catalog: &'a str,
    pub width: usize,
    /// The dense features' training mean and spread: input `j` is `(dense[j] − mean[j]) / scale[j]`.
    pub mean: &'a [f64],
    pub scale: &'a [f64],
    /// The sparse keys the network has a row for, sorted: row `r` of `sparse_in` is `keys[r]`'s.
    pub keys: &'a [&'a str],
    /// `dense.len() × width`, row-major by input.
    pub dense_in: &'a [f64],
    /// `keys.len() × width`, row-major by key.
    pub sparse_in: &'a [f64],
    /// The first layer's bias, `width` of them.
    pub bias: &'a [f64],
    pub head: &'a [Layer<'a>],
    pub out_w: &'a [f64],
    pub out_b: f64,
    /// (b)'s linear term over the standardized dense features, one per input, added to the output.
    pub linear: &'a [f64],
    /// The skip connection: `a·hand/hand_unit + hand_b`.
    pub hand_a: f64,
    pub hand_b: f64,
    pub hand_unit: f64,
}

mod int;
pub use int::*;

/// The network the search runs: a float one, which part 5 screens, or the integer one that ships
/// (#1011). `LEARNED` is whichever `src/learned/weights.json` holds (`"format"` 1 or 2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Learned<'a> {
    Float(Network<'a>),
    Int(IntNetwork<'a>),
}

impl Learned<'_> {
    pub fn features_version(&self) -> u32 {
        match self {
            Learned::Float(net) => net.features_version,
            Learned::Int(net) => net.features_version,
        }
    }

    /// How many dense features it reads.
    pub fn dense_len(&self) -> usize {
        match self {
            Learned::Float(net) => net.mean.len(),
            Learned::Int(net) => net.dense,
        }
    }

    /// The sparse keys it has a row for, sorted.
    pub fn keys(&self) -> &[&str] {
        match self {
            Learned::Float(net) => net.keys,
            Learned::Int(net) => net.keys,
        }
    }

    pub fn correction(&self, features: &Features) -> f64 {
        match self {
            Learned::Float(net) => net.correction(features),
            Learned::Int(net) => net.correction(features),
        }
    }

    pub fn points(&self, hand: f64, features: &Features) -> f64 {
        match self {
            Learned::Float(net) => net.points(hand, features),
            Learned::Int(net) => net.points(hand, features),
        }
    }
}

include!(concat!(env!("OUT_DIR"), "/learned_weights.rs"));

/// The ReLU clipped at one, as comparisons: below zero is zero, above one is one, NaN stays NaN.
/// Written out rather than `f64::clamp` so the comparisons R1010001 names are the code itself.
#[allow(clippy::manual_clamp)]
fn clip(x: f64) -> f64 {
    if x < 0.0 {
        0.0
    } else if x > 1.0 {
        1.0
    } else {
        x
    }
}

impl Network<'_> {
    /// `net(x)`: the logit the network adds to the calibrated hand eval. Zero when the features are
    /// not the layout the network was fitted on, which a test catches long before a game does.
    pub fn correction(&self, features: &Features) -> f64 {
        let width = self.width;
        if features.dense.len() != self.mean.len() || self.bias.len() != width {
            return 0.0;
        }
        let mut hidden: Vec<f64> = self.bias.to_vec();
        let mut linear = 0.0;
        for (j, value) in features.dense.iter().enumerate() {
            let x = (f64::from(*value) - self.mean[j]) / self.scale[j];
            let row = &self.dense_in[j * width..(j + 1) * width];
            for (h, w) in hidden.iter_mut().zip(row) {
                *h += x * w;
            }
            linear += x * self.linear.get(j).copied().unwrap_or(0.0);
        }
        for (key, count) in &features.sparse {
            if let Ok(r) = self.keys.binary_search(&key.as_str()) {
                let row = &self.sparse_in[r * width..(r + 1) * width];
                let count = f64::from(*count);
                for (h, w) in hidden.iter_mut().zip(row) {
                    *h += count * w;
                }
            }
        }
        for h in hidden.iter_mut() {
            *h = clip(*h);
        }
        for layer in self.head {
            let outputs = layer.b.len();
            let mut next: Vec<f64> = layer.b.to_vec();
            for (k, out) in next.iter_mut().enumerate().take(outputs) {
                let row = &layer.w[k * layer.inputs..(k + 1) * layer.inputs];
                for (w, h) in row.iter().zip(&hidden) {
                    *out += w * h;
                }
                *out = clip(*out);
            }
            hidden = next;
        }
        let mut out = self.out_b;
        for (w, h) in self.out_w.iter().zip(&hidden) {
            out += w * h;
        }
        out + linear
    }

    /// The position's value in the hand eval's points: `hand + (b + net(x))·U/a`.
    pub fn points(&self, hand: f64, features: &Features) -> f64 {
        if self.hand_a == 0.0 {
            return hand;
        }
        hand + (self.hand_b + self.correction(features)) * self.hand_unit / self.hand_a
    }
}

/// The score a finalist gets where `reply_score` scores it: the hand eval, or with
/// `EvalKind::Learned` the network's correction of it at `LEARNED_CORRECTION_SHARE` (1v1 only, D9;
/// R1012001).
pub fn finalist_score(state: &GameState, seat: PlayerId, next: NextSwing, prices: &Prices) -> f64 {
    finalist_score_with(state, seat, next, AI_EVAL_KIND, &LEARNED, prices)
}

/// R1012002: the score a complete line gets where the beam closes it (`static_score`): a finalist's
/// with `LEARNED_REACH` at `Lines` or past it, so the network also picks the finalists; the hand eval
/// otherwise.
pub fn line_score(state: &GameState, seat: PlayerId, next: NextSwing, prices: &Prices) -> f64 {
    line_score_with(state, seat, next, LEARNED_REACH, AI_EVAL_KIND, &LEARNED, prices)
}

/// R1012002: the score an open line gets where the beam ranks it at each depth: a finalist's with
/// `LEARNED_REACH` at `Beam`, the hand eval otherwise.
pub fn open_score(state: &GameState, seat: PlayerId, next: NextSwing, prices: &Prices) -> f64 {
    open_score_with(state, seat, next, LEARNED_REACH, AI_EVAL_KIND, &LEARNED, prices)
}

/// `line_score` under a given reach, switch and network, for tests and screens.
pub fn line_score_with(
    state: &GameState,
    seat: PlayerId,
    next: NextSwing,
    reach: LearnedReach,
    kind: EvalKind,
    network: &Learned<'_>,
    prices: &Prices,
) -> f64 {
    if reach >= LearnedReach::Lines {
        finalist_score_with(state, seat, next, kind, network, prices)
    } else {
        evaluate_priced(state, seat, next, &AI_EVAL, prices)
    }
}

/// `open_score` under a given reach, switch and network, for tests and screens.
pub fn open_score_with(
    state: &GameState,
    seat: PlayerId,
    next: NextSwing,
    reach: LearnedReach,
    kind: EvalKind,
    network: &Learned<'_>,
    prices: &Prices,
) -> f64 {
    if reach >= LearnedReach::Beam {
        finalist_score_with(state, seat, next, kind, network, prices)
    } else {
        evaluate_priced(state, seat, next, &AI_EVAL, prices)
    }
}

/// R1012003: whether a seat plays the tutorial opponent's tier: its handicap is `AI_TUTORIAL`'s
/// resources, whatever the lesson adds that is no resource (R2201000's floor, R2201001's pool hold,
/// R2533000's bar and R2533001's copies, which a lesson played before them lacks; R2433000's Core-only
/// bar, which a lesson played under it held, folds to no field now).
pub fn is_tutorial_opponent(handicap: Option<Handicap>) -> bool {
    let resources = |handicap: Handicap| Handicap {
        hero_floor: None,
        pool_hold: None,
        tutorial_cards_only: None,
        deck_copies: None,
        ..handicap
    };
    handicap.is_some_and(|handicap| resources(handicap) == resources(AI_TUTORIAL))
}

/// `finalist_score` under a given switch and network, for tests and screens.
pub fn finalist_score_with(
    state: &GameState,
    seat: PlayerId,
    next: NextSwing,
    kind: EvalKind,
    network: &Learned<'_>,
    prices: &Prices,
) -> f64 {
    let hand = evaluate_priced(state, seat, next, &AI_EVAL, prices);
    match kind {
        EvalKind::Hand => hand,
        EvalKind::Learned => {
            // R1012003: the tutorial's opponent (R290) plays a game the network never saw, at 3 mana
            // with a 12-card deck, as a Duos duel is one (D9).
            if state.result.is_some()
                || state.players[seat].duos.is_some()
                || is_tutorial_opponent(state.players[seat].handicap)
            {
                return hand;
            }
            // R1012001: the search takes the correction at its share.
            let points = network.points(hand, &features_of(state, seat));
            hand + LEARNED_CORRECTION_SHARE * (points - hand)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::{FEATURES_VERSION, dense_len};

    /// R1010000: the network compiled in was fitted on this build's feature layout.
    #[test]
    fn r1010000_the_compiled_network_was_fitted_on_this_feature_version_and_layout() {
        assert_eq!(LEARNED.features_version(), FEATURES_VERSION);
        assert_eq!(LEARNED.dense_len(), dense_len());
        match LEARNED {
            Learned::Float(net) => {
                assert_eq!(net.scale.len(), dense_len());
                assert_eq!(net.dense_in.len(), dense_len() * net.width);
                assert_eq!(net.sparse_in.len(), net.keys.len() * net.width);
            }
            Learned::Int(net) => {
                assert_eq!(net.dense_in.len(), dense_len() * net.width);
                assert_eq!(net.sparse_in.len(), net.keys.len() * net.width);
            }
        }
        assert!(
            LEARNED.keys().windows(2).all(|pair| pair[0] < pair[1]),
            "keys sorted"
        );
        assert_eq!(LEARNED_DENSE_NAMES.len(), dense_len());
        assert_eq!(
            LEARNED_DENSE_NAMES
                .iter()
                .map(|name| name.to_string())
                .collect::<Vec<_>>(),
            crate::features::dense_names()
        );
    }

    #[test]
    fn clip_is_comparisons() {
        assert_eq!(clip(-0.5), 0.0);
        assert_eq!(clip(0.25), 0.25);
        assert_eq!(clip(3.0), 1.0);
        assert!(clip(-0.0) == 0.0);
    }
}
