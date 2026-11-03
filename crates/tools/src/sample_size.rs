//! `cargo jackioh stats sample-size [--sigma S] [--rho R] [--explain]` (#2888): the games a
//! gather run needs so that, with 95% confidence across all K deckable cards, no card measured 3 SD
//! from the mean is really within 1 SD of it. The formula is N = ⌊z_K²/(16σ²) ÷ 2f(1−f)⌋ rounded
//! down to a multiple of `GAMES_STEP`, with z_K = Φ⁻¹(1 − ALPHA/K) and f = ρ·DECK_SIZE/K. The result
//! goes to `gather.yml`'s `GAMES`, which a person wires up (the bots never edit `.github/`).
//!
//! Tooling: the formula is pure arithmetic; only `deal_pool` reads the registered catalog.

use anyhow::Result;

use jackioh_ai::{AI_DECK_GATED_SETS, SHADOW_BAN_IDS};
use jackioh_engine::config::{DECK_SIZE, MAX_COPIES};
use jackioh_engine::{CatalogQueryArgs, query};

/// The chance, across the whole catalog at once, that some card's noise exceeds the gap. It is
/// one-sided: only noise that pushes a card's rate further out on its own side makes a false flag.
/// It is split over the K cards (Bonferroni).
pub const ALPHA: f64 = 0.05;
/// A card flagged at 3 SD that is really at 1 SD carries 3 − 1 = 2 SD of noise.
pub const SD_GAP: f64 = 2.0;
/// The largest SD of one game's win, a fair coin's √(0.5·0.5). A rate over n games has noise of at
/// most this over √n.
pub const MAX_GAME_SD: f64 = 0.5;
/// σ, the true spread of the card win rates as a share (0.02 is 2 points). It is a guess until a
/// big run measures it as var(observed rates) − mean(p(1−p)/nᵢ). Halving σ makes N four times bigger.
pub const DEFAULT_SIGMA: f64 = 0.02;
/// ρ, the least-held card's deck share over the mean share. 1 means every card is dealt equally
/// often. R1370's leaned-set floor may make some cards rarer.
pub const DEFAULT_RHO: f64 = 1.0;
/// N rounds down to a multiple of this (the owner's call, #2888).
pub const GAMES_STEP: u64 = 1000;
/// The `--explain` print precision of z_K.
const Z_DECIMALS: usize = 6;
/// The `--explain` print precision of the counted share f.
const SHARE_DECIMALS: usize = 6;
/// The `--explain` print precision of the games per card.
const GAMES_DECIMALS: usize = 1;

/// Acklam's rational approximation to Φ⁻¹, relative error under 1.15e-9.
const ACKLAM_A: [f64; 6] = [
    -39.69683028665376,
    220.9460984245205,
    -275.9285104469687,
    138.357751867269,
    -30.66479806614716,
    2.506628277459239,
];
const ACKLAM_B: [f64; 5] = [
    -54.47609879822406,
    161.5858368580409,
    -155.6989798598866,
    66.80131188771972,
    -13.28068155288572,
];
const ACKLAM_C: [f64; 6] = [
    -0.007784894002430293,
    -0.3223964580411365,
    -2.400758277161838,
    -2.549732539343734,
    4.374664141464968,
    2.938163982698783,
];
const ACKLAM_D: [f64; 4] = [
    0.007784695709041462,
    0.3224671290700398,
    2.445134137142996,
    3.754408661907416,
];
/// Below this p, and above 1 − it, Acklam's tail form applies.
const ACKLAM_TAIL: f64 = 0.02425;

fn acklam_tail(q: f64) -> f64 {
    let [c0, c1, c2, c3, c4, c5] = ACKLAM_C;
    let [d0, d1, d2, d3] = ACKLAM_D;
    (((((c0 * q + c1) * q + c2) * q + c3) * q + c4) * q + c5)
        / ((((d0 * q + d1) * q + d2) * q + d3) * q + 1.0)
}

