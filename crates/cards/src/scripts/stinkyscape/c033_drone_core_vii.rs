//! S #33 Drone Core VII (SPEC §8.10 row 33): (5) Spell, Drone Core, Epic, a card of the Stinkyscape set.
//!   Base:    "Float reduces cost. / Summon a Drone Corvette and {interceptors|Drone Interceptor|Drone
//!            Interceptors}." — interceptors 2
//!   Radiant: "Float reduces cost. / Summon a Radiant Drone Corvette and {interceptors|Radiant Drone
//!            Interceptor|Radiant Drone Interceptors}." — interceptors 2
//! Engine: "Float reduces cost" (§6.2, R1563000) is the engine's `float_reduces_cost`, a hand trigger (R464) as
//!   S #34 Drone Core VIII has it, and none in the deck (R2657002): each Float of its owner's lowers its cost
//!   by that much, a `costMod` kept until a graveyard or exile (R766), so at (5), past `MAX_MANA`, it is played
//!   only so or with mana gained. Through v0.5.11o it cost (4) and had no hand trigger (`script_v0_5_11o`,
//!   kept, R767000). Two kinds of summon, the Corvette first, then one per Interceptor, each into the caster's
//!   leftmost open unit zone in turn (R64), then the backrow's once the unit row is full, where each stands
//!   (R2248000), no Cry (R1), as many as fit; the Interceptors' count is the
//!   declared `interceptors` (R386). Each drone draws its own combat keywords as it arrives (R781004).
//!   Radiant: all Radiant.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-033";

/// The Drone Interceptor token (§8.10 T-drone-interceptor).
const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
/// The Drone Corvette token (§8.10 T-drone-corvette).
const CORVETTE: &str = "stinkyscape-t-drone-corvette";

/// One face. `float` is the hand trigger the card has had since #3142.
fn face(radiant: bool, float: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut effects = vec![summon(json_as(json!({ "defId": CORVETTE, "radiant": radiant })))];
            effects.extend(
                (0..param(&*ctx, "interceptors"))
                    .map(|_| summon(json_as(json!({ "defId": INTERCEPTOR, "radiant": radiant })))),
            );
            effects
        })),
        hand_triggers: if float {
            vec![float_reduces_cost("drone-core-vii-float-hand")]
        } else {
            vec![]
        },
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false, true),
        radiant: face(true, true),
    }
}

/// R767000: through v0.5.11o it cost (4) and had no Float trigger; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_11o() -> CardScripts {
    CardScripts {
        base: face(false, false),
        radiant: face(true, false),
    }
}

