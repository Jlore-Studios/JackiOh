//! The AI's model of a position caught up with the engine's state (#898): `search_signature` signs
//! every public axis a line can move, each only when it is not the default (R1224's encoding) and
//! free of the order the moves came in, so positions differing on them stop merging in the beam's
//! duplicate check and the lethal solver's visited set.
//!
//! The cards: Mr. Vanilla (core-008) 4/4, Tempo Timmy (core-011) 3/3 Rush First Strike, Pointmaster
//! (core-020) 7/1 First Strike and Midrange Menace (core-019) 9/9 Taunt.

use indexmap::IndexMap;
use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, register_cards};

fn board(setup: Value) -> GameState {
    register_cards();
    scenario(setup).state().clone()
}

mod signature {
    use super::*;

    /// The AI with Mr. Vanilla and Tempo Timmy in hand and Pointmaster on the field, the enemy with
    /// Midrange Menace: a position with none of the new parts.
    fn base() -> GameState {
        board(json!({
            "seed": "ai-signature-axes",
            "p1": { "hand": ["core-008", "core-011"], "field": ["core-020"], "library": ["core-008"] },
            "p2": { "field": ["core-019"], "library": ["core-008"] },
        }))
    }

    /// `change` alone moves the AI's signature of `base()`.
    #[track_caller]
    fn assert_signed(change: impl FnOnce(&mut GameState)) {
        let before = base();
        let mut after = before.clone();
        change(&mut after);
        assert_ne!(search_signature(&before, AI), search_signature(&after, AI));
    }

    fn modifier(id: &str, amount: i32) -> PlayerModifier {
        PlayerModifier {
            id: id.to_string(),
            expiry: ModifierExpiry::Never,
            kind: ModifierKind::ComboDraw { amount },
        }
    }

    /// `player`'s first unit on the field.
    fn field_unit(state: &mut GameState, player: PlayerId) -> &mut CardInstance {
        let id = active_units_of(state, player)[0].id.clone();
        find_instance_mut(state, &id).expect("the unit stands on the field")
    }

    #[test]
    fn the_mana_maximum_is_signed() {
        assert_signed(|state| state.players[AI].mana.max += 1);
    }

    #[test]
    fn next_turns_mana_is_signed() {
        assert_signed(|state| state.players[HUMAN].mana.next_turn_mod = -1);
    }

    #[test]
    fn permanent_mana_is_signed() {
        assert_signed(|state| state.players[AI].mana.perm_mod = 1);
    }

    #[test]
    fn a_locked_zone_is_signed() {
        assert_signed(|state| state.players[HUMAN].locks.units[0] = true);
    }

    #[test]
    fn player_modifiers_are_signed() {
        assert_signed(|state| state.players[AI].mods.push(modifier("m1", 1)));
    }

    #[test]
    fn each_turn_log_count_is_signed() {
        assert_signed(|state| state.players[AI].turn_log.cards_played = 1);
        assert_signed(|state| {
            let id = state.players[AI].hand[0].id.clone();
            state.players[AI].turn_log.played_ids = vec![id];
        });
        assert_signed(|state| state.players[AI].turn_log.costs_paid = Some(vec![1]));
        assert_signed(|state| {
            state.players[AI].turn_log.played_by_type = Some(IndexMap::from([(CardType::Spell, 1)]));
        });
        assert_signed(|state| state.players[AI].turn_log.mana_borrowed = Some(1));
        assert_signed(|state| state.players[AI].turn_log.borrowed_parts = Some(1));
        assert_signed(|state| state.players[AI].turn_log.mana_locked = Some(1));
    }

    #[test]
    fn the_unspent_mana_the_enemys_last_turn_closed_on_is_left_out() {
        // It stays in their log all through the AI's turn, the same in every position a search compares.
        let mut closed = base();
        closed.players[HUMAN].turn_log.unspent_at_end = Some(2);
        assert_eq!(search_signature(&base(), AI), search_signature(&closed, AI));
    }

    #[test]
    fn fatigue_is_signed() {
        assert_signed(|state| state.players[AI].fatigue_count = 1);
    }

    #[test]
    fn the_jade_count_is_signed() {
        assert_signed(|state| state.players[AI].jade = Some(1));
    }

    #[test]
    fn extra_turns_are_signed() {
        assert_signed(|state| state.players[AI].extra_turns = Some(1));
    }

    #[test]
    fn a_hand_cap_is_signed() {
        assert_signed(|state| state.players[AI].hand_cap = Some(5));
    }

