use serde::{Deserialize, Serialize};
use crate::api::EffectCall;
use crate::{LuauError, Site};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[derive(ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct RandomEntryLuau {
    pub label: String,
    pub effects: Vec<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[derive(ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct DoRandomEntriesArgs {
    pub count: i32,
    pub entries: Vec<RandomEntryLuau>,
}

pub fn build_do_random_entries(
    call: &EffectCall,
    site: Site,
    bytecode: &'static [u8],
) -> Result<jackioh_engine::Effect, LuauError> {
    let args: DoRandomEntriesArgs = crate::api::typed(call, site)?;
    let mut entries = Vec::new();
    for entry in args.entries {
        let calls = crate::api::effect_calls(&serde_json::Value::Array(entry.effects), site)?;
        let mut effects = Vec::new();
        for call in calls {
            effects.push(crate::api::effect_of(&call, site, bytecode)?);
        }
        entries.push(jackioh_engine::effects::RandomEntry {
            label: entry.label,
            effects,
        });
    }
    Ok(jackioh_engine::effects::do_random_entries(jackioh_engine::effects::DoRandomEntriesArgs {
        count: args.count,
        entries,
    }))
}

verbs! {
    verb!(do_random_entries, DoRandomEntriesArgs);
}
