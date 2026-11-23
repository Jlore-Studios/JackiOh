//! Stinkyscape #109 xxFreshJack (SPEC §8.10 row 109). (4) Unit, Human, Acclaimed, Mythic, 6/11 → 12/22.
//!   Base:    "Tribute 2
//!            Float reduces Tribute requirement by 1.
//!            Cry: Cast a random BUSTED card from anywhere in the game's code."
//!   Radiant: "Tribute 2
//!            Float reduces Tribute requirement by 1.
//!            Cry: Discover a Radiant BUSTED card from anywhere in the game's code and cast it."
//!
//! Tribute 2 since issue #4440 is the play validator's (§6.3, R101), read off `staticFlags.tribute`, the
//! keyword line's printed Tribute (R366), lowered 1 by each of its owner's Floats while it waits in hand,
//! never in the deck (R2657002), never below 0 (`float_lowers_tribute`, R1841003, as S+ #21 Jlockeed
//! Shredder-12's). After v0.6.6h through v0.6.9n it was Tribute 1 with no Float line (issue #4154,
//! `script_v0_6_9n`, kept, R767000), and through v0.6.6h it took none (`script_v0_6_6h`, kept).
//!
//! Cry (R1): the pool is `{ tier: "S", everySet: true, past: "also" }` (R3538005): every S-tier card
//! of the sets that ship and the single-player sets, current or past, never this card (R387). The
//! base face casts one at random (`cast_random`, R452): free, a play for every rule (R70), every
//! choice random. The Radiant face Discovers one of three (§6.3) and casts it Radiant with its
//! caster's choices (`cast_new`, E12, R453).

use jackioh_engine::effects::{CastRandomArgs, CastRandomQuery, cast_random};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-109";

/// The resume step the Radiant Discover's answer comes back to (§10.6).
const PICKED: &str = "picked";

/// R3538005: "a BUSTED card that exists anywhere in the game's code": S tier, current or past, of
/// every set.
fn busted_anywhere() -> CatalogQueryArgs {
    json_as(json!({ "tier": "S", "everySet": true, "past": "also" }))
}

/// "Tribute 2" (R101, R366: printed, declared nowhere), since issue #4440.
const TRIBUTE: i32 = 2;
/// "Tribute 1", after v0.6.6h through v0.6.9n (issue #4154).
const TRIBUTE_V0_6_9G: i32 = 1;

/// `script_v0_6_6h`'s faces played for `tribute` Units; `float`: whether "Float reduces Tribute
/// requirement by 1" is printed (since issue #4440), a hand trigger (R1841003).
fn tributed(tribute: i32, float: bool) -> CardScripts {
    let mut scripts = script_v0_6_6h();
    for face in [&mut scripts.base, &mut scripts.radiant] {
        face.static_flags = Some(StaticFlags {
            tribute: Some(tribute),
            ..StaticFlags::default()
        });
        if float {
            face.hand_triggers = vec![float_lowers_tribute("stinkyscape-109-float-hand")];
        }
    }
    scripts
}

pub fn script() -> CardScripts {
    tributed(TRIBUTE, true)
}

/// R767000: after v0.6.6h through v0.6.9n it was Tribute 1 with no Float line (issue #4440); `KEPT_SCRIPTS`
/// folds those replays under it.
pub fn script_v0_6_9n() -> CardScripts {
    tributed(TRIBUTE_V0_6_9G, false)
}

/// R767000: no Tribute through v0.6.6h (issue #4154); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_6h() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|_ctx| {
                vec![cast_random(CastRandomArgs {
                    query: CastRandomQuery::Fixed(Box::new(busted_anywhere())),
                    count: None,
                    radiant: None,
                    target_enemies: None,
                    afterward: None,
                })]
            })),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| {
                vec![discover_from_catalog(json_as(json!({
                    "step": PICKED,
                    "query": { "tier": "S", "everySet": true, "past": "also" },
                    "prompt": "xxFreshJack: Discover a Radiant BUSTED card to cast",
                })))]
            })),
            resume: IndexMap::from([(
                PICKED,
                hook(|ctx| {
                    let Some(picked) = chosen_options(ctx).into_iter().next() else {
                        return vec![];
                    };
                    vec![cast_new(json_as(json!({ "def": picked, "radiant": true })))]
                }),
            )]),
            ..Script::default()
        },
    }
}

