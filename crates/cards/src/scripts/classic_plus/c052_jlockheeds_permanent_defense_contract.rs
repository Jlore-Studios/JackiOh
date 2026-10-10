//! C+ #52 Jlockheed's Permanent Defense Contract (SPEC §8.7 row 52). (2) Spell, Jlockeed, Epic.
//!   Base:    "For the rest of the game: At the start of your turn, add {cards|random Jlockheed card|…}
//!            to your hand." — cards 1
//!   Radiant: "… add {cards|random Radiant Jlockheed card|…} to your hand. Each costs ({discount}) less."
//!            — cards 1, discount 1
//!   Engine:  "A player effect for the rest of the game (§10.1): a `never`-expiry player modifier on the
//!            caster that acts at their start of turn, among the start-of-turn triggers (R62). The pool
//!            is the non-token `Jlockeed` cards but this one (R387): #13, #14, C #4, C+ #48, C+ #51."
//!
//! `forRestOfGame` (B5 E28, R458) re-enters `delayed` at each of the caster's turn starts with no `self`
//! (R127), so the numbers are read through `param` as the Spell resolves and carried in the entry's
//! data (R594). The pool leaves this card out by its def id (R387); the price lands only on a hand card (§2.4, R4).

use jackioh_engine::effects::{add_random_from_catalog, for_rest_of_game};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-052";

/// §10.6's data is JSON: a number carried in the entry, or the printed value when it is missing.
fn carried(ctx: &EffectContext<'_>, key: &str) -> i32 {
    match ctx.data.get(key).and_then(Value::as_i64) {
        Some(value) => value as i32,
        None => param(ctx, key),
    }
}

