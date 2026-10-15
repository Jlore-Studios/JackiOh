//! Custom cards in one game (#915, SPEC §12, R915000–R915009).
//!
//! - **The id space** (R915000): `custom-` and a tail of lowercase letters, digits and hyphens. No
//!   catalog id starts with it (`cargo jackioh catalog check`), so a custom card never shadows a
//!   catalog card or a token.
//! - **The frame** (R915001): `validate_custom_card` is §12.2's every limit, the one check the
//!   server, the practice worker, the creator and `create_game` share; `custom_def` turns a valid
//!   frame into the `CardDef` the game reads, of set `Custom`, which never ships, so no pool, Discover
//!   or random deck finds one.
//! - **The game's copy** (R915006): `create_game` keeps each custom card (frame, version, source; the
//!   builder's model dropped) in `GameState.custom_cards` and its definition in
//!   `GameState.custom_defs`, so `hash_state` covers them and a fold with the same input folds the
//!   same game. `catalog::find_def` reads `custom_defs` after `transient_defs`.
//! - **The scripts** (R915002): only a host that runs Luau can build a custom card's scripts, and the
//!   engine depends on none. `with_custom_scripts` lends the engine such a host for the length of one
//!   call: `scripts::scripts_ref` and `face_ref` ask it for a custom card's scripts, by the card the
//!   state holds. With no host lent, a custom card has no script, as a vanilla card has none.
//!
//! CLAUDE.md rule 4: the host is held in a thread-local `Cell` for exactly the call
//! `with_custom_scripts` wraps, and put back as it was when the call returns or unwinds. It holds no
//! game state: what it answers is a function of the custom card the state hands it, which the state
//! carries, so a reduce under it is still a pure function of its inputs, on any thread. Like
//! `jackioh-luau`'s VM it caches only compiled scripts, keyed by the card's id and source.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use indexmap::IndexSet;

use crate::config::{
    CUSTOM_ATTACK_MAX, CUSTOM_BUILDER_MAX_BYTES, CUSTOM_COST_MAX, CUSTOM_HEALTH_MAX, CUSTOM_ID_MAX_LENGTH,
    CUSTOM_ID_PREFIX, CUSTOM_KEYWORDS, CUSTOM_KEYWORDS_MAX, CUSTOM_NAME_MAX_LENGTH,
    CUSTOM_PARAM_KEY_MAX_LENGTH, CUSTOM_PARAM_MAX, CUSTOM_PARAMS_MAX, CUSTOM_RADIANT_STAT_MAX,
    CUSTOM_SOURCE_MAX_BYTES, CUSTOM_TEXT_MAX_LENGTH,
};
use crate::script::CardScripts;
use crate::state::GameState;
use crate::wire::{
    CardCost, CardDef, CardFace, CardType, CustomCard, CustomFrame, Keyword, KeywordKind, Param, ParamBetter,
    PrintedRarity, Rarity, SetName,
};

/// R915000: whether `id` is in the custom id space (it begins `custom-`). Well-formed or not: an id
/// with the prefix is never a catalog id.
pub fn is_custom_id(id: &str) -> bool {
    id.starts_with(CUSTOM_ID_PREFIX)
}

/// R915000: whether `id` is a well-formed custom id: the prefix, then 1 or more lowercase ASCII
/// letters, digits and hyphens, `CUSTOM_ID_MAX_LENGTH` characters in all at most.
pub fn custom_id_well_formed(id: &str) -> bool {
    let Some(tail) = id.strip_prefix(CUSTOM_ID_PREFIX) else {
        return false;
    };
    !tail.is_empty()
        && id.len() <= CUSTOM_ID_MAX_LENGTH
        && tail
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// R915001: one way a custom card breaks §12.2: the field it is about and a sentence for the player.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CustomIssue {
    /// `id`, `version`, `name`, `type`, `cost`, `rarity`, `attack`, `health`, `radiantAttack`,
    /// `radiantHealth`, `keywords`, `text`, `params`, `source` or `builder`.
    pub field: String,
    pub message: String,
}

fn issue(field: &str, message: String) -> CustomIssue {
    CustomIssue {
        field: field.to_string(),
        message,
    }
}

/// A string's length in characters (Unicode scalar values), as the creator counts it.
fn chars(text: &str) -> usize {
    text.chars().count()
}

