//! `constants`: every number a client reads, as one JSON object, so a host takes them from the module
//! and restates none (CLAUDE.md rule 9).
//!
//! The object is flat. Its keys are:
//!
//! - the AI's five, `AI_BUDGET`, `AI_GATE_BUDGET`, `SHADOW_BAN_IDS`, `FIXTURE_BANNED_IDS` and
//!   `AI_DECK`, which `apps/web/src/wasm/index.ts`'s `aiConstants()` reads;
//! - `RUN`, the dungeon run's numbers (`jackioh_run::run_constants`, as `run_constants` answers);
//! - each constant of `apps/web/src/wire/engineConfig.ts` under its own name, so `DECK_SIZE` is
//!   `constants.DECK_SIZE` as it is in the web. The names are the list that
//!   `crates/engine/tests/export_config.rs` writes that file from; this module's test holds the two
//!   to each other;
//! - each constant of `jackioh_shared::config` (the shared logic's, `crates/shared/README.md`'s host
//!   contract, #3098) under its own name, `SERIES_WINS_NEEDED` and the economy's drop tables and token bars
//!   (#3104) among them. A later shared module's
//!   numbers join them here, and this module's test holds the list to that file.

use serde::Serialize;
use serde_json::{Map, Value};

use crate::to_json;

/// Adds one key, refusing a name given twice.
fn put<T: Serialize + ?Sized>(all: &mut Map<String, Value>, name: &str, value: &T) -> Result<(), String> {
    let value = serde_json::to_value(value).map_err(|error| format!("{name} does not serialize: {error}"))?;
    if all.insert(name.to_owned(), value).is_some() {
        return Err(format!("{name} is given twice"));
    }
    Ok(())
}

/// Adds each named constant of `jackioh_engine::config` under its Rust name.
macro_rules! put_engine {
    ($all:ident; $($name:ident),+ $(,)?) => {
        $( put(&mut $all, stringify!($name), &jackioh_engine::config::$name)?; )+
    };
}

