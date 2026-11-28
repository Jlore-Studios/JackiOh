//! FvH #112 Jilliax Deluxe (SPEC §8.14 row 112, FV2 #112; §6.3 Discover and Fuse, R77, R348, R352, R387,
//! R469, R5239005, R5262001, R5262002): (X) Unit, Mythic, 0/0 → 0/0, of the held Felinors vs Humans set.
//!   Base:    "Cry: Discover one of the four dearest (X) Cost or less Modules, then another. Fuse both into
//!            this."
//!   Radiant: "Cry: Discover one of the four dearest Radiant (X) Cost or less Modules, then another. Fuse
//!            both into this."
//!
//! R5262001: two chained Discovers (C+ #30 Felinor Fuser's shape, R352) over the thirteen module tokens
//! #112.1 to #112.13 printed at X or less, `order: dearest` (R5239005): the four dearest, a tie at the cut
//! drawn by the match rng, never a module already picked or already fused in. X is the X paid as the Cry
//! runs, carried in the prompt's data, so the second offer reads the same X. Then one `fuse_cards` of both
//! into this (R77), Radiant modules on the Radiant face (R469), so a Radiant Viral Module's Immutable lands
//! only once both are in. A second offer with nothing left fuses the one pick. The 0/0 body stands while
//! the Discovers are open: no state check runs while a prompt is open on a play. After the fusion, from
//! the picks alone: Starpiece's Starship Piece is granted to the fused card (which R1852001 keeps), and Twin
//! summons copies of the whole fused card, one (Radiant two), no Cry (R1). R5262002: the fused card costs
//! R77's min(X + the modules' costs, 4) and is no X card, so a Radiant Recursion Module's Flashback replay
//! runs this Cry with X 0.

use jackioh_engine::effects::{chosen_options, discover_from_catalog, fuse_cards};
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-112";

/// FV2 #112: the thirteen modules, #112.1 to #112.13, the pool every offer draws from.
const MODULES: [&str; 13] = [
    "felinorsvshumans-112-1",
    "felinorsvshumans-112-2",
    "felinorsvshumans-112-3",
    "felinorsvshumans-112-4",
    "felinorsvshumans-112-5",
    "felinorsvshumans-112-6",
    "felinorsvshumans-112-7",
    "felinorsvshumans-112-8",
    "felinorsvshumans-112-9",
    "felinorsvshumans-112-10",
    "felinorsvshumans-112-11",
    "felinorsvshumans-112-12",
    "felinorsvshumans-112-13",
];
/// #112.4 Starpiece Module, whose Starship Piece the post-step grants.
const STARPIECE: &str = "felinorsvshumans-112-4";
/// #112.9 Twin Module, whose copies the post-step summons.
const TWIN: &str = "felinorsvshumans-112-9";

/// "one of the four dearest".
const OFFER: i32 = 4;
/// R352: "Discover one …, then another".
const PICKS: usize = 2;
/// Twin's "a copy" (base) and "two copies" (Radiant).
const TWIN_COPIES: usize = 1;
const RADIANT_TWIN_COPIES: usize = 2;

/// The resume step each Discover's answer re-enters (§10.6).
const PICKED: &str = "picked";
/// The data keys the picks so far, the X paid and the modules already fused in travel under.
const PICKS_KEY: &str = "picks";
const X_KEY: &str = "x";
const FUSED_KEY: &str = "fused";

const PROMPT: &str = "Discover one of the four dearest Modules";
const RADIANT_PROMPT: &str = "Discover one of the four dearest Radiant Modules";

/// R5262001: every module the card `def_id` names has fused into it, a fused ingredient read through.
fn fused_modules(state: &GameState, def_id: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut left: Vec<String> = vec![def_id.to_string()];
    while let Some(next) = left.pop() {
        match fused_id_parts(Some(state), &next) {
            Some(parts) => left.extend(parts),
            None => {
                if MODULES.contains(&next.as_str()) && !found.contains(&next) {
                    found.push(next);
                }
            }
        }
    }
    found
}

/// R5262001: the modules printed at `x` or less, tokens all, never one of `excluded`.
fn module_query(x: i32, excluded: &[String]) -> Value {
    let mut query = json!({ "defId": MODULES, "withTokens": true, "costRange": { "max": x } });
    if !excluded.is_empty() {
        query["excludeDefId"] = json!(excluded);
    }
    query
}

/// `picks` and `fused` together: what no offer of this Cry may hold.
fn excluded_by(picks: &[String], fused: &[String]) -> Vec<String> {
    let mut excluded = fused.to_vec();
    excluded.extend(picks.iter().cloned());
    excluded
}