/// R767000: through v0.5.13g, the code-line BUSTED pool; the release re-tags its `KEPT_SCRIPTS` row.
pub fn script_v0_5_13g() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|_ctx| {
                vec![cast_random(CastRandomArgs {
                    query: CastRandomQuery::Fixed(Box::new(json_as(json!({
                        "busted": true,
                        "everySet": true,
                    })))),
                    count: None,
                    radiant: None,
                    target_enemies: None,
                    afterward: None,
                })]
            })),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| {
                vec![discover_from_catalog(json_as(json!({
                    "step": PICKED,
                    "query": { "busted": true, "everySet": true },
                    "prompt": "xxFreshJack: Discover a Radiant BUSTED card to cast",
                })))]
            })),
            resume: IndexMap::from([(
                PICKED,
                hook(|ctx| {
                    let Some(picked) = chosen_options(ctx).into_iter().next() else {
                        return vec![];
                    };
                    vec![cast_new(json_as(json!({ "def": picked, "radiant": true })))]
                }),
            )]),
            ..Script::default()
        },
    }
}

// Stinkyscape #109 xxFreshJack — SPEC §8.10 row 109, BUILD M12 row S 109 (R3538005, R70).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::catalog::is_busted;
    use jackioh_engine::play_choices::tribute_cost_of;
    use jackioh_engine::testkit::*;

    use crate::{filler, next_turn_of, scenario};

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-005"; // Stockpile: a Spell
    /// The units its Tribute 2 takes (issue #4440), in lanes 2 and 3; the first alone paid the Tribute 1 of
    /// v0.6.9n and before.
    const BODY: &str = "stand-in (1) 1/1";
    const SECOND: &str = "stand-in (1) 2/2";
    /// Both of them, as a play's `tributes`.
    const BODIES: [&str; 2] = [BODY, SECOND];

    fn board(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": [{ "def": BODY, "lane": 2 }, { "def": SECOND, "lane": 3 }],
                "library": filler(8),
                "mana": 4,
            },
            "p2": { "hand": [FILLER], "library": filler(8) },
        }));
        s.state_mut().players.p1.auto_end_turn = Some(false);
        s
    }

    fn cast_since(s: &Scenario, from: usize) -> Vec<(String, bool, i32)> {
        s.events()[from..]
            .iter()
            .filter_map(|event| match event {
                GameEvent::CardPlayed {
                    player: P1,
                    instance_id,
                    def_id,
                    cost_paid,
                    ..
                } if def_id != ID => Some((def_id.clone(), s.card(instance_id.as_str()).radiant, *cost_paid)),
                _ => None,
            })
            .collect()
    }

    fn discovered(s: &Scenario) -> Vec<String> {
        let pending = s.state().pending.clone().expect("the Discover");
        assert_eq!(pending.kind, PromptKind::Discover);
        assert_eq!(pending.player_id, P1);
        let options = s.prompt_modes();
        assert_eq!(options.len(), pending.options.len(), "a card option each");
        options
    }

    #[test]
    fn r3538005_base_casts_an_s_tier_card_or_past_version_never_itself() {
        crate::register_all();
        let run = || {
            let mut s = board(false, "xxfreshjack");
            let from = s.events().len();
            s.play(ID, json!({ "zone": 1, "tributes": BODIES }));
            assert!(s.state().pending.is_none(), "a random cast asks nothing (R452)");
            cast_since(&s, from)
        };
        let cast = run();
        assert_eq!(cast.len(), 1, "{cast:?}");
        let (def_id, radiant, paid) = &cast[0];
        assert_eq!(tier_of_record(def_id), Some(CardTier::S), "{def_id}");
        assert_ne!(past_card_of(def_id), ID, "{def_id}");
        assert!(!radiant);
        assert_eq!(*paid, 0, "free, and a play (R70)");
        assert_eq!(run(), cast);
    }

    #[test]
    fn r3538005_the_pool_holds_past_versions_of_s_tier_cards() {
        crate::register_all();
        let pool = query(&busted_anywhere());
        assert!(!pool.is_empty());
        assert!(
            pool.iter().all(|def| tier_of_record(&def.id) == Some(CardTier::S)),
            "every pick is S tier of record"
        );
        assert!(pool.iter().all(|def| past_card_of(&def.id) != ID));
        assert!(
            pool.iter().any(|def| split_past_id(&def.id).is_some()),
            "past versions play too"
        );
    }

    #[test]
    fn r3538005_radiant_discovers_one_and_casts_the_past_version_picked() {
        crate::register_all();
        // The first option is a past version under the found seed, so the cast proves a past
        // version plays when picked. Searched, not pinned, so a pool rotation still finds one.
        let (mut s, options) = (0..100)
            .find_map(|n| {
                let mut s = board(true, &format!("xxfreshjack-radiant-past-{n}"));
                s.play(ID, json!({ "zone": 1, "tributes": BODIES }));
                let options = discovered(&s);
                split_past_id(&options[0]).is_some().then_some((s, options))
            })
            .expect("a seed whose first option is a past version");
        assert_eq!(options.len(), DISCOVER_OPTIONS as usize);
        assert!(
            options.iter().all(|def_id| tier_of_record(def_id) == Some(CardTier::S)
                && past_card_of(def_id) != ID),
            "{options:?}"
        );
        assert!(
            split_past_id(&options[0]).is_some(),
            "the pick is a past version: {options:?}"
        );
        let me = s.unit(P1, 1).expect("xxFreshJack").id;
        let stats = s.stats(me.as_str());
        assert_eq!((stats.attack, stats.health), (12, 22));
        let from = s.events().len();
        s.answer(json!(discovered(&s)[0].clone()));
        while let Some(prompt) = s.state().pending.clone() {
            s.answer(json!(prompt.options[0].key));
        }
        let cast = cast_since(&s, from);
        assert_eq!(cast.first(), Some(&(options[0].clone(), true, 0)), "{cast:?}");
    }

    /// Issue #4440: Tribute 2 on both faces (R101): with no unit, or one, to pay it the play is refused, and the
    /// units paid die paying it. Its Cry casts a random BUSTED card from the build's own list, which a new
    /// version's list changes and which may move both cards on (v0.6.7's exiles the board and the
    /// graveyard), so the Tribute is read off the events: each paid unit's `destroyed` comes before
    /// anything the Cry casts.
    #[test]
    fn issue_4440_both_faces_need_tribute_2() {
        crate::register_all();
        for radiant in [false, true] {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": filler(8), "mana": 4 },
                "p2": { "hand": [FILLER], "library": filler(8) },
            }));
            s.expect_refused_with(|s| s.play(ID, json!({ "zone": 1 })), "Tribute");
            let mut s = board(radiant, "xxfreshjack-tribute");
            assert_eq!(tribute_cost_of(s.state(), s.card(ID)), TRIBUTE);
            s.expect_refused(|s| s.play(ID, json!({ "zone": 1, "tributes": [BODY] })));
            let bodies = [2, 3].map(|lane| s.unit(P1, lane).expect("a body").id);
            let from = s.events().len();
            s.play(ID, json!({ "zone": 1, "tributes": BODIES }));
            let events = &s.events()[from..];
            let cast = events
                .iter()
                .position(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if def_id != ID))
                .unwrap_or(events.len());
            for body in &bodies {
                let paid = events
                    .iter()
                    .position(|event| matches!(event, GameEvent::Destroyed { instance_id, .. } if instance_id == body));
                assert!(paid.is_some_and(|at| at < cast), "radiant {radiant}: the Tribute took {body} first");
            }
        }
    }

    /// R1841003, R2657002: since issue #4440 each of its owner's Floats lowers its Tribute by 1 while it waits
    /// in hand, never in the deck, on both faces; a lowered Tribute 1 is paid by one unit.
    #[test]
    fn r1841003_each_float_lowers_its_tribute_by_1_in_hand_never_in_the_deck() {
        crate::register_all();
        for radiant in [false, true] {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                    "field": [{ "def": BODY, "lane": 2 }],
                    "library": [FILLER, FILLER, FILLER, FILLER, { "def": ID, "radiant": radiant }],
                    "mana": 2,
                },
                "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER, FILLER] },
            }));
            let in_hand = s.hand(P1).into_iter().find(|card| card.def_id == ID).expect("in hand").id;
            let in_deck = s
                .pile(P1, "library")
                .into_iter()
                .find(|card| card.def_id == ID)
                .expect("in the deck")
                .id;
            let tribute = |s: &Scenario, id: &str| tribute_cost_of(s.state(), s.card(id));
            s.end_turn();
            assert_eq!(
                (tribute(&s, &in_hand), tribute(&s, &in_deck)),
                (TRIBUTE - 1, TRIBUTE),
                "radiant {radiant}"
            );
            next_turn_of(&mut s, P1);
            assert_eq!(tribute(&s, &in_hand), TRIBUTE - 1, "radiant {radiant}");
            // The Cry may move the paid body on (a cast BUSTED card), so the payment is read off the events.
            let body = s.unit(P1, 2).expect("the body").id;
            let from = s.events().len();
            s.play(&in_hand, json!({ "zone": 1, "tributes": [body.as_str()] }));
            assert!(
                s.events()[from..]
                    .iter()
                    .any(|event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == body)),
                "radiant {radiant}: one unit paid the lowered Tribute"
            );
        }
    }

    /// R767000: after v0.6.6h through v0.6.9n one unit paid it and a Float lowered nothing (issue #4440).
    #[test]
    fn r767000_the_script_kept_for_v0_6_9n_needs_tribute_1_and_no_float_lowers_it() {
        let kept = script_v0_6_9n();
        assert!(kept.base.hand_triggers.is_empty() && kept.radiant.hand_triggers.is_empty(), "no Float");
        crate::under_kept(ID, kept, || {
            let mut s = board(false, "xxfreshjack-fold-9g");
            assert_eq!(tribute_cost_of(s.state(), s.card(ID)), TRIBUTE_V0_6_9G);
            let body = s.unit(P1, 2).expect("the body").id;
            let from = s.events().len();
            s.play(ID, json!({ "zone": 1, "tributes": [BODY] }));
            assert!(
                s.events()[from..]
                    .iter()
                    .any(|event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == body)),
                "one unit paid it"
            );
        });
    }

    /// R767000: a game on v0.6.6h and before plays it with no Tribute, under the kept script.
    #[test]
    fn issue_4154_the_kept_v0_6_6h_script_asks_no_tribute() {
        crate::under_kept(ID, script_v0_6_6h(), || {
            let mut s = board(false, "xxfreshjack-fold-6e");
            s.play(ID, json!({ "zone": 1 }));
            s.expect_in_zone(ID, "field");
            assert!(s.unit(P1, 2).is_some(), "no unit paid for it");
        });
    }

    #[test]
    fn r3538005_a_fold_of_v0_5_13f_runs_the_kept_script_on_its_code_line_list() {
        crate::register_all();
        // R3631001: v0.5.13f is named v0.5.12af since the v0.5.13 retcon.
        let v0_5_13f = crate::shipped_catalog("v0.5.12af").expect("v0.5.12af is carried");
        jackioh_engine::catalog::with_shipped(Some(v0_5_13f), || {
            let mut s = board(false, "xxfreshjack-fold");
            let from = s.events().len();
            s.play(ID, json!({ "zone": 1 }));
            assert!(s.state().pending.is_none(), "a random cast asks nothing (R452)");
            let cast = cast_since(&s, from);
            assert_eq!(cast.len(), 1, "{cast:?}");
            let (def_id, radiant, paid) = &cast[0];
            assert!(is_busted(def_id) && def_id != ID, "{def_id}");
            assert!(!radiant);
            assert_eq!(*paid, 0, "free, and a play (R70)");
        });
    }
}
