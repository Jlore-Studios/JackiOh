//! SPEC §8.14 FvH #111 Vicious Fledgeling ((1) Unit, Swarm, Rare, 3/3 → 6/6), a card of the Felinors vs
//! Humans set (R5235000).
//!   Base:    "Evolves after this attacks the enemy hero. / Endless Evolve: Discover a combat keyword to
//!            gain."
//!   Radiant: "Evolves after this attacks the enemy hero. / Endless Evolve: Discover a combat keyword to
//!            gain, then another."
//!
//! "Evolves after this attacks the enemy hero" (R3536001's card evolving itself): an `after_attack` hook,
//! run for a Fledgeling that survived the combat (R2285001), whose target was the enemy hero returns
//! `evolve` on itself, declared and forced attacks alike (R5261003). `StaticFlags.endless_evolve`
//! (R5236030): it evolves again every time, each one evolution with the multiplier (R3536003). The Evolve
//! text: `discover_keyword` of the combat keywords of R21's pool it lacks (R5239004), three offered,
//! fewer with fewer left, none with none; the pick is granted to it (`grant_keyword`), lost as it leaves
//! the field (R78). The Radiant face opens a second Discover once the first pick is in, which therefore
//! never offers it again. Declares no numbers. Built by S11 (#5261).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-111";

/// The resume steps: the first Discover's answer, and the Radiant face's second (§10.6).
const FIRST: &str = "first-keyword";
const SECOND: &str = "second-keyword";

/// A Discover of the combat keywords this lacks (R21's pool, R5239004).
fn discover(step: &str) -> Effect {
    discover_keyword(DiscoverKeywordArgs {
        step: step.to_string(),
        lacking_on: Some(TargetSpec::SelfCard),
        ..DiscoverKeywordArgs::default()
    })
}

/// The keyword picked, gained.
fn gain_pick(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    match discovered_keyword(ctx) {
        Some(keyword) => vec![grant_keyword(GrantKeywordArgs {
            target: TargetSpec::SelfCard,
            keyword,
        })],
        None => vec![],
    }
}

/// R5261003: after an attack on the enemy hero it survived, it evolves.
fn evolves_after_hitting_the_hero(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(combat) = after_attack_of(ctx) else {
        return vec![];
    };
    let enemy_hero = format!("hero-{}", opponent_of(ctx.controller).as_str());
    if combat.target_id != enemy_hero {
        return vec![];
    }
    vec![evolve(json_as(json!({ "target": { "of": "self" } })))]
}

