//! SPEC §8.12 S+ #62 Overdrive Projector (R3556000, R3556001, R3556008; R62, R1420):
//! (1) Field Spell, Mindustrial, Epic, a card of the Stinkyscape+ set.
//!   Base:    "Dormant for {dormant|turn|turns}.
//!            All Units gain Rush and Windfury and Spells gains Echo +{spellEcho}."
//!   Radiant: "Dormant for {dormant|turn|turns}.
//!            Your Units gain Rush and Windfury and Spells gains Echo +{spellEcho}."
//!
//! It enters Dormant for `dormant` turns (R3556000). While it acts awake (R3556008), every Unit on
//! both sides — only its controller's on the Radiant face — gains Rush and Windfury (as S #23 Warp
//! Disruptor's aura), and every Spell its controller plays or casts — either player's with the base
//! face's `both` — resolves `spellEcho` more times (R3556001).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-062";

/// "Dormant for 1 turn".
const DORMANT_TURNS: i32 = 1;
/// "Spells gain Echo +1".
const SPELL_ECHO: i32 = 1;

/// R3556008: Rush and Windfury on every Unit — only the controller's on the Radiant face — read
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
                keywords: Some(vec![Keyword::Rush, Keyword::Windfury]),
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

// S+ #62 Overdrive Projector — SPEC §8.12 row S+ 62 (R3556000, R3556001, R3556008).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, next_turn_of, weapon_board};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A durable body to read keywords on.
    const BODY: &str = "stand-in (1) 4/4 R12/12";
    /// True Strike (Core #44): a (1) Spell dealing 4 damage.
    const ZAP: &str = "core-044";

    /// p1 holds the Projector with Units on both sides; `"mana": 10` plays it from hand.
    fn projector(radiant: bool, seed: &str) -> Scenario {
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

    fn keywords(s: &Scenario, player: PlayerId, lane: i32) -> Vec<KeywordKind> {
        let id = s.unit(player, lane).expect("a Unit there").id;
        s.stats(id.as_str()).keywords.iter().map(Keyword::kind).collect()
    }

    fn has(s: &Scenario, player: PlayerId, lane: i32, kind: KeywordKind) -> bool {
        keywords(s, player, lane).contains(&kind)
    }

    mod base {
        use super::*;

        #[test]
        fn r3556000_dormant_for_one_turn_nothing_applies() {
            let s = projector(false, "s62-sleep");
            assert_eq!(slumber_turns_of(s.card(ID)), Some(1));
            assert!(!has(&s, P1, 1, KeywordKind::Rush), "Dormant: no Rush yet");
            assert!(!has(&s, P1, 1, KeywordKind::Windfury), "Dormant: no Windfury yet");
        }

        #[test]
        fn r3556008_awake_every_unit_has_rush_and_windfury() {
            let mut s = projector(false, "s62-awake");
            next_turn_of(&mut s, P1);
            assert_eq!(slumber_turns_of(s.card(ID)), None, "awake on the next own turn");
            for player in [P1, P2] {
                assert!(has(&s, player, 1, KeywordKind::Rush), "{player:?}'s Unit has Rush");
                assert!(has(&s, player, 1, KeywordKind::Windfury), "{player:?}'s Unit has Windfury");
            }
        }

        #[test]
        fn r3556001_awake_both_players_spells_echo_once_more() {
            let mut s = weapon_board(
                "s62-echo",
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
            // p1's Zap resolves twice: 4 + 4.
            s.play(ZAP, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            // §6.2 Echo: each repeat asks its target afresh.
            s.answer(json!("hero:p2"));
            s.expect_health(P2, HERO_HEALTH - 8);
            // And p2's own Zap, on p2's turn with the Projector awake, echoes too.
            s.end_turn();
            s.play(ZAP, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
            s.answer(json!("hero:p1"));
            s.expect_health(P1, HERO_HEALTH - 8);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3556008_r3556001_radiant_only_its_controllers() {
            let mut s = weapon_board(
                "s62-radiant",
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
            assert!(has(&s, P1, 1, KeywordKind::Rush), "its Unit has Rush");
            assert!(!has(&s, P2, 1, KeywordKind::Rush), "the enemy's has none");
            // p1's Zap echoes; p2's resolves once.
            s.play(ZAP, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            // §6.2 Echo: each repeat asks its target afresh.
            s.answer(json!("hero:p2"));
            s.expect_health(P2, HERO_HEALTH - 8);
            s.end_turn();
            s.play(ZAP, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
            s.expect_health(P1, HERO_HEALTH - 4);
        }
    }
}
