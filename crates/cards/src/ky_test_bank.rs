//! C+ #42 KY's Test's question bank (SPEC §8.7 row 42, R420): Medium and Hard problems (easy from RNG; R580, R2179003).
//! Card data versioned with catalog (R388). Patch v0.3.10 (#831) hardened topics.
//! Issue #2179 (R2179003) moved v0.5.2 problems to Medium, Hard to graduate math (R767000 kept script).

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

/// R2179003: the bank since issue #2179 (R420): Medium problems then graduate Hard.
pub static KY_TEST_BANK: LazyLock<Vec<Problem>> = LazyLock::new(|| {
    let mut bank: Vec<Problem> = KY_TEST_BANK_V0_5_2
        .iter()
        .cloned()
        .map(|problem| Problem {
            difficulty: KyTestDifficulty::Medium,
            ..problem
        })
        .collect();
    bank.extend(graduate());
    bank
});

/// Hard problems since #2179: graduate qualifying exam problems.
fn graduate() -> Vec<Problem> {
    vec![
        // Complex analysis.
        hard("g01", "∫₋∞^∞ dx/(1 + x⁴) = ?", "π/√2", &["π/2", "π/4", "π√2"]),
        hard(
            "g02",
            "The residue of e^(1/z) at z = 0 is?",
            "1",
            &["0", "e", "1/2"],
        ),
        hard(
            "g03",
            "∮ dz/(z² + 1) around |z| = 2, counterclockwise, = ?",
            "0",
            &["2πi", "πi", "4πi"],
        ),
        hard(
            "g04",
            "By Rouché's theorem, the number of zeros of z⁵ + 3z + 1 in |z| < 1 is?",
            "1",
            &["0", "3", "5"],
        ),
        hard("g05", "ζ(−1) = ?", "−1/12", &["0", "1/12", "−1/2"]),
        // Real and harmonic analysis.
        hard("g06", "Σₙ₌₁^∞ 1/n⁴ = ?", "π⁴/90", &["π⁴/96", "π⁴/72", "π⁴/45"]),
        hard(
            "g07",
            "Σₙ₌₁^∞ (−1)ⁿ⁺¹/n² = ?",
            "π²/12",
            &["π²/6", "π²/8", "π²/24"],
        ),
        hard(
            "g08",
            "The radius of convergence of Σ (n!/nⁿ) zⁿ is?",
            "e",
            &["1", "1/e", "∞"],
        ),
        hard("g09", "∫₀^∞ (sin x)/x dx = ?", "π/2", &["π", "1", "π/4"]),
        hard(
            "g10",
            "With f̂(ξ) = ∫ f(x) e^(−2πixξ) dx, the Fourier transform of e^(−πx²) is?",
            "e^(−πξ²)",
            &["e^(−ξ²/4)", "√π e^(−π²ξ²)", "e^(−2πξ²)"],
        ),
        // Measure theory and functional analysis.
        hard(
            "g11",
            "The Hausdorff dimension of the middle-thirds Cantor set is?",
            "log 2 / log 3",
            &["1/2", "2/3", "log 3 / log 2"],
        ),
        hard(
            "g12",
            "For which p ≥ 1 is f(x) = x^(−1/2) on (0, 1) in Lᵖ?",
            "1 ≤ p < 2",
            &["1 ≤ p ≤ 2", "p > 2", "every p ≥ 1"],
        ),
        hard(
            "g13",
            "The dual space of ℓ¹ is isometrically isomorphic to?",
            "ℓ^∞",
            &["ℓ¹", "c₀", "ℓ²"],
        ),
        hard(
            "g14",
            "The spectrum of the unilateral right shift on ℓ²(ℕ) is?",
            "the closed unit disc",
            &["the unit circle", "{0}", "{1}"],
        ),
        hard(
            "g15",
            "The cardinality of the set of continuous functions ℝ → ℝ is?",
            "𝔠 = 2^ℵ₀",
            &["ℵ₀", "2^𝔠", "uncountable but less than 𝔠"],
        ),
        // Topology.
        hard(
            "g16",
            "The fundamental group of the Klein bottle is?",
            "⟨a, b | abab⁻¹⟩",
            &["ℤ × ℤ", "ℤ/2", "ℤ * ℤ"],
        ),
        hard("g17", "H₁(ℝP²; ℤ) = ?", "ℤ/2", &["0", "ℤ", "ℤ²"]),
        hard(
            "g18",
            "The Euler characteristic of a closed orientable surface of genus 3 is?",
            "−4",
            &["−2", "−6", "4"],
        ),
        hard("g19", "π₃(S²) = ?", "ℤ", &["0", "ℤ/2", "ℤ²"]),
        hard(
            "g20",
            "The degree of the antipodal map of S⁴ is?",
            "−1",
            &["1", "0", "4"],
        ),
        // Differential geometry, Lie theory and PDE.
        hard(
            "g21",
            "By Gauss–Bonnet, ∬ K dA over a closed orientable surface of genus 2 = ?",
            "−4π",
            &["0", "−2π", "4π"],
        ),
        hard("g22", "dim 𝔰𝔬(5) = ?", "10", &["5", "15", "25"]),
        hard(
            "g23",
            "Which Φ on ℝ³ ∖ {0} solves −ΔΦ = δ₀?",
            "1/(4π|x|)",
            &["−(1/2π) log|x|", "1/(2π|x|)", "1/(4π|x|²)"],
        ),
        // Group theory and representations.
        hard(
            "g24",
            "The number of Sylow 5-subgroups of the simple group A₅ is?",
            "6",
            &["1", "4", "12"],
        ),
        hard(
            "g25",
            "The number of abelian groups of order 72, up to isomorphism, is?",
            "6",
            &["4", "5", "9"],
        ),
        hard("g26", "|GL₂(𝔽₃)| = ?", "48", &["24", "72", "36"]),
        hard(
            "g27",
            "The number of irreducible complex representations of S₄ is?",
            "5",
            &["4", "24", "3"],
        ),
        hard(
            "g28",
            "The dimensions of the irreducible complex representations of the quaternion group Q₈ are?",
            "1, 1, 1, 1 and 2",
            &["1, 1, 2 and 2", "2 and 2", "1 eight times"],
        ),
        hard(
            "g29",
            "The number of units of ℤ/360ℤ is?",
            "96",
            &["72", "120", "144"],
        ),
        // Fields, rings and number theory.
        hard(
            "g30",
            "The Galois group of x³ − 2 over ℚ is?",
            "S₃",
            &["ℤ/3", "ℤ/6", "ℤ/2"],
        ),
        hard("g31", "[ℚ(√2, √3, √5) : ℚ] = ?", "8", &["6", "4", "3"]),
        hard(
            "g32",
            "The number of monic irreducible polynomials of degree 2 over 𝔽₅ is?",
            "10",
            &["5", "20", "25"],
        ),
        hard(
            "g33",
            "ℤ[i]/(3) is isomorphic to?",
            "the field 𝔽₉",
            &["𝔽₃ × 𝔽₃", "ℤ/9ℤ", "𝔽₃"],
        ),
        hard("g34", "The class number of ℚ(√−5) is?", "2", &["1", "3", "4"]),
        hard("g35", "The Legendre symbol (3/101) = ?", "−1", &["1", "0", "3"]),
        // Algebraic geometry and commutative algebra.
        hard(
            "g36",
            "The Krull dimension of ℂ[x, y, z]/(xy − z²) is?",
            "2",
            &["1", "3", "0"],
        ),
        hard(
            "g37",
            "The genus of a smooth plane curve of degree 4 is?",
            "3",
            &["1", "2", "6"],
        ),
        // Probability and combinatorics.
        hard(
            "g38",
            "A standard Brownian motion starts at 0. The expected time it first leaves (−1, 2) is?",
            "2",
            &["1", "3", "4"],
        ),
        hard(
            "g39",
            "The simple symmetric random walk on ℤ³ is?",
            "transient",
            &["recurrent", "null recurrent", "positive recurrent"],
        ),
        hard(
            "g40",
            "The number of labelled trees on 6 vertices is?",
            "1296",
            &["720", "7776", "216"],
        ),
        hard("g41", "The Ramsey number R(3, 4) = ?", "9", &["6", "10", "18"]),
    ]
}