fn param_key_well_formed(key: &str) -> bool {
    let mut bytes = key.bytes();
    matches!(bytes.next(), Some(first) if first.is_ascii_lowercase())
        && key.len() <= CUSTOM_PARAM_KEY_MAX_LENGTH
        && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// The `{key}`s a text names, in order, each once: a plural form `{key|card|cards}` names `key`.
fn text_keys(text: &str) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            break;
        };
        let key = after[..close].split('|').next().unwrap_or_default();
        if !keys.iter().any(|known| known == key) {
            keys.push(key.to_string());
        }
        rest = &after[close + 1..];
    }
    keys
}

fn in_range(field: &str, what: &str, value: i32, min: i32, max: i32) -> Option<CustomIssue> {
    if value < min || value > max {
        Some(issue(field, format!("{what} must be from {min} to {max}")))
    } else {
        None
    }
}

/// R915001: every way `card` breaks §12.2, in field order; empty when it is a valid custom card.
pub fn validate_custom_card(card: &CustomCard) -> Vec<CustomIssue> {
    let mut issues: Vec<CustomIssue> = Vec::new();
    if !custom_id_well_formed(&card.id) {
        issues.push(issue(
            "id",
            format!(
                "a custom card's id is \"{CUSTOM_ID_PREFIX}\" and up to {} lowercase letters, digits and hyphens",
                CUSTOM_ID_MAX_LENGTH - CUSTOM_ID_PREFIX.len()
            ),
        ));
    }
    if card.version < 1 {
        issues.push(issue(
            "version",
            "a custom card's version starts at 1".to_string(),
        ));
    }
    issues.extend(validate_custom_frame(&card.frame));
    if card.source.len() > CUSTOM_SOURCE_MAX_BYTES {
        issues.push(issue(
            "source",
            format!("the script is longer than {CUSTOM_SOURCE_MAX_BYTES} bytes"),
        ));
    }
    if let Some(builder) = &card.builder {
        let size = serde_json::to_string(builder).map_or(usize::MAX, |text| text.len());
        if size > CUSTOM_BUILDER_MAX_BYTES {
            issues.push(issue(
                "builder",
                format!("the builder's model is longer than {CUSTOM_BUILDER_MAX_BYTES} bytes"),
            ));
        }
    }
    issues
}

/// R915001: every way a frame breaks §12.2.
pub fn validate_custom_frame(frame: &CustomFrame) -> Vec<CustomIssue> {
    let mut issues: Vec<CustomIssue> = Vec::new();
    let name_length = chars(&frame.name);
    if frame.name.trim().is_empty() || name_length > CUSTOM_NAME_MAX_LENGTH {
        issues.push(issue(
            "name",
            format!("a name is 1 to {CUSTOM_NAME_MAX_LENGTH} characters, not only spaces"),
        ));
    }
    let unit = frame.type_ == CardType::Unit;
    if !unit && frame.type_ != CardType::Spell {
        issues.push(issue("type", "a custom card is a Unit or a Spell".to_string()));
    }
    issues.extend(in_range("cost", "the cost", frame.cost, 0, CUSTOM_COST_MAX));
    if unit {
        match frame.attack {
            Some(attack) => issues.extend(in_range("attack", "the attack", attack, 0, CUSTOM_ATTACK_MAX)),
            None => issues.push(issue("attack", "a Unit has an attack".to_string())),
        }
        match frame.health {
            Some(health) => issues.extend(in_range("health", "the health", health, 1, CUSTOM_HEALTH_MAX)),
            None => issues.push(issue("health", "a Unit has a health".to_string())),
        }
        if let Some(attack) = frame.radiant_attack {
            issues.extend(in_range(
                "radiantAttack",
                "the Radiant attack",
                attack,
                0,
                CUSTOM_RADIANT_STAT_MAX,
            ));
        }
        if let Some(health) = frame.radiant_health {
            issues.extend(in_range(
                "radiantHealth",
                "the Radiant health",
                health,
                1,
                CUSTOM_RADIANT_STAT_MAX,
            ));
        }
        if frame.keywords.len() > CUSTOM_KEYWORDS_MAX {
            issues.push(issue(
                "keywords",
                format!("a custom card has at most {CUSTOM_KEYWORDS_MAX} keywords"),
            ));
        }
        for (at, kind) in frame.keywords.iter().enumerate() {
            if !CUSTOM_KEYWORDS.contains(kind) {
                issues.push(issue(
                    "keywords",
                    format!("{} is not a keyword a custom card may have", kind.as_str()),
                ));
            } else if frame.keywords[..at].contains(kind) {
                issues.push(issue("keywords", format!("{} is listed twice", kind.as_str())));
            }
        }
    } else {
        let stats = [
            frame.attack,
            frame.health,
            frame.radiant_attack,
            frame.radiant_health,
        ];
        if stats.iter().any(Option::is_some) {
            issues.push(issue("attack", "a Spell has no attack or health".to_string()));
        }
        if !frame.keywords.is_empty() {
            issues.push(issue("keywords", "a Spell has no keywords".to_string()));
        }
    }
    if chars(&frame.text) > CUSTOM_TEXT_MAX_LENGTH {
        issues.push(issue(
            "text",
            format!("the text is longer than {CUSTOM_TEXT_MAX_LENGTH} characters"),
        ));
    }
    if frame.params.len() > CUSTOM_PARAMS_MAX {
        issues.push(issue(
            "params",
            format!("a custom card has at most {CUSTOM_PARAMS_MAX} numbers"),
        ));
    }
    for (at, param) in frame.params.iter().enumerate() {
        if !param_key_well_formed(&param.key) {
            issues.push(issue(
                "params",
                format!(
                    "\"{}\" is not a number's name: 1 to {CUSTOM_PARAM_KEY_MAX_LENGTH} lowercase letters, digits and underscores, starting with a letter",
                    param.key
                ),
            ));
        } else if frame.params[..at].iter().any(|other| other.key == param.key) {
            issues.push(issue(
                "params",
                format!("the number {{{}}} is listed twice", param.key),
            ));
        }
        for (face, value) in [("base", param.base), ("Radiant", param.radiant)] {
            if !(0..=CUSTOM_PARAM_MAX).contains(&value) {
                issues.push(issue(
                    "params",
                    format!(
                        "the number {{{}}}'s {face} value must be from 0 to {CUSTOM_PARAM_MAX}",
                        param.key
                    ),
                ));
            }
        }
    }
    for key in text_keys(&frame.text) {
        if !frame.params.iter().any(|param| param.key == key) {
            issues.push(issue(
                "text",
                format!("the text names {{{key}}}, which is none of the card's numbers"),
            ));
        }
    }
    issues
}

