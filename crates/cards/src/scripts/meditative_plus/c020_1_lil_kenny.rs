//! Meditative+ #20.1 Lil' Kenny (SPEC §8.9 row 20.1, §7, R1184003). (1) Unit, KY, Token (printed Rare),
//! X/X → 2X/2X.
//!
//!   Base:    "With X at least 4, it has Rush; 8, Divine Shield; 12, First Strike; 16, Cleave; 20,
//!             Lifesteal; 30, Indestructible."
//!   Radiant: the same text, on a 2X/2X.
//!
//! Only M+ #20 KY's Competition makes one, summoned with X its right answers (`summon`'s `x`). Its
//! stats are its faces' `xStats`, 1/1 and 2/2 times X (B2.7, as C+ #69 Buff Billy's), so the Radiant
//! face's 2X/2X is data. Its keywords are a `conditional_keywords` read of its X (C #69's shape), each
//! threshold cumulative: they read X, never the stats, so a buff gives none, and the Radiant face has the
//! same thresholds on X; a Vanilla takes them and keeps X/X (R781003, X being no buff). Both faces run
//! the same script. Declares no numbers: the thresholds are the frame.
//!
//! Voice audio is owed until the set ships (R1420): the `card-audio.json5` entry is written, and the
//! release renders it (`[gen:voice]`, R1448).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-020-1";

/// R1184003: each keyword from this X on, cumulatively.
const KEYWORDS_BY_X: &[(i32, Keyword)] = &[
    (4, Keyword::Rush),
    (8, Keyword::DivineShield),
    (12, Keyword::FirstStrike),
    (16, Keyword::Cleave),
    (20, Keyword::Lifesteal),
    (30, Keyword::Indestructible),
];

