//! M #49 YileGPT Tamed (SPEC §8.8 row 49): (2) Unit, CN, Acclaimed, Mythic, 2/6 → 4/12.
//!
//! Base:    "Cry: Summon a Yile's Virus for your opponent across from this.
//!           Death: Put a YileGPT Unleashed on the bottom of your deck."
//! Radiant: "Cry: Summon a Radiant Yile's Virus for your opponent across from this.
//!           Death: Put a Radiant YileGPT Unleashed on the bottom of your deck."
//! Engine: the Cry is an aimed summon of M #49.2 for the opponent into the unit zone across from
//! this (§3.2, R47): they own and control it (R12), and it fails if that zone is occupied. The
//! Death puts a fresh M #49.1 on the bottom of the deck of the player who controlled this as it
//! died (ME-DECK-BOTTOM, never shuffled): R80's cap applies, and its owner's own-library list
//! shows it (R310).

use jackioh_engine::effects::{shuffle_into, summon};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-049";

/// M #49.2 Yile's Virus, summoned for the opponent.
const VIRUS: &str = "meditative-049-2";
/// M #49.1 YileGPT Unleashed, put on the bottom of the deck.
const UNLEASHED: &str = "meditative-049-1";

/// The unit zone across from this: its lane, read as the Cry resolves.
fn across(ctx: &EffectContext) -> Option<i32> {
    match ctx.live_self().map(|me| &me.zone) {
        Some(Zone::Field { lane, .. }) => Some(*lane),
        _ => None,
    }
}

fn tamed(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let Some(lane) = across(ctx) else {
                return vec![];
            };
            vec![summon(json_as(json!({
                "defId": VIRUS,
                "player": "enemy",
                "lane": lane,
                "radiant": radiant,
            })))]
        })),
        death: Some(hook(move |_ctx| {
            vec![shuffle_into(json_as(json!({
                "defId": UNLEASHED,
                "count": 1,
                "player": "self",
                "radiant": radiant,
                "position": "bottom",
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: tamed(false),
        radiant: tamed(true),
    }
}

// M #49 YileGPT Tamed — SPEC §8.8 row 49, BUILD M10 row M 49: "Cry (played or cast): a base Yile's
// Virus summoned for the opponent, owned by them, into the unit zone across from it; with that zone
// occupied, none; Death: a fresh base YileGPT Unleashed put on the bottom of its controller's deck,
// never shuffled, refused by a full deck (R80), shown in its owner's list (R310); radiant 4/12,
// both tokens Radiant".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TAMED: &str = "meditative-049";
    const VIRUS: &str = "meditative-049-2";
    const UNLEASHED: &str = "meditative-049-1";
    const BIGOT: &str = "core-002"; // 6/1, the killer.
    const OCCUPIER: &str = "core-012"; // 3/4, holding the zone across.
    const FILLER: &str = "core-005";

    /// p1 plays Tamed into `lane`; p2 holds lane 1, so lane 2 across is open unless stated.
    fn played(seed: &str, radiant: bool, lane: i32) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": TAMED, "radiant": radiant }, FILLER],
                "library": [FILLER, FILLER, FILLER, FILLER],
                "mana": 2,
            },
            "p2": {
                "hand": [FILLER],
                "field": [{ "def": OCCUPIER, "lane": 1 }],
                "library": [FILLER, FILLER],
            },
        }));
        s.play(TAMED, json!({ "zone": lane }));
        s
    }

    /// p2's 6/1 kills the Tamed standing in lane 1 of p1's field.
    fn kill_tamed(s: &mut Scenario) {
        s.end_turn();
        s.attack(BIGOT, TAMED);
    }

    mod m49_yilegpt_tamed {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn cry_summons_a_virus_for_the_opponent_across_from_this() {
                let s = played("tamed-base", false, 2);
                // Across from lane 2: the opponent's unit zone 2 holds their Virus.
                let virus = s.unit(P2, 2).expect("the virus");
                assert_eq!(virus.def_id, VIRUS);
                assert_eq!(virus.owner, P2);
                assert_eq!(virus.controller, P2);
                assert!(!virus.radiant);
            }

            #[test]
            fn cry_with_that_zone_occupied_summons_none() {
                let mut s = scenario(json!({
                    "seed": "tamed-blocked",
                    "p1": {
                        "hand": [TAMED, FILLER],
                        "library": [FILLER, FILLER, FILLER, FILLER],
                        "mana": 2,
                    },
                    "p2": {
                        "hand": [FILLER],
                        "field": [{ "def": OCCUPIER, "lane": 2 }],
                        "library": [FILLER, FILLER],
                    },
                }));
                s.play(TAMED, json!({ "zone": 2 }));
                // The zone stayed its occupier's: no Virus across from this (R47).
                let holder = s.unit(P2, 2).expect("the occupier");
                assert_eq!(holder.def_id, OCCUPIER);
            }

            #[test]
            fn r80_death_puts_an_unleashed_on_the_bottom_of_its_controllers_deck() {
                let mut s = scenario(json!({
                    "seed": "tamed-death",
                    "p1": {
                        "field": [{ "def": TAMED, "lane": 1 }],
                        "library": [FILLER, FILLER],
                    },
                    "p2": {
                        "field": [{ "def": BIGOT, "lane": 1 }],
                        "hand": [FILLER],
                        "library": [FILLER, FILLER],
                    },
                }));
                let before: Vec<String> =
                    s.pile(P1, "library").iter().map(|card| card.id.clone()).collect();

                kill_tamed(&mut s);

                // A fresh Unleashed, last in the deck, never shuffled — the rest untouched.
                let after = s.pile(P1, "library");
                assert_eq!(after.len(), before.len() + 1);
                assert_eq!(
                    after[..before.len()].iter().map(|card| &card.id).collect::<Vec<_>>(),
                    before.iter().collect::<Vec<_>>()
                );
                let last = after.last().expect("a last card");
                assert_eq!(last.def_id, UNLEASHED);
                assert!(!last.radiant);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r310_radiant_a_radiant_virus_and_a_radiant_unleashed() {
                let s = played("tamed-radiant", true, 2);
                let virus = s.unit(P2, 2).expect("the virus");
                assert_eq!(virus.def_id, VIRUS);
                assert!(virus.radiant);

                let mut s = scenario(json!({
                    "seed": "tamed-radiant-death",
                    "p1": {
                        "field": [{ "def": TAMED, "radiant": true, "lane": 1 }],
                        "library": [FILLER, FILLER],
                    },
                    "p2": {
                        "field": [{ "def": BIGOT, "lane": 1 }],
                        "hand": [FILLER],
                        "library": [FILLER, FILLER],
                    },
                }));
                kill_tamed(&mut s);
                let after = s.pile(P1, "library");
                let last = after.last().expect("a last card");
                assert_eq!(last.def_id, UNLEASHED);
                assert!(last.radiant);
            }
        }
    }
}
