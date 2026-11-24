//! IA #240 Unpatched (SPEC §8.13 row 240), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Reveals when your opponent plays a Spell: Counter it. Cast it for yourself."
//!   Radiant: "Reveals when your opponent plays a Spell: Counter it. Cast it for yourself, and add
//!            a copy of it to your hand."
//!
//! Any opponent's announced Spell springs it — its own controller's Spells never do (R448). The
//! counter takes the Spell to the thief's hand (as Wormhole's does, stinkyscape #139), and the cast
//! aims it freely at once: a target prompt opens for the thief, with no `friendsOnly` fence. A full
//! hand burns the countered card, and the cast still resolves from the graveyard. On the Radiant
//! face a copy is read off the countered card before the counter moves it — the copy keeping its
//! face (R57) — and added to the thief's hand.
//!
//! Engine: `counterPlay`, `cast`, `addToHand` (R4594015). Built by C10 (#4594).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-240";

/// An opponent's announced Spell, by instance.
fn opponents_spell(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<Announced> {
    let GameEvent::CardAnnounced { player, instance_id, card_type, .. } = event else {
        return None;
    };
    if *player == ctx.controller || *card_type != CardType::Spell {
        return None;
    }
    let card = find_instance(&*ctx.state, instance_id)?;
    Some(Announced { instance_id: instance_id.clone(), def_id: card.def_id.clone(), radiant: card.radiant })
}

/// The announced Spell, read before the counter moves it.
struct Announced {
    instance_id: String,
    def_id: String,
    radiant: bool,
}

fn unpatched(radiant: bool) -> Script {
    Script {
        triggers: vec![TriggerDef::new(
            "unpatched",
            &[GameEventType::CardAnnounced],
            move |ctx, event| {
                let Some(announced) = opponents_spell(ctx, event) else {
                    return vec![];
                };
                let card = json!({ "of": "instance", "instanceId": announced.instance_id });
                let mut effects = vec![counter_play(json_as(json!({
                    "to": "thief",
                    "target": card.clone(),
                })))];
                if radiant {
                    // Read before the counter moves it: the copy keeps the face (R57).
                    effects.push(add_to_hand(json_as(json!({
                        "defId": announced.def_id,
                        "radiant": announced.radiant,
                    }))));
                }
                effects.push(cast(json_as(json!({ "target": card }))));
                effects
            },
        )
        .with_when(|ctx, event| opponents_spell(ctx, event).is_some())],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts { base: unpatched(false), radiant: unpatched(true) }
}

// IA #240 Unpatched — SPEC §8.13 row 240 (R4594015).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.
    const STATS: &str = "classicplus-057"; // Book of Stats: give a Unit +3/+3.
    const VANILLA: &str = "core-008"; // Mr. Vanilla: a 4/4.

    fn trapped(radiant: bool, seed: &str, p1_extra: Value, p2_extra: Value) -> Scenario {
        scenario(json!({
            "seed": seed,
            "active": "p2",
            "p1": {
                "field": [VANILLA],
                "backrow": [{ "def": ID, "radiant": radiant }],
                "hand": p1_extra,
                "library": crate::filler(6),
                "health": 20,
            },
            "p2": { "field": [VANILLA], "hand": p2_extra, "library": crate::filler(6) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r4594015_an_enemy_stockpile_draws_for_p1() {
            let mut s = trapped(false, "unpatched-stockpile", json!([STOCKPILE]), json!([STOCKPILE]));
            let hand = s.hand(P1).len();
            s.play(STOCKPILE, json!({}));
            assert_eq!(s.hand(P1).len(), hand + 2, "the cast draws for the thief");
            s.expect_health(P1, 20 + 2);
        }

        #[test]
        fn r4594015_the_recast_asks_p1_with_free_aim() {
            let mut s = trapped(false, "unpatched-stats", json!([STOCKPILE]), json!([STATS]));
            let theirs = s.unit_at(P2, 1);
            s.play(STATS, json!({ "targets": pick::instance(&theirs) }));
            let mine = s.unit_at(P1, 1);
            s.answer(pick::instance(&mine));
            s.expect_stats(&mine, json!({ "attack": 4 + 5, "health": 4 + 5 }));
            s.expect_stats(&theirs, json!({ "attack": 4, "health": 4 }));
        }

        #[test]
        fn r4594015_a_full_hand_still_casts() {
            let mut hand = vec![json!(STOCKPILE)];
            hand.extend((0..9).map(|_| json!(STOCKPILE)));
            let mut s = trapped(false, "unpatched-full", json!(hand), json!([STOCKPILE]));
            assert_eq!(s.hand(P1).len(), 10);
            s.play(STOCKPILE, json!({}));
            s.expect_health(P1, 20 + 2);
        }

        #[test]
        fn r4594015_own_spells_do_nothing() {
            let mut s = trapped(false, "unpatched-own", json!([STOCKPILE]), json!([STOCKPILE]));
            s.end_turn();
            let hand = s.hand(P1).len();
            s.play(STOCKPILE, json!({}));
            assert_eq!(s.hand(P1).len(), hand - 1 + 2, "resolves for its caster");
            assert!(s.backrow(P1, 1).is_some(), "still set");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4594015_the_radiant_copy_keeps_the_face() {
            let mut s = trapped(
                true,
                "unpatched-radiant",
                json!([STOCKPILE]),
                json!([{ "def": STOCKPILE, "radiant": true }]),
            );
            s.play(STOCKPILE, json!({}));
            // The countered card is cast (a Radiant Stockpile draws four), and the Radiant face
            // adds one Radiant copy: 1 kept + 1 countered - 1 cast + 4 drawn + 1 copy.
            assert_eq!(s.hand(P1).len(), 1 + 1 - 1 + 4 + 1);
            let radiant: Vec<CardInstance> =
                s.hand(P1).into_iter().filter(|card| card.def_id == STOCKPILE && card.radiant).collect();
            assert_eq!(radiant.len(), 1, "the copy keeps the face");
        }
    }
}
