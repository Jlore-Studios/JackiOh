//! #15 Me and Mr Token (SPEC §8.1): 1/1 → 2/2 Unit, Human, cost 1. Base "Cry: summon a Rush Token",
//! radiant "Cry: summon 3 Rush Tokens". §8's Engine cell is "Board full → fewer".
//!
//! The Rush Token is the §7 token `core-t-rush` (3/3, Rush), so the token's stats and keyword come
//! from its own catalog face and are never restated here.
//!
//! "Board full → fewer" needs no check of its own: R64 gives a laneless summon the leftmost empty,
//! unlocked, unreserved unit zone and `summon` fizzles silently when the row has none. So the radiant
//! face is three independent summons — with two zones free it makes two tokens and the third does
//! nothing, which is exactly what the row asks for. Nothing about the Cry is conditional, so no hook
//! reads state at all.

use jackioh_engine::effects::summon;
use jackioh_engine::prelude::*;

pub const ID: &str = "core-015";

/// §7: the Rush Token def.
const RUSH_TOKEN: &str = "core-t-rush";

/// One Rush Token into the leftmost free unit zone of the controller's row (R64).
fn rush_token() -> Effect {
    summon(json_as(json!({ "defId": RUSH_TOKEN, "player": "self" })))
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|_ctx| vec![rush_token()])),
        ..Script::default()
    };
    // "Summon 3 Rush Tokens": the declared number `tokens` (R386), tuned on the Radiant face only — the
    // base face's "a Rush Token" is 1 and never moves (R749).
    let radiant = Script {
        cry: Some(hook(|ctx| (0..param(&*ctx, "tokens")).map(|_| rush_token()).collect())),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// #15 Me and Mr Token (SPEC §8.1, BUILD M4-T4 row 15): "1 Rush Token; radiant 3; fewer when the
// board is nearly full". The "fewer" clause is R64: a laneless summon takes the leftmost empty,
// unlocked, unreserved unit zone and fizzles silently when the row has none.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const RUSH_TOKEN: &str = "core-t-rush";

    /// Make the hand copy radiant, then play it.
    fn play_radiant<'a>(s: &'a mut Scenario, card: &str) -> &'a mut Scenario {
        let id = s.card(card).id.clone();
        match find_instance_mut(s.state_mut(), &id) {
            Some(instance) => instance.radiant = true,
            None => panic!("no instance {id}"),
        }
        s.play(card, json!({}))
    }

    /// The controller's unit row as def ids, lane 1 to 5, with `None` for an empty zone.
    fn row(s: &Scenario, player: PlayerId) -> Vec<Option<String>> {
        (1..=5).map(|lane| s.unit(player, lane).map(|unit| unit.def_id)).collect()
    }

    fn token_count(s: &Scenario, player: PlayerId) -> usize {
        row(s, player)
            .iter()
            .filter(|def_id| def_id.as_deref() == Some(RUSH_TOKEN))
            .count()
    }

    /// `["core-015", RUSH_TOKEN, null, …]` as the row's Rust shape.
    fn ids(row: &[Option<&str>]) -> Vec<Option<String>> {
        row.iter().map(|id| id.map(str::to_string)).collect()
    }

    mod n15_me_and_mr_token {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn summons_one_rush_token_into_the_next_free_zone_r64() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": ["core-015", "core-005"], "library": ["core-005"] } }));

                s.play("core-015", json!({}));

                assert_eq!(
                    row(&s, PlayerId::P1),
                    ids(&[Some("core-015"), Some(RUSH_TOKEN), None, None, None])
                );
                // §7: the token's stats and its Rush come from its own catalog face, never from this card.
                s.expect_stats("core-015", json!({ "attack": 1, "health": 1 }));
                let token = s.card(RUSH_TOKEN).clone();
                s.expect_stats(&token, json!({ "attack": 3, "health": 3 }));
            }

            #[test]
            fn summons_nothing_extra_when_the_unit_itself_takes_the_last_zone_r64() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "field": ["core-008", "core-008", "core-008", "core-008"],
                        "hand": ["core-015", "core-005"],
                        "library": ["core-005"]
                    }
                }));

                s.play("core-015", json!({}));

                assert_eq!(
                    row(&s, PlayerId::P1),
                    ids(&[Some("core-008"), Some("core-008"), Some("core-008"), Some("core-008"), Some("core-015")])
                );
                assert_eq!(token_count(&s, PlayerId::P1), 0);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn summons_3_rush_tokens() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": ["core-015", "core-005"], "library": ["core-005"] } }));

                play_radiant(&mut s, "core-015");

                assert_eq!(
                    row(&s, PlayerId::P1),
                    ids(&[Some("core-015"), Some(RUSH_TOKEN), Some(RUSH_TOKEN), Some(RUSH_TOKEN), None])
                );
                s.expect_stats("core-015", json!({ "attack": 2, "health": 2 }));
                assert_eq!(token_count(&s, PlayerId::P1), 3);
            }

            #[test]
            fn r64_makes_fewer_tokens_when_the_board_is_nearly_full_and_the_extras_fizzle_silently() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "field": ["core-008", "core-008", "core-008"],
                        "hand": ["core-015", "core-005"],
                        "library": ["core-005"]
                    }
                }));

                play_radiant(&mut s, "core-015");

                // One zone was left after the unit itself landed, so one token; the other two summons
                // found no zone and did nothing — no error, no burn, no replacement.
                assert_eq!(
                    row(&s, PlayerId::P1),
                    ids(&[Some("core-008"), Some("core-008"), Some("core-008"), Some("core-015"), Some(RUSH_TOKEN)])
                );
                assert_eq!(token_count(&s, PlayerId::P1), 1);
            }

            #[test]
            fn r386_an_upgrade_summons_4_tokens_and_a_degrade_2() {
                for (upgrade, tokens) in [(true, 4), (false, 2)] {
                    crate::register_all();
                    let mut s = scenario(json!({
                        "p1": { "hand": [{ "def": "core-015", "radiant": true }, "core-005"], "library": ["core-005"] }
                    }));
                    let moved = if upgrade {
                        crate::upgrade_number(&mut s, "core-015", "tokens")
                    } else {
                        crate::degrade_number(&mut s, "core-015", "tokens")
                    };
                    assert_eq!(moved, tokens);
                    s.play("core-015", json!({}));
                    assert_eq!(token_count(&s, PlayerId::P1), tokens as usize);
                }
            }
        }

        #[test]
        fn r749_the_base_face_s_one_token_is_never_tuned() {
            crate::register_all();
            let s = scenario(json!({ "p1": { "hand": ["core-015"] } }));
            assert!(!crate::can_upgrade_number(&s, "core-015", "tokens"));
            assert!(!crate::can_degrade_number(&s, "core-015", "tokens"));
        }
    }
}
