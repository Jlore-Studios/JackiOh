//! R1846000–R1846003 (issue #1846): the card metrics over the real catalog. `crates/cards/build.rs`
//! counts every script's code lines and compiles in the BUSTED and complicated lists, which
//! `register_all` hands to the engine and the catalog query reaches (`busted`, `complicated`). Both
//! lists are computed on every build, so they grow with the catalog and this file names no card it
//! expects in them: it holds each list to its threshold and to the cards on each side of it. The
//! query's options on fixture cards, `everySet` among them, are the engine's
//! (`crates/engine/tests/rules/card_metrics.rs`). #3116: the build also lists the cards whose script
//! owes an extra turn, which the AI prices in hand (R846).

use jackioh_cards::code_lines::owes_extra_turn;
use jackioh_cards::query::{CardQuery, query};
use jackioh_cards::{
    BUSTED, CATALOG, CODE_LINES, COMPLICATED, EXTRA_TURN_CARDS, carried_versions, register_all,
    shipped_catalog,
};
use jackioh_engine::catalog::set_is_open;
use jackioh_engine::{
    BUSTED_CODE_LINES, COMPLICATED_CODE_LINES, CardDef, GLITCH_DEF_ID, SINGLE_PLAYER_SETS, card_metrics,
    code_lines_of, is_busted, is_complicated, is_extra_turn_card, with_shipped,
};

fn ids_of_defs<'a>(defs: impl IntoIterator<Item = &'a CardDef>) -> Vec<String> {
    defs.into_iter().map(|def| def.id.clone()).collect()
}

fn sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids
}

fn recount(source: &str) -> usize {
    source
        .lines()
        .map(str::trim)
        .take_while(|line| !line.starts_with("#[cfg(test)]"))
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .count()
}

#[test]
fn r1846000_every_catalog_entry_has_its_scripts_code_lines_sorted_by_id() {
    register_all();
    let ids: Vec<&str> = CODE_LINES.iter().map(|(id, _)| *id).collect();
    let mut catalog: Vec<&str> = CATALOG.keys().map(String::as_str).collect();
    catalog.sort_unstable();
    assert_eq!(
        ids, catalog,
        "one count per catalog entry, a token's included, sorted by id"
    );
    for (id, count) in CODE_LINES {
        assert!(*count > 0, "{id}: a script holds code");
        assert_eq!(code_lines_of(id), Some(*count), "{id}");
    }
    assert_eq!(
        card_metrics().code_lines,
        CODE_LINES,
        "register_all hands the compiled metrics to the engine"
    );
}

#[test]
fn r1846000_a_count_is_its_scripts_code_lines_above_its_tests() {
    register_all();
    // Two scripts with no block comment, read here as text: Core #91 Fed Fauci, near the complicated
    // threshold, and Classic #19 Lizard's Breath, near the BUSTED one.
    for (id, source) in [
        (
            "core-091",
            include_str!("../../src/scripts/core/c091_fed_fauci.rs"),
        ),
        (
            "classic-019",
            include_str!("../../src/scripts/classic/c019_lizards_breath.rs"),
        ),
    ] {
        assert!(!source.contains("/*"), "{id}: no block comment");
        assert_eq!(code_lines_of(id), Some(recount(source)), "{id}");
    }
}

