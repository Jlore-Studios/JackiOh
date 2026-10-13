use std::sync::Arc;
use serde::{Deserialize, Serialize};
use jackioh_engine::effects::TargetSpec;
use crate::api::EffectCall;
use crate::{LuauError, Site};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct WithKillCreditArgs {
    pub killer: TargetSpec,
    #[serde(default)]
    pub transfer: bool,
    #[serde(default)]
    pub during: serde_json::Value,
    #[serde(default)]
    pub pairs_callback_id: Option<u32>,
    #[serde(default)]
    pub then_callback_id: Option<u32>,
}

pub fn build_with_kill_credit(call: &EffectCall, site: Site) -> Result<jackioh_engine::Effect, LuauError> {
    let args: WithKillCreditArgs = crate::api::typed(call, site)?;
    let during_calls = crate::api::effect_calls(&args.during, site)?;
    let during = match during_calls.first() {
        Some(c) => crate::api::effect_of(c, site)?,
        None => jackioh_engine::Effect::new("noop", |_| {}),
    };
    Ok(jackioh_engine::effects::with_kill_credit(jackioh_engine::effects::WithKillCreditArgs {
        killer: args.killer,
        pairs: Arc::new(|_ctx, _card| Vec::new()),
        transfer: args.transfer,
        during,
        then: None,
    }))
}

verbs! {
    verb!(with_kill_credit, WithKillCreditArgs);
}
