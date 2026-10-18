//! The Meditative+ spell-power systems (issue #1180, docs/meditative-plus-set.md MP5), proved on
//! fixture cards (`fixtures::spell_power`) before the real cards use them:
//!
//! - R1180000: a player's Spell Damage is one sum — their Units' keyword, every Spell Damage aura acting
//!   on either side that reaches them (one player or both), and their "this turn" modifiers — read as
//!   each hit of a Spell lands; a Field Spell's or a Trap's hit is never raised.
//! - R1180001: "Spell Damage +N this turn" is the activator's, stacks, stays when its card leaves and
//!   ends at the turn's cleanup.
//! - R1180002: "Your Spells have Lifesteal" reaches its controller's resolving Spells only, once per hit,
//!   once however many give it, and ends when its card leaves.
//! - R1180003: the healing multiplier doubles every heal of its controller's hero and Units, Lifesteal's
//!   included, two quadruple, the opponent's heals untouched, and a heal a replacement turned into
//!   damage, or a "heal up to N", is not doubled.
//! - R1180004: health as the next Spell's price.
//! - R1180005: the targets a Spell declared or chose kept, and the one card in its player's hand or deck
//!   that answers it.

use jackioh_engine::damage::spell_damage_of;
use jackioh_engine::effects::{self, damage, heal};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::sink_for;
use crate::rules::fixtures::spell_power::{
    AMP, AMP_DECLARED, AMP_OWN, ASKER, BLOOM, BLOOM_ECHO, BODY, BOLT, DISCOUNT, DOUBLER, ECHO_BOLT,
    FIELD_ZAP, MAGE, MEND, MOON, NOTHING, PAWN, PILER, POTION, TRAP, WIZARD, XBOLT, ZAP, register,
};

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

/// A board for the side to act, a pawn on each side and a stocked library each, so no turn auto-ends
/// (§2.5) and no draw fatigues; `p1` and `p2` add their own entries over the defaults.
fn game(seed: &str, p1: Value, p2: Value) -> Scenario {
    register();
    let mut sides = json!({
        "seed": seed,
        "p1": { "field": [{ "def": PAWN, "lane": 5 }], "library": [PAWN, PAWN, PAWN, PAWN] },
        "p2": { "field": [{ "def": PAWN, "lane": 5 }], "library": [PAWN, PAWN, PAWN, PAWN] },
    });
    for (seat, extra) in [("p1", p1), ("p2", p2)] {
        if let Value::Object(entries) = extra {
            for (key, value) in entries {
                let side = sides[seat].as_object_mut().expect("a side is an object");
                match (side.get_mut(&key), value) {
                    (Some(Value::Array(list)), Value::Array(more)) => {
                        // A side's own entries come first, so the defaults never take a lane it names.
                        let mut merged = more;
                        merged.extend(list.drain(..));
                        *list = merged;
                    }
                    (_, value) => {
                        side.insert(key, value);
                    }
                }
            }
        }
    }
    scenario(sides)
}

fn health(s: &Scenario, player: PlayerId) -> i32 {
    s.state().players[player].hero.health
}

fn hero(player: PlayerId) -> Value {
    json!([{ "pick": "hero", "player": player.as_str() }])
}

fn instance(card: &CardInstance) -> Value {
    json!([{ "pick": "instance", "instanceId": card.id }])
}

fn unit_at(s: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
    s.unit(player, lane).expect("a unit stands in that lane")
}

fn hand_card(s: &Scenario, player: PlayerId, def_id: &str) -> CardInstance {
    s.hand(player)
        .into_iter()
        .find(|card| card.def_id == def_id)
        .expect("the card is in hand")
}

/// Every hit's amount, in order, on the target `target_id` names.
fn hits_on(s: &Scenario, target_id: &str) -> Vec<i32> {
    s.events()
        .iter()
        .filter_map(|event| match event {
            GameEvent::Damage {
                target_id: hit,
                amount,
                ..
            } if hit == target_id => Some(*amount),
            _ => None,
        })
        .collect()
}