// S #33 Drone Core VII — SPEC §8.10 row 33, BUILD M12 row S 33 (R1563000, R2248000, R2657002, R767000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::script_v0_5_11o;
    use crate::{filler, scenario, under_kept_as};

    const P1: PlayerId = PlayerId::P1;

    const CORE: &str = "stinkyscape-033";
    const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
    const CORVETTE: &str = "stinkyscape-t-drone-corvette";
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FILLER: &str = "core-005";

    fn setup(radiant: bool, field: Value) -> Scenario {
        scenario(json!({
            "seed": "drone-core-vii",
            "p1": { "hand": [{ "def": CORE, "radiant": radiant }, FILLER], "field": field, "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    fn price_of(s: &Scenario, id: &str) -> i32 {
        effective_cost(s.state(), s.card(id), CostOptions::default())
    }

    /// p1 holds one Drone Core VII in hand and one in the deck, `mana` unspent as its turn ends.
    fn floating(mana: i32, radiant: bool) -> (Scenario, String, String) {
        let s = scenario(json!({
            "p1": {
                "hand": [{ "def": CORE, "radiant": radiant }, FILLER],
                "library": [{ "def": CORE, "radiant": radiant }, FILLER, FILLER],
                "mana": mana,
            },
            "p2": { "hand": [FILLER], "library": filler(2) },
        }));
        let hand = s.hand(P1).into_iter().find(|card| card.def_id == CORE).map(|card| card.id).unwrap_or_default();
        let deck = s.pile(P1, "library").into_iter().find(|card| card.def_id == CORE).map(|card| card.id).unwrap_or_default();
        (s, hand, deck)
    }

    /// #3142: it costs (5), past `MAX_MANA` (4), on both faces, where it cost (4): with no Float it is not
    /// played, and with one it is, after the turn that floated.
    #[test]
    fn it_costs_5_and_is_played_only_after_a_float() {
        for radiant in [false, true] {
            let (mut s, hand, _) = floating(4, radiant);
            assert_eq!(price_of(&s, &hand), 5, "radiant {radiant}");
            crate::expect_costs(&mut s, CORE, 5, json!({}));

            // A Float of 2: its price is (3) from then on, so the next turn's mana plays it.
            let (mut s, hand, _) = floating(2, radiant);
            s.end_turn();
            assert_eq!(price_of(&s, &hand), 3, "radiant {radiant}");
            s.end_turn();
            assert_eq!(s.state().active, P1);
            s.play(CORE, json!({}));
            s.expect_in_zone(&hand, "graveyard");
            assert_eq!(s.unit(P1, 1).expect("the Corvette").def_id, CORVETTE, "radiant {radiant}");
        }
    }

    /// #3142, R1563000, R2657002: a Float lowers it by that much in its owner's hand, never in the deck.
    #[test]
    fn r1563000_r2657002_a_float_of_2_makes_it_cost_two_less_in_hand_never_in_the_deck() {
        for radiant in [false, true] {
            let (mut s, hand, deck) = floating(2, radiant);
            s.end_turn();
            assert_eq!(s.card(hand.as_str()).cost_mod, -2, "radiant {radiant}");
            assert_eq!(s.card(deck.as_str()).cost_mod, 0, "radiant {radiant}");
            assert_eq!(price_of(&s, &hand), 3, "radiant {radiant}");
        }
    }

    #[test]
    fn r1563000_the_opponents_float_lowers_nothing() {
        let (mut s, hand, _) = floating(0, false);
        s.end_turn();
        assert_eq!(s.card(hand.as_str()).cost_mod, 0);
        s.state_mut().players.p2.mana.current = 4;
        s.end_turn();
        assert_eq!(s.card(hand.as_str()).cost_mod, 0);
        assert_eq!(price_of(&s, &hand), 5);
    }

    /// R767000: through v0.5.11o it cost (4) and a Float lowered nothing.
    #[test]
    fn r767000_the_kept_script_has_no_float_trigger() {
        under_kept_as(CORE, script_v0_5_11o(), |def| def.cost = json_as(json!(4)), || {
            let (mut s, hand, _) = floating(2, false);
            s.end_turn();
            assert_eq!(s.card(hand.as_str()).cost_mod, 0);
            assert_eq!(price_of(&s, &hand), 4);
        });
    }

    fn row(s: &Scenario) -> Vec<Option<(String, bool)>> {
        (1..=5)
            .map(|lane| s.unit(P1, lane).map(|unit| (unit.def_id, unit.radiant)))
            .collect()
    }

    fn backrow(s: &Scenario) -> Vec<Option<(String, bool)>> {
        (1..=5)
            .map(|lane| s.backrow(P1, lane).map(|card| (card.def_id, card.radiant)))
            .collect()
    }

    fn of(def_id: &str, radiant: bool) -> Option<(String, bool)> {
        Some((def_id.to_string(), radiant))
    }

    #[test]
    fn r64_summons_the_corvette_first_then_two_interceptors() {
        let mut s = setup(false, json!([{ "def": VANILLA, "lane": 1 }]));
        s.play(CORE, json!({}));
        assert_eq!(
            row(&s),
            vec![
                of(VANILLA, false),
                of(CORVETTE, false),
                of(INTERCEPTOR, false),
                of(INTERCEPTOR, false),
                None
            ]
        );
    }

    #[test]
    fn r64_r2248000_with_one_open_zone_the_corvette_takes_it_and_the_interceptors_fly_into_the_backrow() {
        let mut s = setup(false, json!(vec![VANILLA; 4]));
        s.play(CORE, json!({}));
        assert_eq!(row(&s)[4], of(CORVETTE, false));
        assert_eq!(
            backrow(&s),
            vec![of(INTERCEPTOR, false), of(INTERCEPTOR, false), None, None, None]
        );
    }

    #[test]
    fn interceptors_reads_through_param() {
        let mut s = setup(false, json!([]));
        set_param(s.card_mut(CORE), "interceptors", 3);
        s.play(CORE, json!({}));
        assert_eq!(
            row(&s),
            vec![
                of(CORVETTE, false),
                of(INTERCEPTOR, false),
                of(INTERCEPTOR, false),
                of(INTERCEPTOR, false),
                None
            ]
        );
    }

    #[test]
    fn radiant_all_three_radiant() {
        let mut s = setup(true, json!([]));
        s.play(CORE, json!({}));
        assert_eq!(
            row(&s),
            vec![
                of(CORVETTE, true),
                of(INTERCEPTOR, true),
                of(INTERCEPTOR, true),
                None,
                None
            ]
        );
    }
}
