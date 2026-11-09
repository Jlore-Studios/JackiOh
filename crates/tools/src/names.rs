//! `names`: each card's name, written beside its id from `catalog.json` (#3111).
//!
//! Four files hold a card's name for people to read: a comment on the line of its id in
//! `apps/web/src/audio/card-audio.json5` and `crates/cards/card-text.yaml` and in each
//! `apps/web/src/fx/cards/*.json5`, and the `Card` cell of the id-keyed rows at the end of
//! `docs/radiant-audit.md`. `voice-lines.test.ts`, `export_card_text.rs`, `tables.test.ts` and
//! `radiant_standard.rs` hold each to the catalog, so a renamed card used to fail them until a person
//! edited the lines. This rewrites them, so a rename is one command.
//!
//! It only ever rewrites the name on a line that is already an entry. It never adds, removes or reorders
//! one, because an entry carries a person's judgment (a voice, a motif, a verdict) and a generated stub
//! would pass the gate that asks for it.
//!
//! The card tests (#3486, every `.rs` file under `crates/cards/src` and `crates/cards/tests`, a script's
//! own file from its `#[cfg(test)]` on): a helper line, `const <NAME>: &str = "<id>"; // <comment>` with a
//! catalog id, keeps the card's name in its comment, and nothing beside a catalog id gives a cost or
//! stats, since a balance patch would make them stale: those are refused rather than checked, so there
//! is nothing to keep in sync (a test that wants only numbers names a stand-in, `stand-in (1) 4/4`,
//! `crates/engine/src/testkit/stand_in.rs`). A helper's comment that is its name, or starts with
//! `<name>: `, is kept; one whose text before a `: ` is a card's name (another card's, written beside
//! the wrong id) has that name put back; any other gets the name in front, `<name>: <comment>`, so a
//! comment's own words (`Cry: …`, `Radiant: …`) are never lost. A helper line with no comment is left,
//! and so is an id the catalog does not hold (a stand-in, a fixture, a fused id).

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Result, bail};

use crate::patches::repo_root;

/// A card's id and its name, as `catalog.json` has them.
type Names = BTreeMap<String, String>;

/// A file's rewriter: its new text, and the problems it found (a line it would not rewrite).
type Rewriter = fn(&str, &Names) -> (String, Vec<String>);

/// What `plan` finds: each file to change as `(path, new text)`, and the problems.
type Plan = (Vec<(String, String)>, Vec<String>);

const AUDIO: &str = "apps/web/src/audio/card-audio.json5";
const YAML: &str = "crates/cards/card-text.yaml";
const AUDIT: &str = "docs/radiant-audit.md";
const FX_DIR: &str = "apps/web/src/fx/cards";

/// The line that opens the audio file's cards section, and the prefix of the line that closes it.
const AUDIO_CARDS_OPEN: &str = "  cards: {";
const AUDIO_CARDS_CLOSE: &str = "  }";
/// The YAML's last section, which is no card.
const YAML_EMOTES_ID: &str = "emotes";
/// The header of the audit's id-keyed table (`radiant_standard.rs` reads the same line).
const AUDIT_HEADER: &str = "| Id | Card | Verdict | Reasoning |";
/// Where an fx entry's comment starts (`tables.test.ts`'s `ENTRY_LINE`).
const FX_COMMENT: &str = ", // ";

/// Where the card tests are (#3486): every `.rs` file under these.
const CARD_TEST_DIRS: [&str; 2] = ["crates/cards/src", "crates/cards/tests"];
/// The card scripts' files, whose card tests start at their `TESTS_OPEN` line; the script above it is
/// left alone.
const SCRIPTS_DIR: &str = "crates/cards/src/scripts/";
const TESTS_OPEN: &str = "#[cfg(test)]";
/// What a cost or stats beside a catalog id in a card test is told.
const NUMBERS_PROBLEM: &str = "the comment gives a cost or stats, which a balance patch makes stale: write `// <Name>` or `// <Name>: <role>`, or name a stand-in such as \"stand-in (1) 4/4 R12/12\" (crates/engine/src/testkit/stand_in.rs)";
/// The visibilities a helper line's `const` may carry.
const HELPER_VISIBILITIES: [&str; 3] = ["pub ", "pub(crate) ", "pub(super) "];

