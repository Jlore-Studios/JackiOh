//! `cargo jackioh classify --claims` (#1370, R1370000): a pull request's pending fragments claim
//! what it does to the cards, no more and no less. The classifier (R1365000) names the families that
//! play differently; the head's fragments claim them through `cards` and `plays`, and a rule every game
//! meets through `rule` (R1366000). It fails, naming the family or the fragment, when a family plays
//! differently and no fragment claims it; when a `plays` claim this change adds names a family stage 2
//! shows playing as before (an earlier pull request's pending claims stand); when a rule every game
//! meets (`RULE_MIN_GAMES`, R2455000) goes unclaimed, or a new rule claim has none behind it; and,
//! R2529000, when a change to the cards' look and a `visual` claim do not match.
//!
//! The catalog claims are `patches check`'s (R646): this adds what the catalog diff cannot see, a
//! changed script or engine rule. CI runs it on every pull request (`.github/workflows/ci.yml`, `rust
//! (classify)`), with stage 2 against main's build of the base whenever stage 1 left something unchecked.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result};

use crate::classify::{Classification, family};
use crate::patches::git;
use crate::patches::patches_io::{NamedFragment, PendingFragment};

const PENDING_DIR: &str = "crates/cards/patches/pending";

/// The pending fragments `rev` holds, by file name. A file that is no fragment is left to `patches
/// check`, which names it.
pub(crate) fn fragments_at(root: &Path, rev: &str) -> Result<Vec<NamedFragment>> {
    let listing = git(
        root,
        &["ls-tree", "--name-only", &format!("{rev}:{PENDING_DIR}")],
        &[],
    )
    .unwrap_or_default();
    let mut out = Vec::new();
    for name in listing.lines().filter(|name| name.ends_with(".json")) {
        let text = git(
            root,
            &["cat-file", "-p", &format!("{rev}:{PENDING_DIR}/{name}")],
            &[],
        )
        .with_context(|| format!("{rev}'s pending/{name}"))?;
        if let Ok(fragment) = serde_json::from_str::<PendingFragment>(&text) {
            out.push(NamedFragment {
                name: name.to_string(),
                fragment,
            });
        }
    }
    Ok(out)
}

fn families<'a>(ids: impl IntoIterator<Item = &'a String>) -> BTreeSet<String> {
    ids.into_iter().map(|id| family(id)).collect()
}