    #[test]
    fn the_graveyard_is_signed() {
        assert_signed(|state| {
            let card = state.players[AI].hand[0].clone();
            state.players[AI].graveyard.push(card);
        });
    }

    #[test]
    fn the_exile_is_signed() {
        assert_signed(|state| {
            let card = state.players[AI].hand[0].clone();
            state.players[AI].exile.push(card);
        });
    }

    #[test]
    fn a_backrow_pile_is_signed() {
        assert_signed(|state| {
            let card = state.players[AI].hand[0].clone();
            state.players[AI].backrow_piles = Some(vec![vec![card], vec![], vec![], vec![], vec![]]);
        });
    }

    #[test]
    fn a_delayed_effect_is_signed() {
        assert_signed(|state| {
            state.delayed.push(DelayedEffect {
                id: "d1".to_string(),
                seq: 1,
                owner: AI,
                at: DelayedAt {
                    phase: Phase::End,
                    player: AI,
                },
                not_before: None,
                resume: Resume {
                    def_id: "core-008".to_string(),
                    hook: "cry".to_string(),
                    step: "later".to_string(),
                    radiant: false,
                    instance_id: None,
                    data: IndexMap::new(),
                },
                watch: None,
                hand_watch: None,
            });
        });
    }

    #[test]
    fn a_mark_is_signed() {
        assert_signed(|state| {
            let instance_id = active_units_of(state, HUMAN)[0].id.clone();
            state.marks = Some(vec![MarkRecord {
                instance_id,
                mark: "target".to_string(),
                color: "purple".to_string(),
                delayed_id: "d1".to_string(),
            }]);
        });
    }

    #[test]
    fn a_cards_memory_is_signed_in_hand_and_on_the_field() {
        assert_signed(|state| {
            state.players[AI].hand[0]
                .memory
                .insert("count".to_string(), json!(1));
        });
        assert_signed(|state| {
            field_unit(state, AI).memory.insert("count".to_string(), json!(1));
        });
    }

    #[test]
    fn a_cards_tuning_is_signed_in_hand_and_on_the_field() {
        let tuning = Tuning {
            attack: Some(1),
            ..Tuning::default()
        };
        assert_signed(|state| state.players[AI].hand[0].tuning = Some(tuning.clone()));
        assert_signed(|state| field_unit(state, AI).tuning = Some(tuning.clone()));
    }

    #[test]
    fn a_hand_cards_enchantments_are_signed() {
        assert_signed(|state| state.players[AI].hand[0].enchantments = Some(vec![Enchantment::CastOnDraw]));
    }

    #[test]
    fn a_plain_position_signs_none_of_the_new_parts() {
        let signature = search_signature(&base(), AI);
        let new_parts = [
            "Mp", "Lp", "Xp", "Tp", "Fp", "Jp", "Ep", "Cp", "Gp", "Yp", "Pp", "D", "K",
        ];
        for part in signature.split('|') {
            assert!(
                !new_parts.iter().any(|prefix| part.starts_with(prefix)),
                "{part} in {signature}"
            );
            assert!(
                [";m", ";t", ";e"].iter().all(|rider| !part.contains(rider)),
                "{part} in {signature}"
            );
        }
    }

    #[test]
    fn two_orders_of_the_same_moves_still_sign_alike() {
        let mut first = base();
        let a = first.players[AI].hand[0].clone();
        let b = first.players[AI].hand[1].clone();
        let mut second = first.clone();

        let side = &mut first.players[AI];
        side.graveyard = vec![a.clone(), b.clone()];
        side.turn_log.played_ids = vec![a.id.clone(), b.id.clone()];
        side.turn_log.costs_paid = Some(vec![0, 1]);
        side.mods = vec![modifier("m8", 1), modifier("m9", 2)];

        let side = &mut second.players[AI];
        side.graveyard = vec![b.clone(), a.clone()];
        side.turn_log.played_ids = vec![b.id, a.id];
        side.turn_log.costs_paid = Some(vec![1, 0]);
        side.mods = vec![modifier("m8", 2), modifier("m9", 1)];

        assert_eq!(search_signature(&first, AI), search_signature(&second, AI));
    }
}

mod soak {
    use super::*;
    use crate::ai::support::act;

