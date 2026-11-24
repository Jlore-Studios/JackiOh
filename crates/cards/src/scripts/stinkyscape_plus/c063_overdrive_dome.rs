//! SPEC §8.12 S+ #63 Overdrive Dome (R3556000, R3556001, R3556008, R4665002; R62, R1420):
//! (4) Field Spell, Mindustrial, Epic, a card of the Stinkyscape+ set.
//!   Base:    "Dormant for {dormant|turn|turns}.
//!            Your Units gain Charge and Windfury, and your Spells gain Echo +{spellEcho}." (2 turns)
//!   Radiant: the same (1 turn).
//!
//! It enters Dormant for `dormant` turns (R3556000), 2 on the base face and 1 on the Radiant one since
//! issue #4665. While it acts awake (R3556008), every Unit of its controller's gains Charge and
//! Windfury (as S #23 Warp Disruptor's aura), and every Spell its controller plays or casts resolves
//! `spellEcho` more times (R3556001). Issue #4665 made it (4), from (2), and its base face its
//! controller's alone (R4665002): through v0.6.12b that face reached every Unit on both sides and
//! either player's Spells (`SpellEchoAura.both`), which `script_v0_6_12b` keeps (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-063";

/// "Dormant for 2 turns", the printed countdown its declared `dormant` tunes (R386).
const DORMANT_TURNS: i32 = 2;
/// "Dormant for 1 turn" on the Radiant face since issue #4665 (2 before, `DORMANT_TURNS`).
const RADIANT_DORMANT_TURNS: i32 = 1;
/// "Spell gains Echo +2".
const SPELL_ECHO: i32 = 2;

/// R3556008, R4665002: Charge and Windfury on its controller's Units — every Unit on both sides with
/// `everyone`, as the base face ran through v0.6.12b — read through the layers while it is awake.
fn overdrive_aura(everyone: bool) -> AuraHook {
    aura_hook(move |args| {
        let controller = args.self_.controller;
        let state = args.state;
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| {
                in_units_row_or_standing(state, unit) && (everyone || unit.controller == controller)
            }),
            mod_: StatMod {
                keywords: Some(vec![Keyword::Charge, Keyword::Windfury]),
                ..StatMod::default()
            },
        }]
    })
}

/// R4665002: `everyone`, both sides' Units and Spells, as the base face ran through v0.6.12b; `turns`,
/// the printed countdown.
fn overdrive(everyone: bool, turns: i32) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            slumber: Some(SlumberText {
                turns: Some(turns),
                wakes_by: None,
            }),
            spell_echo_aura: Some(SpellEchoAura {
                amount: SPELL_ECHO,
                both: everyone.then_some(true),
            }),
            ..StaticFlags::default()
        }),
        aura: Some(overdrive_aura(everyone)),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // R4665002: both faces reach only their controller's since issue #4665, the Radiant face asleep 1
    // turn.
    CardScripts {
        base: overdrive(false, DORMANT_TURNS),
        radiant: overdrive(false, RADIANT_DORMANT_TURNS),
    }
}

/// R767000, R4665002: through v0.6.12b the base face reached every Unit and both players' Spells, and
/// the Radiant face slept 2 turns; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_12b() -> CardScripts {
    CardScripts {
        base: overdrive(true, DORMANT_TURNS),
        radiant: overdrive(false, DORMANT_TURNS),
    }
}