/// R5262001, R5239005: one Discover of the four dearest modules `x` affords, the Cry's state in its data.
fn discover(radiant: bool, x: i32, picks: &[String], fused: &[String]) -> Effect {
    let mut args = json!({
        "step": PICKED,
        "query": module_query(x, &excluded_by(picks, fused)),
        "count": OFFER,
        "order": "dearest",
        "prompt": if radiant { RADIANT_PROMPT } else { PROMPT },
        "data": {},
    });
    args["data"][PICKS_KEY] = json!(picks);
    args["data"][X_KEY] = json!(x);
    args["data"][FUSED_KEY] = json!(fused);
    discover_from_catalog(json_as(args))
}

/// A list of ids a paused Cry carries in its prompt's data.
fn ids_at(ctx: &EffectContext<'_>, key: &str) -> Vec<String> {
    match ctx.data.get(key).and_then(Value::as_array) {
        Some(stored) => stored.iter().filter_map(|id| id.as_str().map(str::to_string)).collect(),
        None => Vec::new(),
    }
}

/// R77, R5262001: one fusion of the picks into this, then the post-steps the picks alone decide.
fn fuse_picks(ctx: &EffectContext<'_>, radiant: bool, picks: &[String]) -> Vec<Effect> {
    // The unit fused onto is this one, while it still stands on the field (R77).
    let Some(me) = ctx.self_.as_ref() else {
        return vec![];
    };
    let mut args = json!({ "defIds": picks, "targetInstanceId": me.id });
    if radiant {
        args["radiantIngredients"] = json!(true);
    }
    let mut effects = vec![fuse_cards(json_as(args))];
    // R1852001: a fusion that keeps this drops the module's printed piece; the grant is this card's own.
    if picks.iter().any(|pick| pick == STARPIECE) {
        effects.push(grant_keyword(json_as(json!({
            "target": { "of": "self" },
            "keyword": Keyword::StarshipPiece,
        }))));
    }
    // R5262001: "the fused card" is the whole of this once both are in; the copies run no Cry (R1).
    if picks.iter().any(|pick| pick == TWIN) {
        let copies = if radiant { RADIANT_TWIN_COPIES } else { TWIN_COPIES };
        for _ in 0..copies {
            effects.push(summon_copy(json_as(json!({ "of": { "of": "self" } }))));
        }
    }
    effects
}

