//! SPEC §7 Stinkyscape T-red-narcor Red Narcor, a Resource token of the Stinkyscape set (§7's token
//! table).
//!   Base:    "Gain {mana} mana. Draw {draw|card|cards}." (2, 1)
//!   Radiant: the same (4, 2).
//!
//! A Mythic Resource (R1840000), tagged Resource and Token (§5's tags, issue #2521), played from its
//! owner's hand at (0) for its Credits (R2250000, R2250001); the Radiant S #39 The Narcor Project makes
//! them. "Gain {mana} mana" is S #153 Stimmycheck's `gain_mana`: temporary mana this turn, which may
//! take current above max (§2.3, §6.3 Mana); then "Draw" is §6.3's draw of `draw` cards from its
//! caster's library, a full hand burning the overflow and an empty library dealing fatigue (§2.4, R4).
//! Both numbers are declared on each face (`mana` 2/4, `draw` 1/2) and read as it resolves (R386).
//! Issue #2521 replaced its old random Legendary-or-Mythic add (at a discount, Radiant at a set cost)
//! with this text.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-red-narcor";

fn face() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            vec![
                gain_mana(json_as(json!({ "amount": param(&*ctx, "mana") }))),
                draw(json_as(json!({ "count": param(&*ctx, "draw") }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(),
        radiant: face(),
    }
}

// S T-red-narcor — SPEC §7, BUILD M12 row S T-red-narcor (issue #2521): played from the hand at (0),
// its caster gains 2 mana (past max too) and draws 1; mana and draw read through `param()`; radiant
// gains 4 and draws 2. Tagged Resource and Token.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{card_def, def_ids, play_resource, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";
    /// The library's cards, top first: the draws take them in this order.
    const LIBRARY: [&str; 4] = ["core-008", "core-019", "core-025", "core-005"];

    /// p1 holds the Red Narcor and a filler (R82) at the harness's 4/4 mana, `LIBRARY` to draw from.
    fn in_hand(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": LIBRARY },
        }))
    }

    fn played(radiant: bool) -> Scenario {
        let mut s = in_hand(radiant);
        play_resource(&mut s, P1, ID, Value::Null);
        s
    }

    /// The cards the play drew: the hand past the filler that stayed.
    fn drawn(s: &Scenario) -> Vec<String> {
        let hand = s.hand(P1);
        assert_eq!(hand[0].def_id, FILLER, "the filler stays first");
        def_ids(&hand[1..])
    }

    #[test]
    fn tagged_resource_and_token() {
        assert_eq!(card_def(ID).tags, vec![Tag::Resource, Tag::Token]);
    }

    #[test]
    fn gains_2_mana_and_draws_1() {
        let mut s = played(false);
        // 4 of 4, the Red Narcor at (0): 2 more, above max as temporary mana is (§2.3).
        s.expect_mana(P1, 6);
        assert_eq!(drawn(&s), ["core-008"], "the library's top card");
        assert_eq!(s.pile(P1, "library").len(), LIBRARY.len() - 1);
    }

    #[test]
    fn adds_no_random_card() {
        let s = played(false);
        for card in s.hand(P1) {
            assert!(
                card.def_id == FILLER || LIBRARY.contains(&card.def_id.as_str()),
                "{} came from the library or was held",
                card.def_id
            );
            assert_eq!(card.cost_mod, 0, "{} keeps its price", card.def_id);
            assert_eq!(card.cost_override, None, "{} keeps its price", card.def_id);
        }
    }

    #[test]
    fn mana_reads_through_param() {
        let mut s = in_hand(false);
        let id = crate::resources_in_hand(&s, P1)[0].id.clone();
        set_param(s.card_mut(id.as_str()), "mana", 5);
        play_resource(&mut s, P1, ID, Value::Null);
        s.expect_mana(P1, 9);
    }

    #[test]
    fn draw_reads_through_param() {
        let mut s = in_hand(false);
        let id = crate::resources_in_hand(&s, P1)[0].id.clone();
        set_param(s.card_mut(id.as_str()), "draw", 3);
        play_resource(&mut s, P1, ID, Value::Null);
        assert_eq!(drawn(&s), ["core-008", "core-019", "core-025"]);
    }

    mod radiant {
        use super::*;

        #[test]
        fn gains_4_mana_and_draws_2() {
            let mut s = played(true);
            s.expect_mana(P1, 8);
            assert_eq!(drawn(&s), ["core-008", "core-019"]);
            assert_eq!(s.pile(P1, "library").len(), LIBRARY.len() - 2);
        }

        #[test]
        fn mana_and_draw_read_through_param() {
            let mut s = in_hand(true);
            let id = crate::resources_in_hand(&s, P1)[0].id.clone();
            set_param(s.card_mut(id.as_str()), "mana", 1);
            set_param(s.card_mut(id.as_str()), "draw", 1);
            play_resource(&mut s, P1, ID, Value::Null);
            s.expect_mana(P1, 5);
            assert_eq!(drawn(&s), ["core-008"]);
        }
    }
}