fn heals_on(s: &Scenario, target_id: &str) -> Vec<i32> {
    s.events()
        .iter()
        .filter_map(|event| match event {
            GameEvent::Healed {
                target_id: healed,
                amount,
            } if healed == target_id => Some(*amount),
            _ => None,
        })
        .collect()
}

fn mods_of(s: &Scenario, player: PlayerId, pick: fn(&ModifierKind) -> bool) -> usize {
    s.state().players[player]
        .mods
        .iter()
        .filter(|modifier| pick(&modifier.kind))
        .count()
}

fn health_prices(s: &Scenario, player: PlayerId) -> usize {
    mods_of(s, player, |kind| matches!(kind, ModifierKind::HealthPrice { .. }))
}

/// Apply `effects` with `source` as the running card, controlled by `player`, then settle (§10.3: the
/// state check collects what they destroyed).
fn apply_as(s: &mut Scenario, source: Option<&CardInstance>, player: PlayerId, effects: &[Effect]) {
    let mut sink = sink_for(s.state_mut());
    {
        let mut ctx = make_context(
            &mut sink,
            source,
            HookOptions {
                controller: Some(player),
                ..Default::default()
            },
        );
        apply_effects(effects, &mut ctx);
    }
    settle(&mut sink, SettleOptions::default());
}

fn badges(s: &Scenario, player: PlayerId) -> Vec<String> {
    s.view(player)
        .you
        .modifiers
        .into_iter()
        .map(|badge| badge.label)
        .collect()
}

// ---- R1180000: Spell Damage beyond Units ----

mod r1180000_spell_damage_is_one_sum {
    use super::*;

    #[test]
    fn r1180000_an_aura_for_both_players_raises_either_players_spell_hits() {
        let mut s = game(
            "sp-amp-both",
            json!({ "backrow": [AMP], "hand": [BOLT] }),
            json!({}),
        );
        assert_eq!(spell_damage_of(s.state(), P1), 2);
        assert_eq!(spell_damage_of(s.state(), P2), 2);
        s.play(BOLT, json!({ "targets": hero(P2) }));
        assert_eq!(hits_on(&s, "hero-p2"), vec![3]);
    }

    #[test]
    fn r1180000_the_opponent_is_raised_by_an_aura_they_do_not_control() {
        let mut s = game(
            "sp-amp-theirs",
            json!({ "backrow": [AMP] }),
            json!({ "hand": [BOLT] }),
        );
        s.end_turn();
        s.play(BOLT, json!({ "targets": hero(P1) }));
        assert_eq!(hits_on(&s, "hero-p1"), vec![3]);
    }

    #[test]
    fn r1180000_an_aura_for_one_player_reaches_its_controller_alone() {
        let s = game("sp-amp-own", json!({ "backrow": [AMP_OWN] }), json!({}));
        assert_eq!(spell_damage_of(s.state(), P1), 3);
        assert_eq!(spell_damage_of(s.state(), P2), 0);
    }

    #[test]
    fn r1180000_it_sums_with_units_spell_damage_and_the_modifier() {
        let mut s = game(
            "sp-amp-sum",
            json!({ "backrow": [AMP], "field": [WIZARD, POTION], "hand": [BOLT] }),
            json!({}),
        );
        s.activate(POTION, json!({}));
        // Units' 1, the aura's 2, the modifier's 5.
        assert_eq!(spell_damage_of(s.state(), P1), 8);
        s.play(BOLT, json!({ "targets": hero(P2) }));
        assert_eq!(hits_on(&s, "hero-p2"), vec![9]);
    }

    #[test]
    fn r1180000_a_field_spell_or_trap_hit_is_not_raised() {
        let mut s = game(
            "sp-amp-field",
            json!({ "backrow": [AMP, { "def": TRAP, "faceUp": true }], "hand": [FIELD_ZAP] }),
            json!({}),
        );
        s.play(FIELD_ZAP, json!({}));
        assert_eq!(hits_on(&s, "hero-p2"), vec![1]);
        let trap = s.backrow(P1, 2).expect("the trap");
        apply_as(
            &mut s,
            Some(&trap),
            P1,
            &[damage(json_as(
                json!({ "to": { "of": "enemyHero" }, "amount": 1 }),
            ))],
        );
        assert_eq!(health(&s, P2), 30 - 1 - 1);
    }

