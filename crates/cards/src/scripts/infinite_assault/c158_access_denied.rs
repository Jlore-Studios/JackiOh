//! SPEC §8.13 IA #158 Access Denied ((1) Trap, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Reveals when your opponent plays a Spell that targets: Counter it."
//!   Radiant: "Reveals when your opponent plays a Spell that targets: Counter it. Your hero gains {armor} Armor." (armor 5)
//!
//! Engine: §10.5's announce window (`cardAnnounced`): an opponent's Spell (never a Field Spell,
//! R4591007) announced with at least one target — on either side — is countered (§6.3): it never
//! resolves and counts for no play. On the Radiant face the controller's hero then gains the
//! declared {armor} Armor. Every condition is in `when` (R99).

use jackioh_engine::effects::{counter_play, gain_hero_armor};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-158";

/// The opponent's announced Spell this trap answers (R4591007): a Spell, never a Field Spell, with
/// at least one declared target — aimed anywhere, on either side.
fn denied(event: &GameEvent, you: PlayerId) -> Option<String> {
    let GameEvent::CardAnnounced {
        player,
        card_type,
        targets,
        instance_id,
        ..
    } = event
    else {
        return None;
    };
    if *player == you || *card_type != CardType::Spell {
        return None;
    }
    if targets.is_empty() {
        return None;
    }
    Some(instance_id.clone())
}

fn denial(radiant: bool) -> TriggerDef {
    TriggerDef::new("access-denied", &[GameEventType::CardAnnounced], move |ctx, event| {
        let countered = denied(event, ctx.controller).unwrap_or_default();
        let mut effects = vec![counter_play(json_as(json!({
            "target": { "of": "instance", "instanceId": countered },
        })))];
        if radiant {
            effects.push(gain_hero_armor(json_as(json!({
                "amount": param(&*ctx, "armor"),
            }))));
        }
        effects
    })
    .with_when(move |ctx, event| denied(event, ctx.controller).is_some())
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            triggers: vec![denial(false)],
            ..Script::default()
        },
        radiant: Script {
            triggers: vec![denial(true)],
            ..Script::default()
        },
    }
}

// IA #158 Access Denied — SPEC §8.13 row 158 (R99, R448, R4591007).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const DENIED: &str = "infiniteassault-158";
    const VANILLA: &str = "stand-in (1) 4/4";
    const MENACE: &str = "core-019";
    const HIT_JOB: &str = "core-016";
    const STOCKPILE: &str = "core-005";

    fn setup(radiant_face: bool) -> Scenario {
        scenario(json!({
            "seed": "access-denied",
            "active": "p2",
            "p1": {
                "hand": [VANILLA],
                "field": [MENACE],
                "backrow": [{ "def": DENIED, "radiant": radiant_face }],
                "library": [VANILLA, STOCKPILE],
            },
            "p2": {
                "hand": [HIT_JOB, STOCKPILE, VANILLA],
                "field": [VANILLA],
                "library": [VANILLA, VANILLA, VANILLA],
                "mana": 8,
            },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r4591007_counters_a_targeting_spell() {
            let mut s = setup(false);
            let menace = s.unit_at(P1, 1);
            let hit_job = s.card(HIT_JOB).clone();
            let targets = pick::unit_at(&s, P1, 1);
            s.play(HIT_JOB, json!({ "targets": targets }));
            s.expect_in_zone(&hit_job, "graveyard");
            s.expect_in_zone(menace.id.as_str(), "field");
            s.expect_in_zone(DENIED, "graveyard");
            assert_eq!(s.last_count("cardPlayed"), 0);
            assert_eq!(s.last_count("cardResolved"), 0);
        }

        #[test]
        fn r4591007_lets_an_untargeted_spell_through() {
            let mut s = setup(false);
            s.play(STOCKPILE, json!({}));
            assert_eq!(s.last_count("cardResolved"), 1);
            assert_eq!(s.count("countered"), 0);
            s.expect_in_zone(DENIED, "field");
        }

        #[test]
        fn a_spell_aimed_only_at_their_own_side_still_counts() {
            let mut s = setup(false);
            let victim = s.unit_at(P2, 1).id.clone();
            let targets = pick::unit_at(&s, P2, 1);
            s.play(HIT_JOB, json!({ "targets": targets }));
            assert_eq!(s.count("countered"), 1, "a target on either side springs it");
            s.expect_in_zone(victim.as_str(), "field");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn countering_gains_5_armor() {
            let mut s = setup(true);
            let armor = s.state().players[P1].hero.armor;
            let targets = pick::unit_at(&s, P1, 1);
            s.play(HIT_JOB, json!({ "targets": targets }));
            assert_eq!(s.count("countered"), 1);
            assert_eq!(s.state().players[P1].hero.armor, armor + 5);
        }
    }
}
