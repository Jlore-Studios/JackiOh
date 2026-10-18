//! R1183004 (Meditative+ #25 KY's Intuition): every card's code block, compiled in by
//! `crates/cards/build.rs` from its own script and handed to the engine by `register_all`. A block is
//! the script's lines of code as `cargo jackioh catalog loc` counts them, less its `ID` line, the first
//! `CODE_BLOCK_MAX_LINES` of them, naming neither its own card's id nor its name.

use jackioh_cards::{CATALOG, CODE_BLOCKS, register_all};
use jackioh_engine::{CODE_BLOCK_MAX_LINES, code_block_of};

#[test]
fn r1183004_every_card_but_a_token_has_one_code_block_sorted_by_id() {
    register_all();
    let ids: Vec<&str> = CODE_BLOCKS.iter().map(|(id, _)| *id).collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(ids, sorted, "sorted by id, one each");
    for (id, def) in CATALOG.iter() {
        assert_eq!(
            code_block_of(id).is_some(),
            !def.token,
            "{id}: a card has a code block, a token none"
        );
    }
}

#[test]
fn r1183004_a_block_is_the_scripts_first_lines_of_code_and_names_not_its_card() {
    register_all();
    for (id, code) in CODE_BLOCKS {
        let def = &CATALOG[*id];
        let lines: Vec<&str> = code.lines().collect();
        assert!(!lines.is_empty(), "{id}: a block holds code");
        assert!(
            lines.len() <= CODE_BLOCK_MAX_LINES,
            "{id}: at most CODE_BLOCK_MAX_LINES lines"
        );
        for line in &lines {
            let trimmed = line.trim();
            assert!(!trimmed.is_empty(), "{id}: no blank line");
            assert!(!trimmed.starts_with("//"), "{id}: no comment: {line}");
            assert!(!trimmed.starts_with("use "), "{id}: no use declaration: {line}");
            assert!(!trimmed.starts_with("pub const ID"), "{id}: no ID line");
        }
        assert!(!code.contains(*id), "{id}: the block never spells its card's id");
        assert!(
            !code.contains(&def.name),
            "{id}: the block never spells its card's name"
        );
    }
}

#[test]
fn r1183004_cn_techs_block_reads_its_script() {
    register_all();
    let code = code_block_of("meditative-050").expect("CN Tech has a block");
    let first = code.lines().next().expect("a first line");
    assert_eq!(first, "fn tech(radiant: bool) -> Script {");
    // Its own id is `ID` where the code reads it, as the script names it.
    assert!(code.contains("win_rate_leader(ID)"));
}