/// Calls `$callback` with `$args` and then the name of every `pub const` of `jackioh_shared::config`, in
/// the order that file holds them: the one list `constants` and its test both read, which the test holds
/// to the file.
macro_rules! shared_constants {
    ($callback:ident ($($args:tt)*)) => {
        $callback!(
            $($args)*
            MS_PER_SECOND, SERIES_WINS_NEEDED, SERIES_MAX_GAMES, SERIES_PICK_SECONDS, TEMPORARY_LAST_COUNT,
            GLITCH_SEED, GLITCH_SWAP_IN, GLITCH_MARK_IN, GLITCH_ODDS_SCALE, TURN_CLOCK_SECONDS,
            PROMPT_CLOCK_SECONDS, MULLIGAN_CLOCK_SECONDS, DISCONNECT_GRACE_SECONDS, MATCH_CEILING_MINUTES,
            MATCH_SERVER_ACTION_RETRY_MS, BOT_WRITE_RETRY_MS, AIM_RELAY_INTERVAL_MS, REMATCH_OFFER_TTL_MS,
            STAKE_NORMAL, STAKE_DOUBLE, RATING_START, RATING_DEVIATION_START, RATING_VOLATILITY_START,
            GLICKO_TAU, GLICKO_SCALE, GLICKO_CONVERGENCE, GLICKO_MAX_ITERATIONS, RANK_TIER_PERCENTS,
            RANK_DIVISIONS_PER_TIER, RANK_PIPS_PER_DIVISION, RANK_PLACEMENT_GAMES, RANK_WIN_PIPS,
            RANK_LOSS_PIPS, RANK_STREAK_LENGTH, RANK_STREAK_BONUS_PIPS, RANK_CONVERGENCE_GAP_PIPS,
            RANK_CONVERGENCE_PIPS, JLORIOUS_SIZE, SEASON_RESET_STRENGTH, SEASON_RESET_DEVIATION_BOOST,
            BOT_PROMOTION_DEVIATION_BOOST, SORT_KEY_GRAPE_BASE, SORT_KEY_JLORIOUS_BASE, SORT_KEY_RATING_BITS,
            MS_PER_DAY, MAX_FAIR_GAP, CARD_STATS_MIN_SAMPLE, THINK_TIME_FITS, THINK_COMPLEXITY_SLOPE,
            THINK_LETHAL_SHIFT, BOT_CONCEDE_FIT, BOT_DRAW_ANSWER_FIT, BOT_DRAW_OFFER, BOT_TIMEOUT_FIT,
            BOT_REMATCH_FIT, BOT_EMOTE_DELAY_FIT, BOT_EMOTE_REPLY_DELAY_FIT, BOT_PERSONA_WEIGHTS,
            BOT_SEED_RATINGS, BOT_MID_STRENGTH_BUDGET, BOT_STRENGTHS, BOT_CALIBRATION_GENERATION,
            SKILL_ELO_ANCHOR, SKILL_ELO_TABLE, BOT_POOL_MIN, BOT_POOL_MAX, BOT_POOL_PER_WEEKLY_ACTIVE,
            BOT_POOL_ACTIVE_WINDOW_DAYS, BOT_GRIND_REFERENCE_RANK, BOT_GRIND_GAP, BOT_GRIND_HYSTERESIS,
            BOT_GRINDERS, BOT_GRIND_ACTIVE_HOURS, BOT_VS_BOT_MIN_GAP_SECONDS, BOT_FALLBACK_MIN_SECONDS,
            BOT_FALLBACK_MAX_SECONDS, BOT_RECENT_OPPONENTS, BOT_ACTIVE_HOURS_MIN, BOT_ACTIVE_HOURS_MAX,
            BOT_HOURS_PER_DAY, BOT_SPEED_MIN, BOT_SPEED_MAX, BOT_DECKS_MIN, BOT_DECKS_MAX,
            BOT_DECK_DRAW_ATTEMPTS, BOT_ACCOUNT_AGE_MIN_DAYS, BOT_ACCOUNT_AGE_MAX_DAYS,
            BOT_DEFAULT_USERNAME_SHARE, BOT_NAME_SHARE_PLAIN, BOT_NAME_SHARE_NONSENSE, BOT_NAME_SHARE_GAMER,
            BOT_NAME_SHARE_EDGY, BOT_NAME_SHARE_ANIME, BOT_NAME_SHARE_CHINESE_HAN,
            BOT_NAME_SHARE_CHINESE_PINYIN, BOT_NAME_SHARE_GIVEN, BOT_NAME_SHARE_CRUDE,
            BOT_NAME_SHARE_OTHER_REFS, BOT_NAME_SHARE_POLITICS, BOT_NAME_LUIS_SHARE, BOT_NAME_DRAW_ATTEMPTS,
            BOT_NAME_TWO_DIGIT_MIN, BOT_NAME_TWO_DIGIT_MAX, BOT_NAME_BIRTH_YEAR_MIN, BOT_NAME_BIRTH_YEAR_MAX,
            BOT_NAME_STRETCH_MIN, BOT_NAME_STRETCH_MAX, BOT_NAME_SYLLABLES_MIN, BOT_NAME_SYLLABLES_MAX,
            BOT_NAME_YEAR_SHARE, BOT_NAME_FILLER_NUMBER_SHARE, BOT_NAME_CHINESE_YEAR_SHARE,
            BOT_NAME_COMPOUND_LETTERS, BOT_NAME_FILLER_NUMBERS, BOT_NAME_CHINESE_NUMBERS, BOT_NAME_SHAPES_PLAIN,
            BOT_NAME_SHAPES_NONSENSE, BOT_NAME_SHAPES_GAMER, BOT_NAME_SHAPES_EDGY, BOT_NAME_SHAPES_REFERENCE,
            BOT_NAME_SHAPES_CRUDE, BOT_NAME_SHAPES_POLITICS, BOT_NAME_SHAPES_GIVEN, BOT_NAME_SHAPES_CHINESE_HAN,
            BOT_NAME_SHAPES_CHINESE_PINYIN, BOT_THINK_PRIOR_FIRST_MU, BOT_THINK_PRIOR_FIRST_SIGMA,
            BOT_THINK_PRIOR_LATER_MU, BOT_THINK_PRIOR_LATER_SIGMA, BOT_THINK_PRIORS,
            BOT_THINK_PRIOR_LN_LEGAL_MEAN, BOT_AIM_LEAD_MIN_MS, BOT_AIM_LEAD_MAX_MS, BOT_REMATCH_LINGER_MIN_MS,
            BOT_REMATCH_LINGER_MAX_MS, BOT_REMATCH_DELAY_MIN_MS, BOT_REMATCH_DELAY_MAX_MS,
            BOT_PICK_DELAY_MIN_MS, BOT_PICK_DELAY_MAX_MS, BOT_PICK_RULE_SHARE, BOT_THINK_MIN_MS,
            BOT_THINK_MAX_MS, BOT_CLOCK_MARGIN_MS, BOT_DECISION_DEADLINE_MS, BOT_COSMETIC_FIT_MIN_SAMPLES,
            BOT_COSMETIC_MIN_OWNERS, BOT_COSMETIC_MIN_AGE_DAYS, BOT_COSMETIC_REDRAW_DAYS,
            BOT_COSMETIC_AGE_BUCKETS_DAYS, BOT_COSMETIC_PRIOR_DEFAULT_SHARE, BOT_COSMETIC_PRIOR_PER_CARD_SHARE,
            BOT_COSMETIC_PRIOR_CARDS_PER_DECK, BOT_COSMETIC_PRIOR_FULL_ART_WEAR, BOT_COSMETIC_PRIOR_OWNERSHIP,
            BOT_COSMETIC_SLOT_CHANGE_SHARE, BOT_COSMETIC_FITS, BOT_COSMETIC_PRIOR, PERSONA_NAMES, EMOTE_TRIGGERS,
            EMOTE_REPLY_KEYS, AI_PERSONAS, PERSONA_DEAL_STEPS, AI_EMOTE, DUOS_ROUND_CLOCK_MS,
            DUOS_PROMPT_CLOCK_MS, DUOS_MULLIGAN_CLOCK_MS, DUOS_AI_STEP_DELAY_MS,
            PLAYER_SETTINGS_TEXT_MAX_LENGTH, GLOBAL_PACK_NAME, PACK_SLOTS, PACK_UNOWNED_WEIGHTS,
            TIER_WEIGHT_TOTAL, CARD_TIER_WEIGHTS, COSMETIC_TIER_WEIGHTS, PACK_FLOORED_SLOT, PACK_FLOOR_RARITY,
            COSMETIC_PACK_FLOOR_RARITY, GLOBAL_COSMETIC_ROLL_WEIGHT, LEVEL_SKIP_WEIGHT,
            LEVEL_SKIP_MAX_PERCENT_OF_IN_ORDER, GLOBAL_COSMETIC_FLOOR_RARITY, VERSION_OFFER_MIN_CARDS,
            VERSION_OFFER_MIN_PER_SET, VARIANT_MIN_PER_CARD, OFFER_SAMPLE_PERCENT_SCALE, PITY_EPIC_PACKS,
            PITY_LEGENDARY_PACKS, PITY_MYTHIC_PACKS, CARD_TOKEN_BAR_SIZE, COSMETIC_TOKEN_BAR_SIZE_RARE,
            COSMETIC_TOKEN_BAR_SIZE_EPIC, COSMETIC_TOKEN_BAR_SIZE_LEGENDARY, COSMETIC_TOKEN_BAR_SIZE_PACK,
            TOKEN_BAR_POINTS_BY_CARD_RARITY, TOKEN_BAR_POINTS_BY_COSMETIC_RARITY, CARD_TOKEN_BAR_TARGET_PACKS,
            COSMETIC_TOKEN_BAR_TARGET_PACKS_RARE, COSMETIC_TOKEN_BAR_TARGET_PACKS_EPIC,
            COSMETIC_TOKEN_BAR_TARGET_PACKS_LEGENDARY, COSMETIC_TOKEN_BAR_TARGET_PACKS_PACK,
            COSMETIC_TOKEN_CYCLE, COSMETIC_PACK_SLOTS, COSMETIC_PACK_CARD_WEIGHTS, CHASE_SHARD_PACKS,
            SET_TRACK_XP_PER_LEVEL,
            SET_TRACK_LEVELS, SET_TRACK_UNLOCK_LEVEL, SET_TRACK_FIRST_LEVEL, FLEET_TRACK_LEVELS,
            FLEET_TRACK_XP_PER_LEVEL, FLEET_TRACK_XP_PER_STINKYSCAPE_PLAY, FLEET_TRACK_XP_PER_PIECE_FUSED,
            FLEET_TRACK_XP_PER_LAUNCH, FLEET_TRACK_XP_STINKYSCAPE_WIN, FLEET_TRACK_DECK_SHARE,
            FLEET_TRACK_XP_DAILY_MAX, FLEET_TRACK_NEXT_REWARDS_SHOWN, JLORE_LADDER_WIN,
            JLORE_LADDER_LOSS, JLORE_PRACTICE_WIN, JLORE_PRACTICE_LOSS, JLORE_UNRANKED_WIN,
            JLORE_UNRANKED_LOSS, JLORE_CONQUEST_WIN, JLORE_CONQUEST_LOSS, JLORE_COOP_WIN, JLORE_COOP_LOSS,
            JLORE_GAMES_DAILY_CAP, XP_PER_ACTION, XP_WIN_BONUS, XP_GAME_CAP, XP_PRACTICE_SCALE,
            XP_COOP_SCALE, XP_LADDER_TARGET_MEAN, QUEST_INTERVAL_HOURS, QUEST_MAX_HELD, QUEST_JLORE_MIN,
            QUEST_JLORE_MAX, QUEST_XP_PER_JLORE, QUEST_REPLACES_PER_DAY,
            PRACTICE_AI_CLOCK_MS, PRACTICE_LETHAL_HINT_CLOCK_MS, PRACTICE_REPLAYS_KEPT,
        )
    };
}