/// Fail, writing nothing, when a name comment is not the catalog's.
#[derive(clap::Args)]
pub struct Args {
    #[arg(long)]
    pub check: bool,
}

fn catalog_names() -> Names {
    jackioh_cards::CATALOG
        .values()
        .map(|card| (card.id.clone(), card.name.clone()))
        .collect()
}

/// Rewrite each line of `text` with `entry`, which answers the new line, `Ok(None)` for a line that is no
/// entry (copied as is) or the problem with an entry it will not rewrite (its line copied as is).
fn rewrite_lines(
    text: &str,
    mut entry: impl FnMut(&str) -> Result<Option<String>, String>,
) -> (String, Vec<String>) {
    let mut problems = Vec::new();
    let lines: Vec<String> = text
        .split('\n')
        .map(|line| match entry(line) {
            Ok(Some(new)) => new,
            Ok(None) => line.to_string(),
            Err(problem) => {
                problems.push(problem);
                line.to_string()
            }
        })
        .collect();
    (lines.join("\n"), problems)
}

/// `    "<id>": {` plus nothing or ` // …`, within `cards: { … }` in `card-audio.json5`, with `// <name>`.
fn rewrite_audio(text: &str, names: &Names) -> (String, Vec<String>) {
    let mut inside = false;
    rewrite_lines(text, |line| {
        if !inside {
            inside = line == AUDIO_CARDS_OPEN;
            return Ok(None);
        }
        if line.starts_with(AUDIO_CARDS_CLOSE) {
            inside = false;
            return Ok(None);
        }
        let Some(rest) = line.strip_prefix("    \"") else {
            return Ok(None);
        };
        let Some((id, tail)) = rest.split_once("\": {") else {
            return Ok(None);
        };
        if !(tail.is_empty() || tail.starts_with(" //")) || id.contains('"') {
            return Ok(None);
        }
        let name = names.get(id).ok_or_else(|| format!("{id} is not a catalog id"))?;
        Ok(Some(format!("    \"{id}\": {{ // {name}")))
    })
}

/// `<id>:` plus nothing or ` # …`, at the margin of `card-text.yaml`, with `# <name>`.
fn rewrite_yaml(text: &str, names: &Names) -> (String, Vec<String>) {
    rewrite_lines(text, |line| {
        if line.is_empty() || line.starts_with([' ', '#', '\t']) {
            return Ok(None);
        }
        let Some((id, tail)) = line.split_once(':') else {
            return Ok(None);
        };
        if !(tail.is_empty() || tail.starts_with(" #")) || id == YAML_EMOTES_ID {
            return Ok(None);
        }
        let name = names.get(id).ok_or_else(|| format!("{id} is not a catalog id"))?;
        Ok(Some(format!("{id}: # {name}")))
    })
}

/// `  "<id>": <entry>,` plus nothing or ` // …`, in an fx table, with `// <name>`.
fn rewrite_fx(text: &str, names: &Names) -> (String, Vec<String>) {
    rewrite_lines(text, |line| {
        let Some(rest) = line.strip_prefix("  \"") else {
            return Ok(None);
        };
        let Some((id, value)) = rest.split_once("\": ") else {
            return Ok(None);
        };
        let value = match value.rfind(FX_COMMENT) {
            Some(at) => &value[..at],
            None => match value.strip_suffix(',') {
                Some(value) => value,
                None => return Err(format!("{id}: no trailing comma")),
            },
        };
        let name = names.get(id).ok_or_else(|| format!("{id} is not a catalog id"))?;
        Ok(Some(format!("  \"{id}\": {value}{FX_COMMENT}{name}")))
    })
}

/// The cells of one audit row, trimmed, the last holding any `|` the reasoning has; `None` for a row of
/// fewer than four.
fn audit_cells(line: &str) -> Option<Vec<&str>> {
    let inner = line.trim().strip_prefix('|')?;
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    let cells: Vec<&str> = inner.splitn(4, '|').map(str::trim).collect();
    (cells.len() == 4).then_some(cells)
}

