//! SPEC §8.10 Stinkyscape 40 Poison Capsule, a card of the Stinkyscape set.
//! (3) Unit, Plague, Rare, 2/2 → 4/4: Starship Piece.
//! "Cry: Place {counters|Plague Counter|Plague Counters}." (counters 2, Radiant 4) /
//! "End of turn: Deal {damage} damage to all enemies for each Plague Counter on this."
//! (damage 1; R1860001, R1860002).
//!
//! first if they have none (R1852000). Cry (R1): R689's placement, `place_plague_tokens`: the
//! counters all on the one permanent a single prompt names, either side, face-down cards included,
//! this one among them; on another permanent they do nothing for it. End of turn (R62): one hit of
//! damage × the counters on it to the enemy hero and to every enemy Unit (`damage_all`, R59), none
//! with no counter on it. Tagged Plague by §5's rule for a card that uses Plague Counters
//! (R1857001). The Radiant face runs the same script: its numbers are the declared `counters` and
//! `damage` (R386).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-040";

/// The Plague Counters on the running card now (its last-known ones on a Death-like read, R78).
fn plague_on(ctx: &EffectContext) -> i32 {
    ctx.self_
        .as_ref()
        .and_then(|me| me.counters.plague)
        .unwrap_or(0)
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![place_plague_tokens(json_as(json!({
                "count": param(ctx, "counters"),
            })))]
        })),
        end_of_turn: Some(hook(|ctx| {
            let amount = plague_on(ctx) * param(ctx, "damage");
            if amount <= 0 {
                return vec![];
            }
            vec![damage_all(json_as(json!({
                "side": "enemy",
                "heroes": true,
                "amount": amount,
            })))]
        })),
        ..Script::default()
    };
    // One script: the Radiant face's 4 counters are its declared `counters`, which `param` reads
    // off the running face; its stats are the catalog's.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Stinkyscape #40 Poison Capsule — SPEC §8.10 row 40, BUILD M12 row S 40 (R689).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;
    use crate::filler;

    const CAPSULE: &str = "stinkyscape-040";
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).

    fn hits_from(s: &Scenario, source: &str) -> Vec<i32> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage {
                    source_id: Some(from),
                    amount,
                    ..
                } if from == source => Some(*amount),
                _ => None,
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r1860001_cry_places_2_counters_on_the_one_permanent_a_single_prompt_names() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [CAPSULE, ANCHOR], "mana": 10, "library": filler(4) },
                "p2": { "hand": [ANCHOR], "field": [MENACE], "library": filler(4) },
            }));
            s.play(CAPSULE, json!({ "zone": 1 }));
            let menace = s.card(MENACE).clone();
            assert!(s.state().pending.is_some(), "one prompt names the permanent");

            s.answer(json!(menace.id));

            assert!(s.state().pending.is_none());
            assert_eq!(s.card(&menace).counters.plague, Some(2));
            assert_eq!(s.card(CAPSULE).counters.plague, None);
        }

        #[test]
        fn r1860001_this_card_is_a_choice_among_the_rest() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [CAPSULE, ANCHOR], "mana": 10, "library": filler(4) },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            s.play(CAPSULE, json!({ "zone": 1 }));
            let me = s.card(CAPSULE).clone();

            s.answer(json!(me.id));

            assert_eq!(s.card(CAPSULE).counters.plague, Some(2));
        }

        #[test]
        fn r1860002_end_of_turn_one_hit_of_counters_times_1_to_each_enemy() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [ANCHOR],
                    "field": [{ "def": CAPSULE, "lane": 1, "counters": { "plague": 2 } }],
                    "library": filler(4),
                },
                "p2": {
                    "hand": [ANCHOR],
                    "field": [{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 2 }],
                    "library": filler(4),
                },
            }));
            let me = s.card(CAPSULE).id.clone();
            s.end_turn();

            assert_eq!(hits_from(&s, &me), vec![2, 2, 2]);
            s.expect_health(PlayerId::P2, 28);
            assert_eq!(s.unit(PlayerId::P2, 1).expect("lane 1").damage, 2);
            assert_eq!(s.unit(PlayerId::P2, 2).expect("lane 2").damage, 2);
        }

        #[test]
        fn r1860002_with_no_counter_on_it_nothing() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "field": [CAPSULE], "library": filler(4) },
                "p2": { "hand": [ANCHOR], "field": [VANILLA], "library": filler(4) },
            }));
            let me = s.card(CAPSULE).id.clone();
            s.end_turn();

            assert!(hits_from(&s, &me).is_empty());
            s.expect_health(PlayerId::P2, 30);
            assert_eq!(s.unit(PlayerId::P2, 1).expect("lane 1").damage, 0);
        }

        #[test]
        fn r1860002_counters_on_another_permanent_add_nothing() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [ANCHOR],
                    "field": [
                        { "def": CAPSULE, "lane": 1, "counters": { "plague": 2 } },
                        { "def": VANILLA, "lane": 2, "counters": { "plague": 9 } },
                    ],
                    "library": filler(4),
                },
                "p2": { "hand": [ANCHOR], "field": [VANILLA], "library": filler(4) },
            }));
            let me = s.card(CAPSULE).id.clone();
            s.end_turn();

            assert_eq!(hits_from(&s, &me), vec![2, 2]);
            s.expect_health(PlayerId::P2, 28);
        }

        #[test]
        fn counters_and_damage_read_through_param() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [CAPSULE, ANCHOR], "mana": 10, "library": filler(4) },
                "p2": { "hand": [ANCHOR], "field": [VANILLA], "library": filler(4) },
            }));
            let counters = crate::upgrade_number(&mut s, CAPSULE, "counters");
            let damage = crate::upgrade_number(&mut s, CAPSULE, "damage");
            s.play(CAPSULE, json!({ "zone": 1 }));
            let me = s.card(CAPSULE).id.clone();
            s.answer(json!(me));
            assert_eq!(s.card(CAPSULE).counters.plague, Some(counters));

            s.end_turn();
            assert_eq!(hits_from(&s, &me), vec![counters * damage; 2]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1860001_radiant_cry_places_4_counters() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": CAPSULE, "radiant": true }, ANCHOR],
                    "mana": 10,
                    "library": filler(4),
                },
                "p2": { "hand": [ANCHOR], "field": [MENACE], "library": filler(4) },
            }));
            s.play(CAPSULE, json!({ "zone": 1 }));
            let menace = s.card(MENACE).clone();

            s.answer(json!(menace.id));

            assert_eq!(s.card(&menace).counters.plague, Some(4));
        }

        #[test]
        fn r1860002_radiant_end_of_turn_one_hit_of_counters_times_1_to_each_enemy() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [ANCHOR],
                    "field": [{ "def": CAPSULE, "radiant": true, "lane": 1, "counters": { "plague": 4 } }],
                    "library": filler(4),
                },
                "p2": { "hand": [ANCHOR], "field": [VANILLA], "library": filler(4) },
            }));
            let me = s.card(CAPSULE).id.clone();
            s.end_turn();

            assert_eq!(hits_from(&s, &me), vec![4, 4]);
            s.expect_health(PlayerId::P2, 26);
        }
    }
}
