//! R436: Call to Chaos names the effects it rolled, to both players.
//!
//! The engine reports the roll as `chaosRolled { player, instanceId, defId, effects }`, each effect as the
//! clause its card prints (the engine's `label` in `CHAOS_EFFECTS`, `CHAOS_PLUS_EFFECTS` and
//! `CHAOS_MED_EFFECTS`, `crates/engine/src/subsystems/call_to_chaos*.rs`), in resolve order, on both seats
//! (R97 redacts only the card, never the roll). `data/chaos-names.json` turns a clause into a reel's short
//! words, per edition: "Deck" (R373), "(3) Cost" as the noun and "cost (2) less" as the verb (R432). The web's
//! reel and banner read the same file. A clause the table lacks is shown as the engine wrote it (R202 is not
//! at stake: the roll is public).

use crate::face::data::{CHAOS_NAMES, find};

fn own(def_id: &str, key: &str) -> Option<&'static str> {
    find(CHAOS_NAMES, def_id)
        .and_then(|names| find(names, key))
        .copied()
}

fn as_words(key: &str) -> String {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return "Unknown effect".to_string();
    }
    if trimmed.chars().any(char::is_whitespace) {
        return trimmed.to_string();
    }
    let mut chars = trimmed.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// One clause as a reel shows it, in English: the edition's own words, else the words every edition that has
/// the clause spells it by, else the clause as the engine wrote it.
pub fn chaos_effect_name(def_id: &str, key: &str) -> String {
    if let Some(named) = own(def_id, key) {
        return named.to_string();
    }
    // An edition the viewer cannot name: a key every edition that has it spells the same way reads so.
    let mut spellings: Vec<&str> = Vec::new();
    for (_, names) in CHAOS_NAMES {
        if let Some(name) = find(names, key)
            && !spellings.contains(name)
        {
            spellings.push(name);
        }
    }
    match spellings.as_slice() {
        [only] => (*only).to_string(),
        _ => as_words(key),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clause_reads_by_its_editions_words() {
        assert_eq!(
            chaos_effect_name("core-095", "Heal your hero 30"),
            "Heal the caster's hero 30"
        );
        assert_eq!(
            chaos_effect_name("meditative-095", "Summon a Jade Beauty"),
            "Summon a Jade Beauty"
        );
    }

    #[test]
    fn an_edition_the_viewer_cannot_name_reads_a_clause_every_edition_spells_alike() {
        assert_eq!(
            chaos_effect_name("hidden", "Cast a random Call to Chaos"),
            "Cast a random Call to Chaos"
        );
        // Only one edition has this clause, so its words are the only spelling.
        assert_eq!(
            chaos_effect_name("hidden", "Heal your hero 30"),
            "Heal the caster's hero 30"
        );
    }

    #[test]
    fn a_clause_the_table_lacks_reads_as_the_engine_wrote_it() {
        assert_eq!(chaos_effect_name("core-095", "  "), "Unknown effect");
        assert_eq!(chaos_effect_name("core-095", "shrug"), "Shrug");
        assert_eq!(chaos_effect_name("core-095", "Do two things"), "Do two things");
    }
}
