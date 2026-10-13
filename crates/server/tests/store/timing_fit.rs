//! `cli/timing_fit.rs` (timing-fit) and the argument check `cli/timing_backfill.rs` shares with it,
//! SPEC §9.11, R1442: the human think times of each bucket fitted with a log-normal, the AI's moves
//! left out, a bucket under the minimum dropped, and the rows printed to paste into config. The
//! store's read is `tests/store/contract.rs`'s; the backfill's fold is `tests/actor/telemetry.rs`'s.

use serde_json::{Value, json};

use jackioh_engine::wire::{EmoteId, GameOverReason, Pilot};
use jackioh_engine::{ActionType, PlayerId};
use jackioh_server::cli::timing_fit::{
    BucketFit, SignalFits, fit_complexity_slope, fit_concede, fit_draw_answer, fit_emote_delay,
    fit_lethal_shift, fit_persona_weights, fit_rematch, fit_signals, fit_think_times, fit_timeouts,
    render_constants, render_fits,
};
use jackioh_server::cli::{timing_backfill, timing_fit};
use jackioh_server::config::{
    ConcedeFit, DrawAnswerFit, EmoteDelayFit, PersonaWeights, ScalarFit, TimeoutFit,
};
use jackioh_server::db::store::{ActionTimingRow, EmoteEventRow, MatchSignalRow, SignalOutcomeRow};
use jackioh_server::ranked::ladder::RankTier;

/// How close two fitted numbers must be: well under the printed four decimals.
const EPSILON: f64 = 1e-9;

fn timing(seq: i64, kind: &str, first: bool, rank: Value, think_ms: i64, pilot: &str) -> ActionTimingRow {
    serde_json::from_value(json!({
        "matchId": "m-fit",
        "seat": "p1",
        "seq": seq,
        "actionKind": kind,
        "legalCount": 3,
        "turn": 2,
        "thinkMs": think_ms,
        "firstInTurn": first,
        "rankBucket": rank,
        "pilot": pilot,
    }))
    .expect("an action timing")
}

/// Three human plays in Golden Grape (100 ms, 1 s, 10 s: a geometric mean of 1 s), two human turn
/// ends with no rank (0 ms, which counts as 1 ms, and 1 ms), and AI moves in the first bucket and in
/// a bucket of their own, which would move the first fit and add a bucket if they were counted.
fn rows() -> Vec<ActionTimingRow> {
    vec![
        timing(1, "play", true, json!("golden"), 100, "human"),
        timing(2, "play", true, json!("golden"), 1_000, "human"),
        timing(3, "play", true, json!("golden"), 10_000, "human"),
        timing(4, "play", true, json!("golden"), 900_000, "ai"),
        timing(5, "endTurn", false, Value::Null, 0, "human"),
        timing(6, "endTurn", false, Value::Null, 1, "human"),
        timing(7, "attack", false, json!("mythic"), 5_000, "ai"),
        timing(8, "attack", false, json!("mythic"), 6_000, "ai"),
        timing(9, "attack", false, json!("mythic"), 7_000, "ai"),
    ]
}

#[track_caller]
fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < EPSILON, "{actual} is not {expected}");
}

#[test]
fn r1442_timing_fit_fits_each_bucket_and_leaves_out_ai_rows() {
    let fits = fit_think_times(&rows(), 2);
    let keys: Vec<(Option<RankTier>, ActionType, bool, usize)> = fits
        .iter()
        .map(|fit| (fit.rank_bucket, fit.action_kind, fit.first_in_turn, fit.samples))
        .collect();
    // No rank sorts before a tier, and the AI-only Mythic bucket is not there at all.
    assert_eq!(
        keys,
        vec![
            (None, ActionType::EndTurn, false, 2),
            (Some(RankTier::Golden), ActionType::Play, true, 3),
        ]
    );
    // ln(1) twice.
    close(fits[0].mu, 0.0);
    close(fits[0].sigma, 0.0);
    // ln(100), ln(1000), ln(10000): the mean is ln(1000), each end ln(10) from it.
    close(fits[1].mu, 1_000f64.ln());
    close(fits[1].sigma, 10f64.ln() * (2.0f64 / 3.0).sqrt());
    // R639000: every move had three legal actions.
    close(fits[0].ln_legal_mean, 3f64.ln());
    close(fits[1].ln_legal_mean, 3f64.ln());
}

