// A fixture for `luau surface`, read as text and never compiled: two `Script { … }` literals, one
// binding a hook's list by shorthand, and a call below the tests that must not be read.
use jackioh_engine::prelude::*;

pub const ID: &str = "fixture-literals";

pub fn script() -> CardScripts {
    let targets = vec![TargetDecl::target(1, 1, json!({}))];
    let base = Script {
        targets,
        cry: Some(hook(|_ctx| vec![destroy(json_as(json!({})))])),
        ..Script::default()
    };
    let radiant = Script {
        cry: Some(hook(|_ctx| vec![destroy_all(json_as(json!({})))])),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

#[cfg(test)]
mod tests { fn t() { heal(json_as(json!({}))); } }
