//! Meditative #44 CN Jade Well (SPEC §8.8 row 44, R964, R3837005). (3) Field Spell, CN, Rare.
//!
//!   Base:    "Activate: Add an Auspicious Rock to your hand. It has Lucky {lucky}." (1)
//!   Radiant: "Activate: Add a Radiant Auspicious Rock to your hand. It has Lucky {lucky}." (1)
//!
//! Engine: an `ActivationDecl` (R384), once a turn and free, as M #40 Feng Shui's (R3140000), that
//! `add_to_hand`s the Rock (M #39.1). Issue #3837 turned its "Start of turn" into the Activate
//! (R3837005); through v0.6.2c it was a Field Spell's `start_of_turn` (R62), Core #6 Mana Well's shape,
//! which `script_v0_6_2c` keeps for those replays (R767000). MD-C15: an Activate needs a card that
//! stays on the field as much as a start-of-turn text does, as R402 reasoned for C #78.
//! Issue #4154 gave the Rock Lucky 1 on both faces (the add's `lucky` rider, R1438; the declared `lucky`,
//! R386; R4154212); through v0.6.6h it had none, which `script_v0_6_6h` keeps (R767000).
//! Tunes: lucky 1 ↑.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-044";

/// The Rock this adds (M #39.1).
pub const ROCK: &str = "meditative-039-1";

/// "Activate: Add an Auspicious Rock to your hand." (R384, R3837005): once a turn, free, Radiant on the
/// Radiant face; with `lucky`, "It has Lucky {lucky}" (R4154212).
fn rock(radiant: bool, lucky: bool) -> ActivationDecl {
    ActivationDecl {
        id: "rock".to_string(),
        label: if radiant {
            "Add a Radiant Auspicious Rock to your hand".to_string()
        } else {
            "Add an Auspicious Rock to your hand".to_string()
        },
        uses: ActivationUses::Count(1),
        cost: None,
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(move |ctx| {
            let mut args = json!({ "defId": ROCK, "radiant": radiant });
            if lucky {
                args["lucky"] = json!(param(&*ctx, "lucky"));
            }
            vec![add_to_hand(json_as(args))]
        }),
    }
}

fn well(radiant: bool, lucky: bool) -> Script {
    Script {
        activations: vec![rock(radiant, lucky)],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: well(false, true),
        radiant: well(true, true),
    }
}

/// R767000: the Activate through v0.6.6h, whose Rock had no Lucky (issue #4154). `KEPT_SCRIPTS` folds
/// those replays under it.
pub fn script_v0_6_6h() -> CardScripts {
    CardScripts {
        base: well(false, false),
        radiant: well(true, false),
    }
}

/// R767000: the Rock at the start of each of its controller's turns, through v0.6.2c; `KEPT_SCRIPTS`
/// folds those replays under it.
fn well_v0_6_2c(radiant: bool) -> Script {
    Script {
        start_of_turn: Some(hook(move |_ctx| {
            vec![add_to_hand(json_as(json!({ "defId": ROCK, "radiant": radiant })))]
        })),
        ..Script::default()
    }
}

pub fn script_v0_6_2c() -> CardScripts {
    CardScripts {
        base: well_v0_6_2c(false),
        radiant: well_v0_6_2c(true),
    }
}