#[test]
fn r1442_timing_fit_drops_a_bucket_under_the_minimum() {
    let fits = fit_think_times(&rows(), 3);
    assert_eq!(fits.len(), 1);
    assert_eq!(fits[0].rank_bucket, Some(RankTier::Golden));
    assert_eq!(fits[0].samples, 3);
    assert!(fit_think_times(&rows(), 4).is_empty());
}

#[test]
fn r1442_timing_fit_prints_rows_to_paste_into_config() {
    let fits = [
        BucketFit {
            rank_bucket: None,
            action_kind: ActionType::EndTurn,
            first_in_turn: false,
            samples: 40,
            mu: 7.5,
            sigma: 0.25,
            ln_legal_mean: 1.0,
        },
        BucketFit {
            rank_bucket: Some(RankTier::Golden),
            action_kind: ActionType::Play,
            first_in_turn: true,
            samples: 31,
            mu: 8.123_41,
            sigma: 0.734_49,
            ln_legal_mean: 2.123_46,
        },
    ];
    assert_eq!(
        render_fits(&fits),
        "// jackioh-server timing-fit: paste into THINK_TIME_FITS, crates/server/src/config.rs\n\
         \x20   ThinkTimeFit { rank_bucket: None, action_kind: \"endTurn\", first_in_turn: false, samples: 40, mu: 7.5000, sigma: 0.2500, ln_legal_mean: 1.0000 },\n\
         \x20   ThinkTimeFit { rank_bucket: Some(\"golden\"), action_kind: \"play\", first_in_turn: true, samples: 31, mu: 8.1234, sigma: 0.7345, ln_legal_mean: 2.1235 },\n"
    );
    assert_eq!(
        render_fits(&[]),
        "// jackioh-server timing-fit: paste into THINK_TIME_FITS, crates/server/src/config.rs\n"
    );
}

#[tokio::test]
async fn timing_commands_refuse_an_unknown_option() {
    for refusal in [
        timing_fit::run(vec!["--min=3".to_string()]).await,
        timing_backfill::run(vec!["--all".to_string()]).await,
    ] {
        let message = refusal.expect_err("an option neither command takes").to_string();
        assert!(message.contains("Unrecognised argument \"--"), "{message}");
        assert!(message.contains("Usage: jackioh-server timing-"), "{message}");
    }
}

// ---------------------------------------------------------------------------------------------
// R639000: the rest of a person's habits
// ---------------------------------------------------------------------------------------------

/// One seat's signals beside its match's outcome, with nothing signalled.
fn outcome(
    match_id: &str,
    seat: PlayerId,
    pilot: Pilot,
    won: Option<bool>,
    reason: GameOverReason,
    turns: i64,
    rematchable: bool,
) -> SignalOutcomeRow {
    SignalOutcomeRow {
        signal: MatchSignalRow {
            match_id: match_id.to_string(),
            seat,
            conceded_turn: None,
            concede_eval_deficit: None,
            draw_offers: 0,
            draw_accepted: false,
            rematch_offered: false,
            rematch_accepted: false,
            timeouts: 0,
            pilot,
        },
        won,
        reason,
        turns,
        rematchable,
    }
}

fn with(mut row: SignalOutcomeRow, change: impl FnOnce(&mut MatchSignalRow)) -> SignalOutcomeRow {
    change(&mut row.signal);
    row
}

