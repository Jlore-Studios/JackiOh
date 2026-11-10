//! Stand-in cards (#3486): a card test that wants "some 3-cost 9/9" names it by an id that spells it
//! out, `"stand-in (3) 9/9"`, and `catalog::find_def` builds the card from the id on a miss, under
//! `testkit` alone. A test that borrowed a real card for its numbers broke, or quietly changed, when a
//! balance patch moved them; a stand-in's numbers are its id, so there is nothing to keep in sync.
//!
//! THE GRAMMAR (`STAND_IN_GRAMMAR`): a Unit is `stand-in (<cost>) <attack>/<health>[ <keywords>]` and
//! then, for its Radiant face, `[ R<attack>/<health>[ <keywords>]]`; any other card is
//! `stand-in (<cost>) Spell`, `Field Spell`, `Trap` or `Field Trap`, with no text. Numbers are ASCII
//! digits. Keywords are joined by `", "`, each as `keyword_key` writes it (`Armor 7`, `Spell Damage 1`,
//! `Temporary` or `Temporary 2`). The Radiant part starts at the first word that is `R` and a digit. A
//! Unit with no Radiant part takes R349's fallback (`spec/05-card-types.md`): its base face with the
//! attack and health multiplied by `RADIANT_FALLBACK_FACTOR`, the same keywords, `radiantFallback`
//! set. A Radiant part with no keywords keeps the base face's. `+`, `*`, `#`, `@` and `:` are refused:
//! a fused id splits at `+` (R179) and marks a Radiant ingredient with `*` (R469), a digest id reads `#`
//! (R468), a versioned one `@` (R388) and a crafted one `:`. A malformed id panics with the grammar.
//!
//! IDENTITY: the id is its id, its index and its name; a Common Core card with no tags, no token, no
//! text but its keyword line, and no script, so it plays as a Vanilla card of its numbers.
//!
//! The cache is a thread-local list beside the scenario's overrides, each card leaked as
//! `register_catalog` leaks (a test builds a handful) and never cleared, since a card is a pure
//! function of its id. It stays out of `registered()`, which every random pool, `query`, `def_by_index`
//! and the deck validator read (§5.1), so no pool gains a member and no seeded draw moves.

use std::cell::Cell;

use crate::config::RADIANT_FALLBACK_FACTOR;
use crate::wire::{
    CardCost, CardDef, CardFace, CardType, Keyword, KeywordKind, Rarity, SetName, keyword_key,
};

/// What every stand-in id starts with; no catalog id does (`its_prefix_is_no_catalog_id…`).
pub const STAND_IN_PREFIX: &str = "stand-in";

/// The grammar a malformed stand-in id's panic gives.
pub const STAND_IN_GRAMMAR: &str = "A stand-in id is `stand-in (<cost>) <attack>/<health>[ <keywords>][ R<attack>/<health>[ <keywords>]]` for a Unit (\"stand-in (3) 9/9\", \"stand-in (1) 4/4 R12/12\", \"stand-in (4) 7/7 Armor 7\", \"stand-in (1) 3/3 Rush, First Strike\"), its keywords joined by \", \" as `keyword_key` writes them, a Radiant face left out being R349's fallback and one with no keywords keeping the base face's; or `stand-in (<cost>) Spell`, `Field Spell`, `Trap` or `Field Trap`, a card with no text. Numbers are digits; `+ * # @ :` are refused.";

/// The characters a fused (`+`, `*`), digest (`#`), versioned (`@`) or crafted (`:`) id reads.
const REFUSED: [char; 5] = ['+', '*', '#', '@', ':'];

/// Where the Radiant part of a Unit's id starts: a word that is this and a digit.
const RADIANT_MARK: char = 'R';

thread_local! {
    static STAND_INS: Cell<&'static [&'static CardDef]> = const { Cell::new(&[]) };
}

pub fn is_stand_in(def_id: &str) -> bool {
    def_id.starts_with(STAND_IN_PREFIX)
}