/// The id-keyed table's rows, `| <id> | <name> | <verdict> | <reasoning> |`, with the name written.
fn rewrite_audit(text: &str, names: &Names) -> (String, Vec<String>) {
    // The rows follow the header and its separator, and run while a line starts with `| `.
    let mut seen_header = false;
    let mut separators_left = 0;
    rewrite_lines(text, |line| {
        if !seen_header {
            if line == AUDIT_HEADER {
                seen_header = true;
                separators_left = 1;
            }
            return Ok(None);
        }
        if separators_left > 0 {
            separators_left -= 1;
            return Ok(None);
        }
        if !line.starts_with("| ") {
            return Ok(None);
        }
        let Some(cells) = audit_cells(line) else {
            return Err(format!("malformed row: {line}"));
        };
        let id = cells[0];
        let name = names.get(id).ok_or_else(|| format!("{id} is not a catalog id"))?;
        Ok(Some(format!("| {id} | {name} | {} | {} |", cells[2], cells[3])))
    })
}

/// The files whose text differs from what is written for the catalog, as `(path, new text)`, and the
/// problems the rewriters found.
fn plan(root: &Path, names: &Names) -> Result<Plan> {
    let mut files: Vec<(String, Rewriter)> = vec![
        (AUDIO.to_string(), rewrite_audio),
        (YAML.to_string(), rewrite_yaml),
    ];
    let mut fx: Vec<String> = Vec::new();
    let fx_dir = root.join(FX_DIR);
    for entry in fs::read_dir(&fx_dir).map_err(|error| anyhow::anyhow!("{}: {error}", fx_dir.display()))? {
        let path = entry?.path();
        if path.extension().is_some_and(|extension| extension == "json5")
            && let Some(file) = path.file_name().and_then(|file| file.to_str())
        {
            fx.push(format!("{FX_DIR}/{file}"));
        }
    }
    fx.sort();
    files.extend(fx.into_iter().map(|path| (path, rewrite_fx as Rewriter)));
    files.push((AUDIT.to_string(), rewrite_audit));
    files.extend(card_test_rewriters(root)?);
    apply(root, files, names)
}

/// Each file's rewrite, as `(path, new text)` where it changes, and the problems, by path.
fn apply(root: &Path, files: Vec<(String, Rewriter)>, names: &Names) -> Result<Plan> {
    let mut changed = Vec::new();
    let mut problems = Vec::new();
    for (path, rewrite) in files {
        let full = root.join(&path);
        let text = fs::read_to_string(&full).map_err(|error| anyhow::anyhow!("{path}: {error}"))?;
        let (new, found) = rewrite(&text, names);
        problems.extend(found.into_iter().map(|problem| format!("{path}: {problem}")));
        if new != text {
            changed.push((path, new));
        }
    }
    Ok((changed, problems))
}

/// Each card test file with its rewriter: a script's own file from its tests on, any other whole.
fn card_test_rewriters(root: &Path) -> Result<Vec<(String, Rewriter)>> {
    Ok(card_test_files(root)?
        .into_iter()
        .map(|path| {
            let rewrite: Rewriter = if path.starts_with(SCRIPTS_DIR) {
                rewrite_card_script
            } else {
                rewrite_card_test
            };
            (path, rewrite)
        })
        .collect())
}

/// Every `.rs` file under `CARD_TEST_DIRS`, as sorted paths from the repository's root.
fn card_test_files(root: &Path) -> Result<Vec<String>> {
    let mut pending: Vec<PathBuf> = CARD_TEST_DIRS.iter().map(|dir| root.join(dir)).collect();
    let mut files = Vec::new();
    while let Some(next) = pending.pop() {
        for entry in fs::read_dir(&next).map_err(|error| anyhow::anyhow!("{}: {error}", next.display()))? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let relative = path.strip_prefix(root)?;
                let parts: Vec<&str> = relative
                    .components()
                    .filter_map(|part| part.as_os_str().to_str())
                    .collect();
                files.push(parts.join("/"));
            }
        }
    }
    files.sort();
    Ok(files)
}

