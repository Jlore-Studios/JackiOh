//! S #16 Rod of Ages (SPEC §8.10 row 16): (1) Field Spell, Quickdraw, Mythic, no stats (R2280000; 1/1 →
//! 2/2 before issue #2280), of the Stinkyscape set.
//!   Base:    "Animated, Flying / The first Unit you play onto this is Equipped into it. / End of turn:
//!            Gain {keywords|random combat keyword|random combat keywords}. / Upon reaching 10 keywords,
//!            make this Radiant."
//!   Radiant: "Animated, Flying, Indestructible / The first Unit you play onto this is Equipped into it.
//!            / End of turn: Gain {keywords|random combat keyword|random combat keywords}."
//! Animated and Flying catalog keywords (§6.1, R2280000). lends_equip lets first unit equip into it (R1869000, R1086, R77, R102, R64, R2280001).
//! End of turn grants random combat keywords (R21, R781005). Base becomes Radiant at 10 keywords (R1850005, R1748003).

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
        fn r2280000_it_enters_and_stays_in_its_backrow_zone_no_unit() {
            let s = played(false, json!([]));
            let rod = s.backrow(P1, 2).expect("with no stats it stays in its backrow zone");
            assert_eq!(rod.def_id, ID);
            assert!(s.unit(P1, 2).is_none());
            assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::Animated));
            let keywords = s.stats(&rod).keywords;
            assert!(has_keyword(&keywords, KeywordKind::Animated) && has_keyword(&keywords, KeywordKind::Flying));
        }

        #[test]
        fn r1869000_r2280001_the_first_unit_resolves_on_it_in_its_backrow_zone_fuses_in_and_rod_animates_with_its_stats() {
            let mut s = played(false, json!([MR_TOKEN]));
            let rod = s.backrow(P1, 2).expect("the Rod");
            let token = s.card(MR_TOKEN).clone();
            assert_eq!(equip_lanes(&s, &token), vec![2]);
            s.play(&token, json!({ "zone": 2, "row": "backrow", "equip": true }));
            assert!(find_instance(s.state(), &token.id).is_none());
            assert!(s.events().iter().any(|event| matches!(
                event,
                GameEvent::Summoned { def_id, .. } if def_id == RUSH_TOKEN
            )));
            assert!(s.last_events().iter().any(|event| matches!(event, GameEvent::Fused { .. })));
            assert!(s.last_events().iter().any(|event| matches!(event, GameEvent::Animated { .. })));
            assert!(s.backrow(P1, 2).is_none());
            let fused = s.unit_at(P1, 2);
            assert_eq!(fused.id, rod.id, "the Rod is the kept host");
            s.expect_stats(&fused, json!({ "attack": 1, "health": 1 }));
            assert!(has_keyword(&s.stats(&fused).keywords, KeywordKind::Flying));
        }

        #[test]
        fn r1869000_a_second_unit_plays_as_usual() {
            let mut s = played(false, json!([MR_TOKEN, VANILLA]));
            s.state_mut().players.p1.mana.current = 4;
            s.play(MR_TOKEN, json!({ "zone": 2, "row": "backrow", "equip": true }));
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
        fn r1869000_r2280001_with_every_unit_zone_full_it_hosts_in_its_backrow_zone_and_stays_fused() {
            let vanillas: Vec<Value> = (1..=5).map(|lane| json!({ "def": VANILLA, "lane": lane })).collect();
            let mut s = crate::scenario(json!({
                "p1": { "hand": [ID, MR_TOKEN, FILLER], "field": vanillas, "library": crate::filler(6) },
                "p2": { "hand": [FILLER], "library": crate::filler(6) },
            }));
            s.state_mut().players.p1.mana.current = 4;
            s.play(ID, json!({ "zone": 2 }));
            let rod = s.backrow(P1, 2).expect("no Unit, it stays");
            let token = s.card(MR_TOKEN).clone();
            assert_eq!(equip_lanes(&s, &token), vec![2]);
            s.play(MR_TOKEN, json!({ "zone": 2, "row": "backrow", "equip": true }));
            assert!(s.last_events().iter().any(|event| matches!(event, GameEvent::Fused { .. })));
            let fused = s.backrow(P1, 2).expect("the fused Rod stays");
            assert_eq!(fused.id, rod.id);
            assert_ne!(fused.def_id, ID);
            assert!(!s.last_events().iter().any(|event| matches!(event, GameEvent::Animated { .. })));
        }

        #[test]
        fn end_of_turn_grants_one_random_keyword() {
            let mut s = played(false, json!([]));
            let rod = s.backrow(P1, 2).expect("the Rod");
            assert_eq!(kinds(&s, &rod), 2);
            s.end_turn();
            assert_eq!(s.card(&rod.id).granted_keywords.len(), 1);
            assert_eq!(kinds(&s, &rod), 3);
            assert_eq!(s.state().active, P2);
        }

        #[test]
        fn r2280000_as_a_unit_its_end_of_turn_still_grants_a_keyword() {
            let mut s = played(false, json!([VANILLA]));
            s.state_mut().players.p1.mana.current = 4;
            s.play(VANILLA, json!({ "zone": 2, "row": "backrow", "equip": true }));
            let rod = s.unit_at(P1, 2);
            assert!(s.card(&rod.id).granted_keywords.is_empty());
            s.end_turn();
            assert_eq!(s.card(&rod.id).granted_keywords.len(), 1);
            assert_eq!(s.unit_at(P1, 2).id, rod.id);
        }

        #[test]
        fn r1850005_its_end_of_turn_keyword_reaches_10_and_makes_it_radiant() {
            let mut s = played(false, json!([]));
            let rod = s.backrow(P1, 2).expect("the Rod");
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
            assert_eq!(s.backrow(P1, 2).map(|card| card.id), Some(rod.id.clone()));
        }

        #[test]
        fn keywords_reads_through_param() {
            let mut s = played(false, json!([]));
            let rod = s.backrow(P1, 2).expect("the Rod");
            step_param(s.card_mut(&rod.id), "keywords", 1);
            s.end_turn();
            assert_eq!(s.card(&rod.id).granted_keywords.len(), 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r2280000_it_stays_in_its_backrow_zone_indestructible_and_gains_two() {
            let mut s = played(true, json!([]));
            let rod = s.backrow(P1, 2).expect("with no stats it stays in its backrow zone");
            assert!(s.unit(P1, 2).is_none());
            assert!(has_keyword(&s.stats(&rod).keywords, KeywordKind::Indestructible));
            s.end_turn();
            assert_eq!(s.card(&rod.id).granted_keywords.len(), 2);
            assert!(!s.card(&rod.id).granted_keywords.contains(&Keyword::Taunt), "R347: no Taunt beside Indestructible");
        }

        #[test]
        fn r1869000_it_lends_equip_too() {
            let mut s = played(true, json!([VANILLA]));
            let rod = s.backrow(P1, 2).expect("the Rod");
            s.play(VANILLA, json!({ "zone": 2, "row": "backrow", "equip": true }));
            assert_eq!(s.unit_at(P1, 2).id, rod.id);
            // The kept Rod is Radiant, so the fusion runs its Radiant face: no stats of its own and the
            // Vanilla's Radiant 12/12 (R77, R1086); it animated with them, Indestructible.
            s.expect_stats(&rod, json!({ "attack": 12, "health": 12 }));
            assert!(has_keyword(&s.stats(&rod).keywords, KeywordKind::Indestructible));
            assert!(has_keyword(&s.stats(&rod).keywords, KeywordKind::Flying));
        }
    }
}
