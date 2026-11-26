//! What a fused definition is made of, for how its card is named (R3045000, issue #3045). The engine's Fuse
//! (`crates/engine/src/subsystems/fuse.rs` `build_def`) unions the ingredients' tags and joins their names, so a
//! Starship that took a piece is named "Dormant Starship + Miner v1". What a fused Dormant Starship is called is
//! the face's: the hull's own name, whatever was fused into it (`model.rs`, `chinese.rs`). The engine's names and
//! tags stay as they are, since a rule can read them (R2529000). Presentation only (CLAUDE.md rule 7).
//!
//! The web's art code reads the same shapes in `apps/web/src/cards/fused.ts` (which ingredient a fused card is
//! drawn as); the two walk an id the same way.

use std::str::FromStr;

use jackioh_engine::wire::{CardDef, KeywordKind, Tag};

use crate::config::keyword_line_rank;

/// The Dormant Starship's catalog id (Stinkyscape, R1852000); `tests/face.rs` holds both to the catalog's.
pub const DORMANT_STARSHIP_ID: &str = "stinkyscape-t-dormant-starship";
pub const DORMANT_STARSHIP_NAME: &str = "Dormant Starship";

/// The head of a fused id, `t-<digits>:` (`catalog.rs` `fused_head_len`, R179).
fn fused_head_len(id: &str) -> Option<usize> {
    let rest = id.strip_prefix("t-")?;
    let digits = rest.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 || rest.as_bytes().get(digits) != Some(&b':') {
        return None;
    }
    Some("t-".len() + digits + ":".len())
}

/// R468: a fused id past the cap spells a digest of its ingredients after its head, not their ids.
const DIGEST_MARK: char = '#';
/// R469: the mark an ingredient that went in on its Radiant face carries in a readable id.
const RADIANT_MARK: char = '*';
/// What the Fuse joins its ingredients' texts with (`fuse.rs` `fused_face`), and a Chinese face its table's.
pub(crate) const FUSED_LINE: &str = "\n";
const OPEN: char = '(';
const CLOSE: char = ')';
const JOIN: char = '+';

/// An ingredient as a fused id spells it (R179: a fused one in parentheses, R469: a Radiant one marked), as its id
/// and whether it carries the Radiant mark.
fn bare_id(spelled: &str) -> (&str, bool) {
    let unmarked = spelled.strip_suffix(RADIANT_MARK).unwrap_or(spelled);
    let id = match unmarked
        .strip_prefix(OPEN)
        .and_then(|inner| inner.strip_suffix(CLOSE))
    {
        Some(inner) => inner,
        None => unmarked,
    };
    (id, unmarked.len() != spelled.len())
}

/// R179, R468, R469: the ingredients a readable fused id spells, split at its top-level `+` signs, each with its
/// Radiant mark; `None` for a digest, which spells none.
fn spelled_ingredients(id: &str) -> Option<Vec<(String, bool)>> {
    let body = fused_head_len(id).map_or(id, |head| &id[head..]);
    if body.starts_with(DIGEST_MARK) {
        return None;
    }
    let chars: Vec<(usize, char)> = body.char_indices().collect();
    let mut spelled = Vec::new();
    let mut depth = 0_i32;
    let mut from = 0;
    for at in 0..=chars.len() {
        let (offset, mark) = chars.get(at).copied().unwrap_or((body.len(), '\0'));
        if at < chars.len() && mark == OPEN {
            depth += 1;
        } else if at < chars.len() && mark == CLOSE {
            depth -= 1;
        } else if at == chars.len() || (mark == JOIN && depth == 0) {
            let (bare, marked) = bare_id(&body[from..offset]);
            spelled.push((bare.to_string(), marked));
            from = offset + JOIN.len_utf8();
        }
    }
    Some(spelled)
}