/// Five matches. m1: p1 concedes on turn 8, 30 behind, after a timeout, and meets p2's rematch
/// offer. m2: p2 concedes on turn 12, 50 behind, having declined p1's draw offer. m3: p2 accepts
/// p1's draw offer, in a match no rematch could follow (p1's offer there counts for nothing). m4: p1
/// wins by a hero's death and offers a rematch p2 never meets; p2 timed out twice. m5: p1 against
/// the AI, whose concession on turn 3, 99 behind, is the AI's and left out.
fn outcomes() -> Vec<SignalOutcomeRow> {
    use GameOverReason::*;
    use PlayerId::*;
    let person = Pilot::Human;
    vec![
        with(outcome("m1", P1, person, Some(false), Concede, 8, true), |s| {
            s.conceded_turn = Some(8);
            s.concede_eval_deficit = Some(30.0);
            s.timeouts = 1;
            s.rematch_offered = true;
            s.rematch_accepted = true;
        }),
        with(outcome("m1", P2, person, Some(true), Concede, 8, true), |s| {
            s.rematch_offered = true;
        }),
        with(outcome("m2", P1, person, Some(true), Concede, 12, true), |s| {
            s.draw_offers = 1;
        }),
        with(outcome("m2", P2, person, Some(false), Concede, 12, true), |s| {
            s.conceded_turn = Some(12);
            s.concede_eval_deficit = Some(50.0);
        }),
        with(outcome("m3", P1, person, None, DrawAccepted, 10, false), |s| {
            s.draw_offers = 1;
            s.rematch_offered = true;
        }),
        with(outcome("m3", P2, person, None, DrawAccepted, 10, false), |s| {
            s.draw_accepted = true;
        }),
        with(outcome("m4", P1, person, Some(true), HeroDeath, 9, true), |s| {
            s.rematch_offered = true;
        }),
        with(outcome("m4", P2, person, Some(false), HeroDeath, 9, true), |s| {
            s.timeouts = 2;
        }),
        outcome("m5", P1, person, Some(true), Concede, 3, true),
        with(outcome("m5", P2, Pilot::Ai, Some(false), Concede, 3, true), |s| {
            s.conceded_turn = Some(3);
            s.concede_eval_deficit = Some(99.0);
            s.timeouts = 5;
        }),
    ]
}

fn emote(
    match_id: &str,
    seat: PlayerId,
    ordinal: i32,
    emote_id: EmoteId,
    ms_since_trigger: i64,
    reply_to_opponent_ms: Option<i64>,
    pilot: Pilot,
) -> EmoteEventRow {
    EmoteEventRow {
        match_id: match_id.to_string(),
        seat,
        ordinal,
        emote_id,
        turn: 2,
        trigger_event: None,
        ms_since_trigger: Some(ms_since_trigger),
        reply_to_opponent_ms,
        pilot,
    }
}

/// m1's p1 sends no taunt (Polite), m1's p2 mostly taunts (BM), m2's p1 taunts half the time
/// (Balanced), and every other person's seat sends nothing (Silent). The AI's taunt is left out of
/// both the personas and the windows.
fn emotes() -> Vec<EmoteEventRow> {
    use EmoteId::*;
    use PlayerId::*;
    let person = Pilot::Human;
    vec![
        emote("m1", P1, 0, Greetings, 100, None, person),
        emote("m1", P2, 1, Threaten, 200, Some(500), person),
        emote("m1", P1, 2, WellPlayed, 300, Some(1_500), person),
        emote("m1", P2, 3, Laugh, 400, None, person),
        emote("m1", P2, 4, Greetings, 500, Some(2_500), person),
        emote("m2", P1, 0, Threaten, 600, None, person),
        emote("m2", P1, 1, Greetings, 700, None, person),
        emote("m5", P2, 0, Threaten, 99_999, Some(99_999), Pilot::Ai),
    ]
}

#[test]
fn r639000_timing_fit_fits_concede_draw_timeout_rematch_emotes_and_personas() {
    let rows = outcomes();
    // Two of the three people who lost conceded, on turns 8 and 12, 30 and 50 behind.
    let concede = fit_concede(&rows, 2).expect("two conceders");
    assert_eq!(concede.samples, 2);
    assert_eq!(concede.min_turn, 8);
    close(concede.share, 2.0 / 3.0);
    close(concede.deficit_mu, 40.0);
    close(concede.deficit_sigma, 10.0);
    assert_eq!(fit_concede(&rows, 3), None);

    // Offered a draw: m2's p2 (declined) and m3's p2 (accepted).
    assert_eq!(
        fit_draw_answer(&rows, 2),
        Some(DrawAnswerFit {
            samples: 2,
            accept_share: 0.5
        })
    );
    assert_eq!(fit_draw_answer(&rows, 3), None);

    // Own turns: m1 4 + 4, m2 6 + 6, m3 5 + 5, m4 5 + 4, m5's person 2, with three timeouts.
    let timeout = fit_timeouts(&rows, 41).expect("41 own turns");
    assert_eq!(timeout.samples, 41);
    close(timeout.chance_per_turn, 3.0 / 41.0);
    assert_eq!(fit_timeouts(&rows, 42), None);

    // Seven people's seats a rematch could follow: two offered first (m1's p2, m4's p1), and of the
    // two offered to first, m1's p1 met it.
    let rematch = fit_rematch(&rows, 7).expect("seven seats");
    assert_eq!(rematch.samples, 7);
    close(rematch.offer_share, 2.0 / 7.0);
    close(rematch.accept_share, 0.5);
    assert_eq!(fit_rematch(&rows, 8), None);

    // Seven people's emotes after a trigger (100 to 700 ms), three replies.
    let after: Vec<i64> = (1..=7).map(|at| at * 100).collect();
    assert_eq!(
        fit_emote_delay(&after, 7),
        Some(EmoteDelayFit {
            samples: 7,
            min_ms: 100.0,
            max_ms: 600.0
        })
    );
    assert_eq!(fit_emote_delay(&after, 8), None);
    let fits = fit_signals(&[], &rows, &emotes(), 3);
    assert_eq!(fits.emote_delay, fit_emote_delay(&after, 3));
    assert_eq!(
        fits.emote_reply_delay,
        Some(EmoteDelayFit {
            samples: 3,
            min_ms: 500.0,
            max_ms: 1_500.0
        })
    );

    // Nine people's seats: one Balanced, one Polite, one BM, six Silent, in `PERSONA_NAMES` order.
    let personas = fit_persona_weights(&rows, &emotes(), 9).expect("nine seats");
    assert_eq!(personas.samples, 9);
    for (weight, expected) in personas.weights.iter().zip([1.0, 1.0, 1.0, 6.0]) {
        close(*weight, expected / 9.0);
    }
    assert_eq!(fit_persona_weights(&rows, &emotes(), 10), None);
    assert_eq!(fits.persona_weights, fit_persona_weights(&rows, &emotes(), 3));
}

