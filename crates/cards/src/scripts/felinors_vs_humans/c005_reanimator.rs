//! SPEC §8.14 FvH #5 Reanimator ((3) Unit, Human, Epic, 2/5 → 4/10), a card of the held Felinors vs Humans
//! set (R5235000). Built by S09 (#5259).
//!   Base:    "Cry: Summon a random Unit with a Death. Trigger its Death."
//!   Radiant: "Cry: Discover a Radiant Unit with a Death. Summon it and trigger its Death."
//!
//! Engine: the Cry (R1) draws from the non-token Units whose base script has a Death (`CatalogQueryArgs.hooks:
//! [Death]`, R5239002), never this card (R387). The base face's `summon_random` lands its pick per R64 with
//! no Cry; then the Unit this Cry summoned runs its Death once where it stands, without dying (`trigger_death`,
//! R4154205, FV2 #5). No open zone summons none, and no Death runs (R129). The Radiant face Discovers from
//! the same pool (three options, M #88 The True Sheep's "Discover a Radiant …"), and its answer summons the
//! pick Radiant and triggers its Radiant face's Death; with no open zone the Discover is still made and the
//! pick summons nothing (R5259001). Declares no numbers.

use jackioh_engine::effects::{chosen_options, discover_from_catalog, summon_random, trigger_death};
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-005";

/// The resume step the Discover's answer comes back to (§10.6).
const PICKED: &str = "picked";

/// R5239002: the Units whose base face prints a Death.
fn death_units() -> CatalogQueryArgs {
    let mut query: CatalogQueryArgs = json_as(json!({ "type": "Unit" }));
    query.hooks = Some(vec![HookFilter::Death]);
    query
}

/// R4154205: the first Unit this run summoned (the Cry's own summon comes before anything it sets off) runs
/// its Death once, read as the clause begins.
fn trigger_the_summoned_death() -> Effect {
    for_each_card(ForEachCardArgs {
        cards: Arc::new(|ctx: &mut EffectContext<'_>| {
            summoned_so_far(ctx).into_iter().take(1).collect()
        }),
        each: Arc::new(|instance_id: &str| {
            trigger_death(TriggerDeathArgs {
                instance_id: instance_id.to_string(),
            })
        }),
    })
}

fn base() -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            let mut random: SummonRandomArgs = json_as(json!({}));
            random.query = Some(death_units());
            vec![summon_random(random), trigger_the_summoned_death()]
        })),
        ..Script::default()
    }
}