/// The catalog cards a definition is made of, in ingredient order, a fusion of fusions walked to its leaves:
/// `[id]` for a card that is no fusion. `listed` answers the ingredient ids a definition the reader holds
/// lists (R468); one it does not hold (the view carries the definitions its cards name, R243, not the ones
/// inside them) is read from its readable id (R179). `None` when a digest hides some of them (R468), or a
/// definition lists itself.
pub fn fused_parts(
    id: &str,
    listed: &dyn Fn(&str) -> Option<Vec<String>>,
    inside: &[String],
) -> Option<Vec<String>> {
    let marked =
        |id: &str| listed(id).map(|ingredients| ingredients.into_iter().map(|id| (id, false)).collect());
    let leaves = leaves_in(id, false, &marked, inside)?;
    Some(leaves.into_iter().map(|leaf| leaf.def_id).collect())
}

/// R469, R5049000: the ingredients a held definition lists, each with its Radiant mark (`CardDef.ingredients`).
pub type MarkedListing<'a> = &'a dyn Fn(&str) -> Option<Vec<(String, bool)>>;

/// R469, R5049000: one catalog card a fused definition is made of, and whether it went in on its Radiant face:
/// marked so itself, or inside a fused ingredient that was (`fuse.rs` `fused_face` puts such a fusion in on
/// its Radiant face, every part of it with it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FusedLeaf {
    pub def_id: String,
    pub radiant: bool,
}

/// R469, R5049000: `fused_parts` with each leaf's Radiant mark. `listed` answers the ingredients a held
/// definition lists, each with its mark (`CardDef.ingredients`).
pub fn fused_leaves(id: &str, listed: MarkedListing<'_>) -> Option<Vec<FusedLeaf>> {
    leaves_in(id, false, listed, &[])
}

/// R5049000: the leaves of the definition `def`, the ones inside it read from `lookup` (the definitions the
/// view carries, R243) or from the ids they spell (R179).
pub fn leaves_of(def: &CardDef, lookup: &dyn Fn(&str) -> Option<CardDef>) -> Option<Vec<FusedLeaf>> {
    let listed = |id: &str| -> Option<Vec<(String, bool)>> {
        let ingredients = if id == def.id {
            def.ingredients.clone()
        } else {
            lookup(id)?.ingredients
        }?;
        Some(
            ingredients
                .into_iter()
                .map(|ingredient| (ingredient.def_id, ingredient.radiant == Some(true)))
                .collect(),
        )
    };
    fused_leaves(&def.id, &listed)
}

fn leaves_in(
    id: &str,
    radiant: bool,
    listed: MarkedListing<'_>,
    inside: &[String],
) -> Option<Vec<FusedLeaf>> {
    let ingredients = match listed(id).filter(|ingredients| !ingredients.is_empty()) {
        Some(ingredients) => Some(ingredients),
        None if fused_head_len(id).is_some() => spelled_ingredients(id),
        None => {
            return Some(vec![FusedLeaf {
                def_id: id.to_string(),
                radiant,
            }]);
        }
    };
    let ingredients = ingredients?;
    if inside.iter().any(|seen| seen == id) {
        return None;
    }
    let mut deeper = inside.to_vec();
    deeper.push(id.to_string());
    let mut leaves = Vec::new();
    for (ingredient, marked) in &ingredients {
        leaves.extend(leaves_in(ingredient, radiant || *marked, listed, &deeper)?);
    }
    Some(leaves)
}

/// Whether `def` is a fusion with the Dormant Starship's hull among its ingredients, in any depth. Only `def`
/// itself is held: a fusion inside it is read from the id it spells.
pub fn holds_dormant_starship(def: &CardDef) -> bool {
    let listed = |id: &str| -> Option<Vec<String>> {
        if id != def.id {
            return None;
        }
        def.ingredients.as_ref().map(|ingredients| {
            ingredients
                .iter()
                .map(|ingredient| ingredient.def_id.clone())
                .collect()
        })
    };
    fused_parts(&def.id, &listed, &[])
        .is_some_and(|parts| parts.len() > 1 && parts.iter().any(|part| part == DORMANT_STARSHIP_ID))
}

/// R3045000, R5050000: the Stinkyscape Miners, Miner v1 to v5, gear that prints no Weapon tag; the web's
/// `fusedArt.ts` `MINER_IDS` and `tests/face.rs` hold both to the catalog's.
pub const MINER_IDS: [&str; 5] = [
    "stinkyscape-062",
    "stinkyscape-063",
    "stinkyscape-064",
    "stinkyscape-065",
    "stinkyscape-066",
];

