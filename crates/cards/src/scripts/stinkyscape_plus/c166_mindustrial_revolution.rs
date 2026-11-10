//! SPEC §8.12 S+ #166 Mindustrial Revolution (§10.1 player effects; R62, R65, R127, R458, R1420, R3535000,
//! R3549004): (2) Spell, Mindustrial, Rare, a card of the held Stinkyscape+ set.
//!   Base:    "For the rest of the game at the start of its owner's turn add a random Mindustrial card to
//!            your hand."
//!   Radiant: "… add a random Radiant Mindustrial card to your hand, it costs (1) less."
//!
//! Engine: C+ #52's player effect for the rest of the game (R458): a `never`-expiry modifier on the caster
//! that acts at their start of turn among the start-of-turn triggers (R62), before the draw, and never on
//! the opponent's turns (R3549004, "its owner's"). Each turn it adds one card from the Stinkyscape+ cards
//! tagged Mindustrial, tokens out (§5.1), the pool naming its set so that it reaches them while the set is
//! held (R1420). The Spell goes to the graveyard and the effect is the player's, so no board wipe ends it.
//! The Radiant face adds Radiant cards that cost `DISCOUNT` less, never below (0) (R65), and only on a
//! card that reached the hand (§2.4). `forRestOfGame` re-enters `delayed` with no `self` (R127); the
//! re-entry carries this card's def id, which the verb leaves out of the pool (R387), this card being
//! Mindustrial itself. The badge is the face as the catalog the game runs under prints it (`def_of`), so a
//! replay of an older version badges it in that version's words (R767000).

use jackioh_engine::effects::for_rest_of_game;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-166";

/// "add a random Mindustrial card" at each of its owner's turn starts (R1748003: printed, declared nowhere).
const CARDS: i32 = 1;
/// "it costs (1) less" on the Radiant face (R1748003: printed, declared nowhere).
const DISCOUNT: i32 = 1;
/// The rest-of-game effect's step (§10.6).
const STEP: &str = "revolution";

/// R3549004: "a random Mindustrial card", the held set's.
fn mindustrial() -> CatalogQueryArgs {
    json_as(json!({ "set": SetName::StinkyscapePlus, "tags": [Tag::Mindustrial] }))
}

