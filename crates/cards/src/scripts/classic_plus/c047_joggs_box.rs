//! C+ #47 Jogg's Box (SPEC §8.7 row 47). (4) Spell, Legendary.
//!   Engine:  "Random casts (Cast, §6.3), one after another: each a random non-token card of the Spell
//!            type (not Field Spell or Trap) of every set (R380) but Jogg's Box (R387), repeats allowed
//!            (R60), cast from no zone. Every choice of a random cast is random, targets, modes and
//!            Discover picks alike, as "Targets chosen randomly" (§6.2), and its X is your current mana,
//!            at least 1; so nothing pauses for a prompt. Each cast is free and counts as a play (R70),
//!            and each cast Spell goes to your graveyard when it resolves (R87)."
//!
//! `castRandom` (B5 E12, R452) is the card's casts: each cast draws its Spell as it begins, never this
//! card's definition, and makes every choice from the match rng; a Call to Chaos it casts is the first
//! link of its chain (R28, R593). "End the turn" (since #3140) is the verb `end_turn` last in the list
//! (R456): the casts and everything they set off resolve first, and the turn then ends as if its player
//! had pressed End turn, every end-of-turn step running (R62); on the opponent's turn it does nothing.
//! Base "Cast {casts} random Spells. End the turn." (10); Radiant the same, 20 (`casts` 10 → 20). The
//! Radiant face had Echo 1 through v0.5.11p, a repeat that ran the Cry again with fresh picks, and the
//! Box ended no turn (`script_v0_5_11o`, R767000).

use jackioh_engine::effects::{CastRandomArgs, CastRandomCount, CastRandomQuery, cast_random};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-047";

fn cast_spells(count: i32) -> Effect {
    cast_random(CastRandomArgs {
        query: CastRandomQuery::Fixed(json_as(json!({ "type": "Spell" }))),
        count: Some(CastRandomCount::Fixed(count)),
        radiant: None,
        target_enemies: None,
        afterward: None,
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| vec![cast_spells(param(&*ctx, "casts")), end_turn(json_as(json!({})))])),
        ..Script::default()
    };

    // The same script: the Radiant face differs only in its declared `casts`, read through `param`.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

/// R767000: through v0.5.11p the Box ended no turn, and its Radiant face had Echo 1 (20 casts as two runs of
/// 10); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_11o() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| vec![cast_spells(param(&*ctx, "casts"))])),
        ..Script::default()
    };

    let radiant = Script {
        static_flags: Some(StaticFlags {
            echo: Some(1),
            ..StaticFlags::default()
        }),
        ..base.clone()
    };

    CardScripts { base, radiant }
}