    /// The enemy hero's health lost when the AI's units attack in `face_threat`'s order, smallest
    /// first, each the enemy's first Taunt while one stands and the hero after. R82 would end the turn
    /// after the last attack and fatigue would run on, so the AI turns it off first (R345).
    fn played(state: &GameState) -> i32 {
        let mut attackers: Vec<(String, i32)> = active_units_of(state, AI)
            .into_iter()
            .map(|unit| (unit.id.clone(), unit_view(state, unit).attack))
            .collect();
        attackers.sort_by_key(|(_, attack)| *attack);
        let mut after = state.clone();
        after.players[AI].auto_end_turn = Some(false);
        for (attacker_id, _) in attackers {
            let target_id = active_units_of(&after, HUMAN)
                .into_iter()
                .find(|unit| has_keyword(&unit_view(&after, unit).keywords, KeywordKind::Taunt))
                .map_or_else(|| format!("hero-{HUMAN}"), |unit| unit.id.clone());
            after = act(
                &after,
                AI,
                ActionBody::Attack {
                    attacker_id,
                    target_id,
                },
            );
        }
        state.players[HUMAN].hero.health - after.players[HUMAN].hero.health
    }

    /// `face_threat` counts what playing the attacks out does.
    #[track_caller]
    fn assert_soaks(setup: Value, expected: i32) {
        let state = board(setup);
        assert_eq!(face_threat(&state, AI), expected, "face_threat");
        assert_eq!(played(&state), expected, "the attacks played out");
    }

    #[test]
    fn armor_comes_off_each_hit_so_one_attack_units_never_scratch_an_armored_taunt() {
        // A Mr. Vanilla in Defense Position at 1 health with Armor 1: each 1-attack hit loses its point
        // to the Armor, and the 4/4 destroys it with nothing left over. Pooled as health + armor (2),
        // the two 1s paid it off and the 4 went face.
        assert_soaks(
            json!({
                "seed": "ai-soak-armor",
                "p1": { "field": [
                    { "def": "core-008", "statsOverride": { "attack": 1, "health": 4 } },
                    { "def": "core-008", "statsOverride": { "attack": 1, "health": 4 } },
                    "core-008",
                ] },
                "p2": { "field": [{ "def": "core-008", "position": "DEF", "damage": 3 }] },
            }),
            0,
        );
    }

    #[test]
    fn a_poisonous_attacker_destroys_the_taunt_with_one_point_and_the_rest_go_face() {
        // Wrong-House Attacker's 1 Poisonous point destroys Midrange Menace (9/9 Taunt), so Mr.
        // Vanilla's 4 goes face; pooled, the 1 and the 4 never paid off 9.
        assert_soaks(
            json!({
                "seed": "ai-soak-poisonous",
                "p1": { "field": ["classicplus-006", "core-008"] },
                "p2": { "field": ["core-019"] },
            }),
            4,
        );
    }

    #[test]
    fn a_trample_attacker_sends_what_lands_beyond_the_taunts_health_to_the_hero() {
        // Plague Bringer Goliath's 7 Trample into Midrange Menace at 2 health: 5 tramples through to
        // the hero, where the pooled soak counted the whole 7 spent.
        assert_soaks(
            json!({
                "seed": "ai-soak-trample",
                "p1": { "field": ["classic-061"] },
                "p2": { "field": [{ "def": "core-019", "damage": 7 }] },
            }),
            5,
        );
    }

    #[test]
    fn a_first_strike_taunt_destroys_the_attackers_it_outstrikes_before_they_land_a_blow() {
        // Pointmaster (7/1 First Strike) in Defense Position strikes each Mr. Vanilla for 7 before its 4
        // lands, so neither touches it and nothing goes face. Pooled (1 health + 1 Armor), the first 4
        // paid it off and the second went face.
        assert_soaks(
            json!({
                "seed": "ai-soak-first-strike",
                "p1": { "field": ["core-008", "core-008"] },
                "p2": { "field": [{ "def": "core-020", "position": "DEF" }] },
            }),
            0,
        );
    }

    #[test]
    fn a_reborn_taunt_comes_back_with_its_printed_shield_and_binds_again() {
        // Right-house defender (1/1 Taunt, Divine Shield, Reborn): one Mr. Vanilla takes the shield, one
        // destroys it, its body comes back with the shield, and two more go into it before the fifth
        // goes face for 4. Pooled, the shield and the 1 health took two and three went face for 12.
        assert_soaks(
            json!({
                "seed": "ai-soak-reborn",
                "p1": { "field": ["core-008", "core-008", "core-008", "core-008", "core-008"] },
                "p2": { "field": ["core-003"] },
            }),
            4,
        );
    }
}