/// Φ⁻¹(p), the standard normal quantile, for 0 < p < 1 (Acklam).
pub fn normal_quantile(p: f64) -> f64 {
    if p < ACKLAM_TAIL {
        acklam_tail((-2.0 * p.ln()).sqrt())
    } else if p > 1.0 - ACKLAM_TAIL {
        -acklam_tail((-2.0 * (1.0 - p).ln()).sqrt())
    } else {
        let [a0, a1, a2, a3, a4, a5] = ACKLAM_A;
        let [b0, b1, b2, b3, b4] = ACKLAM_B;
        let q = p - 0.5;
        let r = q * q;
        (((((a0 * r + a1) * r + a2) * r + a3) * r + a4) * r + a5) * q
            / (((((b0 * r + b1) * r + b2) * r + b3) * r + b4) * r + 1.0)
    }
}

/// K: the cards the All Random deal can pick, the pool `build_ai_deck` draws from under
/// `AiDeckOptions::default()`: `query`'s default pool (§2.6's deckable cards) less the shadow ban
/// (R186) and the soft gate's sets (R1390). Call `jackioh_cards::register_all()` first.
pub fn deal_pool() -> usize {
    query(&CatalogQueryArgs::default())
        .iter()
        .filter(|def| !SHADOW_BAN_IDS.contains(&def.id.as_str()) && !AI_DECK_GATED_SETS.contains(&def.set))
        .count()
}

/// The sample size at `cards` deckable cards: the Bonferroni z, the games one card needs, its
/// counted share of all games, and the total games N.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SampleSize {
    pub cards: usize,
    pub z: f64,
    pub games_per_card: f64,
    pub share: f64,
    pub games: u64,
}

pub fn sample_size(cards: usize, sigma: f64, rho: f64) -> SampleSize {
    let k = cards as f64;
    let z = normal_quantile(1.0 - ALPHA / k);
    let games_per_card = (z * MAX_GAME_SD / (SD_GAP * sigma)).powi(2);
    // The fewest distinct cards a deck holds, which gives the smallest share and so the largest N.
    let distinct = f64::from(DECK_SIZE) / f64::from(MAX_COPIES);
    let share = rho * distinct / k;
    // `holder_rate` counts a game when exactly one seat holds the card, p1 and not p2 or the
    // reverse (R1284000).
    let counted = share * (1.0 - share) + (1.0 - share) * share;
    let games = (games_per_card / counted / GAMES_STEP as f64).floor() as u64 * GAMES_STEP;
    SampleSize {
        cards,
        z,
        games_per_card,
        share,
        games,
    }
}

/// The command's stdout: N alone, or with `--explain` K, z_K, the games per card, f and N.
pub fn render(sized: &SampleSize, explain: bool) -> String {
    if !explain {
        return sized.games.to_string();
    }
    [
        format!("K: {}", sized.cards),
        format!("z_K: {:.*}", Z_DECIMALS, sized.z),
        format!("games per card: {:.*}", GAMES_DECIMALS, sized.games_per_card),
        format!("f: {:.*}", SHARE_DECIMALS, sized.share),
        format!("N: {}", sized.games),
    ]
    .join("\n")
}

fn sigma(raw: &str) -> Result<f64, String> {
    match raw.parse::<f64>() {
        Ok(value) if value.is_finite() && value > 0.0 => Ok(value),
        _ => Err(format!("--sigma must be a positive number (got {raw:?}).")),
    }
}

fn rho(raw: &str) -> Result<f64, String> {
    match raw.parse::<f64>() {
        Ok(value) if value > 0.0 && value <= 1.0 => Ok(value),
        _ => Err(format!("--rho must be above 0 and at most 1 (got {raw:?}).")),
    }
}

/// `cargo jackioh stats sample-size [--sigma S] [--rho R] [--explain]`.
#[derive(clap::Args, Debug, Clone, PartialEq)]
pub struct Args {
    /// σ, the true spread of the card win rates as a share (0.02 is 2 points).
    #[arg(long, value_parser = sigma, default_value_t = DEFAULT_SIGMA)]
    pub sigma: f64,
    /// ρ, the least-held card's deck share over the mean share, above 0 and at most 1.
    #[arg(long, value_parser = rho, default_value_t = DEFAULT_RHO)]
    pub rho: f64,
    /// Print K, z_K, the games per card, f and N, one per line, instead of N alone.
    #[arg(long)]
    pub explain: bool,
}

/// The command's stdout over this build's catalog.
pub fn output(args: &Args) -> String {
    jackioh_cards::register_all();
    render(&sample_size(deal_pool(), args.sigma, args.rho), args.explain)
}

