// Training Grounds part 5 (issue #2198): the engine systems TG #15 In a hurry, #34 Dino-size and #37
// Seance are written with, each through a fixture script (`fixtures/tg_spells.rs`) before a card uses
// it: R2198000's set stats on one Unit (`set_stats`), R2198001's exact copy (`add_to_hand`'s `exact`
// rider) and R2198002's tokens destroyed at the end of the turn they arrive (`summon`'s `rememberAs`,
// then a `delay` under `DELAYED_DESTROY_HOOK` watching each).

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{big_body, plain};
use crate::rules::fixtures::tg_spells::{
    COPY_COST, SET_TO, TOKENS, copier, dasher, dino, hurry, register_tg5_fixtures, seance, seance_free, stoic,
};

/// `p1`'s main phase; each side's setup as given, with plain cards to draw so no turn ends by itself
/// (§2.5).
fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    register_tg5_fixtures();
    let mut p1 = p1;
    let mut p2 = p2;
    for side in [&mut p1, &mut p2] {
        if side.get("library").is_none() {
            side["library"] = json!([plain.id, plain.id, plain.id, plain.id]);
        }
        side["mana"] = json!(10);
    }
    scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }))
}

fn id_at(s: &Scenario, player: PlayerId, lane: i32) -> String {
    s.unit(player, lane)
        .unwrap_or_else(|| panic!("no unit at {player:?}'s lane {lane}"))
        .id
}

fn pick(id: &str) -> Value {
    json!({ "targets": [{ "pick": "instance", "instanceId": id }] })
}

/// The set fixture played into lane 4 with `id` chosen.
fn dino_on(id: &str) -> Value {
    json!({ "zone": 4, "targets": [{ "pick": "instance", "instanceId": id }] })
}

fn stats_of(s: &Scenario, id: &str) -> (i32, i32, i32) {
    let view = s.stats(id);
    (view.attack, view.max_health, view.health)
}

fn in_hand(s: &Scenario, player: PlayerId, def_id: &str) -> Vec<CardInstance> {
    s.hand(player)
        .into_iter()
        .filter(|card| card.def_id == def_id)
        .collect()
}

mod r2198000_set_stats_sets_one_unit_outright {
    use super::*;

    #[test]
    fn r2198000_the_chosen_unit_is_set_undamaged_its_buffs_gone_and_no_other_unit_moves() {
        let mut s = board(
            "r2198000-set",
            json!({ "hand": [dino.id, plain.id], "field": [{ "def": big_body.id, "lane": 1, "damage": 2 }] }),
            json!({ "hand": [plain.id], "field": [{ "def": big_body.id, "lane": 2, "damage": 1 }] }),
        );
        let mine = id_at(&s, P1, 1);
        let theirs = id_at(&s, P2, 2);
        s.card_mut(mine.as_str()).buffs = AttackHealth { attack: 3, health: 3 };
        let before = stats_of(&s, &theirs);

        s.play(dino.id.as_str(), dino_on(&mine));

        assert_eq!(stats_of(&s, &mine), (SET_TO, SET_TO, SET_TO));
        assert_eq!(s.card(mine.as_str()).buffs, AttackHealth { attack: 0, health: 0 });
        assert_eq!(s.card(mine.as_str()).damage, 0);
        assert_eq!(stats_of(&s, &theirs), before, "only the chosen Unit is set");
        let changed: Vec<(String, i32)> = s
            .last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::NumberChanged {
                    instance_id,
                    key,
                    value,
                    ..
                } if *instance_id == mine => Some((key.clone(), *value)),
                _ => None,
            })
            .collect();
        assert_eq!(
            changed,
            vec![("attack".to_string(), SET_TO), ("health".to_string(), SET_TO)]
        );
    }

    #[test]
    fn r2198000_an_enemy_unit_may_be_set() {
        let mut s = board(
            "r2198000-enemy",
            json!({ "hand": [dino.id, plain.id] }),
            json!({ "hand": [plain.id], "field": [{ "def": big_body.id, "lane": 3 }] }),
        );
        let theirs = id_at(&s, P2, 3);
        s.play(dino.id.as_str(), dino_on(&theirs));
        assert_eq!(stats_of(&s, &theirs), (SET_TO, SET_TO, SET_TO));
    }

    #[test]
    fn r2198000_an_immutable_unit_is_untouched() {
        let mut s = board(
            "r2198000-immutable",
            json!({ "hand": [dino.id, plain.id], "field": [{ "def": stoic.id, "lane": 1 }] }),
            json!({ "hand": [plain.id] }),
        );
        let stone = id_at(&s, P1, 1);
        let before = stats_of(&s, &stone);
        s.play(dino.id.as_str(), dino_on(&stone));
        assert_eq!(stats_of(&s, &stone), before);
    }
}

mod r2198001_an_exact_copy_keeps_the_field_copy_and_the_price {
    use super::*;

