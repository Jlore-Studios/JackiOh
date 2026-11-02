//! TG #38 Tutorial Master (SPEC §8.11 row 38): (4) Unit, Human, Legendary, 4/16 → 8/32, a card of the held
//! Training Grounds set (R1420, R2194000).
//!   Base:    "Cry: Add {cards|random Training Grounds card|random Training Grounds cards} to your hand.
//!            They cost ({discount}) less. / Death: the same." — cards 3, discount 1
//!   Radiant: the same with Radiant cards — cards 3, discount 1
//! Engine: S #45 Drone Commander's add, as a Cry and as a Death. The pool names the set (R2194001): its
//!   non-token cards but this one (R387), one match-rng draw per card, repeats allowed (R60), each with a
//!   `costMod` of minus `discount` flooring at (0) (R65); a full hand burns the rest (§2.4). `cards` and
//!   `discount` are read by both hooks. Radiant: added Radiant.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-038";

fn master(radiant: bool) -> Script {
    let add = hook(move |ctx| {
        vec![add_random_from_catalog(json_as(json!({
            "query": { "set": "Training Grounds" },
            "count": param(&*ctx, "cards"),
            "radiant": radiant,
            "costMod": -param(&*ctx, "discount"),
        })))]
    });
    Script {
        cry: Some(add.clone()),
        death: Some(add),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: master(false),
        radiant: master(true),
    }
}

// TG #38 Tutorial Master — SPEC §8.11 row 38, BUILD M13 row TG 38.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const FILLER: &str = "core-005"; // (1) Spell Stockpile: a card in hand, so no turn ends by itself.

    fn game(radiant: bool, seed: &str) -> Scenario {
        crate::scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, HIT_JOB, FILLER],
                "library": crate::filler(4),
                "mana": 30,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }))
    }

    /// The cards the last step added to p1's hand.
    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand { player, instance_id, .. } if *player == P1 => Some(instance_id.clone()),
                _ => None,
            })
            .map(|id| s.card(id.as_str()).clone())
            .collect()
    }

    fn play_master(s: &mut Scenario) -> Vec<CardInstance> {
        s.play(ID, json!({ "zone": 1 }));
        added(s)
    }

    fn kill_master(s: &mut Scenario) -> Vec<CardInstance> {
        let me = s.unit_at(P1, 1).id;
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": me }] }));
        s.expect_in_zone(me.as_str(), "graveyard");
        added(s)
    }

    /// R2194001, R387: a non-token Training Grounds card, never this one, of the face asked, at (1) less.
    fn from_the_pool(card: &CardInstance, radiant: bool) {
        let def = catalog::def_of(None, &card.def_id);
        assert_eq!(def.set, SetName::TrainingGrounds, "{}", card.def_id);
        assert!(!def.token, "{}", card.def_id);
        assert_ne!(card.def_id, ID);
        assert_eq!(card.radiant, radiant);
        assert_eq!(card.cost_mod, -1);
    }

    /// p1's hand as p1 sees it: (instance, cost).
    fn hand_costs(s: &Scenario) -> Vec<(String, i32)> {
        let HandView::Cards(hand) = s.view(P1).you.hand else {
            panic!("your own hand is shown card by card");
        };
        hand.iter().map(|view| (view.instance_id.clone(), view.cost)).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r65_r2194001_its_cry_adds_3_training_grounds_cards_each_1_less_never_itself() {
            let mut s = game(false, "tutorial-master");
            let cards = play_master(&mut s);
            assert_eq!(cards.len(), 3);
            cards.iter().for_each(|card| from_the_pool(card, false));
        }

        #[test]
        fn r2194001_its_death_adds_3_more() {
            let mut s = game(false, "tutorial-master");
            play_master(&mut s);
            let cards = kill_master(&mut s);
            assert_eq!(cards.len(), 3);
            cards.iter().for_each(|card| from_the_pool(card, false));
        }

        #[test]
        fn r60_r387_repeats_are_allowed_and_it_never_adds_itself() {
            for n in 0..64 {
                let mut s = game(false, &format!("tutorial-master-{n}"));
                let mut cards = play_master(&mut s);
                cards.extend(kill_master(&mut s));
                cards.iter().for_each(|card| from_the_pool(card, false));
                let mut defs: Vec<&str> = cards.iter().map(|card| card.def_id.as_str()).collect();
                defs.sort_unstable();
                if defs.windows(2).any(|pair| pair[0] == pair[1]) {
                    return;
                }
            }
            panic!("no seed drew a repeat");
        }

        #[test]
        fn r65_a_0_cost_card_stays_0() {
            for n in 0..64 {
                let mut s = game(false, &format!("tutorial-master-free-{n}"));
                let cards = play_master(&mut s);
                let costs = hand_costs(&s);
                let free: Vec<&CardInstance> = cards
                    .iter()
                    .filter(|card| printed_cost(s.state(), card) == 0)
                    .collect();
                if let Some(card) = free.first() {
                    assert!(costs.contains(&(card.id.clone(), 0)), "{costs:?}");
                    return;
                }
            }
            panic!("no seed drew a (0) card");
        }

        #[test]
        fn cards_and_discount_read_through_param() {
            let mut s = game(false, "tutorial-master");
            let me = s.card(ID).id.clone();
            assert_eq!(crate::upgrade_number(&mut s, &me, "cards"), 4);
            assert_eq!(crate::upgrade_number(&mut s, &me, "discount"), 2);
            let cards = play_master(&mut s);
            assert_eq!(cards.len(), 4);
            assert!(cards.iter().all(|card| card.cost_mod == -2));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r2194001_it_is_8_32_and_its_cry_and_death_add_radiant_cards() {
            let mut s = game(true, "tutorial-master");
            let cards = play_master(&mut s);
            let me = s.unit_at(P1, 1);
            s.expect_stats(&me, json!({ "attack": 8, "maxHealth": 32 }));
            assert_eq!(cards.len(), 3);
            cards.iter().for_each(|card| from_the_pool(card, true));
            let cards = kill_master(&mut s);
            assert_eq!(cards.len(), 3);
            cards.iter().for_each(|card| from_the_pool(card, true));
        }
    }
}