pub fn script() -> CardScripts {
    let base = Script {
        conditional_keywords: Some(read_hook(|args| {
            let x = x_of(args.self_);
            KEYWORDS_BY_X
                .iter()
                .filter(|(at, _)| x >= *at)
                .map(|(_, keyword)| keyword.clone())
                .collect()
        })),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Meditative+ #20.1 Lil' Kenny — SPEC §8.9 row 20.1, §7, R1184003, BUILD M11 row M+ 20.1: "summoned
// with X = 3 it is 3/3 with no keyword; X = 4 Rush; 8 also Divine Shield; 12 First Strike; 16 Cleave;
// 20 Lifesteal; 30 Indestructible; a +10 buff gives no keyword; Vanilla keeps X/X and drops them;
// radiant 2X/2X with the same thresholds on X". Each Kenny is summoned as M+ #20 summons one.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const COMPETITION: &str = "meditativeplus-020";
    const SILENCE: &str = "classicplus-009"; // (0) Spell: Vanilla a Unit.
    const FILLER: &str = "core-005";

    use KeywordKind::{Cleave, DivineShield, FirstStrike, Indestructible, Lifesteal, Rush};

    /// A Lil' Kenny summoned with X = `x` as M+ #20 summons one: the Competition cast (Radiant on
    /// `radiant`), `x` questions answered right, then Stop (none after the last). p2's field holds
    /// `enemies`.
    fn kenny(x: i32, radiant: bool, hand: &[&str], enemies: &[&str]) -> Scenario {
        let mut cards = vec![json!({ "def": COMPETITION, "radiant": radiant })];
        cards.extend(hand.iter().map(|id| json!(id)));
        let mut s = crate::scenario(json!({
            "seed": format!("lil-kenny-{x}-{radiant}"),
            "p1": { "hand": cards },
            "p2": { "hand": [FILLER], "field": enemies },
        }));
        s.play(COMPETITION, json!({}));
        for _ in 0..x {
            let data = s
                .state()
                .pending
                .as_ref()
                .map(|prompt| prompt.resume.data.clone())
                .unwrap_or_default();
            let right = answer_key_of(&data).unwrap_or_else(|| panic!("a question with its key"));
            s.answer(json!(format!("mode:{right}")));
        }
        if s.state().pending.is_some() {
            s.answer(json!(ANSWER_STOP_KEY));
        }
        s
    }

    fn the_kenny(s: &Scenario) -> CardInstance {
        let found: Vec<&CardInstance> = active_units_of(s.state(), P1)
            .into_iter()
            .filter(|card| card.def_id == ID)
            .collect();
        assert_eq!(found.len(), 1);
        found
            .first()
            .map(|card| (*card).clone())
            .unwrap_or_else(|| panic!("a Lil' Kenny"))
    }

    /// Keyword kinds in one order, so two lists compare as sets.
    fn sorted(kinds: &[KeywordKind]) -> Vec<String> {
        let mut names: Vec<String> = kinds.iter().map(|kind| format!("{kind:?}")).collect();
        names.sort();
        names
    }

    /// The Kenny's attack, health and keywords as §10.4 has them.
    fn read(s: &Scenario) -> (i32, i32, Vec<String>) {
        let card = the_kenny(s);
        let view = unit_view(s.state(), &card);
        let kinds: Vec<KeywordKind> = view.keywords.iter().map(|keyword| keyword.kind()).collect();
        (view.attack, view.health, sorted(&kinds))
    }

    /// The attacks the Kenny may declare now.
    fn kenny_attacks(s: &Scenario) -> usize {
        let id = the_kenny(s).id;
        legal_actions(s.state(), P1)
            .into_iter()
            .filter(|action| matches!(action, ActionBody::Attack { attacker_id, .. } if *attacker_id == id))
            .count()
    }

    #[test]
    fn declares_no_numbers() {
        crate::register_all();
        assert!(crate::card_def(ID).params.is_none());
    }

    mod base {
        use super::*;

        #[test]
        fn r1184003_summoned_with_x_3_it_is_a_3_3_with_no_keyword() {
            assert_eq!(read(&kenny(3, false, &[], &[])), (3, 3, vec![]));
        }

        #[test]
        fn r1184003_each_threshold_of_x_adds_its_keyword_to_those_below_it() {
            let cases: [(i32, &[KeywordKind]); 7] = [
                (4, &[Rush]),
                (7, &[Rush]),
                (8, &[Rush, DivineShield]),
                (12, &[Rush, DivineShield, FirstStrike]),
                (16, &[Rush, DivineShield, FirstStrike, Cleave]),
                (20, &[Rush, DivineShield, FirstStrike, Cleave, Lifesteal]),
                (
                    30,
                    &[Rush, DivineShield, FirstStrike, Cleave, Lifesteal, Indestructible],
                ),
            ];
            for (x, keywords) in cases {
                assert_eq!(
                    read(&kenny(x, false, &[], &[])),
                    (x, x, sorted(keywords)),
                    "X = {x}"
                );
            }
        }

        #[test]
        fn r1184003_its_rush_from_x_4_lets_it_attack_a_unit_the_turn_it_is_summoned_and_none_below() {
            assert_eq!(kenny_attacks(&kenny(3, false, &[], &["core-008"])), 0);
            assert_eq!(kenny_attacks(&kenny(4, false, &[], &["core-008"])), 1);
        }

        #[test]
        fn r1184003_a_10_10_buff_gives_no_keyword_the_thresholds_read_x_not_the_stats() {
            let mut s = kenny(3, false, &[], &[]);
            let id = the_kenny(&s).id;
            s.card_mut(id.as_str()).buffs = AttackHealth {
                attack: 10,
                health: 10,
            };
            assert_eq!(read(&s), (13, 13, vec![]));
        }

        #[test]
        fn r781003_r1184003_a_vanilla_kenny_keeps_its_x_x_and_drops_its_keywords() {
            let mut s = kenny(8, false, &[SILENCE], &[]);
            let id = the_kenny(&s).id;
            s.play(
                SILENCE,
                json!({ "targets": [{ "pick": "instance", "instanceId": id }] }),
            );
            assert_eq!(read(&s), (8, 8, vec![]));
            assert_eq!(x_of(&the_kenny(&s)), 8);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1184003_summoned_with_x_3_it_is_a_6_6_with_no_keyword() {
            let s = kenny(3, true, &[], &[]);
            assert!(the_kenny(&s).radiant);
            assert_eq!(read(&s), (6, 6, vec![]));
        }

        #[test]
        fn r1184003_the_same_thresholds_read_x_not_its_doubled_stats() {
            assert_eq!(read(&kenny(4, true, &[], &[])), (8, 8, sorted(&[Rush])));
            assert_eq!(
                read(&kenny(12, true, &[], &[])),
                (24, 24, sorted(&[Rush, DivineShield, FirstStrike]))
            );
            assert_eq!(
                read(&kenny(30, true, &[], &[])),
                (
                    60,
                    60,
                    sorted(&[Rush, DivineShield, FirstStrike, Cleave, Lifesteal, Indestructible])
                )
            );
        }

        #[test]
        fn r781003_r1184003_a_vanilla_radiant_kenny_keeps_its_2x_2x_and_drops_its_keywords() {
            let mut s = kenny(8, true, &[SILENCE], &[]);
            let id = the_kenny(&s).id;
            s.play(
                SILENCE,
                json!({ "targets": [{ "pick": "instance", "instanceId": id }] }),
            );
            assert_eq!(read(&s), (16, 16, vec![]));
        }
    }
}
