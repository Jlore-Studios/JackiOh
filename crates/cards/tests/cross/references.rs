//! R279: a card's text links the cards and tokens it names. The catalog lists them per card
//! (`refs`, by id), and this file proves the list against the texts both ways, from the catalog
//! alone: every card or token a text names is in that card's `refs`, and every id in `refs` is named
//! in one of the card's two texts. It also proves that every token is named by some card, except The
//! Coin, which §2.1's setup deals rather than a card (R244).
//!
//! What "names" means is written once, here, and the client's renderer (apps/web/src/cards/refs.ts)
//! reads the same rule: a card's name, or its name before a parenthesis ("Call to Chaos (Core
//! Edition)" is named as "Call to Chaos"), alone or with a plural "s", standing as whole words — the
//! characters either side are not letters, digits, an apostrophe or a hyphen, so "CN-Viral" does not
//! name "CN-Virus" and "Mr. Vanilla" is not named by #61's "Vanilla copy".
//!
//! Patch v0.2.0 adds two things to the rule. R381 (B2.8): four Classic cards are named like rules
//! words — #10 Exile, #36 Burn, #57 Echo and #30 Recycle — so a text that says "Exile …" or "Echo 1"
//! names none of them unless its `refs` lists the card (a card's `refs` stays curated). R480: the
//! eight Pancake tokens and the ten AI generated cards reach play only through a pool a card names by
//! its tag ("a random Pancake token", "a random AI generated card"), so a text naming that pool names
//! each of its members for the rule that every token is named by some card.
//!
//! Port of `packages/cards/test/references.test.ts`.

use indexmap::IndexSet;
use jackioh_cards::CATALOG;
use jackioh_engine::{CardDef, SetName, Tag};

fn entries() -> Vec<&'static CardDef> {
    CATALOG.values().collect()
}

/// §2.1, R244: dealt by a rule, so no card's text names it — The Coin, and Glitch, which R673's roll deals.
const DEALT_BY_A_RULE: &[&str] = &["core-t-coin", "classic-t-glitch"];

/// R381 (B2.8): card names that are also rules words. A text using one names that card only when the
/// card's `refs` lists it: Exile the verb and the pile, Burn at the hand cap, Echo the keyword,
/// Recycle the verb.
const RULES_WORDS: &[&str] = &["Exile", "Burn", "Echo", "Recycle"];

/// R480: the token pools a card names by tag, and the phrase that names each.
struct TokenPool {
    tag: Tag,
    phrase: &'static str,
}

const TOKEN_POOLS: &[TokenPool] = &[
    TokenPool {
        tag: Tag::Pancake,
        phrase: "Pancake token",
    },
    TokenPool {
        tag: Tag::Ai,
        phrase: "AI generated card",
    },
    // Balance patch 1 reworded C+ #78 to "AI Generated card": the pool is named in either case.
    TokenPool {
        tag: Tag::Ai,
        phrase: "AI Generated card",
    },
    // Balance patch 1 hid the odds: "Grape" names the five Grapes (C+ #65, C+ #66).
    TokenPool {
        tag: Tag::Fruit,
        phrase: "Grape",
    },
    // Balance patch 1 hid the reward lists: "a random reward" names KY's Gift (C+ #42).
    TokenPool {
        tag: Tag::Ky,
        phrase: "random reward",
    },
];

/// `name.replace(/\s*\(.*\)\s*$/, "")`: the name with a closing parenthetical, and the space before
/// it, cut off. The leftmost `(` whose tail runs to a `)` and only spaces after it is where the cut
/// starts (`.*` is greedy and the match is leftmost), together with the spaces just before it.
fn without_parenthetical(name: &str) -> String {
    // Names hold no line break, so `.` reaches every character between the brackets.
    if !name.trim_end().ends_with(')') {
        return name.to_string();
    }
    match name.find('(') {
        Some(open) => name[..open].trim_end().to_string(),
        None => name.to_string(),
    }
}

/// The names a text may call a card by: its name, and its name before a parenthesis.
fn names_of(def: &CardDef) -> Vec<String> {
    let bare = without_parenthetical(&def.name);
    if bare == def.name {
        vec![def.name.clone()]
    } else {
        vec![def.name.clone(), bare]
    }
}

