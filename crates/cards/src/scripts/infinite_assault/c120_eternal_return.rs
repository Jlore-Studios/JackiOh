//! SPEC §8.13 IA #120 Eternal Return ((4) Spell, Mythic), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Shuffle every card you've played this game into your deck. They cost ({discount}) less. Draw three cards." (discount 1)
//!   Radiant: "Shuffle every card you've played this game into your deck. They cost ({discount}) less. Draw four cards." (discount 2)
//!
//! Every entry of the controller's `played_this_game` log (R4589000) but this card's own (R4589002:
//! never itself) is shuffled into its deck with `shuffleInto`'s `costMod` of the declared `discount`
//! — a fresh copy on the face it was played with (R4577002), up to the library cap (R80, R316) —
//! then the draw (§2.4's pipeline). The discount is R65's floor: never below (0).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-120";

/// "Draw three cards", the base face's.
const DRAW: i32 = 3;
/// "Draw four cards", the Radiant face's.
const RADIANT_DRAW: i32 = 4;

fn eternal_return(draws: i32) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let view: &EffectContext = ctx;
            let discount = param(view, "discount");
            let mut effects: Vec<Effect> = played_this_game(&*view.state, view.controller)
                .into_iter()
                .filter(|record| record.def_id != ID)
                .map(|record| {
                    shuffle_into(json_as(json!({
                        "defId": record.def_id,
                        "count": 1,
                        "player": "self",
                        "radiant": record.radiant,
                        "costMod": -discount,
                    })))
                })
                .collect();
            effects.push(draw(json_as(json!({ "count": draws }))));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: eternal_return(DRAW),
        radiant: eternal_return(RADIANT_DRAW),
    }
}

// IA #120 Eternal Return — SPEC §8.13 row 120 (R4589002).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const SPELL: &str = "stand-in (0) Spell";
    const FILLER: &str = "stand-in (0) Spell";

    /// p1 casts two Spells, then Eternal Return, with IA #97 in the library to keep the log.
    fn returned(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "hand": [SPELL, SPELL, { "def": ID, "radiant": radiant }],
                "library": ["infiniteassault-097", FILLER, FILLER, FILLER, FILLER],
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        s.play(SPELL, json!({}));
        s.play(SPELL, json!({}));
        s.play(ID, json!({}));
        s
    }

    /// The shuffled copies: the played Spells, told from the library's fillers by their discount.
    fn shuffled(s: &Scenario) -> Vec<CardInstance> {
        s.pile(P1, "library")
            .into_iter()
            .filter(|card| card.def_id == SPELL && card.cost_mod != 0)
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r4589002_shuffles_played_cards_cheaper() {
            let s = returned(false);
            let copies = shuffled(&s);
            assert_eq!(copies.len(), 2, "both Spells, never itself");
            for copy in nonempty(copies, "the copies") {
                assert_eq!(copy.cost_mod, -1, "they cost (1) less");
            }
            assert!(
                s.pile(P1, "library").iter().all(|card| card.def_id != ID),
                "never itself"
            );
            assert_eq!(s.hand(P1).len(), 3, "spent, then drew three");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4589002_radiant_shuffles_cheaper_and_draws_four() {
            let s = returned(true);
            let copies = shuffled(&s);
            assert_eq!(copies.len(), 2, "both Spells, never itself");
            for copy in nonempty(copies, "the copies") {
                assert_eq!(copy.cost_mod, -2, "they cost (2) less");
            }
            assert!(
                s.pile(P1, "library").iter().all(|card| card.def_id != ID),
                "never itself"
            );
            assert_eq!(s.hand(P1).len(), 4, "spent, then drew four");
        }
    }
}
