//! SPEC §7 Stinkyscape T-red-narcor Red Narcor, a Resource token of the held Stinkyscape set (R1420).
//!   Base:    "Add a random Legendary or Mythic card to your hand. It costs ({discount}) less." (2)
//!   Radiant: "Add a random Radiant Legendary or Mythic card to your hand. It costs ({setCost})." (0)
//!
//! A Mythic Resource (R1840000), played from its owner's tray at (0) (R1840003) or sold (R1853003); the
//! Radiant S #39 The Narcor Project makes them. The pool is every non-token Legendary and Mythic card of
//! the sets that ship (R380, R1420), drawn uniformly (R60); a full hand burns the card, which then takes
//! no price (§2.4, R4). The base face's price is a `costMod` of −`discount` (never below (0), §2.3),
//! the Radiant face's a `costOverride` of `setCost`, each declared on its own face (R1431, R824000).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-red-narcor";

fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut args = json!({ "query": { "rarity": ["Legendary", "Mythic"] } });
            if radiant {
                args["radiant"] = json!(true);
                args["costOverride"] = json!(param(&*ctx, "setCost"));
            } else {
                args["costMod"] = json!(-param(&*ctx, "discount"));
            }
            vec![add_random_from_catalog(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// S T-red-narcor — SPEC §7, BUILD M12 row S T-red-narcor: "played from the tray, adds one random
// non-token Legendary or Mythic card at (2) less, never below (0); discount reads through `param()` on
// the base face only (R1431); radiant a Radiant one at (0), setCost on the Radiant face only (R824000)".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{card_def, play_from_tray, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";

    fn tray(seed: &str, radiant: bool) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "tray": [{ "def": ID, "radiant": radiant }], "hand": [FILLER] },
        }))
    }

    /// The card the play added, after checking it is a non-token Legendary or Mythic of a set that ships.
    fn added(s: &Scenario) -> CardInstance {
        let hand = s.hand(P1);
        assert_eq!(hand.len(), 2, "one card added");
        let card = hand[1].clone();
        let def = card_def(&card.def_id);
        assert!(
            matches!(def.rarity, Rarity::Legendary | Rarity::Mythic),
            "{} is Legendary or Mythic",
            def.id
        );
        assert!(
            !def.token && set_is_open(def.set),
            "{} is a shipped non-token",
            def.id
        );
        card
    }

    #[test]
    fn legendary_or_mythic_at_2_less() {
        let mut rarities = Vec::new();
        for seed in (0..20).map(|n| format!("red-narcor-{n}")) {
            let mut s = tray(&seed, false);
            play_from_tray(&mut s, P1, ID, Value::Null);
            let card = added(&s);
            assert!(!card.radiant);
            let printed = printed_cost(s.state(), &card);
            assert_eq!(
                cost_now(s.state(), &card),
                (printed - 2).max(0),
                "{seed}: {}",
                card.def_id
            );
            rarities.push(card_def(&card.def_id).rarity);
        }
        assert!(rarities.contains(&Rarity::Legendary) && rarities.contains(&Rarity::Mythic));
    }

    #[test]
    fn discount_reads_through_param() {
        let mut s = tray("red-narcor-param", false);
        let id = s.pile(P1, "tray")[0].id.clone();
        set_param(s.card_mut(id.as_str()), "discount", 10);
        play_from_tray(&mut s, P1, ID, Value::Null);
        let card = added(&s);
        assert_eq!(cost_now(s.state(), &card), 0, "never below (0)");
    }

    #[test]
    fn radiant_at_0() {
        for seed in (0..5).map(|n| format!("red-narcor-radiant-{n}")) {
            let mut s = tray(&seed, true);
            play_from_tray(&mut s, P1, ID, Value::Null);
            let card = added(&s);
            assert!(card.radiant, "{seed}: a Radiant one");
            assert_eq!(cost_now(s.state(), &card), 0, "{seed}: at (0)");
        }
    }

    #[test]
    fn radiant_set_cost_reads_through_param() {
        let mut s = tray("red-narcor-radiant-param", true);
        let id = s.pile(P1, "tray")[0].id.clone();
        set_param(s.card_mut(id.as_str()), "setCost", 1);
        play_from_tray(&mut s, P1, ID, Value::Null);
        let card = added(&s);
        assert_eq!(cost_now(s.state(), &card), 1);
    }
}
