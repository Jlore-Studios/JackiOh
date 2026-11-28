//! SPEC §8.14 FvH #81 Old-Timer Catboy ((2) Unit, Felinor, Common, 7/6 → 14/12), a card of the held Felinors
//! vs Humans set (R5235000).
//!   Base:    "Cry: Discover a combat keyword to gain. Add 1~3 to your Woozy counter."
//!   Radiant: "Cry: Discover a pair of combat keywords to gain. Add 1~3 to your Woozy counter."
//!
//! Cry (R1), R5257001. Base: `discover_keyword` of three options from R21's combat pool that it lacks
//! (R5239004), drawn as the effect resolves; the pick is granted to itself for good. Radiant: three options of
//! two different combat keywords each, six different keywords it lacks drawn by the match rng as the effect
//! resolves, the pair it picks granted together. The designer's "each keyword (each set of 2) comes with a
//! Woozy 1-3" is its controller's Woozy counter (R3548000) raised by a uniform 1 to 3 rolled as the pick is
//! gained (`gain_woozy`, R5238011), once per pick on both faces; with nothing to offer there is no pick and
//! no Woozy. Declares no numbers: 1~3 is a roll's range.
//!
//! The engine has no verb that Discovers keyword pairs, so the Radiant offer is drawn inside a
//! `for_each_card` (off-label: its one "card" is the whole offer, kept as the part's memo across the pause)
//! and asked as a Choose one (`choose_mode`), each option naming its pair. Built by S07 (#5257).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-081";

/// The printed "Add 1~3 to your Woozy counter": the least and most one pick adds.
const WOOZY_LEAST: i32 = 1;
const WOOZY_MOST: i32 = 3;

/// The Radiant face's options: two keywords each.
const PAIR: usize = 2;

/// How a pair's option names its two keywords ("Taunt + Rush"), and how the offer's options are carried
/// through `for_each_card` as one string.
const PAIR_JOIN: &str = " + ";
const OFFER_JOIN: &str = "|";

/// The steps the picks resume at.
const GAIN: &str = "gainKeyword";
const GAIN_PAIR: &str = "gainKeywordPair";

/// R5257001: the Woozy roll that comes with a pick, uniform over 1 to 3. No prompt follows it in its step,
/// so the step is built once and the roll drawn once.
fn woozy_roll(ctx: &mut EffectContext<'_>) -> Effect {
    let amount = WOOZY_LEAST + ctx.sink.rng.int(WOOZY_MOST - WOOZY_LEAST + 1);
    gain_woozy(GainWoozyArgs { player: None, amount })
}

/// R21's combat keywords this lacks now (through the layers, §10.4), and no Taunt for an Indestructible
/// one, as `discover_keyword` leaves them out (R5239004).
fn lacking_keywords(ctx: &EffectContext<'_>) -> Vec<Keyword> {
    let held = ctx
        .live_self()
        .map(|me| unit_view(&*ctx.state, me).keywords)
        .unwrap_or_default();
    let indestructible = has_keyword(&held, KeywordKind::Indestructible);
    random_pool_keywords()
        .into_iter()
        .filter(|keyword| {
            !has_keyword(&held, keyword.kind()) && !(indestructible && keyword.kind() == KeywordKind::Taunt)
        })
        .collect()
}

fn pair_label(pair: &[Keyword]) -> String {
    pair.iter()
        .map(|keyword| keyword.kind().as_str())
        .collect::<Vec<_>>()
        .join(PAIR_JOIN)
}

/// The combat keyword an option names by its name.
fn keyword_named(name: &str) -> Option<Keyword> {
    random_pool_keywords()
        .into_iter()
        .find(|keyword| keyword.kind().as_str() == name)
}

