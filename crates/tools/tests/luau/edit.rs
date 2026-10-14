// A fixture for `luau surface`, read as text and never compiled: the Radiant face is the base's clone
// with a hook set after it is made.
use jackioh_engine::prelude::*;

pub const ID: &str = "fixture-edit";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|_ctx| vec![draw(json_as(json!({})))])),
        ..Script::default()
    };
    let mut radiant = base.clone();
    radiant.death = Some(hook(|_ctx| vec![heal(json_as(json!({})))]));
    CardScripts { base, radiant }
}