/// The card a stand-in id spells out, built (and cached on this thread) the first time it is asked
/// for; `None` for an id that is no stand-in. Panics, giving the grammar, on a malformed one.
pub fn stand_in_def(def_id: &str) -> Option<&'static CardDef> {
    if !is_stand_in(def_id) {
        return None;
    }
    let built = STAND_INS.with(Cell::get);
    if let Some(def) = built.iter().find(|def| def.id == def_id) {
        return Some(def);
    }
    let def = match parse_stand_in(def_id) {
        Ok(def) => def,
        Err(why) => panic!("{def_id:?} is no stand-in id: {why}. {STAND_IN_GRAMMAR}"),
    };
    let def: &'static CardDef = Box::leak(Box::new(def));
    let mut all = built.to_vec();
    all.push(def);
    let all: &'static [&'static CardDef] = Box::leak(all.into_boxed_slice());
    STAND_INS.with(|cell| cell.set(all));
    Some(def)
}

/// The card `def_id` spells out, or why it spells none.
pub fn parse_stand_in(def_id: &str) -> Result<CardDef, String> {
    let head = format!("{STAND_IN_PREFIX} (");
    let Some(rest) = def_id.strip_prefix(&head) else {
        return Err(format!("it does not start with `{head}`"));
    };
    if let Some(refused) = def_id.chars().find(|ch| REFUSED.contains(ch)) {
        return Err(format!("it holds `{refused}`, which another kind of id reads"));
    }
    if def_id.contains("  ")
        || def_id.ends_with(' ')
        || def_id.chars().any(|ch| ch.is_whitespace() && ch != ' ')
    {
        return Err("its words are not each one space apart".to_string());
    }
    let Some((cost, rest)) = rest.split_once(") ") else {
        return Err("its cost is not followed by `) `".to_string());
    };
    let cost = number(cost, "cost")?;
    if rest.starts_with(|ch: char| ch.is_ascii_digit()) {
        return unit(def_id, cost, rest);
    }
    match rest.parse::<CardType>() {
        Ok(CardType::Unit) => {
            Err("a Unit is written by its stats, `<attack>/<health>`, not `Unit`".to_string())
        }
        Ok(type_) => {
            let face = face(None, Vec::new());
            Ok(card(def_id, cost, type_, face.clone(), face, None))
        }
        Err(_) => match CardType::ALL
            .iter()
            .find(|type_| rest.starts_with(&format!("{} ", type_.as_str())))
        {
            Some(type_) => Err(format!(
                "a stand-in {} has no text, and {rest:?} gives some",
                type_.as_str()
            )),
            None => Err(format!("{rest:?} is neither stats nor a card type")),
        },
    }
}

/// A Unit's faces from `<attack>/<health>[ <keywords>][ R<attack>/<health>[ <keywords>]]`.
fn unit(def_id: &str, cost: i32, rest: &str) -> Result<CardDef, String> {
    let words: Vec<&str> = rest.split(' ').collect();
    let radiant_at = words.iter().position(|word| {
        let mut chars = word.chars();
        chars.next() == Some(RADIANT_MARK) && chars.next().is_some_and(|ch| ch.is_ascii_digit())
    });
    let (base_words, radiant_words) = match radiant_at {
        Some(at) => (&words[..at], Some(&words[at..])),
        None => (&words[..], None),
    };
    let (base_stats, base_keywords) = unit_face(base_words, "the base face")?;
    let base = face(Some(base_stats), base_keywords.clone());
    let Some(radiant_words) = radiant_words else {
        let doubled = (
            base_stats.0 * RADIANT_FALLBACK_FACTOR,
            base_stats.1 * RADIANT_FALLBACK_FACTOR,
        );
        let radiant = face(Some(doubled), base_keywords);
        return Ok(card(def_id, cost, CardType::Unit, base, radiant, Some(true)));
    };
    let mut radiant_words = radiant_words.to_vec();
    radiant_words[0] = &radiant_words[0][RADIANT_MARK.len_utf8()..];
    let (radiant_stats, radiant_keywords) = unit_face(&radiant_words, "the Radiant face")?;
    let radiant_keywords = if radiant_words.len() == 1 {
        base_keywords
    } else {
        radiant_keywords
    };
    let radiant = face(Some(radiant_stats), radiant_keywords);
    Ok(card(def_id, cost, CardType::Unit, base, radiant, None))
}