/// R5257001: the Radiant offer, drawn as the effect resolves: six different keywords it lacks, two to an
/// option; with fewer than two left, the one left alone.
fn pair_offer(ctx: &mut EffectContext<'_>) -> Vec<String> {
    let lacking = lacking_keywords(ctx);
    let wanted = PAIR * usize::try_from(DISCOVER_OPTIONS).unwrap_or(0);
    let drawn: Vec<Keyword> = ctx.sink.rng.shuffle(&lacking).into_iter().take(wanted).collect();
    let mut options: Vec<String> = drawn
        .chunks(PAIR)
        .filter(|pair| pair.len() == PAIR)
        .map(pair_label)
        .collect();
    if options.is_empty() {
        options = drawn.iter().map(|keyword| keyword.kind().as_str().to_string()).collect();
    }
    if options.is_empty() {
        return vec![];
    }
    vec![options.join(OFFER_JOIN)]
}

/// The picks' continuations, on both faces.
fn resumes() -> IndexMap<&'static str, Hook> {
    IndexMap::from([
        (
            GAIN,
            hook(|ctx| {
                let Some(keyword) = discovered_keyword(ctx) else {
                    return vec![];
                };
                vec![
                    grant_keyword(GrantKeywordArgs {
                        target: TargetSpec::SelfCard,
                        keyword,
                    }),
                    woozy_roll(ctx),
                ]
            }),
        ),
        (
            GAIN_PAIR,
            hook(|ctx| {
                let Some(picked) = chosen_options(ctx).into_iter().next() else {
                    return vec![];
                };
                let keywords: Vec<Keyword> = picked.split(PAIR_JOIN).filter_map(keyword_named).collect();
                if keywords.is_empty() {
                    return vec![];
                }
                vec![
                    grant_keywords(GrantKeywordsArgs {
                        target: TargetSpec::SelfCard,
                        keywords,
                    }),
                    woozy_roll(ctx),
                ]
            }),
        ),
    ])
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|_ctx| {
            vec![discover_keyword(DiscoverKeywordArgs {
                step: GAIN.to_string(),
                lacking_on: Some(TargetSpec::SelfCard),
                ..DiscoverKeywordArgs::default()
            })]
        })),
        resume: resumes(),
        ..Script::default()
    };
    let radiant = Script {
        cry: Some(hook(|_ctx| {
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(|ctx: &mut EffectContext<'_>| pair_offer(ctx)),
                each: Arc::new(|offer: &str| {
                    choose_mode(ChooseModeArgs {
                        options: offer.split(OFFER_JOIN).map(str::to_string).collect(),
                        step: GAIN_PAIR.to_string(),
                        prompt: Some("Discover a pair of combat keywords".to_string()),
                        ..ChooseModeArgs::default()
                    })
                }),
            })]
        })),
        resume: resumes(),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// FvH #81 Old-Timer Catboy — SPEC §8.14 row 81.
#[cfg(test)]
mod tests {
    use super::{ID, PAIR, PAIR_JOIN, WOOZY_LEAST, WOOZY_MOST};
    use jackioh_engine::effects::random_pool_keywords;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const WEAPONS: &str = "core-014"; // Jlockeed's Weapons: Aura: your Units have Rush and First Strike
    const FILLER: &str = "core-005"; // Stockpile: a card to hold