    #[test]
    fn r1180000_face_down_or_gone_it_gives_nothing() {
        let mut s = game("sp-amp-gone", json!({ "backrow": [AMP] }), json!({}));
        let amp = s.backrow(P1, 1).expect("the amplifier");
        // Set face-down, it is a Trap its opponent may not read, and acts for nobody (R33).
        s.card_mut(&amp.id).set_as = Some(SetAs {
            reveal: RevealAt::Cloaked,
            set_turn: 1,
            echo: None,
            revealing: None,
        });
        s.card_mut(&amp.id).face_up = Some(false);
        assert_eq!(spell_damage_of(s.state(), P1), 0);
        s.card_mut(&amp.id).set_as = None;
        s.card_mut(&amp.id).face_up = Some(true);
        assert_eq!(spell_damage_of(s.state(), P2), 2);
        apply_as(
            &mut s,
            None,
            P1,
            &[effects::destroy(json_as(
                json!({ "target": { "of": "instance", "instanceId": amp.id } }),
            ))],
        );
        assert_eq!(spell_damage_of(s.state(), P1), 0);
        assert_eq!(spell_damage_of(s.state(), P2), 0);
    }

    #[test]
    fn r1180000_the_amount_moves_with_the_cards_declared_number() {
        let mut s = game("sp-amp-declared", json!({ "backrow": [AMP_DECLARED] }), json!({}));
        assert_eq!(spell_damage_of(s.state(), P1), 4);
        let amp = s.backrow(P1, 1).expect("the amplifier").id.clone();
        // A Buff, a Nerf or KY's Constant moves the declared number, and the aura with it (R386).
        set_param(s.card_mut(&amp), "spellDamage", 6);
        assert_eq!(spell_damage_of(s.state(), P1), 6);
        assert_eq!(spell_damage_of(s.state(), P2), 6);
    }

    #[test]
    fn r1180000_a_game_without_one_reads_its_units_alone() {
        let s = game("sp-amp-none", json!({ "field": [WIZARD] }), json!({}));
        assert_eq!(spell_damage_of(s.state(), P1), 1);
        assert_eq!(spell_damage_of(s.state(), P2), 0);
    }
}

// ---- R1180001: "Spell Damage +N this turn" ----

mod r1180001_spell_damage_this_turn {
    use super::*;

    fn spell_damage_mods(s: &Scenario, player: PlayerId) -> usize {
        mods_of(s, player, |kind| matches!(kind, ModifierKind::SpellDamage { .. }))
    }

    #[test]
    fn r1180001_it_is_the_activators_until_the_turns_cleanup() {
        let mut s = game(
            "sp-potion-turn",
            json!({ "field": [POTION], "hand": [BOLT, BOLT] }),
            json!({ "hand": [PAWN] }),
        );
        s.activate(POTION, json!({}));
        assert_eq!(spell_damage_of(s.state(), P1), 5);
        assert_eq!(spell_damage_of(s.state(), P2), 0);
        assert!(badges(&s, P1).contains(&"You have Spell Damage +5 this turn".to_string()));
        s.play(BOLT, json!({ "targets": hero(P2) }));
        assert_eq!(hits_on(&s, "hero-p2"), vec![6]);
        s.end_turn();
        assert_eq!(spell_damage_mods(&s, P1), 0);
        assert_eq!(spell_damage_of(s.state(), P1), 0);
    }