/// `/[A-Za-z0-9'-]/`: a character that continues a word.
fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '\'' || c == '-'
}

/// Whether `text` names `name` as whole words, alone or plural.
fn names(text: &str, name: &str) -> bool {
    let mut from = 0;
    loop {
        let Some(offset) = text[from..].find(name) else {
            return false;
        };
        let at = from + offset;
        let before = text[..at].chars().next_back();
        let mut end = at + name.len();
        if text[end..].starts_with('s') {
            end += 1;
        }
        let after = text[end..].chars().next();
        if before.is_none_or(|c| !is_word(c)) && after.is_none_or(|c| !is_word(c)) {
            return true;
        }
        // `from = at + 1`, kept on a character boundary.
        from = at + text[at..].chars().next().map_or(1, char::len_utf8);
    }
}

/// Every catalog id a card's base or Radiant text names, in catalog order.
fn named_by(card: &CardDef) -> Vec<String> {
    let texts = [&card.base.text, &card.radiant.text];
    entries()
        .into_iter()
        .filter(|other| {
            names_of(other)
                .iter()
                .any(|name| texts.iter().any(|text| names(text, name)))
        })
        .map(|other| other.id.clone())
        .collect()
}

/// Whether an id names a card whose name is a rules word (R381).
fn is_rules_word(id: &str) -> bool {
    CATALOG
        .get(id)
        .is_some_and(|card| RULES_WORDS.contains(&card.name.as_str()))
}

