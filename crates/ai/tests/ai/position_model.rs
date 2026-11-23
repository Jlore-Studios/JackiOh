//! The AI's model of a position caught up with the engine's state (#898): `search_signature` signs
//! every public axis a line can move, each only when it is not the default (R1224's encoding) and
//! free of the order the moves came in, so positions differing on them stop merging in the beam's
//! duplicate check and the lethal solver's visited set.
//!
//! The cards: Mr. Vanilla 4/4, Tempo Timmy 3/3 Rush First Strike, Pointmaster 7/1 First Strike and Midrange
//! Menace 9/9 Taunt.

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

    /// A position with none of the new parts.
    fn base() -> GameState {
        board(json!({
            "seed": "ai-signature-axes",
            "p1": { "hand": ["core-008", "core-011"], "field": ["core-020"], "library": ["core-008"] },
            "p2": { "field": ["core-019"], "library": ["core-008"] },
        }))
    }

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
    fn a_lost_refresh_is_signed() {
        assert_signed(|state| {
            let through = state.players[HUMAN].turns_started + 1;
            state.players[HUMAN].lost_refresh_through = Some(through);
        });
        let mut one = base();
        let through = one.players[AI].turns_started + 1;
        one.players[AI].lost_refresh_through = Some(through);
        let mut two = one.clone();
        two.players[AI].lost_refresh_through = Some(through + 1);
        assert_ne!(search_signature(&one, AI), search_signature(&two, AI));
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
                due_own_turn: None,
                label: None,
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

    fn secret(
        id: &str,
        owner: PlayerId,
        choice: Option<SecretChoice>,
        revealed: Option<bool>,
    ) -> SecretRecord {
        SecretRecord {
            id: id.to_string(),
            owner,
            choice,
            revealed,
        }
    }

    /// `base()` with the seat's one secret, minted as `id`, keeping `choice`.
    fn keeping(id: &str, choice: SecretChoice) -> GameState {
        let mut state = base();
        state.secrets = Some(vec![secret(id, AI, Some(choice), None)]);
        state
    }

    #[test]
    fn r3772000_the_seats_own_secret_choice_is_signed() {
        let signed = |choice| search_signature(&keeping("secret-1", choice), AI);
        let (greed, attack, defend) = (
            signed(SecretChoice::Greed),
            signed(SecretChoice::Attack),
            signed(SecretChoice::Defend),
        );
        assert_ne!(greed, attack);
        assert_ne!(greed, defend);
        assert_ne!(attack, defend);
        // A secret held at all is signed.
        assert_ne!(search_signature(&base(), AI), greed);
    }

    #[test]
    fn r3772000_a_secrets_minted_id_is_not_signed() {
        assert_eq!(
            search_signature(&keeping("secret-1", SecretChoice::Attack), AI),
            search_signature(&keeping("secret-9", SecretChoice::Attack), AI)
        );
    }

    #[test]
    fn r3772000_a_secrets_owner_and_revealed_flag_are_signed() {
        let held = keeping("secret-1", SecretChoice::Attack);
        let with = |record: SecretRecord| {
            let mut state = held.clone();
            state.secrets = Some(vec![record]);
            search_signature(&state, AI)
        };
        let signatures = [
            search_signature(&held, AI),
            with(secret("secret-1", HUMAN, Some(SecretChoice::Attack), None)),
            with(secret("secret-1", AI, Some(SecretChoice::Attack), Some(true))),
            with(secret("secret-1", AI, None, None)),
        ];
        for (at, one) in signatures.iter().enumerate() {
            for other in &signatures[at + 1..] {
                assert_ne!(one, other);
            }
        }
    }

    #[test]
    fn r3772000_two_secrets_sign_alike_in_either_order() {
        let mut first = base();
        first.secrets = Some(vec![
            secret("secret-1", AI, Some(SecretChoice::Greed), None),
            secret("secret-2", HUMAN, Some(SecretChoice::Defend), None),
        ]);
        let mut second = first.clone();
        second.secrets = Some(vec![
            secret("secret-7", HUMAN, Some(SecretChoice::Defend), None),
            secret("secret-8", AI, Some(SecretChoice::Greed), None),
        ]);
        assert_eq!(search_signature(&first, AI), search_signature(&second, AI));
    }

    #[test]
    fn r3772000_a_position_with_no_secret_signs_as_before() {
        let mut emptied = base();
        emptied.secrets = Some(Vec::new());
        assert_eq!(search_signature(&base(), AI), search_signature(&emptied, AI));
    }

    #[test]
    fn a_hand_cards_enchantments_are_signed() {
        assert_signed(|state| state.players[AI].hand[0].enchantments = Some(vec![Enchantment::CastOnDraw]));
    }

    #[test]
    fn a_plain_position_signs_none_of_the_new_parts() {
        let signature = search_signature(&base(), AI);
        let new_parts = [
            "Mp", "Lp", "Xp", "Tp", "Fp", "Jp", "Ep", "Cp", "Gp", "Yp", "Pp", "D", "K", "S",
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

    /// The most hero health the AI's units can take over every order they may attack in, each the
    /// enemy's first Taunt while one stands and the hero after. R82 would end the turn after the last
    /// attack and fatigue would run on, so the AI turns it off first (R345).
    fn played(state: &GameState) -> i32 {
        let mut start = state.clone();
        start.players[AI].auto_end_turn = Some(false);
        let waiting: Vec<String> = active_units_of(&start, AI)
            .iter()
            .map(|unit| unit.id.clone())
            .collect();
        best_after(&start, &waiting, state.players[HUMAN].hero.health)
    }

    /// The most hero health `waiting`, attacking from `after` in any order, takes.
    fn best_after(after: &GameState, waiting: &[String], before: i32) -> i32 {
        let mut best = before - after.players[HUMAN].hero.health;
        for attacker_id in waiting {
            if !active_units_of(after, AI)
                .iter()
                .any(|unit| unit.id == *attacker_id)
            {
                continue;
            }
            let target_id = active_units_of(after, HUMAN)
                .into_iter()
                .find(|unit| has_keyword(&unit_view(after, unit).keywords, KeywordKind::Taunt))
                .map_or_else(|| format!("hero-{HUMAN}"), |unit| unit.id.clone());
            let next = act(
                after,
                AI,
                ActionBody::Attack {
                    attacker_id: attacker_id.clone(),
                    target_id,
                },
            );
            let rest: Vec<String> = waiting.iter().filter(|id| *id != attacker_id).cloned().collect();
            best = best.max(best_after(&next, &rest, before));
        }
        best
    }

    #[track_caller]
    fn assert_soaks(setup: Value, expected: i32) {
        let state = board(setup);
        assert_eq!(face_threat(&state, AI), expected, "face_threat");
        assert_eq!(played(&state), expected, "the attacks played out");
    }

    /// A Mr. Vanilla in Defense Position at 1 health with Armor 1, against two 1/4s and a 4/4.
    fn armored_guard() -> Value {
        json!({
            "seed": "ai-soak-armor",
            "p1": { "field": [
                { "def": "core-008", "statsOverride": { "attack": 1, "health": 4 } },
                { "def": "core-008", "statsOverride": { "attack": 1, "health": 4 } },
                "core-008",
            ] },
            "p2": { "field": [{ "def": "core-008", "position": "DEF", "damage": 3 }] },
        })
    }

    /// The issue's board (#3937): a Mr. Vanilla in Defense Position at 3 health, against two 3/4s and a
    /// 4/4.
    fn guard_vs_3_3_4() -> Value {
        json!({
            "seed": "ai-soak-lone-breaker",
            "p1": { "field": [
                { "def": "core-008", "statsOverride": { "attack": 3, "health": 4 } },
                { "def": "core-008", "statsOverride": { "attack": 3, "health": 4 } },
                "core-008",
            ] },
            "p2": { "field": [{ "def": "core-008", "position": "DEF", "damage": 1 }] },
        })
    }

    #[test]
    fn r3937000_armor_stops_the_one_attack_units_so_the_four_breaks_the_taunt_first_and_they_go_face() {
        // Each 1-attack hit would lose its point to the Armor, so neither goes in while the Taunt stands:
        // the 4/4 destroys it and both 1s go face. The old order spent them on the Armor and counted 0.
        assert_soaks(armored_guard(), 2);
        assert_eq!(face_threat_with(&board(armored_guard()), AI, &GREEDY_EVAL), 0);
    }

    #[test]
    fn a_poisonous_attacker_destroys_the_taunt_with_one_point_and_the_rest_go_face() {
        // Wrong-House Attacker's 1 Poisonous point destroys Midrange Menace (9/9 Taunt), so Mr.
        // Vanilla's 4 goes face.
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
        // Plague Bringer Goliath's 7 Trample into Midrange Menace at 2 health: 5 tramples through.
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
        // lands, so neither touches it and nothing goes face.
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
        // goes face for 4.
        assert_soaks(
            json!({
                "seed": "ai-soak-reborn",
                "p1": { "field": ["core-008", "core-008", "core-008", "core-008", "core-008"] },
                "p2": { "field": ["core-003"] },
            }),
            4,
        );
    }

    #[test]
    fn r3937000_a_lone_breaker_goes_at_the_guard_first_and_the_rest_go_face() {
        // The 4 destroys the 3-health guard alone, so both 3s go face for 6. Smallest first sends the two
        // 3s into it and counts 4.
        assert_soaks(guard_vs_3_3_4(), 6);
        assert_eq!(face_threat_with(&board(guard_vs_3_3_4()), AI, &GREEDY_EVAL), 4);
    }

    #[test]
    fn r3937000_a_taunt_no_attacker_breaks_alone_is_still_soaked_smallest_first() {
        // Midrange Menace (9/9 Taunt) outlasts every one of the 3s and 4s alone: the two 3s and a 4 destroy
        // it and the last 4 goes face.
        assert_soaks(
            json!({
                "seed": "ai-soak-no-breaker",
                "p1": { "field": [
                    { "def": "core-008", "statsOverride": { "attack": 3, "health": 4 } },
                    { "def": "core-008", "statsOverride": { "attack": 3, "health": 4 } },
                    "core-008",
                    "core-008",
                ] },
                "p2": { "field": ["core-019"] },
            }),
            4,
        );
    }

    #[test]
    fn r3937000_the_better_of_two_orders_counts_when_the_lone_breaker_is_worth_more_at_the_face() {
        // A full-health Mr. Vanilla in Defense Position (4 health, Armor 1) against four 2s and a 9: the 9
        // breaks it alone and the 2s go face for 8, but the 2s break it (1 each past the Armor) and the 9
        // goes face for 9, so smallest first counts.
        let two = json!({ "def": "core-008", "statsOverride": { "attack": 2, "health": 4 } });
        assert_soaks(
            json!({
                "seed": "ai-soak-two-orders",
                "p1": { "field": [
                    two.clone(),
                    two.clone(),
                    two.clone(),
                    two,
                    { "def": "core-008", "statsOverride": { "attack": 9, "health": 4 } },
                ] },
                "p2": { "field": [{ "def": "core-008", "position": "DEF" }] },
            }),
            9,
        );
    }

    #[test]
    fn r3937000_attackers_a_first_strike_taunt_would_destroy_wait_for_the_one_that_breaks_it() {
        // Pointmaster (6/1 First Strike) in Defense Position would strike each Mr. Vanilla down first, so
        // they wait; the 7/7's Armor 7 takes its strike and its 7 destroys it, and both Vanillas go face
        // for 8. The old order spent the Vanillas on it and counted 0.
        let setup = json!({
            "seed": "ai-soak-first-strike-wait",
            "p1": { "field": ["core-008", "core-008", "core-025"] },
            "p2": { "field": [{ "def": "core-020", "position": "DEF" }] },
        });
        assert_eq!(face_threat_with(&board(setup.clone()), AI, &GREEDY_EVAL), 0);
        assert_soaks(setup, 8);
    }

    #[test]
    fn r3937000_attackers_that_may_not_hit_the_hero_soak_the_guard_before_one_that_may() {
        // The 3s may not hit the hero. The 4 alone breaks the 3-health guard (Armor 1), but sent first it
        // leaves the 3s nothing to hit, so the 3s soak the guard and the 4 goes face, as before.
        let state = board(json!({
            "seed": "ai-soak-hero-ineligible",
            "p1": { "field": [{ "def": "core-008", "position": "DEF", "damage": 1 }] },
            "p2": { "field": [
                { "def": "core-008", "statsOverride": { "attack": 3, "health": 4 } },
                { "def": "core-008", "statsOverride": { "attack": 3, "health": 4 } },
                "core-008",
            ] },
        }));
        let units = active_units_of(&state, HUMAN);
        let attacks = [(units[0], 3, false), (units[1], 3, false), (units[2], 4, true)];
        assert_eq!(damage_past_taunts_with(&state, AI, &attacks, &AI_EVAL), 4);
        assert_eq!(damage_past_taunts_with(&state, AI, &attacks, &GREEDY_EVAL), 4);
    }

    #[test]
    fn r2179005_anime_armor_on_guard_takes_one_from_each_hit_so_three_four_fours_spend_themselves_on_it() {
        // Anime Armor (3/3) in Defense Position: each Mr. Vanilla's 4 loses 1 to the Armor and its own
        // guard caps the 3 left at 1, so all three hits go into it and nothing goes face. Uncapped, the walk
        // had the first Vanilla's 3 destroy it and counted the other two at the face, 1 each through the
        // hero's same cap.
        assert_soaks(
            json!({
                "seed": "ai-soak-anime-armor",
                "p1": { "field": ["core-008", "core-008", "core-008"] },
                "p2": { "field": [{ "def": "classicplus-011", "position": "DEF" }] },
            }),
            0,
        );
    }

    #[test]
    fn r2179005_anime_armor_outlasts_a_first_strike_taunt_and_frees_mr_vanilla_for_the_face() {
        // Pointmaster (First Strike) in Defense Position strikes Anime Armor first, but its own guard caps
        // the hit at 1, so Anime Armor lives to destroy it and Mr. Vanilla goes face for 4. Uncapped, the
        // walk had Pointmaster strike Anime Armor down and Mr. Vanilla wait on it for good.
        assert_soaks(
            json!({
                "seed": "ai-soak-anime-armor-first-strike",
                "p1": { "field": ["classicplus-011", "core-008"] },
                "p2": { "field": [{ "def": "core-020", "position": "DEF" }] },
            }),
            4,
        );
    }

    #[test]
    fn r3937000_greedy_eval_keeps_the_old_order() {
        const { assert!(AI_EVAL.best_attack_order_past_taunts) };
        const { assert!(!GREEDY_EVAL.best_attack_order_past_taunts) };
    }
}
