//! M #95.1 CN Golem (SPEC §8.8 row 95.1, §7; R42, R80, R311, R1245): (4) Unit, CN, Token, printed
//! Legendary, 10/10 → 20/20.
//!
//! Base:    "Rush, Poisonous, Cleave, Pierce\nWhenever this destroys a Unit, shuffle
//!          {viruses|CN-Virus|CN-Viruses} into your opponent's deck."
//! Radiant: "Rush, Poisonous, Cleave, Pierce, Windfury\nWhenever this destroys a Unit, shuffle
//!          {viruses|Radiant CN-Virus|Radiant CN-Viruses} into your opponent's deck."
//! Engine: the keywords are catalog data (§6.1). A kill trigger, C+ #19.5 Bot Loser's: on each
//! `destroyed` whose `killerId` is this (R42), Cleave and Poisonous kills included, while it still
//! stands on the field as the event is dispatched (R1245), `shuffle_into` the opponent's deck
//! `viruses` CN-Viruses (Core #90.1), the opponent's cards, Radiant on the Radiant face; the shuffle-in
//! is public and recorded in their list (R311), and a full deck refuses it (R80).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-095-1";

/// §7: the CN-Virus it shuffles in.
const CN_VIRUS: &str = "core-090-1";

/// "Whenever this destroys a Unit, shuffle {viruses} CN-Virus into your opponent's deck."
fn on_kill(radiant: bool) -> TriggerDef {
    TriggerDef::new("cn-golem-kill", &[GameEventType::Destroyed], move |ctx, event| {
        let killed_by_me = match (event, ctx.self_.as_ref()) {
            (
                GameEvent::Destroyed {
                    killer_id: Some(killer),
                    ..
                },
                Some(me),
            ) => *killer == me.id,
            _ => false,
        };
        if killed_by_me {
            vec![shuffle_into(json_as(json!({
                "defId": CN_VIRUS,
                "count": param(&*ctx, "viruses"),
                "player": "enemy",
                "radiant": radiant,
            })))]
        } else {
            Vec::new()
        }
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        triggers: vec![on_kill(false)],
        ..Script::default()
    };
    let radiant = Script {
        triggers: vec![on_kill(true)],
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// M #95.1 CN Golem — SPEC §8.8 row 95.1, R1245: "Rush, Poisonous, Cleave, Pierce, 10/10; whenever it is
// R42's killer of a Unit, Cleave kills included, while it still stands on the field, `viruses` CN-Viruses
// go into the opponent's deck, theirs and listed (R311), a full deck refusing them (R80); radiant 20/20
// with Windfury (R636), and the viruses Radiant".
#[cfg(test)]
mod tests {
    use super::{CN_VIRUS, ID};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008"; // Mr. Vanilla, 4/4, no text; 12/12 Radiant
    const FILLER: &str = "core-005";

    /// The Golem in p1's lane 3 (`golem` is its field entry), p2's units `enemies`, p2's library
    /// `library` fillers long.
    fn game(golem: Value, enemies: Value, library: usize) -> Scenario {
        let mut golem = golem;
        golem["def"] = json!(ID);
        golem["lane"] = json!(3);
        crate::scenario(json!({
            "seed": "cn-golem",
            "p1": { "field": [golem], "hand": [FILLER], "library": [FILLER, FILLER] },
            "p2": { "field": enemies, "hand": [FILLER], "library": vec![FILLER; library] },
        }))
    }

    fn golem_id(s: &Scenario) -> String {
        s.unit(P1, 3)
            .unwrap_or_else(|| panic!("expected the Golem in p1's lane 3"))
            .id
    }

    /// Attacks the unit in p2's `lane` with the Golem, wherever it stands.
    fn attack_lane(s: &mut Scenario, golem: &str, lane: i32) {
        let target = crate::unit_or_blank(s, P2, lane);
        s.attack(golem, target);
    }

    fn viruses(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        s.pile(player, "library")
            .into_iter()
            .filter(|card| card.def_id == CN_VIRUS)
            .collect()
    }

    fn shuffled_in(s: &Scenario) -> Vec<(PlayerId, String)> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::ShuffledIn { player, def_id, .. } => Some((*player, def_id.clone())),
                _ => None,
            })
            .collect()
    }

    fn destroyed(s: &Scenario) -> Vec<(String, Option<String>)> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Destroyed {
                    instance_id,
                    killer_id,
                    ..
                } => Some((instance_id.clone(), killer_id.clone())),
                _ => None,
            })
            .collect()
    }

    mod m95_1_cn_golem {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn keywords_and_ten_ten() {
                let def = crate::card_def(ID);
                assert_eq!(def.cost, CardCost::Fixed(4));
                assert!(def.token);
                assert_eq!([def.base.attack, def.base.health], [Some(10), Some(10)]);
                assert_eq!(
                    def.base.keywords,
                    vec![
                        Keyword::Rush,
                        Keyword::Poisonous,
                        Keyword::Cleave,
                        Keyword::Pierce
                    ]
                );
                let params = def.params.unwrap_or_default();
                let viruses = params.iter().find(|param| param.key == "viruses");
                assert_eq!(viruses.map(|param| (param.base, param.radiant)), Some((1, 1)));
                let s = game(json!({}), json!([]), 4);
                let golem = golem_id(&s);
                let view = s.stats(golem.as_str());
                assert_eq!((view.attack, view.max_health), (10, 10));
            }

            #[test]
            fn r1245_a_kill_shuffles_a_cn_virus_into_the_opponents_deck() {
                let mut s = game(json!({}), json!([{ "def": VANILLA, "lane": 3 }]), 4);
                let golem = golem_id(&s);
                let target = crate::unit_or_blank(&s, P2, 3);
                attack_lane(&mut s, &golem, 3);
                assert_eq!(destroyed(&s), vec![(target, Some(golem.clone()))]);
                s.expect_in_zone(golem.as_str(), "field");
                assert_eq!(s.pile(P2, "library").len(), 5);
                let added = viruses(&s, P2);
                assert_eq!(added.len(), 1);
                assert_eq!(
                    (added[0].owner, added[0].controller, added[0].radiant),
                    (P2, P2, false)
                );
                assert_eq!(shuffled_in(&s), vec![(P2, CN_VIRUS.to_string())]);
                // Nothing into the Golem's own deck.
                assert_eq!(s.pile(P1, "library").len(), 2);
                assert!(viruses(&s, P1).is_empty());
                // R311: shuffled in openly, so p2's own list names it.
                assert_eq!(
                    crate::js(&s.view(Some(P2)).you.own_library),
                    json!({
                        "cards": [
                            { "defId": CN_VIRUS, "radiant": false, "count": 1 },
                            { "defId": FILLER, "radiant": false, "count": 4 },
                        ],
                        "unknown": 0,
                    })
                );
            }

            #[test]
            fn r1245_a_cleave_kill_counts() {
                let mut s = game(
                    json!({}),
                    json!([{ "def": VANILLA, "lane": 3 }, { "def": VANILLA, "lane": 4 }]),
                    4,
                );
                let golem = golem_id(&s);
                let splashed = crate::unit_or_blank(&s, P2, 4);
                attack_lane(&mut s, &golem, 3);
                // The neighbour died to the Cleave, the Golem its killer (R42).
                assert!(destroyed(&s).contains(&(splashed, Some(golem.clone()))));
                assert_eq!(destroyed(&s).len(), 2);
                assert_eq!(viruses(&s, P2).len(), 2);
                assert_eq!(s.pile(P2, "library").len(), 6);
            }

            #[test]
            fn r1245_a_golem_dying_in_the_same_combat_shuffles_nothing() {
                // 9 damage on it already: the Vanilla's 4 back kills it as it kills the Vanilla.
                let mut s = game(json!({ "damage": 9 }), json!([{ "def": VANILLA, "lane": 3 }]), 4);
                let golem = golem_id(&s);
                let target = crate::unit_or_blank(&s, P2, 3);
                attack_lane(&mut s, &golem, 3);
                // It was R42's killer of the Vanilla, but no longer stood on the field to hear it.
                assert!(destroyed(&s).contains(&(target, Some(golem.clone()))));
                assert!(destroyed(&s).iter().any(|(id, _)| *id == golem));
                // A Token that leaves the field ceases to exist.
                s.expect_in_zone(golem.as_str(), "gone");
                assert!(viruses(&s, P2).is_empty());
                assert!(shuffled_in(&s).is_empty());
                assert_eq!(s.pile(P2, "library").len(), 4);
            }

            #[test]
            fn r80_a_full_deck_refuses_the_virus() {
                let cap = LIBRARY_CAP as usize;
                let mut s = game(json!({}), json!([{ "def": VANILLA, "lane": 3 }]), cap);
                let golem = golem_id(&s);
                attack_lane(&mut s, &golem, 3);
                assert!(s.unit(P2, 3).is_none());
                assert_eq!(s.pile(P2, "library").len(), cap);
                assert!(viruses(&s, P2).is_empty());
                assert!(shuffled_in(&s).is_empty());
            }

            #[test]
            fn viruses_reads_through_param() {
                let mut s = game(json!({}), json!([{ "def": VANILLA, "lane": 3 }]), 4);
                let golem = golem_id(&s);
                assert_eq!(crate::upgrade_number(&mut s, &golem, "viruses"), 2);
                attack_lane(&mut s, &golem, 3);
                assert_eq!(viruses(&s, P2).len(), 2);
                assert_eq!(s.pile(P2, "library").len(), 6);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn twenty_twenty_with_windfury_attacks_twice() {
                let def = crate::card_def(ID);
                assert_eq!([def.radiant.attack, def.radiant.health], [Some(20), Some(20)]);
                assert_eq!(
                    def.radiant.keywords,
                    vec![
                        Keyword::Rush,
                        Keyword::Poisonous,
                        Keyword::Cleave,
                        Keyword::Pierce,
                        Keyword::Windfury
                    ]
                );
                let mut s = game(
                    json!({ "radiant": true }),
                    json!([
                        { "def": VANILLA, "lane": 1 },
                        { "def": VANILLA, "lane": 3 },
                        { "def": VANILLA, "lane": 5 },
                    ]),
                    4,
                );
                let golem = golem_id(&s);
                s.expect_stats(golem.as_str(), json!({ "attack": 20, "maxHealth": 20 }));
                attack_lane(&mut s, &golem, 1);
                attack_lane(&mut s, &golem, 5);
                assert!(s.unit(P2, 1).is_none());
                assert!(s.unit(P2, 5).is_none());
                // R636: two declared attacks a turn, and no third.
                let third = crate::unit_or_blank(&s, P2, 3);
                s.expect_refused(|s| s.attack(golem.as_str(), third.as_str()));
                assert_eq!(viruses(&s, P2).len(), 2);
            }

            #[test]
            fn r1245_shuffles_radiant_viruses() {
                let mut s = game(
                    json!({ "radiant": true }),
                    json!([{ "def": VANILLA, "lane": 3 }]),
                    4,
                );
                let golem = golem_id(&s);
                attack_lane(&mut s, &golem, 3);
                let added = viruses(&s, P2);
                assert_eq!(added.len(), 1);
                assert!(added[0].radiant);
                assert_eq!(added[0].owner, P2);
            }
        }
    }
}
