// A fixture for `luau surface`, read as text and never compiled: the Radiant face is the base's clone.
use jackioh_engine::prelude::*;

pub const ID: &str = "fixture-clone";

pub fn script() -> CardScripts {
    let base = Script {
        death: Some(hook(|_ctx| vec![draw(json_as(json!({})))])),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}
