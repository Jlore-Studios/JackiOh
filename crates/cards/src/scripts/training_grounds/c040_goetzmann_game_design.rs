//! TG #40 Goetzmann Game Design (SPEC §8.11 row TG 40; §2.4, §5.1, R74, R97, R380, R387, R640, R1420):
//! (0) Spell, Quickdraw, Mythic, a card of the held Training Grounds set (R1420, R2194000).
//!   Base:    "Add a random card to your hand. It costs ({discount}) less." (1)
//!   Radiant: "Add a random Radiant card to your hand. It costs ({discount}) less." (1)
//! Core #59 add_random_from_catalog from shipped sets (R380, R1420), excluding own def (§5.1, R387), hidden from opponent (R97, §2.4). Radiant is Radiant (R74). Quickdraw flag (R640).
//! Issue #4154 added "It costs (1) less", the declared `discount` (R386), as the add's `costMod` rider
//! (R766: a price it has in that hand, never below (0), which a full hand's burn takes with it, R215;
//! R4154210); through v0.6.6g the card cost what it printed, which `script_v0_6_6g` keeps (R767000).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-040";

/// `discounted`: whether the card added costs the declared `discount` less (since issue #4154).
fn design(radiant: bool, discounted: bool) -> Script {
    Script {
        // R640: Quickdraw, the tag's rule, is this flag.
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(move |ctx| {
            let mut args = json!({ "query": { "excludeDefId": ID }, "count": 1 });
            if radiant {
                args["radiant"] = json!(true);
            }
            if discounted {
                args["costMod"] = json!(-param(&*ctx, "discount"));
            }
            vec![add_random_from_catalog(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: design(false, true),
        radiant: design(true, true),
    }
}

/// R767000: the scripts through v0.6.6g, whose card cost what it printed (issue #4154). `KEPT_SCRIPTS`
/// folds those replays under it.
pub fn script_v0_6_6g() -> CardScripts {
    CardScripts {
        base: design(false, false),
        radiant: design(true, false),
    }
}

// TG #40 Goetzmann Game Design — SPEC §8.11 row TG 40, BUILD M13 row TG 40.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const DESIGN: &str = "traininggrounds-040";

    fn board(seed: &str, radiant: bool) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": DESIGN, "radiant": radiant }], "library": filler(4) },
            "p2": { "hand": filler(1), "library": filler(4) },
        }))
    }

    /// The one card the play added to p1's hand.
    fn added(s: &mut Scenario) -> CardInstance {
        let before: Vec<String> = s.hand(P1).iter().map(|card| card.id.clone()).collect();
        s.play(DESIGN, json!({}));
        let new: Vec<CardInstance> = s.hand(P1).into_iter().filter(|card| !before.contains(&card.id)).collect();
        assert_eq!(new.len(), 1, "one card added");
        new[0].clone()
    }

    mod base {
        use super::*;

        #[test]
        fn one_random_non_token_card_of_a_shipped_set_into_your_hand() {
            for seed in ["tg40-a", "tg40-b", "tg40-c", "tg40-d", "tg40-e"] {
                let mut s = board(seed, false);
                let card = added(&mut s);
                let def = catalog::def_of(None, &card.def_id);
                assert!(!def.token, "{seed}: {}", def.id);
                assert!(set_ships(def.set), "{seed}: {} of a held set", def.id);
                assert!(!card.radiant);
            }
        }

        #[test]
        fn never_itself() {
            for seed in ["tg40-f", "tg40-g", "tg40-h", "tg40-i", "tg40-j"] {
                let mut s = board(seed, false);
                assert_ne!(added(&mut s).def_id, DESIGN, "{seed}");
            }
        }

        #[test]
        fn hidden_from_the_opponent() {
            let mut s = board("tg40-hidden", false);
            let card = added(&mut s);
            let seen = serde_json::to_string(&s.view(P2)).unwrap();
            assert!(!seen.contains(&card.id), "R97: the opponent's view never names it");
        }

        #[test]
        fn quickdraw_in_the_opening_hand() {
            crate::register_all();
            let others: Vec<String> = query(&CatalogQueryArgs::default())
                .iter()
                .map(|def| def.id.clone())
                .filter(|id| id != DESIGN)
                .take((DECK_SIZE - 1) as usize)
                .collect();
            let mut deck = others;
            deck.push(DESIGN.to_string());
            for seed in ["tg40-q1", "tg40-q2", "tg40-q3"] {
                let options: CreateGameOptions = json_as(json!({ "seed": seed, "decks": [deck, deck] }));
                let begun = begin_game(&create_game(&options)).state;
                assert!(begun.players.p1.hand.iter().any(|card| card.def_id == DESIGN), "{seed}");
                assert!(begun.players.p2.hand.iter().any(|card| card.def_id == DESIGN), "{seed}");
            }
        }
    }

    /// R4154210: the card added costs (1) less on either face: a `costMod` of -1, which R766's floor keeps
    /// at (0) or more.
    #[test]
    fn r4154210_the_card_added_costs_1_less() {
        for radiant in [false, true] {
            for seed in ["tg40-cost-a", "tg40-cost-b", "tg40-cost-c"] {
                let mut s = board(seed, radiant);
                let card = added(&mut s);
                assert_eq!(card.cost_mod, -1, "{seed}: {}", card.def_id);
            }
        }
    }

    /// R767000: through v0.6.6g the card cost what it printed (issue #4154).
    #[test]
    fn r767000_the_script_kept_for_v0_6_6g_adds_the_card_at_its_price() {
        crate::under_kept(DESIGN, super::script_v0_6_6g(), || {
            let mut s = board("tg40-kept", false);
            assert_eq!(added(&mut s).cost_mod, 0);
        });
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_card_is_radiant() {
            for seed in ["tg40-r1", "tg40-r2", "tg40-r3"] {
                let mut s = board(seed, true);
                let card = added(&mut s);
                assert!(card.radiant, "{seed}");
                assert_ne!(card.def_id, DESIGN);
            }
        }
    }
}
