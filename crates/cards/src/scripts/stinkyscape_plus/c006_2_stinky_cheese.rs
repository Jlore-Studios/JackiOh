//! SPEC §8.12 S+ #6.2 Stinky Cheese, a card of the Stinkyscape+ set (R1420, R3535000). (1) Field Spell,
//! Swarm, Token (printed Epic).
//!   Base:    "Temporary 2. Evolves when this is discarded to Temporary. Evolve: Gain Flashback, Animated, and
//!            +{buff}/+{buff}." — 5.
//!   Radiant: "Temporary 1 …" — +{buff2}/+{buff2}, 10.
//!
//! R3566004: Temporary discards it (the base face at the end of its controller's second turn holding it,
//! the Radiant face's plain Temporary at the first) and says so on the `Discarded` event (R3536001), which
//! the card answers in its graveyard with an `evolve` of itself. The Evolve hook gives Flashback and
//! Animated, then a 0/0 stat line (`give_stat_line`: a buff never makes one, R2280000) and the buff on it,
//! so its Flashback play makes it a {buff}/{buff} Unit. Another discard (a hand attack, a Discard effect)
//! does not evolve it.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-006-2";

/// The line the Evolve gives a card with none: the buff makes up all of its stats.
const BARE: i32 = 0;

fn cheese(buff_key: &'static str) -> Script {
    Script {
        graveyard_triggers: vec![TriggerDef::new(
            "stinky-cheese-evolves",
            &[GameEventType::Discarded],
            |ctx, event| {
                let GameEvent::Discarded {
                    instance_id,
                    by_temporary,
                    ..
                } = event
                else {
                    return vec![];
                };
                let own = ctx.self_.as_ref().is_some_and(|card| card.id == *instance_id);
                if own && *by_temporary == Some(true) {
                    vec![evolve(json_as(json!({ "target": { "of": "self" } })))]
                } else {
                    vec![]
                }
            },
        )],
        evolve: Some(hook(move |ctx| {
            let own = json!({ "of": "self" });
            let n = param(ctx, buff_key);
            vec![
                grant_keyword(json_as(
                    json!({ "target": own, "keyword": { "kind": "Flashback" } }),
                )),
                grant_keyword(json_as(
                    json!({ "target": own, "keyword": { "kind": "Animated" } }),
                )),
                give_stat_line(json_as(json!({ "target": own, "attack": BARE, "health": BARE }))),
                buff(json_as(json!({ "target": own, "attack": n, "health": n }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: cheese("buff"),
        radiant: cheese("buff2"),
    }
}

// S+ #6.2 Stinky Cheese — SPEC §8.12 row 6.2 (R3566004).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";
    const HINDER: &str = "core-021"; // Hinder: a discard that is not Temporary's

    fn holding(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": filler(12) },
            "p2": { "hand": [FILLER], "library": filler(12) },
        }))
    }

    fn id_in_hand(s: &Scenario) -> String {
        s.hand(P1)[0].id.clone()
    }

    /// A play of the graveyard card, sent to `reduce` as `legalActions` offers it (the harness's `play`
    /// takes a card from a hand).
    fn flashed_back(s: &mut Scenario, id: &str) {
        let action = json_as::<Action>(json!({
            "type": "play",
            "playerId": "p1",
            "instanceId": id,
            "nonce": format!("cheese-flashback-{id}"),
        }));
        let result = reduce(s.state(), &action);
        assert_eq!(result.error, None, "the Flashback play was refused");
        *s.state_mut() = result.state;
    }

    fn evolved_in_the_graveyard(s: &Scenario, id: &str) -> bool {
        s.card(id).zone.z() == ZoneName::Graveyard && s.card(id).evolved == Some(true)
    }

    #[test]
    fn r3566004_base_is_discarded_after_two_turns_and_evolves() {
        let mut s = holding(false);
        let id = id_in_hand(&s);
        s.end_turn().end_turn();
        assert_eq!(s.card(&id).zone.z(), ZoneName::Hand, "one turn held: still there");
        s.end_turn();
        assert!(
            evolved_in_the_graveyard(&s, &id),
            "the second turn's end discards and evolves it"
        );
        let keywords = s.stats(&id).keywords;
        assert!(has_keyword(&keywords, KeywordKind::Flashback));
        assert!(has_keyword(&keywords, KeywordKind::Animated));
        assert_eq!(
            s.card(&id).stats_override,
            Some(AttackHealth { attack: 0, health: 0 })
        );
        assert_eq!(
            s.card(&id).buffs,
            AttackHealth { attack: 5, health: 5 },
            "the buff is on it"
        );
    }

    #[test]
    fn r3566004_flashed_back_it_is_a_5_5_unit() {
        let mut s = holding(false);
        let id = id_in_hand(&s);
        s.end_turn().end_turn().end_turn().end_turn();
        assert!(evolved_in_the_graveyard(&s, &id));
        flashed_back(&mut s, &id);
        assert_ne!(s.card(&id).zone.z(), ZoneName::Graveyard, "it left the graveyard");
        s.expect_stats(id.as_str(), json!({ "attack": 5, "health": 5 }));
    }

    #[test]
    fn r3566004_radiant_goes_after_one_turn_with_plus_10() {
        let mut s = holding(true);
        let id = id_in_hand(&s);
        s.end_turn();
        assert!(
            evolved_in_the_graveyard(&s, &id),
            "Temporary 1: the first turn's end"
        );
        s.end_turn();
        flashed_back(&mut s, &id);
        s.expect_stats(id.as_str(), json!({ "attack": 10, "health": 10 }));
    }

    #[test]
    fn r3566004_another_discard_does_not_evolve_it() {
        // Core #21 Hinder discards a random card of its controller's hand: the Cheese, alone in it.
        let mut s = scenario(json!({
            "p1": { "hand": [HINDER, ID], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        let id = s
            .hand(P1)
            .iter()
            .find(|card| card.def_id == ID)
            .expect("the Cheese")
            .id
            .clone();
        s.play(HINDER, json!({}));
        assert_eq!(s.card(&id).zone.z(), ZoneName::Graveyard, "Hinder discarded it");
        assert_ne!(s.card(&id).evolved, Some(true));
    }
}
