//! #63 Plastic Surgery (SPEC §8.3, §6.1, R21, R78, R81, R90, R703).
//!
//! Base: "Target unit gets +3/+3 and 1 random keyword". Radiant: "+6/+6 and 2 random keywords",
//! which §8's Conventions read as changing only those numbers.
//!
//! §8.3's Engine cell is "Pool in 6.1", i.e. R21's eleven keywords. R21 also fixes the two
//! constraints on the draw: a unit never gets a keyword it already has, and one grant never repeats.
//! `grant_random_keywords` enforces both, so the pool is NOT restated here: a copy would be a second
//! source of truth (see #80 Zao Gao, the pool's other user).
//!
//! The buff (layer 4 of §10.4) and the grant are permanent on the instance and drop when the card
//! leaves the field (R78).
//!
//! R81: the target travels in the `play` action, so resolution never pauses. R703: the pick is
//! `required`, so with no legal target the card is not playable at all, where R90 would let it play
//! and fizzle. A cast still fizzles (R70, a cast is never refused), so both effects resolve
//! `{ of: "chosen" }` and do nothing when it is empty.

use jackioh_engine::effects::{buff, grant_random_keywords};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-063";

/// "Target unit": either side, §8's Conventions, and no narrowing in either cell.
fn targets() -> Vec<TargetDecl> {
    vec![
        // R656: a buff and a keyword help, so a random cast that targets enemies aims this at friends.
        // R703: and the play needs it, so no Unit to target means no play.
        json_as(json!({
            "kind": "target",
            "min": 1,
            "max": 1,
            "filter": { "side": "any", "of": ["unit"] },
            "aim": "help",
            "required": true,
        })),
    ]
}

