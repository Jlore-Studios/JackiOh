//! Meditative #8 Reach the Summit (SPEC §8.8 row 8). (0) Spell, Quickdraw, Wincon, Mythic.
//!   Base:    "Ascent level: starts at 0. This costs (Ascent level). … Ascent 0: Heal your hero {heal}.
//!             … Ascent 10: Win the game."
//!   Radiant: the same with "This costs (Ascent level - 1), never less than (0)." and 3 of each.
//!
//! The Ascent level is R429's play count on the instance: `counts_plays` counts each play at §10.5
//! step 4 (casts included, countered plays never), and the count rides the card through every zone.
//! While it resolves the play is already counted, so the level is the count less one; anywhere else
//! the level is the count. A copy starts at 0 (R57 resets the count), an Echo repeat is no play and
//! resolves the same level again, and making the card Radiant keeps the level (R840). Each play
//! gains only its own level's line (R841). The cost is a computed `cost` hook (R55): the level, or
//! max(0, level − 1) on the Radiant face, with R65's modifiers on top (R842). Ascent 1's target is a
//! resolution prompt over any Unit or hero (§10.6, R81), picked at random under a random cast (R843).
//! The return is Core #31's `end_of_turn` hook without the cost raise: from the graveyard, only on
//! the turn it was played (R153, R155), a `bounce` that a full hand burns back (§2.4). A `preview`
//! (R280) labelled "Ascent level" shows the count in play.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-008";

/// Ascent 1's resolution prompt, answered by the damage below.
const ASCENT_DAMAGE_STEP: &str = "ascent-damage";

/// The enemy permanents: tops of unit piles and backrow cards, face-down ones included (§3.2, R13).
fn enemy_permanents() -> BoardScope {
    json_as(json!({ "side": "enemy", "rows": ["units", "backrow"] }))
}

/// R840: the Ascent level while this play resolves — the counted plays less the current one.
/// A counted play precedes every cry, and an Echo repeat is not counted, so repeats resolve the
/// same level.
fn resolving_level(self_: &CardInstance) -> i32 {
    (times_played_of(self_) - 1).max(0)
}

/// R841, R60, R129: Ascent 4's picks — N distinct random enemy permanents. A pool holding N or
/// fewer is used as is with no shuffle; otherwise N are drawn from a shuffle.
fn exile_picks(ctx: &mut EffectContext<'_>, count: i32) -> Vec<String> {
    let pool = cards_in_scope(&*ctx, &enemy_permanents());
    if pool.len() as i32 <= count {
        pool.into_iter().map(|card| card.id).collect()
    } else {
        ctx.rng
            .shuffle(&pool)
            .into_iter()
            .take(count.max(0) as usize)
            .map(|card| card.id)
            .collect()
    }
}

fn ascent_cry(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(self_) = ctx.live_self().cloned() else {
        return vec![];
    };
    match resolving_level(&self_) {
        0 => vec![heal(json_as(
            json!({ "target": { "of": "selfHero" }, "amount": param(&*ctx, "heal") }),
        ))],
        1 => vec![choose_target(json_as(json!({
            "step": ASCENT_DAMAGE_STEP,
            "scope": { "side": "any", "of": ["unit", "hero"] },
            "prompt": "Deal damage",
        })))],
        2 => vec![draw(json_as(json!({ "count": param(&*ctx, "draw") })))],
        3 => vec![discard_random(json_as(
            json!({ "count": param(&*ctx, "discards"), "player": "enemy" }),
        ))],
        4 => {
            let count = param(&*ctx, "exiles");
            let ids = exile_picks(ctx, count);
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(move |_: &mut EffectContext<'_>| ids.clone()),
                each: Arc::new(|instance_id: &str| {
                    exile(json_as(
                        json!({ "target": { "of": "instance", "instanceId": instance_id } }),
                    ))
                }),
            })]
        }
        5 => vec![add_random_from_catalog(json_as(json!({
            "query": {},
            "count": param(&*ctx, "adds"),
            "radiant": true,
            "costOverride": 0,
        })))],
        6..=9 => vec![],
        _ => vec![win_game(json_as(json!({})))],
    }
}

/// Ascent 1's answer: the chosen Unit or hero takes the damage.
fn ascent_damage(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    vec![damage(json_as(
        json!({ "to": { "of": "chosen" }, "amount": param(&*ctx, "damage") }),
    ))]
}

