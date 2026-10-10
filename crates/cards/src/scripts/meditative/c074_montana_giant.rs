//! M #74 Montaña Giant (SPEC §8.8 row 74): (8) Unit, Epic, 8/8 → 16/16.
//!   Base:    "Costs ({discount}) less for each card in your hand.\nEnd of turn: If your hand is full, make this Radiant."
//!   Radiant: "Costs ({discount}) less for each card in your hand.\nEnd of turn: If your hand is full, this gains +{buff}/+{buff}."
//! Engine: a cost hook as C+ #64 Mulch Muncher's (R65, floor 0), counting the hand itself included
//!   (MD-D33, R1065); at its controller's end of turn with a full hand the base face makes itself
//!   Radiant (§5.2) and the Radiant face buffs +8/+8.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-074";

fn giant_cost() -> CostHook {
    cost_hook(move |CostArgs { state, instance }| {
        let printed = query_cost(&crate::card_def(ID));
        let at = &instance.zone;
        let for_play = matches!(at, Zone::Hand { .. })
            || (matches!(at, Zone::Graveyard { .. }) && playable_from_graveyard(state, instance));
        if !for_play {
            return printed;
        }
        let discount = param(
            &HookArgs { state, self_: instance, radiant: instance.radiant },
            "discount",
        );
        (printed - discount * zone_count(state, at.player(), OffFieldZone::Hand)).max(0)
    })
}

fn giant_end(radiant_face: bool) -> Hook {
    hook(move |ctx| {
        if zone_count(ctx.state, ctx.controller, OffFieldZone::Hand) < HAND_CAP {
            return vec![];
        }
        if radiant_face {
            let gain = param(&*ctx, "buff");
            vec![buff(json_as(json!({ "target": { "of": "self" }, "attack": gain, "health": gain })))]
        } else {
            vec![set_radiant(json_as(json!({ "target": { "of": "self" } })))]
        }
    })
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cost: Some(giant_cost()),
            end_of_turn: Some(giant_end(false)),
            ..Script::default()
        },
        radiant: Script {
            cost: Some(giant_cost()),
            end_of_turn: Some(giant_end(true)),
            ..Script::default()
        },
    }
}

// M #74 Montaña Giant — SPEC §8.8 row 74, BUILD M10 row M 74.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const GIANT: &str = "meditative-074";
    const FILLER: &str = "core-005";

    fn hand_cost(s: &Scenario, player: PlayerId) -> i32 {
        let card = s
            .hand(player)
            .into_iter()
            .find(|held| held.def_id == GIANT)
            .unwrap_or_else(|| panic!("no Giant in hand"));
        effective_cost(s.state(), &card, Default::default())
    }

    #[test]
    fn r1065_costs_8_less_one_per_card_in_hand_itself_included() {
        crate::register_all();
        // Alone: 8 - 1 = 7.
        let s = scenario(json!({ "p1": { "hand": [GIANT] }, "p2": { "hand": [FILLER] } }));
        assert_eq!(hand_cost(&s, PlayerId::P1), 7);
        // With four cards: 8 - 4 = 4.
        let s = scenario(json!({ "p1": { "hand": [GIANT, FILLER, FILLER, FILLER] }, "p2": { "hand": [FILLER] } }));
        assert_eq!(hand_cost(&s, PlayerId::P1), 4);
        // With ten: floored at 0.
        let mut hand = vec![json!(GIANT)];
        hand.extend((0..9).map(|_| json!(FILLER)));
        let s = scenario(json!({ "p1": { "hand": hand }, "p2": { "hand": [FILLER] } }));
        assert_eq!(hand_cost(&s, PlayerId::P1), 0);
    }

    #[test]
    fn r1065_full_hand_at_your_end_of_turn_it_becomes_radiant_damage_and_buffs_kept() {
        crate::register_all();
        let hand = vec![json!(FILLER); 10];
        let mut s = scenario(json!({
            "p1": { "hand": hand, "field": [{ "def": GIANT, "lane": 1 }], "mana": 10 },
            "p2": { "hand": [FILLER] },
        }));
        // Damage 3 kept through the Radiant layer swap.
        let id = s.unit(PlayerId::P1, 1).expect("giant").id.clone();
        find_instance_mut(s.state_mut(), &id).expect("giant").damage = 3;
        s.end_turn();
        let giant = s.card(GIANT).clone();
        assert!(giant.radiant);
        s.expect_stats(GIANT, json!({ "attack": 16, "health": 13, "maxHealth": 16 }));
    }

    #[test]
    fn r1065_nine_cards_it_stays_base() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [FILLER, FILLER], "field": [{ "def": GIANT, "lane": 1 }] },
            "p2": { "hand": [FILLER] },
        }));
        s.end_turn();
        assert!(!s.card(GIANT).radiant);
    }

    #[test]
    fn r386_discount_reads_through_param() {
        crate::register_all();
        let mut s = scenario(json!({ "p1": { "hand": [GIANT] }, "p2": { "hand": [FILLER] } }));
        step_param(s.card_mut(GIANT), "discount", 1);
        assert_eq!(hand_cost(&s, PlayerId::P1), 6);
    }

    #[test]
    fn radiant_gains_8_8_at_each_full_hand_end_of_turn() {
        crate::register_all();
        let hand = vec![json!(FILLER); 10];
        let mut s = scenario(json!({
            "p1": { "hand": hand, "field": [{ "def": GIANT, "lane": 1, "radiant": true }] },
            "p2": { "hand": [FILLER] },
        }));
        s.end_turn();
        s.expect_stats(GIANT, json!({ "attack": 24, "maxHealth": 24 }));
    }

    #[test]
    fn r386_radiant_buff_reads_through_param() {
        crate::register_all();
        let hand = vec![json!(FILLER); 10];
        let mut s = scenario(json!({
            "p1": { "hand": hand, "field": [{ "def": GIANT, "lane": 1, "radiant": true }] },
            "p2": { "hand": [FILLER] },
        }));
        step_param(s.card_mut(GIANT), "buff", 2);
        s.end_turn();
        s.expect_stats(GIANT, json!({ "attack": 26, "maxHealth": 26 }));
    }
}
