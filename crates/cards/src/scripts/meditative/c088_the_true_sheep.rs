//! M #88 The True Sheep (SPEC §8.8 row 88). (4) Unit, Epic, 1/1 → 2/2.
//!
//! Base:    "Worth {worth|Tribute|Tributes}. Cry: Discover a card with Tribute to replace this with."
//! Radiant: "Worth {worth|Tribute|Tributes}. Cry: Discover a Radiant card with Tribute to replace
//!          this with."
//! Engine:  "Its Tribute worth (§6.3 Tribute, the Sheep Token's and C #82 Sheeople's static flag) is 5,
//!          counted toward a Tribute X only (R101). The Cry Discovers over the non-token cards of
//!          every set with a Tribute X play cost (R380; R1221's `tribute` catalog query, a script
//!          flag, so the pool keeps up with every new Tribute card), three different options shown to
//!          the chooser only (§6.3 Discover), Radiant on the Radiant face, the pick to your hand (MD-E12,
//!          R1222: the Sheep stays on the field). Tunes: worth 5 ↑ (Radiant 500; step 1)."
//!
//! The worth is the static flag `playChoices.tributeValueOf` reads through the declared `worth`
//! (R386), toward a play's Tribute X only. The Discover names the pool by query, never by ids, so no
//! `defId` pool skips the R1420 gate or lets tokens in.

use jackioh_engine::effects::{add_to_hand, discover_from_catalog};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-088";

/// The one resume step: the Discover's answer comes back here (§10.6).
const PICKED: &str = "picked";

/// R29: "Discover shows the top 3" — §6.3's Discover is 1 of 3 either way, said explicitly.
const OFFERED: i32 = 3;

/// The catalog's declared `worth`, read off the face that is up (R386).
fn worth() -> Param {
    match crate::card_def(ID)
        .params
        .as_ref()
        .and_then(|params| params.iter().find(|entry| entry.key == "worth"))
    {
        Some(param) => param.clone(),
        None => panic!("meditative-088 declares its worth (catalog params)"),
    }
}