/// Core #31's return without the cost raise: from the graveyard, only on the turn it was played
/// (R153, R155). A full hand burns the bounce back (§2.4).
fn return_to_hand(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(self_) = ctx.live_self().cloned() else {
        return vec![];
    };
    if self_.zone.z() != ZoneName::Graveyard {
        return vec![];
    }
    if !was_played_this_turn(ctx.state, self_.owner, &self_) {
        return vec![];
    }
    vec![bounce(json_as(json!({ "target": { "of": "self" } })))]
}

/// R842: the computed cost — the level (base) or max(0, level − 1) (Radiant).
fn cost_of(radiant: bool) -> CostHook {
    cost_hook(move |CostArgs { instance, .. }| {
        let level = times_played_of(instance);
        if radiant { (level - 1).max(0) } else { level }
    })
}

/// R280: the Ascent level as the card stands, labelled with the catalog text's own words.
fn preview() -> PreviewHook {
    condition_hook(move |ctx: ConditionContext<'_>| -> Vec<PreviewValue> {
        vec![PreviewValue {
            label: "Ascent level".to_string(),
            value: times_played_of(ctx.self_),
            display: None,
            ids: None,
        }]
    })
}

fn face(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            counts_plays: Some(true),
            ..StaticFlags::default()
        }),
        cost: Some(cost_of(radiant)),
        cry: Some(hook(ascent_cry)),
        resume: IndexMap::from([(ASCENT_DAMAGE_STEP, hook(ascent_damage))]),
        end_of_turn: Some(hook(return_to_hand)),
        preview: Some(preview()),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// Meditative #8 Reach the Summit — SPEC §8.8 row 8, BUILD M10 row M 8: Quickdraw; played at level 0
