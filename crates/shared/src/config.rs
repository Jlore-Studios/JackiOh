//! The shared logic's numbers (CLAUDE.md rule 9): the server re-exports the ones its own code and
//! `serverConfig.ts` name (`crates/server/src/config.rs`), and the hosts that call this crate through
//! `jackioh-wasm-core` read them from its `constants` export, so none restates one.

use jackioh_engine::EmoteId;
use jackioh_engine::wire::PrintedRarity;
use serde::{Deserialize, Serialize};

use crate::cosmetics::CosmeticRarity;

/// Milliseconds in a second: rows and clocks are stamped in epoch milliseconds, and the series' pick
/// clock is stated in seconds.
pub const MS_PER_SECOND: i64 = 1000;

/// SPEC §11 R330: game wins that take a Conquest series — one with each deck of a trio, since a deck
/// that has won is locked. `tests/series.rs` asserts it equals the validator's
/// `TRIO_DECKS`. (R259's Best of 3 needed 2.)
pub const SERIES_WINS_NEEDED: i32 = 3;
/// SPEC §11 R334: the most games a series plays, drawn games included. Without a draw a series is
/// decided by its fifth game at the latest (2 × `SERIES_WINS_NEEDED` − 1: a side is then at three
/// wins), so the cap leaves room for two drawn games in the longest series and stops a run of draws
/// from holding both players for ever. At the cap more wins takes the series and equal wins is a
/// series draw. (R259's Best of 3 played at most 3.)
pub const SERIES_MAX_GAMES: i32 = 7;
/// SPEC §11 R333: how long both players have to pick their deck for the next game of a series.
pub const SERIES_PICK_SECONDS: i64 = 60;

/// #2437, R637, R2179001: the Temporary count at which a card in a hand is discarded at its owner's next end of
/// turn. A plain Temporary is Temporary 1; the face model's Temporary badge counts above it (`face/state.rs`).
pub const TEMPORARY_LAST_COUNT: i32 = 1;

/// Glitch (issue #170, SPEC §7) prints corrupted words (`face/glitch.rs`): the fixed seed every host jumbles with,
/// so every client prints the same jumble.
pub const GLITCH_SEED: u32 = 170;
/// Out of [`GLITCH_ODDS_SCALE`]: below this a Glitch character becomes a glyph.
pub const GLITCH_SWAP_IN: u32 = 45;
/// Out of [`GLITCH_ODDS_SCALE`]: below `GLITCH_SWAP_IN` plus this a Glitch character gains a combining mark.
pub const GLITCH_MARK_IN: u32 = 35;
pub const GLITCH_ODDS_SCALE: u32 = 100;

// ---------------------------------------------------------------------------------------------
// The match host's numbers (#3101): the clocks (SPEC §9.5, R79, R268, R744), the relays (R738) and
// the rematch window (R672, R2685000) that `host` decides with. `crates/server/src/config.rs`
// re-exports them, so `serverConfig.ts` and the server's code name them as before.
// ---------------------------------------------------------------------------------------------

/// R79: the turn clock belongs to the active player and ends a stalled turn.
pub const TURN_CLOCK_SECONDS: i64 = 75;
/// R79: a prompt held by the non-active player (e.g. a trap firing on the opponent's turn).
pub const PROMPT_CLOCK_SECONDS: i64 = 30;
/// SPEC §11 R268: the mulligan clock. Both seats' mulligans are open at once (R265), so one deadline
/// runs for both, armed when the window opens and never re-armed when one seat answers; on expiry
/// every seat still owing is timed out and keeps its whole hand. Longer than the prompt clock because
/// a mulligan reads a whole opening hand, shorter than the turn clock because nothing is played.
pub const MULLIGAN_CLOCK_SECONDS: i64 = 45;
/// R79: grace window after a disconnect before `disconnectExpired` ends the match as a loss.
pub const DISCONNECT_GRACE_SECONDS: i64 = 60;
/// R79, R389: hard wall-clock ceiling; reaching it ends the match as a draw via `ceilingReached`.
/// Patch v0.2.0 doubled the turn cap to 60 player-turns (B4.3), which at a full turn clock is 75
/// minutes, so the ceiling doubled with it and a slow game ends on the cap, not the clock.
pub const MATCH_CEILING_MINUTES: i64 = 120;
/// SPEC §11 R776000: how long a clock's server action whose log row the store failed to write waits
/// before its clock runs out again, so a store that keeps failing is retried at this pace until the
/// row lands, the match ends or the actor stops, never in a loop.
pub const MATCH_SERVER_ACTION_RETRY_MS: i64 = 1_000;
/// R1452: how long a bot waits to decide again when its move could not be written to the log, so a
/// store that keeps failing is retried at this pace, never in a loop.
pub const BOT_WRITE_RETRY_MS: i64 = 1_000;
/// SPEC §11 R738: the least time between two `aim` frames the host relays for one seat. Aims that
/// arrive faster are coalesced, never queued: the newest waits out the interval and goes alone, so
/// the opponent's arrow always ends where the sender's aim ended, at most this long behind it.
/// Ten a second follows a hover from target to target and bounds what a scripted client can push.
/// R2739001's `choiceHover` relays are coalesced per seat by the same interval, on a window of their
/// own.
pub const AIM_RELAY_INTERVAL_MS: i64 = 100;
/// SPEC §11 R672: how long one seat's rematch offer stands after a non-series match ends. Ten
/// minutes: long enough to read the result screen and press the button, short enough that a stale
/// offer cannot summon a game long after both players moved on.
pub const REMATCH_OFFER_TTL_MS: i64 = 600_000;
/// R672: a normal rematch moves the rating once.
pub const STAKE_NORMAL: i32 = 1;
/// R672: a double-or-nothing rematch moves it twice. Ranked matches only.
pub const STAKE_DOUBLE: i32 = 2;

// The hidden rating: Glicko-2 (SPEC §9.12, R603). The maths is `src/ranked/glicko2.rs`; the numbers were the
// server's (#3105), which re-exports them so its `config.rs` and its code name them as before.

/// SPEC §11 R603: the rating a new profile starts at. R79's Elo started at 1000, and Glicko-2's update
/// depends only on rating differences, so the ratings Elo left carry over unchanged as Glicko-2
/// ratings and only the deviation and volatility are new (migration 0022).
pub const RATING_START: f64 = 1000.0;
/// R603: a new profile's rating deviation: Glickman's starting value, the most uncertain rating.
pub const RATING_DEVIATION_START: f64 = 350.0;
/// R603: a new profile's rating volatility: Glickman's starting value.
pub const RATING_VOLATILITY_START: f64 = 0.06;
/// R603: Glicko-2's system constant τ, which bounds how fast volatility moves. Glickman's 0.3–1.2.
pub const GLICKO_TAU: f64 = 0.5;
/// R603: the factor between a displayed rating and Glicko-2's internal scale, 400 / ln 10. Glickman
/// writes it 173.7178; this is the same number to double precision (TS `400 / Math.LN10`).
pub const GLICKO_SCALE: f64 = 400.0 / std::f64::consts::LN_10;
/// R603: the volatility iteration stops once its bracket is narrower than this (Glickman's ε).
pub const GLICKO_CONVERGENCE: f64 = 0.000001;
/// R603: the most volatility iterations one update runs. The Illinois iteration converges in a
/// handful of steps on any real input; the cap only makes a non-finite input end rather than spin.
pub const GLICKO_MAX_ITERATIONS: u32 = 100;

// The visible ladder (SPEC §9.12, R605–R608). The rules are `src/ranked/ladder.rs`. PUBLIC: the client reads the
// shape (divisions, pips, placements) to draw a rank it is handed.

/// The shape of `RANK_TIER_PERCENTS` (SURFACE §4.2: a constant object becomes a const of a struct
/// named after it). Serializes as TS's object, keys in TS order.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct RankTierPercents {
    pub rotten: i32,
    pub normal: i32,
    pub large: i32,
    pub golden: i32,
    pub mythic: i32,
}

/// SPEC §11 R606: the share of active players each Grape tier is meant to hold, in whole percents,
/// lowest first: Rotten 12%, Normal 60%, Large 20%, Golden 7%, Mythic 1%. A player's hidden rating is
/// placed at its percentile among the season's placed players and read off these shares, so retuning
/// a boundary is a change here and nowhere else. Whole numbers, so a boundary is exact; they sum to 100.
pub const RANK_TIER_PERCENTS: RankTierPercents = RankTierPercents {
    rotten: 12,
    normal: 60,
    large: 20,
    golden: 7,
    mythic: 1,
};
/// R605: divisions per Grape tier, III up to I.
pub const RANK_DIVISIONS_PER_TIER: i32 = 3;
/// R605: pips per division. A division is climbed one pip at a time; a full division promotes.
pub const RANK_PIPS_PER_DIVISION: i32 = 3;
/// R605: rated games a season's placements take. Until they are played the player is a Raisin.
pub const RANK_PLACEMENT_GAMES: i32 = 5;
/// R606: pips a win gives before any bonus.
pub const RANK_WIN_PIPS: i32 = 1;
/// R606: pips a loss takes before any adjustment. A draw moves no pip.
pub const RANK_LOSS_PIPS: i32 = 1;
/// R606: the win that makes a streak this long, and every win after it, earns the streak bonus.
pub const RANK_STREAK_LENGTH: i32 = 3;
/// R606: the streak bonus, in pips, earned below Mythic Grape only.
pub const RANK_STREAK_BONUS_PIPS: i32 = 1;
/// R606: how far, in pips, the rank the hidden rating calls for must sit from the visible rank
/// before a game's pips lean toward it. One division: inside it a game is a plain win or loss.
pub const RANK_CONVERGENCE_GAP_PIPS: i32 = 3;
/// R606: how many pips the lean adds — to a win when the rating is above the visible rank, to a loss
/// when it is below. Gentle by design: never more than this, and a win never gives fewer pips than a
/// plain win, nor a loss take fewer than a plain loss.
pub const RANK_CONVERGENCE_PIPS: i32 = 1;
/// R608: Jlorious is the top this-many Mythic Grape players by hidden rating.
pub const JLORIOUS_SIZE: usize = 100;

// Seasons (SPEC §9.12, R609). The reset is `src/ranked/season.rs`.

/// SPEC §11 R609: how far a season's soft reset pulls each rating toward the players' mean: 0 keeps
/// every rating, 1 puts everyone on the mean. Half way.
pub const SEASON_RESET_STRENGTH: f64 = 0.5;
/// R609: the deviation a season's reset adds, combined in quadrature as Glicko adds uncertainty for
/// time away: `min(√(RD² + this²), RATING_DEVIATION_START)`. A settled 60 becomes about 160.
pub const SEASON_RESET_DEVIATION_BOOST: f64 = 150.0;

/// R1560000: the deviation each AI promotion adds to a ladder bot's rating before its next rated game,
/// combined in quadrature as R609's `SEASON_RESET_DEVIATION_BOOST` is: `min(√(RD² + n × this²),
/// RATING_DEVIATION_START)` for n promotions since it was last rated. About one promotion's measured
/// gain over its parent (generations 10 to 13 gained 35 to 241 Elo), and the season reset's own.
pub const BOT_PROMOTION_DEVIATION_BOOST: f64 = 150.0;

// The leaderboard's sort key (`src/ranked/board.rs`, #3105): one integer per rating record that a store
// sorts as it is, an OrderedDataStore among them. It fits 53 bits, so a Luau number holds it exactly.

/// A listed, placed Grape player's key is this plus their ladder position (0 up to the ladder's top), so
/// the key rises with the position and every Grape key sits above an unplaced player's 0.
pub const SORT_KEY_GRAPE_BASE: i64 = 1;
/// A Jlorious player's key is this plus their rating in fixed point: above every Grape key (the ladder has
/// a few dozen positions), and below 2^51.
pub const SORT_KEY_JLORIOUS_BASE: i64 = 1 << 50;
/// The fractional bits of the rating inside a Jlorious key: two ratings further apart than 2^-36 (about
/// 1.5e-11 of a point) get distinct keys, in the ratings' order. A rating is held in 14 whole bits, so one
/// of 16384 or more (a hand-set rating stops at `DEV_RATING_MAX`, 3000) shares the top key, and one below
/// 0 the bottom.
pub const SORT_KEY_RATING_BITS: u32 = 36;