    #[test]
    fn r1180001_two_stack_and_stay_when_the_card_leaves() {
        let mut s = game(
            "sp-potion-stack",
            json!({ "field": [POTION], "hand": [BOLT] }),
            json!({}),
        );
        s.activate(POTION, json!({}));
        s.activate(POTION, json!({}));
        assert_eq!(spell_damage_mods(&s, P1), 2);
        let potion = unit_at(&s, P1, 1);
        apply_as(
            &mut s,
            None,
            P1,
            &[effects::destroy(json_as(
                json!({ "target": { "of": "instance", "instanceId": potion.id } }),
            ))],
        );
        assert_eq!(spell_damage_of(s.state(), P1), 10);
        s.play(BOLT, json!({ "targets": hero(P2) }));
        assert_eq!(hits_on(&s, "hero-p2"), vec![11]);
    }
}

// ---- R1180002: "Your Spells have Lifesteal" ----

mod r1180002_spells_have_lifesteal {
    use super::*;

    #[test]
    fn r1180002_a_spell_of_its_controller_heals_their_hero_once_per_hit() {
        let mut s = game(
            "sp-mage-hits",
            json!({ "field": [MAGE], "hand": [BOLT, ZAP], "health": 20, "mana": 9 }),
            json!({}),
        );
        s.play(BOLT, json!({ "targets": hero(P2) }));
        assert_eq!(health(&s, P1), 21);
        s.play(ZAP, json!({}));
        assert_eq!(heals_on(&s, "hero-p1"), vec![1, 1, 1]);
        assert_eq!(health(&s, P1), 23);
    }

    #[test]
    fn r1180002_two_give_it_once() {
        let mut s = game(
            "sp-mage-two",
            json!({ "field": [MAGE, MAGE], "hand": [BOLT], "health": 20 }),
            json!({}),
        );
        s.play(BOLT, json!({ "targets": hero(P2) }));
        assert_eq!(heals_on(&s, "hero-p1"), vec![1]);
    }

    #[test]
    fn r1180002_the_opponents_spells_a_field_spell_and_a_trap_take_nothing() {
        let mut s = game(
            "sp-mage-not",
            json!({ "field": [MAGE], "backrow": [{ "def": TRAP, "faceUp": true }], "hand": [FIELD_ZAP], "health": 20 }),
            json!({ "hand": [BOLT], "health": 20 }),
        );
        s.play(FIELD_ZAP, json!({}));
        let trap = s.backrow(P1, 1).expect("the trap");
        apply_as(
            &mut s,
            Some(&trap),
            P1,
            &[damage(json_as(
                json!({ "to": { "of": "enemyHero" }, "amount": 1 }),
            ))],
        );
        assert_eq!(health(&s, P1), 20);
        s.end_turn();
        s.play(BOLT, json!({ "targets": hero(P1) }));
        assert_eq!(health(&s, P2), 18);
        assert!(heals_on(&s, "hero-p2").is_empty());
    }

    #[test]
    fn r1180002_it_ends_when_its_card_leaves() {
        let mut s = game(
            "sp-mage-gone",
            json!({ "field": [MAGE], "hand": [BOLT], "health": 20 }),
            json!({}),
        );
        let mage = unit_at(&s, P1, 1);
        apply_as(
            &mut s,
            None,
            P1,
            &[effects::destroy(json_as(
                json!({ "target": { "of": "instance", "instanceId": mage.id } }),
            ))],
        );
        s.play(BOLT, json!({ "targets": hero(P2) }));
        assert_eq!(health(&s, P1), 20);
    }
}

// ---- R1180003: the healing multiplier ----

mod r1180003_healing_twice_as_effective {
    use super::*;

    #[test]
    fn r1180003_a_heal_of_its_controllers_hero_or_unit_is_doubled() {
        let mut s = game(
            "sp-double-heal",
            json!({ "field": [DOUBLER, { "def": BODY, "damage": 8 }], "hand": [MEND, MEND], "health": 10, "mana": 9 }),
            json!({}),
        );
        s.play(MEND, json!({ "targets": hero(P1) }));
        assert_eq!(health(&s, P1), 16);
        let body = unit_at(&s, P1, 2);
        s.play(hand_card(&s, P1, MEND), json!({ "targets": instance(&body) }));
        assert_eq!(s.card(&body.id).damage, 2);
    }

