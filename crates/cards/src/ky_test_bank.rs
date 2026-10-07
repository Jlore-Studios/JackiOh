//! C+ #42 KY's Test's question bank (SPEC §8.7 row 42, R420): its Medium and Hard problems. Easy ones
//! are generated from the rng (`subsystems.easyProblem`, R580). Card data like `catalog.json`: public,
//! shipped in the client bundle, and versioned with the catalog (R388), so changing a problem is a patch.
//! Each entry has four different options, one of them its `answer` (test/classic-plus/042-kys-test.test.ts
//! proves the floor of `KY_TEST_MIN_PROBLEMS` per difficulty). Plain text with Unicode maths.
//!
//! Port of `packages/cards/src/kyTestBank.ts`. TS's `KY_TEST_BANK` array of objects cannot be a `const`
//! (a problem holds `String`s), so it is a read-only `LazyLock`, built once on first use like the
//! crate's `CATALOG`, and never written after.

use std::sync::LazyLock;

use jackioh_engine::config::KyTestDifficulty;
use jackioh_engine::subsystems::ky_test::KyTestProblem;

type Problem = KyTestProblem;

fn problem(difficulty: KyTestDifficulty, id: &str, statement: &str, answer: &str, wrong: &[&str]) -> Problem {
    let mut options = vec![answer.to_string()];
    options.extend(wrong.iter().map(|option| option.to_string()));
    Problem {
        id: id.to_string(),
        difficulty,
        statement: statement.to_string(),
        options,
        answer: answer.to_string(),
    }
}

fn medium(id: &str, statement: &str, answer: &str, wrong: &[&str]) -> Problem {
    problem(KyTestDifficulty::Medium, id, statement, answer, wrong)
}

fn hard(id: &str, statement: &str, answer: &str, wrong: &[&str]) -> Problem {
    problem(KyTestDifficulty::Hard, id, statement, answer, wrong)
}

