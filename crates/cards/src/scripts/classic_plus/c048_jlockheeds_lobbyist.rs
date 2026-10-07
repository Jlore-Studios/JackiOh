//! C+ #48 Jlockheed's Lobbyist (SPEC §8.7 row 48). (1) Unit, Jlockeed, Legendary, 0/3 → 0/6.
//!   Base:    "Can't be in Defense Position. Death: Add a random Jlockheed card to your hand. It costs (0)."
//!   Radiant: "Death: Add a random Radiant Jlockheed card to your hand. It costs (0)."
//!   Engine:  "The pool is the non-token cards with the `Jlockeed` tag (#13, #14, C #4, C+ #48, C+ #51,
//!            C+ #52) but this one (R387): #13, #14, C #4, C+ #51 and C+ #52; `costOverride` 0; the hand
//!            cap burns it (§2.4). The Defense restriction is a position validator flag (as #65.1's),
//!            which the Radiant face drops. With 0 attack it never attacks. Tunes: none."
//!
//! The restriction is #65.1's `neverDefense` flag, which the switch action, `legalActions` and a switch
//! made as an effect all honour (R20). The Death's pool is one tag (R278); `addRandomFromCatalog` leaves
//! out the card running the hook by its def id (R387), and puts the (0) price on a card only once it is
//! in the hand, so a burned one keeps none (§2.4, R4).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-048";

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            never_defense: Some(true),
            ..StaticFlags::default()
        }),
        death: Some(hook(|_ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "tags": ["Jlockeed"] },
                "costOverride": 0,
            })))]
        })),
        ..Script::default()
    };

    let radiant = Script {
        death: Some(hook(|_ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "tags": ["Jlockeed"] },
                "costOverride": 0,
                "radiant": true,
            })))]
        })),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C+ #48 Jlockheed's Lobbyist — SPEC §8.7 row 48, BUILD M9 Classic+ row C+ 48: "0/3: cannot be in
