use std::sync::Arc;
use mlua::{Function, Value};
use serde::{Deserialize, Serialize};
use crate::api::{effect_calls, EffectCall};
use crate::reads::register_ctx;
use crate::vm::VM;
use crate::{numbers, LuauError, Site};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ForEachCardArgs {
    #[serde(default)]
    pub cards: Vec<String>,
    #[serde(default)]
    pub cards_callback_id: Option<u32>,
    #[serde(default)]
    pub each_callback_id: Option<u32>,
}

pub fn build_for_each_card(call: &EffectCall, site: Site) -> Result<jackioh_engine::Effect, LuauError> {
    let args: ForEachCardArgs = crate::api::typed(call, site)?;
    let static_cards = args.cards;
    let cards_cb = args.cards_callback_id;
    let each_cb = args.each_callback_id;

    let cards: jackioh_engine::effects::ForEachCardCards = Arc::new(move |ctx| {
        if let Some(id) = cards_cb {
            VM.with(|vm| {
                let key = crate::api::after_check::lookup_callback(id);
                let Some(key) = key else { return Vec::new(); };
                let func: Function = vm.sandbox.lua.registry_value(&*key).expect("callback in registry");
                vm.sandbox.lua.scope(|scope| {
                    let ctx_ud = scope.create_any_userdata(ctx, register_ctx)?;
                    let res: Value = func.call(ctx_ud)?;
                    let json = numbers::to_json(&res, site).map_err(|e| mlua::Error::runtime(e.to_string()))?;
                    match json {
                        serde_json::Value::Array(arr) => Ok(arr.into_iter().filter_map(|v| v.as_str().map(str::to_string)).collect()),
                        _ => Ok(Vec::new()),
                    }
                }).unwrap_or_default()
            })
        } else {
            static_cards.clone()
        }
    });

    let each: jackioh_engine::effects::ForEachCardEach = Arc::new(move |id: &str| {
        if let Some(cb_id) = each_cb {
            let id_str = id.to_string();
            VM.with(|vm| {
                let key = crate::api::after_check::lookup_callback(cb_id);
                let Some(key) = key else { return jackioh_engine::Effect::new("noop", |_| {}); };
                let func: Function = vm.sandbox.lua.registry_value(&*key).expect("callback in registry");
                let ret: Value = func.call(id_str).expect("each callback call failed");
                let json = numbers::to_json(&ret, site).expect("json conversion");
                let calls = effect_calls(&json, site).expect("effect calls");
                match calls.first() {
                    Some(c) => crate::api::effect_of(c, site).expect("effect_of"),
                    None => jackioh_engine::Effect::new("noop", |_| {}),
                }
            })
        } else {
            jackioh_engine::Effect::new("noop", |_| {})
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
