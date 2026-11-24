//! SPEC §8.12 S+ #62 Overdrive Projector (R3556000, R3556001, R3556008, R4665002; R62, R1420):
//! (3) Field Spell, Mindustrial, Epic, a card of the Stinkyscape+ set.
//!   Base:    "Dormant for {dormant|turn|turns}.
//!            Your Units gain Rush and Windfury, and your Spells gain Echo +{spellEcho}." (2 turns)
//!   Radiant: the same (1 turn).
//!
//! It enters Dormant for `dormant` turns (R3556000), 2 on the base face and 1 on the Radiant one since
//! issue #4665. While it acts awake (R3556008), every Unit of its controller's gains Rush and Windfury
//! (as S #23 Warp Disruptor's aura), and every Spell its controller plays or casts resolves `spellEcho`
//! more times (R3556001). Issue #4665 made it (3), from (1), and its base face its controller's alone
//! (R4665002): through v0.6.12c that face reached every Unit on both sides and either player's Spells
//! (`SpellEchoAura.both`), which `script_v0_6_12c` keeps (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-062";

/// "Dormant for 2 turns" on the base face since issue #4665, the printed countdown its declared
/// `dormant` tunes (R386).
const DORMANT_TURNS: i32 = 2;
/// "Dormant for 1 turn" on the Radiant face.
const RADIANT_DORMANT_TURNS: i32 = 1;
/// R767000: the base face's countdown through v0.6.12c.
const DORMANT_TURNS_V0_6_12C: i32 = 1;
/// "Spells gain Echo +1".
const SPELL_ECHO: i32 = 1;

/// R3556008, R4665002: Rush and Windfury on its controller's Units — every Unit on both sides with
/// `everyone`, as the base face ran through v0.6.12c — read through the layers while it is awake.
fn overdrive_aura(everyone: bool) -> AuraHook {
    aura_hook(move |args| {
        let controller = args.self_.controller;
        let state = args.state;
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| {
                in_units_row_or_standing(state, unit) && (everyone || unit.controller == controller)
            }),
            mod_: StatMod {
                keywords: Some(vec![Keyword::Rush, Keyword::Windfury]),
                ..StatMod::default()
            },
        }]
    })
}

/// R4665002: `everyone`, both sides' Units and Spells, as the base face ran through v0.6.12c; `turns`,
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
    // R4665002: both faces reach only their controller's since issue #4665, the base face asleep 2
    // turns and the Radiant face 1.
    CardScripts {
        base: overdrive(false, DORMANT_TURNS),
        radiant: overdrive(false, RADIANT_DORMANT_TURNS),
    }
}

/// R767000, R4665002: through v0.6.12c the base face reached every Unit and both players' Spells, and
/// slept 1 turn; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_12c() -> CardScripts {
    CardScripts {
        base: overdrive(true, DORMANT_TURNS_V0_6_12C),
        radiant: overdrive(false, RADIANT_DORMANT_TURNS),
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

    /// R4665002: p1 holds the Projector and a Zap and p2 a Zap, each with a Unit; `"mana": 10` plays it.
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

    /// R3556001, R4665002: awake, p1's Zap resolves twice (4 + 4) and p2's, on p2's turn, once.
    fn only_p1_echoes(s: &mut Scenario) {
        s.play(ZAP, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
        // §6.2 Echo: each repeat asks its target afresh.
        s.answer(json!("hero:p2"));
        s.expect_health(P2, HERO_HEALTH - 8);
        s.end_turn();
        s.play(ZAP, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
        s.expect_health(P1, HERO_HEALTH - 4);
    }

    mod base {
        use super::*;

        /// R4665002: Dormant for 2 turns since issue #4665.
        #[test]
        fn r3556000_r4665002_dormant_for_two_turns_nothing_applies() {
            let mut s = projector(false, "s62-sleep");
            assert_eq!(slumber_turns_of(s.card(ID)), Some(2));
            assert!(!has(&s, P1, 1, KeywordKind::Rush), "Dormant: no Rush yet");
            assert!(!has(&s, P1, 1, KeywordKind::Windfury), "Dormant: no Windfury yet");
            next_turn_of(&mut s, P1);
            assert_eq!(
                slumber_turns_of(s.card(ID)),
                Some(1),
                "still Dormant after one own turn"
            );
            assert!(!has(&s, P1, 1, KeywordKind::Rush), "still no Rush");
        }

        #[test]
        fn r3556008_r4665002_awake_only_its_controllers_units_have_rush_and_windfury() {
            let mut s = projector(false, "s62-awake");
            next_turn_of(&mut s, P1);
            next_turn_of(&mut s, P1);
            assert_eq!(slumber_turns_of(s.card(ID)), None, "awake on the second own turn");
            assert!(has(&s, P1, 1, KeywordKind::Rush), "its Unit has Rush");
            assert!(has(&s, P1, 1, KeywordKind::Windfury), "its Unit has Windfury");
            assert!(!has(&s, P2, 1, KeywordKind::Rush), "the enemy's has no Rush");
            assert!(
                !has(&s, P2, 1, KeywordKind::Windfury),
                "the enemy's has no Windfury"
            );
        }

        #[test]
        fn r3556001_r4665002_awake_only_its_controllers_spells_echo_once_more() {
            let mut s = zaps(false, "s62-echo");
            next_turn_of(&mut s, P1);
            next_turn_of(&mut s, P1);
            only_p1_echoes(&mut s);
        }

        /// R767000, R4665002: through v0.6.12c the base face reached both sides' Units and Spells.
        #[test]
        fn r767000_r4665002_the_script_kept_for_v0_6_12c_reaches_both_sides() {
            crate::under_kept(ID, script_v0_6_12c(), || {
                let mut s = zaps(false, "s62-kept");
                next_turn_of(&mut s, P1);
                next_turn_of(&mut s, P1);
                for player in [P1, P2] {
                    assert!(
                        has(&s, player, 1, KeywordKind::Rush),
                        "{player:?}'s Unit has Rush"
                    );
                }
                s.play(ZAP, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
                s.answer(json!("hero:p2"));
                s.expect_health(P2, HERO_HEALTH - 8);
                // p2's own Zap echoes too.
                s.end_turn();
                s.play(ZAP, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
                s.answer(json!("hero:p1"));
                s.expect_health(P1, HERO_HEALTH - 8);
            });
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3556008_r3556001_radiant_dormant_for_one_turn_then_only_its_controllers() {
            let mut s = zaps(true, "s62-radiant");
            assert_eq!(slumber_turns_of(s.card(ID)), Some(1));
            next_turn_of(&mut s, P1);
            assert_eq!(slumber_turns_of(s.card(ID)), None, "awake on the next own turn");
            assert!(has(&s, P1, 1, KeywordKind::Rush), "its Unit has Rush");
            assert!(!has(&s, P2, 1, KeywordKind::Rush), "the enemy's has none");
            only_p1_echoes(&mut s);
        }
    }
}
