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

use std::{collections::BTreeMap, fs, path::Path};

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
