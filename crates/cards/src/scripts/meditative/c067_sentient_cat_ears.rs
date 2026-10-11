//! M #67 Sentient Cat Ears (SPEC §8.8 row 67, R1086): (1) Unit, Felinor, Epic, 1/1 → 2/2.
//!
//! Base and Radiant: "Magnetic\nEnd of turn: Shuffle {bombs|Love Bomb|Love Bombs} into your deck."
//! Engine: ME-MAGNETIC (R1086) comes from the engine — the printed Magnetic keyword offers the play
//! onto your acting, non-Immutable Units, resolving on top before fusing into the host. The script
//! is the end-of-turn shuffle of a fresh Love Bomb (M #30.1), `param("bombs")` copies (1, radiant
//! 2), R80's cap turning a full library away.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-067";

/// M #30.1 Love Bomb, the token this card shuffles (§7).
const BOMB: &str = "meditative-030-1";

fn ears() -> Script {
    Script {
        end_of_turn: Some(hook(|ctx| {
            vec![shuffle_into(json_as(json!({
                "defId": BOMB,
                "count": param(&*ctx, "bombs"),
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // Both faces share the one script: only the declared `bombs` differs (1, radiant 2).
    let base = ears();
    let radiant = ears();
    CardScripts { base, radiant }
}

// M #67 Sentient Cat Ears — SPEC §8.8 row 67, BUILD M10 row M 67: "Played into an empty zone it is
// a plain Unit; played as Magnetic onto your acting non-Immutable Unit it resolves on top (its Cry
// included), then fuses into the host, which keeps its zone, damage and exertion and gains +1/+1,
// Felinor and the end-of-turn line (R1086), the Cat Ears ceasing to exist with no Death; an enemy
// or Immutable Unit is no host; at your end of turn only, one Love Bomb shuffled into your deck
// (R80's cap); bombs reads through `param()`; radiant 2/2 and two Love Bombs".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const EARS: &str = "meditative-067";
    /// (1) 4/4 Unit with no hooks: the Magnetic host.
    const VANILLA: &str = "core-008";
    /// (1) Spell, deals 3 to a target.
    const ECLIPSE: &str = "core-035";
    const FILLER: &str = "core-005";

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    fn library_def_ids(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.state().players[player].library.iter().map(|card| card.def_id.clone()).collect()
    }

    fn fused(s: &Scenario) -> bool {
        s.events().iter().any(|event| matches!(event, GameEvent::Fused { .. }))
    }

    mod m67_sentient_cat_ears {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn played_into_an_empty_zone_it_is_a_plain_unit() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "ears-plain",
                    "p1": { "hand": [EARS, FILLER], "library": filler(3) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                let id = s.card(EARS).id.clone();
                s.play(EARS, json!({}));

                assert_eq!(s.unit(P1, 1).map(|unit| unit.id), Some(id));
                assert!(!fused(&s));
            }

            #[test]
            fn r1086_onto_your_unit_fuses_felinor_and_the_line() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "ears-magnetic",
                    "p1": {
                        "hand": [EARS, FILLER],
                        "field": [VANILLA],
                        "library": filler(3),
                    },
                    "p2": {
                        "hand": [ECLIPSE, FILLER],
                        "library": filler(4),
                    },
                }));
                // Wound the host first: the fusion keeps the damage.
                s.end_turn();
                let host = s.unit(P1, 1).expect("the host").id.clone();
                s.play(ECLIPSE, json!({ "targets": [{ "pick": "instance", "instanceId": host }] }));
                s.end_turn();
                let ears = s.card(EARS).id.clone();
                s.play(EARS, json!({ "zone": 1, "magnetic": true }));

                // The host is kept in its zone with summed stats and kept damage (4+1 attack, 4+1
                // health less the 3 dealt), and the Ears are gone with no Death.
                assert!(fused(&s));
                let host_now = s.unit(P1, 1).expect("the fused host");
                assert_eq!(host_now.id, host);
                let stats = s.stats(host_now.id.as_str());
                assert_eq!((stats.attack, stats.health), (5, 2));
                assert!(s.events().iter().all(|event| !matches!(
                    event,
                    GameEvent::Destroyed { instance_id, .. } if instance_id == &ears
                )));
                // The fused host carries the end-of-turn line: a Love Bomb reaches the deck.
                s.end_turn();
                assert!(library_def_ids(&s, P1).contains(&BOMB.to_string()));
            }

            #[test]
            fn an_immutable_unit_is_no_host() {
                crate::register_all();
                // Keymaster Keenus (M #65) is Immutable: nothing fuses onto it.
                let mut s = scenario(json!({
                    "seed": "ears-immutable",
                    "p1": {
                        "hand": [EARS, FILLER],
                        "field": ["meditative-065"],
                        "library": filler(3),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.expect_refused(|s| s.play(EARS, json!({ "zone": 1, "magnetic": true })));
            }

            #[test]
            fn end_of_turn_shuffles_one_love_bomb() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "ears-bomb",
                    "p1": { "hand": [EARS, FILLER], "library": filler(3) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(EARS, json!({}));
                s.end_turn();

                assert_eq!(
                    library_def_ids(&s, P1).iter().filter(|id| *id == BOMB).count(),
                    1
                );
            }

            #[test]
            fn bombs_reads_through_param() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "ears-param",
                    "p1": { "hand": [EARS, FILLER], "library": filler(3) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                set_param(s.card_mut(EARS), "bombs", 3);
                s.play(EARS, json!({}));
                s.end_turn();

                assert_eq!(
                    library_def_ids(&s, P1).iter().filter(|id| *id == BOMB).count(),
                    3
                );
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn end_of_turn_shuffles_two_love_bombs() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "ears-radiant",
                    "p1": {
                        "hand": [{ "def": EARS, "radiant": true }, FILLER],
                        "library": filler(3),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(EARS, json!({}));
                let stats = s.stats(EARS);
                assert_eq!((stats.attack, stats.health), (2, 2));
                s.end_turn();

                assert_eq!(
                    library_def_ids(&s, P1).iter().filter(|id| *id == BOMB).count(),
                    2
                );
            }
        }
    }
}
