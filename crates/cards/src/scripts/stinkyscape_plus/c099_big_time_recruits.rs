//! SPEC §8.12 S+ #99 Big Time Recruits, a card of the Stinkyscape+ set (R1420, R3535000). Built by S08 (#3555).
//! Draw the 3 most expensive permanents and make them GIGA.  
//! \~\~\~  
//! Draw the 3 most expensive permanents and make them GIGA and Radiant.
//!
//! The library's non-Spell cards by R24's costs (R3555002): every card above the cut, then the cut's
//! group in order when it fits exactly, else a random tie-break off the match rng — which draws
//! nothing when there is no choice (R129). Each is its own named draw, dearest first (R135), given
//! GIGA in the hand by `drawFromLibrary`'s `keywords` rider; the Radiant face also makes it Radiant
//! (R1061). Fewer permanents draw what there is.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-099";

/// R3555002: the ids of the `draw` dearest permanents of the controller's library, dearest first.
/// Reads nothing but the library (CLAUDE.md rule 5) and draws from the rng only when a tie straddles
/// the cut (R129).
fn dearest_permanents(at: &mut EffectContext<'_>) -> Vec<String> {
    let mut permanents: Vec<(String, i32)> = zone_cards(at.state, at.controller, OffFieldZone::Library)
        .into_iter()
        .filter(|card| card_type_of(at.state, card) != CardType::Spell)
        .map(|card| {
            let cost = effective_cost(at.state, &card, Default::default());
            (card.id, cost)
        })
        .collect();
    // Stable, dearest first.
    permanents.sort_by_key(|entry| std::cmp::Reverse(entry.1));
    let n = param(&*at, "draw").max(0) as usize;
    if n == 0 || permanents.is_empty() {
        return Vec::new();
    }
    if permanents.len() <= n {
        return permanents.into_iter().map(|(id, _)| id).collect();
    }
    let cut = permanents[n - 1].1;
    let mut dearest: Vec<String> = permanents
        .iter()
        .filter(|(_, cost)| *cost > cut)
        .map(|(id, _)| id.clone())
        .collect();
    let group: Vec<String> = permanents
        .iter()
        .filter(|(_, cost)| *cost == cut)
        .map(|(id, _)| id.clone())
        .collect();
    let need = n - dearest.len();
    if group.len() <= need {
        dearest.extend(group);
    } else {
        dearest.extend(at.rng.shuffle(&group).into_iter().take(need));
    }
    dearest
}

fn big_time_recruits(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(|at: &mut EffectContext<'_>| dearest_permanents(at)),
                each: Arc::new(move |id| {
                    draw_from_library(json_as(json!({
                        "instanceId": id,
                        "keywords": [{ "kind": "GIGA" }],
                        "radiant": radiant,
                    })))
                }),
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: big_time_recruits(false),
        radiant: big_time_recruits(true),
    }
}

// S+ #99 Big Time Recruits — SPEC §8.12 row S+ 99 (R24, R129, R135, R1061, R1878003, R3555002).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{def_ids, weapon_board};

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-005";

    /// p1 holds the Recruits with this library, mana to cast it.
    fn casting(radiant: bool, seed: &str, library: Value) -> Scenario {
        weapon_board(
            seed,
            json!({ "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": library, "mana": 10 }),
            json!({}),
        )
    }

    fn giga_in_hand(s: &Scenario, id: &str) -> bool {
        s.card(id)
            .granted_keywords
            .iter()
            .any(|keyword| keyword.kind() == KeywordKind::Giga)
    }

    mod base {
        use super::*;

        #[test]
        fn r3555002_it_draws_the_three_dearest_permanents_and_makes_them_giga() {
            let mut s = casting(
                false,
                "s99-dearest",
                json!([
                    "stand-in (6) Spell",
                    "stand-in (5) 5/5",
                    "stand-in (4) 4/4",
                    "stand-in (0) Spell",
                    "stand-in (2) 2/2",
                    "stand-in (1) 1/1",
                ]),
            );
            s.play(ID, json!({}));
            assert!(s.state().pending.is_none());
            // The 5, the 4 and the 2: never a Spell, never the 1.
            assert_eq!(
                def_ids(&s.hand(P1)),
                vec![
                    FILLER.to_string(),
                    "stand-in (5) 5/5".to_string(),
                    "stand-in (4) 4/4".to_string(),
                    "stand-in (2) 2/2".to_string(),
                ]
            );
            for def in ["stand-in (5) 5/5", "stand-in (4) 4/4", "stand-in (2) 2/2"] {
                let id = s.hand(P1).iter().find(|card| card.def_id == def).unwrap().id.clone();
                assert!(giga_in_hand(&s, &id), "{def} is GIGA in the hand");
                assert!(!s.card(&id).radiant, "{def} is not Radiant");
            }
            s.expect_in_zone(ID, "graveyard");
        }

        #[test]
        fn r3555002_a_tie_at_the_cut_is_broken_at_random() {
            // The 5 and the 4, then two (3)s for the last place: a seed for each outcome.
            fn library() -> Value {
                json!([
                    "stand-in (5) 5/5",
                    "stand-in (4) 4/4",
                    "stand-in (3) 3/3",
                    "stand-in (3) 3/30",
                    "stand-in (1) 1/1",
                ])
            }
            fn draws(seed: &str, wanted: &str, unwanted: &str) -> bool {
                let mut s = casting(false, seed, library());
                s.play(ID, json!({}));
                let held = def_ids(&s.hand(P1));
                held.contains(&wanted.to_string()) && !held.contains(&unwanted.to_string())
            }
            let _ = seed_where("s99-tie", |seed| {
                draws(seed, "stand-in (3) 3/3", "stand-in (3) 3/30")
            });
            let _ = seed_where("s99-tie-mirror", |seed| {
                draws(seed, "stand-in (3) 3/30", "stand-in (3) 3/3")
            });
        }

        #[test]
        fn r3555002_fewer_permanents_draw_what_there_is() {
            let mut s = casting(false, "s99-fewer", json!(["stand-in (2) 2/2", "stand-in (6) Spell"]));
            s.play(ID, json!({}));
            assert_eq!(def_ids(&s.hand(P1)), vec![FILLER.to_string(), "stand-in (2) 2/2".to_string()]);
            let id = s.hand(P1).iter().find(|card| card.def_id == "stand-in (2) 2/2").unwrap().id.clone();
            assert!(giga_in_hand(&s, &id));
            s.expect_in_zone(ID, "graveyard");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3555002_they_are_also_radiant() {
            let mut s = casting(
                true,
                "s99-radiant",
                json!([
                    "stand-in (5) 5/5",
                    "stand-in (4) 4/4",
                    "stand-in (2) 2/2",
                    "stand-in (1) 1/1",
                ]),
            );
            s.play(ID, json!({}));
            for def in ["stand-in (5) 5/5", "stand-in (4) 4/4", "stand-in (2) 2/2"] {
                let id = s.hand(P1).iter().find(|card| card.def_id == def).unwrap().id.clone();
                assert!(giga_in_hand(&s, &id), "{def} is GIGA in the hand");
                assert!(s.card(&id).radiant, "{def} is Radiant");
            }
        }
    }
}
