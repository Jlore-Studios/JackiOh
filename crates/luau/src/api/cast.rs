use std::sync::Arc;
use serde::{Deserialize, Serialize};
use jackioh_engine::effects::{CastEachArgs as EngineCastEachArgs, CastHow};
use crate::api::EffectCall;
use crate::{LuauError, Site};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[derive(ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct CastEachArgs {
    pub cards: Vec<String>,
    #[serde(flatten)]
    pub how: CastHow,
}

pub fn build_cast_each(
    call: &EffectCall,
    site: Site,
    _bytecode: &'static [u8],
) -> Result<jackioh_engine::Effect, LuauError> {
    let args: CastEachArgs = crate::api::typed(call, site)?;
    let cards = args.cards;
    Ok(jackioh_engine::effects::cast_each(EngineCastEachArgs {
        cards: Arc::new(move |_ctx| cards.clone()),
        how: args.how,
    }))
}

verbs! {
    verb!(cast, CastArgs);
    verb!(cast_each, CastEachArgs);
    verb!(cast_new, CastNewArgs);
    verb!(cast_random, CastRandomArgs);
    verb!(add_cost_rule, AddCostRuleArgs);
    verb!(enchant_next_spell, EnchantNextSpellArgs);
}
