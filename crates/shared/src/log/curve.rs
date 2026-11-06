//! C+ #62 KY's Papaya's curve as an equation (SPEC §8.7 row 62, R831003): the log names the curve a
//! `curveDrawn` draws. Presentation only (CLAUDE.md rule 7): it reads the event's public cells and decides
//! nothing.
//!
//! The grid is the engine's (`subsystems/papaya.rs`, R422), from the caster's seat, x the lane less one. The
//! event names every cell the curve meets, so the lowest-degree polynomial through the named cells is the
//! curve itself, worked out by Lagrange interpolation in exact fractions, never floating point. It prints
//! with the minus sign (U+2212) for a negative.

use jackioh_engine::config::UNIT_ZONES;
use jackioh_engine::subsystems::papaya::{PapayaPoint, point_of_zone};
use jackioh_engine::wire::{PlayerId, ZoneRef};

const SUPERSCRIPT_DIGITS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
const SUBSCRIPT_DIGITS: [char; 10] = ['₀', '₁', '₂', '₃', '₄', '₅', '₆', '₇', '₈', '₉'];
const MINUS: char = '−';
const FRACTION_SLASH: char = '⁄';

/// A reduced fraction: an integer over a positive integer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Fraction {
    num: i64,
    den: i64,
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 { a.abs() } else { gcd(b, a % b) }
}

fn fraction(num: i64, den: i64) -> Fraction {
    if num == 0 {
        return Fraction { num: 0, den: 1 };
    }
    let divisor = gcd(num, den) * den.signum();
    Fraction {
        num: num / divisor,
        den: den / divisor,
    }
}

fn sum(a: Fraction, b: Fraction) -> Fraction {
    fraction(a.num * b.den + b.num * a.den, a.den * b.den)
}

/// The polynomial through the points (Lagrange), its coefficients by power, the constant first.
fn coefficients(points: &[PapayaPoint]) -> Vec<Fraction> {
    let mut out = vec![fraction(0, 1); points.len()];
    for (i, pi) in points.iter().enumerate() {
        // The basis polynomial's numerator, the product of (x − xj), and its denominator, of (xi − xj).
        let mut basis: Vec<i64> = vec![1];
        let mut den: i64 = 1;
        for (j, pj) in points.iter().enumerate() {
            if j == i {
                continue;
            }
            let mut next = vec![0_i64; basis.len() + 1];
            for (k, c) in basis.iter().enumerate() {
                next[k + 1] += c;
                next[k] -= i64::from(pj.x) * c;
            }
            basis = next;
            den *= i64::from(pi.x - pj.x);
        }
        for (k, c) in basis.iter().enumerate() {
            out[k] = sum(out[k], fraction(i64::from(pi.y) * c, den));
        }
    }
    out
}

fn digits(n: i64, set: &[char; 10]) -> String {
    n.to_string()
        .chars()
        .map(|digit| digit.to_digit(10).map_or(digit, |at| set[at as usize]))
        .collect()
}

fn magnitude(f: Fraction) -> String {
    let num = f.num.abs();
    if f.den == 1 {
        num.to_string()
    } else {
        format!(
            "{}{FRACTION_SLASH}{}",
            digits(num, &SUPERSCRIPT_DIGITS),
            digits(f.den, &SUBSCRIPT_DIGITS)
        )
    }
}

/// One term without its sign.
fn term(f: Fraction, power: usize) -> String {
    if power == 0 {
        return magnitude(f);
    }
    let coefficient = if f.num.abs() == 1 && f.den == 1 {
        String::new()
    } else {
        magnitude(f)
    };
    let exponent = if power == 1 {
        String::new()
    } else {
        digits(power as i64, &SUPERSCRIPT_DIGITS)
    };
    format!("{coefficient}x{exponent}")
}

fn signed(f: Fraction, first: bool) -> String {
    match (first, f.num < 0) {
        (true, true) => MINUS.to_string(),
        (true, false) => String::new(),
        (false, true) => format!(" {MINUS} "),
        (false, false) => " + ".to_string(),
    }
}

