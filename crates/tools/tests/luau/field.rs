// A fixture for `luau surface`, read as text and never compiled: the Radiant face borrows one hook of
// the base's, and only that one.
use jackioh_engine::prelude::*;

pub const ID: &str = "fixture-field";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|_ctx| vec![recruit_all(Default::default())])),
        death: Some(hook(|_ctx| vec![draw(json_as(json!({})))])),
        ..Script::default()
    };
    let radiant = Script {
        cry: base.cry.clone(),
        ..Script::default()
    };
    CardScripts { base, radiant }
}