fn timed(
    match_id: &str,
    seat: &str,
    turn: i32,
    kind: &str,
    legal: i32,
    think_ms: i64,
    pilot: &str,
) -> ActionTimingRow {
    serde_json::from_value(json!({
        "matchId": match_id,
        "seat": seat,
        "seq": 1,
        "actionKind": kind,
        "legalCount": legal,
        "turn": turn,
        "thinkMs": think_ms,
        "firstInTurn": false,
        "pilot": pilot,
    }))
    .expect("an action timing")
}

#[test]
fn r639000_timing_fit_fits_the_complexity_slope_and_lethal_shift() {
    // Plays: think time 100 ms per legal action (slope 1 in the logs). Turn ends: the same 500 ms
    // whatever the board (slope 0). Both spread their ln(legal) by the same 2·ln(2)², so the
    // pooled slope is the mean, 0.5. The AI's slow plays would steepen it if they counted.
    let rows = vec![
        timed("c", "p1", 2, "play", 1, 100, "human"),
        timed("c", "p1", 2, "play", 2, 200, "human"),
        timed("c", "p1", 2, "play", 4, 400, "human"),
        timed("c", "p1", 2, "endTurn", 2, 500, "human"),
        timed("c", "p1", 2, "endTurn", 8, 500, "human"),
        timed("c", "p2", 2, "play", 4, 900_000, "ai"),
    ];
    let pooled = fit_complexity_slope(&rows, 2).expect("five moves");
    assert_eq!(pooled.samples, 5);
    close(pooled.value, 0.5);
    // Under three a bucket, the turn ends are no fit, and the plays alone give their own slope.
    let plays = fit_complexity_slope(&rows, 3).expect("three plays");
    assert_eq!(plays.samples, 3);
    close(plays.value, 1.0);
    assert_eq!(fit_complexity_slope(&rows, 4), None);
    // A board that never varies gives no slope.
    assert_eq!(fit_complexity_slope(&rows[3..4], 1), None);

    // The lethal shift: the winner's moves on the last turn of a game two people played to a hero's
    // death, against its bucket's fit and the slope.
    let fit = BucketFit {
        rank_bucket: None,
        action_kind: ActionType::Play,
        first_in_turn: false,
        samples: 30,
        mu: 6.0,
        sigma: 0.5,
        ln_legal_mean: 2f64.ln(),
    };
    let slope = 0.5;
    let won = |match_id: &str, reason| {
        vec![
            outcome(match_id, PlayerId::P1, Pilot::Human, Some(true), reason, 5, true),
            outcome(match_id, PlayerId::P2, Pilot::Human, Some(false), reason, 5, true),
        ]
    };
    let mut ends = won("lethal", GameOverReason::HeroDeath);
    ends.extend(won("conceded", GameOverReason::Concede));
    ends.push(outcome(
        "bot",
        PlayerId::P1,
        Pilot::Human,
        Some(true),
        GameOverReason::HeroDeath,
        5,
        true,
    ));
    ends.push(outcome(
        "bot",
        PlayerId::P2,
        Pilot::Ai,
        Some(false),
        GameOverReason::HeroDeath,
        5,
        true,
    ));
    let moves = vec![
        timed("lethal", "p1", 5, "play", 2, 400, "human"),
        timed("lethal", "p1", 5, "play", 8, 1_000, "human"),
        // Not the last turn, the loser's, a bucket with no fit, the AI's: none counts.
        timed("lethal", "p1", 4, "play", 2, 50_000, "human"),
        timed("lethal", "p2", 5, "play", 2, 50_000, "human"),
        timed("lethal", "p1", 5, "attack", 2, 50_000, "human"),
        timed("lethal", "p1", 5, "play", 2, 50_000, "ai"),
        // A game won by concession, and one against the AI: neither counts.
        timed("conceded", "p1", 5, "play", 2, 50_000, "human"),
        timed("bot", "p1", 5, "play", 2, 50_000, "human"),
    ];
    let shift = fit_lethal_shift(&moves, &ends, &[fit], slope, 2).expect("two lethal moves");
    assert_eq!(shift.samples, 2);
    let first = 400f64.ln() - 6.0;
    let second = 1_000f64.ln() - (6.0 + slope * (8f64.ln() - 2f64.ln()));
    close(shift.value, (first + second) / 2.0);
    assert_eq!(fit_lethal_shift(&moves, &ends, &[fit], slope, 3), None);
}