fn fledgeling(twice: bool) -> Script {
    let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
    if twice {
        resume.insert(
            FIRST,
            hook(|ctx| {
                let mut effects = gain_pick(ctx);
                effects.push(discover(SECOND));
                effects
            }),
        );
        resume.insert(SECOND, hook(gain_pick));
    } else {
        resume.insert(FIRST, hook(gain_pick));
    }
    Script {
        after_attack: Some(hook(evolves_after_hitting_the_hero)),
        evolve: Some(hook(|_ctx| vec![discover(FIRST)])),
        resume,
        static_flags: Some(StaticFlags {
            endless_evolve: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: fledgeling(false),
        radiant: fledgeling(true),
    }
}

// FvH #111 Vicious Fledgeling — SPEC §8.14 row 111 (R5261003, R5236030, R5239004).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself
    /// An enemy Unit that strikes back for nothing.
    const SPONGE: &str = "stand-in (1) 0/30";

    fn board(radiant: bool, theirs: Value) -> Scenario {
        scenario(json!({
            "seed": "vicious-fledgeling",
            "p1": {
                "field": [{ "def": ID, "lane": 1, "radiant": radiant }],
                "hand": [FILLER],
                "library": filler(6),
            },
            "p2": { "hand": [FILLER], "field": theirs, "library": filler(6) },
        }))
    }

    fn has(s: &Scenario, card: &CardInstance, name: &str) -> bool {
        s.stats(card).keywords.iter().any(|keyword| keyword.kind().as_str() == name)
    }

    /// The open prompt's keyword options, after checking it is p1's keyword Discover.
    fn keyword_offer(s: &Scenario) -> Vec<String> {
        let pending = must(s.state().pending.clone(), "an open Discover");
        assert_eq!(pending.kind, PromptKind::Discover);
        assert_eq!(pending.player_id, P1);
        s.prompt_modes()
    }

    /// Answers the open Discover with its first keyword; that keyword.
    fn pick_first(s: &mut Scenario) -> String {
        let modes = keyword_offer(s);
        let first = must(modes.first().cloned(), "a keyword offered");
        s.answer(pick::mode(&first));
        first
    }

    /// The offer's keywords are R21's pool's, none the Fledgeling has.
    fn offers_combat_keywords_it_lacks(s: &Scenario, fledgeling: &CardInstance) {
        let pool: Vec<String> = jackioh_engine::effects::random_pool_keywords()
            .iter()
            .map(|keyword| keyword.kind().as_str().to_string())
            .collect();
        let modes = keyword_offer(s);
        assert_eq!(modes.len(), DISCOVER_OPTIONS as usize);
        for mode in nonempty(modes, "the offered keywords") {
            assert!(pool.contains(&mode), "{mode} is a combat keyword (R21)");
            assert!(!has(s, fledgeling, &mode), "{mode} is one it lacks");
        }
    }

    mod base {
        use super::*;

        #[test]
        fn declares_no_tuned_numbers() {
            assert!(crate::card_def(ID).params.is_none());
        }

        #[test]
        fn r5261003_after_it_attacks_the_enemy_hero_it_discovers_a_combat_keyword_and_gains_it() {
            let mut s = board(false, json!([]));
            let fledgeling = s.unit_at(P1, 1);
            s.attack(&fledgeling, "hero");
            offers_combat_keywords_it_lacks(&s, &fledgeling);
            let picked = pick_first(&mut s);
            assert!(has(&s, &fledgeling, &picked), "it gained {picked}");
            assert!(s.state().pending.is_none(), "one Discover on the base face");
        }

        #[test]
        fn r5261003_attacking_a_unit_evolves_nothing() {
            let mut s = board(false, json!([SPONGE]));
            let fledgeling = s.unit_at(P1, 1);
            let sponge = s.unit_at(P2, 1);
            s.attack(&fledgeling, &sponge);
            assert!(s.state().pending.is_none(), "no Discover");
            assert_eq!(s.count(GameEventType::Evolved), 0);
        }

        #[test]
        fn r5236030_endless_evolve_it_evolves_again_after_each_attack_on_the_hero() {
            let mut s = board(false, json!([]));
            let fledgeling = s.unit_at(P1, 1);
            s.attack(&fledgeling, "hero");
            let first = pick_first(&mut s);
            s.end_turn();
            s.end_turn();
            s.attack(&fledgeling, "hero");
            assert_eq!(s.count(GameEventType::Evolved), 2, "it evolved a second time");
            let modes = keyword_offer(&s);
            assert!(!modes.contains(&first), "the keyword it gained is not offered again");
            let second = pick_first(&mut s);
            assert!(has(&s, &fledgeling, &first));
            assert!(has(&s, &fledgeling, &second));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5261003_after_it_attacks_the_enemy_hero_it_discovers_a_combat_keyword_then_another() {
            let mut s = board(true, json!([]));
            let fledgeling = s.unit_at(P1, 1);
            s.attack(&fledgeling, "hero");
            offers_combat_keywords_it_lacks(&s, &fledgeling);
            let first = pick_first(&mut s);
            assert!(has(&s, &fledgeling, &first));
            let modes = keyword_offer(&s);
            assert!(!modes.contains(&first), "the second Discover leaves out the first pick");
            let second = pick_first(&mut s);
            assert!(has(&s, &fledgeling, &first));
            assert!(has(&s, &fledgeling, &second), "it gained both");
            assert!(s.state().pending.is_none(), "two Discovers in all");
        }

        #[test]
        fn r5261003_attacking_a_unit_evolves_nothing() {
            let mut s = board(true, json!([SPONGE]));
            let fledgeling = s.unit_at(P1, 1);
            let sponge = s.unit_at(P2, 1);
            s.attack(&fledgeling, &sponge);
            assert!(s.state().pending.is_none(), "no Discover");
        }
    }
}