/// The equation of the curve a `curveDrawn` names (R831003), read from `caster`'s seat; None when the cells
/// make no curve (none, too many, a lane off the board, or two in one lane).
pub fn curve_equation(cells: &[ZoneRef], caster: PlayerId) -> Option<String> {
    if cells.is_empty() || cells.len() > UNIT_ZONES as usize {
        return None;
    }
    if cells.iter().any(|cell| !(1..=UNIT_ZONES).contains(&cell.lane)) {
        return None;
    }
    let points: Vec<PapayaPoint> = cells.iter().map(|cell| point_of_zone(caster, cell)).collect();
    if points.iter().any(|point| point.y < 0) {
        return None;
    }
    for (at, point) in points.iter().enumerate() {
        if points[..at].iter().any(|other| other.x == point.x) {
            return None;
        }
    }
    let terms: Vec<(Fraction, usize)> = coefficients(&points)
        .into_iter()
        .enumerate()
        .filter(|(_, f)| f.num != 0)
        .map(|(power, f)| (f, power))
        .rev()
        .collect();
    if terms.is_empty() {
        return Some("y = 0".to_string());
    }
    let body: String = terms
        .iter()
        .enumerate()
        .map(|(at, (f, power))| format!("{}{}", signed(*f, at == 0), term(*f, *power)))
        .collect();
    Some(format!("y = {body}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::wire::{
        PlayerId::{P1, P2},
        Row,
    };

    fn cell(player: PlayerId, row: Row, lane: i32) -> ZoneRef {
        ZoneRef { player, row, lane }
    }

    #[test]
    fn a_constant_crosses_its_whole_row_and_reads_zero_on_the_casters_backrow() {
        let row = |player, row| (1..=5).map(|lane| cell(player, row, lane)).collect::<Vec<_>>();
        assert_eq!(curve_equation(&row(P1, Row::Units), P1).as_deref(), Some("y = 1"));
        assert_eq!(
            curve_equation(&row(P1, Row::Backrow), P1).as_deref(),
            Some("y = 0")
        );
        assert_eq!(
            curve_equation(&[cell(P2, Row::Units, 3)], P1).as_deref(),
            Some("y = 2")
        );
    }

    #[test]
    fn a_line_with_a_negative_slope_leaves_out_a_coefficient_of_one() {
        let cells = [
            cell(P2, Row::Backrow, 1),
            cell(P2, Row::Units, 2),
            cell(P1, Row::Units, 3),
            cell(P1, Row::Backrow, 4),
        ];
        assert_eq!(curve_equation(&cells, P1).as_deref(), Some("y = −x + 3"));
    }

    #[test]
    fn fractional_coefficients_print_in_superscript_and_subscript_digits() {
        let line = [
            cell(P1, Row::Backrow, 1),
            cell(P1, Row::Units, 3),
            cell(P2, Row::Units, 5),
        ];
        assert_eq!(curve_equation(&line, P1).as_deref(), Some("y = ¹⁄₂x"));
        let shifted = [cell(P1, Row::Backrow, 2), cell(P1, Row::Units, 4)];
        assert_eq!(curve_equation(&shifted, P1).as_deref(), Some("y = ¹⁄₂x − ¹⁄₂"));
        let parabola = [
            cell(P1, Row::Backrow, 1),
            cell(P2, Row::Units, 2),
            cell(P2, Row::Backrow, 3),
            cell(P2, Row::Backrow, 4),
            cell(P2, Row::Units, 5),
        ];
        assert_eq!(
            curve_equation(&parabola, P1).as_deref(),
            Some("y = −¹⁄₂x² + ⁵⁄₂x")
        );
        let cubic = [
            cell(P1, Row::Units, 1),
            cell(P1, Row::Backrow, 2),
            cell(P2, Row::Backrow, 4),
            cell(P1, Row::Backrow, 5),
        ];
        assert_eq!(
            curve_equation(&cubic, P1).as_deref(),
            Some("y = −⁷⁄₁₂x³ + ¹⁹⁄₆x² − ⁴³⁄₁₂x + 1")
        );
    }

    #[test]
    fn the_grid_is_read_from_the_casters_side_whichever_seat_cast_it() {
        let cells = [
            cell(P2, Row::Backrow, 1),
            cell(P1, Row::Units, 2),
            cell(P1, Row::Backrow, 3),
            cell(P1, Row::Backrow, 4),
            cell(P1, Row::Units, 5),
        ];
        assert_eq!(curve_equation(&cells, P2).as_deref(), Some("y = −¹⁄₂x² + ⁵⁄₂x"));
        let other = [
            cell(P1, Row::Backrow, 1),
            cell(P2, Row::Units, 2),
            cell(P2, Row::Backrow, 3),
            cell(P2, Row::Backrow, 4),
            cell(P2, Row::Units, 5),
        ];
        assert_eq!(
            curve_equation(&other, P2).as_deref(),
            Some("y = ¹⁄₂x² − ⁵⁄₂x + 3")
        );
    }

    #[test]
    fn cells_that_make_no_curve_make_no_equation() {
        assert_eq!(curve_equation(&[], P1), None);
        for lane in [0, 6, 999, -1] {
            assert_eq!(curve_equation(&[cell(P1, Row::Units, lane)], P1), None);
        }
        // Two cells in one lane.
        assert_eq!(
            curve_equation(&[cell(P1, Row::Units, 2), cell(P2, Row::Units, 2)], P1),
            None
        );
        // Too many cells for the lanes there are.
        let crowded: Vec<ZoneRef> = (0..120).map(|_| cell(P1, Row::Units, 1)).collect();
        assert_eq!(curve_equation(&crowded, P1), None);
    }
}