// S+ #63 Overdrive Dome — SPEC §8.12 row S+ 63 (R3556000, R3556001, R3556008).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, next_turn_of, upgrade_number, weapon_board};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A durable body to read keywords on.
    const BODY: &str = "stand-in (1) 4/4 R12/12";
    /// True Strike (Core #44): a (1) Spell dealing 4 damage.
    const ZAP: &str = "core-044";

    /// p1 holds the Dome with Units on both sides; `"mana": 10` plays it from hand.
    fn dome(radiant: bool, seed: &str) -> Scenario {
        let mut s = weapon_board(
            seed,
            json!({
                "mana": 10,
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": [BODY],
                "library": filler(6),
            }),
            json!({
                "field": [BODY],
                "library": filler(6),
            }),
        );
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    fn has(s: &Scenario, player: PlayerId, lane: i32, kind: KeywordKind) -> bool {
        let id = s.unit(player, lane).expect("a Unit there").id;
        s.stats(id.as_str()).keywords.iter().any(|keyword| keyword.kind() == kind)
    }

    /// R4665002: p1 holds the Dome and a Zap and p2 a Zap, each with a Unit; `"mana": 10` plays it.
    fn zaps(radiant: bool, seed: &str) -> Scenario {
        let mut s = weapon_board(
            seed,
            json!({
                "mana": 10,
                "hand": [{ "def": ID, "radiant": radiant }, ZAP],
                "field": [BODY],
                "library": filler(6),
            }),
            json!({
                "hand": [ZAP],
                "field": [BODY],
                "library": filler(6),
            }),
        );
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    /// R3556001, R4665002: awake, p1's Zap resolves three times (4 + 4 + 4) and p2's, on p2's turn,
    /// once.
    fn only_p1_echoes(s: &mut Scenario) {
        s.play(ZAP, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
        // §6.2 Echo: each repeat asks its target afresh.
        s.answer(json!("hero:p2"));
        s.answer(json!("hero:p2"));
        s.expect_health(P2, HERO_HEALTH - 12);
        s.end_turn();
        s.play(ZAP, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
        s.expect_health(P1, HERO_HEALTH - 4);
    }

    mod base {
        use super::*;

        #[test]
        fn r3556000_dormant_for_two_turns_nothing_applies() {
            let s = dome(false, "s63-sleep");
            assert_eq!(slumber_turns_of(s.card(ID)), Some(2));
            assert!(!has(&s, P1, 1, KeywordKind::Charge), "Dormant: no Charge yet");
            assert!(!has(&s, P2, 1, KeywordKind::Windfury), "Dormant: no Windfury yet");
        }

        #[test]
        fn r3556000_an_upgrade_of_its_dormant_number_shortens_the_countdown() {
            let mut tuned = weapon_board(
                "s63-tune",
                json!({
                    "mana": 10,
                    "hand": [{ "def": ID, "radiant": false }],
                    "field": [BODY],
                    "library": filler(6),
                }),
                json!({
                    "field": [BODY],
                    "library": filler(6),
                }),
            );
            // An Upgrade moves the declared dormant down (R386): 2 becomes 1.
            upgrade_number(&mut tuned, ID, "dormant");
            tuned.play(ID, json!({ "zone": 1 }));
            assert_eq!(slumber_turns_of(tuned.card(ID)), Some(1));
        }

        #[test]
        fn r3556008_r4665002_awake_only_its_controllers_units_have_charge_and_windfury() {
            let mut s = dome(false, "s63-awake");
            next_turn_of(&mut s, P1);
            assert_eq!(slumber_turns_of(s.card(ID)), Some(1), "still Dormant after one own turn");
            next_turn_of(&mut s, P1);
            assert_eq!(slumber_turns_of(s.card(ID)), None, "awake on the second own turn");
            assert!(has(&s, P1, 1, KeywordKind::Charge), "its Unit has Charge");
            assert!(has(&s, P1, 1, KeywordKind::Windfury), "its Unit has Windfury");
            assert!(!has(&s, P2, 1, KeywordKind::Charge), "the enemy's has no Charge");
            assert!(
                !has(&s, P2, 1, KeywordKind::Windfury),
                "the enemy's has no Windfury"
            );
        }

        #[test]
        fn r3556001_r4665002_awake_only_its_controllers_spells_echo_twice_more() {
            let mut s = zaps(false, "s63-echo");
            next_turn_of(&mut s, P1);
            next_turn_of(&mut s, P1);
            only_p1_echoes(&mut s);
        }

        /// R767000, R4665002: through v0.6.12b the base face reached both sides' Units and Spells.
        #[test]
        fn r767000_r4665002_the_script_kept_for_v0_6_12b_reaches_both_sides() {
            crate::under_kept(ID, script_v0_6_12b(), || {
                let mut s = zaps(false, "s63-kept");
                next_turn_of(&mut s, P1);
                next_turn_of(&mut s, P1);
                for player in [P1, P2] {
                    assert!(
                        has(&s, player, 1, KeywordKind::Charge),
                        "{player:?}'s Unit has Charge"
                    );
                }
                s.play(ZAP, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
                s.answer(json!("hero:p2"));
                s.answer(json!("hero:p2"));
                s.expect_health(P2, HERO_HEALTH - 12);
                // p2's own Zap echoes twice too.
                s.end_turn();
                s.play(ZAP, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
                s.answer(json!("hero:p1"));
                s.answer(json!("hero:p1"));
                s.expect_health(P1, HERO_HEALTH - 12);
            });
        }
    }

    mod radiant {
        use super::*;

        /// R4665002: Dormant for 1 turn on the Radiant face since issue #4665.
        #[test]
        fn r3556008_r3556001_r4665002_radiant_dormant_for_one_turn_then_only_its_controllers() {
            let mut s = zaps(true, "s63-radiant");
            assert_eq!(slumber_turns_of(s.card(ID)), Some(1));
            next_turn_of(&mut s, P1);
            assert_eq!(slumber_turns_of(s.card(ID)), None, "awake on the next own turn");
            assert!(has(&s, P1, 1, KeywordKind::Charge), "its Unit has Charge");
            assert!(!has(&s, P2, 1, KeywordKind::Charge), "the enemy's has none");
            only_p1_echoes(&mut s);
        }
    }
}
