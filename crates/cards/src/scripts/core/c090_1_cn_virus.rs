//! #90.1 CN-Virus (SPEC §8.5, §7, §4.4, R57, R58, R70, R80, R316, R350).
//!
//! Base: "Cast on draw: take 1 damage; at end of turn, shuffle 2 copies of this into your deck".
//! Radiant: take 2 damage, 3 copies (§8's cell, R275: both numbers scale).
//!
//! A spell token (§7, R11): `static_flags.cast_on_draw` is the whole of "Cast on draw", on both faces.
//! The draw casts it free as a card played (R70) and caps the chain (R58). It hits the drawer's own
//! hero (control follows ownership off the field, R12) as damage, not "lose health" (R18): §4.4's
//! pipeline, so Armor applies, and a hit reduced to 0 triggers nothing (R63) but the copies are owed.
//! The copies wait for the end of the turn it was cast on, whoever's (R350), at §2.2's delayed-effect
//! point in R68's order, so a chain casts only library viruses; a later cast is due next time (R62).
//! They are fresh copies with the cast face's radiant flag (R57); a full library refuses (R80, R316).

use jackioh_engine::prelude::*;

pub const ID: &str = "core-090-1";

// No `mod tests` here: #90.1's behaviour is proved by #90 CN Viral Injection's tests
// (`c090_cn_viral_injection.rs`).

/// R350: the step the end-of-turn delayed effect re-enters (§10.6: `script.resume[step]`).
const COPIES_STEP: &str = "copies";

/// The two numbers are the whole of the radiant text, declared as `damage` and `copies` (R386); less
/// of either is better for the virus's controller, the player who draws it.
fn virus() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(|ctx| {
            let amount = param(&*ctx, "damage");
            vec![
                damage(json_as(json!({ "to": { "of": "selfHero" }, "amount": amount }))),
                // R350: the copies at the end of the turn this is cast on, whoever's turn that is.
                delay(json_as(json!({
                    "at": { "phase": "end", "player": THIS_TURN },
                    "step": COPIES_STEP,
                    "hook": RESUME_HOOK,
                }))),
            ]
        })),
        resume: IndexMap::from([(
            // R57: fresh copies with the flag of the face that was cast; R80 and R316 at a full library.
            COPIES_STEP,
            hook(|ctx| {
                let mut args = json!({
                    "defId": ID,
                    "count": param(&*ctx, "copies"),
                    "player": "self",
                    "radiant": ctx.radiant,
                });
                if let Some(this) = ctx.self_.as_ref() {
                    args["copyOf"] = json!(this.id);
                }
                vec![shuffle_into(json_as(args))]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = virus();
    let radiant = virus();
    CardScripts { base, radiant }
}