/// The ids of the cards whose base or Radiant text contains `phrase` (TS `naming`).
fn naming(phrase: &str) -> Vec<String> {
    entries()
        .into_iter()
        .filter(|card| {
            [&card.base.text, &card.radiant.text]
                .iter()
                .any(|text| text.contains(phrase))
        })
        .map(|card| card.id.clone())
        .collect()
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

mod r279_the_reference_map_spec_5_7_10_10 {
    use super::*;

    #[test]
    fn r279_lists_in_every_card_s_refs_exactly_the_cards_and_tokens_its_texts_name() {
        let mut wrong: Vec<String> = Vec::new();
        for card in entries() {
            let named = named_by(card);
            let mut listed: Vec<String> = card.refs.clone().unwrap_or_default();
            listed.sort();
            // A rules-word name is required of no text (R381), and may be listed where a text means it.
            let mut expected: Vec<String> = named
                .into_iter()
                .filter(|id| !is_rules_word(id) || listed.contains(id))
                .collect();
            expected.sort();
            if expected != listed {
                wrong.push(format!(
                    "{} {}: texts name [{}], refs list [{}]",
                    card.id,
                    card.name,
                    expected.join(", "),
                    listed.join(", ")
                ));
            }
        }
        assert_eq!(wrong, Vec::<String>::new());
    }

    #[test]
    fn r279_has_every_token_named_by_at_least_one_card_except_what_a_rule_deals_the_coin() {
        let mut named: IndexSet<String> = entries()
            .into_iter()
            .flat_map(|card| card.refs.clone().unwrap_or_default())
            .collect();
        // R480: a card whose text names a token pool by its tag names every member of it.
        for pool in TOKEN_POOLS {
            let named_by_tag = entries().into_iter().any(|card| {
                [&card.base.text, &card.radiant.text]
                    .iter()
                    .any(|text| text.contains(pool.phrase))
            });
            if !named_by_tag {
                continue;
            }
            for member in entries()
                .into_iter()
                .filter(|card| card.token && card.tags.contains(&pool.tag))
            {
                named.insert(member.id.clone());
            }
        }
        let unnamed: Vec<String> = entries()
            .into_iter()
            .filter(|card| card.token && !named.contains(&card.id))
            .map(|card| card.id.clone())
            .filter(|id| !DEALT_BY_A_RULE.contains(&id.as_str()))
            .collect();
        assert_eq!(unnamed, Vec::<String>::new());
    }

    #[test]
    fn r480_names_the_eight_pancake_tokens_and_the_ten_ai_generated_cards_by_their_tags_from_the_cards_that_make_them()
     {
        let indices_of = |tag: Tag| -> Vec<String> {
            entries()
                .into_iter()
                .filter(|card| card.token && card.tags.contains(&tag))
                .map(|card| card.index.clone())
                .collect()
        };
        assert_eq!(
            indices_of(Tag::Pancake),
            strings(&["12.1", "12.2", "12.3", "12.4", "12.5", "12.6", "12.7", "12.8"])
        );
        assert_eq!(
            indices_of(Tag::Ai),
            strings(&[
                "T-AI-1", "T-AI-2", "T-AI-3", "T-AI-4", "T-AI-5", "T-AI-6", "T-AI-7", "T-AI-8", "T-AI-9",
                "T-AI-10",
            ])
        );
        assert_eq!(
            naming("Pancake token"),
            strings(&["classicplus-012", "classicplus-013"])
        );
        // AI Slop and Claude's Datacenter make them; Scaling Law counts them.
        assert_eq!(
            naming("AI generated card"),
            strings(&["classicplus-043", "classicplus-t-ai-02"])
        );
        // Balance patch 1 reworded Claude's Datacenter to "AI Generated card".
        assert_eq!(naming("AI Generated card"), strings(&["classicplus-078"]));
        assert_eq!(
            indices_of(Tag::Fruit),
            strings(&["65.1", "65.2", "65.3", "65.4", "65.5"])
        );
        // Balance patch 1 hid the odds and the reward lists: "Grape" names the five
        // Grapes, and "a random reward" names KY's Gift.
        assert_eq!(naming("Grape"), strings(&["classicplus-065", "classicplus-066"]));
        assert_eq!(naming("random reward"), strings(&["classicplus-042"]));
    }

    #[test]
    fn r381_reads_exile_burn_echo_and_recycle_as_rules_words_a_text_that_says_one_names_no_card_unless_its_refs_list_it()
     {
        for word in RULES_WORDS {
            let card = entries().into_iter().find(|entry| entry.name == *word);
            assert_eq!(
                card.map(|card| card.set),
                Some(SetName::Classic),
                "{word} is a Classic card"
            );
        }
        // They are all over the texts as rules words — and no card lists them.
        let uses_exile = entries()
            .into_iter()
            .filter(|card| names(&card.base.text, "Exile") || names(&card.radiant.text, "Exile"))
            .count();
        let uses_echo = entries()
            .into_iter()
            .filter(|card| names(&card.base.text, "Echo") || names(&card.radiant.text, "Echo"))
            .count();
        assert!(uses_exile > 10);
        assert!(uses_echo > 2);
        let listing: Vec<String> = entries()
            .into_iter()
            .filter(|card| card.refs.iter().flatten().any(|id| is_rules_word(id)))
            .map(|card| card.id.clone())
            .collect();
        assert_eq!(listing, Vec::<String>::new());
        // "Recycler" is a longer word, so Malzahar's Recycler never names Recycle at all.
        assert!(!names("Malzahar's Recycler", "Recycle"));
    }

    #[test]
    fn r279_names_a_card_by_its_name_before_a_parenthesis_and_never_by_a_longer_word() {
        let chaos = CATALOG.get("core-095");
        assert!(chaos.is_some_and(|card| card.refs.iter().flatten().any(|id| id == "core-095")));
        assert!(!names("Shuffle a CN-Viral Injection", "CN-Virus"));
        assert!(!names("summon a Vanilla copy", "Mr. Vanilla"));
        assert!(names("fill your board with Rush Tokens; if", "Rush Token"));
        assert!(names(
            "your units other than Spikey Pillows have",
            "Spikey Pillow"
        ));
    }

    #[test]
    fn r279_keeps_an_empty_list_out_of_the_catalog_a_card_that_names_nothing_has_no_refs() {
        let empty: Vec<String> = entries()
            .into_iter()
            .filter(|card| card.refs.as_ref().is_some_and(Vec::is_empty))
            .map(|card| card.id.clone())
            .collect();
        assert_eq!(empty, Vec::<String>::new());
    }

    #[test]
    fn the_parenthetical_cut_reads_as_its_regular_expression() {
        assert_eq!(
            without_parenthetical("Call to Chaos (Core Edition)"),
            "Call to Chaos"
        );
        assert_eq!(
            without_parenthetical("Call to Chaos (Classic+ Edition)"),
            "Call to Chaos"
        );
        assert_eq!(without_parenthetical("Mr. Vanilla"), "Mr. Vanilla");
        assert_eq!(without_parenthetical("A (b) c"), "A (b) c");
    }
}