    /// p1 holds `spell`; p2's lane 1 holds a Radiant big body with 2 damage, +2/+1 in buffs, a granted
    /// Taunt and a (1) discount.
    fn copied(seed: &str, spell: &str) -> (Scenario, String) {
        let mut s = board(
            seed,
            json!({ "hand": [spell, plain.id] }),
            json!({ "hand": [plain.id], "field": [{ "def": big_body.id, "lane": 1, "radiant": true, "damage": 2 }] }),
        );
        let theirs = id_at(&s, P2, 1);
        {
            let card = s.card_mut(theirs.as_str());
            card.buffs = AttackHealth { attack: 2, health: 1 };
            card.granted_keywords.push(Keyword::Taunt);
            card.cost_mod = -1;
        }
        s.play(spell, pick(&theirs));
        (s, theirs)
    }

    #[test]
    fn r2198001_the_copy_carries_radiant_buffs_granted_keywords_and_the_price_undamaged() {
        let (mut s, theirs) = copied("r2198001-exact", seance.id.as_str());
        let copies = in_hand(&s, P1, &big_body.id);
        assert_eq!(copies.len(), 1);
        let copy = &copies[0];
        assert_ne!(copy.id, theirs, "a fresh card, the source staying");
        assert!(copy.radiant);
        assert_eq!(copy.buffs, AttackHealth { attack: 2, health: 1 });
        assert!(copy.granted_keywords.contains(&Keyword::Taunt));
        assert_eq!(copy.cost_mod, -1);
        assert_eq!(copy.damage, 0);
        assert_eq!(copy.owner, P1);
        s.expect_in_zone(theirs.as_str(), "field");
    }

    #[test]
    fn r2198001_a_stated_price_replaces_the_copied_one() {
        let (s, _) = copied("r2198001-priced", seance_free.id.as_str());
        let copy = &in_hand(&s, P1, &big_body.id)[0];
        assert_eq!(copy.cost_override, Some(COPY_COST));
        assert_eq!(copy.cost_mod, 0);
        assert_eq!(own_cost(s.state(), copy), Some(COPY_COST));
        assert_eq!(copy.buffs, AttackHealth { attack: 2, health: 1 });
    }

    #[test]
    fn r2198001_without_exact_a_copy_takes_r1242s_riders_alone() {
        let (s, _) = copied("r2198001-plain", copier.id.as_str());
        let copy = &in_hand(&s, P1, &big_body.id)[0];
        assert!(copy.radiant);
        assert_eq!(copy.buffs, AttackHealth { attack: 0, health: 0 });
        assert!(copy.granted_keywords.is_empty());
        assert_eq!(copy.cost_mod, 0);
    }
}

mod r2198002_tokens_destroyed_at_the_end_of_this_turn {
    use super::*;

    fn dashers(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        (1..=5)
            .filter_map(|lane| s.unit(player, lane))
            .filter(|card| card.def_id == dasher.id)
            .collect()
    }

    #[test]
    fn r2198002_the_tokens_stand_this_turn_and_are_gone_before_the_next() {
        let mut s = board(
            "r2198002-end",
            json!({ "hand": [hurry.id, plain.id], "field": [{ "def": big_body.id, "lane": 5 }] }),
            json!({ "hand": [plain.id] }),
        );
        let keeper = id_at(&s, P1, 5);
        s.play(hurry.id.as_str(), json!({}));
        assert_eq!(dashers(&s, P1).len(), TOKENS as usize);

        s.end_turn();

        assert!(dashers(&s, P1).is_empty(), "none stands on the opponent's turn");
        s.expect_in_zone(keeper.as_str(), "field");
    }

    #[test]
    fn r2198002_a_token_already_gone_is_not_counted_again_and_nothing_else_is_destroyed() {
        let mut s = board(
            "r2198002-gone",
            json!({ "hand": [hurry.id, plain.id], "field": [{ "def": big_body.id, "lane": 5 }] }),
            json!({ "hand": [plain.id], "field": [{ "def": big_body.id, "lane": 1 }] }),
        );
        let keeper = id_at(&s, P1, 5);
        let theirs = id_at(&s, P2, 1);
        s.play(hurry.id.as_str(), json!({}));
        let first = dashers(&s, P1)[0].id.clone();
        // The first token trades into the enemy body and dies (Rush: it may attack a Unit).
        s.card_mut(first.as_str()).damage = 2;
        s.attack(first.as_str(), theirs.as_str());
        assert_eq!(dashers(&s, P1).len(), TOKENS as usize - 1);

        s.end_turn();

        assert!(dashers(&s, P1).is_empty());
        s.expect_in_zone(keeper.as_str(), "field");
        s.expect_in_zone(theirs.as_str(), "field");
    }

    #[test]
    fn r2198002_cast_on_the_opponents_turn_they_go_at_that_turns_end() {
        let mut s = board(
            "r2198002-their-turn",
            json!({ "hand": [plain.id] }),
            json!({ "hand": [hurry.id, plain.id] }),
        );
        s.end_turn();
        s.play(hurry.id.as_str(), json!({}));
        assert_eq!(dashers(&s, P2).len(), TOKENS as usize);
        s.end_turn();
        assert!(dashers(&s, P2).is_empty());
    }
}
