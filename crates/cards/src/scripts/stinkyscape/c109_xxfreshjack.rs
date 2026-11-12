//! Stinkyscape #109 xxFreshJack (SPEC §8.10 row 109). (4) Unit, Human, Acclaimed, Mythic, 6/11 → 12/22.
//!   Base:    "Cry: Cast a random BUSTED card from anywhere in the game's code."
//!   Radiant: "Cry: Discover a Radiant BUSTED card from anywhere in the game's code and cast it."
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

pub fn script() -> CardScripts {
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
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-005"; // Stockpile: a Spell

    fn board(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": filler(8), "mana": 4 },
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
        pending
            .options
            .iter()
            .map(|option| match &option.selection {
                Selection::Mode { option } => option.clone(),
                other => panic!("a card option, not {other:?}"),
            })
            .collect()
    }

    #[test]
    fn r3538005_base_casts_an_s_tier_card_or_past_version_never_itself() {
        crate::register_all();
        let run = || {
            let mut s = board(false, "xxfreshjack");
            let from = s.events().len();
            s.play(ID, json!({ "zone": 1 }));
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
                s.play(ID, json!({ "zone": 1 }));
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

    #[test]
    fn r3538005_a_fold_of_v0_5_13f_runs_the_kept_script_on_its_code_line_list() {
        crate::register_all();
        let v0_5_13f = crate::shipped_catalog("v0.5.13f").expect("v0.5.13f is carried");
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