pub fn run(args: &Args) -> Result<()> {
    println!("{}", output(args));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    /// `Args` under a parser of its own, as `stats.rs` mounts it.
    #[derive(clap::Parser, Debug)]
    struct Cli {
        #[command(flatten)]
        args: Args,
    }

    fn parse(argv: &[&str]) -> Result<Args, clap::Error> {
        Cli::try_parse_from(std::iter::once("sample-size").chain(argv.iter().copied())).map(|cli| cli.args)
    }

    #[test]
    fn normal_quantile_matches_the_standard_normal_quantiles_to_1e_6() {
        for (p, x) in [
            (0.975, 1.959963984540054),
            (1.0 - 0.05 / 634.0, 3.7785744370236),
            (1.0 - 0.05 / 2000.0, 4.055626981122),
            (0.025, -1.959963984540054),
            (0.5, 0.0),
        ] {
            assert!(
                (normal_quantile(p) - x).abs() < 1e-6,
                "Φ⁻¹({p}) = {}, want {x}",
                normal_quantile(p)
            );
        }
    }

    #[test]
    fn sample_size_reproduces_every_cell_of_the_issues_table() {
        for (cards, z, at_0_02, at_0_03) in [
            (100, 3.29, 4_000, 2_000),
            (250, 3.54, 10_000, 4_000),
            (500, 3.72, 22_000, 10_000),
            (634, 3.78, 29_000, 13_000),
            (1_000, 3.89, 48_000, 21_000),
            (2_000, 4.06, 104_000, 46_000),
        ] {
            assert!(
                (sample_size(cards, 0.02, 1.0).z - z).abs() < 0.005,
                "K = {cards}: z = {}",
                sample_size(cards, 0.02, 1.0).z
            );
            assert_eq!(sample_size(cards, 0.02, 1.0).games, at_0_02, "K = {cards}");
            assert_eq!(sample_size(cards, 0.03, 1.0).games, at_0_03, "K = {cards}");
        }
    }

    #[test]
    fn half_the_rho_doubles_the_games_near_enough() {
        assert_eq!(sample_size(634, 0.02, 0.5).games, 57_000);
    }

    #[test]
    fn explain_prints_k_z_the_games_per_card_f_and_n() {
        assert_eq!(
            render(&sample_size(634, 0.02, 1.0), true),
            "K: 634\nz_K: 3.778574\ngames per card: 2230.9\nf: 0.039432\nN: 29000"
        );
        assert_eq!(render(&sample_size(634, 0.02, 1.0), false), "29000");
    }

    #[test]
    fn reads_sigma_rho_and_explain_and_refuses_values_out_of_range() {
        assert_eq!(
            parse(&[]).unwrap(),
            Args {
                sigma: DEFAULT_SIGMA,
                rho: DEFAULT_RHO,
                explain: false
            }
        );
        assert_eq!(
            parse(&["--sigma", "0.03", "--rho", "0.5", "--explain"]).unwrap(),
            Args {
                sigma: 0.03,
                rho: 0.5,
                explain: true
            }
        );
        for argv in [
            vec!["--sigma", "0"],
            vec!["--sigma", "-0.02"],
            vec!["--sigma", "x"],
            vec!["--sigma", "inf"],
            vec!["--rho", "0"],
            vec!["--rho", "1.5"],
            vec!["--rho", "NaN"],
        ] {
            assert!(parse(&argv).is_err(), "{argv:?}");
        }
    }

    #[test]
    fn sample_size_on_the_builds_catalog_is_the_formula_at_the_pool_build_ai_deck_draws_from() {
        jackioh_cards::register_all();
        let cards = deal_pool();
        let oversized = std::panic::catch_unwind(|| {
            jackioh_ai::build_ai_deck(
                &mut jackioh_engine::Rng::new("sample-size", 0),
                i32::try_from(cards).unwrap() + 1,
                &jackioh_ai::AiDeckOptions::default(),
            )
        })
        .expect_err("a deck one card past the pool must panic");
        assert!(
            oversized.downcast_ref::<String>().is_some_and(
                |message| message.contains(&format!("but only {cards} non-token cards are unbanned"))
            ),
            "build_ai_deck's pool is not this command's K"
        );
        assert_eq!(
            output(&parse(&[]).unwrap()),
            sample_size(cards, DEFAULT_SIGMA, DEFAULT_RHO).games.to_string()
        );
    }
}
