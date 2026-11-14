//! SPEC §8.12 S+ #20 B-Patch, a card of the Stinkyscape+ set (R1420, R3535000). (1) Spell, Epic.
//!   Base:    "Cast on draw: Flip a coin for the whole effect. Randomly either Buff or Nerf every card on
//!            the field and in both hands."
//!   Radiant: "Lucky 1" and the same.
//!
//! Cast on draw (§6.2, `StaticFlags.cast_on_draw`): the one coin is flipped in the Cry, so every card
//! the walk reaches is Buffed or every one Nerfed, never a mix (R3557000). Heads is the Buff, so Lucky 1
//! (catalog data on the Radiant face) gives the second flip that keeps heads (R1440). A card that
//! cannot be Buffed or Nerfed is skipped by the verb. A hand's and a deck's changes are silent to the
//! other seat (R440). Declares no numbers.

use jackioh_engine::effects::{degrade, upgrade};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-020";

/// The card's Lucky X, printed plus given (R1438), plus its controller's Luck (R987).
fn lucky_of(ctx: &EffectContext<'_>) -> i32 {
    ctx.self_.as_ref().map_or(0, |own| lucky_on(ctx.sink.state, own)) + luck_of(ctx.sink.state, ctx.controller)
}

fn patch() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(|ctx| {
            // One flip for the whole effect (R3557000); heads Buffs.
            let lucky = lucky_of(ctx);
            let scope = json!({ "side": "any", "zones": ["field", "hand"] });
            if ctx.rng.lucky_coin(lucky) {
                vec![upgrade(json_as(json!({ "scope": scope, "times": 1 })))]
            } else {
                vec![degrade(json_as(json!({ "scope": scope, "times": 1 })))]
            }
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // Lucky 1 is the Radiant face's printed keyword: catalog data.
    let base = patch();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #20 B-Patch — SPEC §8.12 row 20 (R3557000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const FILLER: &str = "core-005";
    const BODY: &str = "core-019"; // Midrange Menace
    const NETHER: &str = "core-088"; // Twisting Nether: a Spell with nothing a Nerf could take away.

    fn board(seed: &str, radiant: bool) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [FILLER],
                "library": [{ "def": ID, "radiant": radiant }, FILLER, FILLER],
                "field": [BODY],
            },
            "p2": { "hand": [FILLER, NETHER], "library": filler(4), "field": [BODY, BODY] },
        }))
    }

    fn count(events: &[GameEvent], kind: &str) -> usize {
        events.iter().filter(|event| event.event_type().as_str() == kind).count()
    }

    /// The (upgraded, degraded) counts the draw of B-Patch leaves.
    fn flipped(seed: &str, radiant: bool) -> (usize, usize) {
        let mut s = board(seed, radiant);
        s.start_turn();
        (count(s.events(), "upgraded"), count(s.events(), "degraded"))
    }

    #[test]
    fn r3557000_one_roll_buffs_everything_or_nerfs_everything_never_a_mix() {
        crate::register_all();
        let buffed = seed_where("r3557000-buff", |seed| flipped(seed, false).0 > 0);
        let nerfed = seed_where("r3557000-nerf", |seed| flipped(seed, false).1 > 0);
        for (seed, buff) in [(&buffed, true), (&nerfed, false)] {
            let (up, down) = flipped(seed, false);
            let (wanted, other) = if buff { (up, down) } else { (down, up) };
            assert_eq!(other, 0, "{seed}: no card got the other change");
            // p1's two units' worth: its field Menace and the drawn-over hand cards, p2's field and hand,
            // 
            assert!(wanted >= 5, "{seed}: every field card and every hand card changed ({wanted})");
        }
    }

    #[test]
    fn r3557000_a_card_that_cannot_be_nerfed_is_skipped() {
        crate::register_all();
        let seed = seed_where("r3557000-skip", |seed| flipped(seed, false).1 > 0);
        let mut s = board(&seed, false);
        s.start_turn();
        let nether = s.card(NETHER).id.clone();
        // R440: a card no change reaches draws the bare cue `none`, as every hidden card's change does.
        let touched = s
            .events()
            .iter()
            .filter(|event| matches!(event.event_type().as_str(), "upgraded" | "degraded"))
            .map(crate::js)
            .any(|event| event["instanceId"] == nether && event["change"]["kind"] != "none");
        assert!(!touched, "Twisting Nether took no Nerf");
    }

    #[test]
    fn the_radiant_face_does_the_same_and_its_lucky_one_flips_to_keep_heads() {
        crate::register_all();
        let buffed = seed_where("r3557000-radiant", |seed| flipped(seed, true).0 > 0);
        let (up, down) = flipped(&buffed, true);
        assert!(up > 0 && down == 0);
        // Lucky 1: the Radiant face flips twice and keeps heads, so it Buffs more often than the base face.
        let seeds: Vec<String> = (0..40).map(|n| format!("r3557000-lucky-{n}")).collect();
        let base = seeds.iter().filter(|seed| flipped(seed, false).0 > 0).count();
        let lucky = seeds.iter().filter(|seed| flipped(seed, true).0 > 0).count();
        assert!(lucky > base, "Lucky 1 buffs more often: {lucky} against {base}");
    }

    /// R987, R1440: its controller's Luck flips that many more coins, heads kept. No card gives Luck since
    /// R3140000, so Feng Shui's kept v0.5.11r script, "You have Luck 1", lends it, against today's Feng
    /// Shui on the same board and seeds: Luck never turns a Buff into a Nerf, and turns some Nerf into a
    /// Buff.
    #[test]
    fn r987_its_controllers_luck_flips_one_more_coin_and_keeps_heads() {
        const FENG_SHUI: &str = "meditative-040"; // Feng Shui: "You have Luck 1" through v0.5.11r
        crate::register_all();
        let buffs = |seed: &str| {
            let mut s = scenario(json!({
                "seed": seed,
                "p1": {
                    "hand": [FILLER],
                    "library": [ID, FILLER, FILLER],
                    "field": [BODY],
                    "backrow": [FENG_SHUI],
                },
                "p2": { "hand": [FILLER, NETHER], "library": filler(4), "field": [BODY, BODY] },
            }));
            s.start_turn();
            count(s.events(), "upgraded") > 0
        };
        let mut turned = false;
        for n in 0..20 {
            let seed = format!("r987-b-patch-{n}");
            let plain = buffs(&seed);
            let lucky = crate::under_kept(
                FENG_SHUI,
                crate::scripts::meditative::c040_feng_shui::script_v0_5_11o(),
                || buffs(&seed),
            );
            assert!(lucky || !plain, "{seed}: Luck kept the Buff");
            turned |= lucky && !plain;
        }
        assert!(turned, "Luck 1 turned a Nerf into a Buff on some seed");
    }

    #[test]
    fn it_is_cast_free_and_goes_to_the_graveyard_when_drawn() {
        crate::register_all();
        let mut s = board("r3557000-cast", false);
        s.start_turn();
        s.expect_in_zone(ID, "graveyard");
    }
}
