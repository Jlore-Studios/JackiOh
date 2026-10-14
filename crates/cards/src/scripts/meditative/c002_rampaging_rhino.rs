//! M #2 Rampaging Rhino (SPEC §8.8 row 2, R59, R63, R174, R212, R386, R682, R780, R800): (2) Unit, Common,
//! 5/9 → 11/20.
//!
//! Base:    "Whenever this takes damage, discard {discards|card|cards}."
//! Radiant: "Trample\nWhenever this takes damage, discard {discards|card|cards}."
//! Engine: a trigger on `damage` aimed at this card, as Core #91 Fed Fauci's, returning
//! `discard_random { count: discards, player: self }`. Every damage instance is one discard, and each hit
//! of a sweep is its own (R59). A hit Armor or a cap took whole, or a Divine Shield took, emits no
//! `damage` event (R63), so it costs nothing. The discard is random (R682) and comes from the hand of
//! whoever controls the Rhino as the hit lands (R212); an empty hand discards nothing and draws no random
//! number (R129); on the opponent's turn the discard is a forced one, which M #1 Disruptive Disruptor's
//! guard stops (R800). A hit that kills it discards nothing: the state check removes it before its trigger
//! pops, and a trigger of a card that has left the field ends there (R153, R174, R780). Trample is
//! catalog data, so the Radiant face's script is the base's.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-002";

/// "Whenever THIS takes damage": the event carries the target, so a hit this card dealt (its strike-back,
/// its Trample overflow) is not a hit it took.
fn is_hit_on_self(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    let Some(self_) = ctx.self_.as_ref() else {
        return false;
    };
    match event {
        GameEvent::Damage { target_id, .. } => *target_id == self_.id,
        _ => false,
    }
}

fn takes_damage() -> TriggerDef {
    TriggerDef::new("rampaging-rhino-discard", &[GameEventType::Damage], |ctx, event| {
        if is_hit_on_self(ctx, event) {
            vec![discard_random(json_as(json!({ "count": param(&*ctx, "discards"), "player": "self" })))]
        } else {
            vec![]
        }
    })
    .with_when(is_hit_on_self)
}

fn rampaging_rhino() -> Script {
    Script {
        triggers: vec![takes_damage()],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: rampaging_rhino(),
        radiant: rampaging_rhino(),
    }
}

