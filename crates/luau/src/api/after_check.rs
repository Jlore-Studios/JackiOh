use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::Effect;
use mlua::{Function, Lua, RegistryKey, Value};
use serde::{Deserialize, Serialize};

use crate::api::{effect_calls, EffectCall};
use crate::reads::register_ctx;
use crate::vm::VM;
use crate::{numbers, LuauError, Site};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct AfterStateCheckArgs {
    pub callback_id: u32,
}

static CALLBACK_COUNTER: AtomicU32 = AtomicU32::new(1);

thread_local! {
    static CALLBACKS: RefCell<HashMap<u32, Arc<RegistryKey>>> = RefCell::new(HashMap::new());
}

pub fn register_callback(lua: &Lua, callback: Function) -> mlua::Result<u32> {
    let id = CALLBACK_COUNTER.fetch_add(1, Ordering::Relaxed);
    let key = Arc::new(lua.create_registry_value(callback)?);
    CALLBACKS.with(|cbs| cbs.borrow_mut().insert(id, key));
    Ok(id)
}

pub fn lookup_callback(id: u32) -> Option<Arc<RegistryKey>> {
    CALLBACKS.with(|cbs| cbs.borrow().get(&id).cloned())
}

pub fn build_after_state_check(call: &EffectCall, site: Site) -> Result<Effect, LuauError> {
    let args: AfterStateCheckArgs = crate::api::typed(call, site)?;
    let id = args.callback_id;
    Ok(jackioh_engine::effects::after_state_check(move |ctx| {
        VM.with(|vm| {
            let key = CALLBACKS.with(|cbs| cbs.borrow().get(&id).cloned());
            let Some(key) = key else {
                panic!("after_state_check callback {id} not found");
            };
            let func: Function = vm
                .sandbox
                .lua
                .registry_value(&*key)
                .expect("callback in registry");
            let returned: Value = vm
                .sandbox
                .lua
                .scope(|scope| {
                    let ctx_ud = scope.create_any_userdata(ctx, register_ctx)?;
                    func.call(ctx_ud)
                })
                .expect("after_state_check callback failed");
            let json = numbers::to_json(&returned, site).expect("json conversion");
            let calls = effect_calls(&json, site).expect("effect calls");
            calls
                .iter()
                .map(|c| crate::api::effect_of(c, site).expect("effect_of"))
                .collect()
        })
    }))
}

verbs! {
    verb!(after_state_check, AfterStateCheckArgs);
}