// it heals 2 and climbs; each play resolves only its own level's line and raises the level by one,
// counted per instance (casts included, countered plays not); it costs (level); at the end of the
// turn it returns to hand from the graveyard only on the turn it was played, a full hand burning it;
// level 1 prompts over any Unit or hero (random under a random cast); 2 draws 2; 3 discards 2 random
// enemy cards; 4 exiles 2 different random enemy permanents, face-down ones included; 5 adds 2 random
// Radiant non-token cards costing (0), never itself; 6 to 9 do nothing; 10 wins unless the caster's
// hero is at 0; a copy starts at 0; an Echo repeat resolves the same level; the preview shows the
// level; radiant costs max(0, level − 1) and reads 3 of each.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SUMMIT: &str = "meditative-008";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const FILLER: &str = "core-005"; // (1) Spell Stockpile.
    const TWINSPELL: &str = "core-079"; // Field Spell: the next Spell gains Echo +1.
    const TAX: &str = "classic-009"; // (2) Trap.

    #[derive(Clone, Copy, Default)]
    struct BoardOptions {
        times_played: Option<i32>,
        radiant: bool,
        health: Option<i32>,
        twinspell: bool,
    }

    /// The plays the card has had before the fixture's own (R429): what the harness cannot seed.
    fn set_times_played(s: &mut Scenario, card: &str, times: i32) {
        let id = s.card(card).id.clone();
        find_instance_mut(s.state_mut(), &id).expect("the card is in the state").times_played =
            Some(times);
    }

    /// A board where p1 can play the summit and neither side auto-ends its turn.
    fn board(options: BoardOptions) -> Scenario {
        let mut p1 = json!({
            "hand": [{ "def": SUMMIT, "radiant": options.radiant }, FILLER],
            "field": [VANILLA],
            "library": [FILLER, FILLER, FILLER, FILLER],
            "mana": 10,
        });
        if options.twinspell {
            p1["backrow"] = json!([TWINSPELL]);
        }
        let p2 = json!({
            "hand": [FILLER],
            "field": [VANILLA],
            "library": [FILLER, FILLER, FILLER, FILLER],
        });
        if let Some(health) = options.health {
            p1["health"] = json!(health);
        }
        let mut s = scenario(json!({ "seed": "summit", "p1": p1, "p2": p2 }));
        if let Some(times) = options.times_played {
            set_times_played(&mut s, SUMMIT, times);
        }
        s
    }

    fn preview_of(s: &Scenario, card: &str) -> Option<i32> {
        let HandView::Cards(hand) = s.view(P1).you.hand else {
            panic!("own hand in full");
        };
        hand.into_iter()
            .find(|held| held.def_id == card)
            .and_then(|held| held.preview)
            .and_then(|preview| preview.first().map(|each| each.value))
    }

    mod base {
        use super::*;

        #[test]
        fn r840_level_0_heals_2_and_climbs() {
            crate::register_all();
            let mut s = board(BoardOptions { health: Some(20), ..Default::default() });
            s.play(SUMMIT, json!({}));
            s.expect_health(P1, 22);
            assert_eq!(times_played_of(s.card(SUMMIT)), 1);
            s.end_turn();
            s.expect_in_zone(SUMMIT, "hand");
            assert_eq!(preview_of(&s, SUMMIT), Some(1));
        }

        #[test]
        fn r840_a_copy_starts_at_0() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "summit-copy",
                "p1": {
                    "hand": [SUMMIT, SUMMIT, FILLER],
                    "field": [VANILLA],
                    "library": [FILLER, FILLER, FILLER, FILLER],
                    "mana": 10,
                    "health": 20,
                },
                "p2": { "hand": [FILLER], "field": [VANILLA], "library": [FILLER, FILLER] },
            }));
            s.play(SUMMIT, json!({}));
            s.expect_health(P1, 22);
            // The copy still in hand never climbed: no plays, level 0.
            assert_eq!(preview_of(&s, SUMMIT), Some(0));
            let levels: Vec<i32> = s
                .hand(P1)
                .into_iter()
                .filter(|held| held.def_id == SUMMIT)
                .map(|held| times_played_of(&held))
                .collect();
            assert_eq!(levels, vec![0]);
        }

        #[test]
        fn r840_an_echo_repeat_resolves_the_same_level() {
            crate::register_all();
            let mut s = board(BoardOptions { health: Some(20), twinspell: true, ..Default::default() });
            s.play(SUMMIT, json!({}));
            // Level 0 twice: the repeat is no play, so the count stays 1.
            s.expect_health(P1, 24);
            assert_eq!(times_played_of(s.card(SUMMIT)), 1);
        }

        #[test]
        fn r840_radiant_keeps_the_level() {
            crate::register_all();
            let mut s = board(BoardOptions { health: Some(20), ..Default::default() });
            set_times_played(&mut s, SUMMIT, 2);
            // Made Radiant mid-climb: the count rides the instance, so level 2 still draws.
            let id = s.card(SUMMIT).id.clone();
            find_instance_mut(s.state_mut(), &id).expect("the summit").radiant = true;
            let hand = s.hand(P1).len();
            s.play(SUMMIT, json!({}));
            assert_eq!(s.hand(P1).len(), hand + 3 - 1);
        }

        #[test]
        fn r841_level_2_draws_2() {
            crate::register_all();
            let mut s = board(BoardOptions { times_played: Some(2), ..Default::default() });
            let hand = s.hand(P1).len();
            s.play(SUMMIT, json!({}));
            assert_eq!(s.hand(P1).len(), hand + 2 - 1);
        }

        #[test]
        fn r841_level_3_discards_2_random_enemy_cards() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "summit-discard",
                "p1": {
                    "hand": [SUMMIT],
                    "field": [VANILLA],
                    "library": [FILLER, FILLER, FILLER, FILLER],
                    "mana": 10,
                },
                "p2": { "hand": [FILLER, FILLER, FILLER], "field": [VANILLA], "library": [FILLER] },
            }));
            set_times_played(&mut s, SUMMIT, 3);
            s.play(SUMMIT, json!({}));
            assert_eq!(s.hand(P2).len(), 1);
        }

        #[test]
        fn r800_r841_level_3_against_a_guarded_hand_discards_nothing() {
            crate::register_all();
            // p2's M #1 Disruptive Disruptor guards p2's hand on p1's turn: the discard is stopped.
            let mut s = scenario(json!({
                "seed": "summit-discard",
                "p1": {
                    "hand": [SUMMIT],
                    "field": [VANILLA],
                    "library": [FILLER, FILLER, FILLER, FILLER],
                    "mana": 10,
                },
                "p2": {
                    "hand": [FILLER, FILLER, FILLER],
                    "field": [VANILLA],
                    "backrow": [{ "def": "meditative-001", "faceUp": true, "lane": 1 }],
                    "library": [FILLER],
                },
            }));
            set_times_played(&mut s, SUMMIT, 3);
            s.play(SUMMIT, json!({}));
            assert_eq!(s.hand(P2).len(), 3);
            assert!(s.events().iter().any(|event| matches!(
                event,
                GameEvent::DiscardPrevented { player, count: 2 } if *player == P2
            )));
        }

        #[test]
        fn r841_level_4_exiles_2_different_enemy_permanents_including_a_face_down_trap() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "summit-exile",
                "p1": {
                    "hand": [SUMMIT],
                    "field": [VANILLA],
                    "library": [FILLER, FILLER, FILLER, FILLER],
                    "mana": 10,
                },
                "p2": {
                    "hand": [FILLER],
                    "field": [VANILLA, VANILLA],
                    "backrow": [{ "def": TAX, "faceUp": false }],
                    "library": [FILLER],
                },
            }));
            set_times_played(&mut s, SUMMIT, 4);
            s.play(SUMMIT, json!({}));
            assert_eq!(s.state().players.p2.exile.len(), 2);
            let exiled: Vec<String> =
                s.state().players.p2.exile.iter().map(|card| card.def_id.clone()).collect();
            assert_eq!(exiled.len(), 2);
            assert!(exiled.iter().all(|def| def == VANILLA || *def == TAX));
        }

        #[test]
        fn r841_level_5_adds_2_radiant_cards_costing_0_never_itself() {
            crate::register_all();
            let _preview = preview_sets(&[SetName::Meditative]);
            let mut s = scenario(json!({
                "seed": "summit-add",
                "p1": {
                    "hand": [SUMMIT],
                    "field": [VANILLA],
                    "library": [FILLER, FILLER, FILLER, FILLER],
                    "mana": 10,
                },
                "p2": { "hand": [FILLER], "field": [VANILLA], "library": [FILLER] },
            }));
            set_times_played(&mut s, SUMMIT, 5);
            s.play(SUMMIT, json!({}));
            let added: Vec<CardInstance> = s
                .hand(P1)
                .into_iter()
                .filter(|held| held.def_id != SUMMIT)
                .collect();
            assert_eq!(added.len(), 2);
            for card in &added {
                assert!(card.radiant, "added Radiant");
                assert_ne!(card.def_id, SUMMIT, "never itself");
                assert_eq!(effective_cost(s.state(), card, Default::default()), 0);
            }
        }

        #[test]
        fn r841_level_6_does_nothing() {
            crate::register_all();
            let mut s = board(BoardOptions { times_played: Some(6), ..Default::default() });
            let health = s.state().players.p1.hero.health;
            let hand = s.hand(P1).len();
            s.play(SUMMIT, json!({}));
            assert_eq!(s.state().players.p1.hero.health, health);
            assert_eq!(s.hand(P1).len(), hand - 1);
        }

        #[test]
        fn r842_costs_its_level_and_discounts_apply() {
            crate::register_all();
            let s = board(BoardOptions { times_played: Some(3), ..Default::default() });
            assert_eq!(play_cost(s.state(), s.card(SUMMIT)), 3);
            let mut s = board(BoardOptions { ..Default::default() });
            assert_eq!(play_cost(s.state(), s.card(SUMMIT)), 0);
            // R65's modifiers apply on top of the computed cost.
            s.state_mut().players.p1.mods.push(PlayerModifier {
                id: "test-discount".to_string(),
                expiry: ModifierExpiry::Never,
                kind: ModifierKind::CostDiscount {
                    amount: 1,
                    only_type: None,
                    min_current_cost: None,
                    once_per_turn: None,
                },
            });
            set_times_played(&mut s, SUMMIT, 3);
            assert_eq!(play_cost(s.state(), s.card(SUMMIT)), 2);
        }

        #[test]
        fn r842_radiant_costs_level_minus_1_at_least_0() {
            crate::register_all();
            let s = board(BoardOptions { times_played: Some(3), radiant: true, ..Default::default() });
            assert_eq!(play_cost(s.state(), s.card(SUMMIT)), 2);
            let s = board(BoardOptions { radiant: true, ..Default::default() });
            assert_eq!(play_cost(s.state(), s.card(SUMMIT)), 0);
        }

        #[test]
        fn r843_level_1_prompts_and_deals_2() {
            crate::register_all();
            let mut s = board(BoardOptions { times_played: Some(1), ..Default::default() });
            s.play(SUMMIT, json!({}));
            assert!(s.state().pending.is_some(), "a resolution prompt opens");
            let target = s.card(VANILLA).id.clone();
            s.answer(json!([{ "pick": "instance", "instanceId": target }]));
            // The 4/4 took 2.
            let unit = s.unit(P1, 1).expect("the unit stands");
            assert_eq!(unit.damage, 2);
        }

        #[test]
        fn r843_a_random_cast_picks_the_target_itself() {
            crate::register_all();
            let mut s = board(BoardOptions { times_played: Some(1), ..Default::default() });
            let summit = s.card(SUMMIT).id.clone();
            let mut rng = create_rng(&s.state().seed, s.state().rng_cursor);
            let mut events: Vec<GameEvent> = Vec::new();
            {
                let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
                {
                    let mut ctx = make_context(
                        &mut sink,
                        None,
                        HookOptions { controller: Some(P1), ..HookOptions::default() },
                    );
                    apply_effects(
                        &[cast(json_as(json!({
                            "target": { "of": "instance", "instanceId": summit },
                            "random": true,
                        })))],
                        &mut ctx,
                    );
                }
                settle(&mut sink, SettleOptions::default());
            }
            s.state_mut().rng_cursor = rng.cursor();
            // No prompt stays open: the random cast picked the target itself.
            assert!(s.state().pending.is_none());
            assert_eq!(times_played_of(s.card(SUMMIT)), 2);
        }

        #[test]
        fn r850_level_10_wins_unless_the_casters_hero_is_at_0() {
            crate::register_all();
            let mut s = board(BoardOptions { times_played: Some(10), ..Default::default() });
            s.state_mut().players.p1.mana.current = 10;
            s.play(SUMMIT, json!({}));
            let result = s.state().result.expect("Ascent 10 wins");
            assert_eq!(result.winner, Winner::P1);
            assert_eq!(result.reason, GameOverReason::WonByEffect);

            // A hero at 0 loses even holding the win.
            let mut s = board(BoardOptions { times_played: Some(10), ..Default::default() });
            s.state_mut().players.p1.mana.current = 10;
            s.state_mut().players.p1.hero.health = 0;
            s.play(SUMMIT, json!({}));
            let result = s.state().result.expect("the game ends");
            assert_eq!(result.winner, Winner::P2);
        }

        #[test]
        fn returns_only_on_the_turn_played() {
            crate::register_all();
            let mut s = board(BoardOptions::default());
            let summit = s.card(SUMMIT).clone();
            s.play(SUMMIT, json!({}));
            s.expect_in_zone(&summit, "graveyard");
            s.end_turn();
            s.expect_in_zone(&summit, "hand");
            // A later turn's end leaves it where it is.
            s.end_turn();
            s.expect_in_zone(&summit, "hand");
        }

        #[test]
        fn a_full_hand_burns_the_return() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "summit-burn",
                "p1": {
                    "hand": [SUMMIT],
                    "field": [VANILLA],
                    "library": [FILLER, FILLER, FILLER, FILLER],
                    "mana": 10,
                },
                "p2": { "hand": [FILLER], "field": [VANILLA], "library": [FILLER] },
            }));
            let summit = s.card(SUMMIT).clone();
            s.play(SUMMIT, json!({}));
            for _ in 0..10 {
                s.state_mut().players.p1.hand.push(summit.clone());
            }
            assert_eq!(s.hand(P1).len(), 10);
            s.end_turn();
            s.expect_in_zone(&summit, "graveyard");
        }

        #[test]
        fn the_preview_shows_the_level() {
            crate::register_all();
            let s = board(BoardOptions::default());
            assert_eq!(preview_of(&s, SUMMIT), Some(0));
            let s = board(BoardOptions { times_played: Some(4), ..Default::default() });
            assert_eq!(preview_of(&s, SUMMIT), Some(4));
        }

        #[test]
        fn radiant_numbers_are_3() {
            crate::register_all();
            let def = crate::card_def(SUMMIT);
            for key in ["heal", "damage", "draw", "discards", "exiles", "adds"] {
                let param = def.params.as_ref().expect("params").iter().find(|entry| entry.key == key).expect("the param");
                assert_eq!((param.base, param.radiant), (2, 3), "for {key}");
            }
        }

        #[test]
        fn level_1_answers_a_declared_enemy_hero_pick_with_a_prompt() {
            crate::register_all();
            // No declared target: the play carries none, the prompt opens at resolution.
            let mut s = board(BoardOptions { times_played: Some(1), ..Default::default() });
            s.play(SUMMIT, json!({}));
            assert!(s.state().pending.is_some());
        }
    }
}
