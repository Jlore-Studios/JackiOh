//! Custom cards' scripts (#913, #915, SPEC §12.3 to §12.5): kit environment, runtime loading,
//! budget `CUSTOM_HOOK_INTERRUPTS` (K7), and error handling (`effects::script_failed`, R915004).
//! VM cache is thread-local and pure (CLAUDE.md rule 4).

use std::sync::Arc;

use jackioh_engine::custom::CustomScriptHost;
use jackioh_engine::effects::script_failed;
use jackioh_engine::{CardScripts, CustomCard, Effect, Script, hook};

use crate::{LuauError, Site, compile, lint};

/// The kit's source, embedded.
pub const KIT_SOURCE: &str = include_str!("kit/kit.luau");

/// Whether a site is a custom card's (its name is in the custom id space).
pub fn is_custom_site(site: Site) -> bool {
    jackioh_engine::custom::is_custom_id(site.card)
}

/// The name a custom card's chunk is cached and every error is reported under: its id and the FNV-1a
/// digest of its source, so two sources of one card are two chunks.
pub fn chunk_name(card: &CustomCard) -> String {
    format!("{}#{}", card.id, jackioh_engine::fnv1a32_utf16(&card.source))
}

/// One problem with a custom card's script, for the creator: the line it is on, when it has one, and
/// a sentence for the author.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScriptProblem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    pub message: String,
}

/// `file:line: message` (Luau's and the lint's form, or a runtime error's `[string "…"]:line:`) as a
/// line and the message alone; anything else is a message with no line.
pub fn problem_of(text: &str) -> ScriptProblem {
    // Luau's traceback, after the first line, says nothing an author can use.
    let first = text.split("\nstack traceback").next().unwrap_or(text);
    let trimmed = first.lines().next().unwrap_or(first).trim();
    // A runtime error names the chunk `custom-…#hex:LINE: message`; a lint finding `file:LINE: rule`.
    for (at, _) in trimmed.match_indices(':') {
        let rest = &trimmed[at + 1..];
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if digits.is_empty() {
            continue;
        }
        let after = &rest[digits.len()..];
        if let Some(message) = after.strip_prefix(':')
            && let Ok(line) = digits.parse::<u32>()
        {
            return ScriptProblem {
                line: Some(line),
                message: plain(message.trim()),
            };
        }
    }
    ScriptProblem {
        line: None,
        message: plain(trimmed),
    }
}

/// A message with the host's own words taken off: the L-rule tags the lint and the sandbox add, and
/// a nested error's prefix naming the chunk.
fn plain(message: &str) -> String {
    let mut text = message.to_string();
    for tag in [" (L5)", " (L6)", " (L7)"] {
        text = text.replace(tag, "");
    }
    text
}

/// The problems a custom card's source has, if any.
pub fn check_custom_source(card: &CustomCard) -> Vec<ScriptProblem> {
    let name = chunk_name(card);
    match compile(&name, &card.source) {
        Err(LuauError::Lint(findings)) => findings
            .iter()
            .map(|finding| ScriptProblem {
                line: Some(finding.line),
                message: plain(&finding.rule.to_string()),
            })
            .collect(),
        Err(error) => vec![problem_of(&error.to_string())],
        Ok(bytecode) => {
            if let Err(error) = crate::vm::check_custom_module(&name, &bytecode) {
                return vec![problem_of(&error)];
            }
            // The module runs: now load it as the game will, declarations and all.
            match build(card, &name) {
                Ok(scripts) => {
                    crate::vm::cache_custom(name, Arc::new(scripts));
                    Vec::new()
                }
                Err(error) => vec![problem_of(&error)],
            }
        }
    }
}

/// R915004: effect created when a custom card hook fails.
pub fn failure(site: Site, message: &str) -> Effect {
    let problem = problem_of(message.split_once(": ").map_or(message, |(head, rest)| {
        // `custom-…#hex base cry: <error>`: the site's own prefix goes, the error stays.
        if head.starts_with(jackioh_engine::config::CUSTOM_ID_PREFIX) {
            rest
        } else {
            message
        }
    }));
    let text = match problem.line {
        Some(line) => format!("{} failed on line {line}: {}", site.hook, problem.message),
        None => format!("{} failed: {}", site.hook, problem.message),
    };
    script_failed(text)
}

/// R915004: fallback scripts for a card whose source fails to load.
pub fn failing_scripts(message: &str) -> CardScripts {
    let text = format!("its script did not load: {}", problem_of(message).message);
    let say = |text: String| hook(move |_ctx| vec![script_failed(text.clone())]);
    let script = Script {
        cry: Some(say(text.clone())),
        death: Some(say(text)),
        ..Script::default()
    };
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

/// R915002: loads or retrieves cached scripts for a custom card.
pub fn load_custom_card(card: &CustomCard) -> Arc<CardScripts> {
    let name = chunk_name(card);
    if let Some(scripts) = crate::vm::cached_custom(&name) {
        return scripts;
    }
    let scripts = Arc::new(build(card, &name).unwrap_or_else(|error| failing_scripts(&error)));
    crate::vm::cache_custom(name, scripts.clone());
    scripts
}

/// Compiles and loads custom card scripts, leaking bytecode to `'static` for loader sites.
fn build(card: &CustomCard, name: &str) -> Result<CardScripts, String> {
    if card.source.trim().is_empty() {
        return Ok(CardScripts::default());
    }
    let bytecode = compile(name, &card.source).map_err(|error| error.to_string())?;
    let card_name: &'static str = Box::leak(name.to_string().into_boxed_str());
    let bytecode: &'static [u8] = Box::leak(bytecode.into_boxed_slice());
    crate::script::load_card(card_name, bytecode).map_err(|error| error.to_string())
}

/// Engine script host backed by thread-local VM and card kit.
pub struct KitHost;

impl CustomScriptHost for KitHost {
    fn scripts(&self, card: &CustomCard) -> Option<Arc<CardScripts>> {
        Some(load_custom_card(card))
    }
}

/// Lint findings on the embedded kit source.
pub fn kit_findings() -> Vec<lint::Finding> {
    lint::lint("kit.luau", KIT_SOURCE)
}
