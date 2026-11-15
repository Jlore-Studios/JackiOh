//! What a fused definition is made of, for how its card is named (R3045000, issue #3045). The engine's Fuse
//! (`crates/engine/src/subsystems/fuse.rs` `build_def`) unions the ingredients' tags and joins their names, so a
//! Starship that took a piece is named "Dormant Starship + Miner v1". What a fused Dormant Starship is called is
//! the face's: the hull's own name, whatever was fused into it (`model.rs`, `chinese.rs`). The engine's names and
//! tags stay as they are, since a rule can read them (R2529000). Presentation only (CLAUDE.md rule 7).
//!
//! The web's art code reads the same shapes in `apps/web/src/cards/fused.ts` (which ingredient a fused card is
//! drawn as); the two walk an id the same way.

use jackioh_engine::wire::CardDef;

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
const OPEN: char = '(';
const CLOSE: char = ')';
const JOIN: char = '+';

/// An ingredient as a fused id spells it (R179: a fused one in parentheses, R469: a Radiant one marked), as its id.
fn bare_id(spelled: &str) -> &str {
    let unmarked = spelled.strip_suffix(RADIANT_MARK).unwrap_or(spelled);
    match unmarked
        .strip_prefix(OPEN)
        .and_then(|inner| inner.strip_suffix(CLOSE))
    {
        Some(inner) => inner,
        None => unmarked,
    }
}

/// R179, R468: the ingredient ids a readable fused id spells, split at its top-level `+` signs; `None` for a
/// digest, which spells none.
fn spelled_ingredients(id: &str) -> Option<Vec<String>> {
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
            spelled.push(bare_id(&body[from..offset]).to_string());
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
    let ingredients = match listed(id).filter(|ingredients| !ingredients.is_empty()) {
        Some(ingredients) => Some(ingredients),
        None if fused_head_len(id).is_some() => spelled_ingredients(id),
        None => return Some(vec![id.to_string()]),
    };
    let ingredients = ingredients?;
    if inside.iter().any(|seen| seen == id) {
        return None;
    }
    let mut deeper = inside.to_vec();
    deeper.push(id.to_string());
    let mut parts = Vec::new();
    for ingredient in &ingredients {
        parts.extend(fused_parts(ingredient, listed, &deeper)?);
    }
    Some(parts)
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

/// R3941002 (issue #3941): a fused text with its ingredients' keyword lines as one, at its head. Each ingredient's
/// text opens with its own keyword line (R366) and a fusion joins the texts (R77, R102), so an Equipped Unit
/// printed `Equip` and `Starship Piece` again on a line of their own, and a keyword its host has twice. The items
/// keep their order, one printed twice printing once, and every other line keeps its place after them. A text
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
    #[test]
    fn r3941002_a_fused_text_prints_its_ingredients_keyword_lines_as_one() {
        assert_eq!(
            merged_keyword_lines(
                "Rush, Pierce\nCry: Deal 2 damage.\nStarship Piece, Equip, Pierce\nDamage also Burns 2."
            ),
            "Rush, Pierce, Starship Piece, Equip\nCry: Deal 2 damage.\nDamage also Burns 2."
        );
        assert_eq!(
            merged_keyword_lines("Tribute 1, Armor 3\nTaunt, Spell Damage +1, Armor X, Can't Defend"),
            "Tribute 1, Armor 3, Taunt, Spell Damage +1, Armor X, Can't Defend"
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
