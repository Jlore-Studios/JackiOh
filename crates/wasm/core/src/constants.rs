//! `constants`: every number a client reads, as one JSON object, so a host states none of them
//! (CLAUDE.md rule 9). The AI's and the dungeon run's come from their crates; the engine's are the
//! 66 constants of `apps/web/src/wire/engineConfig.ts`, under the names the web imports them by.

use serde::Serialize;
use serde_json::{Map, Value};

use crate::to_json;

/// Add `name`: `value` as JSON to `all`; two numbers under one name is a bug here, not a last write.
fn put<T: Serialize + ?Sized>(all: &mut Map<String, Value>, name: &str, value: &T) -> Result<(), String> {
    let value = serde_json::to_value(value).map_err(|error| format!("constants: {name}: {error}"))?;
    match all.insert(name.to_owned(), value) {
        None => Ok(()),
        Some(_) => Err(format!("constants: {name} is named twice")),
    }
}

/// Add each named `jackioh_engine::config` constant to `all` under its own name, so the list names
/// each once and a key cannot differ from the Rust constant it holds.
macro_rules! put_engine {
    ($all:ident; $($name:ident),+ $(,)?) => {
        $( put(&mut $all, stringify!($name), &jackioh_engine::config::$name)?; )+
    };
}

/// One flat object: `{ AI_BUDGET, AI_GATE_BUDGET, SHADOW_BAN_IDS, FIXTURE_BANNED_IDS, AI_DECK, RUN, … }`.
///
/// - The AI's, for practice, the tutorial harness, the "More cards from the newest set" toggle
///   (`AI_DECK.leanMinShare`, R1370) and their tests. `FIXTURE_BANNED_IDS` is the frozen fixture pool
///   (issue #1113), `SHADOW_BAN_IDS` as it stood when the fixtures were dealt.
/// - `RUN`: the dungeon run's numbers the screens show, as `run_constants()` answers them.
/// - The rules' (`crates/engine/src/config.rs`): the constants of `apps/web/src/wire/engineConfig.ts`
///   by name (`DECK_SIZE`, `HERO_HEALTH`, `AI_DIFFICULTY`, …), so a host reads `constants.DECK_SIZE` as
///   the web imports it. `crates/engine/tests/export_config.rs` writes that file from the same list;
///   this module's test holds the two together.
///
/// The server's numbers (`serverConfig.ts`) are not here.
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

    use super::*;

    /// The generated file the web imports the engine's numbers from.
    const ENGINE_CONFIG_TS: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../apps/web/src/wire/engineConfig.ts"
    );

    /// The AI's and the run's keys, which `engineConfig.ts` does not hold.
    const NOT_ENGINE: [&str; 6] = [
        "AI_BUDGET",
        "AI_GATE_BUDGET",
        "SHADOW_BAN_IDS",
        "FIXTURE_BANNED_IDS",
        "AI_DECK",
        "RUN",
    ];

    fn answer() -> Map<String, Value> {
        match serde_json::from_str(&constants().expect("constants answers")).expect("constants is JSON") {
            Value::Object(all) => all,
            other => panic!("constants is an object, not {other}"),
        }
    }

    /// Each `export const NAME[: type] = JSON;` line of `engineConfig.ts`: the name, and the value parsed.
    fn engine_config() -> Vec<(String, Value)> {
        let text = std::fs::read_to_string(ENGINE_CONFIG_TS)
            .unwrap_or_else(|error| panic!("cannot read {ENGINE_CONFIG_TS}: {error}"));
        text.lines()
            .filter_map(|line| line.strip_prefix("export const "))
            .map(|rest| {
                let name = rest.split([':', ' ']).next().unwrap_or_default();
                let (_, json) = rest
                    .split_once(" = ")
                    .unwrap_or_else(|| panic!("{rest}: no ` = `"));
                let json = json
                    .strip_suffix(';')
                    .unwrap_or_else(|| panic!("{name}: no closing `;`"));
                let value = serde_json::from_str(json).unwrap_or_else(|error| panic!("{name}: {error}"));
                (name.to_owned(), value)
            })
            .collect()
    }

    // A rules number added to `export_config.rs` and not here (or the reverse) is a number a host would
    // lack, or one the web would not: both files name the same constants, each with the same value.
    #[test]
    fn constants_hold_every_engine_config_constant() {
        let all = answer();
        let expected = engine_config();
        assert!(
            expected.len() >= 50,
            "the scan of engineConfig.ts found {} names",
            expected.len()
        );
        for (name, value) in &expected {
            assert_eq!(all.get(name), Some(value), "{name}");
        }
        let names: BTreeSet<&str> = expected
            .iter()
            .map(|(name, _)| name.as_str())
            .chain(NOT_ENGINE)
            .collect();
        let keys: BTreeSet<&str> = all.keys().map(String::as_str).collect();
        assert_eq!(
            keys, names,
            "constants() names exactly engineConfig.ts's constants and the AI's and run's"
        );
    }

    #[test]
    fn constants_carry_the_ai_and_run_numbers() {
        let all = answer();
        for (name, expected) in [
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
            assert_eq!(all.get(name), Some(&expected.unwrap()), "{name}");
        }
    }

    #[test]
    fn put_refuses_a_name_given_twice() {
        let mut all = Map::new();
        put(&mut all, "N", &1).unwrap();
        assert!(put(&mut all, "N", &2).unwrap_err().contains("named twice"));
    }
}