/// The radiant face is the same card at doubled numbers, so one script carries both: the declared
/// numbers `buff` (3, 6) and `keywords` (1, 2), R386.
fn surgery() -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            let stat = param(&*ctx, "buff");
            let keywords = param(&*ctx, "keywords");
            vec![
                buff(json_as(json!({ "target": { "of": "chosen" }, "attack": stat, "health": stat }))),
                grant_random_keywords(json_as(json!({ "target": { "of": "chosen" }, "count": keywords }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = surgery();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// #63 Plastic Surgery — SPEC §8.3, BUILD M4-T4 row 63.
//
// Must-pass: "+3/+3 and one pool keyword the unit lacks (R21); radiant +6/+6 and two distinct
// keywords; not playable with no Unit to target, though a cast with none fizzles (R703)."
#[cfg(test)]
mod tests {
    use jackioh_engine::reduce::legal_actions;
    use jackioh_engine::resolve::cast_card;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const SURGERY: &str = "core-063"; // Spell, 1
    const FELINOR: &str = "core-t-felinor"; // 1/1 with no keywords at all: a clean slate for the grant
    const TIMMY: &str = "core-011"; // 3/3 with Rush and First Strike, both in the pool
    const MENACE: &str = "core-019"; // 9/9 with Taunt, in the pool

    /// R21's pool as keyword KINDS: "Armor 1" is its one numbered entry (§6.1). Read from
    /// `RANDOM_KEYWORD_POOL`, so this test never restates the eleven keywords (SPEC §6.1).
    fn pool_kinds() -> Vec<&'static str> {
        RANDOM_KEYWORD_POOL
            .iter()
            .map(|entry| if *entry == "Armor 1" { "Armor" } else { *entry })
            .collect()
    }

    fn sel(card: &CardInstance) -> Value {
        json!({ "pick": "instance", "instanceId": card.id })
    }

    fn granted_kinds(card: &CardInstance) -> Vec<String> {
        card.granted_keywords
            .iter()
            .map(|keyword| keyword.kind().as_str().to_string())
            .collect()
    }

    /// The target lists of every `play` `legal_actions` offers p1 for this card (R81).
    fn offered(state: &GameState, card: &CardInstance) -> Vec<Vec<String>> {
        legal_actions(state, P1)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Play {
                    instance_id, targets, ..
                } if instance_id == card.id => Some(
                    targets
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|target| match target {
                            Selection::Instance { instance_id } => Some(instance_id),
                            _ => None,
                        })
                        .collect(),
                ),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn r386_an_upgrade_gives_4_4_or_2_keywords_and_a_degrade_2_2_and_a_keyword_at_its_floor_of_1() {
        for (upgrade, key, stat, keywords) in [(true, "buff", 4, 1), (true, "keywords", 3, 2), (false, "buff", 2, 1)] {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [SURGERY], "field": [FELINOR], "mana": 4 } }));
            let felinor = s.card(FELINOR).clone();
            assert!(!crate::can_degrade_number(&s, SURGERY, "keywords"));
            if upgrade {
                crate::upgrade_number(&mut s, SURGERY, key);
            } else {
                crate::degrade_number(&mut s, SURGERY, key);
            }

            s.play(SURGERY, json!({ "targets": [sel(&felinor)] }));

            s.expect_stats(&felinor, json!({ "attack": 1 + stat, "maxHealth": 1 + stat }));
            assert_eq!(granted_kinds(s.card(&felinor)).len(), keywords);
        }
    }

    mod plastic_surgery {
        use super::*;

        #[test]
        fn gives_the_target_3_3_and_one_keyword_from_r21_s_pool() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [SURGERY], "field": [FELINOR], "mana": 4 } }));
            let felinor = s.card(FELINOR).clone();

            s.play(SURGERY, json!({ "targets": [sel(&felinor)] }));

            s.expect_stats(&felinor, json!({ "attack": 4, "maxHealth": 4 }));
            let kinds = granted_kinds(s.card(&felinor));
            assert_eq!(kinds.len(), 1);
            assert!(pool_kinds().contains(&kinds[0].as_str()));
            s.expect_events(json!(["cardPlayed", "buffed", "keywordGranted"]));
        }

        #[test]
        fn buffs_a_unit_on_either_side() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [SURGERY], "mana": 4 }, "p2": { "field": [FELINOR] } }));
            let felinor = s.card(FELINOR).clone();

            s.play(SURGERY, json!({ "targets": [sel(&felinor)] }));

            s.expect_stats(&felinor, json!({ "attack": 4, "maxHealth": 4 }));
        }

        #[test]
        fn r19_the_buff_raises_max_health_so_a_damaged_unit_keeps_its_damage() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [SURGERY], "field": [{ "def": MENACE, "damage": 4 }], "mana": 4 } }));
            let menace = s.card(MENACE).clone();
            s.expect_stats(&menace, json!({ "attack": 9, "maxHealth": 9, "health": 5 }));

            s.play(SURGERY, json!({ "targets": [sel(&menace)] }));

            s.expect_stats(&menace, json!({ "attack": 12, "maxHealth": 12, "health": 8 }));
        }

        #[test]
        fn r21_never_grants_a_keyword_the_unit_already_has() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [SURGERY], "field": [MENACE], "mana": 4 } }));
            let menace = s.card(MENACE).clone();

            s.play(SURGERY, json!({ "targets": [sel(&menace)] }));

            // Taunt is printed on #19 and is in the pool, so the one draw cannot be Taunt.
            assert!(!granted_kinds(s.card(&menace)).iter().any(|kind| kind == "Taunt"));
        }

        #[test]
        fn r21_radiant_gives_6_6_and_two_distinct_keywords() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [{ "def": SURGERY, "radiant": true }], "field": [FELINOR], "mana": 4 } }));
            let felinor = s.card(FELINOR).clone();

            s.play(SURGERY, json!({ "targets": [sel(&felinor)] }));

            s.expect_stats(&felinor, json!({ "attack": 7, "maxHealth": 7 }));
            let kinds = granted_kinds(s.card(&felinor));
            assert_eq!(kinds.len(), 2);
            assert_eq!(kinds.iter().collect::<IndexSet<_>>().len(), 2);
            for kind in &kinds {
                assert!(pool_kinds().contains(&kind.as_str()), "{kind} is not in R21's pool");
            }
        }

        #[test]
        fn r21_radiant_s_two_keywords_avoid_the_ones_the_unit_already_has() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [{ "def": SURGERY, "radiant": true }], "field": [TIMMY], "mana": 4 } }));
            let timmy = s.card(TIMMY).clone();

            s.play(SURGERY, json!({ "targets": [sel(&timmy)] }));

            let kinds = granted_kinds(s.card(&timmy));
            assert_eq!(kinds.len(), 2);
            assert!(!kinds.iter().any(|kind| kind == "Rush"));
            assert!(!kinds.iter().any(|kind| kind == "First Strike"));
        }

        #[test]
        fn s9_3_the_same_seed_draws_the_same_keywords() {
            crate::register_all();
            let run = || -> Vec<String> {
                let mut s = scenario(json!({ "seed": "surgery-63", "p1": { "hand": [SURGERY], "field": [FELINOR], "mana": 4 } }));
                let felinor = s.card(FELINOR).clone();
                s.play(SURGERY, json!({ "targets": [sel(&felinor)] }));
                granted_kinds(s.card(&felinor))
            };

            assert_eq!(run(), run());
        }

        #[test]
        fn r703_is_not_playable_with_no_unit_on_the_board_on_either_face_never_offered_and_refused() {
            crate::register_all();
            for radiant in [false, true] {
                let mut s = scenario(json!({ "p1": { "hand": [{ "def": SURGERY, "radiant": radiant }], "mana": 4 }, "p2": {} }));
                let surgery = s.card(SURGERY).clone();

                assert!(offered(s.state(), &surgery).is_empty());
                s.expect_refused_with(|s| s.play(SURGERY, json!({})), "cannot be played without 1 legal target");

                s.expect_in_zone(SURGERY, "hand");
                s.expect_mana(P1, 4);
            }
        }

        #[test]
        fn r703_is_offered_with_a_unit_on_either_side_once_per_unit_on_either_face() {
            crate::register_all();
            for radiant in [false, true] {
                let s = scenario(json!({
                    "p1": { "hand": [{ "def": SURGERY, "radiant": radiant }], "field": [FELINOR], "mana": 4 },
                    "p2": { "field": [MENACE] },
                }));

                assert_eq!(
                    offered(s.state(), s.card(SURGERY)),
                    vec![vec![s.card(FELINOR).id.clone()], vec![s.card(MENACE).id.clone()]]
                );
            }
        }

        #[test]
        fn r703_a_cast_with_no_unit_on_the_board_is_never_refused_r70_it_fizzles_and_still_counts_as_played() {
            crate::register_all();
            for radiant in [false, true] {
                let mut s = scenario(json!({ "p1": { "hand": [{ "def": SURGERY, "radiant": radiant }], "mana": 4 }, "p2": {} }));
                let mut events: Vec<GameEvent> = Vec::new();
                let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
                let surgery = s.card(SURGERY).clone();

                {
                    let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
                    cast_card(&mut sink, &surgery, Default::default());
                }

                assert!(s.state().pending.is_none());
                s.expect_in_zone(SURGERY, "graveyard");
                assert!(events.iter().any(|event| event.event_type() == GameEventType::CardPlayed));
            }
        }
    }
}