    #[test]
    fn r1180003_lifesteal_is_doubled_and_two_quadruple() {
        let mut s = game(
            "sp-double-lifesteal",
            json!({ "field": [MAGE, DOUBLER, DOUBLER], "hand": [BOLT], "health": 10 }),
            json!({}),
        );
        s.play(BOLT, json!({ "targets": hero(P2) }));
        assert_eq!(heals_on(&s, "hero-p1"), vec![4]);
    }

    #[test]
    fn r1180003_the_opponents_heals_are_untouched() {
        let mut s = game(
            "sp-double-theirs",
            json!({ "field": [DOUBLER] }),
            json!({ "hand": [MEND], "health": 10 }),
        );
        s.end_turn();
        s.play(MEND, json!({ "targets": hero(P2) }));
        assert_eq!(health(&s, P2), 13);
    }

    #[test]
    fn r1180003_a_heal_turned_into_damage_is_not_doubled() {
        let mut s = game(
            "sp-double-moon",
            json!({ "field": [DOUBLER], "hand": [MEND], "health": 20 }),
            json!({ "backrow": [{ "def": MOON, "faceUp": true }] }),
        );
        s.play(MEND, json!({ "targets": hero(P1) }));
        // The heal of 3 became 3 Pierce damage from the converter, never 6.
        assert_eq!(health(&s, P1), 17);
    }

    #[test]
    fn r1180003_heal_up_to_n_stops_at_n() {
        let mut s = game(
            "sp-double-up-to",
            json!({ "field": [DOUBLER, { "def": BODY, "damage": 8 }], "health": 10 }),
            json!({}),
        );
        let body = unit_at(&s, P1, 2);
        apply_as(
            &mut s,
            None,
            P1,
            &[
                heal(json_as(json!({ "target": { "of": "selfHero" }, "upTo": 15 }))),
                heal(json_as(
                    json!({ "target": { "of": "instance", "instanceId": body.id }, "upTo": 4 }),
                )),
            ],
        );
        assert_eq!(health(&s, P1), 15);
        assert_eq!(s.card(&body.id).damage, 5);
    }
}

// ---- R1180004: health as the price ----

mod r1180004_health_as_the_price {
    use super::*;

    fn plays_of(s: &Scenario, player: PlayerId, def_id: &str) -> usize {
        legal_actions(s.state(), player)
            .into_iter()
            .filter(|action| match action {
                ActionBody::Play { instance_id, .. } => {
                    find_instance(s.state(), instance_id).is_some_and(|card| card.def_id == def_id)
                }
                _ => false,
            })
            .count()
    }

    #[test]
    fn r1180004_the_next_spell_costs_no_mana_and_three_times_its_cost_in_health() {
        let mut s = game("sp-bloom-price", json!({ "hand": [BLOOM, ZAP] }), json!({}));
        s.play(BLOOM, json!({}));
        assert_eq!(health_prices(&s, P1), 1);
        assert!(
            badges(&s, P1).contains(
                &"Your next Spell costs health equal to 3 times its cost instead of mana".to_string()
            )
        );
        let mana = s.state().players.p1.mana.current;
        s.play(ZAP, json!({}));
        assert_eq!(s.state().players.p1.mana.current, mana);
        assert_eq!(health(&s, P1), 24);
        assert_eq!(health_prices(&s, P1), 0);
        assert!(s.events().iter().any(|event| matches!(
            event,
            GameEvent::HealthLost {
                player: PlayerId::P1,
                amount: 6
            }
        )));
    }

    #[test]
    fn r1180004_a_spell_only_the_health_affords_is_offered_and_played() {
        let mut s = game(
            "sp-bloom-legal",
            json!({ "hand": [BLOOM, ZAP], "mana": 0 }),
            json!({}),
        );
        assert_eq!(plays_of(&s, P1, ZAP), 0);
        s.play(BLOOM, json!({}));
        assert_eq!(plays_of(&s, P1, ZAP), 1);
        s.play(ZAP, json!({}));
        assert_eq!(health(&s, P1), 24);
        assert_eq!(health(&s, P2), 28);
    }

