//! `after_state_check` by name: the script passes the name of a function in its face
//! table (`J.after_state_check({ hook = "after_check", data = { ... } })`), and the
//! build closure captures only the card, the face, the name, the data and the site.
//! When the engine calls it, the card's chunk is re-run through the thread-local VM
//! and `module[face][hook]` is called with the scoped `ctx` and the data, the way
//! part 4 builds every hook. Nothing a script sets survives the call (L8), and no
//! registry, counter or map holds script state anywhere (CLAUDE.md rule 4).

use serde::{Deserialize, Serialize};

use crate::api::EffectCall;
use crate::{LuauError, Site};

/// `after_state_check`'s arguments: the face function to call back, and the data it
/// reads — Classic #43's mana count and plagued ids, computed before the destroys.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct AfterStateCheckArgs {
    pub hook: String,
    #[serde(default)]
    pub data: serde_json::Value,
}

pub fn build_after_state_check(
    call: &EffectCall,
    site: Site,
    bytecode: &'static [u8],
) -> Result<jackioh_engine::Effect, LuauError> {
    let args: AfterStateCheckArgs = crate::api::typed(call, site)?;
    // The name resolves when the verb builds, so a misspelling fails the card's own
    // test with an error naming the card, face and function.
    crate::hooks::check_named_hook(site, bytecode, &args.hook)?;
    let name = args.hook;
    let data = args.data;
    Ok(jackioh_engine::effects::after_state_check(move |ctx| {
        match crate::hooks::call_named_effects(site, bytecode, &name, ctx, &data) {
            Ok(effects) => effects,
            Err(error) => panic!("{error}"),
        }
    }))
}

verbs! {
    verb!(after_state_check, AfterStateCheckArgs);
}