/// The Medium and Hard problems, in the TS file's order (C+ #42's draw reads this order, R420).
pub static KY_TEST_BANK: LazyLock<Vec<Problem>> = LazyLock::new(|| {
    vec![
        // Medium: double integrals.
        medium("m01", "∫₀¹ ∫₀¹ (x + y) dy dx = ?", "1", &["1/2", "2", "3/2"]),
        medium("m02", "∫₀¹ ∫₀¹ xy dy dx = ?", "1/4", &["1/2", "1", "1/8"]),
        medium("m03", "∫₀² ∫₀³ 1 dy dx = ?", "6", &["5", "3", "9"]),
        medium("m04", "∫₀¹ ∫₀² x dy dx = ?", "1", &["2", "1/2", "4"]),
        medium("m05", "∫₀¹ ∫₀¹ x²y dy dx = ?", "1/6", &["1/3", "1/2", "1/12"]),
        medium("m06", "∫₀¹ ∫₀ˣ 1 dy dx = ?", "1/2", &["1", "1/3", "1/4"]),
        medium("m07", "∫₀¹ ∫₀ˣ x dy dx = ?", "1/3", &["1/2", "1/6", "2/3"]),
        medium("m08", "∫₀² ∫₀¹ (x² + y) dy dx = ?", "11/3", &["8/3", "7/3", "13/3"]),
        medium("m09", "∬ y sin x dA over [0, π] × [0, 1] = ?", "1", &["2", "π", "1/2"]),
        medium("m10", "∫₀¹ ∫₀¹ eˣ⁺ʸ dy dx = ?", "(e − 1)²", &["e² − 1", "e − 1", "2(e − 1)"]),
        medium("m11", "∫₀¹ ∫₀² 6xy² dy dx = ?", "8", &["4", "16", "6"]),
        medium("m12", "∬ 1 dA over the disc x² + y² ≤ 4 = ?", "4π", &["2π", "8π", "16π"]),
        medium("m13", "∫₀^2π ∫₀¹ r dr dθ = ?", "π", &["2π", "π/2", "1"]),
        medium("m14", "∫₀^2π ∫₀² r² dr dθ = ?", "16π/3", &["8π/3", "4π", "8π"]),
        medium("m15", "∬ (x² + y²) dA over the disc x² + y² ≤ 1 = ?", "π/2", &["π", "π/4", "2π"]),
        medium("m16", "∬ x dA over the triangle 0 ≤ y ≤ x ≤ 1 = ?", "1/3", &["1/2", "2/3", "1/6"]),
        medium("m17", "∫₀² ∫₀ˣ 1 dy dx = ?", "2", &["4", "1", "3"]),
        medium("m18", "∫₀¹ ∫₀^x² x dy dx = ?", "1/4", &["1/3", "1/5", "1/2"]),
        medium("m19", "∫₁² ∫₁² 1/(xy) dy dx = ?", "(ln 2)²", &["2 ln 2", "ln 2", "1/4"]),
        medium("m20", "∫₀¹ ∫₀¹ (2x + 3y) dy dx = ?", "5/2", &["5", "3/2", "2"]),
        medium("m21", "∫₀³ ∫₀² (x + 1) dy dx = ?", "15", &["12", "9", "18"]),
        medium("m22", "∬ sin x cos y dA over [0, π/2] × [0, π/2] = ?", "1", &["π/2", "0", "2"]),
        medium("m23", "∫₀¹ ∫₀¹ x eʸ dy dx = ?", "(e − 1)/2", &["e − 1", "e/2", "(e + 1)/2"]),
        medium("m24", "The volume under z = 4 − x² − y² above the disc x² + y² ≤ 4 = ?", "8π", &["16π", "4π", "32π/3"]),
        medium("m25", "∫₀¹ ∫₀¹ (x − y)² dy dx = ?", "1/6", &["1/3", "0", "1/12"]),
        medium("m26", "∫₀² ∫₀^(2−x) x dy dx = ?", "4/3", &["2/3", "2", "8/3"]),
        medium("m27", "∫₀¹ ∫₀¹ 1/(1 + x) dy dx = ?", "ln 2", &["1/2", "ln 3", "1"]),
        medium("m28", "∬ sin(x + y) dA over [0, π] × [0, π] = ?", "0", &["2", "4", "π"]),
        medium("m29", "∫₀¹ ∫₀¹ x²y² dy dx = ?", "1/9", &["1/6", "1/3", "1/4"]),
        medium("m30", "∬ xy dA over [0, 2] × [0, 2] = ?", "4", &["2", "8", "16"]),
        medium("m31", "∫₀¹ ∫₀^√(1−x²) 1 dy dx = ?", "π/4", &["π/2", "1", "π"]),
        medium("m32", "∬ eˣ⁺ʸ dA over [0, ln 2] × [0, ln 3] = ?", "2", &["6", "1", "5"]),
        medium("m33", "∫₀¹ ∫ₓ¹ e^(y²) dy dx = ? (swap the order)", "(e − 1)/2", &["e − 1", "e/2", "(e² − 1)/2"]),

        // Hard: linear algebra.
        hard("h01", "The eigenvalues of [[2, 1], [1, 2]] are?", "1 and 3", &["2 and 2", "0 and 4", "−1 and 3"]),
        hard("h02", "The rank of [[1, 2, 3], [2, 4, 6], [1, 0, 1]] is?", "2", &["1", "3", "0"]),
        hard("h03", "det [[1, 2, 3], [0, 4, 5], [0, 0, 6]] = ?", "24", &["12", "15", "6"]),
        hard("h04", "A is 3 × 3 with det A = 2. det(2A) = ?", "16", &["4", "8", "32"]),
        hard("h05", "A is 3 × 3 with eigenvalues 1, 2 and 4. det A = ?", "8", &["7", "6", "16"]),
        hard("h06", "A 4 × 6 matrix has rank 3. The dimension of its null space is?", "3", &["1", "2", "4"]),
        hard("h07", "The inverse of [[2, 0], [0, 4]] is?", "[[1/2, 0], [0, 1/4]]", &["[[−2, 0], [0, −4]]", "[[4, 0], [0, 2]]", "[[1/4, 0], [0, 1/2]]"]),
        hard("h08", "A is an invertible n × n matrix with A² = A. Then A = ?", "I", &["0", "−I", "2I"]),
        hard("h09", "The eigenvalues of the rotation [[0, −1], [1, 0]] are?", "i and −i", &["1 and −1", "0 and 1", "1, twice"]),
        hard("h10", "Any three vectors in ℝ² are?", "linearly dependent", &["linearly independent", "a basis of ℝ²", "pairwise orthogonal"]),
        // Hard: Markov chains.
        hard("h11", "A chain has P = [[0.5, 0.5], [0.2, 0.8]]. Its stationary distribution is?", "(2/7, 5/7)", &["(5/7, 2/7)", "(1/2, 1/2)", "(2/5, 3/5)"]),
        hard("h12", "The chain with P = [[0, 1], [1, 0]] has period?", "2", &["1", "0", "∞"]),
        hard("h13", "A chain has P = [[0.9, 0.1], [0.3, 0.7]]. Its stationary distribution is?", "(3/4, 1/4)", &["(1/4, 3/4)", "(9/10, 1/10)", "(1/2, 1/2)"]),
        hard("h14", "A fair random walk on {0, 1, 2, 3}, absorbed at 0 and 3, starts at 1. P(it is absorbed at 3) = ?", "1/3", &["1/2", "2/3", "1/4"]),
        hard("h15", "A chain has P = [[0.9, 0.1], [0.3, 0.7]]. The expected return time to state 1 is?", "4/3", &["4", "3/4", "10/9"]),
        hard("h16", "Gambler's ruin with a fair coin: start at 2, stop at 0 or 5. P(reach 5 first) = ?", "2/5", &["1/2", "3/5", "1/5"]),
        hard("h17", "A fair random walk on {0, …, 4}, absorbed at both ends, starts at 2. The expected steps to absorption are?", "4", &["2", "8", "6"]),
        hard("h18", "P = [[1/2, 1/2, 0], [1/2, 1/2, 0], [0, 0, 1]]. How many communicating classes?", "2", &["1", "3", "0"]),
        hard("h19", "P = [[0, 1], [1/2, 1/2]] on states 1 and 2. P(X₂ = 2 | X₀ = 2) = ?", "3/4", &["1/2", "1/4", "1"]),
        // Hard: statistics.
        hard("h20", "Z ~ N(0, 1). P(|Z| ≤ 1.96) ≈ ?", "0.95", &["0.90", "0.975", "0.68"]),
        hard("h21", "25 independent draws with variance 100. The variance of their mean is?", "4", &["2", "20", "100"]),
        hard("h22", "7 successes in 20 Bernoulli trials. The maximum-likelihood estimate of p is?", "0.35", &["0.5", "0.7", "0.3"]),
        hard("h23", "X ~ Exponential with rate λ = 2. E[X] = ?", "1/2", &["2", "1/4", "4"]),
        hard("h24", "X ~ Poisson(3). Var(X) = ?", "3", &["9", "√3", "1/3"]),
        hard("h25", "The unbiased sample variance divides the sum of squared deviations by?", "n − 1", &["n", "n + 1", "√n"]),
        hard("h26", "Two fair dice. P(the sum is 7) = ?", "1/6", &["1/12", "7/36", "1/36"]),
        hard("h27", "X and Y are independent, Var(X) = 4 and Var(Y) = 9. Var(X − Y) = ?", "13", &["5", "−5", "36"]),
        hard("h28", "1% have a disease; a test catches 99% of cases and falsely flags 1% of the healthy. P(ill | positive) = ?", "1/2", &["0.99", "0.01", "0.0099"]),
        hard("h29", "σ = 10 known, n = 100, x̄ = 50. The 95% confidence interval for the mean is?", "(48.04, 51.96)", &["(30.4, 69.6)", "(49.804, 50.196)", "(48.36, 51.64)"]),
        // Hard: PDEs.
        hard("h30", "uₜ = k·uₓₓ on [0, π], u(0, t) = u(π, t) = 0, u(x, 0) = sin x. u(x, t) = ?", "e^(−kt) sin x", &["e^(kt) sin x", "e^(−t) sin(kx)", "cos(kt) sin x"]),
        hard("h31", "The wave equation uₜₜ − uₓₓ = 0 is?", "hyperbolic", &["parabolic", "elliptic", "first order"]),
        hard("h32", "Laplace's equation Δu = 0 is?", "elliptic", &["hyperbolic", "parabolic", "first order"]),
        hard("h33", "uₜₜ = c²uₓₓ with u(x, 0) = f(x) and uₜ(x, 0) = 0. u(x, t) = ?", "(f(x − ct) + f(x + ct))/2", &["f(x − ct) + f(x + ct)", "f(x − ct)", "f(x) cos(ct)"]),
        hard("h34", "uₜ = uₓₓ on [0, L] with u = 0 at both ends. Separation of variables gives the eigenvalues?", "(nπ/L)²", &["nπ/L", "(nL/π)²", "n²π"]),
        hard("h35", "Which function is harmonic?", "x² − y²", &["x² + y²", "x³ + y³", "x²y"]),
        hard("h36", "uₜ + 3uₓ = 0 with u(x, 0) = g(x). u(x, t) = ?", "g(x − 3t)", &["g(x + 3t)", "g(3x − t)", "e^(−3t) g(x)"]),
        // Hard: proofs.
        hard("h37", "To prove √2 irrational, suppose √2 = p/q in lowest terms. The contradiction is that?", "p and q are both even", &["p = q", "q = 0", "p² = q"]),
        hard("h38", "Inducting 1 + 2 + ⋯ + n = n(n + 1)/2, the step shows n(n + 1)/2 + (n + 1) equals?", "(n + 1)(n + 2)/2", &["n(n + 2)/2", "(n + 1)²/2", "(n² + 1)/2"]),
        hard("h39", "Euclid: given primes p₁, …, pₖ, which number always has a prime factor outside the list?", "p₁p₂⋯pₖ + 1", &["p₁ + p₂ + ⋯ + pₖ", "pₖ²", "2pₖ"]),
        hard("h40", "By the pigeonhole principle, the fewest people certain to include two born in the same month is?", "13", &["12", "7", "24"]),
        hard("h41", "Cantor's diagonal argument proves that?", "ℝ is uncountable", &["ℚ is uncountable", "√2 is irrational", "there are infinitely many primes"]),
    ]
});