/// The `Keyword` a custom card's keyword kind is (`CUSTOM_KEYWORDS` takes none with a number).
fn keyword_of(kind: KeywordKind) -> Option<Keyword> {
    Some(match kind {
        KeywordKind::Taunt => Keyword::Taunt,
        KeywordKind::Rush => Keyword::Rush,
        KeywordKind::Charge => Keyword::Charge,
        KeywordKind::FirstStrike => Keyword::FirstStrike,
        KeywordKind::Poisonous => Keyword::Poisonous,
        KeywordKind::Lifesteal => Keyword::Lifesteal,
        KeywordKind::Reborn => Keyword::Reborn,
        KeywordKind::DivineShield => Keyword::DivineShield,
        KeywordKind::Trample => Keyword::Trample,
        KeywordKind::Windfury => Keyword::Windfury,
        KeywordKind::Ranged => Keyword::Ranged,
        KeywordKind::Flying => Keyword::Flying,
        _ => return None,
    })
}

/// The rarity a printed rarity is.
fn rarity_of(printed: PrintedRarity) -> Rarity {
    match printed {
        PrintedRarity::Common => Rarity::Common,
        PrintedRarity::Rare => Rarity::Rare,
        PrintedRarity::Epic => Rarity::Epic,
        PrintedRarity::Legendary => Rarity::Legendary,
        PrintedRarity::Mythic => Rarity::Mythic,
    }
}

/// R915001: a Radiant stat: the frame's, or twice the base one when the frame gives none.
fn radiant_stat(radiant: Option<i32>, base: Option<i32>) -> Option<i32> {
    radiant.or_else(|| base.map(|value| value.saturating_mul(2)))
}

