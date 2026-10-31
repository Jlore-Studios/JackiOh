//! Stinkyscape #109 xxFreshJack (SPEC §8.10 row 109). (4) Unit, Human, Acclaimed, Mythic, 6/11 → 12/22.
//!   Base:    "Cry: Cast a random BUSTED card from anywhere in the game's code."
//!   Radiant: "Cry: Discover a Radiant BUSTED card from anywhere in the game's code and cast it."
//!
//! Cry (R1): the pool is `{ busted: true, everySet: true }` (R1846000, R1846001): every BUSTED catalog card
//! of the sets that ship and the single-player sets, never this card (R387). The base face casts one at
//! random (`cast_random`, R452): free, a play for every rule (R70), every choice random. The Radiant face
//! Discovers one of three (§6.3) and casts it Radiant with its caster's choices (`cast_new`, E12, R453).

use jackioh_engine::effects::{CastRandomArgs, CastRandomQuery, cast_random};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-109";

/// The resume step the Radiant Discover's answer comes back to (§10.6).
const PICKED: &str = "picked";

/// R1846000, R1846001: "a BUSTED card that exists anywhere in the game's code".
fn busted_anywhere() -> CatalogQueryArgs {
    json_as(json!({ "busted": true, "everySet": true }))
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

// Stinkyscape #109 xxFreshJack — SPEC §8.10 row 109, BUILD M12 row S 109 (R1846001, R70).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::catalog::is_busted;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-005"; // (1) Spell.

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

    #[test]
    fn r1846001_its_cry_casts_one_random_busted_card_free_never_itself_the_same_seed_the_same() {
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
        assert!(is_busted(def_id) && def_id != ID, "{def_id}");
        assert!(!radiant);
        assert_eq!(*paid, 0, "free, and a play (R70)");
        assert_eq!(run(), cast);
    }

    #[test]
    fn r1846001_radiant_discovers_three_radiant_busted_cards_and_casts_the_pick() {
        crate::register_all();
        let mut s = board(true, "xxfreshjack-radiant");
        s.play(ID, json!({ "zone": 1 }));
        let pending = s.state().pending.clone().expect("the Discover");
        assert_eq!(pending.kind, PromptKind::Discover);
        assert_eq!(pending.player_id, P1);
        let options: Vec<String> = pending
            .options
            .iter()
            .map(|option| match &option.selection {
                Selection::Mode { option } => option.clone(),
                other => panic!("a card option, not {other:?}"),
            })
            .collect();
        assert_eq!(options.len(), DISCOVER_OPTIONS as usize);
        assert!(options.iter().all(|def_id| is_busted(def_id) && def_id != ID));
        let me = s.unit(P1, 1).expect("xxFreshJack").id;
        let stats = s.stats(me.as_str());
        assert_eq!((stats.attack, stats.health), (12, 22));
        let from = s.events().len();
        s.answer(json!(pending.options[0].key));
        while let Some(prompt) = s.state().pending.clone() {
            s.answer(json!(prompt.options[0].key));
        }
        let cast = cast_since(&s, from);
        assert_eq!(cast.first(), Some(&(options[0].clone(), true, 0)), "{cast:?}");
    }
}