/// v0.5.2's problems in draw order (R420, R767000).
pub static KY_TEST_BANK_V0_5_2: LazyLock<Vec<Problem>> = LazyLock::new(|| {
    vec![
        // Medium: double integrals.
        medium("m01", "∫₀¹ ∫₀¹ (x + y) dy dx = ?", "1", &["1/2", "2", "3/2"]),
        medium("m02", "∫₀¹ ∫₀¹ xy dy dx = ?", "1/4", &["1/2", "1", "1/8"]),
        medium("m03", "∫₀² ∫₀³ 1 dy dx = ?", "6", &["5", "3", "9"]),
        medium("m04", "∫₀¹ ∫₀² x dy dx = ?", "1", &["2", "1/2", "4"]),
        medium("m05", "∫₀¹ ∫₀¹ x²y dy dx = ?", "1/6", &["1/3", "1/2", "1/12"]),
        medium("m06", "∫₀¹ ∫₀ˣ 1 dy dx = ?", "1/2", &["1", "1/3", "1/4"]),
        medium("m07", "∫₀¹ ∫₀ˣ x dy dx = ?", "1/3", &["1/2", "1/6", "2/3"]),
        medium(
            "m08",
            "∫₀² ∫₀¹ (x² + y) dy dx = ?",
            "11/3",
            &["8/3", "7/3", "13/3"],
        ),
        medium(
            "m09",
            "∬ y sin x dA over [0, π] × [0, 1] = ?",
            "1",
            &["2", "π", "1/2"],
        ),
        medium(
            "m10",
            "∫₀¹ ∫₀¹ eˣ⁺ʸ dy dx = ?",
            "(e − 1)²",
            &["e² − 1", "e − 1", "2(e − 1)"],
        ),
        medium("m11", "∫₀¹ ∫₀² 6xy² dy dx = ?", "8", &["4", "16", "6"]),
        medium(
            "m12",
            "∬ 1 dA over the disc x² + y² ≤ 4 = ?",
            "4π",
            &["2π", "8π", "16π"],
        ),
        medium("m13", "∫₀^2π ∫₀¹ r dr dθ = ?", "π", &["2π", "π/2", "1"]),
        medium("m14", "∫₀^2π ∫₀² r² dr dθ = ?", "16π/3", &["8π/3", "4π", "8π"]),
        medium(
            "m15",
            "∬ (x² + y²) dA over the disc x² + y² ≤ 1 = ?",
            "π/2",
            &["π", "π/4", "2π"],
        ),
        medium(
            "m16",
            "∬ x dA over the triangle 0 ≤ y ≤ x ≤ 1 = ?",
            "1/3",
            &["1/2", "2/3", "1/6"],
        ),
        medium("m17", "∫₀² ∫₀ˣ 1 dy dx = ?", "2", &["4", "1", "3"]),
        medium("m18", "∫₀¹ ∫₀^x² x dy dx = ?", "1/4", &["1/3", "1/5", "1/2"]),
        medium(
            "m19",
            "∫₁² ∫₁² 1/(xy) dy dx = ?",
            "(ln 2)²",
            &["2 ln 2", "ln 2", "1/4"],
        ),
        medium("m20", "∫₀¹ ∫₀¹ (2x + 3y) dy dx = ?", "5/2", &["5", "3/2", "2"]),
        medium("m21", "∫₀³ ∫₀² (x + 1) dy dx = ?", "15", &["12", "9", "18"]),
        medium(
            "m22",
            "∬ sin x cos y dA over [0, π/2] × [0, π/2] = ?",
            "1",
            &["π/2", "0", "2"],
        ),
        medium(
            "m23",
            "∫₀¹ ∫₀¹ x eʸ dy dx = ?",
            "(e − 1)/2",
            &["e − 1", "e/2", "(e + 1)/2"],
        ),
        medium(
            "m24",
            "The volume under z = 4 − x² − y² above the disc x² + y² ≤ 4 = ?",
            "8π",
            &["16π", "4π", "32π/3"],
        ),
        medium("m25", "∫₀¹ ∫₀¹ (x − y)² dy dx = ?", "1/6", &["1/3", "0", "1/12"]),
        medium("m26", "∫₀² ∫₀^(2−x) x dy dx = ?", "4/3", &["2/3", "2", "8/3"]),
        medium(
            "m27",
            "∫₀¹ ∫₀¹ 1/(1 + x) dy dx = ?",
            "ln 2",
            &["1/2", "ln 3", "1"],
        ),
        medium(
            "m28",
            "∬ sin(x + y) dA over [0, π] × [0, π] = ?",
            "0",
            &["2", "4", "π"],
        ),
        medium("m29", "∫₀¹ ∫₀¹ x²y² dy dx = ?", "1/9", &["1/6", "1/3", "1/4"]),
        medium("m30", "∬ xy dA over [0, 2] × [0, 2] = ?", "4", &["2", "8", "16"]),
        medium("m31", "∫₀¹ ∫₀^√(1−x²) 1 dy dx = ?", "π/4", &["π/2", "1", "π"]),
        medium(
            "m32",
            "∬ eˣ⁺ʸ dA over [0, ln 2] × [0, ln 3] = ?",
            "2",
            &["6", "1", "5"],
        ),
        medium(
            "m33",
            "∫₀¹ ∫ₓ¹ e^(y²) dy dx = ? (swap the order)",
            "(e − 1)/2",
            &["e − 1", "e/2", "(e² − 1)/2"],
        ),
        // Hard: linear algebra.
        hard(
            "h01",
            "The eigenvalues of [[2, 1, 0], [1, 2, 0], [0, 0, 5]] are?",
            "1, 3 and 5",
            &["2, 2 and 5", "0, 4 and 5", "1, 3 and 0"],
        ),
        hard(
            "h02",
            "The rank of [[1, 2, 3, 4], [2, 4, 6, 8], [1, 0, 1, 0], [0, 2, 2, 4]] is?",
            "2",
            &["1", "3", "4"],
        ),
        hard(
            "h03",
            "det [[2, 1, 3], [1, 0, 2], [4, 1, 1]] = ?",
            "6",
            &["−6", "10", "2"],
        ),
        hard(
            "h04",
            "A is 4 × 4 with det A = 3. det(2A⁻¹) = ?",
            "16/3",
            &["2/3", "8/3", "48"],
        ),
        hard(
            "h05",
            "A is 3 × 3 with eigenvalues 1, 2 and 4. trace(A²) = ?",
            "21",
            &["49", "8", "64"],
        ),
        hard(
            "h06",
            "A 5 × 8 matrix has rank 3. The dimension of the null space of its transpose is?",
            "2",
            &["5", "3", "0"],
        ),
        hard(
            "h07",
            "The inverse of [[2, 1], [1, 1]] is?",
            "[[1, −1], [−1, 2]]",
            &["[[1, 1], [1, 2]]", "[[2, −1], [−1, 1]]", "[[1/2, 1], [1, 1]]"],
        ),
        hard(
            "h08",
            "A is n × n with A² = A and A ≠ I. det A = ?",
            "0",
            &["1", "−1", "2"],
        ),
        hard(
            "h09",
            "The eigenvalues of [[0, −2], [2, 0]] are?",
            "2i and −2i",
            &["2 and −2", "i and −i", "0, twice"],
        ),
        hard(
            "h10",
            "The vectors (1, 2, 3), (4, 5, 6) and (7, 8, 9) are?",
            "linearly dependent",
            &["linearly independent", "a basis of ℝ³", "pairwise orthogonal"],
        ),
        // Hard: Markov chains.
        hard(
            "h11",
            "A chain has P = [[1/2, 1/2, 0], [1/4, 1/2, 1/4], [0, 1/2, 1/2]]. Its stationary distribution is?",
            "(1/4, 1/2, 1/4)",
            &["(1/3, 1/3, 1/3)", "(1/2, 1/4, 1/4)", "(1/4, 1/4, 1/2)"],
        ),
        hard(
            "h12",
            "The chain with P = [[0, 1, 0], [0, 0, 1], [1, 0, 0]] has period?",
            "3",
            &["1", "2", "∞"],
        ),
        hard(
            "h13",
            "A chain has P = [[0.8, 0.2, 0], [0, 0.5, 0.5], [0.5, 0, 0.5]]. Its stationary distribution is?",
            "(5/9, 2/9, 2/9)",
            &["(1/3, 1/3, 1/3)", "(2/9, 5/9, 2/9)", "(1/2, 1/4, 1/4)"],
        ),
        hard(
            "h14",
            "A walk on {0, 1, 2, 3} steps up with probability 2/3 and down with 1/3, absorbed at 0 and 3, and starts at 1. P(it is absorbed at 3) = ?",
            "4/7",
            &["1/3", "2/3", "1/2"],
        ),
        hard(
            "h15",
            "A chain has P = [[1/2, 1/2, 0], [1/4, 1/2, 1/4], [0, 1/2, 1/2]]. The expected return time to state 1 is?",
            "4",
            &["2", "1/4", "3"],
        ),
        hard(
            "h16",
            "Gambler's ruin winning each bet with probability 0.6: start at 2, stop at 0 or 4. P(reach 4 first) = ?",
            "9/13",
            &["1/2", "3/5", "4/13"],
        ),
        hard(
            "h17",
            "A fair random walk on {0, …, 6}, absorbed at both ends, starts at 2. The expected steps to absorption are?",
            "8",
            &["4", "12", "6"],
        ),
        hard(
            "h18",
            "P = [[1/2, 1/2, 0, 0], [1/2, 1/2, 0, 0], [1/4, 1/4, 1/4, 1/4], [0, 0, 0, 1]]. How many communicating classes?",
            "3",
            &["2", "1", "4"],
        ),
        hard(
            "h19",
            "P = [[0, 1], [1/2, 1/2]] on states 1 and 2. P(X₃ = 2 | X₀ = 2) = ?",
            "5/8",
            &["3/4", "1/2", "3/8"],
        ),
        // Hard: statistics.
        hard(
            "h20",
            "Z ~ N(0, 1). P(−1 ≤ Z ≤ 2) ≈ ?",
            "0.82",
            &["0.68", "0.95", "0.48"],
        ),
        hard(
            "h21",
            "25 independent draws with variance 100. The standard deviation of their sum is?",
            "50",
            &["10", "2500", "2"],
        ),
        hard(
            "h22",
            "Independent draws from an Exponential with rate λ average 4. The maximum-likelihood estimate of λ is?",
            "1/4",
            &["4", "1/2", "1/16"],
        ),
        hard(
            "h23",
            "X ~ Exponential with rate λ = 2. P(X > 1) = ?",
            "e^(−2)",
            &["e^(−1/2)", "1/2", "1 − e^(−2)"],
        ),
        hard("h24", "X ~ Poisson(3). E[X²] = ?", "12", &["9", "3", "6"]),
        hard(
            "h25",
            "X₁, …, Xₙ are independent N(μ, σ²) with sample variance S². (n − 1)S²/σ² follows?",
            "χ² with n − 1 degrees of freedom",
            &[
                "χ² with n degrees of freedom",
                "t with n − 1 degrees of freedom",
                "N(0, 1)",
            ],
        ),
        hard(
            "h26",
            "Three fair dice. P(the sum is 10) = ?",
            "1/8",
            &["1/6", "5/36", "1/9"],
        ),
        hard(
            "h27",
            "X and Y are independent, Var(X) = 4 and Var(Y) = 9. Var(2X − 3Y) = ?",
            "97",
            &["−65", "13", "25"],
        ),
        hard(
            "h28",
            "2% have a disease; a test catches 90% of cases and falsely flags 5% of the healthy. P(ill | positive) = ?",
            "18/67",
            &["0.90", "0.02", "0.05"],
        ),
        hard(
            "h29",
            "σ = 10 known, n = 25, x̄ = 50. The 99% confidence interval for the mean is?",
            "(44.85, 55.15)",
            &["(46.08, 53.92)", "(47.42, 52.58)", "(30.4, 69.6)"],
        ),
        // Hard: PDEs.
        hard(
            "h30",
            "uₜ = k·uₓₓ on [0, π], u(0, t) = u(π, t) = 0, u(x, 0) = sin x + 3 sin 2x. u(x, t) = ?",
            "e^(−kt) sin x + 3e^(−4kt) sin 2x",
            &[
                "e^(−kt) sin x + 3e^(−2kt) sin 2x",
                "e^(−kt)(sin x + 3 sin 2x)",
                "e^(−kt) sin x + 3e^(−4t) sin 2x",
            ],
        ),
        hard(
            "h31",
            "The equation uₓₓ + 4uₓᵧ + uᵧᵧ = 0 is?",
            "hyperbolic",
            &["parabolic", "elliptic", "first order"],
        ),
        hard(
            "h32",
            "The equation uₓₓ + 2uₓᵧ + uᵧᵧ = 0 is?",
            "parabolic",
            &["hyperbolic", "elliptic", "first order"],
        ),
        hard(
            "h33",
            "uₜₜ = c²uₓₓ with u(x, 0) = 0 and uₜ(x, 0) = g(x). u(x, t) = ?",
            "(1/(2c)) ∫ g(s) ds from x − ct to x + ct",
            &[
                "(1/2) ∫ g(s) ds from x − ct to x + ct",
                "(g(x − ct) + g(x + ct))/2",
                "t·g(x)",
            ],
        ),
        hard(
            "h34",
            "uₜ = uₓₓ on [0, L] with uₓ = 0 at both ends. Separation of variables gives the eigenvalues?",
            "(nπ/L)², n = 0, 1, 2, …",
            &[
                "(nπ/L)², n = 1, 2, 3, …",
                "((n + 1/2)π/L)², n = 0, 1, 2, …",
                "nπ/L, n = 1, 2, 3, …",
            ],
        ),
        hard(
            "h35",
            "Which function is harmonic?",
            "eˣ cos y",
            &["eˣ cos x", "x² + y²", "eˣ y"],
        ),
        hard(
            "h36",
            "uₜ + 3uₓ = −u with u(x, 0) = g(x). u(x, t) = ?",
            "e^(−t) g(x − 3t)",
            &["e^(t) g(x − 3t)", "e^(−t) g(x + 3t)", "g(x − 3t) − t"],
        ),
        // Hard: proofs.
        hard(
            "h37",
            "To prove √3 irrational, suppose √3 = p/q in lowest terms, so p² = 3q². The contradiction is that?",
            "3 divides both p and q",
            &["p = 3q", "q = 1", "p and q are both even"],
        ),
        hard(
            "h38",
            "Inducting 1² + 2² + ⋯ + n² = n(n + 1)(2n + 1)/6, the step shows n(n + 1)(2n + 1)/6 + (n + 1)² equals?",
            "(n + 1)(n + 2)(2n + 3)/6",
            &[
                "(n + 1)(n + 2)(2n + 1)/6",
                "n(n + 1)(2n + 3)/6",
                "(n + 1)²(2n + 1)/6",
            ],
        ),
        hard(
            "h39",
            "Euclid's argument for infinitely many primes of the form 4k + 3: given such primes p₁, …, pₖ, which number has a prime factor of that form outside the list?",
            "4p₁p₂⋯pₖ − 1",
            &["p₁p₂⋯pₖ + 1", "4p₁p₂⋯pₖ + 1", "4(p₁ + ⋯ + pₖ) + 3"],
        ),
        hard(
            "h40",
            "By the pigeonhole principle, the fewest people certain to include three born in the same month is?",
            "25",
            &["24", "36", "13"],
        ),
        hard(
            "h41",
            "Cantor's theorem shows that for every set S?",
            "no function from S onto its power set exists",
            &[
                "S is countable",
                "S and its power set are the same size",
                "every subset of S is finite",
            ],
        ),
    ]
});