// C+ #47 Jogg's Box — SPEC §8.7 row 47, BUILD M9 Classic+ row C+ 47.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BOX: &str = "classicplus-047";
    const FILLER: &str = "core-005";
    const VANILLA: &str = "core-008";
    /// Every Call to Chaos edition (R380): Core #95, C+ #73, M #95, Stinkyscape #150 and #151 (#1884)
    /// and Training Grounds #39 (#2204, R2200004).
    const CALLS: [&str; 6] = [
        "core-095",
        "classicplus-073",
        "meditative-095",
        "stinkyscape-150",
        "stinkyscape-151",
        "traininggrounds-039",
    ];

    #[derive(Clone, Debug)]
    struct Played {
        player: PlayerId,
        instance_id: String,
        def_id: String,
        cost_paid: i32,
    }

    /// `box_`'s options (`box` is a Rust keyword).
    #[derive(Default)]
    struct BoxOpts {
        seed: String,
        radiant: bool,
        empty: bool,
        health: Option<i32>,
        library: Option<Vec<&'static str>>,
    }

    fn seeded(seed: impl Into<String>) -> BoxOpts {
        BoxOpts {
            seed: seed.into(),
            ..BoxOpts::default()
        }
    }

    fn box_(opts: BoxOpts) -> Scenario {
        let mut card = json!({ "def": BOX });
        if opts.radiant {
            card["radiant"] = json!(true);
        }
        let field = if opts.empty { json!([]) } else { json!([VANILLA]) };
        let mut p1 = json!({
            "hand": [card, FILLER],
            "library": opts.library.unwrap_or_else(|| vec![VANILLA, VANILLA, VANILLA]),
            "field": field.clone(),
        });
        let mut p2 = json!({
            "hand": [FILLER, FILLER],
            "field": field,
            "library": [VANILLA, VANILLA],
        });
        if let Some(health) = opts.health {
            p1["health"] = json!(health);
            p2["health"] = json!(health);
        }
        scenario(json!({ "seed": opts.seed, "p1": p1, "p2": p2 }))
    }

    /// The casts Jogg's Box made itself: plays begun while it alone was resolving (nested casts left out).
    fn casts(events: &[GameEvent], box_id: &str) -> Vec<Played> {
        let mut open: Vec<String> = Vec::new();
        let mut out = Vec::new();
        for event in events {
            if let GameEvent::CardPlayed {
                player,
                instance_id,
                def_id,
                cost_paid,
                ..
            } = event
            {
                if open.len() == 1 && open[0] == box_id {
                    out.push(Played {
                        player: *player,
                        instance_id: instance_id.clone(),
                        def_id: def_id.clone(),
                        cost_paid: *cost_paid,
                    });
                }
                open.push(instance_id.clone());
            }
            if let GameEvent::CardResolved { instance_id, .. } = event
                && let Some(at) = open.iter().rposition(|id| id == instance_id)
            {
                open.remove(at);
            }
        }
        out
    }

    /// A prompt's first options, at least one.
    fn first_picks(pending: &PendingChoice) -> Vec<Selection> {
        pending
            .options
            .iter()
            .take(pending.min.max(1) as usize)
            .map(|option| option.selection.clone())
            .collect()
    }

    /// Answer the standing prompt with its first options.
    fn answer_first(s: &mut Scenario) {
        let picks = first_picks(s.state().pending.as_ref().expect("a prompt"));
        s.answer(json!(picks));
    }

    /// Play the Box and answer every prompt the other player is asked with its first options.
    fn open(s: &mut Scenario) -> Vec<Played> {
        let id = s.card(BOX).id.clone();
        let from = s.events().len();
        s.play(BOX, json!({}));
        let mut guard = 0;
        while guard < 20 && s.state().pending.is_some() && s.state().result.is_none() {
            answer_first(s);
            guard += 1;
        }
        casts(&s.events()[from..], &id)
    }

    /// Seeds that cast all their Spells with no prompt and no end of the game, and end p1's turn (R456).
    fn quiet_seed(make: impl Fn(&str) -> Scenario, want: usize) -> (Scenario, Vec<Played>) {
        for i in 0..60 {
            let mut s = make(&format!("jogg-quiet-{i}"));
            let id = s.card(BOX).id.clone();
            s.play(BOX, json!({}));
            let cast = casts(s.last_events(), &id);
            // R456: the Box ends the turn, so p2 holds it with nothing asked of anyone; keep a seed whose casts
            // asked nothing, ended no game and gave no extra turn.
            let quiet = s.state().pending.is_none() && s.state().result.is_none() && s.state().active == P2;
            if quiet && cast.len() == want {
                return (s, cast);
            }
        }
        panic!("no quiet seed");
    }

    /// Runs its closure when dropped, on a panic too.
    struct Finally<F: FnOnce()>(Option<F>);

    impl<F: FnOnce()> Drop for Finally<F> {
        fn drop(&mut self) {
            if let Some(f) = self.0.take() {
                f();
            }
        }
    }

    /// The shipped catalog with every Spell but the Box and `keep` left out.
    fn only_spells(keep: &str) -> CardDefs {
        crate::CATALOG
            .iter()
            .filter(|(id, entry)| entry.type_ != CardType::Spell || id.as_str() == BOX || id.as_str() == keep)
            .map(|(id, entry)| (id.clone(), entry.clone()))
            .collect()
    }

    /// Registers `defs`; the guard puts the shipped catalog back.
    fn with_catalog(defs: CardDefs) -> Finally<impl FnOnce()> {
        jackioh_engine::testkit::register_catalog_as(defs, crate::CATALOG_VERSION);
        Finally(Some(|| {
            jackioh_engine::testkit::register_catalog_as(crate::CATALOG.clone(), crate::CATALOG_VERSION);
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r452_r70_it_casts_10_random_spells_one_after_another_each_a_free_play_landing_with_you_r87() {
            crate::register_all();
            let (mut s, cast) = quiet_seed(|seed| box_(seeded(seed)), 10);
            assert_eq!(cast.len(), 10);
            for played in &cast {
                assert_eq!(played.player, P1);
                assert_eq!(played.cost_paid, 0);
                let card = s.card(&played.instance_id);
                assert_eq!(card.owner, P1);
                assert_ne!(card.zone.z(), ZoneName::Resolving);
            }
            // The Box and its ten casts (and any cast they set off) were p1's plays.
            let plays = s
                .last_events()
                .iter()
                .filter(|event| matches!(event, GameEvent::CardPlayed { player: P1, .. }))
                .count();
            assert!(plays >= 11);
            s.expect_in_zone(BOX, "graveyard");
        }

        #[test]
        fn r456_the_turn_ends_once_the_casts_have_resolved_every_end_of_turn_step_running() {
            crate::register_all();
            let (s, _) = quiet_seed(|seed| box_(seeded(seed)), 10);
            assert_eq!(s.state().active, P2, "p1's turn ended");
            let events = s.last_events();
            let last_cast = events
                .iter()
                .rposition(|event| matches!(event, GameEvent::CardPlayed { player: P1, .. }))
                .expect("the casts");
            let ended = events
                .iter()
                .position(|event| matches!(event, GameEvent::TurnEnded { player: P1, .. }))
                .expect("p1's turn ended");
            assert!(last_cast < ended, "the turn ends last");
        }

        #[test]
        fn r456_a_degrade_leaves_it_ending_the_turn_all_the_same() {
            crate::register_all();
            let (s, cast) = quiet_seed(
                |seed| {
                    let mut s = box_(seeded(seed));
                    step_param(s.card_mut(BOX), "casts", -3);
                    s
                },
                4,
            );
            assert_eq!(cast.len(), 4);
            assert_eq!(s.state().active, P2);
        }

        #[test]
        fn r380_r387_over_many_seeds_non_token_spells_of_every_set_never_jogg_s_box() {
            crate::register_all();
            let mut seen: IndexSet<String> = IndexSet::new();
            for i in 0..25 {
                let mut s = box_(seeded(format!("jogg-pool-{i}")));
                for played in open(&mut s) {
                    seen.insert(played.def_id);
                }
            }
            let pool = box_(seeded("pool"));
            let state = pool.state();
            let defs: Vec<CardDef> = seen.iter().map(|id| def_of(Some(state), id).clone()).collect();
            assert!(!seen.contains(BOX));
            assert!(defs.iter().all(|entry| entry.type_ == CardType::Spell && !entry.token));
            let sets: IndexSet<SetName> = defs.iter().map(|entry| entry.set).collect();
            // Spells of every set that ships, Meditative+ (#1187) among them.
            assert_eq!(
                sets,
                IndexSet::from([
                    SetName::Core,
                    SetName::Classic,
                    SetName::ClassicPlus,
                    SetName::Meditative,
                    SetName::MeditativePlus,
                    SetName::Stinkyscape,
                    SetName::TrainingGrounds,
                ])
            );
        }

        #[test]
        fn r387_with_jogg_s_box_and_fig_of_life_the_only_spells_all_ten_casts_are_fig_of_life() {
            crate::register_all();
            const FIG: &str = "core-047"; // (3) Spell: heal a target 20
            let _restore = with_catalog(only_spells(FIG));
            let mut s = scenario(json!({
                "seed": "jogg-only-fig",
                "p1": { "hand": [BOX, VANILLA], "field": [VANILLA], "library": [VANILLA] },
                "p2": { "hand": [VANILLA], "field": [VANILLA], "library": [VANILLA] },
            }));
            let id = s.card(BOX).id.clone();
            s.play(BOX, json!({}));
            let cast: Vec<String> = casts(s.last_events(), &id).into_iter().map(|played| played.def_id).collect();
            assert_eq!(cast, vec![FIG.to_string(); 10]);
        }

        #[test]
        fn r656_jogg_s_box_aims_nothing_its_helpful_casts_land_on_both_sides_across_seeds() {
            crate::register_all();
            const FIG: &str = "core-047"; // (3) Spell: heal a target 20 — aimed help, but the Box carries no targetEnemies
            let mut sides: IndexSet<String> = IndexSet::new();
            {
                let _restore = with_catalog(only_spells(FIG));
                for i in 0..10 {
                    let mut s = scenario(json!({
                        "seed": format!("jogg-fig-aim-{i}"),
                        "p1": { "hand": [BOX, VANILLA], "field": [VANILLA], "library": [VANILLA] },
                        "p2": { "hand": [VANILLA], "field": [VANILLA], "library": [VANILLA] },
                    }));
                    s.play(BOX, json!({}));
                    for event in s.last_events() {
                        let GameEvent::Healed { target_id, .. } = event else {
                            continue;
                        };
                        let target = target_id.as_str();
                        let mine = target == "hero-p1" || (target != "hero-p2" && s.card(target).controller == P1);
                        sides.insert(if mine { "p1" } else { "p2" }.to_string());
                    }
                }
            }
            assert_eq!(sides, IndexSet::from(["p1".to_string(), "p2".to_string()]));
        }

        #[test]
        fn r452_r471_a_cast_book_of_plague_c_n70_places_its_tokens_at_random_its_caster_is_never_asked() {
            crate::register_all();
            const PLAGUE_BOOK: &str = "classic-070"; // (1) Spell: "Place {tokens} Plague Counters."
            let _restore = with_catalog(only_spells(PLAGUE_BOOK));
            let mut s = scenario(json!({
                "seed": "jogg-only-plague",
                "p1": { "hand": [BOX, VANILLA], "field": [VANILLA], "library": [VANILLA] },
                "p2": { "hand": [VANILLA], "field": [VANILLA], "library": [VANILLA] },
            }));
            s.play(BOX, json!({}));
            assert!(s.state().pending.is_none());
            let placed: i32 = [s.unit(P1, 1), s.unit(P2, 1)]
                .iter()
                .map(|unit| unit.as_ref().and_then(|unit| unit.counters.plague).unwrap_or(0))
                .sum();
            assert!(placed >= 10);
        }

        #[test]
        fn s6_2_r452_every_choice_is_random_its_caster_is_never_asked() {
            crate::register_all();
            for i in 0..25 {
                let mut s = box_(seeded(format!("jogg-ask-{i}")));
                s.play(BOX, json!({}));
                let mut guard = 0;
                while guard < 20 && s.state().pending.is_some() {
                    assert_eq!(s.state().pending.as_ref().map(|pending| pending.player_id), Some(P2));
                    answer_first(&mut s);
                    guard += 1;
                }
            }
        }

        #[test]
        fn r453_an_x_spell_it_casts_takes_your_current_mana_as_x_at_least_1() {
            crate::register_all();
            let mut checked = 0;
            for i in 0..60 {
                if checked >= 3 {
                    break;
                }
                let mut s = box_(seeded(format!("jogg-x-{i}")));
                let id = s.card(BOX).id.clone();
                s.play(BOX, json!({}));
                let top: IndexSet<String> =
                    casts(s.last_events(), &id).into_iter().map(|played| played.instance_id).collect();
                let mut mana = s.state().players.p1.mana.current;
                let mut paid = false;
                for event in s.last_events() {
                    if let GameEvent::ManaChanged { player, current, .. } = event
                        && *player == P1
                    {
                        mana = *current;
                        paid = true;
                    }
                    let GameEvent::CardPlayed {
                        instance_id,
                        x: Some(x),
                        ..
                    } = event
                    else {
                        continue;
                    };
                    if !top.contains(instance_id) {
                        continue;
                    }
                    assert!(paid);
                    assert_eq!(*x, mana.max(1));
                    checked += 1;
                }
            }
            assert!(checked > 0);
        }

        #[test]
        fn a_cast_with_no_legal_target_fizzles_and_the_next_goes() {
            crate::register_all();
            let cards = crate::scripts_of();
            let unit_only = |id: &str| -> bool {
                cards.get(id).is_some_and(|card| {
                    card.base.targets.iter().any(|decl| {
                        decl.kind == PromptKind::Target
                            && decl
                                .filter
                                .as_ref()
                                .and_then(|filter| filter.of.as_ref())
                                .is_some_and(|of| {
                                    of.iter().map(|o| o.as_str()).collect::<Vec<_>>().join(",") == "unit"
                                })
                    })
                })
            };
            for i in 0..80 {
                let mut s = box_(BoxOpts {
                    empty: true,
                    ..seeded(format!("jogg-fizzle-{i}"))
                });
                let id = s.card(BOX).id.clone();
                s.play(BOX, json!({}));
                let cast = casts(s.last_events(), &id);
                let first = cast.first().map(|played| played.def_id.clone()).unwrap_or_default();
                if s.state().pending.is_some() || s.state().result.is_some() || !unit_only(&first) {
                    continue;
                }
                assert_eq!(cast.len(), 10);
                return;
            }
            panic!("no seed opened on a Unit-only target with an empty board");
        }

        #[test]
        fn r593_r28_a_call_to_chaos_among_them_is_the_first_cast_of_its_chain_which_stops_at_call_to_chaos_chain_cap() {
            crate::register_all();
            let saved = jackioh_engine::scripts::registered_scripts();
            // Each real cast's depth by instance: a Zephyrs-style scorer among the casts runs a Call's Cry on a
            // simulated state too, and those runs are no cast of this game (R29).
            let (runs_in, runs) = std::sync::mpsc::channel::<(String, i32)>();
            let recurse: Hook = hook(move |ctx| {
                let id = ctx.self_.as_ref().map(|card| card.id.clone()).unwrap_or_default();
                let _ = runs_in.send((id, subsystems::chaos_chain_of(ctx.self_.as_ref())));
                vec![subsystems::cast_random_call_to_chaos()]
            });
            let mut scripts = saved.clone();
            for id in CALLS {
                scripts.insert(
                    id.to_string(),
                    CardScripts {
                        base: Script {
                            cry: Some(recurse.clone()),
                            ..Script::default()
                        },
                        radiant: Script {
                            cry: Some(recurse.clone()),
                            ..Script::default()
                        },
                    },
                );
            }
            jackioh_engine::testkit::register_scripts(scripts);
            let _restore = Finally(Some(move || {
                jackioh_engine::testkit::register_scripts(saved);
            }));
            for i in 0..80 {
                let mut s = box_(seeded(format!("jogg-chaos-{i}")));
                s.play(BOX, json!({}));
                // This seed's runs are the ones sent since the last drain.
                let seed_runs: Vec<(String, i32)> = runs.try_iter().collect();
                let played: IndexSet<String> = s
                    .last_events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::CardPlayed { instance_id, .. } => Some(instance_id.clone()),
                        _ => None,
                    })
                    .collect();
                let depths: Vec<i32> = seed_runs
                    .iter()
                    .filter(|(id, _)| played.contains(id))
                    .map(|(_, depth)| *depth)
                    .collect();
                if depths.is_empty() {
                    continue;
                }
                // Links 1 to the cap, then nothing more of that chain (a later Call the Box casts starts at 1).
                let cap = CALL_TO_CHAOS_CHAIN_CAP as usize;
                assert_eq!(
                    depths.iter().take(cap).copied().collect::<Vec<i32>>(),
                    (1..=CALL_TO_CHAOS_CHAIN_CAP).collect::<Vec<i32>>()
                );
                assert_eq!(depths.get(cap).copied().unwrap_or(1), 1);
                return;
            }
            panic!("no seed cast a Call to Chaos");
        }

        #[test]
        fn s2_5_a_game_that_ends_midway_stops_the_rest() {
            crate::register_all();
            for i in 0..60 {
                let mut s = box_(BoxOpts {
                    health: Some(1),
                    library: Some(vec![]),
                    ..seeded(format!("jogg-end-{i}"))
                });
                let id = s.card(BOX).id.clone();
                s.play(BOX, json!({}));
                let over = s
                    .last_events()
                    .iter()
                    .position(|event| matches!(event, GameEvent::GameOver { .. }));
                // A game ended by the tenth cast has nothing left to stop: look for one that ends sooner.
                let Some(over) = over else {
                    continue;
                };
                if casts(s.last_events(), &id).len() >= 10 {
                    continue;
                }
                assert!(s.state().result.is_some());
                assert!(casts(s.last_events(), &id).len() < 10);
                assert!(!s.last_events()[over + 1..].iter().any(|event| matches!(
                    event,
                    GameEvent::CardPlayed { .. } | GameEvent::CardAnnounced { .. }
                )));
                return;
            }
            panic!("no seed ended the game");
        }

        #[test]
        fn s9_3_a_fixed_seed_casts_the_same_ten() {
            crate::register_all();
            let mut a = box_(seeded("jogg-same"));
            let mut b = box_(seeded("jogg-same"));
            let left: Vec<String> = open(&mut a).into_iter().map(|played| played.def_id).collect();
            let right: Vec<String> = open(&mut b).into_iter().map(|played| played.def_id).collect();
            assert_eq!(left, right);
            assert_eq!(hash_state(a.state()), hash_state(b.state()));
        }

        #[test]
        fn r113_another_player_s_prompt_pauses_the_rest_answered_after_a_json_round_trip_the_rest_follow() {
            crate::register_all();
            for i in 0..60 {
                let mut s = box_(seeded(format!("jogg-pause-{i}")));
                let id = s.card(BOX).id.clone();
                let from = s.events().len();
                s.play(BOX, json!({}));
                if s.state().result.is_some() {
                    continue;
                }
                // R1279000: even a cast Pickle asks nothing at once, so the box casts all ten without
                // pausing on the other player's prompt, and then ends p1's turn (R456).
                assert_eq!(casts(s.last_events(), &id).len(), 10);
                // ...unless the box cast no Pickle: the deferred question opens on p2's turn, which began as p1's
                // ended, pausing the rest of the turn-start order behind it.
                let Some(pending) = s.state().pending.clone() else {
                    continue;
                };
                // A box that cast a card asking its own caster (#2521's pool) pauses on p1's prompt instead:
                // this test wants the other player's.
                if pending.player_id != P2 {
                    continue;
                }
                assert_eq!(pending.kind, PromptKind::Mode);

                let revived: GameState =
                    serde_json::from_str(&serde_json::to_string(s.state()).expect("serializes")).expect("parses");
                assert_eq!(&revived, s.state());
                let picks = first_picks(&pending);
                let resumed = reduce(
                    &revived,
                    &json_as::<Action>(json!({
                        "type": "answer",
                        "playerId": "p2",
                        "choiceId": pending.id,
                        "selection": picks,
                        "nonce": "jogg-pause",
                    })),
                );
                assert!(resumed.error.is_none());
                s.answer(json!(picks));
                assert_eq!(hash_state(&resumed.state), hash_state(s.state()));

                let mut guard = 0;
                while guard < 20 && s.state().pending.is_some() {
                    answer_first(&mut s);
                    guard += 1;
                }
                if s.state().result.is_none() {
                    // The paused rest followed the answers: the owed start-of-turn draw landed. The first
                    // game that pauses has the box cast Core #90 Super-Virus Injection (#2204), so p2's draw
                    // takes a Super-Virus, cast on its draw (§2.4), and goes on to the next card; one card of
                    // p2's draws reached its hand.
                    assert!(s.state().pending.is_none());
                    assert_eq!(
                        s.events()[from..]
                            .iter()
                            .filter(|event| matches!(
                                event,
                                GameEvent::Drawn { player: P2, instance_id, .. }
                                    if s.card(instance_id).zone == Zone::Hand { player: P2 }
                            ))
                            .count(),
                        1
                    );
                }
                return;
            }
            panic!("no seed paused on the other player's prompt");
        }

        #[test]
        fn r386_an_upgrade_casts_12_step_2_a_degrade_8() {
            crate::register_all();
            let (_, up) = quiet_seed(
                |seed| {
                    let mut s = box_(seeded(seed));
                    step_param(s.card_mut(BOX), "casts", 1);
                    s
                },
                12,
            );
            assert_eq!(up.len(), 12);

            let (_, down) = quiet_seed(
                |seed| {
                    let mut s = box_(seeded(seed));
                    step_param(s.card_mut(BOX), "casts", -1);
                    s
                },
                8,
            );
            assert_eq!(down.len(), 8);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r456_it_casts_20_once_with_no_echo_and_then_ends_the_turn() {
            crate::register_all();
            let (s, cast) = quiet_seed(
                |seed| {
                    box_(BoxOpts {
                        radiant: true,
                        ..seeded(seed)
                    })
                },
                20,
            );
            assert_eq!(cast.len(), 20);
            assert!(!cast.iter().any(|played| played.def_id == BOX));
            assert_eq!(s.state().active, P2, "p1's turn ended");
            // One Cry of twenty: the Radiant face has no Echo, in its text or in its script.
            let def = crate::card_def(BOX);
            assert_eq!(def.radiant.text, "Cast {casts|random Spell|random Spells}. End the turn.");
            assert!(def.radiant.keywords.is_empty());
            let scripts = crate::scripts_of();
            let flags = scripts.get(BOX).and_then(|card| card.radiant.static_flags.clone());
            assert!(flags.and_then(|flags| flags.echo).is_none());
        }

        #[test]
        fn r386_an_upgrade_casts_22_step_2_a_degrade_18() {
            crate::register_all();
            let (_, up) = quiet_seed(
                |seed| {
                    let mut s = box_(BoxOpts {
                        radiant: true,
                        ..seeded(seed)
                    });
                    step_param(s.card_mut(BOX), "casts", 1);
                    s
                },
                22,
            );
            assert_eq!(up.len(), 22);

            let (_, down) = quiet_seed(
                |seed| {
                    let mut s = box_(BoxOpts {
                        radiant: true,
                        ..seeded(seed)
                    });
                    step_param(s.card_mut(BOX), "casts", -1);
                    s
                },
                18,
            );
            assert_eq!(down.len(), 18);
        }
    }

    /// R767000: the script kept for the replays of v0.5.11p and before: no End the turn, and Echo 1 on the
    /// Radiant face, whose `casts` was 10.
    mod v0_5_11o {
        use super::*;

        #[test]
        fn r767000_a_game_on_v0_5_11o_casts_10_and_ends_no_turn_and_the_radiant_face_echoes_to_20() {
            crate::register_all();
            let through_v0_5_11o = |def: &mut CardDef| {
                for param in def.params.iter_mut().flatten().filter(|param| param.key == "casts") {
                    param.radiant = 10;
                }
            };
            for (radiant, want) in [(false, 10), (true, 20)] {
                let mut found = false;
                for i in 0..60 {
                    let seed = format!("jogg-kept-{i}");
                    let kept = crate::under_kept_as(BOX, script_v0_5_11o(), through_v0_5_11o, || {
                        let mut s = box_(BoxOpts {
                            radiant,
                            ..seeded(seed)
                        });
                        let id = s.card(BOX).id.clone();
                        s.play(BOX, json!({}));
                        let cast = casts(s.last_events(), &id);
                        let quiet = s.state().pending.is_none() && s.state().result.is_none();
                        (quiet && cast.len() == want).then(|| s.state().active)
                    });
                    if let Some(active) = kept {
                        assert_eq!(active, P1, "radiant {radiant}: the Box ended no turn");
                        found = true;
                        break;
                    }
                }
                assert!(found, "radiant {radiant}: no quiet seed");
            }
        }
    }
}