/// A card test file other than a script's: every line.
fn rewrite_card_test(text: &str, names: &Names) -> (String, Vec<String>) {
    rewrite_helpers(text, names, true)
}

/// A card script's own file: the lines after its first `TESTS_OPEN`.
fn rewrite_card_script(text: &str, names: &Names) -> (String, Vec<String>) {
    rewrite_helpers(text, names, false)
}

/// The card tests' lines, from the first or from the line after `TESTS_OPEN`, each problem with its line.
fn rewrite_helpers(text: &str, names: &Names, from_start: bool) -> (String, Vec<String>) {
    let mut in_tests = from_start;
    let mut number = 0;
    rewrite_lines(text, |line| {
        number += 1;
        if !in_tests {
            in_tests = line.trim() == TESTS_OPEN;
            return Ok(None);
        }
        card_test_line(line, names).map_err(|problem| format!("line {number}: {problem}"))
    })
}

/// One card test line: a helper's comment with its card's name, a cost or stats beside a catalog id
/// refused, any other line left.
fn card_test_line(line: &str, names: &Names) -> Result<Option<String>, String> {
    let Some(at) = comment_at(line) else {
        return Ok(None);
    };
    let (code, after) = (&line[..at], &line[at + "//".len()..]);
    let comment = after.trim();
    if comment.is_empty() {
        return Ok(None);
    }
    if let Some(id) = helper_id(code) {
        let Some(name) = names.get(id) else {
            return Ok(None);
        };
        if gives_numbers(&comment.replace(name.as_str(), "")) {
            return Err(format!("{id}: {NUMBERS_PROBLEM}"));
        }
        let named = named_comment(comment, name, names);
        if named == comment {
            return Ok(None);
        }
        return Ok(Some(format!("{code}// {named}")));
    }
    let ids: Vec<&str> = quoted(code)
        .into_iter()
        .filter(|text| names.contains_key(*text))
        .collect();
    let rest = ids.iter().fold(comment.to_string(), |rest, id| {
        rest.replace(names[*id].as_str(), "")
    });
    if !ids.is_empty() && gives_numbers(&rest) {
        return Err(format!("{}: {NUMBERS_PROBLEM}", ids.join(", ")));
    }
    Ok(None)
}

/// A helper's comment with the card's name: kept when it is the name or starts `<name>: `; when its text
/// before a `: ` is a card's name (the longest such, for a name like "Legacy: Mr. Vanilla" holds one),
/// that name is put back; the name put in front of any other, so none of the comment's own words is lost.
fn named_comment(comment: &str, name: &str, names: &Names) -> String {
    if comment == name || comment.starts_with(&format!("{name}: ")) {
        return comment.to_string();
    }
    let named_head = comment
        .match_indices(": ")
        .map(|(at, _)| at)
        .filter(|&at| names.values().any(|known| *known == comment[..at]))
        .last();
    match named_head {
        Some(at) => format!("{name}: {}", &comment[at + ": ".len()..]),
        None => format!("{name}: {comment}"),
    }
}

/// `[pub ]const <NAME>: &str = "<id>";`, the code of a helper line, as its id.
fn helper_id(code: &str) -> Option<&str> {
    let code = code.trim();
    let code = HELPER_VISIBILITIES
        .iter()
        .find_map(|visibility| code.strip_prefix(visibility))
        .unwrap_or(code);
    let (constant, rest) = code.strip_prefix("const ")?.split_once(": &str = \"")?;
    if constant.is_empty()
        || !constant
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return None;
    }
    let (id, tail) = rest.split_once('"')?;
    (tail == ";").then_some(id)
}

/// Where a line's `//` comment starts, outside any string literal.
fn comment_at(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut in_string = false;
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'\\' if in_string => at += 1,
            b'"' => in_string = !in_string,
            b'/' if !in_string && bytes.get(at + 1) == Some(&b'/') => return Some(at),
            _ => {}
        }
        at += 1;
    }
    None
}