/// R5050000: a unit holding gear, a Miner or a Weapon, Equipped into it (§6.1 Equip, R1841002): the card it was
/// equipped into, and the gear, oldest first, each with R469's mark.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Equipped {
    pub host: FusedLeaf,
    pub gear: Vec<FusedLeaf>,
}

/// R3045000, R5050000: whether the catalog card `id` (its definition `def`, where found) is gear.
pub fn is_gear(id: &str, def: Option<&CardDef>) -> bool {
    MINER_IDS.contains(&id) || def.is_some_and(|def| def.tags.contains(&Tag::Weapon))
}

/// R5050000: `def` as a unit holding gear: a fusion whose leaves are gear but one, its host, the card the gear
/// was equipped into. An Equip fuses onto its host as `[equip, host]`, a fusion again for each pick (`[newest,
/// (older + host)]`), so the gear comes oldest first by reversing the leaves. `None` for any other definition: no
/// gear, gear fused onto gear, two cards that are no gear (a host that was itself a fusion), a Dormant Starship
/// (its pieces are its own list, R2249000), or a fusion a digest hides (R468).
pub fn equipped_of(def: &CardDef, lookup: &dyn Fn(&str) -> Option<CardDef>) -> Option<Equipped> {
    let leaves = leaves_of(def, lookup)?;
    if leaves.len() < 2 || leaves.iter().any(|leaf| leaf.def_id == DORMANT_STARSHIP_ID) {
        return None;
    }
    let (gear, hosts): (Vec<FusedLeaf>, Vec<FusedLeaf>) = leaves
        .into_iter()
        .partition(|leaf| is_gear(&leaf.def_id, lookup(&leaf.def_id).as_ref()));
    let [host] = <[FusedLeaf; 1]>::try_from(hosts).ok()?;
    if gear.is_empty() {
        return None;
    }
    Some(Equipped {
        host,
        gear: gear.into_iter().rev().collect(),
    })
}

/// R5050000: the card a unit holding gear was equipped into (`equipped_of`), its definition the catalog's: what the
/// unit is named for where nothing but the catalog is held (a log line, a Chinese name).
pub fn equipped_host(def: &CardDef) -> Option<CardDef> {
    let in_catalog = |id: &str| jackioh_engine::catalog::find_def(None, id).cloned();
    let gear = equipped_of(def, &in_catalog)?;
    in_catalog(&gear.host.def_id)
}

/// R3941002: whether `id` names a fused definition (`t-<n>:…`, R179).
pub fn is_fused(id: &str) -> bool {
    fused_head_len(id).is_some()
}