/// Milliseconds in a day, a UTC day included (an epoch-ms timestamp has no leap seconds).
pub const MS_PER_DAY: i64 = 86_400_000;

/// SPEC §11 R640001: the widest the rating window gets, in rating points: its width at the 60 s mark
/// where it used to go uncapped. Two people further apart than this never pair, however long they
/// wait; past it the empty-queue fallback (R1453) seats a bot instead.
pub const MAX_FAIR_GAP: i64 = 400;

/// SPEC §11 R654: minimum sample of games for a card row to display a win-rate percentage. Below it,
/// the row displays "not enough games" and has no tier (R3022000). `GET /api/stats/cards` sends no
/// played or drawn delta, which waited for this many on each side (R3185000).
pub const CARD_STATS_MIN_SAMPLE: i64 = 20;

/// R1442: one bucket's log-normal fit of human think times: `ln(think_ms)` has mean `mu` and
/// standard deviation `sigma` over `samples` moves of `action_kind` (an `ActionType` literal), made
/// first or later in the turn, by players in `rank_bucket` (a `RankTier` literal; `None` for the
/// moves no tier was read for). R639000: `ln_legal_mean` is the mean of `ln(legal_count)` over the
/// same moves, the complexity `THINK_COMPLEXITY_SLOPE` is measured from.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
pub struct ThinkTimeFit {
    pub rank_bucket: Option<&'static str>,
    pub action_kind: &'static str,
    pub first_in_turn: bool,
    pub samples: usize,
    pub mu: f64,
    pub sigma: f64,
    pub ln_legal_mean: f64,
}

/// R1442: the think-time fits, pasted by hand from `jackioh-server timing-fit` (CLAUDE.md rule 9);
/// the ladder bots (#636) read it. Empty until enough human games are recorded.
pub const THINK_TIME_FITS: &[ThinkTimeFit] = &[];

/// R639000: one fitted number: `value`, over `samples` observations.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
pub struct ScalarFit {
    pub samples: usize,
    pub value: f64,
}

/// R639000: how people concede. `share` of the players who lose concede at all; those who do never
/// concede before turn `min_turn`, and concede once they stand `deficit` behind (the AI's evaluation
/// from their side, negated, R1442), a threshold distributed normally with mean `deficit_mu` and
/// standard deviation `deficit_sigma`, and only while they trail their opponent (`Standing.behind`).
/// That deficit is not centered on an even board: it counts the enemy hero's health against the seat,
/// so an even opening reads about 30 behind. `samples` is the concessions it was fitted on.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
pub struct ConcedeFit {
    pub samples: usize,
    pub share: f64,
    pub min_turn: i32,
    pub deficit_mu: f64,
    pub deficit_sigma: f64,
}

/// R639000: how people answer a draw offer: `accept_share` of `samples` seats offered one accepted.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
pub struct DrawAnswerFit {
    pub samples: usize,
    pub accept_share: f64,
}

/// R639000: how often a person's clock runs out: `chance_per_turn` of `samples` own turns.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
pub struct TimeoutFit {
    pub samples: usize,
    pub chance_per_turn: f64,
}

/// R639000: how people answer the end of a game that may be rematched (R672), over `samples` seats:
/// `offer_share` offer a rematch the opponent never meets, and `accept_share` of the seats whose
/// opponent offered first meet it.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
pub struct RematchFit {
    pub samples: usize,
    pub offer_share: f64,
    pub accept_share: f64,
}

/// R639000, R645: the window an emote's delay is drawn from, uniformly, in ms, fitted over
/// `samples` emotes.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
pub struct EmoteDelayFit {
    pub samples: usize,
    pub min_ms: f64,
    pub max_ms: f64,
}

/// R639000, R645: the share of bots dealt each persona, in `PERSONA_NAMES` order, fitted over
/// `samples` people's seats.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
pub struct PersonaWeights {
    pub samples: usize,
    pub weights: [f64; 4],
}

/// R639000: how much longer a think time grows per unit of `ln(legal_count)` above its fit's
/// `ln_legal_mean`, in `ln(think_ms)`: a board with more to do takes longer.
pub const THINK_COMPLEXITY_SLOPE: ScalarFit = ScalarFit {
    samples: 0,
    value: 0.2,
};

/// R639000: how much longer the moves of a turn that wins take, in `ln(think_ms)`: a person counts
/// the damage before they swing.
pub const THINK_LETHAL_SHIFT: ScalarFit = ScalarFit {
    samples: 0,
    value: 0.4,
};

/// R639000: the concession model a ladder bot draws its temper from. The prior's threshold sits about
/// 40 past an even opening's deficit, a game clearly lost. R2660000: a bot reads only `share`, whether
/// it is one who concedes at all, and concedes only facing a lethal it cannot stop
/// (`jackioh_ai::unstoppable_lethal`); `min_turn` and the deficit stay fitted, unread.
pub const BOT_CONCEDE_FIT: ConcedeFit = ConcedeFit {
    samples: 0,
    share: 0.4,
    min_turn: 6,
    deficit_mu: 70.0,
    deficit_sigma: 20.0,
};

/// R639000: how often a ladder bot that stands behind accepts a draw offer.
pub const BOT_DRAW_ANSWER_FIT: DrawAnswerFit = DrawAnswerFit {
    samples: 0,
    accept_share: 0.25,
};

/// R2660001: when a ladder bot offers a draw: at the start of its own turn, while it trails its
/// opponent by at least `min_behind`, on the scale of `Standing.behind` (about one point per point of
/// hero health or of a unit's stats), with the chance `chance_per_turn`. Hand-set: `timing-fit` fits
/// no offer rate.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
pub struct DrawOfferRule {
    pub chance_per_turn: f64,
    pub min_behind: f64,
}

/// R2660001: a ladder bot offers a draw on one turn start in twenty while it trails by ten points or
/// more, a 4/4's worth and more.
pub const BOT_DRAW_OFFER: DrawOfferRule = DrawOfferRule {
    chance_per_turn: 0.05,
    min_behind: 10.0,
};

/// R639000: how often a ladder bot lets its turn clock run out.
pub const BOT_TIMEOUT_FIT: TimeoutFit = TimeoutFit {
    samples: 0,
    chance_per_turn: 0.01,
};

/// R639000: how often a ladder bot offers a rematch, and meets one.
pub const BOT_REMATCH_FIT: RematchFit = RematchFit {
    samples: 0,
    offer_share: 0.15,
    accept_share: 0.35,
};

/// R639000, R645: the window a ladder bot's triggered emote waits inside, after its trigger.
pub const BOT_EMOTE_DELAY_FIT: EmoteDelayFit = EmoteDelayFit {
    samples: 0,
    min_ms: 800.0,
    max_ms: 2500.0,
};

/// R639000, R645: the window a ladder bot's reply to a person's emote waits inside.
pub const BOT_EMOTE_REPLY_DELAY_FIT: EmoteDelayFit = EmoteDelayFit {
    samples: 0,
    min_ms: 800.0,
    max_ms: 2500.0,
};

/// R639000, R645: the weights a new ladder bot's persona is dealt by.
pub const BOT_PERSONA_WEIGHTS: PersonaWeights = PersonaWeights {
    samples: 0,
    weights: [0.4, 0.25, 0.2, 0.15],
};

/// R1452, R747000: one strength a ladder bot plays at: the AI's search budget, its style (`AiStyle`:
/// the softmax temperature over the finalists, in evaluation points, and the chance it passes over a
/// lethal it found), its skill (the chance each decision follows that AI, the rest the skill miss,
/// R2214000) and the rating its `bot_ratings` row is seeded with (R610), which is what
/// `expected_elo_for_skill` measured for that skill (`BOT_SEED_RATINGS`, R748000). Always on the
/// spec's resources: a strength is never a practice handicap (R180). The chance is rolled at each
/// decision that finds a lethal, and the bot decides again after every move, so a bot that passed
/// over one often finds it again before the turn ends: it misses a lethal for the whole turn less
/// often than the chance says, which calibration (R610's ratings) takes in as it takes in everything
/// else.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
pub struct BotStrength {
    pub budget: jackioh_ai::SearchBudget,
    pub temperature: f64,
    pub missed_lethal_chance: f64,
    pub skill: f64,
    pub seed_rating: f64,
}

/// R748000, R747000, R1454: the rating each strength seeds a new bot with, weakest first: the Elo
/// column of `SKILL_ELO_TABLE`, read off in `const`, so no number is stated twice: the curve's
/// readings at the strengths' skills (`expected_elo_for_skill`, whose test holds them equal), the
/// reference (the strongest strength) at `SKILL_ELO_ANCHOR`. `jackioh-server bot-calibrate` measures
/// the five strengths as built (their budget, temperature, missed-lethal chance and skill together,
/// which the curve, measured at the arena's budget, does not) against each other, and its `--write`
/// rewrites the rows of `SKILL_ELO_TABLE` (docs/bot-calibration.md). A new bot takes
/// `BOT_STRENGTHS[i]`'s; a bot already in the pool keeps the R610 rating it has earned.
/// The rows are generation 13's calibration with the skill miss (R2214000) (docs/bot-calibration.md,
/// Results).
pub const BOT_SEED_RATINGS: [f64; SKILL_ELO_TABLE.len()] = seed_ratings_from_curve();

/// R748000: `SKILL_ELO_TABLE`'s Elo column, which `BOT_SEED_RATINGS` is.
const fn seed_ratings_from_curve() -> [f64; SKILL_ELO_TABLE.len()] {
    let mut seeds = [0.0; SKILL_ELO_TABLE.len()];
    let mut index = 0;
    while index < seeds.len() {
        seeds[index] = SKILL_ELO_TABLE[index].1;
        index += 1;
    }
    seeds
}

/// R2575000: the search budget of `BOT_STRENGTHS[2]` and `[3]`: the practice AI's `AI_BUDGET` before
/// issue #2575 doubled it, kept so that `BOT_SEED_RATINGS`, calibrated on it (R748000), still hold.
/// Raising it is a person's call, with a calibration after it.
pub const BOT_MID_STRENGTH_BUDGET: jackioh_ai::SearchBudget = jackioh_ai::SearchBudget {
    nodes: 600,
    lethal_nodes: 150,
    determinizations: 3,
    beam_width: 4,
    root_branching: 20,
    branching: 6,
    max_depth: 8,
    finalists: 3,
};

/// R1454, R1452: the strengths a new bot is drawn from, weakest first. A bot keeps its index
/// (`bots.strength`), so a change here changes every bot of that index.
pub const BOT_STRENGTHS: &[BotStrength] = &[
    BotStrength {
        budget: jackioh_ai::SearchBudget {
            nodes: 150,
            lethal_nodes: 40,
            determinizations: 1,
            beam_width: 2,
            root_branching: 10,
            branching: 4,
            max_depth: 6,
            finalists: 3,
        },
        temperature: 3.0,
        missed_lethal_chance: 0.4,
        skill: 0.0,
        seed_rating: BOT_SEED_RATINGS[0],
    },
    BotStrength {
        budget: jackioh_ai::SearchBudget {
            nodes: 300,
            lethal_nodes: 80,
            determinizations: 2,
            beam_width: 3,
            root_branching: 14,
            branching: 5,
            max_depth: 7,
            finalists: 3,
        },
        temperature: 2.0,
        missed_lethal_chance: 0.25,
        skill: 0.25,
        seed_rating: BOT_SEED_RATINGS[1],
    },
    BotStrength {
        budget: BOT_MID_STRENGTH_BUDGET,
        temperature: 1.0,
        missed_lethal_chance: 0.12,
        skill: 0.5,
        seed_rating: BOT_SEED_RATINGS[2],
    },
    BotStrength {
        budget: BOT_MID_STRENGTH_BUDGET,
        temperature: 0.4,
        missed_lethal_chance: 0.05,
        skill: 0.75,
        seed_rating: BOT_SEED_RATINGS[3],
    },
    BotStrength {
        budget: jackioh_ai::SearchBudget {
            nodes: 1200,
            lethal_nodes: 300,
            determinizations: 4,
            beam_width: 5,
            root_branching: 24,
            branching: 7,
            max_depth: 9,
            finalists: 4,
        },
        temperature: 0.0,
        missed_lethal_chance: 0.0,
        skill: 1.0,
        seed_rating: BOT_SEED_RATINGS[4],
    },
];

