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
//!   to each other.

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
        CRAFT_MAX_EFFECTS, CRAFT_MAX_N, CRAFT_NOUNS, CRAFT_RETIRED_VERBS, CRAFT_VERB_PRICES, CUSTOM_ATTACK_MAX,
        CUSTOM_BUILDER_MAX_BYTES, CUSTOM_COST_MAX, CUSTOM_HEALTH_MAX, CUSTOM_ID_PREFIX, CUSTOM_KEYWORDS,
        CUSTOM_KEYWORDS_MAX, CUSTOM_NAME_MAX_LENGTH, CUSTOM_PARAM_KEY_MAX_LENGTH, CUSTOM_PARAM_MAX,
        CUSTOM_PARAMS_MAX, CUSTOM_RADIANT_STAT_MAX, CUSTOM_SOURCE_MAX_BYTES, CUSTOM_TEXT_MAX_LENGTH, DECK_SIZE,
        DIFFICULTIES, DRAWS_PER_TURN, DUOS_HUMAN_TEAM, DUOS_OWN_HEALTH, DUOS_PASS_CAP, DUOS_PASS_COST,
        DUOS_POOL_HEALTH, DUOS_POOL_HEALTH_HARD, EARLIER_DECK_SIZE, EMOTE_COOLDOWN_MS, EMOTE_HAND_SIZE,
        EMOTE_HAND_VOICE, EMOTE_WINDOW_MAX, EMOTE_WINDOW_MS, GLITCH_DEF_ID, HERO_HEALTH, HUMAN_HANDICAP,
        KY_COMPETITION_SECONDS, LIBRARY_CAP, LOADOUT_DECKS, LOSER_DRAWS_REASONS, MAX_COPIES, MAX_MANA,
        PROMPT_CARD_OPTION_LIMIT, RANDOM_ATTACK_TARGET, REPLAY_CHECKPOINT_EVERY, REPLAY_PAGE_STEPS, ROLL_HEADS,
        ROLL_TAILS, SANDBOX_ARMOR_MAX, SANDBOX_DRAW_MAX, SANDBOX_HERO_HEALTH_MAX, SANDBOX_MANA_MAX, TRIBAL_TAGS,
        TURN_CAP_PLAYER_TURNS, TUTORIAL_BARRED_RARITIES, TUTORIAL_CARD_SET, TUTORIAL_HUMAN, VERSIONS_SCHEME,
        VOTE_TALLY_SIZE,
    );
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
        let expected: BTreeSet<&str> = lines
            .iter()
            .map(|(name, _)| name.as_str())
            .chain(BEYOND_ENGINE_CONFIG)
            .collect();
        assert_eq!(
            keys, expected,
            "the object's keys are engineConfig.ts's, plus the AI's and the run's"
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
