//! M #28.1 Ancient Curse (SPEC §8.8 row 28.1, §7; R40, R58, R70, R346, R549, R635, R748).
//!
//! Base:    "Cast on draw: Take {damage} damage."
//! Radiant: "Pierce\nCast on draw: Take {damage} damage."
//! Engine: §6.2 Cast on draw (`static_flags.cast_on_draw`), which is a cast (R40, R70). It deals one
//! `damage` instance from this card to its caster's own hero, through §4.4, with Armor applying.
//! "Take" names the caster's hero, as §7 reads CN-Virus. The caster is whoever drew it (R549 covers a
//! card drawn out of the other deck). The Radiant face prints Pierce: under R346 a Spell's printed
//! Pierce skips the Armor step, read off the resolving face — so no `ignoreArmor` is stated, and a
//! Nerf that removes the keyword is respected. After the cast the draw repeats (§2.4) under R58's
//! chain cap. Setup never deals it (R635, R748).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-028-1";

fn curse() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(|ctx| {
            vec![damage(json_as(json!({
                "to": { "of": "selfHero" },
                "amount": param(&*ctx, "damage"),
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The Radiant face's Pierce is catalog data the damage pipeline reads (R346).
    let base = curse();
    let radiant = curse();
    CardScripts { base, radiant }
}

// M #28.1 Ancient Curse — SPEC §8.8 row 28.1, BUILD M10 row M 28.1: "Cast on draw when its holder
// draws it (R40), out of either deck (R549): one hit of 7 on its caster's own hero, Armor reducing
// it, then the draw repeats (§2.4) under R58's cap; never dealt at setup (R635, R748); uncast in a
// hand (past the chain cap) it can be played for (2) with the same hit; damage reads through
// `param()`; radiant Pierce: the hit skips Armor (R346)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const CURSE: &str = "meditative-028-1";
    const RESOURCES: &str = "classic-058"; // (2) Field Spell: start of turn, draw from the bottom of the opponent's deck.
    const FILLER: &str = "core-005";

    /// One `shuffledIn`-style helper is not needed here; the curse is dealt by setup, not shuffled.
    /// A pile of harmless ordinary cards, for a library whose length is the point.
    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    /// The `damage` events on `target` ("hero-p1"), as amounts.
    fn damage_to(s: &Scenario, target: &str) -> Vec<i32> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { target_id, amount, .. } if target_id == target => Some(*amount),
                _ => None,
            })
            .collect()
    }

    /// The curse on top of its owner's library, so the next draw casts it.
    fn cursed_top(radiant_face: bool, armor: i32) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": if radiant_face { "curse-radiant" } else { "curse-drawn" },
            "p1": {
                "hand": [FILLER],
                "library": [{ "def": CURSE, "radiant": radiant_face }, FILLER, FILLER, FILLER],
                "armor": armor,
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    mod m28_1_ancient_curse {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn drawn_it_casts_and_hits_its_drawer_for_7_through_armor_then_the_draw_repeats() {
                let mut s = cursed_top(false, 3);

                s.start_turn();

                // One cast of the base face: 7 through 3 Armor is 4. Armor is a standing
                // stat, not spent by the hit it softens.
                assert_eq!(damage_to(&s, "hero-p1"), vec![4]);
                assert_eq!(s.state().players[P1].hero.health, 26);
                assert_eq!(s.state().players[P1].hero.armor, 3);
                // The curse is in the graveyard and the draw repeated onto the next card.
                s.expect_in_zone(CURSE, "graveyard");
                assert!(s.hand(P1).iter().any(|card| card.def_id == FILLER));
            }

            #[test]
            fn r549_drawn_from_the_opponent_s_deck_by_c_58_common_resources_it_hits_its_drawer() {
                crate::register_all();
                // p1's Resources face-up in the backrow, on p2's turn; the curse at the bottom of
                // p2's library, which is what Resources draws (C #58's shape).
                let mut s = scenario(json!({
                    "seed": "curse-r549",
                    "active": "p2",
                    "p1": {
                        "hand": [FILLER],
                        "backrow": [{ "def": RESOURCES, "faceUp": true }],
                        "library": filler(2),
                    },
                    "p2": { "hand": [FILLER], "library": [FILLER, CURSE] },
                }));

                s.end_turn();

                // The draw is p1's, so the cast is p1's and "take 7 damage" is p1's hero's.
                assert!(
                    s.events().iter().map(crate::js).any(|event| event["type"] == json!("damage")
                        && event["targetId"] == json!("hero-p1")
                        && event["amount"] == json!(7))
                );
                assert_eq!(s.state().players[P1].hero.health, 23);
                s.expect_in_zone(CURSE, "graveyard");
                assert_eq!(s.card(CURSE).owner, P1);
            }

            #[test]
            fn played_from_hand_for_2_it_deals_the_same_hit() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "curse-played",
                    "p1": { "hand": [CURSE, FILLER], "library": filler(3) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));

                s.play(CURSE, json!({}));

                assert_eq!(damage_to(&s, "hero-p1"), vec![7]);
                assert_eq!(s.state().players[P1].hero.health, 23);
                s.expect_in_zone(CURSE, "graveyard");
                s.expect_mana(P1, 2);
            }

            #[test]
            fn has_the_cast_on_draw_flag_on_both_faces() {
                // R635's guarantee that setup never deals it rests on this flag; a token is in no
                // deck anyway, so the flag is what makes a shuffled-in curse cast on draw.
                let scripts = script();
                for face in [&scripts.base, &scripts.radiant] {
                    assert_eq!(
                        face.static_flags.as_ref().and_then(|flags| flags.cast_on_draw),
                        Some(true)
                    );
                }
            }

            #[test]
            fn damage_reads_through_param() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "curse-param",
                    "p1": {
                        "hand": [FILLER],
                        "library": [CURSE, FILLER, FILLER, FILLER],
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                set_param(s.card_mut(CURSE), "damage", 3);

                s.start_turn();

                assert_eq!(damage_to(&s, "hero-p1"), vec![3]);
                assert_eq!(s.state().players[P1].hero.health, 27);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r346_the_printed_pierce_skips_armor() {
                let mut s = cursed_top(true, 3);

                s.start_turn();

                // The full 7 lands and the Armor is untouched.
                assert_eq!(damage_to(&s, "hero-p1"), vec![7]);
                assert_eq!(s.state().players[P1].hero.health, 23);
                assert_eq!(s.state().players[P1].hero.armor, 3);
                s.expect_in_zone(CURSE, "graveyard");
            }
        }
    }
}