/// R3941002: an item of a keyword line: a keyword kind's label, numbered or not ("Armor 3", "Spell Damage +1",
/// "Armor X"), or a Tribute or Echo count (R366).
fn is_keyword_item(item: &str) -> bool {
    let counted = item.rsplit_once(' ').filter(|(_, n)| {
        let digits = n.strip_prefix('+').unwrap_or(*n);
        *n == "X" || (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
    });
    let name = counted.map_or(item, |(name, _)| name);
    jackioh_engine::KEYWORD_KINDS
        .iter()
        .any(|kind| kind.as_str() == item || kind.as_str() == name)
        || (counted.is_some() && (name == "Tribute" || name == "Echo"))
}

/// R3941002: whether `line` is a keyword line, every item of it a keyword kind's label or a count.
fn is_keyword_line(line: &str) -> bool {
    !line.is_empty() && line.split(", ").all(is_keyword_item)
}

/// R4747000: an item's place in the merged line: Tribute counts first, Echo counts next, then
/// every keyword kind in the line order (a numbered item ranks by its kind, mirroring
/// `is_keyword_item`'s split).
fn merged_item_rank(item: &str) -> (u8, usize) {
    let counted = item.rsplit_once(' ').filter(|(_, n)| {
        let digits = n.strip_prefix('+').unwrap_or(*n);
        *n == "X" || (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
    });
    let name = counted.map_or(item, |(name, _)| name);
    if counted.is_some() {
        if name == "Tribute" {
            return (0, 0);
        }
        if name == "Echo" {
            return (1, 0);
        }
    }
    match KeywordKind::from_str(name) {
        Ok(kind) => (2, keyword_line_rank(kind)),
        // Unreachable: only items of keyword lines are ranked, and `is_keyword_line` checked them.
        Err(_) => (2, usize::MAX),
    }
}

/// R3941002 (issue #3941): a fused text with its ingredients' keyword lines as one, at its head. Each ingredient's
/// text opens with its own keyword line (R366) and a fusion joins the texts (R77, R102), so an Equipped Unit
/// printed `Equip` and `Starship Piece` again on a line of their own, and a keyword its host has twice. The items
/// print once each, in the line order (R4747000), and every other line keeps its place after them. A text
/// with fewer than two keyword lines is unchanged. The fusion's keywords are their union already (R77): only the
/// words change.
pub fn merged_keyword_lines(text: &str) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    if lines.iter().filter(|line| is_keyword_line(line)).count() < 2 {
        return text.to_string();
    }
    let mut items: Vec<&str> = Vec::new();
    for line in lines.iter().filter(|line| is_keyword_line(line)) {
        for item in line.split(", ") {
            if !items.contains(&item) {
                items.push(item);
            }
        }
    }
    // R4747000: a stable sort, so repeats of a numbered keyword stay together in arrival order.
    items.sort_by_key(|item| merged_item_rank(item));
    let merged = items.join(", ");
    std::iter::once(merged.as_str())
        .chain(lines.iter().copied().filter(|line| !is_keyword_line(line)))
        .collect::<Vec<&str>>()
        .join("\n")
}

/// R3941002: `def` with its faces' keyword lines merged, when it is a fused definition; any other as it is.
pub fn with_merged_keyword_lines(mut def: CardDef) -> CardDef {
    if is_fused(&def.id) {
        def.base.text = merged_keyword_lines(&def.base.text);
        def.radiant.text = merged_keyword_lines(&def.radiant.text);
    }
    def
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The definitions held: id to the ingredient ids it lists.
    fn held(table: Vec<(&'static str, Vec<&'static str>)>) -> impl Fn(&str) -> Option<Vec<String>> {
        move |id| {
            table
                .iter()
                .find(|(key, _)| *key == id)
                .map(|(_, list)| list.iter().map(ToString::to_string).collect())
        }
    }

    fn parts(id: &str, listed: &dyn Fn(&str) -> Option<Vec<String>>) -> Option<Vec<String>> {
        fused_parts(id, listed, &[])
    }

    fn strings(ids: &[&str]) -> Option<Vec<String>> {
        Some(ids.iter().map(ToString::to_string).collect())
    }

    /// R3941002: a fusion's keyword lines print as one at its head, each item once, its other lines in order.
    /// R4747000: the merged line is in the line order, not first-seen order.
    #[test]
    fn r3941002_a_fused_text_prints_its_ingredients_keyword_lines_as_one() {
        assert_eq!(
            merged_keyword_lines(
                "Rush, Pierce\nCry: Deal 2 damage.\nStarship Piece, Equip, Pierce\nDamage also Burns 2."
            ),
            "Equip, Starship Piece, Rush, Pierce\nCry: Deal 2 damage.\nDamage also Burns 2."
        );
        assert_eq!(
            merged_keyword_lines("Tribute 1, Armor 3\nTaunt, Spell Damage +1, Armor X, Can't Defend"),
            "Tribute 1, Spell Damage +1, Can't Defend, Taunt, Armor 3, Armor X"
        );
        // One keyword line or none: unchanged, and a sentence is never a keyword line.
        assert_eq!(
            merged_keyword_lines("Rush\nCry: Draw 1 card."),
            "Rush\nCry: Draw 1 card."
        );
        assert_eq!(
            merged_keyword_lines("Taunt\nDeal 2 damage.\nTaunt, Rush loses."),
            "Taunt\nDeal 2 damage.\nTaunt, Rush loses."
        );
        assert!(is_fused("t-1:core-001+core-002"));
        assert!(!is_fused("core-001"));
    }

    #[test]
    fn r3045000_a_card_that_is_no_fusion_is_its_own_part_known_or_not() {
        let none = held(vec![]);
        assert_eq!(parts("core-020", &none), strings(&["core-020"]));
        assert_eq!(parts("core-999", &none), strings(&["core-999"]));
    }

    #[test]
    fn r3045000_a_fusion_of_fusions_is_walked_to_its_leaves_from_the_definitions_or_the_ids_they_spell() {
        let inner = "t-1:stinkyscape-050+core-020";
        let outer = "t-2:stinkyscape-063+(t-1:stinkyscape-050+core-020)";
        let want = strings(&["stinkyscape-063", "stinkyscape-050", "core-020"]);
        // The outer definition is held, the inner one is not: its id spells the rest (R243).
        let outer_held = held(vec![(outer, vec!["stinkyscape-063", inner])]);
        assert_eq!(parts(outer, &outer_held), want);
        assert_eq!(parts(outer, &held(vec![])), want);
        assert_eq!(
            parts(inner, &held(vec![])),
            strings(&["stinkyscape-050", "core-020"])
        );
    }

    #[test]
    fn r3045000_a_radiant_mark_on_a_spelled_ingredient_is_not_part_of_its_id() {
        let none = held(vec![]);
        assert_eq!(
            parts("t-4:stinkyscape-050*+core-020", &none),
            strings(&["stinkyscape-050", "core-020"])
        );
        assert_eq!(
            parts("t-5:stinkyscape-063+(t-4:stinkyscape-050*+core-020)*", &none),
            strings(&["stinkyscape-063", "stinkyscape-050", "core-020"])
        );
    }

    #[test]
    fn r3045000_a_digest_id_hides_what_it_fuses_unless_its_definition_is_held() {
        let digest = "t-6:#0123456789abcdef";
        assert_eq!(parts(digest, &held(vec![])), None);
        let own = held(vec![(digest, vec!["stinkyscape-050", "core-020"])]);
        assert_eq!(parts(digest, &own), strings(&["stinkyscape-050", "core-020"]));
        // One hidden inside a fusion hides the whole list, since its place in it is unknown.
        let outer = held(vec![("t-7:x", vec!["stinkyscape-063", digest])]);
        assert_eq!(parts("t-7:x", &outer), None);
    }

    /// R179: a fused ingredient's parentheses close where they open, so a `+` after them splits the id again.
    #[test]
    fn r3045000_a_spelled_fusion_first_in_an_id_closes_before_the_next_ingredient() {
        assert_eq!(
            parts(
                "t-9:(t-1:stinkyscape-050+core-020)+stinkyscape-063",
                &held(vec![])
            ),
            strings(&["stinkyscape-050", "core-020", "stinkyscape-063"])
        );
    }

    /// R3045000: the Dormant Starship itself is no fusion, so it holds no hull: only a fusion of it with something
    /// is named for it.
    #[test]
    fn r3045000_the_dormant_starship_alone_is_no_fusion_holding_its_hull() {
        jackioh_cards::register_all();
        let hull = jackioh_cards::CATALOG
            .get(DORMANT_STARSHIP_ID)
            .expect("the Dormant Starship");
        assert!(
            hull.ingredients.is_none(),
            "PREMISE: the catalog's Starship fuses nothing"
        );
        assert!(!holds_dormant_starship(hull));
    }

    #[test]
    fn r3045000_a_definition_that_lists_itself_ends_the_walk() {
        let looped = held(vec![("t-8:loop", vec!["t-8:loop", "core-020"])]);
        assert_eq!(parts("t-8:loop", &looped), None);
    }

    #[test]
    fn r3045000_a_fused_id_head_is_t_dash_digits_colon() {
        assert_eq!(fused_head_len("t-12:a+b"), Some(5));
        assert_eq!(fused_head_len("t-:a"), None);
        assert_eq!(fused_head_len("t-1a:a"), None);
        assert_eq!(fused_head_len("core-020"), None);
    }
}