fn jilliax(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            // R5262002: the X this play paid; a fused card is no X card, so its replay's X is 0.
            let x = ctx.x;
            let own = ctx.self_.as_ref().map(|me| me.def_id.clone());
            let fused = own.map(|def_id| fused_modules(ctx.state, &def_id)).unwrap_or_default();
            vec![discover(radiant, x, &[], &fused)]
        })),
        resume: IndexMap::from([(
            PICKED,
            hook(move |ctx| {
                // A Discover is answered with one of its options (§10.6); an answer naming none adds nothing.
                let Some(picked) = chosen_options(ctx).into_iter().next() else {
                    return vec![];
                };
                let mut picks = ids_at(ctx, PICKS_KEY);
                picks.push(picked);
                let fused = ids_at(ctx, FUSED_KEY);
                let x = ctx.data.get(X_KEY).and_then(Value::as_i64).map_or(0, |paid| paid as i32);
                if picks.len() < PICKS {
                    let left = query(&json_as(module_query(x, &excluded_by(&picks, &fused))));
                    if !left.is_empty() {
                        return vec![discover(radiant, x, &picks, &fused)];
                    }
                }
                fuse_picks(ctx, radiant, &picks)
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: jilliax(false),
        radiant: jilliax(true),
    }
}

// FvH #112 Jilliax Deluxe — SPEC §8.14 row 112, R5239005, R5262001, R5262002.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{card_def, filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const RECURSION: &str = "felinorsvshumans-112-1";
    const DURABILITY: &str = "felinorsvshumans-112-2";
    const HAYWIRE: &str = "felinorsvshumans-112-3";
    const HEAVY: &str = "felinorsvshumans-112-7";
    const UTILITY: &str = "felinorsvshumans-112-8";
    const VIRAL: &str = "felinorsvshumans-112-10";
    const PERFECT: &str = "felinorsvshumans-112-11";
    const DEBUFF: &str = "felinorsvshumans-112-12";
    const INDESTRUCTIBLE: &str = "felinorsvshumans-112-13";

    const FILLER: &str = "core-005"; // Stockpile: a card in hand, so no turn ends by itself
    const HIT_JOB: &str = "core-016"; // Hit Job: destroy a Unit
    const MANA: i32 = 10;

    /// p1 plays Jilliax on `radiant`'s face into lane 1 for X = `x`.
    fn played(seed: &str, radiant: bool, x: i32) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, HIT_JOB, FILLER],
                "library": filler(6),
                "mana": MANA,
            },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s.play(ID, json!({ "zone": 1, "x": x }));
        s
    }

    /// `played`, then both Discovers answered with `first` and `second`.
    fn fused_with(seed: &str, radiant: bool, x: i32, first: &str, second: &str) -> Scenario {
        let mut s = played(seed, radiant, x);
        s.answer(json!(first));
        s.answer(json!(second));
        assert!(s.state().pending.is_none(), "both picks made");
        s
    }

    fn sorted(mut ids: Vec<String>) -> Vec<String> {
        ids.sort();
        ids
    }

    fn set_of(ids: &[&str]) -> Vec<String> {
        sorted(ids.iter().map(|id| id.to_string()).collect())
    }

    /// The modules fused into p1's lane-`lane` Unit.
    fn modules_at(s: &Scenario, lane: i32) -> Vec<String> {
        sorted(fused_modules(s.state(), &s.unit_at(P1, lane).def_id))
    }

    /// p1's lane-1 card destroyed by Hit Job, then played again from the graveyard by its Flashback.
    fn flashed_back(s: &mut Scenario) {
        let fused = s.unit_at(P1, 1);
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": fused.id }] }));
        assert!(s.unit(P1, 1).is_none(), "Hit Job destroyed it");
        let in_graveyard = must(
            s.pile(P1, "graveyard").into_iter().find(|card| card.def_id == fused.def_id),
            "the fused card in the graveyard",
        );
        s.state_mut().players.p1.mana.current = MANA;
        let play = must(
            s.plays_of(in_graveyard.id.as_str()).into_iter().next(),
            "a Flashback play of the fused card",
        );
        s.act(play, P1, "the Flashback play");
    }

    #[test]
    fn its_card_declares_no_numbers() {
        crate::register_all();
        assert!(card_def(ID).params.is_none());
    }

    mod base {
        use super::*;

        #[test]
        fn r5262001_r5239005_x_4_offers_the_four_dearest_dearest_first() {
            let s = played("jilliax-x4", false, 4);
            let offer = s.prompt_modes();
            assert_eq!(offer.len(), 4);
            assert_eq!(
                sorted(offer[..3].to_vec()),
                set_of(&[TWIN, VIRAL, PERFECT]),
                "the three (4) modules first"
            );
            assert!(offer[3] == HEAVY || offer[3] == UTILITY, "{offer:?}");
        }

        #[test]
        fn r5239005_a_tie_at_the_cut_is_drawn_at_random() {
            let mut seen: Vec<String> = Vec::new();
            for n in 0..20 {
                let s = played(&format!("jilliax-tie-{n}"), false, 4);
                seen.push(s.prompt_modes()[3].clone());
            }
            assert!(seen.iter().any(|id| id == HEAVY) && seen.iter().any(|id| id == UTILITY), "{seen:?}");
        }

        #[test]
        fn r5262001_the_second_offer_never_repeats_the_first_pick() {
            let mut s = played("jilliax-second", false, 4);
            s.answer(json!(VIRAL));
            assert_eq!(sorted(s.prompt_modes()), set_of(&[TWIN, PERFECT, HEAVY, UTILITY]));
        }

        #[test]
        fn r5262001_x_1_offers_the_four_cheapest_modules() {
            let s = played("jilliax-x1", false, 1);
            assert_eq!(sorted(s.prompt_modes()), set_of(&[RECURSION, DURABILITY, HAYWIRE, STARPIECE]));
        }

        #[test]
        fn r5262001_x_6_offers_indestructible_then_debuff_first() {
            let s = played("jilliax-x6", false, 6);
            let offer = s.prompt_modes();
            assert_eq!(&offer[..2], &[INDESTRUCTIBLE.to_string(), DEBUFF.to_string()]);
        }

        #[test]
        fn r5262001_the_0_0_body_stands_while_its_discovers_are_open() {
            let mut s = played("jilliax-body", false, 3);
            let jilliax = must(s.unit(P1, 1), "Jilliax on the field");
            s.expect_stats(jilliax.id.as_str(), json!({ "attack": 0, "health": 0 }));
            s.answer(json!(HEAVY));
            assert!(s.unit(P1, 1).is_some(), "it stands between the Discovers");
            s.answer(json!(UTILITY));
            let fused = s.unit_at(P1, 1);
            assert_eq!(fused.id, jilliax.id, "R77: the same instance");
            // 0/0 + Heavy 5/5 + Utility 3/1.
            s.expect_stats(fused.id.as_str(), json!({ "attack": 8, "health": 6 }));
        }

        #[test]
        fn r77_r5262001_both_picks_fuse_into_it_in_one_fusion() {
            let s = fused_with("jilliax-fuse", false, 3, HEAVY, UTILITY);
            let fused = s.unit_at(P1, 1);
            let parts = sorted(must(fused_id_parts(Some(s.state()), &fused.def_id), "a fused id"));
            assert_eq!(parts, set_of(&[ID, HEAVY, UTILITY]));
            let view = s.stats(fused.id.as_str());
            assert!(has_keyword(&view.keywords, KeywordKind::Pierce));
            assert!(has_keyword(&view.keywords, KeywordKind::Cleave));
            assert!(!fused.radiant);
        }

        #[test]
        fn r5262001_twin_summons_one_copy_of_the_whole_fused_card() {
            let s = fused_with("jilliax-twin", false, 4, TWIN, HEAVY);
            let fused = s.unit_at(P1, 1);
            let copy = must(s.unit(P1, 2), "the Twin's copy");
            assert_eq!(copy.def_id, fused.def_id, "a copy of the card with both modules in");
            assert!(s.unit(P1, 3).is_none(), "one copy");
            assert_eq!(modules_at(&s, 2), set_of(&[TWIN, HEAVY]));
        }

        #[test]
        fn r1852001_r5262001_starpiece_is_granted_once_fused() {
            let s = fused_with("jilliax-starpiece", false, 1, STARPIECE, HAYWIRE);
            let fused = s.unit_at(P1, 1);
            assert!(has_keyword(&s.stats(fused.id.as_str()).keywords, KeywordKind::StarshipPiece));
            assert_eq!(modules_at(&s, 1), set_of(&[STARPIECE, HAYWIRE]));
        }

        #[test]
        fn r5239006_r5262002_a_base_recursion_replay_runs_no_cry() {
            let mut s = fused_with("jilliax-quiet", false, 1, RECURSION, DURABILITY);
            flashed_back(&mut s);
            assert!(s.state().pending.is_none(), "no Discover opens");
            let back = must(s.unit(P1, 1).or_else(|| s.unit(P1, 2)), "the replayed card on the field");
            assert_eq!(modules_at_card(&s, &back), set_of(&[RECURSION, DURABILITY]));
        }
    }

    /// The modules fused into `card`.
    fn modules_at_card(s: &Scenario, card: &CardInstance) -> Vec<String> {
        sorted(fused_modules(s.state(), &card.def_id))
    }

    mod radiant {
        use super::*;

        #[test]
        fn r469_r5262001_the_modules_go_in_on_their_radiant_faces() {
            let s = fused_with("jilliax-radiant", true, 3, HEAVY, UTILITY);
            let fused = s.unit_at(P1, 1);
            assert!(fused.radiant);
            // 0/0 + Radiant Heavy 15/15 + Radiant Utility 6/2, Deft added.
            let view = s.stats(fused.id.as_str());
            assert_eq!((view.attack, view.health), (21, 17));
            assert!(has_keyword(&view.keywords, KeywordKind::Deft));
        }

        #[test]
        fn r23_r5262001_a_radiant_viral_picked_first_still_takes_the_second() {
            let s = fused_with("jilliax-viral", true, 4, VIRAL, PERFECT);
            let fused = s.unit_at(P1, 1);
            assert_eq!(modules_at(&s, 1), set_of(&[VIRAL, PERFECT]));
            let view = s.stats(fused.id.as_str());
            assert!(has_keyword(&view.keywords, KeywordKind::Immutable));
            assert!(has_keyword(&view.keywords, KeywordKind::Reborn));
        }

        #[test]
        fn r5262001_twin_summons_two_copies() {
            let s = fused_with("jilliax-radiant-twin", true, 4, TWIN, HEAVY);
            let fused = s.unit_at(P1, 1);
            for lane in [2, 3] {
                let copy = must(s.unit(P1, lane), "a Twin copy");
                assert_eq!(copy.def_id, fused.def_id, "lane {lane}");
            }
            assert!(s.unit(P1, 4).is_none(), "two copies");
        }

        #[test]
        fn r5262002_a_radiant_recursion_replay_runs_its_cry_with_x_0() {
            let mut s = played("jilliax-loud", true, 1);
            s.answer(json!(RECURSION));
            s.answer(json!(HAYWIRE));
            flashed_back(&mut s);
            // X 0 affords the (0) modules; Recursion is fused in already, so Durability is all that is left.
            assert_eq!(s.prompt_modes(), vec![DURABILITY.to_string()]);
            s.answer(json!(DURABILITY));
            assert!(s.state().pending.is_none(), "nothing is left for a second offer");
            let back = must(s.unit(P1, 1).or_else(|| s.unit(P1, 2)), "the replayed card on the field");
            assert_eq!(modules_at_card(&s, &back), set_of(&[RECURSION, HAYWIRE, DURABILITY]));
        }
    }
}