/// One Unit face's stats and keywords from its words, `<attack>/<health>` and then its keyword line.
fn unit_face(words: &[&str], which: &str) -> Result<((i32, i32), Vec<Keyword>), String> {
    let Some((stats, line)) = words.split_first() else {
        return Err(format!("{which} gives no stats"));
    };
    let Some((attack, health)) = stats.split_once('/') else {
        return Err(format!("{which}'s stats {stats:?} are not `<attack>/<health>`"));
    };
    let stats = (number(attack, "attack")?, number(health, "health")?);
    if line.is_empty() {
        return Ok((stats, Vec::new()));
    }
    let mut keywords: Vec<Keyword> = Vec::new();
    for written in line.join(" ").split(", ") {
        let keyword = keyword(written)?;
        if keywords.iter().any(|known| known.kind() == keyword.kind()) {
            return Err(format!("{which} gives {} twice", keyword.kind().as_str()));
        }
        keywords.push(keyword);
    }
    Ok((stats, keywords))
}

/// A keyword as `keyword_key` writes it: its kind, then its number for a numbered kind (and, if
/// written, for Temporary, whose number is optional, R2179001).
fn keyword(written: &str) -> Result<Keyword, String> {
    let (kind, n) = match written.parse::<KeywordKind>() {
        Ok(kind) => (kind, None),
        Err(_) => match written.rsplit_once(' ') {
            Some((kind, n)) if kind.parse::<KeywordKind>().is_ok() => {
                (kind.parse::<KeywordKind>()?, Some(number(n, kind)?))
            }
            _ => return Err(format!("{written:?} is no keyword")),
        },
    };
    if KeywordKind::RETIRED.contains(&kind) {
        return Err(format!("{} is a retired keyword", kind.as_str()));
    }
    let numbered = Keyword::of_kind(kind, 1).n().is_some();
    match (n, numbered) {
        (Some(n), true) => Ok(Keyword::of_kind(kind, n)),
        (None, true) => Err(format!("{} takes a number", kind.as_str())),
        (Some(n), false) if kind == KeywordKind::Temporary => Ok(Keyword::Temporary { n: Some(n) }),
        (Some(_), false) => Err(format!("{} takes no number", kind.as_str())),
        (None, false) => Ok(Keyword::of_kind(kind, 0)),
    }
}

/// A number written in ASCII digits alone.
fn number(written: &str, what: &str) -> Result<i32, String> {
    if written.is_empty() || !written.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!("its {what} {written:?} is not a number"));
    }
    written
        .parse::<i32>()
        .map_err(|_| format!("its {what} {written:?} is too large"))
}

/// A face with these stats and keywords, its text its keyword line.
fn face(stats: Option<(i32, i32)>, keywords: Vec<Keyword>) -> CardFace {
    let text = keywords.iter().map(keyword_key).collect::<Vec<_>>().join(", ");
    CardFace {
        type_: None,
        attack: stats.map(|(attack, _)| attack),
        health: stats.map(|(_, health)| health),
        x_stats: None,
        keywords,
        text,
        grants: None,
    }
}

fn card(
    def_id: &str,
    cost: i32,
    type_: CardType,
    base: CardFace,
    radiant: CardFace,
    radiant_fallback: Option<bool>,
) -> CardDef {
    CardDef {
        id: def_id.to_string(),
        index: def_id.to_string(),
        name: def_id.to_string(),
        set: SetName::Core,
        type_,
        tags: Vec::new(),
        rarity: Rarity::Common,
        printed_rarity: None,
        flavor: None,
        token: false,
        cost: CardCost::Fixed(cost),
        refs: None,
        params: None,
        deals_damage: None,
        uses_plague: None,
        resource: None,
        loc: None,
        radiant_fallback,
        ingredients: None,
        craft: None,
        snapshot_of: None,
        base,
        radiant,
    }
}

#[cfg(test)]
mod tests {
    use std::panic::catch_unwind;

    use super::*;
    use crate::catalog::{find_def, fused_head_len, fused_id_parts, is_digest_id};
    use crate::config::CRAFT_ID_PREFIX;
    use crate::wire::CardDefs;

    fn kinds(keywords: &[Keyword]) -> Vec<String> {
        keywords.iter().map(keyword_key).collect()
    }

