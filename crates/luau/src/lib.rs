//! `jackioh_luau`: the Luau host for card scripts (#442, v0.4.0). The sandbox a script runs in (L5),
//! the integer boundary every value crosses (L6), the lint that refuses what the sandbox cannot stop
//! (L6, L7), one VM per thread (L8), and `load_card`, which turns a compiled script into the engine's
//! `CardScripts`. Since v0.5.0 (#902) no official card runs Luau: it runs custom cards' scripts in the
//! card kit (`custom`, #913, #915). `README.md` is the contract.

pub mod api;
pub mod config;
pub mod custom;
pub mod hooks;
pub mod lint;
pub mod numbers;
pub mod reads;
pub mod sandbox;
pub mod script;
pub mod vm;

pub use api::EffectCall;
pub use lint::{Finding, Rule, lint};

use std::fmt;

use jackioh_engine::{CardScripts, Effect, EffectContext};
use mlua::chunk::Compiler;

/// A card's face: the module's `base` or `radiant` table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Face {
    Base,
    Radiant,
}

impl Face {
    /// The face's key in the module.
    pub fn key(self) -> &'static str {
        match self {
            Face::Base => "base",
            Face::Radiant => "radiant",
        }
    }
}

/// One hook of one card: what a hook's closure captures and what every error names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Site {
    pub card: &'static str,
    pub face: Face,
    pub hook: &'static str,
}

impl fmt::Display for Site {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.card, self.face.key(), self.hook)
    }
}

/// Why a script did not compile, load or run.
#[derive(Clone, Debug, PartialEq)]
pub enum LuauError {
    /// The lint's findings (L6, L7).
    Lint(Vec<Finding>),
    /// Luau's compiler refused the source. `message` starts with the line (`1: Expected …`).
    Syntax { file: String, message: String },
    /// The module did not run, or does not have the shape a card's module has.
    Load { card: String, message: String },
    /// A number that is not an `i32` (L6), in what a hook returned.
    Number { site: Site, value: String },
    /// A hook failed: a Luau error, the interrupt's cap (L5), or a value that cannot cross (L6).
    Hook { site: Site, message: String },
}

impl fmt::Display for LuauError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LuauError::Lint(findings) => {
                let lines: Vec<String> = findings.iter().map(Finding::to_string).collect();
                f.write_str(&lines.join("\n"))
            }
            LuauError::Syntax { file, message } => write!(f, "{file}:{message}"),
            LuauError::Load { card, message } => write!(f, "{card}: {message}"),
            LuauError::Number { site, value } => {
                write!(f, "{site}: {value} is not an integer in i32's range (L6)")
            }
            LuauError::Hook { site, message } => write!(f, "{site}: {message}"),
        }
    }
}

impl std::error::Error for LuauError {}

/// A script's source, linted and compiled to the bytecode `load_card` takes. `file` names the source
/// in every finding and error. `crates/cards/build.rs` will call this in part 4.
pub fn compile(file: &str, source: &str) -> Result<Vec<u8>, LuauError> {
    let findings = lint(file, source);
    if !findings.is_empty() {
        return Err(LuauError::Lint(findings));
    }
    compiler().compile(source).map_err(|error| LuauError::Syntax {
        file: file.to_string(),
        message: match error {
            mlua::Error::SyntaxError { message, .. } => message,
            other => other.to_string(),
        },
    })
}

/// The compiler `compile` uses, which builds no fastcall to a builtin the sandbox took away
/// (`sandbox::DISABLED_BUILTINS`), so such a call reads the global, which is gone, behind the lint.
/// It compiles at `LUAU_OPTIMIZATION_LEVEL`.
pub fn compiler() -> Compiler {
    Compiler::new()
        .set_optimization_level(config::LUAU_OPTIMIZATION_LEVEL)
        .set_disabled_builtins(sandbox::DISABLED_BUILTINS)
}

/// A card's scripts from its compiled module. The module runs once here to read which hooks each face
/// declares; each declared hook becomes a closure that captures only its `Site` and the bytecode, so
/// any thread's VM can run it. A hook that fails panics with its error, which names its card, face and
/// hook (an engine hook has no error to return).
pub fn load_card(card: &'static str, bytecode: &'static [u8]) -> Result<CardScripts, LuauError> {
    script::load_card(card, bytecode)
}

/// One hook's effects, built with the engine's verbs (CLAUDE.md rule 5).
pub fn run_hook(
    site: Site,
    bytecode: &'static [u8],
    ctx: &mut EffectContext<'_>,
) -> Result<Vec<Effect>, LuauError> {
    vm::call_hook(site, bytecode, ctx)?
        .iter()
        .map(|call| api::effect_of(call, site, bytecode))
        .collect()
}
