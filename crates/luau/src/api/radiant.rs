//! `radiant_chance` in Luau's integers: the script passes `chancePercent` (30 for "30%"), and the
//! build converts it with the same arithmetic the Rust scripts state as literals (`0.3`, `0.4`),
//! so the rng draws are identical (#464 gap 9). A fraction cannot cross the integer boundary
//! (L6), so the engine's `chance: f64` has no Luau form directly.

use serde::{Deserialize, Serialize};

use crate::api::EffectCall;
use crate::{LuauError, Site};

/// `radiant_chance`'s arguments in Luau: the engine's shape with `chancePercent` for `chance`.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct RadiantChanceLuauArgs {
    pub zone: jackioh_engine::OneOrMany<jackioh_engine::effects::RadiantZone>,
    /// The per-card probability in percent: 30 for "30%".
    pub chance_percent: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lucky: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<jackioh_engine::effects::PlayerSpec>,
}

/// `30.0 / 100.0` is the double the literal `0.3` names: the division is correctly rounded, so the
/// conversion below hands `rng.chance` exactly what the Rust script's literal did.
#[test]
fn percent_to_chance_is_the_literal() {
    assert_eq!(30.0 / 100.0, 0.3);
    assert_eq!(40.0 / 100.0, 0.4);
}

pub fn build_radiant_chance(
    call: &EffectCall,
    site: Site,
) -> Result<jackioh_engine::Effect, LuauError> {
    let args: RadiantChanceLuauArgs = crate::api::typed(call, site)?;
    Ok(jackioh_engine::effects::radiant_chance(
        jackioh_engine::effects::RadiantChanceArgs {
            zone: args.zone,
            chance: f64::from(args.chance_percent) / 100.0,
            lucky: args.lucky,
            player: args.player,
        },
    ))
}

verbs! {
    verb!(set_radiant, RadiantTarget);
    verb!(clear_radiant, RadiantTarget);
    verb!(set_radiant_random, SetRadiantRandomArgs);
    verb!(radiant_chance, RadiantChanceLuauArgs);
}
