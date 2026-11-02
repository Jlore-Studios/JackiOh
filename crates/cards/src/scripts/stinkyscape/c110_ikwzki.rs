//! S #110 iKwzki (SPEC §8.10 row 110): (2) Unit, Human, Acclaimed, Mythic, 6/6 → 12/12, of the
//! Stinkyscape set.
//!   Base:    "Cry: Duo with a permanent and make it Radiant next turn."
//!   Radiant: "Cry: Squad with all your permanents and make them Radiant next turn."
//! Engine: Cry (R1): `duo_radiant` (R1850001) on a declared permanent (R81), either side, face-down or
//! not: it carries the public gold `duo` mark (R437) and is made Radiant at the start of its marker's next
//! turn, not if it has left the field meanwhile (R174), even after this card has left (R76). With none to
//! declare, nothing. The Radiant face is `squad_radiant`: every permanent its controller has on the field,
//! unit tops in lane order then the backrow, a face-up Radiant one (this card among them, ST-S17-10) or a
//! Dormant one passed by.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-110";

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            targets: vec![TargetDecl::target(
                1,
                1,
                json!({ "side": "any", "of": ["unit", "backrow"] }),
            )],
            cry: Some(hook(|_ctx| vec![duo_radiant(DuoRadiantArgs::default())])),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| vec![squad_radiant()])),
            ..Script::default()
        },
    }
}

// S #110 iKwzki — SPEC §8.10 row 110, BUILD M12 row S 110 (R1850001, R62, R82).
#[cfg(test)]
mod tests {
    use jackioh_engine::effects::DUO_MARK;
    use jackioh_engine::marks::marks_on;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const VANILLA: &str = "core-008"; // (1) 4/4.
    const ARMOR: &str = "core-073"; // (2) Field Spell.
    const HIT_JOB: &str = "core-016"; // (3) Spell: destroy a Unit.
    const FILLER: &str = "core-005";

    fn board(radiant: bool, p2_hand: Value) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": [{ "def": VANILLA, "lane": 2 }],
                "backrow": [ARMOR],
                "library": filler(6),
                "mana": 10,
            },
            "p2": { "hand": p2_hand, "field": [VANILLA], "library": filler(6), "mana": 10 },
        }))
    }

    fn duo_marked(s: &Scenario, card: &CardInstance) -> bool {
        marks_on(s.state(), &card.id).iter().any(|mark| mark.mark == DUO_MARK)
    }

    fn pick(card: &CardInstance) -> Value {
        json!({ "targets": [{ "pick": "instance", "instanceId": card.id }], "zone": 1 })
    }

    #[test]
    fn r1850001_the_chosen_enemy_permanent_is_marked_and_made_radiant_at_your_next_turn() {
        let mut s = board(false, json!([FILLER]));
        let theirs = s.unit_at(P2, 1);
        s.play(ID, pick(&theirs));
        assert!(duo_marked(&s, &theirs));
        assert!(!s.card(&theirs).radiant);
        s.end_turn();
        assert!(!s.card(&theirs).radiant, "not at the opponent's turn");
        s.end_turn();
        assert!(s.card(&theirs).radiant);
        assert!(!duo_marked(&s, &theirs));
        let mine = s.unit_at(P1, 2);
        assert!(!mine.radiant, "only the chosen one");
        s.expect_stats(ID, json!({ "attack": 6, "health": 6 }));
    }

    #[test]
    fn r1850001_a_backrow_card_of_yours_may_be_chosen() {
        let mut s = board(false, json!([FILLER]));
        let armor = s.backrow(P1, 1).expect("the Field Spell");
        s.play(ID, pick(&armor));
        s.end_turn();
        s.end_turn();
        assert!(s.card(&armor).radiant);
    }

    #[test]
    fn r174_one_that_left_the_field_meanwhile_is_not_made_radiant() {
        let mut s = board(false, json!([HIT_JOB, FILLER]));
        let mine = s.unit_at(P1, 2);
        s.play(ID, pick(&mine));
        s.end_turn();
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": mine.id }] }));
        s.end_turn();
        assert!(!s.card(&mine).radiant);
    }

    #[test]
    fn r275_radiant_12_12_squads_every_permanent_of_yours_never_the_enemys() {
        let mut s = board(true, json!([FILLER]));
        s.play(ID, json!({ "zone": 1 }));
        let mine = s.unit_at(P1, 2);
        let armor = s.backrow(P1, 1).expect("the Field Spell");
        let theirs = s.unit_at(P2, 1);
        let me = s.unit_at(P1, 1);
        assert!(duo_marked(&s, &mine) && duo_marked(&s, &armor));
        assert!(!duo_marked(&s, &me), "Radiant already, passed by");
        assert!(!duo_marked(&s, &theirs));
        s.end_turn();
        s.end_turn();
        assert!(s.card(&mine).radiant && s.card(&armor).radiant);
        assert!(!s.card(&theirs).radiant);
        s.expect_stats(ID, json!({ "attack": 12, "health": 12 }));
    }
}