/// The string literals in a line's code, in order.
fn quoted(code: &str) -> Vec<&str> {
    let bytes = code.as_bytes();
    let mut found = Vec::new();
    let mut open: Option<usize> = None;
    let mut at = 0;
    while at < bytes.len() {
        match (bytes[at], open) {
            (b'\\', Some(_)) => at += 1,
            (b'"', None) => open = Some(at + 1),
            (b'"', Some(from)) => {
                found.push(&code[from..at]);
                open = None;
            }
            _ => {}
        }
        at += 1;
    }
    found
}

/// Whether a text gives a cost, `(<digits>)` or `(X)`, or stats, a `/` with a digit or `X` before it and
/// one after it (past a `+` or `-`): `4/4`, `+2/+2`, `3X/3X`, `R12/12`.
fn gives_numbers(text: &str) -> bool {
    let number = |byte: u8| byte.is_ascii_digit() || byte == b'X';
    let cost = text.match_indices('(').any(|(at, _)| {
        let inner = &text[at + 1..];
        inner.split_once(')').is_some_and(|(inner, _)| {
            inner == "X" || (!inner.is_empty() && inner.bytes().all(|byte| byte.is_ascii_digit()))
        })
    });
    let bytes = text.as_bytes();
    let stats = text.match_indices('/').any(|(at, _)| {
        let mut after = at + 1;
        if matches!(bytes.get(after), Some(b'+' | b'-')) {
            after += 1;
        }
        at > 0 && number(bytes[at - 1]) && bytes.get(after).copied().is_some_and(number)
    });
    cost || stats
}

