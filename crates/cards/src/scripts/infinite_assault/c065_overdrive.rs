//! SPEC §8.13 IA #65 Overdrive ((3) Spell, Epic), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Gain {mana} mana this turn. End of turn: Deal {damage} damage to your hero for each mana you didn't spend." (mana 4, damage 2)
//!   Radiant: "Gain {mana} mana this turn. End of turn: Deal {damage} damage to your hero for each mana you didn't spend." (mana 6, damage 2)
//!
//! Core #78 /fullsend's shape: the declared `mana` gained this turn (`gain_mana`, §2.3), plus a
//! `delay` to its controller's end of turn (R62) carrying the declared `damage` in its data
//! (§10.6). The resume deals that much to its controller's hero for each mana left unspent then —
//! every unspent mana counts, not just the gained mana, read at its R62 end-of-turn point
//! (R4587002).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-065";

/// The delayed end-of-turn step the Cry names; it labels the pause.
const BLOWBACK_STEP: &str = "blowback";

/// §10.6's data carried across the boundary, or the printed value when it is missing.
fn carried(ctx: &EffectContext<'_>, key: &str) -> i32 {
    match ctx.data.get(key).and_then(Value::as_i64) {
        Some(value) => value as i32,
        None => param(ctx, key),
    }
}

/// R4587002: the blowback, at its R62 end-of-turn point: the carried `damage` for each mana of the
/// whole pool left unspent, none when it is all spent.
fn blowback(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let amount = carried(ctx, "damage") * unspent_mana_of(ctx.state, ctx.controller);
    if amount <= 0 {
        return vec![];
    }
    vec![damage(json_as(json!({ "to": { "of": "selfHero" }, "amount": amount })))]
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![
                gain_mana(json_as(json!({ "amount": param(&*ctx, "mana") }))),
                delay(json_as(json!({
                    "at": { "phase": "end", "player": "self" },
                    "step": BLOWBACK_STEP,
                    "hook": RESUME_HOOK,
                    "data": { "damage": param(&*ctx, "damage") },
                }))),
            ]
        })),
        resume: IndexMap::from([(BLOWBACK_STEP, hook(blowback))]),
        ..Script::default()
    };
    // The same script: the Radiant face's 6 is its declared `mana`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #65 Overdrive — SPEC §8.13 row 65 (R4587002).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const TWO: &str = "stand-in (2) Spell";
    const ONE: &str = "stand-in (1) Spell";

    const FILLER: &str = "core-005"; // Stockpile: a card in hand, so no turn ends by itself

    fn board(radiant: bool, hand: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": hand, "library": crate::filler(4) },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }))
    }

    fn overdrive(radiant: bool) -> Value {
        json!({ "def": ID, "radiant": radiant })
    }

    mod base {
        use super::*;

        #[test]
        fn r4587002_two_damage_for_each_mana_left_at_your_end_of_turn() {
            // 4 paid 3, then the gain: 1 + 4 = 5; one (2) Spell spent: 3 left, 6 to the hero.
            let mut s = board(false, json!([overdrive(false), TWO]));
            s.play(ID, json!({}));
            s.play(TWO, json!({}));
            s.end_turn();
            s.expect_health(P1, HERO_HEALTH - 6);
            s.expect_health(P2, HERO_HEALTH);
        }

        #[test]
        fn spending_it_all_costs_nothing() {
            let mut s = board(false, json!([overdrive(false), TWO, TWO, ONE]));
            s.play(ID, json!({}));
            s.play(TWO, json!({}));
            s.play(TWO, json!({}));
            s.play(ONE, json!({}));
            s.end_turn();
            s.expect_health(P1, HERO_HEALTH);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4587002_the_radiant_face_gains_6() {
            // 4 paid 3, then the gain: 1 + 6 = 7; one (2) Spell spent: 5 left, 10 to the hero.
            let mut s = board(true, json!([overdrive(true), TWO]));
            s.play(ID, json!({}));
            s.play(TWO, json!({}));
            s.end_turn();
            s.expect_health(P1, HERO_HEALTH - 10);
        }
    }
}