fn radiant() -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            let mut discover: DiscoverFromCatalogArgs = json_as(json!({
                "step": PICKED,
                "prompt": "Discover a Radiant Unit with a Death",
            }));
            discover.query = Some(death_units());
            vec![discover_from_catalog(discover)]
        })),
        resume: IndexMap::from([(
            PICKED,
            // A Discover's pick arrives as a `mode` selection carrying a catalog id (§10.6).
            hook(|ctx| {
                let Some(picked) = chosen_options(ctx).into_iter().next() else {
                    return vec![];
                };
                vec![
                    summon(json_as(json!({ "defId": picked, "radiant": true }))),
                    trigger_the_summoned_death(),
                ]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: base(),
        radiant: radiant(),
    }
}

// FvH #5 Reanimator — SPEC §8.14 row 5 (R5239002, R4154205, R129, R387, R5259001).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "core-005"; // Stockpile: a card kept in hand, so no turn ends by itself
    const BODY: &str = "stand-in (1) 1/1"; // a Unit with no Death, to fill a row
    const LAR: &str = "classicplus-076"; // Brother Lar: "Death: Summon a Brother Ping."
    const PING: &str = "classicplus-076-1"; // Brother Ping

    fn board(seed: &str, radiant: bool, field: Value) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": field,
                "library": crate::filler(3),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(3) },
        }))
    }

    fn has_death(def_id: &str) -> bool {
        jackioh_engine::scripts::registered_entry(def_id).is_some_and(|entry| entry.base.death.is_some())
    }

    fn pings(s: &Scenario) -> Vec<CardInstance> {
        (1..=UNIT_ZONES).filter_map(|lane| s.unit(P1, lane)).filter(|unit| unit.def_id == PING).collect()
    }

    #[test]
    fn declares_no_numbers() {
        assert!(crate::card_def(ID).params.is_none());
    }

    mod base {
        use super::*;

        /// Played into lane 1 of an empty row, so the summon lands in lane 2.
        fn played(seed: &str) -> Scenario {
            let mut s = board(seed, false, json!([]));
            s.play(ID, json!({ "zone": 1 }));
            s
        }

        #[test]
        fn r5239002_it_summons_a_non_token_unit_whose_base_face_has_a_death() {
            let s = played("reanimator-any");
            let summoned = s.unit_at(P1, 2);
            let def = crate::card_def(&summoned.def_id);
            assert_eq!(def.type_, CardType::Unit);
            assert!(!def.token, "no token");
            assert!(has_death(&summoned.def_id), "{} has a Death", summoned.def_id);
            assert!(!summoned.radiant);
            assert_eq!(s.count("destroyed"), 0, "it does not die");
        }

        #[test]
        fn r4154205_the_summoned_units_death_runs_once_and_it_stays() {
            let seed = seed_where("reanimator-lar", |seed| {
                played(seed).unit(P1, 2).is_some_and(|unit| unit.def_id == LAR)
            });
            let s = played(&seed);
            assert_eq!(s.unit_at(P1, 2).def_id, LAR, "Lar is summoned and stays");
            let made = pings(&s);
            assert_eq!(made.len(), 1, "its Death ran once");
            assert!(!made[0].radiant);
        }

        #[test]
        fn r129_with_no_open_zone_nothing_is_summoned_and_no_death_runs() {
            let mut s = board("reanimator-full", false, json!([BODY, BODY, BODY, BODY]));
            s.play(ID, json!({}));
            assert_eq!(s.unit_at(P1, UNIT_ZONES).def_id, ID, "it takes the last open zone");
            assert_eq!(s.count("summoned"), 1, "only its own landing");
            assert!(s.state().pending.is_none());
        }
    }

    mod radiant {
        use super::*;

        /// Played into lane 1 of an empty row; the Discover is open.
        fn discovering(seed: &str) -> Scenario {
            let mut s = board(seed, true, json!([]));
            s.play(ID, json!({ "zone": 1 }));
            s
        }

        #[test]
        fn r5239002_it_discovers_from_three_non_token_units_with_a_death() {
            let s = discovering("reanimator-discover");
            let pending = must(s.state().pending.clone(), "the Discover");
            assert_eq!(pending.kind, PromptKind::Discover);
            assert_eq!(pending.player_id, P1);
            let offered = s.prompt_modes();
            assert_eq!(offered.len(), DISCOVER_OPTIONS as usize);
            for id in nonempty(offered, "the options") {
                assert_ne!(id, ID, "R387: never this card");
                assert!(!crate::card_def(&id).token, "{id}: no token");
                assert!(has_death(&id), "{id} has a Death");
            }
        }

        #[test]
        fn r4154205_the_pick_is_summoned_radiant_and_its_radiant_death_runs() {
            let seed = seed_where("reanimator-radiant-lar", |seed| {
                discovering(seed).prompt_modes().iter().any(|id| id == LAR)
            });
            let mut s = discovering(&seed);
            s.answer(pick::mode(LAR));
            let lar = s.unit_at(P1, 2);
            assert_eq!(lar.def_id, LAR);
            assert!(lar.radiant, "summoned Radiant");
            let made = pings(&s);
            assert_eq!(made.len(), 1, "its Death ran once");
            assert!(made[0].radiant, "its Radiant face's Death");
        }

        #[test]
        fn r5259001_with_no_open_zone_it_still_discovers_and_the_pick_summons_nothing() {
            let mut s = board("reanimator-radiant-full", true, json!([BODY, BODY, BODY, BODY]));
            s.play(ID, json!({}));
            let offered = s.prompt_modes();
            let first = must(offered.first().cloned(), "an option");
            s.answer(pick::mode(&first));
            assert_eq!(s.last_count("summoned"), 0, "no zone, no summon");
            assert!(s.state().pending.is_none());
        }
    }
}