pub fn run(args: Args) -> Result<()> {
    let root = repo_root();
    let (changed, problems) = plan(&root, &catalog_names())?;
    for (path, text) in &changed {
        if args.check {
            println!("stale {path}");
        } else {
            fs::write(root.join(path), text).map_err(|error| anyhow::anyhow!("{path}: {error}"))?;
            println!("rewrote {path}");
        }
    }
    if !problems.is_empty() {
        bail!("{}", problems.join("\n"));
    }
    if args.check && !changed.is_empty() {
        bail!("a name comment is not the catalog's: run `cargo jackioh names`");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names() -> Names {
        [("core-001", "Big D-fender"), ("core-002", "Speciesist")]
            .into_iter()
            .map(|(id, name)| (id.to_string(), name.to_string()))
            .collect()
    }

    #[test]
    fn audio_comment_is_rewritten_added_and_the_rest_kept() {
        let text = concat!(
            "{\n",
            "  voices: {\n",
            "    \"core-001\": { say: \"Fred\" },\n",
            "  },\n",
            "  cards: {\n",
            "    \"core-001\": { // Old\n",
            "      play: { voice: \"guard\" },\n",
            "    },\n",
            "    \"core-002\": {\n",
            "      play: { voice: \"snob\" },\n",
            "    },\n",
            "  },\n",
            "  emotes: {\n",
            "    \"core-001\": {\n",
            "  },\n",
            "}\n",
        );
        let (new, problems) = rewrite_audio(text, &names());
        assert_eq!(problems, Vec::<String>::new());
        let want = text
            .replace("\"core-001\": { // Old", "\"core-001\": { // Big D-fender")
            .replace("    \"core-002\": {\n", "    \"core-002\": { // Speciesist\n");
        assert_eq!(new, want);
        assert!(new.ends_with("}\n"));
    }

    #[test]
    fn yaml_comment_is_rewritten_and_emotes_is_left() {
        let text = "# core-001: # not an entry\ncore-001: # Old\n  flavour: x\ncore-002:\n  voice:\n    play: y\nemotes:\n  vanilla:\n    greetings: z\n";
        let (new, problems) = rewrite_yaml(text, &names());
        assert_eq!(problems, Vec::<String>::new());
        assert_eq!(
            new,
            "# core-001: # not an entry\ncore-001: # Big D-fender\n  flavour: x\ncore-002: # Speciesist\n  voice:\n    play: y\nemotes:\n  vanilla:\n    greetings: z\n"
        );
    }

    #[test]
    fn fx_comment_is_rewritten_from_a_stale_or_missing_one() {
        let text = concat!(
            "// header, // not an entry\n",
            "{\n",
            "  \"core-001\": { cry: { motif: \"bolt\" } }, // Old\n",
            "  \"core-002\": \"none: vanilla\",\n",
            "}\n",
        );
        let (new, problems) = rewrite_fx(text, &names());
        assert_eq!(problems, Vec::<String>::new());
        assert_eq!(
            new,
            concat!(
                "// header, // not an entry\n",
                "{\n",
                "  \"core-001\": { cry: { motif: \"bolt\" } }, // Big D-fender\n",
                "  \"core-002\": \"none: vanilla\", // Speciesist\n",
                "}\n",
            )
        );

        let (same, problems) = rewrite_fx("  \"core-001\": \"none: vanilla\"\n", &names());
        assert_eq!(same, "  \"core-001\": \"none: vanilla\"\n");
        assert_eq!(problems, vec!["core-001: no trailing comma".to_string()]);
    }

    #[test]
    fn audit_name_cell_is_rewritten_and_the_reasoning_keeps_its_pipes() {
        let text = format!(
            "# Audit\n| 1 | Big D-fender | Unit | a | b | Meets |\n\n{AUDIT_HEADER}\n| --- | --- | --- | --- |\n| core-001 | Old | Meets | a|b |\nafter\n"
        );
        let (new, problems) = rewrite_audit(&text, &names());
        assert_eq!(problems, Vec::<String>::new());
        assert_eq!(
            new,
            format!(
                "# Audit\n| 1 | Big D-fender | Unit | a | b | Meets |\n\n{AUDIT_HEADER}\n| --- | --- | --- | --- |\n| core-001 | Big D-fender | Meets | a|b |\nafter\n"
            )
        );

        let bare = "| core-001 | Old | Meets | a |\n";
        assert_eq!(rewrite_audit(bare, &names()), (bare.to_string(), Vec::new()));

        let short = format!("{AUDIT_HEADER}\n| --- | --- | --- | --- |\n| core-001 | Old | Meets |\n");
        let (kept, problems) = rewrite_audit(&short, &names());
        assert_eq!(kept, short);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("malformed row"));
    }

    #[test]
    fn an_unknown_id_is_a_problem_and_the_line_is_left() {
        let audit = format!("{AUDIT_HEADER}\n| --- | --- | --- | --- |\n| core-999 | X | Meets | r |\n");
        let cases: [(Rewriter, &str); 4] = [
            (rewrite_audio, "  cards: {\n    \"core-999\": { // X\n  }\n"),
            (rewrite_yaml, "core-999: # X\n  flavour: x\n"),
            (rewrite_fx, "  \"core-999\": \"none: vanilla\", // X\n"),
            (rewrite_audit, &audit),
        ];
        for (rewrite, text) in cases {
            let (new, problems) = rewrite(text, &names());
            assert_eq!(new, text);
            assert_eq!(
                problems,
                vec!["core-999 is not a catalog id".to_string()],
                "{text}"
            );
        }
    }

    fn card_names() -> Names {
        [
            ("core-005", "Stockpile"),
            ("core-016", "Hit Job"),
            ("core-019", "Midrange Menace"),
            ("core-025", "4-mana 7/7"),
            ("traininggrounds-043", "Legacy: Mr. Vanilla"),
        ]
        .into_iter()
        .map(|(id, name)| (id.to_string(), name.to_string()))
        .collect()
    }

    #[test]
    fn card_test_comment_takes_the_name_and_keeps_its_role() {
        let text = concat!(
            "const KILL: &str = \"core-016\"; // Hit Job\n",
            "const KILL_2: &str = \"core-016\"; // Hit Job: the removal\n",
            "    pub const WRONG: &str = \"core-016\"; // Midrange Menace: the removal\n",
            "pub(crate) const ROLE: &str = \"core-016\";  // the removal\n",
            "pub(super) const CRY: &str = \"core-016\"; // Cry: Radiant: a text it keeps\n",
            "const RENAMED: &str = \"core-016\"; // Contract Killer: a name no card has\n",
            "const BARE: &str = \"core-016\";\n",
            "const EMPTY: &str = \"core-016\"; //\n",
            "const VANILLA: &str = \"stand-in (1) 4/4 R12/12\"; // the vanilla\n",
            "const FIXTURE: &str = \"fx-1\"; // a fixture\n",
            "const LEGACY: &str = \"traininggrounds-043\"; // Legacy: Mr. Vanilla: the old one\n",
            "const LEGACY_OLD: &str = \"traininggrounds-043\"; // Hit Job: the old one\n",
            "const WRONG_LEGACY: &str = \"core-016\"; // Legacy: Mr. Vanilla: the removal\n",
            "const SEVEN: &str = \"core-025\"; // 4-mana 7/7\n",
            "let kill = \"core-016\"; // the removal\n",
        );
        let want = concat!(
            "const KILL: &str = \"core-016\"; // Hit Job\n",
            "const KILL_2: &str = \"core-016\"; // Hit Job: the removal\n",
            "    pub const WRONG: &str = \"core-016\"; // Hit Job: the removal\n",
            "pub(crate) const ROLE: &str = \"core-016\";  // Hit Job: the removal\n",
            "pub(super) const CRY: &str = \"core-016\"; // Hit Job: Cry: Radiant: a text it keeps\n",
            "const RENAMED: &str = \"core-016\"; // Hit Job: Contract Killer: a name no card has\n",
            "const BARE: &str = \"core-016\";\n",
            "const EMPTY: &str = \"core-016\"; //\n",
            "const VANILLA: &str = \"stand-in (1) 4/4 R12/12\"; // the vanilla\n",
            "const FIXTURE: &str = \"fx-1\"; // a fixture\n",
            "const LEGACY: &str = \"traininggrounds-043\"; // Legacy: Mr. Vanilla: the old one\n",
            "const LEGACY_OLD: &str = \"traininggrounds-043\"; // Legacy: Mr. Vanilla: the old one\n",
            "const WRONG_LEGACY: &str = \"core-016\"; // Hit Job: the removal\n",
            "const SEVEN: &str = \"core-025\"; // 4-mana 7/7\n",
            "let kill = \"core-016\"; // the removal\n",
        );
        let (new, problems) = rewrite_card_test(text, &card_names());
        assert_eq!(problems, Vec::<String>::new());
        assert_eq!(new, want);
    }

    #[test]
    fn a_scripts_own_body_is_left() {
        let text = concat!(
            "pub const ID: &str = \"core-016\"; // Old\n",
            "const OTHER: &str = \"core-019\"; // (3) 9/9\n",
            "#[cfg(test)]\n",
            "mod tests {\n",
            "    const KILL: &str = \"core-016\"; // removal\n",
            "}\n",
        );
        let (new, problems) = rewrite_card_script(text, &card_names());
        assert_eq!(problems, Vec::<String>::new());
        assert_eq!(new, text.replace("// removal", "// Hit Job: removal"));

        let (_, problems) = rewrite_card_test(text, &card_names());
        assert_eq!(problems, vec![format!("line 2: core-019: {NUMBERS_PROBLEM}")]);
    }

    #[test]
    fn a_cost_or_stats_beside_a_real_id_is_a_problem_with_its_line() {
        let text = concat!(
            "#[cfg(test)]\n",
            "const BIG_ATTACKER: &str = \"core-019\"; // (9) 9/9 Unit\n",
            "        \"field\": [{ \"def\": \"core-019\", \"lane\": 2 }], // 8/10 after the buff\n",
            "const SPARE: &str = \"core-005\"; // Stockpile, a (1) Spell\n",
            "const SEVEN: &str = \"core-025\"; // 4-mana 7/7: (4) 7/7\n",
            "const BIG: &str = \"stand-in (3) 9/9\"; // (3) 9/9\n",
            "// \"core-019\" is a (3) 9/9\n",
            "let pair = [\"fx-1\", \"stand-in (1) 2/2\"]; // 2/2s\n",
            "let url = \"https://core-019\"; // 1/2\n",
        );
        let (new, problems) = rewrite_card_script(text, &card_names());
        assert_eq!(new, text);
        assert_eq!(
            problems,
            vec![
                format!("line 2: core-019: {NUMBERS_PROBLEM}"),
                format!("line 3: core-019: {NUMBERS_PROBLEM}"),
                format!("line 4: core-005: {NUMBERS_PROBLEM}"),
                format!("line 5: core-025: {NUMBERS_PROBLEM}"),
            ]
        );
    }

    #[test]
    fn gives_numbers_reads_costs_and_stats_only() {
        for text in [
            "(1)",
            "a (12) Spell",
            "(X) Unit",
            "4/4",
            "+2/+2",
            "-1/-1",
            "3X/3X",
            "R12/12",
            "X/X",
        ] {
            assert!(gives_numbers(text), "{text}");
        }
        for text in [
            "Hit Job: the removal",
            "(the removal)",
            "and/or",
            "crates/cards/src",
            "Jlockeed Shredder-10",
            "/4",
            "4/",
            "(1a)",
            "()",
        ] {
            assert!(!gives_numbers(text), "{text}");
        }
    }

    #[test]
    fn a_second_card_test_pass_changes_nothing() {
        let text = concat!(
            "const OLD: &str = \"core-016\"; // Contract Killer: the removal\n",
            "const ROLE: &str = \"core-016\"; // the removal\n",
            "const LEGACY: &str = \"traininggrounds-043\"; // the old one\n",
        );
        let (once, _) = rewrite_card_test(text, &card_names());
        assert_ne!(once, text);
        assert_eq!(
            rewrite_card_test(&once, &card_names()),
            (once.clone(), Vec::new())
        );
    }

    /// #3486: what `plan` finds in the card tests alone.
    fn card_test_plan(root: &Path, names: &Names) -> Result<Plan> {
        apply(root, card_test_rewriters(root)?, names)
    }

    /// #3486: the card tests as they are pass `names --check`: no helper's comment lacks its card's
    /// name and none beside a catalog id gives a cost or stats.
    #[test]
    fn card_tests_pass_names_check() {
        let root = repo_root();
        let files = card_test_rewriters(&root).expect("the card tests read");
        assert!(
            files.len() > 700,
            "only {} card test files were walked",
            files.len()
        );
        let (changed, problems) = card_test_plan(&root, &catalog_names()).expect("the card tests read");
        assert!(
            problems.is_empty(),
            "a comment beside a catalog id in a card test gives a cost or stats (#3486):\n{}",
            problems.join("\n")
        );
        let stale: Vec<&str> = changed.iter().map(|(path, _)| path.as_str()).collect();
        assert!(
            stale.is_empty(),
            "a helper's comment lacks its card's name: run `cargo jackioh names` ({stale:?})"
        );
    }

    #[test]
    fn a_second_pass_changes_nothing() {
        let audit = format!("{AUDIT_HEADER}\n| --- | --- | --- | --- |\n| core-001 | Old | Meets | a|b |\n");
        let cases: [(Rewriter, &str); 4] = [
            (
                rewrite_audio,
                "  cards: {\n    \"core-001\": { // Old\n    },\n    \"core-002\": {\n  }\n",
            ),
            (
                rewrite_yaml,
                "core-001: # Old\ncore-002:\n  flavour: x\nemotes:\n",
            ),
            (
                rewrite_fx,
                "  \"core-001\": { cry: { motif: \"bolt\" } }, // Old\n  \"core-002\": \"none: vanilla\",\n",
            ),
            (rewrite_audit, &audit),
        ];
        for (rewrite, text) in cases {
            let (once, _) = rewrite(text, &names());
            assert_ne!(once, text);
            assert_eq!(rewrite(&once, &names()), (once.clone(), Vec::new()));
        }
    }
}
