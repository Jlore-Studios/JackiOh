//! `draw_while` by name: the script passes the name of a face function
//! (`J.draw_while({ hook = "draw_more" })`), called with the scoped `ctx` before each
//! draw. The closure captures only the card, the face, the name and the site: nothing
//! a script sets survives the call (L8), and no registry holds script state (rule 4).

use serde::{Deserialize, Serialize};

use crate::api::EffectCall;
use crate::{LuauError, Site};

/// `draw_while`'s arguments: the face function answering whether to draw again.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DrawWhileArgs {
    pub hook: String,
    #[serde(default)]
    pub player: Option<jackioh_engine::effects::PlayerSpec>,
}

pub fn build_draw_while(
    call: &EffectCall,
    site: Site,
    bytecode: &'static [u8],
) -> Result<jackioh_engine::Effect, LuauError> {
    let args: DrawWhileArgs = crate::api::typed(call, site)?;
    crate::hooks::check_named_hook(site, bytecode, &args.hook)?;
    let name = args.hook;
    let more = std::sync::Arc::new(move |ctx: &mut jackioh_engine::script::EffectContext<'_>| -> bool {
        match crate::vm::with_vm(site, bytecode, |lua, face| {
            let func = crate::hooks::face_function(&face, name.as_str())?;
            lua.scope(|scope| {
                let ctx_ud = scope.create_any_userdata(ctx, crate::reads::register_ctx)?;
                let answered: mlua::Value = func.call(ctx_ud)?;
                Ok(answered.as_boolean().unwrap_or(false))
            })
        }) {
            Ok(keep_going) => keep_going,
            Err(error) => panic!("{error}"),
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