/// R748000: the AI generation (`crates/ai/generation.json`) `BOT_SEED_RATINGS` were measured on.
/// `bot-calibrate` shifts its anchor by the promotions after it.
pub const BOT_CALIBRATION_GENERATION: i64 = 13;

/// R747000, R748000: the Elo the reference, this build's AI at full skill, is held at in
/// `SKILL_ELO_TABLE`, and the strongest bot's seed: 1200 from R1454 until generation 13's calibration
/// added the Elo of the promotions of generations 10 to 13 (docs/bot-calibration.md, Results);
/// `bot-calibrate --write` rewrites it with the rows. Not a new player's `RATING_START`, because the
/// pairing seats no bot for a player rated above every bot (R1453) and a player past the middle of
/// the ladder would otherwise have none.
pub const SKILL_ELO_ANCHOR: f64 = 1832.0;

/// R747000: the skill-to-Elo curve of the AI at the arena's budget, one row per skill, ascending in
/// skill from `AI_SKILL_MIN` to `AI_SKILL_MAX`: the Elo a seat at that skill is expected to hold, the
/// reference (full skill) at `SKILL_ELO_ANCHOR`. The rows are measured, not set: `cargo jackioh arena`
/// games of each skill against the reference and against the random policy, scored as Elo gaps and
/// rounded to 10 by the protocol in `crates/server/README.md` ("Skill to Elo"), where the games'
/// counts are. A new best AI, or a set that ships, is measured again by the same protocol and the
/// rows pasted over, or by R748000's calibration, whose readings of the strengths as built are the
/// rows since generation 13, and with the skill miss since R2214000: `bot-calibrate --write` rewrites
/// them here (docs/bot-calibration.md, Results); the live Glicko ratings (R603) are the ongoing
/// calibration.
pub const SKILL_ELO_TABLE: &[(f64, f64)] = &[
    (0.0, 1517.0),
    (0.25, 1576.0),
    (0.5, 1666.0),
    (0.75, 1716.0),
    (1.0, SKILL_ELO_ANCHOR),
];

/// R747000: the Elo a seat at `skill` is expected to hold, read off `SKILL_ELO_TABLE` by straight
/// lines between its rows; a skill outside the range reads as the end it is past (`clamp_skill`).
pub fn expected_elo_for_skill(skill: f64) -> f64 {
    let skill = jackioh_ai::clamp_skill(skill);
    for pair in SKILL_ELO_TABLE.windows(2) {
        let ((low_skill, low_elo), (high_skill, high_elo)) = (pair[0], pair[1]);
        if skill <= high_skill {
            return low_elo + (high_elo - low_elo) * (skill - low_skill) / (high_skill - low_skill);
        }
    }
    SKILL_ELO_TABLE.last().map_or(SKILL_ELO_ANCHOR, |&(_, elo)| elo)
}

/// R1454: the pool never holds fewer active bots than this, however few people play. R2778001: 40
/// (8 before), so a player grinding the ladder never runs out of bots on, free and not among their
/// last `BOT_RECENT_OPPONENTS`.
pub const BOT_POOL_MIN: usize = 40;

/// R1454: nor more than this, however many do (R2778001: 400, 200 before).
pub const BOT_POOL_MAX: usize = 400;

/// R1454: active bots per person who finished a live game in the last
/// `BOT_POOL_ACTIVE_WINDOW_DAYS`, before the clamp (R2778001: one each, a quarter before).
pub const BOT_POOL_PER_WEEKLY_ACTIVE: f64 = 1.0;

/// R1454: the window "weekly active" counts people over.
pub const BOT_POOL_ACTIVE_WINDOW_DAYS: i64 = 7;

/// R1555000: the place among the season's placed people (highest rating first) whose rating starts
/// the top bots' grind, so one person far ahead does not.
pub const BOT_GRIND_REFERENCE_RANK: usize = 3;

/// R1555000: the grind starts at a refresh when that person is more than this above every active bot.
pub const BOT_GRIND_GAP: f64 = 200.0;

/// R1555000: and stops only once a bot is within `BOT_GRIND_GAP` less this of them, so it does not
/// flap.
pub const BOT_GRIND_HYSTERESIS: f64 = 100.0;

/// R1555000: how many of the highest-rated active bots grind.
pub const BOT_GRINDERS: usize = 3;

/// R1555000: the hours a day a grinder is on, in place of its own schedule's when that is fewer.
pub const BOT_GRIND_ACTIVE_HOURS: i32 = 20;

/// R1555000: the least time between a match between two bots ending and the next one's kickoff.
pub const BOT_VS_BOT_MIN_GAP_SECONDS: i64 = 300;

/// R1453, R641000: the shortest a ticket of any mode waits before it may pair with a bot.
pub const BOT_FALLBACK_MIN_SECONDS: i64 = 15;

/// R1453: the longest; each ticket's wait is drawn from its id and the server's key between the
/// two, inclusive.
pub const BOT_FALLBACK_MAX_SECONDS: i64 = 40;

/// R1453: a player is not paired with a bot they faced in this many of their last games.
pub const BOT_RECENT_OPPONENTS: usize = 5;

/// R1454: the fewest hours a day a bot is on (its schedule, `bots.active_hours`; R2778001: 12, 8
/// before). A bot drawn before keeps its schedule.
pub const BOT_ACTIVE_HOURS_MIN: i32 = 12;

/// R1454: the most hours a day a bot is on (R2778001: 18, 16 before), still under a grinder's
/// `BOT_GRIND_ACTIVE_HOURS`.
pub const BOT_ACTIVE_HOURS_MAX: i32 = 18;

/// R1454: hours in the day a bot's schedule wraps around (UTC).
pub const BOT_HOURS_PER_DAY: i32 = 24;

/// R1452: the fastest a bot thinks, as a multiple of the think time it draws (its `bots.speed`).
pub const BOT_SPEED_MIN: f64 = 0.75;

/// R1452: the slowest.
pub const BOT_SPEED_MAX: f64 = 1.35;

/// R1454: the fewest Best-of-1 decks a bot carries.
pub const BOT_DECKS_MIN: usize = 2;

/// R1454: the most.
pub const BOT_DECKS_MAX: usize = 3;

/// R1454: how many decks the pool tries in all to find a bot's legal ones.
pub const BOT_DECK_DRAW_ATTEMPTS: usize = 20;

/// R1454: the youngest a bot's account reads (its profile's `created_at`), in days.
pub const BOT_ACCOUNT_AGE_MIN_DAYS: i64 = 1;

/// R1454: the oldest.
pub const BOT_ACCOUNT_AGE_MAX_DAYS: i64 = 180;

/// R1454: the share of bots that keep the `Player#n` name a new account is given, as people do
/// who skip the username prompt (R1435).
pub const BOT_DEFAULT_USERNAME_SHARE: f64 = 0.3;

/// R1454: one shape a bot's username may take, filled from the word lists beside the pool
/// (`crates/shared/src/bots/names/`).
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum BotNameShape {
    /// R1454: the word as the list spells it.
    Bare,
    /// R1454: the word lowercased.
    Lower,
    /// R1454: both words capitalised and run together.
    Camel,
    /// R1454: both words lowercased and run together.
    LowerPair,
    /// R1454: both words lowercased with `_` between them.
    SnakePair,
    /// R1454: both words lowercased and run together, then a number.
    LowerPairDigits,
    /// R1454: the word, then a number.
    Digits,
    /// R1454: the word lowercased, then a number.
    LowerDigits,
    /// R1454: the word, then `_`, then a number.
    SnakeDigits,
    /// R1454: the word inside `xX…Xx` (or `xX_…_Xx`).
    Xx,
    /// R1454: an affix from `PREFIXES` capitalised with the word.
    Prefix,
    /// R1454: the word with an affix from `SUFFIXES`.
    Suffix,
    /// R1454: the word with a fan affix from `FAN_SUFFIXES`.
    FanSuffix,
    /// R1454: a fan affix from `FAN_PREFIXES` capitalised with the word.
    FanPrefix,
    /// R1454: the word in leetspeak (`a→4`, `e→3`, `o→0`, `i→1`).
    Leet,
    /// R1454: the word lowercased with one vowel stretched.
    Stretch,
    /// R1454: the word with its last letter doubled or an `ii`/`zz`/`_` ending.
    Doubled,
    /// R1454: nonsense syllables strung from the consonant-vowel table.
    Syllables,
    /// R1454: the given name with an initial (`LuisM`, `Luis_M`).
    Initial,
    /// R1454: the given name's diminutive (`Luisito`, `Lucita`).
    Diminutive,
    /// R1454: the given name (or its diminutive) with `Gamer` and maybe a year.
    Gamer,
    /// R1454: the word with the first `BOT_NAME_COMPOUND_LETTERS` letters of the lead word.
    Compound,
}

/// R1454: the share of named bots whose username is drawn from plain words.
pub const BOT_NAME_SHARE_PLAIN: f64 = 0.22;

/// R1454: the share of named bots whose username is drawn from nonsense words.
pub const BOT_NAME_SHARE_NONSENSE: f64 = 0.08;

/// R1454: the share of named bots whose username is drawn from gamer filler.
pub const BOT_NAME_SHARE_GAMER: f64 = 0.05;

/// R1454: the share of named bots whose username is drawn from edgy words.
pub const BOT_NAME_SHARE_EDGY: f64 = 0.12;

/// R1454: the share of named bots whose username is drawn from anime references.
pub const BOT_NAME_SHARE_ANIME: f64 = 0.13;

/// R1454: the share of named bots whose username is drawn from Han characters.
pub const BOT_NAME_SHARE_CHINESE_HAN: f64 = 0.05;

/// R1454: the share of named bots whose username is drawn from pinyin.
pub const BOT_NAME_SHARE_CHINESE_PINYIN: f64 = 0.05;

/// R1454: the share of named bots whose username is drawn from given names other than Luis.
pub const BOT_NAME_SHARE_GIVEN: f64 = 0.06;

/// R1454: the share of named bots whose username is drawn from crude and meme bait.
pub const BOT_NAME_SHARE_CRUDE: f64 = 0.09;

/// R1454: the share of named bots whose username is drawn from other references.
pub const BOT_NAME_SHARE_OTHER_REFS: f64 = 0.06;

/// R1454: the share of named bots whose username is drawn from politics and public life.
pub const BOT_NAME_SHARE_POLITICS: f64 = 0.06;

/// R1454: the share of all bots named Luis, the most common given name on the ladder: about four
/// in a full pool of 200, the later ones tagged `Luis#1`, `Luis#2`, …
pub const BOT_NAME_LUIS_SHARE: f64 = 0.02;

/// R1454: how many names the bot's register draws before it keeps its `Player#n`.
pub const BOT_NAME_DRAW_ATTEMPTS: usize = 10;

/// R1454: the smallest two-digit number a `{word}{digits}` shape draws.
pub const BOT_NAME_TWO_DIGIT_MIN: i64 = 10;

/// R1454: the largest two-digit number a `{word}{digits}` shape draws.
pub const BOT_NAME_TWO_DIGIT_MAX: i64 = 99;

/// R1454: the earliest birth year a `{word}{digits}` shape draws.
pub const BOT_NAME_BIRTH_YEAR_MIN: i64 = 1985;

/// R1454: the latest birth year a `{word}{digits}` shape draws.
pub const BOT_NAME_BIRTH_YEAR_MAX: i64 = 2012;

/// R1454: the fewest extra repeats a stretched letter draws.
pub const BOT_NAME_STRETCH_MIN: i64 = 2;

/// R1454: the most extra repeats a stretched letter draws.
pub const BOT_NAME_STRETCH_MAX: i64 = 5;

/// R1454: the fewest syllables a nonsense shape draws.
pub const BOT_NAME_SYLLABLES_MIN: i64 = 1;

