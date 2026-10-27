//! S #16 Rod of Ages (SPEC §8.10 row 16): (1) Field Spell, Quickdraw, Mythic, 1/1 → 2/2, of the held
//! Stinkyscape set (R1420).
//!   Base:    "Animated, Flying / The first Unit you play onto this is Equipped into it. / End of turn:
//!            Gain {keywords|random combat keyword|random combat keywords}. / Upon reaching 10 keywords,
//!            make this Radiant."
//!   Radiant: "Animated, Flying, Indestructible / The first Unit you play onto this is Equipped into it.
//!            / End of turn: Gain {keywords|random combat keyword|random combat keywords}."
//! Engine: Animated and Flying are catalog keywords (§6.1): it animates as it enters the field, into its
//! own lane's unit zone or R64's leftmost, a 1/1 Flying Unit. `StaticFlags.lends_equip` (R1869000): while
//! it acts as a Unit and is no fusion, any Unit card its controller plays may be played onto it as an
//! Equip play (R1086), beside the plain zones: the Unit resolves on top, its Cry included, then fuses
//! into it, this card the kept host (R77, R102); once it holds a fusion it lends no more. End of turn:
//! `grant_random_keywords` from R21's pool (R781005), {keywords} of them. Base: `radiant_at_keywords` 10
//! (R1850005): at a state check where its keyword kinds through the layers reach 10, each kind once,
//! Animated and Flying included, it is made Radiant. Radiant: Indestructible and two keywords a turn.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-016";

/// "Upon reaching 10 keywords, make this Radiant": printed data (R1748003), the base face's only.
const RADIANT_AT_KEYWORDS: i32 = 10;

fn face(radiant_at_keywords: Option<i32>) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            lends_equip: Some(true),
            ..StaticFlags::default()
        }),
        end_of_turn: Some(hook(|ctx| {
            vec![grant_random_keywords(json_as(json!({
                "target": { "of": "self" },
                "count": param(&*ctx, "keywords"),
            })))]
        })),
        radiant_at_keywords,
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(Some(RADIANT_AT_KEYWORDS)),
        radiant: face(None),
    }
}

