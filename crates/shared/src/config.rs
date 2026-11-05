//! The shared logic's numbers (CLAUDE.md rule 9): the server re-exports the ones its own code and
//! `serverConfig.ts` name (`crates/server/src/config.rs`), and the hosts that call this crate through
//! `jackioh-wasm-core` read them from its `constants` export, so none restates one.

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
/// SPEC §14.2, §14.3, R1661000, R1661001: the rarity every account cosmetic but its kind's default
/// is at least, and the floor of the global roll's tier: it rolls `COSMETIC_TIER_WEIGHTS`, and a
/// Common or a Rare becomes Epic (Epic 950, Legendary 45, Mythic 5).
pub const GLOBAL_COSMETIC_FLOOR_RARITY: CosmeticRarity = CosmeticRarity::Epic;
/// SPEC §14.2, R2725001: the fewest cards each version and each augment is offered on, across the shipped
/// sets (the registry's check holds every one to it).
pub const VERSION_OFFER_MIN_CARDS: usize = 20;
/// SPEC §14.2, R2725001: the fewest cards of each shipped set each version and each augment is offered on, so
/// every set has some.
pub const VERSION_OFFER_MIN_PER_SET: usize = 2;
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