// Meditative #44 CN Jade Well — SPEC §8.8 row 44, BUILD M10 row M 44.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::def_ids;
    use jackioh_engine::subsystems::activate::why_cannot_activate_ability;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-008";
    const LIBRARY: [&str; 3] = ["core-025", "core-008", "core-020"];

    fn standing(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "mana": 10, "hand": [FILLER], "backrow": [{ "def": ID, "radiant": radiant }], "library": LIBRARY },
            "p2": { "hand": [FILLER], "library": LIBRARY },
        }))
    }

    /// The Rocks `player` holds.
    fn rocks_of(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        s.hand(player).into_iter().filter(|card| card.def_id == ROCK).collect()
    }

    /// Whether `legal_actions` offers `player` an activation of the Well.
    fn listed_for(s: &Scenario, player: PlayerId) -> bool {
        let id = s.card(ID).id.clone();
        legal_actions(s.state(), player)
            .into_iter()
            .any(|body| matches!(body, ActionBody::Activate { instance_id, .. } if instance_id == id))
    }

    #[test]
    fn r964_r3837005_a_field_spell_that_stays_and_activates_a_rock_once_a_turn() {
        crate::register_all();
        assert_eq!(crate::card_def(ID).type_, CardType::FieldSpell);
        let mut s = standing(false);
        let id = s.card(ID).id.clone();
        let mana = s.view(P1).you.mana.current;
        assert!(listed_for(&s, P1));

        s.activate(ID, json!({}));

        let rocks = rocks_of(&s, P1);
        assert_eq!(rocks.len(), 1);
        assert!(!rocks[0].radiant);
        assert_eq!(s.view(P1).you.mana.current, mana, "the Activate is free");
        s.expect_in_zone(&id, "field");
        // R384: once a turn.
        assert!(!listed_for(&s, P1));
        let refused = why_cannot_activate_ability(s.state(), P1, &id, None).err().map(|error| error.message);
        assert_eq!(refused.as_deref(), Some("that ability has already been used this turn"));
        // And again on its controller's next turn.
        s.end_turn();
        s.end_turn();
        assert!(listed_for(&s, P1));
        s.activate(ID, json!({}));
        assert_eq!(rocks_of(&s, P1).len(), 2);
    }

    #[test]
    fn r3837005_its_start_of_turn_adds_nothing() {
        let mut s = standing(false);

        // Refresh, then the start-of-turn triggers, then the draw (§2.2): only the draw lands now.
        s.start_turn();

        let hand = def_ids(&s.hand(P1));
        assert_eq!(hand.len(), 2, "the filler and the draw");
        assert!(!hand.contains(&ROCK.to_string()));
    }

    #[test]
    fn r384_only_its_controller_activates_it() {
        let mut s = standing(false);
        s.end_turn();
        assert_eq!(s.state().active, P2);
        let id = s.card(ID).id.clone();
        let refused = why_cannot_activate_ability(s.state(), P2, &id, Some("rock")).err().map(|error| error.message);
        assert_eq!(refused.as_deref(), Some("that card is not yours"));
        assert!(!listed_for(&s, P2));
    }

    #[test]
    fn destroyed_it_stops() {
        let mut s = scenario(json!({
            // Mana to spare after the (4) exile, so the turn does not auto-end (R82).
            "p1": { "mana": 8, "hand": ["core-034", FILLER], "backrow": [ID], "library": LIBRARY },
            "p2": { "hand": [FILLER], "library": LIBRARY },
        }));
        let well = s.backrow(P1, 1).expect("the Well").id.clone();

        // #34 Collateral Damage exiles the Well (its deck-milling side effect is harmless here).
        s.play("core-034", json!({ "targets": [{ "pick": "instance", "instanceId": well }] }));
        assert_eq!(s.state().active, P1);

        // Exiled, it is no longer offered and cannot be activated.
        assert!(
            !legal_actions(s.state(), P1)
                .into_iter()
                .any(|body| matches!(body, ActionBody::Activate { instance_id, .. } if instance_id == well))
        );
        assert!(why_cannot_activate_ability(s.state(), P1, &well, None).is_err());
    }

    #[test]
    fn radiant_rock() {
        let mut s = standing(true);

        s.activate(ID, json!({}));

        let rocks = rocks_of(&s, P1);
        assert_eq!(rocks.len(), 1);
        assert!(rocks[0].radiant);
    }

    /// The Lucky the Rock one Activate adds holds, its printed Lucky (a Radiant Rock's 2) included.
    fn rock_lucky(radiant: bool) -> i32 {
        let mut s = standing(radiant);
        s.activate(ID, json!({}));
        let rock = rocks_of(&s, P1).into_iter().next().expect("a Rock");
        assert_eq!(rock.radiant, radiant);
        lucky_on(s.state(), &rock)
    }

    #[test]
    fn r4154212_the_rock_has_lucky_1_more_on_either_face() {
        for radiant in [false, true] {
            let plain = crate::under_kept(ID, script_v0_6_6h(), || rock_lucky(radiant));
            assert_eq!(rock_lucky(radiant), plain + 1, "radiant {radiant}");
        }
        assert_eq!(rock_lucky(false), 1, "a base Rock prints none");
    }

    #[test]
    fn r386_lucky_reads_through_param() {
        let mut s = standing(false);
        assert_eq!(crate::upgrade_number(&mut s, ID, "lucky"), 2);
        s.activate(ID, json!({}));
        let rock = rocks_of(&s, P1).into_iter().next().expect("a Rock");
        assert_eq!(lucky_on(s.state(), &rock), 2);
    }

    /// R767000: through v0.6.6h the Rock had no Lucky (issue #4154).
    #[test]
    fn r767000_the_script_kept_for_v0_6_6h_adds_a_rock_without_lucky() {
        crate::under_kept(ID, script_v0_6_6h(), || {
            let mut s = standing(false);
            s.activate(ID, json!({}));
            let rock = rocks_of(&s, P1).into_iter().next().expect("a Rock");
            assert_eq!(lucky_on(s.state(), &rock), 0);
        });
    }

    #[test]
    fn r767000_the_kept_v0_6_2c_script_adds_a_rock_at_the_start_of_turn() {
        crate::under_kept(ID, script_v0_6_2c(), || {
            for radiant in [false, true] {
                let mut s = standing(radiant);
                assert!(!listed_for(&s, P1));

                s.start_turn();

                let rocks = rocks_of(&s, P1);
                assert_eq!(rocks.len(), 1);
                assert_eq!(rocks[0].radiant, radiant);
            }
        });
    }
}