    #[test]
    fn r1180004_a_discount_lowers_the_health_and_is_spent() {
        let mut s = game(
            "sp-bloom-discount",
            json!({ "hand": [BLOOM, DISCOUNT, ZAP] }),
            json!({}),
        );
        s.play(BLOOM, json!({}));
        s.play(DISCOUNT, json!({}));
        s.play(ZAP, json!({}));
        assert_eq!(health(&s, P1), 27);
        assert_eq!(
            mods_of(&s, P1, |kind| matches!(kind, ModifierKind::CostDiscount { .. })),
            0
        );
    }

    #[test]
    fn r1180004_a_field_spell_and_a_cast_leave_it_for_the_next_spell() {
        let mut s = game(
            "sp-bloom-not",
            json!({ "hand": [BLOOM, FIELD_ZAP, ZAP] }),
            json!({}),
        );
        s.play(BLOOM, json!({}));
        let mana = s.state().players.p1.mana.current;
        s.play(FIELD_ZAP, json!({}));
        assert_eq!(s.state().players.p1.mana.current, mana - 1);
        assert_eq!(health_prices(&s, P1), 1);
        apply_as(
            &mut s,
            None,
            P1,
            &[effects::cast_new(json_as(json!({ "def": ZAP })))],
        );
        assert_eq!(health(&s, P1), 30);
        assert_eq!(health_prices(&s, P1), 1);
        s.play(ZAP, json!({}));
        assert_eq!(health(&s, P1), 24);
    }

    #[test]
    fn r1180004_it_lasts_across_turns_until_a_spell_uses_it() {
        let mut s = game(
            "sp-bloom-turns",
            json!({ "hand": [BLOOM, ZAP, PAWN] }),
            json!({ "hand": [PAWN] }),
        );
        s.play(BLOOM, json!({}));
        s.end_turn();
        s.end_turn();
        assert_eq!(health_prices(&s, P1), 1);
        s.play(ZAP, json!({}));
        assert_eq!(health(&s, P1), 24);
    }

    #[test]
    fn r1180004_paying_to_0_loses_at_the_state_check() {
        let mut s = game(
            "sp-bloom-lose",
            json!({ "hand": [BLOOM, ZAP], "health": 6 }),
            json!({}),
        );
        s.play(BLOOM, json!({}));
        s.play(ZAP, json!({}));
        let result = s.state().result.expect("the game is over");
        assert_eq!(result.winner, Winner::P2);
    }

    #[test]
    fn r1180004_a_second_replaces_the_first_and_a_0_spell_uses_it_for_nothing() {
        let mut s = game("sp-bloom-twice", json!({ "hand": [NOTHING, ZAP] }), json!({}));
        let price = |echo: Option<i32>| {
            effects::price_next_spell_in_health(effects::HealthPriceArgs { multiple: 3, echo })
        };
        apply_as(&mut s, None, P1, &[price(None), price(Some(1))]);
        assert_eq!(health_prices(&s, P1), 1);
        assert!(
            s.state()
                .players
                .p1
                .mods
                .iter()
                .any(|modifier| matches!(modifier.kind, ModifierKind::HealthPrice { echo: Some(1), .. }))
        );
        s.play(NOTHING, json!({}));
        assert_eq!(health(&s, P1), 30);
        assert_eq!(health_prices(&s, P1), 0);
        let mana = s.state().players.p1.mana.current;
        s.play(ZAP, json!({}));
        assert_eq!(s.state().players.p1.mana.current, mana - 2);
    }

    #[test]
    fn r1180004_an_x_spell_costs_three_times_its_x() {
        let mut s = game("sp-bloom-x", json!({ "hand": [BLOOM, XBOLT] }), json!({}));
        s.play(BLOOM, json!({}));
        let mana = s.state().players.p1.mana.current;
        s.play(XBOLT, json!({ "x": 2 }));
        assert_eq!(s.state().players.p1.mana.current, mana);
        assert_eq!(health(&s, P1), 24);
        assert_eq!(health(&s, P2), 28);
    }

