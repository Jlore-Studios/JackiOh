//! M #97.2 Wishing Well (SPEC §8.8 row 97.2). (1) Unit, Token (printed Common), 0/6 → 0/12.
//!   Base:    "Can't attack\nActivate: {chance}% chance to add a random Radiant card to your hand."
//!   Radiant: "Can't attack, Lucky 1\nActivate: {chance}% chance to add a random Radiant card to your hand."
//!   Engine:  "Activate (§6.2, R384): its controller, in their main phase, once a turn, the turn it arrives
//!            included. One `rng.chance`, the Radiant face's Lucky 1 rolling again and keeping a success
//!            (§6.1 Lucky); a success adds a random non-token card of every set (R380), Radiant, hidden in
//!            your hand (R97), burned by a full hand (§2.4). Tunes: chance 10 ↑ (Radiant 12, step 2);
//!            Lucky 1 ↑, on the Radiant face only."
//!
//! One script serves both faces: the Radiant face's Lucky 1 is a numbered keyword on the face, read as
//! Nerf and Buff have moved it, so the base face (no Lucky) rolls once and the Radiant face twice.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-097-2";

/// §8.8: "Activate" is once a turn.
const USES: i32 = 1;

/// The declared `chance` is a percentage.
const PERCENT: i32 = 100;

/// The face's Lucky X, as Nerf and Buff have moved it (`effects/fruit.rs`'s `lucky_of` shape).
fn lucky_of(ctx: &EffectContext<'_>) -> i32 {
    let Some(own) = ctx.self_.as_ref() else {
        return 0;
    };
    numbered_keywords_on(ctx.sink.state, own)
        .into_iter()
        .find(|keyword| keyword.key == NumberedKey::Lucky)
        .map(|keyword| keyword.value)
        .unwrap_or(0)
}

fn wish() -> ActivationDecl {
    ActivationDecl {
        id: "wish".to_string(),
        label: "Make a wish".to_string(),
        uses: ActivationUses::Count(USES),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(|ctx| {
            let chance = f64::from(param(&*ctx, "chance")) / f64::from(PERCENT);
            let lucky = lucky_of(ctx);
            if ctx.rng.lucky(lucky, |rng| rng.chance(chance), |a, b| a || b) {
                vec![add_random_from_catalog(json_as(json!({ "radiant": true })))]
            } else {
                vec![]
            }
        }),
    }
}

pub fn script() -> CardScripts {
    let base = Script {
        activations: vec![wish()],
        ..Script::default()
    };
    // The Radiant face's Lucky 1 and 0/12 are catalog data, and its 12% is the declared `chance`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}