#[test]
fn r639000_timing_fit_prints_the_rows_to_paste_over_the_priors() {
    let fits = SignalFits {
        complexity_slope: Some(ScalarFit {
            samples: 120,
            value: 0.183_21,
        }),
        lethal_shift: None,
        concede: Some(ConcedeFit {
            samples: 44,
            share: 0.4,
            min_turn: 7,
            deficit_mu: 38.5,
            deficit_sigma: 17.25,
        }),
        draw_answer: Some(DrawAnswerFit {
            samples: 31,
            accept_share: 0.3,
        }),
        timeout: Some(TimeoutFit {
            samples: 900,
            chance_per_turn: 0.012_34,
        }),
        rematch: None,
        emote_delay: Some(EmoteDelayFit {
            samples: 60,
            min_ms: 700.0,
            max_ms: 3_100.0,
        }),
        emote_reply_delay: None,
        persona_weights: Some(PersonaWeights {
            samples: 50,
            weights: [0.3, 0.3, 0.1, 0.3],
        }),
    };
    assert_eq!(
        render_constants(&fits),
        "// jackioh-server timing-fit: paste each over the row of its name, crates/server/src/config.rs\n\
         pub const THINK_COMPLEXITY_SLOPE: ScalarFit = ScalarFit { samples: 120, value: 0.1832 };\n\
         // THINK_LETHAL_SHIFT: under TIMING_FIT_MIN_SAMPLES; keep the prior\n\
         pub const BOT_CONCEDE_FIT: ConcedeFit = ConcedeFit { samples: 44, share: 0.4000, min_turn: 7, deficit_mu: 38.5000, deficit_sigma: 17.2500 };\n\
         pub const BOT_DRAW_ANSWER_FIT: DrawAnswerFit = DrawAnswerFit { samples: 31, accept_share: 0.3000 };\n\
         pub const BOT_TIMEOUT_FIT: TimeoutFit = TimeoutFit { samples: 900, chance_per_turn: 0.0123 };\n\
         // BOT_REMATCH_FIT: under TIMING_FIT_MIN_SAMPLES; keep the prior\n\
         pub const BOT_EMOTE_DELAY_FIT: EmoteDelayFit = EmoteDelayFit { samples: 60, min_ms: 700.0000, max_ms: 3100.0000 };\n\
         // BOT_EMOTE_REPLY_DELAY_FIT: under TIMING_FIT_MIN_SAMPLES; keep the prior\n\
         pub const BOT_PERSONA_WEIGHTS: PersonaWeights = PersonaWeights { samples: 50, weights: [0.3000, 0.3000, 0.1000, 0.3000] };\n"
    );
    // With nothing recorded, every row keeps its prior.
    let empty = render_constants(&fit_signals(&[], &[], &[], 30));
    assert_eq!(
        empty
            .lines()
            .filter(|line| line.ends_with("keep the prior"))
            .count(),
        9
    );
}
