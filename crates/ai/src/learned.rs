//! The learned evaluation in the search (#466, #1010; `docs/learned-eval.md` D4, D6, D7, D9): a small
//! network over `features`, fitted by `training/ml`, that corrects the hand eval where the search
//! finally scores a line.
//!
//! - **Where.** `AI_EVAL_KIND` (`config.rs`) chooses. With `EvalKind::Hand`, the default, nothing
//!   changes. With `EvalKind::Learned`, `finalist_score` scores the finalists after the opponent's
//!   reply (`reply_score`, the lines `score_line` re-scores on every determinization): D6's option B.
//!   The beam's pruning keeps the hand eval, so the network picks among the lines the hand eval found.
//! - **What.** The network learned the seat's chance to win as `a·hand/U + b + net(x)` (D4's skip
//!   connection, `training/ml/jackioh_ml/models.py`). Mapped back onto the hand eval's points, that is
//!   `hand + (b + net(x))·U/a` (`Network::points`). So `unspent_mana`, the lethal constants and a
//!   ladder bot's `temperature` (R1452), all in evaluation points, keep their meaning.
//! - **Which games.** 1v1 only. A Duos duel keeps the hand eval (D9, R964000): the network never saw
//!   a team's pool. A finished game keeps its won or lost score.
//! - **Arithmetic.** `f64`, every loop in one fixed order, ReLU written as comparisons (never
//!   `f64::max`, so a signed zero cannot differ), and no libm function. That is the determinism the
//!   hand eval's own `f64` rests on (D7). Part 6 (#1011) swaps it for integers before it ships.
//! - **Weights.** `src/learned/weights.json`, compiled in by `build.rs` as `LEARNED` (rule 4: data
//!   reaches the crate at compile time). It records `FEATURES_VERSION`, the catalog it was fitted on,
//!   the id-to-row table (`keys`) and the scale. A feature key the table lacks is no input at all, so a
//!   new or custom card (#915) is valued by its dense stats alone (D5). The checked-in network may be
//!   the null one, whose correction is zero everywhere, until a fit replaces it.

use jackioh_engine::{GameState, PlayerId};

use crate::config::{AI_EVAL, AI_EVAL_KIND, EvalKind};
use crate::evaluate::{NextSwing, evaluate};
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
    /// The skip connection: `a·hand/hand_unit + hand_b`.
    pub hand_a: f64,
    pub hand_b: f64,
    pub hand_unit: f64,
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
        for (j, value) in features.dense.iter().enumerate() {
            let x = (f64::from(*value) - self.mean[j]) / self.scale[j];
            let row = &self.dense_in[j * width..(j + 1) * width];
            for (h, w) in hidden.iter_mut().zip(row) {
                *h += x * w;
            }
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
        out
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
/// `EvalKind::Learned` the network's correction of it (1v1 only, D9).
pub fn finalist_score(state: &GameState, seat: PlayerId, next: NextSwing) -> f64 {
    finalist_score_with(state, seat, next, AI_EVAL_KIND, &LEARNED)
}

/// `finalist_score` under a given switch and network, for tests and screens.
pub fn finalist_score_with(
    state: &GameState,
    seat: PlayerId,
    next: NextSwing,
    kind: EvalKind,
    network: &Network<'_>,
) -> f64 {
    let hand = evaluate(state, seat, next, &AI_EVAL);
    match kind {
        EvalKind::Hand => hand,
        EvalKind::Learned => {
            if state.result.is_some() || state.players[seat].duos.is_some() {
                return hand;
            }
            network.points(hand, &features_of(state, seat))
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
        assert_eq!(LEARNED.features_version, FEATURES_VERSION);
        assert_eq!(LEARNED.mean.len(), dense_len());
        assert_eq!(LEARNED.scale.len(), dense_len());
        assert_eq!(LEARNED.dense_in.len(), dense_len() * LEARNED.width);
        assert_eq!(LEARNED.sparse_in.len(), LEARNED.keys.len() * LEARNED.width);
        assert!(
            LEARNED.keys.windows(2).all(|pair| pair[0] < pair[1]),
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
