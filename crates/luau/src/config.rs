//! Every number the Luau host states (CLAUDE.md rule 9).

/// #442 L5: how many interrupts one hook call may take before it is stopped.
pub const LUAU_HOOK_INTERRUPTS: u32 = 1_000_000;

/// R915004 (K7): custom card hook call interrupt budget.
pub const CUSTOM_HOOK_INTERRUPTS: u32 = 100_000;

/// #442 L5: how deep hook calls may nest before the innermost is stopped.
pub const LUAU_HOOK_DEPTH: usize = 16;

/// #843: maximum bytes a thread's VM may hold.
pub const LUAU_MEMORY_BYTES: usize = 64 * 1024 * 1024;

/// #843: threshold to collect garbage before an outermost hook call begins.
pub const LUAU_MEMORY_COLLECT_BYTES: usize = LUAU_MEMORY_BYTES / 2;

/// #442 L5: optimization level for Luau's compiler.
pub const LUAU_OPTIMIZATION_LEVEL: u8 = 1;

/// #442 L6: maximum nesting depth for values returned from Luau.
pub const LUAU_VALUE_DEPTH: usize = 32;

/// #442 L6: maximum node count across all keys and values in one return table.
pub const LUAU_VALUE_NODES: usize = 10_000;

/// #464 gap 7: engine config constants exposed as `J.config.<NAME>` (CLAUDE.md rule 9).
pub const CONFIG_I32: &[(&str, i32)] = &[
    (
        "ACTIVATE_UNLIMITED_CAP",
        jackioh_engine::config::ACTIVATE_UNLIMITED_CAP,
    ),
    ("ARMOR_UP_ARMOR", jackioh_engine::config::ARMOR_UP_ARMOR),
    ("BLADE_STORM_ROUNDS", jackioh_engine::config::BLADE_STORM_ROUNDS),
    ("BOARD_HISTORY_DEPTH", jackioh_engine::config::BOARD_HISTORY_DEPTH),
    ("BRAINSTORM_DISCOUNT", jackioh_engine::config::BRAINSTORM_DISCOUNT),
    (
        "CALL_TO_CHAOS_CHAIN_CAP",
        jackioh_engine::config::CALL_TO_CHAOS_CHAIN_CAP,
    ),
    (
        "CALL_TO_CHAOS_ETERNAL_CAP",
        jackioh_engine::config::CALL_TO_CHAOS_ETERNAL_CAP,
    ),
    (
        "CAST_ON_DRAW_CHAIN_CAP",
        jackioh_engine::config::CAST_ON_DRAW_CHAIN_CAP,
    ),
    (
        "CHAIN_OF_THOUGHT_REPEATS",
        jackioh_engine::config::CHAIN_OF_THOUGHT_REPEATS,
    ),
    ("CHAOS_MED_ACCLAIMED", jackioh_engine::config::CHAOS_MED_ACCLAIMED),
    ("CHAOS_MED_CN_CARDS", jackioh_engine::config::CHAOS_MED_CN_CARDS),
    ("CHAOS_MED_COST", jackioh_engine::config::CHAOS_MED_COST),
    (
        "CHAOS_MED_FUSE_COPIES",
        jackioh_engine::config::CHAOS_MED_FUSE_COPIES,
    ),
    ("CHAOS_MED_HEAL", jackioh_engine::config::CHAOS_MED_HEAL),
    (
        "CHAOS_MED_HERO_ARMOR",
        jackioh_engine::config::CHAOS_MED_HERO_ARMOR,
    ),
    ("CHAOS_MED_NERFS", jackioh_engine::config::CHAOS_MED_NERFS),
    (
        "CHAOS_MED_PRIME_CARDS",
        jackioh_engine::config::CHAOS_MED_PRIME_CARDS,
    ),
    ("CRAFTED_CARD_COST", jackioh_engine::config::CRAFTED_CARD_COST),
    ("CRAFT_LOC_BUDGET", jackioh_engine::config::CRAFT_LOC_BUDGET),
    ("CRAFT_MAX_COST", jackioh_engine::config::CRAFT_MAX_COST),
    (
        "CRAFT_RADIANT_MULTIPLIER",
        jackioh_engine::config::CRAFT_RADIANT_MULTIPLIER,
    ),
    ("DECK_SIZE", jackioh_engine::config::DECK_SIZE),
    ("DIE_INSECT_DAMAGE", jackioh_engine::config::DIE_INSECT_DAMAGE),
    ("FENG_SHUI_BRITTLE", jackioh_engine::config::FENG_SHUI_BRITTLE),
    ("FUSE_COST_CAP", jackioh_engine::config::FUSE_COST_CAP),
    (
        "GLITCH_ODDS_DENOMINATOR",
        jackioh_engine::config::GLITCH_ODDS_DENOMINATOR,
    ),
    (
        "GLITCH_ODDS_PER_SYSTEM_PLAY",
        jackioh_engine::config::GLITCH_ODDS_PER_SYSTEM_PLAY,
    ),
    ("GRADE_A_DAMAGE", jackioh_engine::config::GRADE_A_DAMAGE),
    ("HAND_CAP", jackioh_engine::config::HAND_CAP),
    ("HAND_CAP_MAX", jackioh_engine::config::HAND_CAP_MAX),
    ("HERO_HEALTH", jackioh_engine::config::HERO_HEALTH),
    (
        "KY_TEST_MIN_PROBLEMS",
        jackioh_engine::config::KY_TEST_MIN_PROBLEMS,
    ),
    ("KY_TEST_OPTIONS", jackioh_engine::config::KY_TEST_OPTIONS),
    ("LIBRARY_CAP", jackioh_engine::config::LIBRARY_CAP),
    ("LIFE_TAP_DAMAGE", jackioh_engine::config::LIFE_TAP_DAMAGE),
    ("LIFE_TAP_DRAW", jackioh_engine::config::LIFE_TAP_DRAW),
    ("MAX_MANA", jackioh_engine::config::MAX_MANA),
    ("MIN_CHOSEN_X", jackioh_engine::config::MIN_CHOSEN_X),
    (
        "NIGHT_MARKET_AI_LOTS",
        jackioh_engine::config::NIGHT_MARKET_AI_LOTS,
    ),
    (
        "NIGHT_MARKET_CN_LOTS",
        jackioh_engine::config::NIGHT_MARKET_CN_LOTS,
    ),
    (
        "NIGHT_MARKET_ROCK_LOTS",
        jackioh_engine::config::NIGHT_MARKET_ROCK_LOTS,
    ),
    ("PING_DAMAGE", jackioh_engine::config::PING_DAMAGE),
    ("PLUCK_COST", jackioh_engine::config::PLUCK_COST),
    (
        "RADIANT_SHEEP_TRIBUTE_VALUE",
        jackioh_engine::config::RADIANT_SHEEP_TRIBUTE_VALUE,
    ),
    ("ROLLBACK_MAX_TURNS", jackioh_engine::config::ROLLBACK_MAX_TURNS),
    ("SHEEP_TRIBUTE_VALUE", jackioh_engine::config::SHEEP_TRIBUTE_VALUE),
    ("STITCHING_MAX_COST", jackioh_engine::config::STITCHING_MAX_COST),
    ("TANK_UP_ARMOR", jackioh_engine::config::TANK_UP_ARMOR),
    ("TUNE_STAT_TOTAL", jackioh_engine::config::TUNE_STAT_TOTAL),
    (
        "TURN_CAP_PLAYER_TURNS",
        jackioh_engine::config::TURN_CAP_PLAYER_TURNS,
    ),
    ("UNIT_ZONES", jackioh_engine::config::UNIT_ZONES),
    (
        "VOLATILITY_RADIANT_BUFF_MULTIPLIER",
        jackioh_engine::config::VOLATILITY_RADIANT_BUFF_MULTIPLIER,
    ),
    (
        "VOLATILITY_TUNE_MULTIPLIER",
        jackioh_engine::config::VOLATILITY_TUNE_MULTIPLIER,
    ),
];