// S #16 Rod of Ages — SPEC §8.10 row 16, BUILD M12 row S 16.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::{ID, RADIANT_AT_KEYWORDS};

    const FILLER: &str = "core-005"; // (1) Spell: a card in hand, so no turn ends by itself.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const MR_TOKEN: &str = "core-015"; // (1) 1/1, "Cry: Summon a Rush Token."
    const RUSH_TOKEN: &str = "core-t-rush";

    /// p1 has played the Rod (`radiant` its face) into backrow zone 2, `hand` still in hand.
    fn played(radiant: bool, hand: Value) -> Scenario {
        let mut cards = vec![json!({ "def": ID, "radiant": radiant })];
        cards.extend(hand.as_array().cloned().unwrap_or_default());
        cards.push(json!(FILLER));
        let mut s = crate::scenario(json!({
            "p1": { "hand": cards, "library": crate::filler(6) },
            "p2": { "hand": [FILLER], "library": crate::filler(6) },
        }));
        s.play(ID, json!({ "zone": 2 }));
        s
    }

    /// The zones p1 may play `card` onto as an Equip play, by lane.
    fn equip_lanes(s: &Scenario, card: &CardInstance) -> Vec<i32> {
        legal_actions(s.state(), P1)
            .iter()
            .filter_map(|action| match action {
                ActionBody::Play {
                    instance_id,
                    zone: Some(zone),
                    equip: Some(true),
                    ..
                } if *instance_id == card.id => Some(zone.lane),
                _ => None,
            })
            .collect()
    }

    fn kinds(s: &Scenario, card: &CardInstance) -> usize {
        let mut kinds: Vec<KeywordKind> = Vec::new();
        for keyword in s.stats(card).keywords {
            if !kinds.contains(&keyword.kind()) {
                kinds.push(keyword.kind());
            }
        }
        kinds.len()
    }

    mod base {
        use super::*;

        #[test]
        fn animates_as_a_1_1_flyer() {
            let mut s = played(false, json!([]));
            let rod = s.unit_at(P1, 2);
            assert_eq!(rod.def_id, ID);
            assert!(s.backrow(P1, 2).is_none(), "it left its backrow zone as it animated");
            s.expect_stats(&rod, json!({ "attack": 1, "health": 1 }));
            let keywords = s.stats(&rod).keywords;
            assert!(has_keyword(&keywords, KeywordKind::Animated) && has_keyword(&keywords, KeywordKind::Flying));
        }

        #[test]
        fn r1869000_the_first_unit_is_offered_onto_it_and_fuses_with_rod_kept() {
            let mut s = played(false, json!([MR_TOKEN]));
            let rod = s.unit_at(P1, 2);
            let token = s.card(MR_TOKEN).clone();
            assert_eq!(equip_lanes(&s, &token), vec![2]);
            s.play(&token, json!({ "zone": 2, "equip": true }));
            let fused = s.unit_at(P1, 2);
            assert_eq!(fused.id, rod.id, "the Rod is the kept host");
            assert!(s.state().players.p1.units.iter().flatten().flatten().all(|card| card.id != token.id));
            // It resolved on top first, its Cry included, then fused: 1/1 + 1/1.
            assert!(s.events().iter().any(|event| matches!(
                event,
                GameEvent::Summoned { def_id, .. } if def_id == RUSH_TOKEN
            )));
            assert!(s.last_events().iter().any(|event| matches!(event, GameEvent::Fused { .. })));
            s.expect_stats(&fused, json!({ "attack": 2, "health": 2 }));
            assert!(has_keyword(&s.stats(&fused).keywords, KeywordKind::Flying));
        }

        #[test]
        fn r1869000_a_second_unit_plays_as_usual() {
            let mut s = played(false, json!([MR_TOKEN, VANILLA]));
            s.state_mut().players.p1.mana.current = 4;
            s.play(MR_TOKEN, json!({ "zone": 2, "equip": true }));
            let vanilla = s.card(VANILLA).clone();
            assert!(equip_lanes(&s, &vanilla).is_empty(), "a fusion lends no more");
            s.expect_refused_with(|s| s.play(VANILLA, json!({ "zone": 2, "equip": true })), "is not Equip");
            s.play(VANILLA, json!({ "zone": 4 }));
            assert_eq!(s.unit_at(P1, 4).id, vanilla.id);
        }

        #[test]
        fn a_unit_played_elsewhere_first_keeps_the_offer_open() {
            let mut s = played(false, json!([VANILLA, MR_TOKEN]));
            s.play(VANILLA, json!({ "zone": 4 }));
            let token = s.card(MR_TOKEN).clone();
            assert_eq!(equip_lanes(&s, &token), vec![2], "R1869000: no fusion yet");
        }

        #[test]
        fn r1869000_a_rod_left_in_the_backrow_hosts_nothing() {
            let vanillas: Vec<Value> = (1..=5).map(|lane| json!({ "def": VANILLA, "lane": lane })).collect();
            let mut s = crate::scenario(json!({
                "p1": { "hand": [ID, MR_TOKEN, FILLER], "field": vanillas, "library": crate::filler(6) },
                "p2": { "hand": [FILLER], "library": crate::filler(6) },
            }));
            s.play(ID, json!({ "zone": 2 }));
            let rod = s.backrow(P1, 2).expect("no open unit zone, so it stays");
            assert_eq!(rod.def_id, ID);
            let token = s.card(MR_TOKEN).clone();
            assert!(equip_lanes(&s, &token).is_empty());
            s.expect_refused(|s| s.play(MR_TOKEN, json!({ "zone": 2, "row": "backrow", "equip": true })));
        }

        #[test]
        fn end_of_turn_grants_one_random_keyword() {
            let mut s = played(false, json!([]));
            let rod = s.unit_at(P1, 2);
            assert_eq!(kinds(&s, &rod), 2);
            s.end_turn();
            assert_eq!(s.card(&rod.id).granted_keywords.len(), 1);
            assert_eq!(kinds(&s, &rod), 3);
            assert_eq!(s.state().active, P2);
        }

        #[test]
        fn r1850005_its_end_of_turn_keyword_reaches_10_and_makes_it_radiant() {
            let mut s = played(false, json!([]));
            let rod = s.unit_at(P1, 2);
            let granted = [
                Keyword::Taunt,
                Keyword::Rush,
                Keyword::FirstStrike,
                Keyword::Ranged,
                Keyword::Poisonous,
                Keyword::Windfury,
                Keyword::Cleave,
            ];
            find_instance_mut(s.state_mut(), &rod.id)
                .expect("the Rod")
                .granted_keywords
                .extend(granted);
            assert_eq!(kinds(&s, &rod), RADIANT_AT_KEYWORDS as usize - 1);
            assert!(!s.card(&rod.id).radiant);
            s.end_turn();
            assert!(s.card(&rod.id).radiant);
            assert!(s.events().iter().any(|event| matches!(
                event,
                GameEvent::RadiantSet { instance_id, .. } if *instance_id == rod.id
            )));
            s.expect_stats(&rod, json!({ "attack": 2, "health": 2 }));
        }

        #[test]
        fn keywords_reads_through_param() {
            let mut s = played(false, json!([]));
            let rod = s.unit_at(P1, 2);
            step_param(s.card_mut(&rod.id), "keywords", 1);
            s.end_turn();
            assert_eq!(s.card(&rod.id).granted_keywords.len(), 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_2_2_indestructible_and_gains_two() {
            let mut s = played(true, json!([]));
            let rod = s.unit_at(P1, 2);
            s.expect_stats(&rod, json!({ "attack": 2, "health": 2 }));
            assert!(has_keyword(&s.stats(&rod).keywords, KeywordKind::Indestructible));
            s.end_turn();
            assert_eq!(s.card(&rod.id).granted_keywords.len(), 2);
            assert!(!s.card(&rod.id).granted_keywords.contains(&Keyword::Taunt), "R347: no Taunt beside Indestructible");
        }

        #[test]
        fn r1869000_it_lends_equip_too() {
            let mut s = played(true, json!([VANILLA]));
            let rod = s.unit_at(P1, 2);
            s.play(VANILLA, json!({ "zone": 2, "equip": true }));
            assert_eq!(s.unit_at(P1, 2).id, rod.id);
            // The kept Rod is Radiant, so the fusion runs its Radiant face: the Rod's 2/2 and the
            // Vanilla's Radiant 12/12 (R77, R1086).
            s.expect_stats(&rod, json!({ "attack": 14, "health": 14 }));
        }
    }
}
