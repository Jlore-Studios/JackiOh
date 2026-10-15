//! Custom cards' scripts (#913, #915, SPEC §12.3 to §12.5): the card kit, loading a custom card at
//! runtime, its budget and what a failure does.
//!
//! - **The kit** is `kit/kit.luau`, host code: it runs once per VM in the sandbox, with `J` in reach,
//!   and hands back `Card`. A custom card's chunk runs in its own environment, which holds `Card`, the
//!   sandbox's globals and no `require`: `J` stays underneath (K4).
//! - **Loading** (`load_custom_card`): the source is linted and compiled as any script's (`compile`),
//!   then loaded as a card's module by `script::load_card`. The card's name in the VM is its id with
//!   a digest of its source (`custom-…#<hex>`), so an edited card is a new chunk, never the cached
//!   one, and both it and the bytecode are leaked to `'static` once per distinct source, as the
//!   loader's sites need: the cache below keeps a source from being leaked twice.
//! - **The budget** (K7): a custom card's hook call starts `CUSTOM_HOOK_INTERRUPTS`, not
//!   `LUAU_HOOK_INTERRUPTS` (`sandbox::Budget::enter`), under the same memory cap.
//! - **A failure** (R915004): a custom card's hook that errors, runs out, or returns what the host
//!   refuses resolves `effects::script_failed` instead of panicking (`hooks::failed`), and its aura
//!   gives nothing. A card whose source does not load gets scripts whose every hook it may have says
//!   so (`failing_scripts`).
//!
//! CLAUDE.md rule 4: the cache is the VM's (`vm::Vm.custom`), so it is thread-local as the VM is, and
//! it holds compiled scripts only, keyed by the card's id and source: what a lookup answers is a
//! function of the card it is handed.

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

/// The problems a custom card's source has: the lint's findings, then Luau's compiler, then loading
/// it as the kit's module (`Card { … }`'s own checks). Empty for a source that loads. The creator
/// shows these; nothing is cached.
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

/// R915004: a custom card's hook failed: the effect that says so, naming what went wrong.
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

/// R915004: the scripts of a card whose source does not load: each hook a custom card may have says
/// so when it runs, and does nothing else.
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

/// R915002: a custom card's scripts, from the VM's cache or compiled and loaded now. Never fails: a
/// source that does not load answers `failing_scripts`.
pub fn load_custom_card(card: &CustomCard) -> Arc<CardScripts> {
    let name = chunk_name(card);
    if let Some(scripts) = crate::vm::cached_custom(&name) {
        return scripts;
    }
    let scripts = Arc::new(build(card, &name).unwrap_or_else(|error| failing_scripts(&error)));
    crate::vm::cache_custom(name, scripts.clone());
    scripts
}

/// A custom card's scripts, compiled and loaded, or why not. The name and bytecode are leaked to
/// `'static`, as `load_card`'s sites need, once per distinct source: every caller caches what this
/// builds under `name` (`vm::cache_custom`).
fn build(card: &CustomCard, name: &str) -> Result<CardScripts, String> {
    if card.source.trim().is_empty() {
        return Ok(CardScripts::default());
    }
    let bytecode = compile(name, &card.source).map_err(|error| error.to_string())?;
    let card_name: &'static str = Box::leak(name.to_string().into_boxed_str());
    let bytecode: &'static [u8] = Box::leak(bytecode.into_boxed_slice());
    crate::script::load_card(card_name, bytecode).map_err(|error| error.to_string())
}

/// The host the browser's Luau module and the tests lend the engine (`with_custom_scripts`): the kit,
/// with this thread's VM and cache.
pub struct KitHost;

impl CustomScriptHost for KitHost {
    fn scripts(&self, card: &CustomCard) -> Option<Arc<CardScripts>> {
        Some(load_custom_card(card))
    }
}

/// The lint's findings on the kit itself, which the kit's own test checks it has none of but the one
/// it needs (its key count, `check_spec`), so the kit is held to the rules a card is.
pub fn kit_findings() -> Vec<lint::Finding> {
    lint::lint("kit.luau", KIT_SOURCE)
}
