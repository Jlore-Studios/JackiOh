//! SPEC §8.12 S+ #63 Overdrive Dome (R3556000, R3556001, R3556008; R62, R1420):
//! (2) Field Spell, Mindustrial, Epic, a card of the Stinkyscape+ set.
//!   Base:    "Dormant for {dormant|turn|turns}.
//!            All Units gain Charge and Windfury and Spell gains Echo +{spellEcho}."
//!   Radiant: "Dormant for {dormant|turn|turns}.
//!            Your Units gain Charge and Windfury and Spell gains Echo +{spellEcho}."
//!
//! It enters Dormant for `dormant` turns (R3556000). While it acts awake (R3556008), every Unit on
//! both sides — only its controller's on the Radiant face — gains Charge and Windfury (as S #23 Warp
//! Disruptor's aura), and every Spell its controller plays or casts — either player's with the base
//! face's `both` — resolves `spellEcho` more times (R3556001).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-063";

/// "Dormant for 2 turns".
const DORMANT_TURNS: i32 = 2;
/// "Spell gains Echo +2".
const SPELL_ECHO: i32 = 2;

/// R3556008: Charge and Windfury on every Unit — only the controller's on the Radiant face — read
/// through the layers while it is awake.
fn overdrive_aura(radiant: bool) -> AuraHook {
    aura_hook(move |args| {
        let controller = args.self_.controller;
        let state = args.state;
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| {
                in_units_row_or_standing(state, unit)
                    && (!radiant || unit.controller == controller)
            }),
            mod_: StatMod {
                keywords: Some(vec![Keyword::Charge, Keyword::Windfury]),
                ..StatMod::default()
            },
        }]
    })
}

fn overdrive(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            slumber: Some(SlumberText {
                turns: Some(DORMANT_TURNS),
                wakes_by: None,
            }),
            spell_echo_aura: Some(SpellEchoAura {
                amount: SPELL_ECHO,
                both: (!radiant).then_some(true),
            }),
            ..StaticFlags::default()
        }),
        aura: Some(overdrive_aura(radiant)),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: overdrive(false),
        radiant: overdrive(true),
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
        fn r3556008_awake_every_unit_has_charge_and_windfury() {
            let mut s = dome(false, "s63-awake");
            next_turn_of(&mut s, P1);
            assert_eq!(slumber_turns_of(s.card(ID)), Some(1), "still Dormant after one own turn");
            next_turn_of(&mut s, P1);
            assert_eq!(slumber_turns_of(s.card(ID)), None, "awake on the second own turn");
            for player in [P1, P2] {
                assert!(has(&s, player, 1, KeywordKind::Charge), "{player:?}'s Unit has Charge");
                assert!(
                    has(&s, player, 1, KeywordKind::Windfury),
                    "{player:?}'s Unit has Windfury"
                );
            }
        }

        #[test]
        fn r3556001_awake_both_players_spells_echo_twice_more() {
            let mut s = weapon_board(
                "s63-echo",
                json!({
                    "mana": 10,
                    "hand": [{ "def": ID, "radiant": false }, ZAP],
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
            next_turn_of(&mut s, P1);
            next_turn_of(&mut s, P1);
            // p1's Zap resolves three times: 4 + 4 + 4.
            s.play(ZAP, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            // §6.2 Echo: each repeat asks its target afresh.
            s.answer(json!("hero:p2"));
            s.answer(json!("hero:p2"));
            s.expect_health(P2, HERO_HEALTH - 12);
            // And p2's own Zap, on p2's turn, echoes twice too.
            s.end_turn();
            s.play(ZAP, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
            s.answer(json!("hero:p1"));
            s.answer(json!("hero:p1"));
            s.expect_health(P1, HERO_HEALTH - 12);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3556008_r3556001_radiant_only_its_controllers() {
            let mut s = weapon_board(
                "s63-radiant",
                json!({
                    "mana": 10,
                    "hand": [{ "def": ID, "radiant": true }, ZAP],
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
            next_turn_of(&mut s, P1);
            next_turn_of(&mut s, P1);
            assert!(has(&s, P1, 1, KeywordKind::Charge), "its Unit has Charge");
            assert!(!has(&s, P2, 1, KeywordKind::Charge), "the enemy's has none");
            // p1's Zap echoes twice; p2's resolves once.
            s.play(ZAP, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            // §6.2 Echo: each repeat asks its target afresh.
            s.answer(json!("hero:p2"));
            s.answer(json!("hero:p2"));
            s.expect_health(P2, HERO_HEALTH - 12);
            s.end_turn();
            s.play(ZAP, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
            s.expect_health(P1, HERO_HEALTH - 4);
        }
    }
}