/// R1454: the most syllables a nonsense shape draws.
pub const BOT_NAME_SYLLABLES_MAX: i64 = 3;

/// R1454: the share of drawn numbers that are a birth year.
pub const BOT_NAME_YEAR_SHARE: f64 = 0.35;

/// R1454: the share of drawn numbers that are a filler number (`69`, `420`, …).
pub const BOT_NAME_FILLER_NUMBER_SHARE: f64 = 0.2;

/// R1454: the share of drawn Chinese numbers that are a birth year.
pub const BOT_NAME_CHINESE_YEAR_SHARE: f64 = 0.3;

/// R1454: how many letters of the lead word a `Compound` shape keeps.
pub const BOT_NAME_COMPOUND_LETTERS: usize = 3;

/// R1454: the filler numbers a `{word}{digits}` shape draws from.
pub const BOT_NAME_FILLER_NUMBERS: &[i64] = &[69, 420, 1337, 666, 777, 999, 123, 360, 101];

/// R1454: the numbers a Chinese shape draws from (the Chinese-internet set).
pub const BOT_NAME_CHINESE_NUMBERS: &[i64] = &[520, 1314, 666, 999, 8, 233, 2333, 521];

/// R1454: the shapes a plain register draws, each with its weight.
pub const BOT_NAME_SHAPES_PLAIN: &[(BotNameShape, f64)] = &[
    (BotNameShape::Camel, 0.28),
    (BotNameShape::LowerPair, 0.10),
    (BotNameShape::SnakePair, 0.08),
    (BotNameShape::LowerPairDigits, 0.06),
    (BotNameShape::Digits, 0.14),
    (BotNameShape::LowerDigits, 0.09),
    (BotNameShape::Prefix, 0.07),
    (BotNameShape::Suffix, 0.06),
    (BotNameShape::Leet, 0.05),
    (BotNameShape::Stretch, 0.04),
    (BotNameShape::Doubled, 0.03),
];

/// R1454: the shapes a nonsense register draws, each with its weight.
pub const BOT_NAME_SHAPES_NONSENSE: &[(BotNameShape, f64)] = &[
    (BotNameShape::Bare, 0.25),
    (BotNameShape::Syllables, 0.30),
    (BotNameShape::Digits, 0.12),
    (BotNameShape::Camel, 0.08),
    (BotNameShape::Stretch, 0.08),
    (BotNameShape::Doubled, 0.06),
    (BotNameShape::Xx, 0.06),
    (BotNameShape::LowerDigits, 0.05),
];

/// R1454: the shapes a gamer register draws, each with its weight.
pub const BOT_NAME_SHAPES_GAMER: &[(BotNameShape, f64)] = &[
    (BotNameShape::Camel, 0.30),
    (BotNameShape::LowerPair, 0.10),
    (BotNameShape::Bare, 0.10),
    (BotNameShape::Digits, 0.15),
    (BotNameShape::Prefix, 0.10),
    (BotNameShape::Xx, 0.10),
    (BotNameShape::Stretch, 0.10),
    (BotNameShape::Doubled, 0.05),
];

/// R1454: the shapes an edgy register draws, each with its weight.
pub const BOT_NAME_SHAPES_EDGY: &[(BotNameShape, f64)] = &[
    (BotNameShape::Camel, 0.25),
    (BotNameShape::LowerPair, 0.08),
    (BotNameShape::SnakePair, 0.05),
    (BotNameShape::Bare, 0.07),
    (BotNameShape::Digits, 0.12),
    (BotNameShape::Xx, 0.12),
    (BotNameShape::Prefix, 0.08),
    (BotNameShape::Leet, 0.10),
    (BotNameShape::Stretch, 0.03),
    (BotNameShape::Doubled, 0.05),
    (BotNameShape::Suffix, 0.05),
];

/// R1454: the shapes an anime or other-reference register draws, each with its weight.
pub const BOT_NAME_SHAPES_REFERENCE: &[(BotNameShape, f64)] = &[
    (BotNameShape::Bare, 0.25),
    (BotNameShape::Lower, 0.08),
    (BotNameShape::FanSuffix, 0.20),
    (BotNameShape::FanPrefix, 0.06),
    (BotNameShape::SnakeDigits, 0.10),
    (BotNameShape::Digits, 0.10),
    (BotNameShape::Xx, 0.08),
    (BotNameShape::Leet, 0.05),
    (BotNameShape::Camel, 0.04),
    (BotNameShape::Doubled, 0.04),
];

/// R1454: the shapes a crude register draws, each with its weight.
pub const BOT_NAME_SHAPES_CRUDE: &[(BotNameShape, f64)] = &[
    (BotNameShape::Bare, 0.35),
    (BotNameShape::Lower, 0.10),
    (BotNameShape::Digits, 0.15),
    (BotNameShape::SnakeDigits, 0.05),
    (BotNameShape::Xx, 0.10),
    (BotNameShape::Suffix, 0.08),
    (BotNameShape::Prefix, 0.05),
    (BotNameShape::Leet, 0.07),
    (BotNameShape::Doubled, 0.05),
];

/// R1454: the shapes a politics register draws, each with its weight.
pub const BOT_NAME_SHAPES_POLITICS: &[(BotNameShape, f64)] = &[
    (BotNameShape::Bare, 0.55),
    (BotNameShape::Lower, 0.10),
    (BotNameShape::Digits, 0.15),
    (BotNameShape::FanSuffix, 0.12),
    (BotNameShape::FanPrefix, 0.08),
];

/// R1454: the shapes a given-name register draws, each with its weight.
pub const BOT_NAME_SHAPES_GIVEN: &[(BotNameShape, f64)] = &[
    (BotNameShape::Bare, 0.22),
    (BotNameShape::LowerDigits, 0.18),
    (BotNameShape::Digits, 0.08),
    (BotNameShape::Initial, 0.12),
    (BotNameShape::Diminutive, 0.10),
    (BotNameShape::Gamer, 0.06),
    (BotNameShape::Prefix, 0.08),
    (BotNameShape::Compound, 0.06),
    (BotNameShape::Xx, 0.06),
    (BotNameShape::Lower, 0.04),
];

/// R1454: the shapes a Han register draws, each with its weight.
pub const BOT_NAME_SHAPES_CHINESE_HAN: &[(BotNameShape, f64)] =
    &[(BotNameShape::Bare, 0.55), (BotNameShape::Digits, 0.45)];

/// R1454: the shapes a pinyin register draws, each with its weight.
pub const BOT_NAME_SHAPES_CHINESE_PINYIN: &[(BotNameShape, f64)] = &[
    (BotNameShape::Camel, 0.25),
    (BotNameShape::LowerPair, 0.15),
    (BotNameShape::SnakePair, 0.12),
    (BotNameShape::LowerPairDigits, 0.18),
    (BotNameShape::LowerDigits, 0.20),
    (BotNameShape::Lower, 0.10),
];

/// R1452: the prior for a bot's first move of a turn, until `THINK_TIME_FITS` holds a fit for it:
/// the mean of `ln(think_ms)`.
pub const BOT_THINK_PRIOR_FIRST_MU: f64 = 8.7;

/// R1452: the first move's prior, the standard deviation of `ln(think_ms)`.
pub const BOT_THINK_PRIOR_FIRST_SIGMA: f64 = 0.6;

/// R1452: the prior for a bot's later moves of a turn: the mean of `ln(think_ms)`.
pub const BOT_THINK_PRIOR_LATER_MU: f64 = 7.8;

/// R1452: the later moves' prior, the standard deviation of `ln(think_ms)`.
pub const BOT_THINK_PRIOR_LATER_SIGMA: f64 = 0.7;

/// R639000: a hand-set prior for one action kind (an `ActionType` literal) whatever its place in the
/// turn, ahead of the first and later moves' priors, until `THINK_TIME_FITS` holds a fit for it: the
/// mean and standard deviation of `ln(think_ms)`.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
pub struct ThinkTimePrior {
    pub action_kind: &'static str,
    pub mu: f64,
    pub sigma: f64,
}

/// R639000: the kinds a person takes longer or shorter over than a play: a mulligan (about 9 s,
/// R266), a draw offer's answer and a concession.
pub const BOT_THINK_PRIORS: &[ThinkTimePrior] = &[
    ThinkTimePrior {
        action_kind: "mulligan",
        mu: 9.1,
        sigma: 0.5,
    },
    ThinkTimePrior {
        action_kind: "answerDraw",
        mu: 8.0,
        sigma: 0.6,
    },
    ThinkTimePrior {
        action_kind: "concede",
        mu: 8.3,
        sigma: 0.6,
    },
];

/// R639000: the `ln(legal_count)` the priors are centered on, as a fit's `ln_legal_mean` is.
pub const BOT_THINK_PRIOR_LN_LEGAL_MEAN: f64 = 2.0;

/// R639000: the shortest a bot's aim arrow shows before its targeted play or attack goes. Never
/// under `AIM_RELAY_INTERVAL_MS`, so the relay sends the aim before the move.
pub const BOT_AIM_LEAD_MIN_MS: i64 = 250;

/// R639000: the longest.
pub const BOT_AIM_LEAD_MAX_MS: i64 = 600;

/// R639000: the shortest a bot stays on a finished ranked match (R672), present for a rematch.
pub const BOT_REMATCH_LINGER_MIN_MS: i64 = 4_000;

/// R639000: the longest; always well inside `REMATCH_OFFER_TTL_MS`.
pub const BOT_REMATCH_LINGER_MAX_MS: i64 = 15_000;

/// R639000: the shortest a bot waits before it offers a rematch, or meets a person's offer.
pub const BOT_REMATCH_DELAY_MIN_MS: i64 = 1_500;

/// R639000: the longest; always under `BOT_REMATCH_LINGER_MIN_MS`, so an offer goes inside the linger.
pub const BOT_REMATCH_DELAY_MAX_MS: i64 = 3_500;

/// R641000: the soonest a bot seated in a Conquest series makes its sealed pick after the pick
/// phase opens, as a person reads the matchup first.
pub const BOT_PICK_DELAY_MIN_MS: i64 = 4_000;

/// R641000: the latest; with one more series sweep (`SERIES_SWEEP_INTERVAL_SECONDS`) still well
/// inside `SERIES_PICK_SECONDS`, so a bot never lets the pick clock run out.
pub const BOT_PICK_DELAY_MAX_MS: i64 = 20_000;

/// R641000: the share of a bot's picks that follow its rule (the fastest deck on the play, the
/// slowest on the draw); the rest are uniform over the decks that have not won.
pub const BOT_PICK_RULE_SHARE: f64 = 0.75;

/// R1452: the quickest a bot moves, decision included.
pub const BOT_THINK_MIN_MS: i64 = 600;

/// R1452: the longest it waits before a move.
pub const BOT_THINK_MAX_MS: i64 = 30_000;

/// R1452: what a bot leaves on its acting clock (R79) when it decides or waits, so it never times
/// out.
pub const BOT_CLOCK_MARGIN_MS: i64 = 5_000;

/// R1452: the wall-clock cap on a bot's decision (`should_stop`), past which it plays the best line
/// found so far; shorter when less than this is left on its clock past `BOT_CLOCK_MARGIN_MS`.
pub const BOT_DECISION_DEADLINE_MS: i64 = 4_000;

// ---------------------------------------------------------------------------------------------
// Ladder bots' cosmetics (SPEC §14.2, R1834005, R1834006; #1834). A bot wears a loadout drawn as a
// person's from a wardrobe drawn from the pool's seed (`bots::cosmetics`), so its seat looks
// like a person's (R1452) while it owns nothing (R1454). The fits are pasted by hand from
// `jackioh-server cosmetics-fit` as `THINK_TIME_FITS` is from `timing-fit`; the priors are hand-set
// until a fit replaces them (`jackioh-server cosmetics-fit` prints the rows to paste over `BOT_COSMETIC_FITS`
// in this file). Cosmetic only: none of it reaches the engine, the log or the search.
// ---------------------------------------------------------------------------------------------