fn sheep(radiant: bool) -> Script {
    let worth = if radiant { worth().radiant } else { worth().base };
    Script {
        static_flags: Some(StaticFlags {
            tribute_worth: Some(worth),
            ..StaticFlags::default()
        }),
        cry: Some(hook(move |_ctx| {
            // R1221: the pool is every non-token card with a Tribute cost, by query — never a defId
            // list, so the pool keeps up with every new Tribute card and the R1420 gate holds.
            let discover: DiscoverFromCatalogArgs = json_as(json!({
                "step": PICKED,
                "count": OFFERED,
                "query": { "tribute": true },
                "prompt": if radiant {
                    "Discover a Radiant card with Tribute"
                } else {
                    "Discover a card with Tribute"
                },
            }));
            vec![discover_from_catalog(discover)]
        })),
        resume: IndexMap::from([(
            PICKED,
            // A Discover's pick arrives as a `mode` selection carrying a catalog id (§10.6). The
            // pick goes to the hand and the Sheep stays (R1222).
            hook(move |ctx| {
                let Some(picked) = chosen_options(ctx).into_iter().next() else {
                    return vec![];
                };
                vec![add_to_hand(json_as(json!({ "defId": picked, "radiant": radiant })))]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: sheep(false),
        radiant: sheep(true),
    }
}

// M #88 The True Sheep — SPEC §8.8 row 88, BUILD M10 row M 88: "Cry (played or cast): Discovers one
// of three different non-token cards with a Tribute cost to your hand (R1221, R1222), shown to you
// only; the Sheep stays on the field; as a Tribute it pays a whole Tribute 2 or 3 alone, a set of
// one (R101); worth reads through `param()`; radiant 2/2, worth 500, the Discovered card Radiant".
#[cfg(test)]
mod tests {
    use super::{ID, OFFERED};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-016";
    const MAX: &str = "classic-080"; // BOOM! Big Max: Tribute 3

    fn holding(seed: &str, radiant: bool) -> Scenario {
        crate::scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": [FILLER, FILLER], "mana": 10 },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }))
    }

    use crate::js;

    fn open(state: &GameState) -> Value {
        match &state.pending {
            Some(pending) => js(pending),
            None => panic!("the Sheep opened no prompt"),
        }
    }

    /// A Discover's options are `mode` selections carrying catalog ids (§10.6).
    fn option_ids(pending: &Value) -> Vec<String> {
        pending["options"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter(|option| option["selection"]["pick"] == "mode")
            .map(|option| option["selection"]["option"].as_str().unwrap_or_default().to_string())
            .collect()
    }

    /// The registered script's static flags for one face.
    fn flags(face: &Script) -> StaticFlags {
        face.static_flags.clone().unwrap_or_default()
    }

    mod m88_the_true_sheep {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r1221_the_pool_is_the_tribute_cost_cards() {
                let _preview = preview_sets(&[SetName::Meditative]);
                let mut s = holding("sheep-pool", false);
                s.play(ID, json!({ "zone": 1 }));
                let options = option_ids(&open(s.state()));
                assert_eq!(options.len(), OFFERED as usize);
                // Three different non-token cards, every one carrying a Tribute cost.
                let mut sorted = options.clone();
                sorted.sort();
                sorted.dedup();
                assert_eq!(sorted.len(), options.len(), "three different options");
                for def_id in &options {
                    let def = crate::card_def(def_id);
                    assert!(!def.token, "{def_id}");
                    let entry = registered_entry(def_id).expect("a registered card");
                    assert!(
                        entry.base.static_flags.as_ref().is_some_and(|flag| flag.tribute.unwrap_or(0) > 0),
                        "{def_id} carries a Tribute cost",
                    );
                }
            }

            #[test]
            fn r1221_the_tribute_query_holds_the_five_shipped_tribute_cost_cards() {
                crate::register_all();
                // A pool that names no set draws from the sets that ship (R1420): Lava Golem, The
                // Rock, Nature Titan, Plague-Bringer Goliath and BOOM! Big Max. Turtinator's
                // Tribute is an Activate's cost, not a play cost, so it is not one of them.
                let pool: Vec<&str> = jackioh_engine::catalog::query(&CatalogQueryArgs {
                    tribute: Some(true),
                    ..CatalogQueryArgs::default()
                })
                .into_iter()
                .map(|def| def.id.as_str())
                .collect();
                assert_eq!(pool, ["core-055", "core-066", "classic-045", "classic-061", "classic-080"]);
                // `false` forbids them.
                let without = jackioh_engine::catalog::query(&CatalogQueryArgs {
                    tribute: Some(false),
                    ..CatalogQueryArgs::default()
                });
                assert!(without.iter().all(|def| !pool.contains(&def.id.as_str())));
                assert!(!without.is_empty());
            }

            #[test]
            fn r1222_discovers_three_different_to_hand_and_stays() {
                let _preview = preview_sets(&[SetName::Meditative]);
                let mut s = holding("sheep-stays", false);
                s.play(ID, json!({ "zone": 1 }));
                let picked = option_ids(&open(s.state())).into_iter().next().expect("three options");
                let hand = s.hand(P1).len();
                s.answer(json!(picked));
                // The pick reaches the hand, not Radiant, and the Sheep stands where it was played.
                let after: Vec<String> = s.hand(P1).iter().map(|card| card.def_id.clone()).collect();
                assert_eq!(after.len(), hand + 1);
                assert!(after.contains(&picked));
                let pick = s.hand(P1).into_iter().find(|card| card.def_id == picked).expect("the pick");
                assert!(!pick.radiant);
                s.expect_in_zone(ID, "field");
            }

            #[test]
            fn worth_5_pays_boom_big_maxs_tribute_3_alone() {
                let _preview = preview_sets(&[SetName::Meditative]);
                let mut s = crate::scenario(json!({
                    "seed": "sheep-pays",
                    "p1": { "field": [ID], "hand": [MAX, FILLER], "library": [FILLER, FILLER], "mana": 10 },
                    "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
                }));
                let sheep = s.card(ID).id.clone();
                let big = s.card(MAX).clone();
                // Worth 5 pays the whole Tribute 3 as a set of one (R101).
                let sets = legal_tribute_sets(s.state(), P1, &big);
                assert!(sets.contains(&vec![sheep.clone()]));
                s.play(MAX, json!({ "zone": 2, "tributes": [sheep] }));
                s.expect_in_zone(MAX, "field");
            }

            #[test]
            fn worth_reads_through_param() {
                let def = crate::card_def(ID);
                let param = def.params.as_ref().and_then(|params| params.iter().find(|entry| entry.key == "worth")).expect("worth");
                assert_eq!((param.base, param.radiant), (5, 500));
                let entry = registered_entry(ID).expect("registered");
                assert_eq!(flags(&entry.base).tribute_worth, Some(5));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn the_pick_is_radiant_and_the_worth_is_500() {
                let _preview = preview_sets(&[SetName::Meditative]);
                let mut s = holding("sheep-radiant", true);
                s.play(ID, json!({ "zone": 1 }));
                let picked = option_ids(&open(s.state())).into_iter().next().expect("three options");
                s.answer(json!(picked));
                let pick = s.hand(P1).into_iter().find(|card| card.def_id == picked).expect("the pick");
                assert!(pick.radiant);
                let entry = registered_entry(ID).expect("registered");
                assert_eq!(flags(&entry.radiant).tribute_worth, Some(500));
                let stats = s.stats(ID);
                assert_eq!((stats.attack, stats.health), (2, 2));
            }
        }
    }
}
