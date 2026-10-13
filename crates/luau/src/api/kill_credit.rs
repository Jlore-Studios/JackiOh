//! `with_kill_credit` by name: the script passes the names of face functions for `pairs` and
//! `then`, with `data` for both (`J.with_kill_credit({ killer = ..., pairs = "kill_pairs",
//! then = "kill_then", data = { ... }, during = { ... } })`). The closures capture only the card,
//! the face, the names, the data and the site — the `after_state_check` pattern — so nothing a
//! script sets survives the call (L8), and no registry holds script state (CLAUDE.md rule 4).
//!
//! The named `pairs` function runs as `face[pairs](ctx, data)` and answers a list of
//! `{ victimId, toId }` records; the named `then` runs as `face[then](pair, data)` for each pair
//! whose victim `during` destroyed, and answers its effects. A face that names neither keeps the
//! old shape: the verb watches nothing and `during` still applies.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use jackioh_engine::effects::TargetSpec;

use crate::api::EffectCall;
use crate::{LuauError, Site};

/// `with_kill_credit`'s arguments. `pairs` and `then` name face functions; `data` reaches both as
/// their second argument.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[derive(ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct WithKillCreditArgs {
    pub killer: TargetSpec,
    #[serde(default)]
    pub transfer: bool,
    #[serde(default)]
    pub during: serde_json::Value,
    #[serde(default)]
    pub pairs: Option<String>,
    #[serde(default)]
    pub then: Option<String>,
    #[serde(default)]
    pub data: serde_json::Value,
}

pub fn build_with_kill_credit(
    call: &EffectCall,
    site: Site,
    bytecode: &'static [u8],
) -> Result<jackioh_engine::Effect, LuauError> {
    let args: WithKillCreditArgs = crate::api::typed(call, site)?;
    if let Some(name) = &args.pairs {
        crate::hooks::check_named_hook(site, bytecode, name)?;
    }
    if let Some(name) = &args.then {
        crate::hooks::check_named_hook(site, bytecode, name)?;
    }
    let during_calls = crate::api::effect_calls(&single_as_list(&args.during), site)?;
    let during = match during_calls.first() {
        Some(first) => crate::api::effect_of(first, site, bytecode)?,
        None => jackioh_engine::Effect::new("noop", |_| {}),
    };
    let pairs_name = args.pairs;
    let pairs_data = args.data.clone();
    let pairs: jackioh_engine::effects::KillCreditPairs = Arc::new(move |ctx, _killer| {
        let Some(name) = &pairs_name else {
            return Vec::new();
        };
        match crate::hooks::call_named_kill_pairs(site, bytecode, name, ctx, &pairs_data) {
            Ok(credits) => credits,
            Err(error) => panic!("{error}"),
        }
    });
    let then_name = args.then;
    let then_data = args.data;
    let then: Option<jackioh_engine::effects::KillCreditThen> = then_name.map(|name| {
        Arc::new(move |pair: &jackioh_engine::KillCredit| {
            match crate::hooks::call_named_kill_then(site, bytecode, &name, pair, &then_data) {
                Ok(effects) => effects,
                Err(error) => panic!("{error}"),
            }
        }) as jackioh_engine::effects::KillCreditThen
    });
    Ok(jackioh_engine::effects::with_kill_credit(
        jackioh_engine::effects::WithKillCreditArgs {
            killer: args.killer,
            pairs,
            transfer: args.transfer,
            during,
            then,
        },
    ))
}

/// One effect call where the engine takes one effect: a bare `{ verb, args }` record reads as a
/// list of one, so `during = J.forced_attack_random(...)` needs no wrapping list.
fn single_as_list(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(record)
            if record.len() == 2 && record.contains_key("verb") && record.contains_key("args") =>
        {
            serde_json::Value::Array(vec![value.clone()])
        }
        other => other.clone(),
    }
}

verbs! {
    verb!(with_kill_credit, WithKillCreditArgs);
}