/// R1834006: the fewest people's seats an age bucket's row needs before it replaces the prior.
pub const BOT_COSMETIC_FIT_MIN_SAMPLES: usize = 30;
/// R1834005: the fewest people who must own a cosmetic (on that card, for a per-card one) before a
/// bot may: a bot never owns a chase fewer people hold.
pub const BOT_COSMETIC_MIN_OWNERS: i64 = 20;
/// R1834005: the fewest days since a cosmetic was first granted to a person before a bot may own it.
pub const BOT_COSMETIC_MIN_AGE_DAYS: i64 = 14;
/// R1834005: how often a bot's cosmetics age, as an account's do: it may gain wardrobe items and
/// change a slot, at most once in this many days.
pub const BOT_COSMETIC_REDRAW_DAYS: i64 = 7;
/// R1834005, R1834006: the account ages the fits are bucketed by, in days: one bucket up to each
/// bound, inclusive, and one past the last.
pub const BOT_COSMETIC_AGE_BUCKETS_DAYS: [i64; 3] = [7, 30, 90];
/// R1834005: the prior's share of seats that wear a slot's default; the rest is split evenly
/// among the kind's starters.
pub const BOT_COSMETIC_PRIOR_DEFAULT_SHARE: f64 = 0.55;
/// R1834005: the prior's share of seats that pick something of its own for some card of the deck.
pub const BOT_COSMETIC_PRIOR_PER_CARD_SHARE: f64 = 0.2;
/// R1834005: the prior's mean number of a deck's cards picked, for a seat that picks any.
pub const BOT_COSMETIC_PRIOR_CARDS_PER_DECK: f64 = 3.0;
/// R1834005: the prior's share of seats that print a full art they own on a card of their deck.
pub const BOT_COSMETIC_PRIOR_FULL_ART_WEAR: f64 = 0.8;
/// R1834005: the prior's share of bots that own an eligible cosmetic no fitted row lists.
pub const BOT_COSMETIC_PRIOR_OWNERSHIP: f64 = 0.05;
/// R1834005: the chance each slot of a bot changes when its cosmetics age.
pub const BOT_COSMETIC_SLOT_CHANGE_SHARE: f64 = 0.1;

/// R1834006: how many people own a cosmetic, as a share of an age bucket's people: an account
/// cosmetic on the account (`card: None`), a per-card one (a full art among them) on `card`.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
pub struct CosmeticOwnership {
    pub cosmetic: &'static str,
    pub card: Option<&'static str>,
    pub share: f64,
}

/// R1834006: an earned foil, finish or border on the one card a person owns it on: of the seats
/// whose person owns `cosmetic` on `card` and whose deck holds that card, the share whose card wears
/// it.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
pub struct CosmeticWear {
    pub cosmetic: &'static str,
    pub card: &'static str,
    pub share: f64,
}

/// R1834005, R1834006: what people of one account age wear, over `samples` of their seats, for
/// accounts up to `max_age_days` old (`None`: older than every bound of
/// `BOT_COSMETIC_AGE_BUCKETS_DAYS`). `slots` is each cosmetic's share of the seats wearing it in its
/// kind's slot (the six of a seat's loadout, defaults included; a kind it lists nothing of takes the
/// prior's split). `per_card_share` is the share of seats with a pick of their own on some card,
/// `cards_per_deck_mean` the mean number of cards such a seat picks, `card_picks` each default's and
/// starter's (a foil, a finish or a border every card may wear) share of those cards' picks of one,
/// `earned_wear` each earned one's wear on its card (`CosmeticWear`; both empty, a card's pick takes
/// the prior's split), `full_art_wear` each full art's share of the seats that own it and play its
/// card that print it, and `ownership` each non-starter's share of the bucket's people that own it.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
pub struct CosmeticFit {
    pub max_age_days: Option<i64>,
    pub samples: usize,
    pub slots: &'static [(&'static str, f64)],
    pub per_card_share: f64,
    pub cards_per_deck_mean: f64,
    pub card_picks: &'static [(&'static str, f64)],
    pub earned_wear: &'static [CosmeticWear],
    pub full_art_wear: &'static [(&'static str, f64)],
    pub ownership: &'static [CosmeticOwnership],
}

/// R1834006: the cosmetic fits, one per age bucket, pasted by hand from `jackioh-server
/// cosmetics-fit` (CLAUDE.md rule 9). Empty until enough people's matches froze a loadout; a bucket
/// with no row, or one under `BOT_COSMETIC_FIT_MIN_SAMPLES`, draws from `BOT_COSMETIC_PRIOR`.
pub const BOT_COSMETIC_FITS: &[CosmeticFit] = &[];

/// R1834005: the hand-set prior every bucket draws from until a fit replaces it: no shares of its
/// own (each slot `BOT_COSMETIC_PRIOR_DEFAULT_SHARE` default, the rest split among the starters),
/// no picks on cards of its own (a card's pick a kind at even odds, by the slots' split), no full
/// art's wear of its own (`BOT_COSMETIC_PRIOR_FULL_ART_WEAR`) and no ownership of its own
/// (`BOT_COSMETIC_PRIOR_OWNERSHIP`).
pub const BOT_COSMETIC_PRIOR: CosmeticFit = CosmeticFit {
    max_age_days: None,
    samples: 0,
    slots: &[],
    per_card_share: BOT_COSMETIC_PRIOR_PER_CARD_SHARE,
    cards_per_deck_mean: BOT_COSMETIC_PRIOR_CARDS_PER_DECK,
    card_picks: &[],
    earned_wear: &[],
    full_art_wear: &[],
    ownership: &[],
};

/// R645: the four personas a bot may be dealt (TS `PersonaName`). Never shown to the player.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "lowercase")]
pub enum PersonaName {
    Balanced,
    Polite,
    Bm,
    Silent,
}

impl PersonaName {
    /// R645: the name as TS spells it.
    pub fn as_str(self) -> &'static str {
        match self {
            PersonaName::Balanced => "balanced",
            PersonaName::Polite => "polite",
            PersonaName::Bm => "bm",
            PersonaName::Silent => "silent",
        }
    }
}

/// R645: the personas in deal order (TS `PERSONA_ORDER`): the weighted pick walks them in this order.
pub const PERSONA_NAMES: [PersonaName; 4] = [
    PersonaName::Balanced,
    PersonaName::Polite,
    PersonaName::Bm,
    PersonaName::Silent,
];

/// R645: the situations a persona may answer (TS `EmoteTrigger`), in the trigger table's order, which
/// is the order one view step detects them in.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EmoteTrigger {
    /// The mulligan phase just ended.
    MulliganEnd,
    /// The bot's turn started while it is ahead (its health − the player's ≥ `lead_health`, or
    /// `lead_units`+ more units).
    TurnStartAhead,
    /// The bot dealt `big_hit`+ damage to the player's hero in one hit.
    DealtBigHit,
    /// The bot killed the player's highest-attack unit.
    KilledTopUnit,
    /// The bot took `big_hit`+ damage to its hero in one hit.
    TookBigHit,
    /// The bot lost its highest-attack unit.
    LostTopUnit,
    /// The player's turn ran past `long_turn_ms` (once per turn).
    PlayerTurnLong,
    /// The player made a big play: `big_hit`+ damage to the bot's hero, or `kills_for_big_play`+ of
    /// its units in one turn.
    PlayerBigPlay,
    /// The player conceded, or the bot dealt lethal.
    MatchWon,
    /// The bot is about to lose.
    MatchLost,
}

/// R645: every trigger, in the table's order (TS `EMOTE_TRIGGERS`).
pub const EMOTE_TRIGGERS: [EmoteTrigger; 10] = [
    EmoteTrigger::MulliganEnd,
    EmoteTrigger::TurnStartAhead,
    EmoteTrigger::DealtBigHit,
    EmoteTrigger::KilledTopUnit,
    EmoteTrigger::TookBigHit,
    EmoteTrigger::LostTopUnit,
    EmoteTrigger::PlayerTurnLong,
    EmoteTrigger::PlayerBigPlay,
    EmoteTrigger::MatchWon,
    EmoteTrigger::MatchLost,
];

/// R645, R1344: the reply table's rows (TS `EmoteReplyKey`): what a player's emote is answered as,
/// when it has a row at all.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum EmoteReplyKey {
    Greetings,
    Compliment,
    Taunt,
    Apology,
}

/// R645: every reply row, in the table's order (TS `EMOTE_REPLY_KEYS`).
pub const EMOTE_REPLY_KEYS: [EmoteReplyKey; 4] = [
    EmoteReplyKey::Greetings,
    EmoteReplyKey::Compliment,
    EmoteReplyKey::Taunt,
    EmoteReplyKey::Apology,
];

/// R645, R1344: one row of a persona's trigger or reply table: the chance a roll hits, and the pool
/// a hit picks from, §6's own emotes first and verbatim, then the MN03 emoji of the same sense
/// (R1344). A roll picks only from the part of the pool the bot's dealt hand holds.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
pub struct PersonaRow {
    pub chance: f64,
    pub pool: &'static [EmoteId],
}

/// R645: a persona's caps: the most emotes within one of the bot's own turns (`own_turn`) or one of
/// the player's (`other_turn`), and per match (`per_match`, TS `match`), the mulligan greeting and
/// the end-of-match emotes exempted from the last.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PersonaCaps {
    pub own_turn: usize,
    pub other_turn: usize,
    pub per_match: usize,
}

/// R645: one persona's whole config (TS `PersonaSpec`). `triggers` and `replies` hold only the rows
/// the table gives it: an absent row (a dash in the table) sends nothing and draws nothing.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
pub struct PersonaSpec {
    /// The share of bots dealt this persona; the four weights sum to 1.
    pub weight: f64,
    pub caps: PersonaCaps,
    pub triggers: &'static [(EmoteTrigger, PersonaRow)],
    pub replies: &'static [(EmoteReplyKey, PersonaRow)],
}

impl PersonaSpec {
    /// R645: the persona's row for `trigger`, or `None` for a dash.
    pub fn trigger(&self, trigger: EmoteTrigger) -> Option<&PersonaRow> {
        self.triggers
            .iter()
            .find(|(key, _)| *key == trigger)
            .map(|(_, row)| row)
    }

    /// R645: the persona's reply row for `key`, or `None` when it has none.
    pub fn reply(&self, key: EmoteReplyKey) -> Option<&PersonaRow> {
        self.replies
            .iter()
            .find(|(row_key, _)| *row_key == key)
            .map(|(_, row)| row)
    }
}

