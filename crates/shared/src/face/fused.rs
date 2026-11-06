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