/// R1370000: every way the fragments at the head disagree with what the change does, or [] when
/// they agree. `base` is the fragments the base held, so a claim an earlier change made is not
/// asked again.
pub(crate) fn problems(
    classification: &Classification,
    head: &[NamedFragment],
    base: &[NamedFragment],
) -> Vec<String> {
    let ran = classification.stage2 == "ran";
    let mut claimed = BTreeSet::new();
    for NamedFragment { fragment, .. } in head {
        claimed.extend(families(&fragment.cards));
        claimed.extend(families(&fragment.plays));
    }
    let mut problems = Vec::new();
    for card in &classification.cards {
        if !claimed.contains(card) {
            problems.push(format!(
                "\"{card}\" plays differently, but no pending fragment claims it: add it to the fragment's --plays (`cargo jackioh patches \"<title>\" --plays {card}`), or its catalog change to --cards (R1370000)"
            ));
        }
    }
    let held = |name: &str| {
        base.iter()
            .find(|named| named.name == name)
            .map(|named| &named.fragment)
    };
    for NamedFragment { name, fragment } in head {
        let before = held(name);
        if ran {
            let old = before
                .map(|fragment| families(&fragment.plays))
                .unwrap_or_default();
            for id in &fragment.plays {
                let card = family(id);
                if !old.contains(&card) && !classification.cards.contains(&card) {
                    problems.push(format!(
                        "pending/{name} claims \"{id}\" plays differently, but its games against the base build play as before (R1370000)"
                    ));
                }
            }
        }
        if fragment.visual && !before.is_some_and(|fragment| fragment.visual) && !classification.visual {
            problems.push(format!(
                "pending/{name} claims a change to how cards look, but nothing under apps/web/src/cards/ changed (R2529000)"
            ));
        }
        let newly_rule = fragment.rule && !before.is_some_and(|fragment| fragment.rule);
        if ran && newly_rule && !classification.rule {
            problems.push(format!(
                "pending/{name} claims a rule every game meets, but too few games against the base build diverge without a card (R1370000, R2455000)"
            ));
        }
    }
    if classification.visual && !head.iter().any(|named| named.fragment.visual) {
        problems.push(
            "how cards look changed (apps/web/src/cards/), a micro, but no pending fragment claims it: add --visual to the fragment (R2529000)"
                .to_string(),
        );
    }
    if classification.rule && !head.iter().any(|named| named.fragment.rule) {
        problems.push(
            "divergences against the base build touch no card in enough games to be a rule every game meets, but no pending fragment claims it: add --rule to the fragment (R1370000, R2455000)"
                .to_string(),
        );
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::{Kind, Tier};

    fn classified(cards: &[&str], rule: bool, stage2: &'static str) -> Classification {
        Classification {
            kind: if cards.is_empty() && !rule {
                Kind::App
            } else {
                Kind::Cards
            },
            tier: (!cards.is_empty() || rule).then_some(Tier::Micro),
            sets: Vec::new(),
            cards: cards.iter().map(|card| (*card).to_string()).collect(),
            reprinted: Vec::new(),
            wording: Vec::new(),
            rule,
            visual: false,
            stage2,
            unchecked: Vec::new(),
        }
    }

    fn named(name: &str, cards: &[&str], plays: &[&str], rule: bool) -> NamedFragment {
        let strings = |ids: &[&str]| ids.iter().map(|id| (*id).to_string()).collect();
        NamedFragment {
            name: name.to_string(),
            fragment: PendingFragment {
                title: name.to_string(),
                cards: strings(cards),
                plays: strings(plays),
                rule,
                ..PendingFragment::default()
            },
        }
    }

    #[test]
    fn r1370000_a_card_script_that_plays_differently_with_no_fragment_fails() {
        let change = classified(&["core-002"], false, "ran");
        assert_eq!(
            problems(&change, &[], &[]),
            vec![
                "\"core-002\" plays differently, but no pending fragment claims it: add it to the fragment's --plays (`cargo jackioh patches \"<title>\" --plays core-002`), or its catalog change to --cards (R1370000)"
                    .to_string()
            ]
        );
        let fragment = named("bigot.json", &[], &["core-002"], false);
        assert!(problems(&change, std::slice::from_ref(&fragment), &[]).is_empty());
        // A token's claim counts for its card, and a catalog claim for its family.
        let token = named("bigot.json", &["core-002-1"], &[], false);
        assert!(problems(&change, &[token], &[]).is_empty());
    }

    #[test]
    fn r1370000_a_claim_the_base_build_plays_as_before_fails_once_stage_2_ran() {
        let refactor = classified(&[], false, "ran");
        let fragment = named("refactor.json", &[], &["core-002"], false);
        assert_eq!(
            problems(&refactor, std::slice::from_ref(&fragment), &[]),
            vec![
                "pending/refactor.json claims \"core-002\" plays differently, but its games against the base build play as before (R1370000)"
                    .to_string()
            ]
        );
        // Unconfirmed without stage 2, and a claim an earlier change made is not asked again.
        assert!(
            problems(
                &classified(&[], false, "skipped"),
                std::slice::from_ref(&fragment),
                &[]
            )
            .is_empty()
        );
        assert!(
            problems(
                &refactor,
                std::slice::from_ref(&fragment),
                std::slice::from_ref(&fragment)
            )
            .is_empty()
        );
    }

    #[test]
    fn r1370000_a_rule_every_game_meets_is_claimed_exactly_when_stage_2_finds_one() {
        let rule = classified(&[], true, "ran");
        assert_eq!(problems(&rule, &[], &[]).len(), 1);
        assert!(problems(&rule, &[named("rule.json", &[], &[], true)], &[]).is_empty());
        let none = classified(&[], false, "ran");
        assert_eq!(
            problems(&none, &[named("rule.json", &[], &[], true)], &[]).len(),
            1
        );
    }

    // R2529000: a change to how cards look is claimed by `visual` exactly when one happened, and a
    // claim an earlier change left pending is not asked again.
    #[test]
    fn r2529000_a_change_to_how_cards_look_is_claimed_exactly_when_one_happened() {
        let visual = |name: &str| NamedFragment {
            name: name.to_string(),
            fragment: PendingFragment {
                title: name.to_string(),
                visual: true,
                ..PendingFragment::default()
            },
        };
        let looks = Classification {
            kind: Kind::Cards,
            tier: Some(Tier::Micro),
            visual: true,
            ..classified(&[], false, "skipped")
        };
        assert_eq!(problems(&looks, &[], &[]).len(), 1);
        assert!(problems(&looks, &[visual("faces.json")], &[]).is_empty());
        let none = classified(&[], false, "skipped");
        assert_eq!(problems(&none, &[visual("faces.json")], &[]).len(), 1);
        assert!(problems(&none, &[visual("faces.json")], &[visual("faces.json")]).is_empty());
    }

    #[test]
    fn r1370000_a_client_change_and_a_two_card_catalog_change_with_their_fragment_pass() {
        assert!(problems(&classified(&[], false, "skipped"), &[], &[]).is_empty());
        let two = classified(&["core-001", "core-002"], false, "skipped");
        let fragment = named("two.json", &["core-001", "core-002"], &[], false);
        assert!(problems(&two, &[fragment], &[]).is_empty());
    }
}
