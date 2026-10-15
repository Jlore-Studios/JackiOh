//! Every number the Luau host states (CLAUDE.md rule 9).

/// #442 L5: how many interrupts one hook call may take before it is stopped, naming its card. Luau
/// interrupts about once per loop iteration and three times per function call, so a card's hook,
/// which runs a few loops over the board, never comes near it, and a loop that never ends stops
/// within a few milliseconds.
pub const LUAU_HOOK_INTERRUPTS: u32 = 1_000_000;

/// R915004 (K7): a custom card's hook call's budget, in the same interrupts. A tenth of an official
/// script's: the kit's verbs and readers spend a few hundred, a question's re-run a few thousand, so
/// a card that does what the kit offers never nears it, and one that loops stops within a millisecond
/// or two, doing nothing, while a turn's AI search runs its hooks thousands of times.
pub const CUSTOM_HOOK_INTERRUPTS: u32 = 100_000;

/// #442 L5: how deep hook calls may nest (a hook running another card's hook from a reader) before
/// the innermost is stopped, naming its card. A reader nests a level or two; a hook that runs itself
/// would otherwise go on until the thread's stack overflowed, which aborts the process, well before
/// it spent `LUAU_HOOK_INTERRUPTS`.
pub const LUAU_HOOK_DEPTH: usize = 16;

/// #843: how many bytes a thread's VM may hold, all told: its libraries, its cache of loaded chunks,
/// what the calls in flight hold and the garbage not yet collected. An allocation that would pass it
/// fails, and so does the hook call that asked, naming its card, so a call that builds one huge
/// string or table without looping (`string.rep`), which spends no interrupts, is bounded too. A
/// sandbox holding 370 loaded chunks of a two-reader fixture peaked at about 2.5 MB over 200,000
/// module runs (measured for #843), so a card's hook never comes near it.
pub const LUAU_MEMORY_BYTES: usize = 64 * 1024 * 1024;

/// #843: how many bytes the VM may hold when an outermost hook call begins before the call first
/// collects all its garbage. Luau collects nothing when an allocation fails, so the garbage of a call
/// that ran out of memory would fail the next one too. Half `LUAU_MEMORY_BYTES`, which a VM serving
/// cards' hooks never comes near, so the collection runs only after a call that took that much.
pub const LUAU_MEMORY_COLLECT_BYTES: usize = LUAU_MEMORY_BYTES / 2;

/// #442 L5: the optimization level `compile` asks of Luau's compiler: mlua's default, stated so that
/// a bump of `mlua` cannot change it. Level 2 folds `math`'s constants (`math.pi`, `math.huge`, …)
/// into the bytecode without reading the global the sandbox removed, and no disabled builtin turns
/// that off.
pub const LUAU_OPTIMIZATION_LEVEL: u8 = 1;

/// #442 L6: how deep a value from Luau may nest before the walk refuses it. A table that holds
/// itself would otherwise never end; an effect's arguments nest a few levels at most.
pub const LUAU_VALUE_DEPTH: usize = 32;

/// #442 L6: how many values and keys, all told, one value from Luau may hold before the walk refuses
/// it. Under `LUAU_VALUE_DEPTH` a table can still hold another twice over at every level, which
/// would walk to billions of values; a hook's effects hold a few dozen.
pub const LUAU_VALUE_NODES: usize = 10_000;

/// #464 gap 7: the engine config constants card scripts read as `J.config.<NAME>`, from the same
/// source as `engineConfig.ts` (`crates/engine/src/config.rs`), so a script never restates one
/// (CLAUDE.md rule 9). The rule for membership: every `i32` constant of the engine's `config.rs`
/// that a card script names. A script needing one not listed here adds it to this list — never a
/// literal — and both the runtime table (`api::module`) and the typed module (`tests/export_api`)
/// read this list, so the two cannot drift.
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