    #[test]
    fn r1180004_the_echo_it_gives_resolves_the_spell_again() {
        let mut s = game("sp-bloom-echo", json!({ "hand": [BLOOM_ECHO, ZAP] }), json!({}));
        s.play(BLOOM_ECHO, json!({}));
        assert!(
            badges(&s, P1).contains(
                &"Your next Spell costs health equal to 3 times its cost instead of mana, and has Echo 1"
                    .to_string()
            )
        );
        s.play(ZAP, json!({}));
        assert_eq!(hits_on(&s, "hero-p2"), vec![1, 1, 1, 1]);
        assert_eq!(health(&s, P1), 24);
    }
}

// ---- R1180005: after a Spell targeting an enemy resolves ----

mod r1180005_after_a_spell_targeting_an_enemy {
    use super::*;

    fn answers(s: &Scenario) -> Vec<(String, SpellAnswer)> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::CardResolved {
                    instance_id,
                    answer: Some(answer),
                    ..
                } => Some((instance_id.clone(), answer.clone())),
                _ => None,
            })
            .collect()
    }

    fn graveyard_count(s: &Scenario, player: PlayerId, def_id: &str) -> usize {
        s.pile(player, "graveyard")
            .into_iter()
            .filter(|card| card.def_id == def_id)
            .count()
    }

    #[test]
    fn r1180005_a_declared_enemy_unit_calls_the_hand_card_at_it() {
        let mut s = game(
            "sp-pile-unit",
            json!({ "hand": [BOLT, PILER] }),
            json!({ "field": [BODY] }),
        );
        let body = unit_at(&s, P2, 1);
        let piler = hand_card(&s, P1, PILER);
        s.play(BOLT, json!({ "targets": instance(&body) }));
        assert_eq!(hits_on(&s, &body.id), vec![1, 2]);
        assert_eq!(graveyard_count(&s, P1, PILER), 1);
        let named = answers(&s);
        assert_eq!(named[0].1.by, piler.id);
        assert_eq!(
            named[0].1.at,
            Selection::Instance {
                instance_id: body.id.clone()
            }
        );
        // Its cast is a play (R70): it is counted.
        assert!(was_played_this_turn(s.state(), P1, piler.id.as_str()));
    }

    #[test]
    fn r1180005_the_enemy_hero_and_a_prompt_chosen_target_call_it_too() {
        let mut s = game("sp-pile-prompt", json!({ "hand": [ASKER, PILER] }), json!({}));
        s.play(ASKER, json!({}));
        s.answer(hero(P2));
        assert_eq!(hits_on(&s, "hero-p2"), vec![1, 2]);
        assert_eq!(graveyard_count(&s, P1, PILER), 1);
    }

    #[test]
    fn r1180005_an_echo_repeats_fresh_pick_is_targeted() {
        let mut s = game(
            "sp-pile-echo",
            json!({ "hand": [ECHO_BOLT, PILER] }),
            json!({ "field": [BODY] }),
        );
        s.play(ECHO_BOLT, json!({ "targets": hero(P2) }));
        // The repeat asks again; the body is picked, and the answer is the first enemy targeted.
        let body = unit_at(&s, P2, 1);
        s.answer(instance(&body));
        assert_eq!(answers(&s)[0].1.at, Selection::Hero { player: P2 });
        assert_eq!(hits_on(&s, "hero-p2"), vec![1, 2]);
    }

    #[test]
    fn r1180005_the_hand_answers_first_then_the_top_of_the_deck() {
        let mut s = game(
            "sp-pile-deck",
            json!({ "hand": [BOLT, BOLT, PILER], "library": [PILER, PILER] }),
            json!({}),
        );
        let in_hand = hand_card(&s, P1, PILER);
        s.play(BOLT, json!({ "targets": hero(P2) }));
        let named = answers(&s);
        // The hand's answers the Bolt; its own cast calls the deck's top one, whose cast calls the last.
        assert_eq!(named[0].1.by, in_hand.id);
        assert_eq!(named.len(), 3);
        assert_eq!(hits_on(&s, "hero-p2"), vec![1, 2, 2, 2]);
        assert_eq!(graveyard_count(&s, P1, PILER), 3);
    }

    #[test]
    fn r1180005_one_card_answers_each_spell_and_the_second_answers_the_first() {
        let mut s = game(
            "sp-pile-chain",
            json!({ "hand": [BOLT, PILER, PILER] }),
            json!({}),
        );
        let first = s.hand(P1)[1].clone();
        let second = s.hand(P1)[2].clone();
        s.play(BOLT, json!({ "targets": hero(P2) }));
        let named = answers(&s);
        assert_eq!(named.len(), 2);
        assert_eq!(named[0].1.by, first.id);
        // The second answers the first's cast, not the Bolt.
        assert_eq!(named[1].0, first.id);
        assert_eq!(named[1].1.by, second.id);
        assert_eq!(hits_on(&s, "hero-p2"), vec![1, 2, 2]);
    }

    #[test]
    fn r1180005_untargeted_own_unit_and_the_opponents_spells_call_nothing() {
        let mut s = game(
            "sp-pile-none",
            json!({ "hand": [ZAP, MEND, PILER], "field": [BODY], "mana": 9 }),
            json!({ "hand": [BOLT] }),
        );
        s.play(ZAP, json!({}));
        let body = unit_at(&s, P1, 1);
        s.play(MEND, json!({ "targets": instance(&body) }));
        s.end_turn();
        s.play(BOLT, json!({ "targets": hero(P1) }));
        assert!(answers(&s).is_empty());
        assert_eq!(s.hand(P1).iter().filter(|card| card.def_id == PILER).count(), 1);
    }

    #[test]
    fn r1180005_a_target_gone_calls_nothing() {
        let mut s = game(
            "sp-pile-gone",
            json!({ "hand": [BOLT, PILER] }),
            json!({ "field": [PAWN] }),
        );
        let pawn = unit_at(&s, P2, 1);
        s.play(BOLT, json!({ "targets": instance(&pawn) }));
        assert!(answers(&s).is_empty());
        assert_eq!(s.hand(P1).iter().filter(|card| card.def_id == PILER).count(), 1);
    }

    #[test]
    fn r1180005_nothing_is_kept_or_named_without_an_answering_card() {
        let mut s = game("sp-pile-quiet", json!({ "hand": [BOLT] }), json!({}));
        s.play(BOLT, json!({ "targets": hero(P2) }));
        assert!(answers(&s).is_empty());
        let bolt = s
            .pile(P1, "graveyard")
            .into_iter()
            .find(|card| card.def_id == BOLT)
            .expect("bolt");
        assert!(!bolt.memory.contains_key(TARGETED_KEY));
    }

    #[test]
    fn r1180005_no_view_forwards_the_answer() {
        let mut s = game("sp-pile-view", json!({ "hand": [BOLT, PILER] }), json!({}));
        s.play(BOLT, json!({ "targets": hero(P2) }));
        for player in [P1, P2] {
            let events = serde_json::to_value(s.view(player).events).expect("events serialise");
            assert!(!events.to_string().contains("\"answer\""), "{player:?}'s view");
        }
    }

    #[test]
    fn r1180005_an_aim_only_cast_whose_aim_is_not_legal_casts_nothing() {
        let mut s = game("sp-pile-illegal", json!({ "hand": [PILER] }), json!({}));
        let piler = hand_card(&s, P1, PILER);
        // Its own hero is no legal pick of "an enemy Unit or the enemy hero".
        apply_as(
            &mut s,
            Some(&piler),
            P1,
            &[effects::cast(json_as(json!({
                "target": { "of": "self" }, "aimAt": "hero:p1", "aimOnly": true,
            })))],
        );
        assert_eq!(s.hand(P1).len(), 1);
        apply_as(
            &mut s,
            Some(&piler),
            P1,
            &[effects::cast(json_as(json!({
                "target": { "of": "self" }, "aimAt": "hero:p2", "aimOnly": true,
            })))],
        );
        assert!(s.hand(P1).is_empty());
        assert_eq!(health(&s, P2), 28);
    }
}