// Defense Position (a switch is refused, a switch-all effect leaves it in Attack) and never attacks
// with 0 attack; Death adds a random non-token Jlockeed card that costs (0), the pool exactly Core #13,
// #14, Classic #4 and C+ #51, #52 (one tag, never itself, R387); a full hand burns it; hidden from the
// opponent (R97); radiant 0/6, may go to Defense Position, and the card is Radiant".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const LOBBYIST: &str = "classicplus-048";
    const HIT_JOB: &str = "core-016"; // (3) Spell: destroy target Unit
    const SWITCH_ALL: &str = "core-048"; // (0) Spell: switch the position of every Unit
    const VANILLA: &str = "core-008"; // Mr. Vanilla 4/4
    const FILLER: &str = "core-005";
    const POOL: [&str; 5] = ["classic-004", "classicplus-051", "classicplus-052", "core-013", "core-014"];

    /// TS `board({ radiant?, seed?, hand? })`'s options.
    #[derive(Default)]
    struct Board {
        radiant: bool,
        seed: Option<String>,
        hand: Option<Vec<&'static str>>,
    }

    fn board(opts: Board) -> Scenario {
        let mut lobbyist = json!({ "def": LOBBYIST });
        if opts.radiant {
            lobbyist["radiant"] = json!(true);
        }
        scenario(json!({
            "seed": opts.seed.unwrap_or_else(|| "lobbyist".to_string()),
            "p1": {
                "field": [lobbyist, VANILLA],
                "hand": opts.hand.unwrap_or_else(|| vec![HIT_JOB, FILLER]),
            },
            "p2": { "hand": [FILLER], "field": [VANILLA] },
        }))
    }

    /// Play Hit Job on the Lobbyist and return the card its Death added (or burned).
    fn kill_lobbyist(s: &mut Scenario) -> Option<String> {
        let lobbyist = s.unit(P1, 1).expect("no Lobbyist");
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": lobbyist.id }] }));
        let added = s.last_events().iter().find_map(|event| match event {
            GameEvent::AddedToHand { player, instance_id, .. } if *player == P1 => Some(instance_id.clone()),
            _ => None,
        });
        let burned = s.last_events().iter().find_map(|event| match event {
            GameEvent::Burned { instance_id, .. } => Some(instance_id.clone()),
            _ => None,
        });
        added.or(burned)
    }

    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialises")
    }

    #[test]
    fn is_a_0_3_jlockeed_unit_0_6_radiant_whose_base_face_alone_carries_the_defense_ban() {
        crate::register_all();
        let def = crate::card_def(super::ID);
        assert_eq!(def.id, LOBBYIST);
        assert_eq!(js(&def.tags), json!(["Jlockeed"]));
        assert_eq!(
            [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
            [Some(0), Some(3), Some(0), Some(6)]
        );
        let scripts = script();
        assert_eq!(scripts.base.static_flags.as_ref().and_then(|flags| flags.never_defense), Some(true));
        assert!(scripts.radiant.static_flags.as_ref().and_then(|flags| flags.never_defense).is_none());
    }

    mod base {
        use super::*;

        #[test]
        fn s4_1_a_switch_to_defense_position_is_refused() {
            crate::register_all();
            let mut s = board(Board::default());
            let lobbyist = s.unit(P1, 1).expect("the Lobbyist");
            s.expect_refused_with(|s| s.switch_position(&lobbyist), "Defense Position");
            let now = s.unit(P1, 1).expect("the Lobbyist");
            assert_eq!(s.stats(&now).position, Position::Atk);
        }

        #[test]
        fn r20_a_switch_all_effect_leaves_it_in_attack_while_every_other_unit_switches() {
            crate::register_all();
            let mut s = board(Board {
                hand: Some(vec![SWITCH_ALL, FILLER]),
                ..Board::default()
            });
            s.play(SWITCH_ALL, json!({}));
            let lobbyist = s.unit(P1, 1).expect("p1's lane 1");
            assert_eq!(s.stats(&lobbyist).position, Position::Atk);
            let vanilla = s.unit(P1, 2).expect("p1's lane 2");
            assert_eq!(s.stats(&vanilla).position, Position::Def);
            let theirs = s.unit(P2, 1).expect("p2's lane 1");
            assert_eq!(s.stats(&theirs).position, Position::Def);
        }

        #[test]
        fn s4_2_with_0_attack_it_never_attacks() {
            crate::register_all();
            let mut s = board(Board::default());
            let lobbyist = s.unit(P1, 1).expect("the Lobbyist");
            s.expect_refused_with(|s| s.attack(&lobbyist, "hero"), "0 attack");
        }

        #[test]
        fn s6_2_death_adds_a_random_jlockeed_card_to_your_hand_that_costs_0() {
            crate::register_all();
            let mut s = board(Board::default());
            let id = kill_lobbyist(&mut s);
            s.expect_in_zone(LOBBYIST, "graveyard");
            let card = s.card(id.as_deref().unwrap_or("")).clone();
            assert_eq!(card.zone.z(), ZoneName::Hand);
            assert!(POOL.contains(&card.def_id.as_str()));
            assert_eq!(card.cost_override, Some(0));
            assert!(!card.radiant);
            let HandView::Cards(hand) = s.view(P1).you.hand else {
                panic!("your own hand is shown card by card");
            };
            assert!(hand.iter().any(|view| view.instance_id == card.id && view.cost == 0));
        }

        #[test]
        fn s6_2_killed_on_the_opponent_s_turn_its_death_still_adds_the_card_to_your_hand() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "field": [LOBBYIST], "hand": [FILLER] },
                "p2": { "hand": [HIT_JOB, FILLER], "field": [VANILLA] },
            }));
            let target = s.card(LOBBYIST).id.clone();
            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": target }] }));
            let added: Vec<(PlayerId, String)> = s
                .last_events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::AddedToHand { player, instance_id, .. } => Some((*player, instance_id.clone())),
                    _ => None,
                })
                .collect();
            assert_eq!(added.iter().map(|(player, _)| *player).collect::<Vec<_>>(), vec![P1]);
            let card = s.card(added.first().map(|(_, id)| id.as_str()).unwrap_or("")).clone();
            assert_eq!(card.zone, Zone::Hand { player: P1 });
            assert!(POOL.contains(&card.def_id.as_str()));
            assert_eq!(card.cost_override, Some(0));
        }

        #[test]
        fn r278_r387_the_pool_is_exactly_core_n13_n14_classic_n4_and_c_n51_n52_one_tag_never_itself() {
            crate::register_all();
            let mut seen: IndexSet<String> = IndexSet::new();
            for i in 0..80 {
                let mut s = board(Board {
                    seed: Some(format!("lobby-{i}")),
                    ..Board::default()
                });
                let id = kill_lobbyist(&mut s);
                seen.insert(s.card(id.as_deref().unwrap_or("")).def_id.clone());
            }
            let mut seen: Vec<String> = seen.into_iter().collect();
            seen.sort();
            assert_eq!(seen, POOL);
        }

        #[test]
        fn s2_4_r4_a_full_hand_burns_the_card_which_keeps_no_0_price() {
            crate::register_all();
            let mut hand = vec![HIT_JOB];
            hand.extend([FILLER; 10]);
            let mut s = board(Board {
                hand: Some(hand),
                ..Board::default()
            });
            let id = kill_lobbyist(&mut s);
            assert!(s.last_events().iter().any(|event| matches!(event, GameEvent::Burned { .. })));
            let card = s.card(id.as_deref().unwrap_or(""));
            assert_eq!(card.zone.z(), ZoneName::Graveyard);
            assert!(card.cost_override.is_none());
        }

        #[test]
        fn r97_the_opponent_sees_only_that_a_card_reached_your_hand() {
            crate::register_all();
            let mut s = board(Board::default());
            let id = kill_lobbyist(&mut s).unwrap_or_default();
            let theirs = s.view(P2);
            assert!(!serde_json::to_string(&theirs).expect("serialises").contains(&format!("\"{id}\"")));
            let added = theirs
                .events
                .iter()
                .find(|event| matches!(event, GameEvent::AddedToHand { player, .. } if *player == P1));
            assert!(matches!(
                added,
                Some(GameEvent::AddedToHand { instance_id, def_id, .. })
                    if instance_id == "hidden" && def_id == "hidden"
            ));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn s5_2_0_6_with_no_defense_ban_it_may_switch_to_defense_position() {
            crate::register_all();
            let mut s = board(Board {
                radiant: true,
                ..Board::default()
            });
            let lobbyist = s.unit(P1, 1).expect("the Lobbyist");
            s.expect_stats(&lobbyist, json!({ "attack": 0, "health": 6 }));
            s.expect_refused_with(|s| s.attack(&lobbyist, "hero"), "0 attack");
            s.switch_position(&lobbyist);
            let now = s.unit(P1, 1).expect("the Lobbyist");
            assert_eq!(s.stats(&now).position, Position::Def);
        }

        #[test]
        fn r74_death_adds_a_radiant_jlockeed_card_that_costs_0_never_itself() {
            crate::register_all();
            let mut seen: IndexSet<String> = IndexSet::new();
            for i in 0..40 {
                let mut s = board(Board {
                    radiant: true,
                    seed: Some(format!("rlobby-{i}")),
                    ..Board::default()
                });
                let id = kill_lobbyist(&mut s);
                let card = s.card(id.as_deref().unwrap_or(""));
                assert!(card.radiant);
                assert_eq!(card.cost_override, Some(0));
                seen.insert(card.def_id.clone());
            }
            assert!(!seen.contains(LOBBYIST));
            assert!(seen.iter().all(|id| POOL.contains(&id.as_str())));
        }
    }
}