/// R915000: the definition a custom card's frame builds. Total: it never panics, so a frame the
/// validator would refuse still builds one (a game never deals one: `create_game` validates first).
pub fn custom_def(card: &CustomCard) -> CardDef {
    let frame = &card.frame;
    let unit = frame.type_ == CardType::Unit;
    let face = |radiant: bool| CardFace {
        type_: None,
        attack: if unit {
            if radiant {
                radiant_stat(frame.radiant_attack, frame.attack)
            } else {
                frame.attack
            }
        } else {
            None
        },
        health: if unit {
            if radiant {
                radiant_stat(frame.radiant_health, frame.health)
            } else {
                frame.health
            }
        } else {
            None
        },
        x_stats: None,
        keywords: if unit {
            frame
                .keywords
                .iter()
                .filter_map(|kind| keyword_of(*kind))
                .collect()
        } else {
            Vec::new()
        },
        text: frame.text.clone(),
        grants: None,
    };
    let params: Vec<Param> = frame
        .params
        .iter()
        .map(|param| Param {
            key: param.key.clone(),
            base: param.base,
            radiant: param.radiant,
            better: ParamBetter::Up,
            step: None,
            min: Some(0),
            max: None,
            tuned_on: None,
            power: None,
        })
        .collect();
    CardDef {
        id: card.id.clone(),
        index: card.id.clone(),
        name: frame.name.clone(),
        set: SetName::Custom,
        type_: frame.type_,
        tags: Vec::new(),
        rarity: rarity_of(frame.rarity),
        printed_rarity: None,
        token: false,
        cost: CardCost::Fixed(frame.cost),
        refs: None,
        params: if params.is_empty() { None } else { Some(params) },
        loc: None,
        radiant_fallback: None,
        ingredients: None,
        craft: None,
        base: face(false),
        radiant: face(true),
    }
}

/// R915006: the copy of a custom card a game keeps: everything but the builder's model, which only
/// the creator reads.
pub fn custom_game_copy(card: &CustomCard) -> CustomCard {
    CustomCard {
        builder: None,
        ..card.clone()
    }
}

/// R915005, R915006: why a game's custom cards cannot be dealt with these decks, if they cannot:
/// each card valid (R915001) and named once, and every custom id a deck holds one of them.
pub fn check_custom_setup(cards: &[CustomCard], decks: &(Vec<String>, Vec<String>)) -> Result<(), String> {
    let mut seen: IndexSet<&str> = IndexSet::new();
    for card in cards {
        if let Some(first) = validate_custom_card(card).first() {
            return Err(format!(
                "the custom card \"{}\" ({}): {}",
                card.frame.name, card.id, first.message
            ));
        }
        if !seen.insert(card.id.as_str()) {
            return Err(format!("the custom card {} is given twice", card.id));
        }
    }
    for id in decks.0.iter().chain(&decks.1) {
        if is_custom_id(id) && !seen.contains(id.as_str()) {
            return Err(format!(
                "the deck names the custom card {id}, which the game was not given"
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// The scoped script host (R915002)
// ---------------------------------------------------------------------------------------------

/// A host that builds a custom card's scripts from its source: `jackioh-luau`'s card kit, in the
/// browser's Luau module, or a test's fixture. It answers the same scripts for the same card (id,
/// version, source), so a game folds the same whatever host it runs under.
pub trait CustomScriptHost {
    /// `card`'s scripts. A card whose source does not load answers scripts that fail as §12.5 says
    /// (every hook does nothing and says so), never `None`: `None` is a host that has no script for
    /// the card at all, which plays it vanilla.
    fn scripts(&self, card: &CustomCard) -> Option<Arc<CardScripts>>;
}

thread_local! {
    // CLAUDE.md rule 4, as this module's header says: lent for one call, never game state.
    static HOST: Cell<Option<Rc<dyn CustomScriptHost>>> = const { Cell::new(None) };
}

/// Puts the host that was lent before back when the call ends, a panic included.
struct Restore(Option<Rc<dyn CustomScriptHost>>);

impl Drop for Restore {
    fn drop(&mut self) {
        let previous = self.0.take();
        HOST.with(|cell| cell.set(previous));
    }
}

/// R915002: run `call` with `host` building custom cards' scripts, then put back whatever host was
/// lent before (none, outside every such call).
pub fn with_custom_scripts<R>(host: Rc<dyn CustomScriptHost>, call: impl FnOnce() -> R) -> R {
    let previous = HOST.with(|cell| cell.replace(Some(host)));
    let _restore = Restore(previous);
    call()
}

/// The host lent for the call in flight, if any.
fn lent_host() -> Option<Rc<dyn CustomScriptHost>> {
    HOST.with(|cell| {
        let host = cell.take();
        cell.set(host.clone());
        host
    })
}

/// R915002: a custom card's scripts in `state`, from the host lent for this call. `None` when the
/// id is no custom card of this game or no host is lent.
pub fn custom_scripts(state: &GameState, def_id: &str) -> Option<Arc<CardScripts>> {
    if !is_custom_id(def_id) {
        return None;
    }
    let card = state.custom_cards.get(def_id)?;
    lent_host()?.scripts(card)
}
