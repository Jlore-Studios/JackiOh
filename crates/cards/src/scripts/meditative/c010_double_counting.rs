//! Meditative #10 Double Counting (SPEC §8.8 row 10; docs/meditative-set.md M6 #10; ME-TRIG (b); R820,
//! R822, R823). (2) Field Spell, Epic.
//!   Base:    "Aura: Your Cry and Death effects trigger {extra|additional time|additional times}." (extra 1)
//!   Radiant: the same, extra 2.
//!
//! The engine's Cry and Death multiplier (`Script.cry_death_extra`), Hearthstone's Brann Bronzebeard and
//! Baron Rivendare in one card: while this acts on its controller's field, the Cry of a permanent they
//! play or cast (§10.5 step 5) or trigger (C #54 Rewind) runs extra more times with the same choices,
//! and the Death of a card that died under their control runs extra more times on its snapshot (§4.5
//! step 3). A Spell's resolution is no Cry (R822). Several multipliers do not add: the highest holds
//! (R820). Both faces run one script; `param` reads the face's `extra`.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-010";

pub fn script() -> CardScripts {
    let base = Script {
        cry_death_extra: Some(read_hook(|args| param(&args, "extra"))),
        ..Script::default()
    };
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// Meditative #10 Double Counting — SPEC §8.8 row 10, BUILD M10 row M 10: "While it acts, the Cry of
// a Unit or Field Spell you play or cast runs twice with the same declared targets and modes
// (R823), each run closed by its own state check, a target gone since fizzling that part; a
// Spell's resolution is not repeated (R822); the Death of a Unit that died under your control
// runs twice on its snapshot; the opponent's Cries and Deaths run once; a Cry C #54 Rewind triggers
// reuses its prompt answers (R467); two Double Countings give one extra (R820); extra reads
// through `param()`; radiant three runs".
//
// Core #68 Twisted Sorcerer (Cry: 4 damage to a declared target) is the aimed Cry it doubles;
// Core #43 Big Felinor (3/10) survives both hits, Core #11 Tempo Timmy (3/3) does not. Core #73
// Anti-oneshot Armor (a Field Spell with "Cry: Draw 1") is the Field Spell Cry; Core #5 Stockpile
// is the Spell whose resolution runs once. C+ #76 Brother Lar (Death: a Brother Ping) is the Death.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SORCERER: &str = "core-068"; // Unit 5/5. Cry: deal 4 damage to a declared target.
    const FELINOR: &str = "core-043"; // Unit 3/10, no text that answers damage.
    const TIMMY: &str = "core-011"; // Unit 3/3: one 4-hit kills it.
    const ARMOR: &str = "core-073"; // Field Spell. Cry: Draw 1.
    const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
    const LAR: &str = "classicplus-076"; // Unit 1/1. Death: Summon a Brother Ping.
    const PING: &str = "classicplus-076-1"; // Brother Ping.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy target Unit.
    const REWIND: &str = "classic-054"; // (1) Spell: trigger a Cry of one of your Units.
    const VANILLA: &str = "core-008"; // a (1) 4/4 with no text.
    const FILLER: &str = "core-010"; // (0) Spell, never played: it only keeps the turn open (R82).

    /// A side with mana to play and a stocked library, so no turn auto-ends (§2.5) or fatigues;
    /// `extra` replaces its own entries.
    fn side(extra: Value) -> Value {
        crate::merged(
            json!({ "mana": 10, "hand": [FILLER], "library": [FILLER, FILLER, FILLER] }),
            extra,
        )
    }

    /// A declared pick of one board instance (R81: it travels in the play action, never a prompt).
    fn pick(id: &str) -> Value {
        json!([{ "pick": "instance", "instanceId": id }])
    }

    /// How many `damage` events hit one instance.
    fn damage_events(s: &Scenario, target_id: &str) -> usize {
        s.events()
            .iter()
            .filter(|event| {
                let event = crate::js(event);
                event["type"] == "damage" && event["targetId"] == target_id
            })
            .count()
    }

    fn pings(s: &Scenario, player: PlayerId) -> usize {
        (1..=5)
            .filter_map(|lane| s.unit(player, lane))
            .filter(|unit| unit.def_id == PING)
            .count()
    }

    #[test]
    fn is_a_2_cost_epic_field_spell_one_script_on_both_faces() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.cost, CardCost::Fixed(2));
        assert_eq!(def.type_, CardType::FieldSpell);
        assert_eq!(def.rarity, Rarity::Epic);
        assert_eq!(
            crate::js(&def.params),
            json!([{ "key": "extra", "base": 1, "radiant": 2, "better": "up", "step": 1, "min": 1 }])
        );
        let scripts = super::script();
        assert!(scripts.base.cry_death_extra.is_some());
        assert!(scripts.radiant.cry_death_extra.is_some());
    }

    mod base {
        use super::*;

        #[test]
        fn r823_a_sorcerer_hits_its_declared_target_twice() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [ID], "hand": [SORCERER, FILLER] })),
                "p2": side(json!({ "field": [FELINOR] })),
            }));
            let victim = s.card(FELINOR).id.clone();
            s.play(SORCERER, json!({ "targets": pick(&victim) }));
            assert_eq!(damage_events(&s, &victim), 2);
            s.expect_stats(FELINOR, json!({ "health": 2, "maxHealth": 10 }));
        }

        #[test]
        fn r823_a_target_it_kills_is_not_hit_again() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [ID], "hand": [SORCERER, FILLER] })),
                "p2": side(json!({ "field": [TIMMY] })),
            }));
            let victim = s.card(TIMMY).id.clone();
            s.play(SORCERER, json!({ "targets": pick(&victim) }));
            s.expect_in_zone(TIMMY, "graveyard");
            assert_eq!(damage_events(&s, &victim), 1);
        }

        #[test]
        fn r823_a_field_spells_cry_draws_twice() {
            let mut s = scenario(json!({
                "p1": side(json!({
                    "backrow": [ID],
                    "hand": [ARMOR, FILLER],
                    "library": [VANILLA, VANILLA, VANILLA],
                })),
                "p2": side(json!({})),
            }));
            s.play(ARMOR, json!({}));
            // The filler plus the two drawn Vanillas.
            assert_eq!(s.hand(P1).len(), 3);
            assert_eq!(
                s.hand(P1).iter().filter(|card| card.def_id == VANILLA).count(),
                2
            );
        }

        #[test]
        fn r822_a_spells_resolution_runs_once() {
            let mut s = scenario(json!({
                "p1": side(json!({
                    "backrow": [ID],
                    "hand": [STOCKPILE, FILLER],
                    "health": 20,
                    "library": [VANILLA, VANILLA, VANILLA],
                })),
                "p2": side(json!({})),
            }));
            s.play(STOCKPILE, json!({}));
            assert_eq!(s.hand(P1).len(), 3);
            s.expect_health(P1, 22);
        }

        #[test]
        fn r823_brother_lar_dies_into_two_brother_pings() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [ID], "field": [LAR], "hand": [HIT_JOB, FILLER] })),
                "p2": side(json!({})),
            }));
            let lar = s.card(LAR).id.clone();
            // Hit Job is a Spell, so its own resolution runs once (R822) while Lar's Death runs
            // twice (R823).
            s.play(HIT_JOB, json!({ "targets": pick(&lar) }));
            s.expect_in_zone(LAR, "graveyard");
            assert_eq!(pings(&s, P1), 2);
        }

        #[test]
        fn r823_the_opponents_brother_lar_dies_into_one() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [ID], "hand": [HIT_JOB, FILLER] })),
                "p2": side(json!({ "field": [LAR] })),
            }));
            let lar = s.card(LAR).id.clone();
            s.play(HIT_JOB, json!({ "targets": pick(&lar) }));
            assert_eq!(pings(&s, P2), 1);
            assert_eq!(pings(&s, P1), 0);
        }

        #[test]
        fn r823_a_rewound_cry_asks_once_and_hits_twice() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [ID], "field": [SORCERER], "hand": [REWIND, FILLER] })),
                "p2": side(json!({})),
            }));
            let sorcerer = s.card(SORCERER).id.clone();
            s.play(REWIND, json!({ "targets": pick(&sorcerer) }));
            // The triggered Cry's declared target is asked once (R467); the extra run reuses it.
            assert!(s.state().pending.is_some());
            s.answer(json!("hero:p2"));
            assert!(s.state().pending.is_none());
            s.expect_health(P2, 22);
        }

        #[test]
        fn r820_two_double_countings_give_one_extra_not_two() {
            let mut s = scenario(json!({
                "p1": side(json!({
                    "backrow": [ID, ID],
                    "hand": [ARMOR, FILLER],
                    "library": [VANILLA, VANILLA, VANILLA],
                })),
                "p2": side(json!({})),
            }));
            s.play(ARMOR, json!({}));
            assert_eq!(s.hand(P1).len(), 3);
        }

        #[test]
        fn r386_extra_reads_through_param_a_buff_gives_three_runs() {
            let mut s = scenario(json!({
                "p1": side(json!({
                    "backrow": [ID],
                    "hand": [ARMOR, FILLER],
                    "library": [VANILLA, VANILLA, VANILLA, VANILLA],
                })),
                "p2": side(json!({})),
            }));
            assert_eq!(crate::upgrade_number(&mut s, ID, "extra"), 2);
            s.play(ARMOR, json!({}));
            assert_eq!(s.hand(P1).len(), 4);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r823_three_runs_in_all() {
            let mut s = scenario(json!({
                "p1": side(json!({
                    "backrow": [{ "def": ID, "radiant": true }],
                    "hand": [ARMOR, FILLER],
                    "library": [VANILLA, VANILLA, VANILLA, VANILLA],
                })),
                "p2": side(json!({})),
            }));
            s.play(ARMOR, json!({}));
            assert_eq!(s.hand(P1).len(), 4);
        }
    }
}