fn contract(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let cards = param(&*ctx, "cards");
            let discount = param(&*ctx, "discount");
            let numbers: IndexMap<String, i32> =
                IndexMap::from([("cards".to_string(), cards), ("discount".to_string(), discount)]);
            let face = if radiant { FaceKind::Radiant } else { FaceKind::Base };
            let label = fill_params(&crate::card_def(ID), face, Some(&numbers));
            vec![for_rest_of_game(json_as(json!({
                "step": "contract",
                "label": label,
                "data": { "cards": cards, "discount": discount },
            })))]
        })),
        delayed: Some(hook(move |ctx| {
            let mut args = json!({ "query": { "tags": ["Jlockeed"] }, "count": carried(ctx, "cards") });
            if radiant {
                args["radiant"] = json!(true);
                args["costMod"] = json!(-carried(ctx, "discount"));
            }
            vec![add_random_from_catalog(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: contract(false),
        radiant: contract(true),
    }
}

// C+ #52 — SPEC §8.7 row 52, BUILD M9 Classic+ row C+ 52: a rest-of-game player modifier (nothing on the
// field to remove) adds a random non-token Jlockeed card each start of your turn, never this card (R387);
// two Contracts add two; a full hand burns; hidden from the opponent (R97); count and discount via
// `param()`; radiant: the card is Radiant and costs (1) less (`costMod` −1, floor 0).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const CONTRACT: &str = "classicplus-052";
    const NETHER: &str = "core-088"; // (4) Spell: destroy all permanents
    const FILLER: &str = "core-005";
    const POOL: [&str; 5] = ["classic-004", "classicplus-048", "classicplus-051", "core-013", "core-014"];

    use crate::scenario;

    fn signed(radiant: bool, seed: Option<&str>, contracts: Option<usize>, fillers: Option<usize>) -> Scenario {
        let contract = json!({ "def": CONTRACT, "radiant": radiant });
        let mut hand: Vec<Value> = (0..contracts.unwrap_or(1)).map(|_| contract.clone()).collect();
        hand.extend((0..fillers.unwrap_or(1)).map(|_| json!(FILLER)));
        scenario(json!({
            "seed": seed.unwrap_or("contract"),
            "p1": {
                "hand": hand,
                "library": [FILLER, FILLER, FILLER, FILLER],
                "mana": 10,
            },
            "p2": { "hand": [FILLER, NETHER], "library": [FILLER, FILLER, FILLER], "mana": 10 },
        }))
    }

    fn play_all(s: &mut Scenario) {
        while s.hand(PlayerId::P1).iter().any(|card| card.def_id == CONTRACT) {
            let card = s
                .hand(PlayerId::P1)
                .into_iter()
                .find(|card| card.def_id == CONTRACT)
                .map(|card| card.id)
                .unwrap_or_default();
            s.play(&card, json!({}));
        }
    }

    /// The instance id an `addedToHand` or `burned` event names.
    fn instance_of(event: &GameEvent) -> String {
        match event {
            GameEvent::AddedToHand { instance_id, .. } | GameEvent::Burned { instance_id, .. } => instance_id.clone(),
            _ => String::new(),
        }
    }

    /// The Jlockeed cards p1's turn start added in the last step (before the turn's draw).
    fn delivered(s: &Scenario) -> Vec<GameEvent> {
        s.last_events()
            .iter()
            .filter(|event| match event {
                GameEvent::AddedToHand { player: PlayerId::P1, def_id, .. } => POOL.contains(&def_id.as_str()),
                GameEvent::Burned { def_id, .. } => POOL.contains(&def_id.as_str()),
                _ => false,
            })
            .cloned()
            .collect()
    }

    /// p1 ends their turn and p2 theirs: p1's next turn starts.
    fn next_turn(s: &mut Scenario) {
        s.end_turn().end_turn();
    }

    #[test]
    fn is_a_2_spell_jlockeed() {
        crate::register_all();
        let def = crate::card_def(super::ID);
        assert_eq!(def.type_, CardType::Spell);
        assert_eq!(def.tags, vec![Tag::Jlockeed]);
    }

    mod base {
        use super::*;

        #[test]
        fn r458_it_leaves_a_rest_of_game_modifier_and_nothing_on_the_field() {
            let mut s = signed(false, None, None, None);
            play_all(&mut s);
            s.expect_in_zone(CONTRACT, "graveyard");
            let mods: Vec<&PlayerModifier> = s
                .state()
                .players
                .p1
                .mods
                .iter()
                .filter(|m| matches!(m.kind, ModifierKind::StartOfTurnEffect { .. }))
                .collect();
            assert_eq!(mods.len(), 1);
            assert_eq!(mods[0].expiry, ModifierExpiry::Never);
            // R169: its badge, public to both seats, in the card's own words.
            let label = "For the rest of the game.\nStart of turn: Add 1 random Jlockheed card to your hand.";
            let theirs: Vec<String> = s.view(PlayerId::P2).opponent.modifiers.iter().map(|m| m.label.clone()).collect();
            assert_eq!(theirs, vec![label.to_string()]);
            let mine: Vec<String> = s.view(PlayerId::P1).you.modifiers.iter().map(|m| m.label.clone()).collect();
            assert_eq!(mine, vec![label.to_string()]);
        }

        #[test]
        fn r62_at_each_start_of_your_turn_it_adds_a_random_jlockeed_card_before_the_draw() {
            let mut s = signed(false, None, None, None);
            play_all(&mut s);
            for _turn in 0..3 {
                next_turn(&mut s);
                let cards = delivered(&s);
                assert_eq!(cards.len(), 1);
                assert_eq!(s.card(instance_of(&cards[0])).zone.z(), ZoneName::Hand);
                let events = s.last_events();
                let contract = events.iter().position(|event| *event == cards[0]).map_or(-1, |i| i as i64);
                let drawn = events
                    .iter()
                    .rposition(|event| matches!(event, GameEvent::Drawn { .. }))
                    .map_or(-1, |i| i as i64);
                assert!(contract < drawn);
            }
        }

        #[test]
        fn nothing_at_the_opponents_start_of_turn() {
            let mut s = signed(false, None, None, None);
            play_all(&mut s);
            s.end_turn();
            assert_eq!(s.state().active, PlayerId::P2);
            assert_eq!(delivered(&s), Vec::<GameEvent>::new());
        }

        #[test]
        fn r278_r387_the_pool_is_exactly_core_n13_n14_classic_n4_and_c_n48_n51_never_this_card() {
            let mut seen: IndexSet<String> = IndexSet::new();
            for i in 0..60 {
                let mut s = signed(false, Some(&format!("contract-{i}")), None, None);
                play_all(&mut s);
                next_turn(&mut s);
                for event in delivered(&s) {
                    if let GameEvent::AddedToHand { def_id, .. } | GameEvent::Burned { def_id, .. } = event {
                        seen.insert(def_id);
                    }
                }
            }
            let mut found: Vec<String> = seen.into_iter().collect();
            found.sort();
            assert_eq!(found, POOL.iter().map(|id| id.to_string()).collect::<Vec<_>>());
            let mut all: IndexSet<String> = IndexSet::new();
            for i in 0..30 {
                let mut s = signed(false, Some(&format!("contract-all-{i}")), None, None);
                play_all(&mut s);
                next_turn(&mut s);
                for event in s.last_events() {
                    if let GameEvent::AddedToHand { player: PlayerId::P1, def_id, .. } = event {
                        all.insert(def_id.clone());
                    }
                }
            }
            assert!(!all.contains(CONTRACT));
        }

        #[test]
        fn s10_1_destroying_every_permanent_does_not_end_it() {
            let mut s = signed(false, None, None, None);
            play_all(&mut s);
            s.end_turn();
            let from = s.events().len();
            s.play(NETHER, json!({}));
            // Nether spends p2's last mana, so their turn may end by itself (§2.5).
            if s.state().active == PlayerId::P2 {
                s.end_turn();
            }
            assert_eq!(s.state().active, PlayerId::P1);
            let added = s.events()[from..]
                .iter()
                .filter(|event| {
                    matches!(event, GameEvent::AddedToHand { player: PlayerId::P1, def_id, .. } if POOL.contains(&def_id.as_str()))
                })
                .count();
            assert_eq!(added, 1);
        }

        #[test]
        fn r458_two_contracts_add_two_cards() {
            let mut s = signed(false, None, Some(2), None);
            play_all(&mut s);
            next_turn(&mut s);
            assert_eq!(delivered(&s).len(), 2);
        }

        #[test]
        fn s2_4_r4_a_full_hand_burns_the_card() {
            let mut s = signed(false, None, None, Some(10));
            play_all(&mut s);
            next_turn(&mut s);
            let burned = delivered(&s)
                .into_iter()
                .filter(|event| matches!(event, GameEvent::Burned { .. }))
                .count();
            assert_eq!(burned, 1);
        }

        #[test]
        fn r97_the_opponent_sees_the_add_under_the_sentinel() {
            let mut s = signed(false, None, None, None);
            play_all(&mut s);
            next_turn(&mut s);
            let id = delivered(&s).first().map(instance_of).unwrap_or_else(|| "?".to_string());
            let theirs = s.view(PlayerId::P2);
            assert!(!serde_json::to_string(&theirs).unwrap().contains(&format!("\"{id}\"")));
            let events: Vec<&GameEvent> = theirs
                .events
                .iter()
                .filter(|event| matches!(event, GameEvent::AddedToHand { player: PlayerId::P1, .. }))
                .collect();
            assert!(!events.is_empty());
            for event in events {
                if let GameEvent::AddedToHand { instance_id, def_id, .. } = event {
                    assert_eq!(instance_id, "hidden");
                    assert_eq!(def_id, "hidden");
                }
            }
        }

        #[test]
        fn r594_r386_an_upgrade_before_the_cast_adds_2_each_turn_the_count_is_read_as_it_resolves_and_carried() {
            let mut s = signed(false, None, None, None);
            step_param(s.card_mut(CONTRACT), "cards", 1);
            play_all(&mut s);
            next_turn(&mut s);
            assert_eq!(delivered(&s).len(), 2);
            next_turn(&mut s);
            assert_eq!(delivered(&s).len(), 2);
        }

        #[test]
        fn r113_the_state_survives_a_json_round_trip_and_the_next_turn_start_replays_the_same() {
            let mut s = signed(false, None, None, None);
            play_all(&mut s);
            s.end_turn();
            let revived: GameState = serde_json::from_value(serde_json::to_value(s.state()).unwrap()).unwrap();
            assert_eq!(&revived, s.state());
            let replayed = reduce(&revived, &Action::new(ActionBody::EndTurn, PlayerId::P2, "contract-replay"));
            assert!(replayed.error.is_none());
            s.end_turn();
            assert_eq!(delivered(&s).len(), 1);
            assert_eq!(hash_state(&replayed.state), hash_state(s.state()));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r74_the_card_is_radiant_and_costs_1_less() {
            for i in 0..12 {
                let mut s = signed(true, Some(&format!("rcontract-{i}")), None, None);
                play_all(&mut s);
                next_turn(&mut s);
                let id = delivered(&s).first().map(instance_of).unwrap_or_default();
                let card = s.card(&id);
                assert!(card.radiant);
                assert_eq!(card.cost_mod, -1);
                assert_ne!(card.def_id, CONTRACT);
            }
        }

        #[test]
        fn r97_r177_the_opponent_sees_neither_the_radiant_card_nor_its_price_change() {
            let mut s = signed(true, None, None, None);
            play_all(&mut s);
            next_turn(&mut s);
            let id = delivered(&s).first().map(instance_of).unwrap_or_else(|| "?".to_string());
            assert_eq!(s.card(&id).zone.z(), ZoneName::Hand);
            let theirs = s.view(PlayerId::P2);
            assert!(!serde_json::to_string(&theirs).unwrap().contains(&format!("\"{id}\"")));
            let seen: Vec<&GameEvent> = theirs
                .events
                .iter()
                .filter(|event| {
                    matches!(event, GameEvent::AddedToHand { player: PlayerId::P1, .. } | GameEvent::CostChanged { .. })
                })
                .collect();
            assert!(!seen.is_empty());
            for event in seen {
                if let GameEvent::AddedToHand { instance_id, .. } | GameEvent::CostChanged { instance_id, .. } = event {
                    assert_eq!(instance_id, "hidden");
                }
            }
        }

        #[test]
        fn s2_3_the_discount_floors_at_0_a_lobbyist_1_costs_0() {
            let mut checked = false;
            let mut i = 0;
            while i < 40 && !checked {
                let mut s = signed(true, Some(&format!("rfloor-{i}")), None, None);
                i += 1;
                play_all(&mut s);
                next_turn(&mut s);
                let id = delivered(&s).first().map(instance_of).unwrap_or_default();
                let card = s.card(&id).clone();
                if card.def_id != "classicplus-048" {
                    continue;
                }
                assert_eq!(effective_cost(s.state(), &card, Default::default()), 0);
                checked = true;
            }
            assert!(checked);
        }

        #[test]
        fn r594_r386_count_and_discount_read_through_param_as_it_resolves() {
            let mut s = signed(true, None, None, None);
            step_param(s.card_mut(CONTRACT), "cards", 1);
            step_param(s.card_mut(CONTRACT), "discount", 1);
            play_all(&mut s);
            next_turn(&mut s);
            let cards: Vec<CardInstance> = delivered(&s).iter().map(|event| s.card(instance_of(event)).clone()).collect();
            assert_eq!(cards.len(), 2);
            assert!(cards.iter().all(|card| card.cost_mod == -2 && card.radiant));
        }
    }
}