type List = (&'static str, &'static [&'static str], usize, fn(&str) -> bool);

#[test]
fn r1846000_each_list_is_every_card_over_its_threshold_with_a_card_on_each_side() {
    register_all();
    assert_eq!((BUSTED_CODE_LINES, COMPLICATED_CODE_LINES), (100, 60));
    let lists: [List; 2] = [
        ("BUSTED", BUSTED, BUSTED_CODE_LINES, is_busted),
        ("complicated", COMPLICATED, COMPLICATED_CODE_LINES, is_complicated),
    ];
    for (name, list, over, reads) in lists {
        let expected: Vec<&str> = CODE_LINES
            .iter()
            .filter(|(_, count)| *count > over)
            .map(|(id, _)| *id)
            .collect();
        assert_eq!(list, expected.as_slice(), "{name}: the ids over {over}, by id");
        for (id, count) in CODE_LINES {
            assert_eq!(reads(id), *count > over, "{name}: {id} has {count} lines");
        }
        let (shortest_over, its_lines) = CODE_LINES
            .iter()
            .filter(|(_, count)| *count > over)
            .min_by_key(|(_, count)| *count)
            .expect("a card over the threshold");
        let (longest_not, not_lines) = CODE_LINES
            .iter()
            .filter(|(_, count)| *count <= over)
            .max_by_key(|(_, count)| *count)
            .expect("a card on or under the threshold");
        assert!(
            reads(shortest_over),
            "{name}: {shortest_over} has {its_lines} lines"
        );
        assert!(!reads(longest_not), "{name}: {longest_not} has {not_lines} lines");
    }
    assert!(
        BUSTED.iter().all(|id| COMPLICATED.contains(id)),
        "a BUSTED card is complicated too"
    );
}

#[test]
fn r1846000_the_query_draws_each_list_from_the_sets_that_ship() {
    register_all();
    let asks = [
        (
            CardQuery {
                busted: Some(true),
                ..CardQuery::default()
            },
            BUSTED,
        ),
        (
            CardQuery {
                complicated: Some(true),
                ..CardQuery::default()
            },
            COMPLICATED,
        ),
    ];
    for (ask, list) in asks {
        let listed = || {
            list.iter()
                .map(|id| &CATALOG[*id])
                .filter(|def| set_is_open(def.set))
        };
        // §5.1: a token joins only a pool that takes tokens, and Glitch none (R674).
        assert_eq!(
            sorted(ids_of_defs(query(&ask))),
            sorted(ids_of_defs(listed().filter(|def| !def.token)))
        );
        let with_tokens = CardQuery {
            with_tokens: Some(true),
            ..ask.clone()
        };
        assert_eq!(
            sorted(ids_of_defs(query(&with_tokens))),
            sorted(ids_of_defs(listed().filter(|def| def.id != GLITCH_DEF_ID)))
        );
        assert!(!query(&ask).is_empty(), "the pool holds cards");
    }
}

#[test]
fn r1846001_every_set_adds_nothing_until_a_single_player_set_holds_a_card() {
    register_all();
    assert!(CATALOG.values().all(|def| !SINGLE_PLAYER_SETS.contains(&def.set)));
    let busted = CardQuery {
        busted: Some(true),
        ..CardQuery::default()
    };
    let every = CardQuery {
        every_set: Some(true),
        ..busted.clone()
    };
    assert_eq!(ids_of_defs(query(&every)), ids_of_defs(query(&busted)));
}

#[test]
fn r1846003_r2173000_a_fold_under_a_carried_version_reads_its_own_lists_or_the_builds() {
    register_all();
    let build = card_metrics();
    let versions = carried_versions();
    assert!(!versions.is_empty(), "the build carries older versions");
    for version in versions {
        let shipped = shipped_catalog(version).expect("a carried version's catalog");
        // R2173000: a version that kept its own lists folds against them; one that kept none (every
        // version before the first card that reads them shipped) against the build's.
        let expected = match shipped.card_lists() {
            None => build,
            Some(lists) => jackioh_engine::CardMetrics {
                code_lines: build.code_lines,
                busted: &lists.busted,
                complicated: &lists.complicated,
                extra_turn_cards: build.extra_turn_cards,
            },
        };
        assert_eq!(
            with_shipped(Some(shipped), card_metrics),
            expected,
            "{version}: its own lists, else the build's"
        );
    }
}

/// #3116: the build lists every script whose code names `take_extra_turn`, sorted by id, and hands the
/// list to the engine, where the AI prices it in hand (R846): M #19.1 Temporal Rift's and S #4.3 Event
/// Horizon's cry, and S #4.4 Last Word's Trap step, but not S #4.1 Farewell, a Terminus Spell beside them.
#[test]
fn r846_the_build_lists_the_scripts_that_owe_an_extra_turn_sorted_by_id() {
    register_all();
    let mut sorted = EXTRA_TURN_CARDS.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(EXTRA_TURN_CARDS, sorted.as_slice(), "sorted by id, each once");
    for id in EXTRA_TURN_CARDS {
        assert!(CATALOG.contains_key(*id), "{id}: a catalog card");
    }
    assert_eq!(
        card_metrics().extra_turn_cards,
        EXTRA_TURN_CARDS,
        "register_all hands the extra-turn cards to the engine"
    );
    for (id, source) in [
        (
            "meditative-019-1",
            include_str!("../../src/scripts/meditative/c019_1_temporal_rift.rs"),
        ),
        (
            "stinkyscape-004-3",
            include_str!("../../src/scripts/stinkyscape/c004_3_event_horizon.rs"),
        ),
        (
            "stinkyscape-004-4",
            include_str!("../../src/scripts/stinkyscape/c004_4_last_word.rs"),
        ),
    ] {
        assert!(owes_extra_turn(source), "{id}: its script owes an extra turn");
        assert!(is_extra_turn_card(id), "{id}: the engine lists it");
    }
    assert!(!owes_extra_turn(include_str!(
        "../../src/scripts/stinkyscape/c004_1_farewell.rs"
    )));
    assert!(!is_extra_turn_card("stinkyscape-004-1"));
}

/// #3116: a script owes an extra turn once its code, above its tests and outside its comments, names
/// `take_extra_turn` as a whole word, whichever hook returns it: a Unit's cry, or a Trap's step that
/// resumes its reveal. So a card added later is priced with no list to join.
#[test]
fn r846_a_fixture_script_owes_an_extra_turn_once_its_code_names_take_extra_turn_in_any_hook() {
    let cry = r#"
pub const ID: &str = "fixture-unit";
use jackioh_engine::prelude::*;
pub fn script() -> CardScripts {
    CardScripts {
        cry: Some(hook(|_ctx| vec![take_extra_turn(json_as(json!({})))])),
        ..CardScripts::default()
    }
}
"#;
    assert!(owes_extra_turn(cry), "a Unit's cry");
    let trap = r#"
pub const ID: &str = "fixture-trap";
use jackioh_engine::effects::{end_turn, take_extra_turn};
use jackioh_engine::prelude::*;
fn after_prevent(ctx: &EffectContext) -> Vec<Effect> {
    vec![end_turn(json_as(json!({}))), take_extra_turn(json_as(json!({})))]
}
"#;
    assert!(owes_extra_turn(trap), "a Trap's resumed reveal step");
    let comments = r#"
//! Owes nothing: `take_extra_turn` is named in its comments only.
pub const ID: &str = "fixture-comment";
/* take_extra_turn(json_as(json!({}))) */
pub fn script() -> CardScripts {
    CardScripts::default() // take_extra_turn
}
"#;
    assert!(!owes_extra_turn(comments), "named in comments only");
    let tests = r#"
pub const ID: &str = "fixture-tested";
pub fn script() -> CardScripts {
    CardScripts::default()
}
#[cfg(test)]
mod tests {
    use jackioh_engine::effects::take_extra_turn;
}
"#;
    assert!(!owes_extra_turn(tests), "named in its tests only");
    let longer = r#"
pub const ID: &str = "fixture-longer";
pub fn script() -> CardScripts {
    let _ = (take_extra_turns(), not_take_extra_turn());
    CardScripts::default()
}
"#;
    assert!(!owes_extra_turn(longer), "only inside longer names");
}