/// R645, R1344: the persona tables (TS `AI_PERSONAS`), in deal order. Absent rows send nothing;
/// R1344 adds the MN03 emoji after §6's emotes, and Polite never taunts.
pub const AI_PERSONAS: [(PersonaName, PersonaSpec); 4] = [
    (
        PersonaName::Balanced,
        PersonaSpec {
            weight: 0.4,
            caps: PersonaCaps {
                own_turn: 1,
                other_turn: 1,
                per_match: 8,
            },
            triggers: &[
                (
                    EmoteTrigger::MulliganEnd,
                    PersonaRow {
                        chance: 1.0,
                        pool: &[EmoteId::Greetings, EmoteId::Wave, EmoteId::Salute],
                    },
                ),
                (
                    EmoteTrigger::TurnStartAhead,
                    PersonaRow {
                        chance: 0.1,
                        pool: &[EmoteId::Threaten, EmoteId::Laugh, EmoteId::Yawn, EmoteId::Cool],
                    },
                ),
                (
                    EmoteTrigger::DealtBigHit,
                    PersonaRow {
                        chance: 0.35,
                        pool: &[EmoteId::Laugh, EmoteId::Threaten, EmoteId::WahWah, EmoteId::Fire],
                    },
                ),
                (
                    EmoteTrigger::KilledTopUnit,
                    PersonaRow {
                        chance: 0.2,
                        pool: &[EmoteId::Laugh, EmoteId::WahWah, EmoteId::Skull],
                    },
                ),
                (
                    EmoteTrigger::TookBigHit,
                    PersonaRow {
                        chance: 0.25,
                        pool: &[
                            EmoteId::Oops,
                            EmoteId::Sob,
                            EmoteId::Angry,
                            EmoteId::Sweat,
                            EmoteId::Gasp,
                        ],
                    },
                ),
                (
                    EmoteTrigger::LostTopUnit,
                    PersonaRow {
                        chance: 0.15,
                        pool: &[
                            EmoteId::Sob,
                            EmoteId::Angry,
                            EmoteId::Oops,
                            EmoteId::Facepalm,
                            EmoteId::Shrug,
                        ],
                    },
                ),
                (
                    EmoteTrigger::PlayerTurnLong,
                    PersonaRow {
                        chance: 0.2,
                        pool: &[EmoteId::Yawn, EmoteId::Thinking],
                    },
                ),
                (
                    EmoteTrigger::MatchWon,
                    PersonaRow {
                        chance: 0.4,
                        pool: &[
                            EmoteId::WahWah,
                            EmoteId::Laugh,
                            EmoteId::WellPlayed,
                            EmoteId::Party,
                            EmoteId::Salute,
                        ],
                    },
                ),
                (
                    EmoteTrigger::MatchLost,
                    PersonaRow {
                        chance: 1.0,
                        pool: &[EmoteId::WellPlayed, EmoteId::Salute, EmoteId::Clap],
                    },
                ),
            ],
            replies: &[
                (
                    EmoteReplyKey::Greetings,
                    PersonaRow {
                        chance: 0.7,
                        pool: &[EmoteId::Greetings, EmoteId::Wave],
                    },
                ),
                (
                    EmoteReplyKey::Compliment,
                    PersonaRow {
                        chance: 0.3,
                        pool: &[EmoteId::Thanks, EmoteId::ThumbsUp],
                    },
                ),
                (
                    EmoteReplyKey::Taunt,
                    PersonaRow {
                        chance: 0.25,
                        pool: &[
                            EmoteId::Laugh,
                            EmoteId::WahWah,
                            EmoteId::Yawn,
                            EmoteId::Threaten,
                            EmoteId::Cool,
                        ],
                    },
                ),
            ],
        },
    ),
    (
        PersonaName::Polite,
        PersonaSpec {
            weight: 0.25,
            caps: PersonaCaps {
                own_turn: 1,
                other_turn: 1,
                per_match: 6,
            },
            triggers: &[
                (
                    EmoteTrigger::MulliganEnd,
                    PersonaRow {
                        chance: 1.0,
                        pool: &[EmoteId::Greetings, EmoteId::Wave, EmoteId::Salute],
                    },
                ),
                (
                    EmoteTrigger::DealtBigHit,
                    PersonaRow {
                        chance: 0.15,
                        pool: &[EmoteId::Oops, EmoteId::Sweat],
                    },
                ),
                (
                    EmoteTrigger::TookBigHit,
                    PersonaRow {
                        chance: 0.3,
                        pool: &[EmoteId::WellPlayed, EmoteId::Oops, EmoteId::Clap, EmoteId::Gasp],
                    },
                ),
                (
                    EmoteTrigger::LostTopUnit,
                    PersonaRow {
                        chance: 0.2,
                        pool: &[EmoteId::WellPlayed, EmoteId::Clap],
                    },
                ),
                (
                    EmoteTrigger::PlayerBigPlay,
                    PersonaRow {
                        chance: 0.4,
                        pool: &[EmoteId::WellPlayed, EmoteId::Clap, EmoteId::ThumbsUp],
                    },
                ),
                (
                    EmoteTrigger::MatchWon,
                    PersonaRow {
                        chance: 1.0,
                        pool: &[EmoteId::WellPlayed, EmoteId::Salute, EmoteId::Heart],
                    },
                ),
                (
                    EmoteTrigger::MatchLost,
                    PersonaRow {
                        chance: 1.0,
                        pool: &[EmoteId::WellPlayed, EmoteId::Clap, EmoteId::Salute],
                    },
                ),
            ],
            replies: &[
                (
                    EmoteReplyKey::Greetings,
                    PersonaRow {
                        chance: 1.0,
                        pool: &[EmoteId::Greetings, EmoteId::Wave],
                    },
                ),
                (
                    EmoteReplyKey::Compliment,
                    PersonaRow {
                        chance: 0.8,
                        pool: &[EmoteId::Thanks, EmoteId::Heart, EmoteId::ThumbsUp],
                    },
                ),
                (
                    EmoteReplyKey::Taunt,
                    PersonaRow {
                        chance: 0.3,
                        pool: &[EmoteId::Oops, EmoteId::Greetings, EmoteId::Shrug],
                    },
                ),
                (
                    EmoteReplyKey::Apology,
                    PersonaRow {
                        chance: 0.5,
                        pool: &[EmoteId::Thanks, EmoteId::ThumbsUp],
                    },
                ),
            ],
        },
    ),
    (
        PersonaName::Bm,
        PersonaSpec {
            weight: 0.2,
            caps: PersonaCaps {
                own_turn: 2,
                other_turn: 2,
                per_match: 20,
            },
            triggers: &[
                (
                    EmoteTrigger::MulliganEnd,
                    PersonaRow {
                        chance: 1.0,
                        pool: &[EmoteId::Threaten, EmoteId::Laugh, EmoteId::Cool, EmoteId::Skull],
                    },
                ),
                (
                    EmoteTrigger::TurnStartAhead,
                    PersonaRow {
                        chance: 0.5,
                        pool: &[
                            EmoteId::Threaten,
                            EmoteId::Laugh,
                            EmoteId::Yawn,
                            EmoteId::WahWah,
                            EmoteId::Cool,
                        ],
                    },
                ),
                (
                    EmoteTrigger::DealtBigHit,
                    PersonaRow {
                        chance: 0.8,
                        pool: &[
                            EmoteId::Laugh,
                            EmoteId::WahWah,
                            EmoteId::Threaten,
                            EmoteId::Fire,
                            EmoteId::Skull,
                        ],
                    },
                ),
                (
                    EmoteTrigger::KilledTopUnit,
                    PersonaRow {
                        chance: 0.6,
                        pool: &[EmoteId::Laugh, EmoteId::WahWah, EmoteId::Yawn, EmoteId::Skull],
                    },
                ),
                (
                    EmoteTrigger::TookBigHit,
                    PersonaRow {
                        chance: 0.4,
                        pool: &[EmoteId::Angry, EmoteId::Threaten],
                    },
                ),
                (
                    EmoteTrigger::LostTopUnit,
                    PersonaRow {
                        chance: 0.3,
                        pool: &[EmoteId::Angry],
                    },
                ),
                (
                    EmoteTrigger::PlayerTurnLong,
                    PersonaRow {
                        chance: 0.7,
                        pool: &[EmoteId::Yawn],
                    },
                ),
                (
                    EmoteTrigger::MatchWon,
                    PersonaRow {
                        chance: 1.0,
                        pool: &[EmoteId::WahWah, EmoteId::Laugh, EmoteId::Party, EmoteId::Cool],
                    },
                ),
                (
                    EmoteTrigger::MatchLost,
                    PersonaRow {
                        chance: 0.5,
                        pool: &[EmoteId::Sob, EmoteId::Angry],
                    },
                ),
            ],
            replies: &[
                (
                    EmoteReplyKey::Greetings,
                    PersonaRow {
                        chance: 0.6,
                        pool: &[EmoteId::Threaten, EmoteId::Laugh, EmoteId::Cool],
                    },
                ),
                (
                    EmoteReplyKey::Compliment,
                    PersonaRow {
                        chance: 0.4,
                        pool: &[EmoteId::Yawn, EmoteId::Cool],
                    },
                ),
                (
                    EmoteReplyKey::Taunt,
                    PersonaRow {
                        chance: 0.9,
                        pool: &[
                            EmoteId::Laugh,
                            EmoteId::WahWah,
                            EmoteId::Yawn,
                            EmoteId::Threaten,
                            EmoteId::Skull,
                            EmoteId::Fire,
                        ],
                    },
                ),
                (
                    EmoteReplyKey::Apology,
                    PersonaRow {
                        chance: 0.7,
                        pool: &[EmoteId::Laugh, EmoteId::WahWah],
                    },
                ),
            ],
        },
    ),
    (
        PersonaName::Silent,
        PersonaSpec {
            weight: 0.15,
            caps: PersonaCaps {
                own_turn: 0,
                other_turn: 0,
                per_match: 0,
            },
            triggers: &[],
            replies: &[],
        },
    ),
];

/// R645: `name`'s row of `AI_PERSONAS` (every persona has one).
pub fn persona_spec(name: PersonaName) -> &'static PersonaSpec {
    let table: &'static [(PersonaName, PersonaSpec); 4] = &AI_PERSONAS;
    match table.iter().find(|(key, _)| *key == name) {
        Some((_, spec)) => spec,
        None => panic!("persona {} has no table", name.as_str()),
    }
}

/// R645: the deal reads each weight in thousandths (TS `pickPersona`'s 1000), whole-number
/// thresholds that keep a draw of 0.85 in Silent despite floating-point summation.
pub const PERSONA_DEAL_STEPS: f64 = 1000.0;

/// The shape of `AI_EMOTE` (SURFACE §4.2: a constant object becomes a const of a struct named after
/// it). The integer types are the view's and the events' (`HeroView.health`, `GameEvent::Damage`'s
/// `amount`: `i32`; unit counts: `usize`), so the checks compare without casts.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiEmoteConfig {
    pub lead_health: i32,
    pub lead_units: usize,
    pub big_hit: i32,
    pub kills_for_big_play: usize,
    pub long_turn_ms: i64,
    pub delay_min_ms: f64,
    pub delay_max_ms: f64,
}

/// R645: the trigger table's thresholds and timings (TS `AI_EMOTE`): "its health − yours ≥ 10, or 3+
/// more units", "10+ damage to a hero in one hit", "you kill 2+ of its units in one turn", "your turn
/// runs past 45 s (once per turn)", and "a random 0.8–2.5 s delay" every rolled emote waits inside.
pub const AI_EMOTE: AiEmoteConfig = AiEmoteConfig {
    lead_health: 10,
    lead_units: 3,
    big_hit: 10,
    kills_for_big_play: 2,
    long_turn_ms: 45_000,
    delay_min_ms: 800.0,
    delay_max_ms: 2500.0,
};

// Duos co-op's clocks (#3101's second part): SPEC S6, B2, B36, B38 (`host::duos`).

/// S6, B38: how long a round of a team with a person in it runs. On expiry every round seat that has
/// not ended has its prompts answered by the AI and its turn ended (`host::duos`).
pub const DUOS_ROUND_CLOCK_MS: i64 = 90_000;
/// S6: a prompt a person holds outside a round clock of their own (the AI team's round, or after the
/// person ended): on expiry the AI answers that prompt for them.
pub const DUOS_PROMPT_CLOCK_MS: i64 = 30_000;
/// S6, B2: one deadline over the four open mulligans; on expiry the AI answers every one a person
/// still owes.
pub const DUOS_MULLIGAN_CLOCK_MS: i64 = 45_000;
/// S6, B36: how long each AI-played seat waits before each action it owes, so a person can follow
/// the AI team's turn.
pub const DUOS_AI_STEP_DELAY_MS: i64 = 700;

// ---------------------------------------------------------------------------------------------
// The collection economy's rolls (SPEC §14, #1294, #3104): the drop tables, pity, the token bars, the
// cosmetic packs and the set-track curve that `economy` decides with. `crates/server/src/config.rs`
// re-exports every one, so `serverConfig.ts` (`tests/export_config.rs`) and the server's code name them
// as before.
// ---------------------------------------------------------------------------------------------

/// SPEC §11 R633: the longest text value, in characters (a station or an intensity name).
pub const PLAYER_SETTINGS_TEXT_MAX_LENGTH: usize = 40;
/// SPEC §14.3: the name a request gives the global card pack; a set's pack goes by its set's name.
pub const GLOBAL_PACK_NAME: &str = "global";
/// The shape of `CARD_TIER_WEIGHTS` and `COSMETIC_TIER_WEIGHTS` (SPEC §14.3, R1339000): the integer
/// weight out of `TIER_WEIGHT_TOTAL` of each tier above the lowest, so a tier's chance in one roll
/// is its weight over the total. The lowest tier, Common, is the fallback: it takes whatever weight
/// is left, computed and never stored, so raising one tier takes from it alone.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TierWeights {
    pub rare: u32,
    pub epic: u32,
    pub legendary: u32,
    pub mythic: u32,
}

