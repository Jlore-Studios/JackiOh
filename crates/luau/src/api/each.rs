//! `for_each_card` by name: the script passes the names of face functions
//! (`J.for_each_card({ cards = "each_cards", each = "each_card" })`), or the ids
//! outright (`cards = { ... }`). The closures capture only the card, the face, the
//! names and the site: nothing a script sets survives the call (L8), and no registry
//! holds script state (CLAUDE.md rule 4).

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::api::EffectCall;
use crate::{LuauError, Site};

/// `for_each_card`'s set: the ids outright, or the face function reading them.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum CardsSource {
    Hook(String),
    Ids(Vec<String>),
}

/// `for_each_card`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ForEachCardArgs {
    #[serde(default)]
    pub cards: Option<CardsSource>,
    #[serde(default)]
    pub each: Option<String>,
}

pub fn build_for_each_card(
    call: &EffectCall,
    site: Site,
    bytecode: &'static [u8],
) -> Result<jackioh_engine::Effect, LuauError> {
    let args: ForEachCardArgs = crate::api::typed(call, site)?;
    // Every named function resolves when the verb builds, so a misspelling fails the
    // card's own test with an error naming the card, face and function.
    if let Some(CardsSource::Hook(name)) = &args.cards {
        crate::hooks::check_named_hook(site, bytecode, name)?;
    }
    if let Some(name) = &args.each {
        crate::hooks::check_named_hook(site, bytecode, name)?;
    }
    let source = args.cards;
    let cards: jackioh_engine::effects::ForEachCardCards = Arc::new(move |ctx| {
        let Some(source) = &source else {
            return Vec::new();
        };
        match source {
            CardsSource::Ids(ids) => ids.clone(),
            CardsSource::Hook(name) => {
                match crate::vm::with_vm(site, bytecode, |lua, face| {
                    let func = crate::hooks::face_function(&face, name.as_str())?;
                    lua.scope(|scope| {
                        let ctx_ud =
                            scope.create_any_userdata(ctx, crate::reads::register_ctx)?;
                        let returned: mlua::Value = func.call(ctx_ud)?;
                        let json = crate::numbers::to_json(&returned, site)
                            .map_err(|error| mlua::Error::runtime(error.to_string()))?;
                        match json {
                            serde_json::Value::Array(items) => Ok(items
                                .into_iter()
                                .filter_map(|item| item.as_str().map(str::to_string))
                                .collect()),
                            _ => Err(mlua::Error::runtime(format!(
                                "{name}: a cards function answers ids, not {}",
                                crate::api::kind(&json)
                            ))),
                        }
                    })
                }) {
                    Ok(ids) => ids,
                    Err(error) => panic!("{error}"),
                }
            }
        }
    });

    let each_name = args.each;
    let each: jackioh_engine::effects::ForEachCardEach = Arc::new(move |id: &str| {
        let Some(name) = &each_name else {
            return jackioh_engine::Effect::new("noop", |_| {});
        };
        // One id, one effect: the first of the list, as before.
        match crate::hooks::call_named_each(site, bytecode, name, id) {
            Ok(mut effects) if !effects.is_empty() => effects.remove(0),
            Ok(_) => jackioh_engine::Effect::new("noop", |_| {}),
            Err(error) => panic!("{error}"),
        }
    });

    Ok(jackioh_engine::effects::for_each_card(jackioh_engine::effects::ForEachCardArgs {
        cards,
        each,
    }))
}

verbs! {
    verb!(for_each_card, ForEachCardArgs);
}
