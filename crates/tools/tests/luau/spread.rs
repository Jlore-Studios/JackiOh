// A fixture for `luau surface`, read as text and never compiled: the Radiant face spreads the base's
// clone under a hook of its own.
use jackioh_engine::prelude::*;

pub const ID: &str = "fixture-spread";

pub fn script() -> CardScripts {
    let base = Script {
        death: Some(hook(|_ctx| vec![draw(json_as(json!({})))])),
        ..Script::default()
    };
    let radiant = Script {
        end_of_turn: Some(hook(|_ctx| vec![heal(json_as(json!({})))])),
        ..base.clone()
    };
    CardScripts { base, radiant }
}