impl TierWeights {
    /// The set tiers' weights summed: what the fallback's weight is `TIER_WEIGHT_TOTAL` less.
    pub const fn set_total(&self) -> u32 {
        self.rare + self.epic + self.legendary + self.mythic
    }
}

/// The shape of `TOKEN_BAR_POINTS_BY_CARD_RARITY` and `TOKEN_BAR_POINTS_BY_COSMETIC_RARITY` (SPEC
/// §14.5, R1953000, R1999000): the token bar points each opened slot adds for each rarity it can
/// roll.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RarityPoints {
    pub common: i64,
    pub rare: i64,
    pub epic: i64,
    pub legendary: i64,
    pub mythic: i64,
}

/// The shape of `PACK_UNOWNED_WEIGHTS` and `COSMETIC_PACK_CARD_WEIGHTS`: how a draw weighs the
/// cards the account owns against the ones it does not (SPEC §14.3, §14.6). A card's weight is
/// its side's number, so an unowned card is drawn `unowned / owned` times as often as an owned
/// one of the same rarity in a card pack, and an owned one `owned / unowned` times as often as an
/// unowned one in a cosmetic pack.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackUnownedWeights {
    pub owned: f64,
    pub unowned: f64,
}

/// One step of `COSMETIC_TOKEN_CYCLE` (SPEC §14.5): a cosmetic token of that rarity, or a
/// cosmetic pack. Serialized lower-camel for the client (`cosmeticPack`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum CosmeticTokenStep {
    Rare,
    Epic,
    Legendary,
    CosmeticPack,
}

impl CosmeticTokenStep {
    /// SPEC §14.5, R2728000: the cosmetic token bar's points this step takes to pay.
    pub const fn bar_size(self) -> i64 {
        match self {
            Self::Rare => COSMETIC_TOKEN_BAR_SIZE_RARE,
            Self::Epic => COSMETIC_TOKEN_BAR_SIZE_EPIC,
            Self::Legendary => COSMETIC_TOKEN_BAR_SIZE_LEGENDARY,
            Self::CosmeticPack => COSMETIC_TOKEN_BAR_SIZE_PACK,
        }
    }
}

/// SPEC §14.3: the slots of a card pack (#1333's table: 5), the Rare-or-better slot one of them.
pub const PACK_SLOTS: usize = 5;
/// SPEC §14.3: how a slot's card draw leans toward the cards the account does not own. #1294's
/// table gives no value; an unowned card weighs four times an owned one of the same rarity, so
/// early packs complete a collection and late packs feed the token bars.
pub const PACK_UNOWNED_WEIGHTS: PackUnownedWeights = PackUnownedWeights {
    owned: 1.0,
    unowned: 4.0,
};
/// SPEC §14.3, R1339000: the fixed total every tier weight table is out of, so a tier's chance in
/// one roll is its weight over this, and one roll is one integer draw below it.
pub const TIER_WEIGHT_TOTAL: u32 = 1000;
/// SPEC §14.3, R1339000: a card pack slot's card rarity (#1339's table): Rare 300, Epic 70,
/// Legendary 20 and Mythic 10, so the fallback, Common, is 600. Per pack before pity, five slots
/// hold an Epic in 1 − (1 − 70/1000)^5 = 30.4% of packs, a Legendary in 9.6% and a Mythic in 4.9%.
pub const CARD_TIER_WEIGHTS: TierWeights = TierWeights {
    rare: 300,
    epic: 70,
    legendary: 20,
    mythic: 10,
};
/// SPEC §14.3, §14.6, R1339000: the cosmetic rarity of a duplicate's cosmetic and of a cosmetic
/// pack slot (#1339's table): Rare 250, Epic 150, Legendary 45 and Mythic 5, so the fallback,
/// Common, is 550.
pub const COSMETIC_TIER_WEIGHTS: TierWeights = TierWeights {
    rare: 250,
    epic: 150,
    legendary: 45,
    mythic: 5,
};
// SPEC §14.3, R1339000: set weights past the total leave the fallback nothing to take.
const _: () = assert!(CARD_TIER_WEIGHTS.set_total() <= TIER_WEIGHT_TOTAL);
const _: () = assert!(COSMETIC_TIER_WEIGHTS.set_total() <= TIER_WEIGHT_TOTAL);
/// SPEC §14.3, R1339000: the card pack slot and the cosmetic pack slot floored at a tier, the
/// first. #1339 says one slot; the first is this issue's pick.
pub const PACK_FLOORED_SLOT: usize = 0;
/// SPEC §14.3, R1339000: the floor of a card pack's floored slot: it rolls `CARD_TIER_WEIGHTS`, and
/// a Common becomes Rare (Rare 900, Epic 70, Legendary 20, Mythic 10).
pub const PACK_FLOOR_RARITY: PrintedRarity = PrintedRarity::Rare;
/// SPEC §14.6, R1339000: the floor of a cosmetic pack's floored slot: it rolls
/// `COSMETIC_TIER_WEIGHTS`, and a Common or a Rare becomes Epic (Epic 950, Legendary 45, Mythic 5).
pub const COSMETIC_PACK_FLOOR_RARITY: CosmeticRarity = CosmeticRarity::Epic;
/// SPEC §14.3, R1661001, R1953000: a duplicate's global cosmetic roll (#1953: 2%, was #1661's 10%):
/// with an account cosmetic left unowned, one draw below `TIER_WEIGHT_TOTAL`, and a draw below this
/// drops an account cosmetic in place of the duplicate's per-card one.
pub const GLOBAL_COSMETIC_ROLL_WEIGHT: u32 = 20;
// SPEC §14.3, R1661001: the hit is a share of the same total, so it cannot pass it.
const _: () = assert!(GLOBAL_COSMETIC_ROLL_WEIGHT <= TIER_WEIGHT_TOTAL);
/// SPEC §14.3, §14.6, R3015003: an out-of-order pull. Where a duplicate's per-card cosmetic or a cosmetic pack
/// slot finds a level past the next one open at its tier, it first draws one integer below `TIER_WEIGHT_TOTAL`;
/// below this (4%) it grants such a level and every level under it in its variant, at or past it the next level
/// as ever. A skip is open far less often than a next level, so this keeps it far rarer than an in-order pull.
pub const LEVEL_SKIP_WEIGHT: u32 = 40;
// R3015003: the skip is a share of the same total, so it cannot pass it.
const _: () = assert!(LEVEL_SKIP_WEIGHT <= TIER_WEIGHT_TOTAL);
/// SPEC §14.3, R3015003: the most out-of-order pulls a seeded simulation of duplicates on one card may count, in
/// percent of the in-order pulls it counts: an out-of-order pull is far rarer than an in-order one.
pub const LEVEL_SKIP_MAX_PERCENT_OF_IN_ORDER: u32 = 10;
/// SPEC §14.2, §14.3, R1661000, R1661001: the rarity every account cosmetic but its kind's default
/// is at least, and the floor of the global roll's tier: it rolls `COSMETIC_TIER_WEIGHTS`, and a
/// Common or a Rare becomes Epic (Epic 950, Legendary 45, Mythic 5).
pub const GLOBAL_COSMETIC_FLOOR_RARITY: CosmeticRarity = CosmeticRarity::Epic;
/// SPEC §14.2, R2725001, R3015000: the fewest cards each level of each variant is offered on, across the shipped
/// sets (the registry's check holds every one to it), and each retired augment's offer.
pub const VERSION_OFFER_MIN_CARDS: usize = 20;
/// SPEC §14.2, R2725001, R3015000: the fewest cards of each shipped set each level of each variant is offered on,
/// so every set has some.
pub const VERSION_OFFER_MIN_PER_SET: usize = 2;
/// SPEC §14.2, R3017001: the fewest variants besides the Default every non-token card is offered, Legacy cards
/// included: #3017's three pairs each offer every card one of their two, so this holds by construction.
pub const VARIANT_MIN_PER_CARD: usize = 3;
/// SPEC §14.2, R2949001: what an offer clause's sample's percent is out of: a card is in the sample when its
/// seeded hash, modulo this, falls below the percent.
pub const OFFER_SAMPLE_PERCENT_SCALE: u32 = 100;
/// SPEC §14.4: the longest an account waits for an Epic: of any 10 card packs in a row, at
/// least one holds an Epic (#1333's table: 10/20/30, and pity never exceeds 30 packs). Pity sits
/// on top of `CARD_TIER_WEIGHTS` and tunes nothing: it only forces a tier on the pack a counter
/// would reach its threshold in (R1339001).
pub const PITY_EPIC_PACKS: usize = 10;
/// SPEC §14.4: the longest an account waits for a Legendary: of any 20 card packs in a row, at
/// least one holds one.
pub const PITY_LEGENDARY_PACKS: usize = 20;
/// SPEC §14.4: the longest an account waits for a Mythic: of any 30 card packs in a row, at
/// least one holds one. The longest wait of all, as SPEC §14.4 requires.
pub const PITY_MYTHIC_PACKS: usize = 30;
/// SPEC §14.5, R1953000, R1999000: the points that fill the card token bar (#1953: 200). On any
/// collection a card pack's slots add about 8.65 points under `CARD_TIER_WEIGHTS` (four slots at
/// 1.61 and the floored one at 2.21, before pity), so the bar fills about once every
/// `CARD_TOKEN_BAR_TARGET_PACKS` packs (`economy/math.rs`). The cosmetic token bar takes a size of
/// its own at each step of its cycle (`CosmeticTokenStep::bar_size`, R2728000).
pub const CARD_TOKEN_BAR_SIZE: i64 = 200;
/// SPEC §14.5, R2728000: the points the cosmetic token bar takes to pay a Rare cosmetic token
/// (#2728: 50), about once every `COSMETIC_TOKEN_BAR_TARGET_PACKS_RARE` packs.
pub const COSMETIC_TOKEN_BAR_SIZE_RARE: i64 = 50;
/// SPEC §14.5, R2728000: the points the cosmetic token bar takes to pay an Epic cosmetic token
/// (#2728: 75), about once every `COSMETIC_TOKEN_BAR_TARGET_PACKS_EPIC` packs.
pub const COSMETIC_TOKEN_BAR_SIZE_EPIC: i64 = 75;
/// SPEC §14.5, R2728000: the points the cosmetic token bar takes to pay a Legendary cosmetic token
/// (#2728: 100), about once every `COSMETIC_TOKEN_BAR_TARGET_PACKS_LEGENDARY` packs.
pub const COSMETIC_TOKEN_BAR_SIZE_LEGENDARY: i64 = 100;
/// SPEC §14.5, R2728000: the points the cosmetic token bar takes to pay a cosmetic pack (#2728:
/// 100), about once every `COSMETIC_TOKEN_BAR_TARGET_PACKS_PACK` packs.
pub const COSMETIC_TOKEN_BAR_SIZE_PACK: i64 = 100;
// SPEC §14.5, R2728000: a step of no points would pay for ever
// (`economy::tokens::settle_cosmetic_bar`).
const _: () = assert!(
    COSMETIC_TOKEN_BAR_SIZE_RARE > 0
        && COSMETIC_TOKEN_BAR_SIZE_EPIC > 0
        && COSMETIC_TOKEN_BAR_SIZE_LEGENDARY > 0
        && COSMETIC_TOKEN_BAR_SIZE_PACK > 0
);
/// SPEC §14.5, R1953000, R1999000: the points each opened slot of a card pack adds to each token
/// bar, whatever it grants, by the card rarity its slot rolled, pity's force included (#1953: 1,
/// 2, 3, 5 and 10).
pub const TOKEN_BAR_POINTS_BY_CARD_RARITY: RarityPoints = RarityPoints {
    common: 1,
    rare: 2,
    epic: 3,
    legendary: 5,
    mythic: 10,
};
/// SPEC §14.5, §14.6, R1306001, R1953000, R1999000: the points each opened slot of a cosmetic pack
/// adds to each token bar, whatever it grants, by the cosmetic rarity it rolled, not the one it
/// fell to (#1953: 1, 2, 3, 5 and 10, as a card's).
pub const TOKEN_BAR_POINTS_BY_COSMETIC_RARITY: RarityPoints = RarityPoints {
    common: 1,
    rare: 2,
    epic: 3,
    legendary: 5,
    mythic: 10,
};
/// SPEC §14.5: the card packs the card token bar takes to fill, on average (#1333's table: ~30;
/// #1953's points and size: 23, `CARD_TOKEN_BAR_SIZE` over the mean points of a pack, rounded,
/// held by `economy/math.rs`).
pub const CARD_TOKEN_BAR_TARGET_PACKS: usize = 23;
/// SPEC §14.5, R2728000: the card packs the cosmetic token bar takes to pay its Rare step, on
/// average (#2728: 6, `COSMETIC_TOKEN_BAR_SIZE_RARE` over the mean points of a pack, rounded, held
/// by `economy/math.rs`).
pub const COSMETIC_TOKEN_BAR_TARGET_PACKS_RARE: usize = 6;
/// SPEC §14.5, R2728000: the card packs the cosmetic token bar takes to pay its Epic step, on
/// average (#2728: 9).
pub const COSMETIC_TOKEN_BAR_TARGET_PACKS_EPIC: usize = 9;
/// SPEC §14.5, R2728000: the card packs the cosmetic token bar takes to pay its Legendary step, on
/// average (#2728: 12).
pub const COSMETIC_TOKEN_BAR_TARGET_PACKS_LEGENDARY: usize = 12;
/// SPEC §14.5, R2728000: the card packs the cosmetic token bar takes to pay its cosmetic pack step,
/// on average (#2728: 12).
pub const COSMETIC_TOKEN_BAR_TARGET_PACKS_PACK: usize = 12;
/// SPEC §14.5, R2728000: the steps the cosmetic token bar pays, in order, verbatim (#2728), each at
/// its own size (`CosmeticTokenStep::bar_size`).
pub const COSMETIC_TOKEN_CYCLE: [CosmeticTokenStep; 6] = [
    CosmeticTokenStep::Rare,
    CosmeticTokenStep::Epic,
    CosmeticTokenStep::Legendary,
    CosmeticTokenStep::Rare,
    CosmeticTokenStep::Epic,
    CosmeticTokenStep::CosmeticPack,
];
/// SPEC §14.6: a cosmetic pack's slots (#1333's table: 5), each rolling `COSMETIC_TIER_WEIGHTS`,
/// the floored one at `COSMETIC_PACK_FLOOR_RARITY` (R1339000).
pub const COSMETIC_PACK_SLOTS: usize = 5;
/// #1306 gives no value; four to one, `PACK_UNOWNED_WEIGHTS` turned around, toward the cards the
/// account owns.
pub const COSMETIC_PACK_CARD_WEIGHTS: PackUnownedWeights = PackUnownedWeights {
    owned: 4.0,
    unowned: 1.0,
};
/// SPEC §14.6: the cosmetic packs that fill the Chase Shard bar (#1333's table: 10). A chase
/// also drops when a duplicate's roll or a cosmetic pack slot lands on Legendary or Mythic
/// (R1333000), so no constant holds a separate chase rate.
pub const CHASE_SHARD_PACKS: i64 = 10;
/// SPEC §14.7: the XP step of the level curve: going from level n to the next costs this times
/// n, uncapped (#1333's table: 100), so level 10 is at 4,500 XP and level 100 at 495,000 XP.
pub const SET_TRACK_XP_PER_LEVEL: i32 = 100;

