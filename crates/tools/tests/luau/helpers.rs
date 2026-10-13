// A fixture for `luau surface`, read as text and never compiled: both faces come from one helper
// that calls another, and a helper no face reaches.
use jackioh_engine::prelude::*;

pub const ID: &str = "fixture-helpers";

fn amount(ctx: &EffectContext<'_>) -> i32 {
    param(ctx, "n")
}

fn cry(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    vec![draw(json_as(json!({ "n": amount(ctx) })))]
}

fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(cry)),
        end_of_turn: if radiant { Some(hook(cry)) } else { None },
        ..Script::default()
    }
}

fn unused() -> Effect {
    heal(json_as(json!({})))
}

pub fn script() -> CardScripts {
    CardScripts { base: face(false), radiant: face(true) }
}
