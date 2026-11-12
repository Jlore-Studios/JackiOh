//! SPEC §8.12 S+ #136 Crawler Assembly Line (§6.3 Activate, Summon; R64, R384, R430, R1420, R1751000,
//! R3535000, R3560000): (2) Field Spell, Mindustrial, Rare, a card of the Stinkyscape+ set.
//!   Base:    "Activate 1: Summon a Crawler."
//!   Radiant: "Activate 1: Summon a Radiant Crawler."
//!
//! Engine: one Activate ability of 1 use a turn (R384), the number declared as `summon` (R386). Its run
//! summons a Crawler (S+ #120) into its controller's side by R64: the leftmost open unit zone, else the
//! leftmost empty Locked one (R688). It is offered only while the unit row has a zone to take the Crawler
//! (R430, `first_entry_zone`), and refused otherwise. The first Crawler to arrive each game adds a Toxopid
//! to its player's hand, as it does from a play (R3560000). The Radiant face summons it Radiant.

use jackioh_engine::effects::summon;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-136";

/// "Activate 1": 1 use a turn (R384).
const USES: i32 = 1;

/// The Crawler (S+ #120).
const CRAWLER: &str = "stinkyscapeplus-120";

/// R430: "Summon a Crawler" needs a unit zone for it.
fn has_entry_zone(state: &GameState, player: PlayerId) -> bool {
    first_entry_zone(state, player, Row::Units).is_some()
}

fn assembly_line(radiant: bool) -> Script {
    let label = if radiant { "Summon a Radiant Crawler" } else { "Summon a Crawler" };
    let activation = ActivationDecl {
        id: "assemble".to_string(),
        label: label.to_string(),
        uses: ActivationUses::Count(USES),
        cost: None,
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: Some(condition_hook(|c| has_entry_zone(c.state, c.controller))),
        has: None,
        run: hook(move |_ctx| vec![summon(json_as(json!({ "defId": CRAWLER, "radiant": radiant })))]),
    };
    Script {
        activations: vec![activation],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: assembly_line(false),
        radiant: assembly_line(true),
    }
}

// S+ #136 Crawler Assembly Line — SPEC §8.12 row S+ 136 (R384, R430; R64, R3560000).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{next_turn_of, weapon_board};
    use jackioh_engine::subsystems::activate::activate_actions_for;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const STAND_IN: &str = "stand-in (0) 1/1";
    const TOXOPID: &str = "stinkyscapeplus-121";

    fn assembling(radiant: bool, field: Value) -> Scenario {
        weapon_board(
            "s136",
            json!({ "backrow": [{ "def": ID, "radiant": radiant }], "field": field }),
            json!({}),
        )
    }

    fn activations(s: &Scenario) -> usize {
        activate_actions_for(s.state(), P1, s.card(ID)).len()
    }

    /// The Crawlers on p1's field, by lane.
    fn crawlers(s: &Scenario) -> Vec<(i32, CardInstance)> {
        (1..=UNIT_ZONES)
            .filter_map(|lane| s.unit(P1, lane).map(|unit| (lane, unit)))
            .filter(|(_, card)| card.def_id == CRAWLER)
            .collect()
    }

    /// R482: its one use a turn is the label `Activate:`, no param, on both faces.
    #[test]
    fn r482_the_one_use_is_its_label_and_no_param() {
        let def = crate::card_def(ID);
        assert!(def.params.is_none());
        for face in [&def.base, &def.radiant] {
            assert!(face.text.starts_with("Activate: Summon a "), "{}", face.text);
        }
        assert_eq!(USES, 1);
    }

    mod base {
        use super::*;

        #[test]
        fn r384_activate_summons_a_crawler_once_a_turn() {
            let mut s = assembling(false, json!([{ "def": STAND_IN, "lane": 1 }]));
            assert_eq!(activations(&s), 1);
            s.activate(ID, json!({ "ability": "assemble" }));
            let found = crawlers(&s);
            assert_eq!(found.iter().map(|(lane, _)| *lane).collect::<Vec<_>>(), vec![2], "the leftmost open zone (R64)");
            assert!(!found[0].1.radiant);
            // The uses are spent: a second is refused, and the Crawler stands alone.
            assert_eq!(activations(&s), 0);
            s.expect_refused(|s| s.activate(ID, json!({ "ability": "assemble" })));
            assert_eq!(crawlers(&s).len(), USES as usize);
            // It works again next turn.
            next_turn_of(&mut s, P1);
            assert_eq!(activations(&s), 1);
            s.activate(ID, json!({ "ability": "assemble" }));
            assert_eq!(crawlers(&s).len(), 2 * USES as usize);
        }

        #[test]
        fn r3560000_the_first_crawler_adds_a_toxopid_once() {
            let mut s = assembling(false, json!([]));
            s.activate(ID, json!({ "ability": "assemble" }));
            let toxopids = |s: &Scenario| s.hand(P1).iter().filter(|card| card.def_id == TOXOPID).count();
            assert_eq!(toxopids(&s), 1);
            next_turn_of(&mut s, P1);
            s.activate(ID, json!({ "ability": "assemble" }));
            assert_eq!(toxopids(&s), 1, "once a game");
        }

        #[test]
        fn r430_not_offered_with_a_full_unit_row() {
            let mut s = assembling(false, json!(vec![STAND_IN; UNIT_ZONES as usize]));
            assert_eq!(activations(&s), 0);
            s.expect_refused(|s| s.activate(ID, json!({ "ability": "assemble" })));
            assert!(crawlers(&s).is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r384_activate_summons_a_radiant_crawler() {
            let mut s = assembling(true, json!([]));
            s.activate(ID, json!({ "ability": "assemble" }));
            let found = crawlers(&s);
            assert_eq!(found.len(), 1);
            assert!(found[0].1.radiant);
            s.expect_refused(|s| s.activate(ID, json!({ "ability": "assemble" })));
        }

        #[test]
        fn r430_not_offered_with_a_full_unit_row() {
            let s = assembling(true, json!(vec![STAND_IN; UNIT_ZONES as usize]));
            assert_eq!(activations(&s), 0);
        }
    }
}
