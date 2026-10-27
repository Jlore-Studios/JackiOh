//! Stinkyscape #4.2 KY's Last Exam's bank of impossible problems (SPEC §8.10 row S 4.2, R420, R465,
//! R1870002). Card data like `ky_test_bank.rs`: public, shipped with the catalog, so changing a problem
//! is a patch.
//!
//! "Solve an impossible problem": each statement leaves its unknowns open, so none of its four options
//! follows from it and nothing in the words gives an answer away. No option is the answer here: the
//! right one is drawn from the match rng when the problem is asked, then the options are shown under
//! letters in an rng order (`choose_answer`), the key kept in the engine (R465). Every answerer, the AI
//! included, guesses one in four. Plain text with Unicode maths, as KY's Test's bank is.

/// How many options each problem offers.
pub const KY_LAST_EXAM_OPTIONS: usize = 4;

/// One impossible problem: its id, its statement and its four different options.
pub struct ImpossibleProblem {
    pub id: &'static str,
    pub statement: &'static str,
    pub options: [&'static str; KY_LAST_EXAM_OPTIONS],
}

/// The bank, in the order a draw reads it.
pub const KY_LAST_EXAM_BANK: &[ImpossibleProblem] = &[
    ImpossibleProblem {
        id: "x1",
        statement: "x = ?",
        options: ["1", "2", "3", "4"],
    },
    ImpossibleProblem {
        id: "x2",
        statement: "x + y = ?",
        options: ["0", "1", "2", "3"],
    },
    ImpossibleProblem {
        id: "x3",
        statement: "f(1) = ?",
        options: ["0", "1", "2", "−1"],
    },
    ImpossibleProblem {
        id: "x4",
        statement: "n mod 4 = ?",
        options: ["0", "1", "2", "3"],
    },
    ImpossibleProblem {
        id: "x5",
        statement: "∫₀¹ f(x) dx = ?",
        options: ["0", "1/2", "1", "2"],
    },
    ImpossibleProblem {
        id: "x6",
        statement: "BB(6) mod 4 = ?",
        options: ["0", "1", "2", "3"],
    },
    ImpossibleProblem {
        id: "x7",
        statement: "a₁₀₀ = ?",
        options: ["0", "1", "99", "100"],
    },
    ImpossibleProblem {
        id: "x8",
        statement: "lim (n→∞) aₙ = ?",
        options: ["0", "1", "−1", "∞"],
    },
];
