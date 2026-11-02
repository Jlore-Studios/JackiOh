//! SPEC §8.10 Stinkyscape 43 Healing Capsule, a card of the Stinkyscape set.
//! (1) Unit, Rare, 2/2 → 4/4: Starship Piece.
//! "End of turn: Heal your hero and each of your Units {heal}." — heal 2, Radiant 4 (R1860003).
//!
//! first if they have none (R1852000). End of turn (R62): `heal` on its controller's hero, no cap
//! on a hero (§6.3 Heal), and on each of their Units, itself included, in lane order (R68) —
//! Classic+ #19.4 Support Loser's line. The Radiant face runs the same script: its heal is the
//! declared `heal` (R386).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-043";

pub fn script() -> CardScripts {
    let base = Script {
        end_of_turn: Some(hook(|ctx| {
            let amount = param(ctx, "heal");
            vec![
                heal(json_as(json!({ "target": { "of": "selfHero" }, "amount": amount }))),
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(|each: &mut EffectContext<'_>| {
                        cards_in_scope(each, &json_as(json!({ "side": "self" })))
                            .into_iter()
                            .map(|card| card.id)
                            .collect()
                    }),
                    each: Arc::new(move |instance_id: &str| {
                        heal(json_as(json!({
                            "target": { "of": "instance", "instanceId": instance_id },
                            "amount": amount,
                        })))
                    }),
                }),
            ]
        })),
        ..Script::default()
    };

    // One script: the Radiant heal 4 is the declared `heal`, which `param` reads off the running
    // face; its stats are the catalog's.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// Stinkyscape #43 Healing Capsule — SPEC §8.10 row 43, BUILD M12 row S 43.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;
    use crate::filler;

    const CAPSULE: &str = "stinkyscape-043";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).

    fn heals(s: &Scenario) -> Vec<i32> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Healed { amount, .. } => Some(*amount),
                _ => None,
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r1860003_end_of_turn_heals_your_hero_past_30_and_each_of_your_units_including_itself() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [ANCHOR],
                    "health": 29,
                    "field": [
                        { "def": CAPSULE, "lane": 1, "damage": 1 },
                        { "def": VANILLA, "lane": 2, "damage": 3 },
                    ],
                    "library": filler(4),
                },
                "p2": {
                    "hand": [ANCHOR],
                    "health": 20,
                    "field": [{ "def": VANILLA, "lane": 1, "damage": 3 }],
                    "library": filler(4),
                },
            }));
            s.end_turn();

            s.expect_health(PlayerId::P1, 31);
            assert_eq!(s.unit(PlayerId::P1, 1).expect("capsule").damage, 0);
            assert_eq!(s.unit(PlayerId::P1, 2).expect("vanilla").damage, 1);
            s.expect_health(PlayerId::P2, 20);
            assert_eq!(s.unit(PlayerId::P2, 1).expect("enemy").damage, 3);
            assert_eq!(heals(&s), vec![2, 1, 2]);
        }

        #[test]
        fn r1860003_undamaged_cards_take_no_heal() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "health": 30, "field": [CAPSULE], "library": filler(4) },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            s.end_turn();

            s.expect_health(PlayerId::P1, 32);
            assert_eq!(heals(&s), vec![2]);
        }

        #[test]
        fn heal_reads_through_param() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "health": 20, "field": [CAPSULE], "library": filler(4) },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            let heal = crate::upgrade_number(&mut s, CAPSULE, "heal");
            s.end_turn();

            s.expect_health(PlayerId::P1, 20 + heal);
            assert_eq!(heals(&s), vec![heal]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1860003_radiant_end_of_turn_heals_4() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [ANCHOR],
                    "health": 29,
                    "field": [
                        { "def": CAPSULE, "radiant": true, "lane": 1, "damage": 2 },
                        { "def": VANILLA, "lane": 2, "damage": 3 },
                    ],
                    "library": filler(4),
                },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            s.end_turn();

            s.expect_health(PlayerId::P1, 33);
            assert_eq!(s.unit(PlayerId::P1, 1).expect("capsule").damage, 0);
            assert_eq!(s.unit(PlayerId::P1, 2).expect("vanilla").damage, 0);
            assert_eq!(heals(&s), vec![4, 2, 3]);
        }
    }
}
