// A fixture for `luau surface`, read as text and never compiled: one call of every kind the
// classifier tells apart, among text it must not read as a call.
use jackioh_engine::prelude::*;

use crate::query::TRAP_TYPES;

#[derive(Clone)]
struct Marker;

fn maybe_cost() -> Option<fn(&EffectContext<'_>) -> i32> {
    None
}

pub const ID: &str = "fixture-calls";

fn cry(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    // destroy(json_as(json!({})))
    let label = "destroy(";
    chosen_options(ctx);
    ctx.live_self();
    let roll = |n: i32| n + 1;
    crate::query::pool(ID, &json_as(json!({ "t": TRAP_TYPES })));
    crate::card_def(ID);
    jackioh_engine::work::part_memory_key(&ctx.data, "k");
    subsystems::call_to_chaos(json_as(json!({})));
    if let Some(cost) = maybe_cost() {
        cost(ctx);
    }
    mystery_helper(ctx);
    vec![damage(json_as(json!({ "n": roll(1) }))), remember(json_as(json!({ "k": label })))]
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(cry)),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}