// ---------------------------------------------------------------------------------------------
// What a finished game pays and how a quest moves (SPEC §14.7 to §14.9, #3104 part 2): the Jlore and XP
// rates, the daily cap, the set tracks' levels and the quest rhythm that `economy::rewards` and
// `economy::quests` decide with. `crates/server/src/config.rs` re-exports every one.
// ---------------------------------------------------------------------------------------------

/// SPEC §14.7: the levels of a set track (#1333's table: 100), whose whole curve costs 495,000 XP
/// (`total_xp_for_level` in `crates/shared/src/economy/math.rs`), a set's lifetime of play.
pub const SET_TRACK_LEVELS: u32 = 100;
/// SPEC §14.7: the newest track's level that unlocks the older ones (#1333's table: 10).
pub const SET_TRACK_UNLOCK_LEVEL: u32 = 10;
/// §14.7: the level a set track starts at, with no XP (migration 0045's `set_tracks.level` default).
pub const SET_TRACK_FIRST_LEVEL: i64 = 1;
/// SPEC §14.15, R3335001: the Fleet track's levels, each paying one Stinkyscape cosmetic, the last the Stinkyscape
/// hull (#3335 proposed 30).
pub const FLEET_TRACK_LEVELS: u32 = 30;
/// SPEC §14.15, R3335001: the XP each Fleet track level costs, the same at every level.
pub const FLEET_TRACK_XP_PER_LEVEL: i32 = 500;
/// SPEC §14.15, R3335001: the Fleet XP a recorded game pays for each Stinkyscape card of the player's deck they
/// played.
pub const FLEET_TRACK_XP_PER_STINKYSCAPE_PLAY: i32 = 10;
/// SPEC §14.15, R3335001: the Fleet XP a recorded game pays for each Starship Piece the player fused.
pub const FLEET_TRACK_XP_PER_PIECE_FUSED: i32 = 15;
/// SPEC §14.15, R3335001: the Fleet XP a recorded game pays for each Starship the player launched.
pub const FLEET_TRACK_XP_PER_LAUNCH: i32 = 50;
/// SPEC §14.15, R3335001: the Fleet XP a recorded game's win pays when the deck held `FLEET_TRACK_DECK_SHARE`
/// Stinkyscape cards or more.
pub const FLEET_TRACK_XP_STINKYSCAPE_WIN: i32 = 50;
/// SPEC §14.15, R3335001: the share of a deck's cards that must be Stinkyscape cards for a win to pay
/// `FLEET_TRACK_XP_STINKYSCAPE_WIN`.
pub const FLEET_TRACK_DECK_SHARE: f64 = 0.5;
/// SPEC §14.15, R3335001: the most Fleet XP the games of one UTC day pay, so the track is a track and not a grind.
pub const FLEET_TRACK_XP_DAILY_MAX: i64 = 1500;
/// SPEC §14.15, R3335001: how many of the Fleet track's next rewards `GET /api/tracks` names.
pub const FLEET_TRACK_NEXT_REWARDS_SHOWN: usize = 3;
/// SPEC §14.8: the Jlore a ladder game pays its winner (#1333's table: 20).
pub const JLORE_LADDER_WIN: i32 = 20;
/// SPEC §14.8: the Jlore a ladder game pays its loser (#1333's table: 10), so losing still moves
/// the wallet.
pub const JLORE_LADDER_LOSS: i32 = 10;
/// SPEC §14.8: the Jlore a practice game pays for a win (#1333's table: 10), since the browser
/// runs it and the server only checks the log.
pub const JLORE_PRACTICE_WIN: i32 = 10;
/// SPEC §14.8: the Jlore a practice game pays for a loss (#1333's table: 5).
pub const JLORE_PRACTICE_LOSS: i32 = 5;
/// SPEC §14.8, R1502000: the Jlore an unranked game pays its winner, at the ladder's rate.
pub const JLORE_UNRANKED_WIN: i32 = 20;
/// SPEC §14.8, R1502000: the Jlore an unranked game pays its loser, at the ladder's rate.
pub const JLORE_UNRANKED_LOSS: i32 = 10;
/// SPEC §14.8, R1502000: the Jlore a game of a Conquest series pays its winner, as its own
/// result, at the ladder's rate.
pub const JLORE_CONQUEST_WIN: i32 = 20;
/// SPEC §14.8, R1502000: the Jlore a game of a Conquest series pays its loser, as its own result,
/// at the ladder's rate.
pub const JLORE_CONQUEST_LOSS: i32 = 10;
/// SPEC §14.8, R1502000: the Jlore a co-op game pays each person of a winning team, at practice's
/// rate, since the team plays the AI.
pub const JLORE_COOP_WIN: i32 = 10;
/// SPEC §14.8, R1502000: the Jlore a co-op game pays each person of a team that did not win, at
/// practice's rate.
pub const JLORE_COOP_LOSS: i32 = 5;
/// SPEC §14.8, R1483000, R1502000: the most Jlore an account's games pay it in one UTC day, every
/// mode's games together, as Hearthstone caps the gold its games pay (quest and track Jlore are not
/// counted and not stopped). The owner's number (#1483): well above a real day's play, far below
/// what a loop of conceded practice games would earn.
pub const JLORE_GAMES_DAILY_CAP: i64 = 2000;
/// SPEC §14.8: the XP of a game per action the player took, a proxy for time played. #1333's
/// table leaves it to measured action counts: over R1333001's games 20, 600 and 2,000 average
/// 999.75 XP a ladder game, `XP_LADDER_TARGET_MEAN`'s 1,000.
pub const XP_PER_ACTION: i32 = 20;
/// SPEC §14.8: the XP more a win pays, the big bonus #1333's table asks for (R1333001).
pub const XP_WIN_BONUS: i32 = 600;
/// SPEC §14.8: the most XP a game pays, so a loop of empty actions cannot farm it: twice
/// `XP_LADDER_TARGET_MEAN` (R1333001).
pub const XP_GAME_CAP: i32 = 2000;
/// SPEC §14.8: a practice game's share of the XP the same game earns on the ladder (#1333's
/// table: 0.5).
pub const XP_PRACTICE_SCALE: f64 = 0.5;
/// SPEC §14.8, R1502000: a co-op game's share of the XP the same moves earn on the ladder, as
/// practice's (the team plays the AI).
pub const XP_COOP_SCALE: f64 = 0.5;
/// SPEC §14.8: the XP a ladder game averages (#1333's table: 1,000), over R1333001's measured
/// games.
pub const XP_LADDER_TARGET_MEAN: i32 = 1000;
/// SPEC §14.9: how often a quest arrives while fewer than `QUEST_MAX_HELD` are held, in hours
/// (#1333's table: 5).
pub const QUEST_INTERVAL_HOURS: i64 = 5;
/// SPEC §14.9: how many quests an account holds at most (#1333's table: 3), so a player away for
/// a day comes back to three waiting, never more.
pub const QUEST_MAX_HELD: usize = 3;
/// SPEC §14.9: the least Jlore a quest pays (#1333's table: 50 to 100), half a pack to a pack.
pub const QUEST_JLORE_MIN: i32 = 50;
/// SPEC §14.9: the most Jlore a quest pays, for the hardest goals (#1333's table: 100).
pub const QUEST_JLORE_MAX: i32 = 100;
/// SPEC §14.9: the XP of each Jlore a quest pays (#1333's table: 50), so a quest's XP weighs
/// like a few games'.
pub const QUEST_XP_PER_JLORE: i32 = 50;
/// SPEC §14.9, R2210000: how many held quests an account may replace in one UTC day, each for a
/// quest drawn as an arrival is (Hearthstone's daily reroll). Exported to serverConfig.ts.
pub const QUEST_REPLACES_PER_DAY: i64 = 1;

// ---------------------------------------------------------------------------------------------
// The practice session's numbers (#3099): what its host bounds a move and a hint by, and how many
// finished games it keeps. `crates/wasm/core`'s `constants` carries them, which the web's practice
// worker and a Roblox server read, so none restates one.
// ---------------------------------------------------------------------------------------------

/// Wall-clock safety cap for one decision of the practice AI (`AiOptions.should_stop`): 3 s since the
/// budget doubled (R2575000). The machine puts it in the `decide` ask (`AiAsk`), and a host
/// with a clock holds the search to it: the node budget stays the real bound (R2560000).
pub const PRACTICE_AI_CLOCK_MS: i64 = 3000;
/// R2102000: wall-clock cap for the lethal hint's check, which every snapshot of the human's turn waits
/// for: a check still running past it gives no hint, so a board whose lines are slow to play out never
/// holds a click's answer back for long. Put in the `lethal` ask (`LethalAsk`).
pub const PRACTICE_LETHAL_HINT_CLOCK_MS: i64 = 300;
/// R768: the finished free games a device keeps for replay, the newest of them.
pub const PRACTICE_REPLAYS_KEPT: u32 = 10;