// M #2 Rampaging Rhino — SPEC §8.8 row 2, BUILD M10 row M 2: "A trigger on `damage` aimed at it, one random
// discard per damage instance from the hand of whoever controls it as the hit lands (R212, R682); a sweep
// hits each Rhino once (R59); a hit Armor takes whole discards nothing (R63); an empty hand discards nothing
// and draws no random number (R129); a hit that kills it discards nothing (R174, R780); Trample on the
// Radiant face; discards reads through param() (R386)".
//
// Fixtures: #61 Prejudiced Postdoc (2/4, its Cry never fires when it is placed, R1) hits for 2 and dies to the
// Rhino's 5; #4 Gary the Gambler is a 1/1 striker and #8 Mr. Vanilla a 4/4; #13 Shredder-10 deals 2 to each
// enemy Unit at its end of turn; #10 Rapid Replenish is a (0) Spell, which fills a hand and keeps the acting
// side from auto-ending its turn (R82); #49 Snom Bunny Mind Control steals the Rhino and #35 Lunar Eclipse
// hits it for 3.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const POSTDOC: &str = "core-061";
    const GARY: &str = "core-004";
    const VANILLA: &str = "core-008";
    const SHREDDER: &str = "core-013";
    const FILLER: &str = "core-010";
    const STEAL: &str = "core-049";
    const ECLIPSE: &str = "core-035";

    /// p2 is to act with `enemies` on its field; p1 holds the Rhino and `hand` spare cards.
    fn board(radiant: bool, hand: usize, enemies: &[&str]) -> Scenario {
        crate::scenario(json!({
            "active": "p2",
            "p1": { "field": [{ "def": ID, "radiant": radiant }], "hand": vec![FILLER; hand], "library": [VANILLA, VANILLA] },
            "p2": { "field": enemies, "hand": [FILLER], "library": [VANILLA, VANILLA] },
        }))
    }

    /// How many of `owner`'s cards the game has discarded since setup.
    fn discarded(s: &Scenario, owner: PlayerId) -> usize {
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Discarded { owner: who, .. } if *who == owner))
            .count()
    }

    fn damage_events_on(s: &Scenario, id: &str) -> usize {
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Damage { target_id, .. } if target_id == id))
            .count()
    }

    fn at(unit: &CardInstance) -> Value {
        json!({ "targets": [{ "pick": "instance", "instanceId": unit.id }] })
    }

    /// The keywords the client sees on p1's lane-1 unit, by kind.
    fn kinds_of_p1_lane_1(s: &Scenario) -> Vec<String> {
        s.view(P1)
            .you
            .units
            .first()
            .cloned()
            .flatten()
            .map(|unit| unit.keywords.iter().map(|keyword| keyword.kind().to_string()).collect())
            .unwrap_or_default()
    }

    #[test]
    fn is_a_2_cost_5_9_declaring_discards() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.cost, CardCost::Fixed(2));
        assert_eq!(def.type_, CardType::Unit);
        assert_eq!(def.rarity, Rarity::Common);
        assert!(def.tags.is_empty());
        assert_eq!(
            [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
            [Some(5), Some(9), Some(11), Some(20)]
        );
        assert_eq!(
            crate::js(&def.params),
            json!([{ "key": "discards", "base": 1, "radiant": 1, "better": "down", "step": 1, "min": 1 }])
        );
        let scripts = super::script();
        assert_eq!(scripts.base.triggers.len(), 1);
        assert_eq!(scripts.radiant.triggers.len(), 1);
    }

    mod base {
        use super::*;

        #[test]
        fn r780_each_damage_instance_discards_one_random_card_from_its_controllers_hand() {
            let mut s = board(false, 3, &[POSTDOC]);
            s.attack(POSTDOC, ID);
            s.expect_stats(ID, json!({ "attack": 5, "health": 7, "maxHealth": 9 }));
            assert_eq!(s.hand(P1).len(), 2);
            assert_eq!(discarded(&s, P1), 1);
            assert_eq!(s.hand(P2).len(), 1);
            assert_eq!(discarded(&s, P2), 0);
        }

        #[test]
        fn r780_two_hits_are_two_discards() {
            let mut s = board(false, 3, &[POSTDOC, POSTDOC]);
            s.attack(POSTDOC, ID).attack(POSTDOC, ID);
            s.expect_stats(ID, json!({ "attack": 5, "health": 5, "maxHealth": 9 }));
            assert_eq!(s.hand(P1).len(), 1);
            assert_eq!(discarded(&s, P1), 2);
        }

        #[test]
        fn r780_r59_a_sweep_hits_each_rhino_once() {
            let mut s = crate::scenario(json!({
                "active": "p2",
                "p1": { "field": [ID, ID], "hand": [FILLER, FILLER, FILLER, FILLER], "library": [VANILLA, VANILLA] },
                "p2": { "field": [SHREDDER], "hand": [FILLER], "library": [VANILLA, VANILLA] },
            }));
            let rhinos: Vec<CardInstance> = [1, 2].iter().filter_map(|lane| s.unit(P1, *lane)).collect();
            s.end_turn();
            // Shredder's end of turn hits each Rhino once, and each hit is its own instance.
            for rhino in &rhinos {
                assert_eq!(damage_events_on(&s, &rhino.id), 1);
            }
            assert_eq!(discarded(&s, P1), 2);
            assert_eq!(discarded(&s, P2), 0);
        }

        #[test]
        fn r780_r63_a_hit_armor_takes_whole_discards_nothing() {
            // Defense Position grants Armor +1 (§4.1), so a 1-attack striker is reduced to 0 at §4.4 step 2.
            let mut s = crate::scenario(json!({
                "active": "p2",
                "p1": { "field": [{ "def": ID, "position": "DEF" }], "hand": [FILLER, FILLER, FILLER] },
                "p2": { "field": [GARY], "hand": [FILLER] },
            }));
            let rhino = s.card(ID).clone();
            s.attack(GARY, ID);
            assert_eq!(damage_events_on(&s, &rhino.id), 0);
            assert_eq!(discarded(&s, P1), 0);
            assert_eq!(s.hand(P1).len(), 3);
            s.expect_stats(ID, json!({ "attack": 5, "health": 9, "maxHealth": 9 }));
        }

        #[test]
        fn r780_an_empty_hand_discards_nothing_and_draws_no_random_number() {
            let mut s = board(false, 0, &[POSTDOC]);
            let cursor = s.state().rng_cursor;
            s.attack(POSTDOC, ID);
            assert_eq!(damage_events_on(&s, &s.card(ID).id.clone()), 1);
            assert_eq!(discarded(&s, P1), 0);
            assert_eq!(s.state().rng_cursor, cursor);
        }

        #[test]
        fn r780_a_hit_that_kills_it_discards_nothing() {
            let mut s = crate::scenario(json!({
                "active": "p2",
                "p1": { "field": [{ "def": ID, "damage": 8 }], "hand": [FILLER, FILLER, FILLER], "library": [VANILLA] },
                "p2": { "field": [POSTDOC], "hand": [FILLER], "library": [VANILLA] },
            }));
            let rhino = s.card(ID).clone();
            s.attack(POSTDOC, ID);
            s.expect_in_zone(&rhino, "graveyard");
            assert_eq!(discarded(&s, P1), 0);
            assert_eq!(s.hand(P1).len(), 3);
        }

        #[test]
        fn r780_r212_taken_by_the_other_player_its_new_controller_discards() {
            let mut s = crate::scenario(json!({
                "p1": { "hand": [STEAL, ECLIPSE, FILLER, FILLER, FILLER], "mana": 8, "library": [VANILLA, VANILLA] },
                "p2": { "field": [ID], "hand": [FILLER, FILLER], "library": [VANILLA, VANILLA] },
            }));
            let rhino = s.card(ID).clone();
            s.play(STEAL, at(&rhino));
            assert_eq!(s.card(&rhino).controller, P1);
            assert_eq!(discarded(&s, P1), 0);
            s.play(ECLIPSE, at(&rhino));
            s.expect_stats(&rhino, json!({ "attack": 5, "health": 6, "maxHealth": 9 }));
            // The hit landed on P1's Rhino, so P1 discards, and P2's hand is as it was.
            assert_eq!(discarded(&s, P1), 1);
            assert_eq!(s.hand(P1).len(), 2);
            assert_eq!(discarded(&s, P2), 0);
            assert_eq!(s.hand(P2).len(), 2);
        }

        #[test]
        fn r800_on_the_opponents_turn_a_disruptive_disruptor_stops_the_discard() {
            // M #1 on p1's side guards p1's hand while p2 is active: the hit lands, the discard does not.
            let mut s = crate::scenario(json!({
                "active": "p2",
                "p1": {
                    "field": [ID],
                    "backrow": [{ "def": "meditative-001", "faceUp": true, "lane": 1 }],
                    "hand": [FILLER, FILLER, FILLER],
                    "library": [VANILLA, VANILLA],
                },
                "p2": { "field": [POSTDOC], "hand": [FILLER], "library": [VANILLA, VANILLA] },
            }));
            let cursor = s.state().rng_cursor;
            s.attack(POSTDOC, ID);
            s.expect_stats(ID, json!({ "attack": 5, "health": 7, "maxHealth": 9 }));
            assert_eq!(discarded(&s, P1), 0);
            assert_eq!(s.hand(P1).len(), 3);
            assert_eq!(s.state().rng_cursor, cursor);
            assert!(
                s.events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::DiscardPrevented { player, count: 1 } if *player == P1))
            );
        }

        #[test]
        fn r780_the_strike_back_of_its_own_attack_discards_too() {
            let mut s = crate::scenario(json!({
                "p1": { "field": [ID], "hand": [FILLER, FILLER, FILLER], "library": [VANILLA, VANILLA] },
                "p2": { "field": [VANILLA], "hand": [FILLER], "library": [VANILLA, VANILLA] },
            }));
            s.attack(ID, VANILLA);
            // Vanilla's 4 comes back on the Rhino; the Rhino's 5 killed it.
            s.expect_stats(ID, json!({ "attack": 5, "health": 5, "maxHealth": 9 }));
            assert_eq!(discarded(&s, P1), 1);
            assert_eq!(s.hand(P1).len(), 2);
        }

        #[test]
        fn a_hit_it_deals_and_does_not_take_discards_nothing() {
            let mut s = crate::scenario(json!({
                "p1": { "field": [ID], "hand": [FILLER, FILLER, FILLER], "library": [VANILLA, VANILLA] },
                "p2": { "hand": [FILLER], "library": [VANILLA, VANILLA] },
            }));
            s.attack(ID, "hero");
            s.expect_health(P2, 25);
            assert_eq!(discarded(&s, P1), 0);
            assert_eq!(s.hand(P1).len(), 3);
        }

        #[test]
        fn r386_a_nerf_discards_2_per_hit() {
            let mut s = board(false, 3, &[POSTDOC]);
            assert_eq!(crate::degrade_number(&mut s, ID, "discards"), 2);
            s.attack(POSTDOC, ID);
            assert_eq!(discarded(&s, P1), 2);
            assert_eq!(s.hand(P1).len(), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_11_20_with_trample_its_excess_reaching_the_hero() {
            let mut s = crate::scenario(json!({
                "p1": { "field": [{ "def": ID, "radiant": true }], "hand": [FILLER, FILLER, FILLER], "library": [VANILLA, VANILLA] },
                "p2": { "field": [GARY], "hand": [FILLER], "library": [VANILLA, VANILLA] },
            }));
            s.expect_stats(ID, json!({ "attack": 11, "health": 20, "maxHealth": 20 }));
            assert_eq!(kinds_of_p1_lane_1(&s), vec!["Trample".to_string()]);
            s.attack(ID, GARY);
            // 11 into a 1/1: 1 to the Gary and the 10 left over to its hero.
            s.expect_health(P2, 20);
        }

        #[test]
        fn still_discards_one_per_hit() {
            let mut s = board(true, 3, &[POSTDOC]);
            s.attack(POSTDOC, ID);
            s.expect_stats(ID, json!({ "attack": 11, "health": 18, "maxHealth": 20 }));
            assert_eq!(discarded(&s, P1), 1);
            assert_eq!(s.hand(P1).len(), 2);
        }
    }
}