    /// (id, cost, base stats, base keywords, Radiant stats, Radiant keywords, fallback).
    type UnitCase = (
        &'static str,
        i32,
        (i32, i32),
        &'static [&'static str],
        (i32, i32),
        &'static [&'static str],
        Option<bool>,
    );

    #[test]
    fn each_unit_form_builds_its_card() {
        let cases: [UnitCase; 7] = [
            ("stand-in (3) 9/9", 3, (9, 9), &[], (18, 18), &[], Some(true)),
            ("stand-in (1) 4/4 R12/12", 1, (4, 4), &[], (12, 12), &[], None),
            (
                "stand-in (4) 7/7 Armor 7",
                4,
                (7, 7),
                &["Armor 7"],
                (14, 14),
                &["Armor 7"],
                Some(true),
            ),
            (
                "stand-in (1) 3/3 Rush, First Strike",
                1,
                (3, 3),
                &["Rush", "First Strike"],
                (6, 6),
                &["Rush", "First Strike"],
                Some(true),
            ),
            (
                "stand-in (1) 3/3 Rush, First Strike R6/6 Charge, First Strike",
                1,
                (3, 3),
                &["Rush", "First Strike"],
                (6, 6),
                &["Charge", "First Strike"],
                None,
            ),
            (
                "stand-in (2) 6/1 Rush, First Strike R12/2",
                2,
                (6, 1),
                &["Rush", "First Strike"],
                (12, 2),
                &["Rush", "First Strike"],
                None,
            ),
            (
                "stand-in (0) 0/1 Spell Damage 1, Temporary",
                0,
                (0, 1),
                &["Spell Damage 1", "Temporary"],
                (0, 2),
                &["Spell Damage 1", "Temporary"],
                Some(true),
            ),
        ];
        for (id, cost, base_stats, base_keywords, radiant_stats, radiant_keywords, fallback) in cases {
            let def = parse_stand_in(id).unwrap_or_else(|why| panic!("{id}: {why}"));
            assert_eq!(def.cost, CardCost::Fixed(cost), "{id}");
            assert_eq!(def.type_, CardType::Unit, "{id}");
            assert_eq!(
                (def.base.attack, def.base.health),
                (Some(base_stats.0), Some(base_stats.1)),
                "{id}"
            );
            assert_eq!(
                (def.radiant.attack, def.radiant.health),
                (Some(radiant_stats.0), Some(radiant_stats.1)),
                "{id}"
            );
            assert_eq!(kinds(&def.base.keywords), base_keywords, "{id}");
            assert_eq!(kinds(&def.radiant.keywords), radiant_keywords, "{id}");
            assert_eq!(def.base.text, base_keywords.join(", "), "{id}");
            assert_eq!(def.radiant.text, radiant_keywords.join(", "), "{id}");
            assert_eq!(def.radiant_fallback, fallback, "{id}");
        }
    }

    #[test]
    fn each_card_type_builds_a_textless_card() {
        for (id, type_) in [
            ("stand-in (0) Spell", CardType::Spell),
            ("stand-in (2) Field Spell", CardType::FieldSpell),
            ("stand-in (1) Trap", CardType::Trap),
            ("stand-in (3) Field Trap", CardType::FieldTrap),
        ] {
            let def = parse_stand_in(id).unwrap_or_else(|why| panic!("{id}: {why}"));
            assert_eq!(def.type_, type_, "{id}");
            assert_eq!(def.base, def.radiant, "{id}");
            assert_eq!((def.base.attack, def.base.health), (None, None), "{id}");
            assert!(def.base.keywords.is_empty() && def.base.text.is_empty(), "{id}");
            assert_eq!(def.radiant_fallback, None, "{id}");
        }
        assert_eq!(
            parse_stand_in("stand-in (2) Field Spell").map(|def| def.cost),
            Ok(CardCost::Fixed(2))
        );
    }

    #[test]
    fn its_identity_is_its_id_a_common_core_card() {
        let id = "stand-in (1) 4/4 R12/12";
        let def = parse_stand_in(id).expect("a stand-in");
        assert_eq!(
            (def.id.as_str(), def.index.as_str(), def.name.as_str()),
            (id, id, id)
        );
        assert_eq!(
            (def.set, def.rarity, def.token),
            (SetName::Core, Rarity::Common, false)
        );
        assert!(def.tags.is_empty());
        assert_eq!(def.flavor, None);
        assert_eq!(def.loc, None);
        assert_eq!(def.refs, None);
        assert_eq!(def.params, None);
    }

    #[test]
    fn every_keyword_reads_back_as_keyword_key_writes_it() {
        for &kind in KeywordKind::ALL {
            if KeywordKind::RETIRED.contains(&kind) {
                continue;
            }
            let written = Keyword::of_kind(kind, 2);
            let id = format!("stand-in (1) 1/1 {}", keyword_key(&written));
            let def = parse_stand_in(&id).unwrap_or_else(|why| panic!("{id}: {why}"));
            assert_eq!(def.base.keywords, vec![written], "{id}");
        }
        let def = parse_stand_in("stand-in (1) 1/1 Temporary 2").expect("Temporary 2");
        assert_eq!(def.base.keywords, vec![Keyword::Temporary { n: Some(2) }]);
    }

    #[test]
    fn a_malformed_id_is_refused() {
        for id in [
            "stand-in",
            "stand-in 4/4",
            "stand-in (1)",
            "stand-in (1)4/4",
            "stand-in (one) 4/4",
            "stand-in (-1) 4/4",
            "stand-in (1) 4",
            "stand-in (1) 4/x",
            "stand-in (1) 4/4 Rush,First Strike",
            "stand-in (1) 4/4 Rush, Rush",
            "stand-in (1) 4/4 Armor",
            "stand-in (1) 4/4 Taunt 2",
            "stand-in (1) 4/4 Flying Fish",
            "stand-in (1) 4/4 Immune to tribal tag based hate",
            "stand-in (1) 4/4 Spell Damage +1",
            "stand-in (1) 4/4 R",
            "stand-in (1) 4/4 R8",
            "stand-in (1) 4/4  R8/8",
            "stand-in (1) 4/4 ",
            "stand-in (1) Unit",
            "stand-in (1) Spell Draw a card",
            "stand-in (1) Spell, Trap",
            "stand-in (1) Creature",
            "stand-in (1) 4/4+stand-in (0) Spell",
            "stand-in (1) 4/4*",
            "stand-in (1) 4/4 #1",
            "stand-in (1) 4/4@v0.5.13",
            "stand-in (1) 4/4: the big one",
            "stand-in (99999999999) 4/4",
        ] {
            assert!(parse_stand_in(id).is_err(), "{id:?} was read");
        }
    }

    #[test]
    #[should_panic(expected = "is no stand-in id")]
    fn find_def_panics_on_a_malformed_stand_in() {
        find_def(None, "stand-in (1) 4/4 Wings");
    }

    #[test]
    fn the_panic_gives_the_grammar() {
        let panic = catch_unwind(|| stand_in_def("stand-in (1) four/4")).expect_err("a malformed id panics");
        let message = panic.downcast_ref::<String>().cloned().unwrap_or_default();
        assert!(
            message.contains("\"stand-in (1) four/4\" is no stand-in id"),
            "{message}"
        );
        assert!(message.contains(STAND_IN_GRAMMAR), "{message}");
    }

    #[test]
    fn a_stand_in_is_built_once() {
        let id = "stand-in (2) 5/6 Taunt";
        let first = find_def(None, id).expect("built");
        let second = find_def(None, id).expect("cached");
        assert!(std::ptr::eq(first, second));
        assert_eq!(first.id, id);
        assert_eq!(stand_in_def("core-008"), None);
    }

    #[test]
    fn its_prefix_is_no_catalog_id_and_no_fused_or_crafted_id() {
        let catalog: CardDefs = serde_json::from_str(include_str!("../../../cards/catalog.json"))
            .expect("catalog.json reads as CardDefs");
        assert!(catalog.len() > 100);
        for def in catalog.values() {
            assert!(!is_stand_in(&def.id), "{} is a stand-in id", def.id);
            assert!(!is_stand_in(&def.name), "{} is named like a stand-in", def.id);
        }
        let id = "stand-in (1) 4/4 R12/12";
        assert_eq!(fused_head_len(id), None);
        assert!(!is_digest_id(id));
        assert!(!id.starts_with(CRAFT_ID_PREFIX));
        assert_eq!(fused_id_parts(None, id), None);
        assert_eq!(
            fused_id_parts(None, "t-1:stand-in (1) 4/4 R12/12+stand-in (0) Spell"),
            Some(vec![
                "stand-in (1) 4/4 R12/12".to_string(),
                "stand-in (0) Spell".to_string()
            ])
        );
    }
}