fn revolution(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let face = if radiant { FaceKind::Radiant } else { FaceKind::Base };
            let label = fill_params(def_of(Some(&*ctx.state), ID), face, None);
            vec![for_rest_of_game(json_as(json!({ "step": STEP, "label": label })))]
        })),
        delayed: Some(hook(move |_ctx| {
            let mut args = json!({ "query": mindustrial(), "count": CARDS });
            if radiant {
                args["radiant"] = json!(true);
                args["costMod"] = json!(-DISCOUNT);
            }
            vec![add_random_from_catalog(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: revolution(false),
        radiant: revolution(true),
    }
}

// S+ #166 Mindustrial Revolution — SPEC §8.12 row S+ 166 (R3549004; R62, R458).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, weapon_board};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005";
    const NETHER: &str = "core-088"; // Twisting Nether: destroy all permanents

    fn holding(radiant: bool, seed: &str, copies: usize, fillers: usize) -> Scenario {
        let mut hand: Vec<Value> = (0..copies).map(|_| json!({ "def": ID, "radiant": radiant })).collect();
        hand.extend((0..fillers).map(|_| json!(FILLER)));
        weapon_board(
            seed,
            json!({ "hand": hand, "library": filler(6), "mana": 10 }),
            json!({ "hand": [FILLER, NETHER], "library": filler(6), "mana": 10 }),
        )
    }

    fn signed(radiant: bool, seed: &str, fillers: usize) -> Scenario {
        holding(radiant, seed, 1, fillers)
    }

    /// Plays every Revolution in p1's hand.
    fn cast(s: &mut Scenario) {
        while let Some(card) = s.hand(P1).into_iter().find(|card| card.def_id == ID) {
            s.play(card.id.as_str(), json!({}));
        }
    }

    fn next_turn(s: &mut Scenario) {
        s.end_turn().end_turn();
    }

    /// Whether `def_id` is a card of the pool: a non-token Stinkyscape+ card tagged Mindustrial.
    fn in_pool(def_id: &str) -> bool {
        let def = crate::card_def(def_id);
        def.set == SetName::StinkyscapePlus && def.tags.contains(&Tag::Mindustrial) && !def.is_token()
    }

    /// The pool cards p1's turn start added or burned in the last step.
    fn delivered(s: &Scenario) -> Vec<GameEvent> {
        s.last_events()
            .iter()
            .filter(|event| match event {
                GameEvent::AddedToHand { player: P1, def_id, .. } => in_pool(def_id),
                GameEvent::Burned { def_id, .. } => in_pool(def_id),
                _ => false,
            })
            .cloned()
            .collect()
    }

    fn instance_of(event: &GameEvent) -> String {
        match event {
            GameEvent::AddedToHand { instance_id, .. } | GameEvent::Burned { instance_id, .. } => instance_id.clone(),
            _ => String::new(),
        }
    }

    fn labels(s: &Scenario) -> Vec<String> {
        s.state()
            .players
            .p1
            .mods
            .iter()
            .filter_map(|m| match &m.kind {
                ModifierKind::StartOfTurnEffect { label, .. } => Some(label.clone()),
                _ => None,
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r3549004_a_rest_of_game_modifier_adds_one_mindustrial_before_each_owner_draw() {
            let mut s = signed(false, "s166-base", 0);
            cast(&mut s);
            s.expect_in_zone(ID, "graveyard");
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
            let label = fill_params(&crate::card_def(ID), FaceKind::Base, None);
            assert!(label.contains("random Mindustrial card"), "{label}");
            assert_eq!(labels(&s), vec![label.clone()]);
            let theirs: Vec<String> = s.view(P2).opponent.modifiers.iter().map(|m| m.label.clone()).collect();
            assert_eq!(theirs, vec![label.clone()]);
            let mine: Vec<String> = s.view(P1).you.modifiers.iter().map(|m| m.label.clone()).collect();
            assert_eq!(mine, vec![label]);
            for _turn in 0..3 {
                next_turn(&mut s);
                let cards = delivered(&s);
                assert_eq!(cards.len(), CARDS as usize);
                let card = s.card(instance_of(&cards[0])).clone();
                assert_eq!(card.zone.z(), ZoneName::Hand);
                assert!(!card.radiant);
                assert_eq!(card.cost_mod, 0);
                let events = s.last_events();
                let added = events.iter().position(|event| *event == cards[0]).map_or(-1, |i| i as i64);
                let drawn = events
                    .iter()
                    .rposition(|event| matches!(event, GameEvent::Drawn { .. }))
                    .map_or(-1, |i| i as i64);
                assert!(added < drawn, "before the turn's draw");
            }
        }

        #[test]
        fn nothing_on_the_opponents_turns() {
            let mut s = signed(false, "s166-opp", 0);
            cast(&mut s);
            s.end_turn();
            assert_eq!(s.state().active, P2);
            assert_eq!(delivered(&s), Vec::<GameEvent>::new());
            assert!(s.view(P2).opponent.modifiers.len() == 1, "the badge stays on its owner");
        }

        #[test]
        fn r3549004_the_pool_is_the_held_sets_mindustrial_cards_never_this_one() {
            let mut seen: IndexSet<String> = IndexSet::new();
            for i in 0..100 {
                let mut s = signed(false, &format!("s166-pool-{i}"), 0);
                cast(&mut s);
                for _turn in 0..3 {
                    next_turn(&mut s);
                    for event in delivered(&s) {
                        if let GameEvent::AddedToHand { def_id, .. } | GameEvent::Burned { def_id, .. } = event {
                            seen.insert(def_id);
                        }
                    }
                }
            }
            assert!(seen.len() > 30, "300 draws cover much of the pool: {}", seen.len());
            assert!(!seen.contains(ID), "R387: a Mindustrial card itself, it never adds itself");
        }

        #[test]
        fn s10_1_destroying_every_permanent_does_not_end_it() {
            let mut s = signed(false, "s166-nether", 0);
            cast(&mut s);
            s.end_turn();
            let from = s.events().len();
            s.play(NETHER, json!({}));
            // Nether spends p2's last mana, so their turn may end by itself (§2.5).
            if s.state().active == P2 {
                s.end_turn();
            }
            assert_eq!(s.state().active, P1);
            let added = s.events()[from..]
                .iter()
                .filter(|event| matches!(event, GameEvent::AddedToHand { player: P1, def_id, .. } if in_pool(def_id)))
                .count();
            assert_eq!(added, CARDS as usize);
        }

        #[test]
        fn r458_two_revolutions_add_two_cards() {
            let mut s = holding(false, "s166-two", 2, 0);
            cast(&mut s);
            assert_eq!(labels(&s).len(), 2);
            next_turn(&mut s);
            assert_eq!(delivered(&s).len(), 2 * CARDS as usize);
        }

        #[test]
        fn s2_4_r4_a_full_hand_burns_the_card() {
            let mut s = signed(false, "s166-full", 10);
            cast(&mut s);
            next_turn(&mut s);
            let burned = delivered(&s).into_iter().filter(|event| matches!(event, GameEvent::Burned { .. })).count();
            assert_eq!(burned, CARDS as usize);
        }

        #[test]
        fn r113_the_state_survives_a_json_round_trip_and_the_next_turn_start_replays_the_same() {
            let mut s = signed(false, "s166-json", 0);
            cast(&mut s);
            s.end_turn();
            let revived: GameState = serde_json::from_value(serde_json::to_value(s.state()).unwrap()).unwrap();
            assert_eq!(&revived, s.state());
            let replayed = reduce(&revived, &Action::new(ActionBody::EndTurn, P2, "revolution-replay"));
            assert!(replayed.error.is_none());
            s.end_turn();
            assert_eq!(delivered(&s).len(), CARDS as usize);
            assert_eq!(hash_state(&replayed.state), hash_state(s.state()));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3549004_the_cards_are_radiant_and_cost_1_less_and_the_badge_says_so() {
            let mut s = signed(true, "s166-radiant", 0);
            cast(&mut s);
            let label = fill_params(&crate::card_def(ID), FaceKind::Radiant, None);
            assert!(label.contains("costs (1) less"), "{label}");
            assert_eq!(labels(&s), vec![label]);
            for _turn in 0..3 {
                next_turn(&mut s);
                let cards = delivered(&s);
                assert_eq!(cards.len(), CARDS as usize);
                let card = s.card(instance_of(&cards[0])).clone();
                assert!(card.radiant);
                assert_eq!(card.cost_mod, -DISCOUNT);
            }
        }

        #[test]
        fn s2_3_the_discount_floors_at_0() {
            let seed = seed_where("s166-floor", |seed| {
                let mut s = signed(true, seed, 0);
                cast(&mut s);
                next_turn(&mut s);
                delivered(&s)
                    .first()
                    .is_some_and(|event| crate::card_def(&s.card(instance_of(event)).def_id).cost == CardCost::Fixed(0))
            });
            let mut s = signed(true, &seed, 0);
            cast(&mut s);
            next_turn(&mut s);
            let card = s.card(delivered(&s).first().map(instance_of).unwrap_or_default()).clone();
            assert_eq!(effective_cost(s.state(), &card, Default::default()), 0, "{}", card.def_id);
        }

        #[test]
        fn s2_4_r4_a_full_hand_burns_the_radiant_card_too() {
            let mut s = signed(true, "s166-rfull", 10);
            cast(&mut s);
            next_turn(&mut s);
            let burned = delivered(&s).into_iter().filter(|event| matches!(event, GameEvent::Burned { .. })).count();
            assert_eq!(burned, CARDS as usize);
        }
    }
}