    fn played(radiant: bool, seed: &str, backrow: Value) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "backrow": backrow, "library": crate::filler(4) },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        s.play(ID, json!({}));
        s
    }

    fn catboy(s: &Scenario) -> CardInstance {
        must(
            active_units_of(s.state(), P1)
                .into_iter()
                .find(|unit| unit.def_id == ID)
                .cloned(),
            "the Old-Timer Catboy on p1's field",
        )
    }

    fn has(s: &Scenario, kind: KeywordKind) -> bool {
        s.stats(catboy(s)).keywords.iter().any(|keyword| keyword.kind() == kind)
    }

    fn woozy(s: &Scenario) -> Option<i32> {
        s.state().players.p1.woozy.map(|counter| counter.count)
    }

    fn woozy_rises(s: &Scenario) -> usize {
        s.events_of(GameEventType::CounterChanged)
            .iter()
            .filter(|event| matches!(event, GameEvent::CounterChanged { counter: CounterKind::Woozy, .. }))
            .count()
    }

    fn kind_named(name: &str) -> KeywordKind {
        must(
            random_pool_keywords()
                .into_iter()
                .map(|keyword| keyword.kind())
                .find(|kind| kind.as_str() == name),
            "a combat keyword of that name",
        )
    }

    fn open_kind(s: &Scenario) -> PromptKind {
        must(s.state().pending.as_ref(), "an open prompt").kind
    }

    mod base {
        use super::*;

        #[test]
        fn r5257001_it_discovers_a_combat_keyword_gains_it_and_adds_1_to_3_woozy() {
            let mut s = played(false, "catboy-base", json!([]));
            assert_eq!(open_kind(&s), PromptKind::Discover);
            let options = nonempty(s.prompt_modes(), "the keywords offered");
            assert_eq!(options.len(), 3);
            for option in &options {
                assert!(
                    random_pool_keywords().iter().any(|keyword| keyword.kind().as_str() == option.as_str()),
                    "{option} is a combat keyword (R21)"
                );
            }
            assert_eq!(woozy(&s), None, "no Woozy before the pick");
            let choice = options[0].clone();
            s.answer(pick::mode(&choice));
            assert!(has(&s, kind_named(&choice)), "it gained {choice}");
            let count = must(woozy(&s), "a Woozy counter");
            assert!((WOOZY_LEAST..=WOOZY_MOST).contains(&count), "{count}");
            assert_eq!(woozy_rises(&s), 1, "one roll for the one pick");
        }

        #[test]
        fn r5257001_the_offer_leaves_out_what_it_has() {
            for seed in 0..8 {
                let s = played(false, &format!("catboy-lacks-{seed}"), json!([WEAPONS]));
                let options = nonempty(s.prompt_modes(), "the keywords offered");
                for option in &options {
                    let kind = kind_named(option);
                    assert!(kind != KeywordKind::Rush && kind != KeywordKind::FirstStrike, "{option}");
                }
            }
        }

        #[test]
        fn r5257001_the_woozy_roll_is_uniform_over_1_to_3() {
            let mut seen: Vec<i32> = Vec::new();
            for seed in 0..24 {
                let mut s = played(false, &format!("catboy-woozy-{seed}"), json!([]));
                let choice = must(s.prompt_modes().into_iter().next(), "an option");
                s.answer(pick::mode(&choice));
                let count = must(woozy(&s), "a Woozy counter");
                assert!((WOOZY_LEAST..=WOOZY_MOST).contains(&count), "{count}");
                if !seen.contains(&count) {
                    seen.push(count);
                }
            }
            assert!(seen.len() > 1, "more than one amount comes up: {seen:?}");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5257001_it_discovers_a_pair_gains_both_and_adds_1_to_3_woozy() {
            let mut s = played(true, "catboy-radiant", json!([]));
            assert_eq!(open_kind(&s), PromptKind::Mode);
            let options = nonempty(s.prompt_modes(), "the pairs offered");
            assert_eq!(options.len(), 3);
            let choice = options[0].clone();
            let pair: Vec<KeywordKind> = choice.split(PAIR_JOIN).map(kind_named).collect();
            assert_eq!(pair.len(), PAIR);
            assert_ne!(pair[0], pair[1], "two different keywords");
            s.answer(pick::mode(&choice));
            for kind in nonempty(pair, "the pair") {
                assert!(has(&s, kind), "it gained {kind:?}");
            }
            let count = must(woozy(&s), "a Woozy counter");
            assert!((WOOZY_LEAST..=WOOZY_MOST).contains(&count), "{count}");
            assert_eq!(woozy_rises(&s), 1, "one roll for the one pair");
        }

        #[test]
        fn r5257001_the_pairs_hold_six_different_keywords_it_lacks() {
            for seed in 0..8 {
                let s = played(true, &format!("catboy-pairs-{seed}"), json!([WEAPONS]));
                let options = nonempty(s.prompt_modes(), "the pairs offered");
                let mut kinds: Vec<KeywordKind> = Vec::new();
                for option in &options {
                    for name in option.split(PAIR_JOIN) {
                        let kind = kind_named(name);
                        assert!(kind != KeywordKind::Rush && kind != KeywordKind::FirstStrike, "{option}");
                        assert!(!kinds.contains(&kind), "{name} is offered once");
                        kinds.push(kind);
                    }
                }
                assert_eq!(kinds.len(), PAIR * 3);
            }
        }
    }
}
