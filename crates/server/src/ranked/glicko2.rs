//! The hidden rating (SPEC §9.12, R603): Glicko-2, as Mark Glickman's "Example of the Glicko-2
//! system" (2013) writes it, step for step (← `apps/server/src/ranked/glicko2.ts`). Every number is
//! `src/config.rs`'s.
//!
//! One rated game is one rating period: each side is updated against the other's rating from before
//! the game, so the two updates do not depend on which is computed first. A draw scores 0.5 for both,
//! a concede and a disconnect are losses (§2.5's endings decide who won; this file only reads the
//! score).
//!
//! Pure and deterministic: the same inputs give the same output on every machine, because the only
//! operations are IEEE-754 arithmetic, `exp`, `ln` and `sqrt`, and the volatility iteration runs a
//! fixed rule to a fixed tolerance. `tests/api/glicko2.rs` checks it against Glickman's worked
//! example and against reference values computed independently at 50 digits.
//!
//! Glicko-2's update depends only on rating differences, so the scale's centre (Glickman's 1500) is
//! any fixed number; this file uses `RATING_START`, and an Elo rating carries over as it is (R603).

use serde::{Deserialize, Serialize};

use crate::config::{
    GLICKO_CONVERGENCE, GLICKO_MAX_ITERATIONS, GLICKO_SCALE, GLICKO_TAU, RATING_DEVIATION_START,
    RATING_START, RATING_VOLATILITY_START,
};

/// One side's hidden rating: Glicko-2's r, RD and σ, on the displayed scale.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Glicko {
    pub rating: f64,
    pub deviation: f64,
    pub volatility: f64,
}

/// A game's score for the side it is read from: a win (1), a draw (0.5) or a loss (0). TS's
/// `0 | 0.5 | 1`, a numeric union, is an `f64` holding one of those three values.
pub type Score = f64;

/// One game of a rating period: the opponent's rating before it, and the score against them.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RatedOpponent {
    pub opponent: Glicko,
    pub score: Score,
}

/// The rating a profile starts with (R603).
pub const START_GLICKO: Glicko = Glicko {
    rating: RATING_START,
    deviation: RATING_DEVIATION_START,
    volatility: RATING_VOLATILITY_START,
};

/// Step 3's g(φ): how much an opponent's own uncertainty discounts a game against them.
fn g(phi: f64) -> f64 {
    1.0 / (1.0 + (3.0 * phi * phi) / (std::f64::consts::PI * std::f64::consts::PI)).sqrt()
}

/// Step 3's E(μ, μj, φj): the expected score against an opponent.
fn expected(mu: f64, mu_j: f64, phi_j: f64) -> f64 {
    1.0 / (1.0 + (-g(phi_j) * (mu - mu_j)).exp())
}

/// Step 5: the new volatility, by the Illinois algorithm exactly as Glickman's §5 sets it out,
/// including the bracket's starting points.
fn next_volatility(sigma: f64, phi: f64, v: f64, delta: f64, tau: f64) -> f64 {
    let a = (sigma * sigma).ln();
    let phi2 = phi * phi;
    let f = |x: f64| -> f64 {
        let ex = x.exp();
        let denominator = phi2 + v + ex;
        (ex * (delta * delta - phi2 - v - ex)) / (2.0 * denominator * denominator) - (x - a) / (tau * tau)
    };

    let mut big_a = a;
    let mut big_b: f64;
    if delta * delta > phi2 + v {
        big_b = (delta * delta - phi2 - v).ln();
    } else {
        let mut k: u32 = 1;
        while f(a - f64::from(k) * tau) < 0.0 && k < GLICKO_MAX_ITERATIONS {
            k += 1;
        }
        big_b = a - f64::from(k) * tau;
    }

    let mut f_a = f(big_a);
    let mut f_b = f(big_b);
    let mut step: u32 = 0;
    while (big_b - big_a).abs() > GLICKO_CONVERGENCE && step < GLICKO_MAX_ITERATIONS {
        let big_c = big_a + ((big_a - big_b) * f_a) / (f_b - f_a);
        let f_c = f(big_c);
        if f_c * f_b <= 0.0 {
            big_a = big_b;
            f_a = f_b;
        } else {
            f_a /= 2.0;
        }
        big_b = big_c;
        f_b = f_c;
        step += 1;
    }
    (big_a / 2.0).exp()
}

/// Glickman's steps 2–8 for one player over one rating period. A period with no games only widens
/// the deviation (step 6 alone); the server never rates one, but the formula is defined there too.
///
/// TS's `tau` defaulted to `GLICKO_TAU`; Rust has no default arguments, so callers pass it.
pub fn glicko2_period(player: &Glicko, games: &[RatedOpponent], tau: f64) -> Glicko {
    // Step 2: onto the Glicko-2 scale.
    let mu = (player.rating - RATING_START) / GLICKO_SCALE;
    let phi = player.deviation / GLICKO_SCALE;
    let sigma = player.volatility;

    if games.is_empty() {
        return Glicko {
            rating: player.rating,
            deviation: (phi * phi + sigma * sigma).sqrt() * GLICKO_SCALE,
            volatility: sigma,
        };
    }

    // Steps 3 and 4: the estimated variance v and the improvement Δ.
    let mut v_inverse = 0.0;
    let mut improvement = 0.0;
    for game in games {
        let mu_j = (game.opponent.rating - RATING_START) / GLICKO_SCALE;
        let phi_j = game.opponent.deviation / GLICKO_SCALE;
        let e = expected(mu, mu_j, phi_j);
        let g_j = g(phi_j);
        v_inverse += g_j * g_j * e * (1.0 - e);
        improvement += g_j * (game.score - e);
    }
    let v = 1.0 / v_inverse;
    let delta = v * improvement;

    // Steps 5 to 7: the new volatility, then the deviation and rating it allows.
    let sigma_next = next_volatility(sigma, phi, v, delta, tau);
    let phi_star = (phi * phi + sigma_next * sigma_next).sqrt();
    let phi_next = 1.0 / (1.0 / (phi_star * phi_star) + 1.0 / v).sqrt();
    let mu_next = mu + phi_next * phi_next * improvement;

    // Step 8: back onto the displayed scale.
    Glicko {
        rating: mu_next * GLICKO_SCALE + RATING_START,
        deviation: phi_next * GLICKO_SCALE,
        volatility: sigma_next,
    }
}

/// `rate_game`'s answer: both sides' ratings after the game (TS `{ a: Glicko; b: Glicko }`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct RatedGame {
    pub a: Glicko,
    pub b: Glicko,
}

/// R603: one rated game between `a` and `b`, `score_a` being a's score (b's is `1 - score_a`). Each
/// side is rated against the other's rating from before the game.
pub fn rate_game(a: &Glicko, b: &Glicko, score_a: Score) -> RatedGame {
    let score_b: Score = 1.0 - score_a;
    RatedGame {
        a: glicko2_period(a, &[RatedOpponent { opponent: *b, score: score_a }], GLICKO_TAU),
        b: glicko2_period(b, &[RatedOpponent { opponent: *a, score: score_b }], GLICKO_TAU),
    }
}
