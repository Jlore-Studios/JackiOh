//! Meditative #39 赌石 Addict (SPEC §8.8 row 39). (2) Unit, CN, Rare, 4/4 → 8/8.
//!
//!   Base:    "Cry: Add an Auspicious Rock to your hand."
//!   Radiant: "Cry: Add a Radiant Auspicious Rock to your hand."
//!
//! Engine: `add_to_hand` the Rock (M #39.1), named, so no pool rule applies (R382); a full hand
//! burns it (§2.4). R1: the Cry fires when played or cast only — recruited, it adds no Rock.
//! Tunes: none. 赌石 (dǔshí) is jade gambling, buying uncut stones in the hope of jade.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-039";

/// The token this adds: Auspicious Rock (M #39.1).
pub const ROCK: &str = "meditative-039-1";

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|_ctx| {
                vec![add_to_hand(json_as(json!({ "defId": ROCK })))]
            })),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| {
                vec![add_to_hand(json_as(json!({ "defId": ROCK, "radiant": true })))]
            })),
            ..Script::default()
        },
    }
}

// Meditative #39 赌石 Addict — SPEC §8.8 row 39, BUILD M10 row M 39: "Cry (played or cast, R1):
// adds one base Auspicious Rock (M 39.1) to your hand, hidden from the opponent, burned at a full
// hand; summoned, no Rock; radiant 8/8 and the Rock is Radiant".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-008";
    const CALL_TO_ARMS: &str = "core-069";

    fn hand_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.hand(player).iter().map(|card| card.def_id.clone()).collect()
    }

    #[test]
    fn r1_the_cry_adds_a_base_rock_hidden_from_the_opponent() {
        crate::register_all();
        let mut s = scenario(json!({ "p1": { "hand": [ID, FILLER] }, "p2": { "hand": [FILLER] } }));

        s.play(ID, json!({}));

        assert_eq!(hand_defs(&s, P1), vec![FILLER, ROCK]);
        let rock = s.hand(P1).into_iter().find(|card| card.def_id == ROCK).expect("the Rock");
        assert!(!rock.radiant);
        assert_eq!(s.hand(P2).len(), 1);
        let theirs = serde_json::to_string(&s.view(P2)).unwrap();
        assert!(!theirs.contains(ROCK), "the Rock is hidden from the opponent");
        assert!(!theirs.contains(&rock.id));
    }

    #[test]
    fn a_full_hand_burns_it() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [ID, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER] },
            "p2": { "hand": [FILLER] },
        }));

        s.play(ID, json!({}));

        // Eleven in hand, ten after the play: the Rock is the eleventh add, so it burns.
        assert_eq!(s.hand(P1).len(), 10);
        assert!(!hand_defs(&s, P1).contains(&ROCK.to_string()));
        let burned: Vec<String> =
            s.pile(P1, "graveyard").into_iter().map(|card| card.def_id).collect();
        assert!(burned.contains(&ROCK.to_string()));
    }

    #[test]
    fn r1_recruited_by_core_069_it_adds_no_rock() {
        crate::register_all();
        // Radiant Call to Arms recruits Units costing 2 or less: the (2) Addict is one.
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": CALL_TO_ARMS, "radiant": true }],
                "library": [ID, FILLER, FILLER, FILLER],
            },
            "p2": { "hand": [FILLER] },
        }));

        s.play(CALL_TO_ARMS, json!({}));

        // Recruited, never played or cast: the Cry stays silent (R1).
        assert!(s.unit(P1, 0).is_some_and(|unit| unit.def_id == ID));
        assert!(hand_defs(&s, P1).iter().all(|def| def != ROCK));
    }

    #[test]
    fn radiant_8_8_adds_a_radiant_rock() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(
            [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
            [Some(4), Some(4), Some(8), Some(8)]
        );
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": true }, FILLER] },
            "p2": { "hand": [FILLER] },
        }));

        s.play(ID, json!({}));

        let rock = s.hand(P1).into_iter().find(|card| card.def_id == ROCK).expect("the Rock");
        assert!(rock.radiant, "the Radiant face adds a Radiant Rock");
    }
}