/// Adds each named constant of `jackioh_shared::config` under its Rust name.
macro_rules! put_shared {
    ($all:ident; $($name:ident),+ $(,)?) => {
        $( put(&mut $all, stringify!($name), &jackioh_shared::config::$name)?; )+
    };
}

/// `{ AI_BUDGET, AI_GATE_BUDGET, SHADOW_BAN_IDS, FIXTURE_BANNED_IDS, AI_DECK, RUN, …engineConfig.ts }`:
/// every rules, AI and run number a client reads, for practice, the tutorial harness, the "More cards
/// from the newest set" toggle (`AI_DECK.leanMinShare`, R1370), the screens and their tests.
/// `FIXTURE_BANNED_IDS` is the frozen fixture pool (issue #1113), `SHADOW_BAN_IDS` as it stood when the
/// fixtures were dealt.
pub fn constants() -> Result<String, String> {
    let mut all = Map::new();
    put(&mut all, "AI_BUDGET", &jackioh_ai::AI_BUDGET)?;
    put(&mut all, "AI_GATE_BUDGET", &jackioh_ai::AI_GATE_BUDGET)?;
    put(&mut all, "SHADOW_BAN_IDS", &jackioh_ai::SHADOW_BAN_IDS)?;
    put(&mut all, "FIXTURE_BANNED_IDS", &jackioh_ai::FIXTURE_BANNED_IDS)?;
    put(&mut all, "AI_DECK", &jackioh_ai::AI_DECK)?;
    put(&mut all, "RUN", &jackioh_run::run_constants())?;
    put_engine!(
        all;
        AI_DIFFICULTY, AI_TUTORIAL, COIN_DEF_ID, CRAFT_ADJECTIVES, CRAFT_HAT_PRICES, CRAFT_KEYWORD_PRICES,
        CRAFT_MAX_EFFECTS, CRAFT_MAX_N, CRAFT_NOUNS, CRAFT_RETIRED_VERBS, CRAFT_VERB_PRICES, DECK_SIZE,
        DIFFICULTIES, DRAWS_PER_TURN, DUOS_HUMAN_TEAM, DUOS_OWN_HEALTH, DUOS_PASS_CAP, DUOS_PASS_COST,
        DUOS_POOL_HEALTH, DUOS_POOL_HEALTH_HARD, EARLIER_DECK_SIZE, EMOTE_COOLDOWN_MS, EMOTE_HAND_SIZE,
        EMOTE_HAND_VOICE, EMOTE_WINDOW_MAX, EMOTE_WINDOW_MS, GLITCH_DEF_ID, HERO_HEALTH, HUMAN_HANDICAP,
        KY_COMPETITION_SECONDS, LIBRARY_CAP, LOADOUT_DECKS, LOSER_DRAWS_REASONS, MAX_COPIES, MAX_MANA,
        PROMPT_CARD_OPTION_LIMIT, RANDOM_ATTACK_TARGET, REPLAY_CHECKPOINT_EVERY, REPLAY_PAGE_STEPS, ROLL_HEADS,
        ROLL_TAILS, SANDBOX_ARMOR_MAX, SANDBOX_DRAW_MAX, SANDBOX_HERO_HEALTH_MAX, SANDBOX_MANA_MAX, STARSHIP_NAMES,
        TRIBAL_TAGS, TURN_CAP_PLAYER_TURNS, TUTORIAL_BARRED_RARITIES, TUTORIAL_CARD_SET, TUTORIAL_HUMAN,
        VERSIONS_SCHEME, VOTE_TALLY_SIZE,
    );
    // R1420, R3114000: the sets that ship, read from the set table, which `engineConfig.ts` holds
    // beside the config constants.
    put(&mut all, "SHIPPED_SETS", &jackioh_engine::SHIPPED_SETS)?;
    shared_constants!(put_shared(all;));
    to_json(&Value::Object(all))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    use super::*;

    /// The AI and run keys, which `engineConfig.ts` does not hold.
    const BEYOND_ENGINE_CONFIG: [&str; 6] = [
        "AI_BUDGET",
        "AI_GATE_BUDGET",
        "SHADOW_BAN_IDS",
        "FIXTURE_BANNED_IDS",
        "AI_DECK",
        "RUN",
    ];

    fn object() -> Map<String, Value> {
        match serde_json::from_str(&constants().expect("constants")).expect("constants is JSON") {
            Value::Object(all) => all,
            other => panic!("constants is not an object: {other}"),
        }
    }

    /// Each `export const NAME[: type] = JSON;` line of the generated file: the name and the JSON.
    fn engine_config_lines() -> Vec<(String, Value)> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../apps/web/src/wire/engineConfig.ts");
        let text =
            std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        text.lines()
            .filter_map(|line| line.strip_prefix("export const "))
            .map(|rest| {
                let name = rest.split([':', ' ']).next().expect("a name").to_owned();
                let (_, json) = rest
                    .split_once(" = ")
                    .unwrap_or_else(|| panic!("no value on: {rest}"));
                let json = json
                    .strip_suffix(';')
                    .unwrap_or_else(|| panic!("no `;` on: {rest}"));
                let value = serde_json::from_str(json).unwrap_or_else(|error| panic!("{name}: {error}"));
                (name, value)
            })
            .collect()
    }

    /// The name of each `pub const` of `crates/shared/src/config.rs`, read off the source.
    fn shared_config_names() -> Vec<String> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../shared/src/config.rs");
        let text =
            std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        text.lines()
            .filter_map(|line| line.strip_prefix("pub const "))
            .map(|rest| rest.split(':').next().expect("a name").trim().to_owned())
            .collect()
    }

    #[test]
    fn constants_hold_every_engine_config_constant() {
        let lines = engine_config_lines();
        assert!(
            lines.len() >= 50,
            "engineConfig.ts scanned to {} constants",
            lines.len()
        );
        let all = object();
        for (name, value) in &lines {
            assert_eq!(all.get(name), Some(value), "{name}");
        }
        let keys: BTreeSet<&str> = all.keys().map(String::as_str).collect();
        let shared = shared_config_names();
        let expected: BTreeSet<&str> = lines
            .iter()
            .map(|(name, _)| name.as_str())
            .chain(BEYOND_ENGINE_CONFIG)
            .chain(shared.iter().map(String::as_str))
            .collect();
        assert_eq!(
            keys, expected,
            "the object's keys are engineConfig.ts's, plus the AI's, the run's and the shared logic's"
        );
    }

    /// The shared logic's numbers (#3098, #3101), by the names its `config.rs` and the server's hold them
    /// under.
    #[test]
    fn constants_carry_the_shared_logics_numbers() {
        let all = object();
        let names = shared_config_names();
        let mut expected = Vec::new();
        macro_rules! shared_numbers {
            ($($name:ident),+ $(,)?) => {
                $(
                    expected.push(stringify!($name).to_owned());
                    assert_eq!(
                        all.get(stringify!($name)),
                        Some(&serde_json::to_value(jackioh_shared::config::$name).expect("serializes")),
                        stringify!($name)
                    );
                )+
            };
        }
        shared_constants!(shared_numbers());
        assert_eq!(
            names, expected,
            "config.rs gained or lost a constant: put it in `constants` (put_shared!) and here"
        );
    }

    #[test]
    fn constants_carry_the_ai_and_run_numbers() {
        let all = object();
        for (name, value) in [
            ("AI_BUDGET", serde_json::to_value(jackioh_ai::AI_BUDGET)),
            ("AI_GATE_BUDGET", serde_json::to_value(jackioh_ai::AI_GATE_BUDGET)),
            ("SHADOW_BAN_IDS", serde_json::to_value(jackioh_ai::SHADOW_BAN_IDS)),
            (
                "FIXTURE_BANNED_IDS",
                serde_json::to_value(jackioh_ai::FIXTURE_BANNED_IDS),
            ),
            ("AI_DECK", serde_json::to_value(jackioh_ai::AI_DECK)),
            ("RUN", serde_json::to_value(jackioh_run::run_constants())),
        ] {
            assert_eq!(all.get(name), Some(&value.expect("serializes")), "{name}");
        }
    }
}
