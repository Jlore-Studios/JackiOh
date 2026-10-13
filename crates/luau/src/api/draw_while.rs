use std::sync::Arc;
use mlua::Function;
use serde::{Deserialize, Serialize};
use jackioh_engine::effects::PlayerSpec;
use crate::api::EffectCall;
use crate::reads::register_ctx;
use crate::vm::VM;
use crate::{LuauError, Site};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DrawWhileArgs {
    #[serde(default)]
    pub callback_id: Option<u32>,
    #[serde(default)]
    pub player: Option<PlayerSpec>,
}

pub fn build_draw_while(call: &EffectCall, site: Site) -> Result<jackioh_engine::Effect, LuauError> {
    let args: DrawWhileArgs = crate::api::typed(call, site)?;
    let callback_id = args.callback_id;
    let more = Arc::new(move |ctx: &mut jackioh_engine::script::EffectContext<'_>| -> bool {
        if let Some(id) = callback_id {
            VM.with(|vm| {
                let key = crate::api::after_check::lookup_callback(id);
                let Some(key) = key else { return false; };
                let func: Function = vm.sandbox.lua.registry_value(&*key).expect("callback in registry");
                vm.sandbox.lua.scope(|scope| {
                    let ctx_ud = scope.create_any_userdata(ctx, register_ctx)?;
                    func.call(ctx_ud)
                }).unwrap_or(false)
            })
        } else {
            false
        }
    });
    Ok(jackioh_engine::effects::draw_while(jackioh_engine::effects::DrawWhileArgs {
        more,
        player: args.player,
    }))
}

verbs! {
    verb!(draw_while, DrawWhileArgs);
}
