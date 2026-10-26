//! `choose_target_where` by name (#1904): `where` names a face function
//! (`J.choose_target_where({ step = "picked", scope = { ... }, where = "admits" })`), called as
//! `face[where](ctx, card)` for each card the scope offers (`card` is nil for a hero) and answering
//! whether to offer it. The closure captures only the card, the face, the name and the site — the
//! `draw_while` pattern — so nothing a script sets survives the call (L8), and no registry holds
//! script state (CLAUDE.md rule 4). A misspelled name fails as the verb builds, naming the card, face
//! and function; a call with no `where` offers everything the scope offers.
//!
//! The engine asks `where` with a shared context (`TargetWhere`), and the effect context's readers
//! take theirs mutably (its `rng_*`), so the function reads a `HookArgs` over the same state: the
//! running card as it stands now (`live_self`), the running face and every shared reader. With no
//! running card (a step re-entered after its Spell left the resolving zone) there is nothing to ask
//! with, and nothing is admitted: the prompt fizzles, as one the scope gives no target does.

use std::sync::Arc;

use indexmap::IndexMap;
use mlua::LuaSerdeExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use jackioh_engine::CardInstance;
use jackioh_engine::effects::{TargetScope, TargetWhere};
use jackioh_engine::script::{EffectContext, HookArgs};

use crate::api::EffectCall;
use crate::{LuauError, Site};

/// `choose_target_where`'s arguments: the engine's, with `where` naming the face function that
/// admits each card.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ChooseTargetWhereArgs {
    pub step: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<TargetScope>,
    #[serde(default, rename = "where", skip_serializing_if = "Option::is_none")]
    pub where_: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<IndexMap<String, Value>>,
}

pub fn build_choose_target_where(
    call: &EffectCall,
    site: Site,
    bytecode: &'static [u8],
) -> Result<jackioh_engine::Effect, LuauError> {
    let args: ChooseTargetWhereArgs = crate::api::typed(call, site)?;
    let where_: Option<TargetWhere> = match args.where_ {
        Some(name) => {
            crate::hooks::check_named_hook(site, bytecode, &name)?;
            Some(Arc::new(move |ctx, card| admits(site, bytecode, &name, ctx, card)))
        }
        None => None,
    };
    Ok(jackioh_engine::effects::choose_target_where(
        jackioh_engine::effects::ChooseTargetWhereArgs {
            step: args.step,
            scope: args.scope,
            where_,
            prompt: args.prompt,
            data: args.data,
        },
    ))
}

/// `face[name](ctx, card)` over a `HookArgs` of `ctx`: whether `card` (a hero, `None`) may be
/// offered. A failure panics naming the card, face and function, since an engine hook has no error
/// to return.
fn admits(
    site: Site,
    bytecode: &'static [u8],
    name: &str,
    ctx: &EffectContext<'_>,
    card: Option<&CardInstance>,
) -> bool {
    let Some(self_) = ctx.live_self() else {
        return false;
    };
    let args = HookArgs {
        state: &*ctx.state,
        self_,
        radiant: ctx.radiant,
    };
    match crate::vm::with_vm(site, bytecode, |lua, face| {
        let func = crate::hooks::face_function(&face, name)?;
        let card = match card {
            Some(card) => lua.to_value(card)?,
            None => mlua::Value::Nil,
        };
        crate::reads::scoped(lua, args, |ctx_ud| {
            let answered: mlua::Value = func.call((ctx_ud, card))?;
            Ok(answered.as_boolean().unwrap_or(false))
        })
    }) {
        Ok(admitted) => admitted,
        Err(error) => panic!("{error}"),
    }
}

verbs! {
    verb!(choose_target_where, ChooseTargetWhereArgs);
}
