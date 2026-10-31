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
//! name "CN-Virus" and "Mr. Vanilla" is not named by #61's "Vanilla copy". R903 adds one alias:
//! "Coin" names The Coin.
//!
//! Patch v0.2.0 adds two things to the rule. R381 (B2.8): four Classic cards are named like rules
//! words — #10 Exile, #36 Burn, #57 Echo and #30 Recycle — so a text that says "Exile …" or "Echo 1"
//! names none of them unless its `refs` lists the card (a card's `refs` stays curated). R480: the
//! eight Pancake tokens and the AI generated cards (the ten T-AI cards, and since v0.3.15 Meditative+
//! #1.1 to #1.4) reach play through a pool a card names by its tag ("a random Pancake token", "a
//! random AI generated card"), so a text naming that pool names each of its members for the rule that
//! every token is named by some card. R1857002 adds the Stinkyscape set's two such pools, its
//! Terminus tokens ("a Terminus card") and its War Doctrines ("a War Doctrine"), and its Resources,
//! which R1840002's roll deals as §2.1's setup deals The Coin, so no card need name one.
//!
//! Port of `packages/cards/test/references.test.ts`.

use indexmap::IndexSet;
use jackioh_cards::CATALOG;
use jackioh_engine::{CardDef, SetName, Tag, set_ships};

fn entries() -> Vec<&'static CardDef> {
    CATALOG.values().collect()
}

/// §2.1, R244: dealt by a rule, so no card's text names it — The Coin, and Glitch, which R673's roll deals.
const DEALT_BY_A_RULE: &[&str] = &[
    "core-t-coin",
    "classic-t-glitch",
    // Summoned by the Jade Counter crossing 5 or 10, never named by a card (R962).
    "meditative-039-5",
];

/// R244, R1857002: whether a rule deals this token, so no card need name it: one of `DEALT_BY_A_RULE`,
/// or a Resource (`resource`), which R1840002's roll deals whichever card or rule asks for one.
fn dealt_by_a_rule(card: &CardDef) -> bool {
    DEALT_BY_A_RULE.contains(&card.id.as_str()) || card.resource.is_some()
}

/// R1420: tokens of a set that has not shipped, landed before the card that names them, by that card's
/// id. Its part empties the entry: the test fails once the maker is in the catalog (Meditative #97
/// Jlockeed's Evil Blueprints, issue #542, names the nine buildings; issue #541 lands five first).
const MAKER_STILL_TO_COME: &[(&str, &[&str])] = &[];

/// R381 (B2.8): card names that are also rules words. A text using one names that card only when the
/// card's `refs` lists it: Exile the verb and the pile, Burn at the hand cap, Echo the keyword,
/// Recycle the verb.
const RULES_WORDS: &[&str] = &["Exile", "Burn", "Echo", "Recycle"];

/// R961: rules phrases that hold a card's name without naming it (R381): "Jade Counter" holds
/// "Jade" (Meditative #39.2), which names nothing inside it. R2198004: "Fortify Mind" (M #22.1)
/// holds "Fortify" (TG #12), and still names itself.
const RULES_PHRASES: &[&str] = &["Jade Counter", "Fortify Mind"];

/// Whether the match at byte range `at..end` lies inside a rules phrase: the phrase holds the
/// name, so the name reads as the phrase's word, not the card's. A match that is the whole phrase
/// is not inside it (R2198004: the phrase may be a card's own name). Both bounds sit on the match's
/// own edges, so every byte index here is a character boundary.
fn inside_rules_phrase(text: &str, at: usize, end: usize) -> bool {
    RULES_PHRASES.iter().any(|phrase| {
        let mut from = 0;
        loop {
            let Some(offset) = text[from..].find(phrase) else {
                return false;
            };
            let start = from + offset;
            if start <= at && end <= start + phrase.len() && (at, end) != (start, start + phrase.len()) {
                return true;
            }
            from = start + 1;
        }
    })
}

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
    // Balance patch 1 reworded C+ #78 to "AI Generated card"; #911 put it back in the pool's one
    // spelling, with Meditative #59's granted Death.
    TokenPool {
        tag: Tag::Ai,
        phrase: "AI generated card",
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
    // R1857002: the Stinkyscape set's #4 Olivian Expanse and #154 Dice of Doom name its six
    // Terminus tokens as "a Terminus card", and #137 its four War Doctrines as "a War Doctrine".
    TokenPool {
        tag: Tag::Terminus,
        phrase: "Terminus card",
    },
    TokenPool {
        tag: Tag::WarDoctrine,
        phrase: "War Doctrine",
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

/// R903: one named alias — "Coin" names The Coin, which M #27's texts call by the short name.
const ALIASES: &[(&str, &str)] = &[("The Coin", "Coin")];

/// The names a text may call a card by: its name, and its name before a parenthesis.
fn names_of(def: &CardDef) -> Vec<String> {
    let bare = without_parenthetical(&def.name);
    let mut out = if bare == def.name {
        vec![def.name.clone()]
    } else {
        vec![def.name.clone(), bare]
    };
    for (name, alias) in ALIASES {
        if *name == def.name {
            out.push(alias.to_string());
        }
    }
    out
}

/// `/[A-Za-z0-9'-]/`: a character that continues a word.
fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '\'' || c == '-'
}

/// Whether `text` names `name` as whole words, alone or plural — never from inside a rules
/// phrase (R961: the "Jade" in "Jade Counter" names no card).
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
        if before.is_none_or(|c| !is_word(c))
            && after.is_none_or(|c| !is_word(c))
            && !inside_rules_phrase(text, at, end)
        {
            return true;
        }
        // `from = at + 1`, kept on a character boundary.
        from = at + text[at..].chars().next().map_or(1, char::len_utf8);
    }
}

/// Every catalog id a card's base or Radiant text names, in catalog order. R1249: a card of a set that
/// ships names no card of a set that has not (R1420), so its shipped `refs` stay as they are until
/// that set ships ("Call to Chaos" on Core #95 and C+ #73 reached Meditative #95 when it shipped, in
/// v0.3.5). R2195002, R2511000: a Core card's name names the Core card alone (S #88 Siva 2's Bigots, C+
/// #61's Stockpile, Core #76's Reminisce), since its Legacy card is named `Legacy: <name>`.
fn named_by(card: &CardDef) -> Vec<String> {
    let texts = [&card.base.text, &card.radiant.text];
    entries()
        .into_iter()
        .filter(|other| !set_ships(card.set) || set_ships(other.set))
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

/// R1420, R1857000: the ids among `ids` of cards whose set ships, so a pin of the shipped sets' cards
/// holds while a held set's cards land (the release re-pins it).
fn shipped(ids: Vec<String>) -> Vec<String> {
    ids.into_iter()
        .filter(|id| CATALOG.get(id.as_str()).is_some_and(|card| set_ships(card.set)))
        .collect()
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
    fn r2195002_r2511000_a_core_card_s_name_names_the_core_card_and_not_its_legacy_card() {
        for (card, core, legacy) in [
            ("stinkyscape-088", "core-002", "traininggrounds-041"),
            ("classicplus-061", "core-005", "traininggrounds-042"),
            ("core-076", "core-072", "traininggrounds-055"),
        ] {
            assert_eq!(CATALOG[legacy].name, format!("Legacy: {}", CATALOG[core].name));
            let named = named_by(&CATALOG[card]);
            assert!(
                named.contains(&core.to_string()),
                "{card} names {core}: {named:?}"
            );
            assert!(
                !named.contains(&legacy.to_string()),
                "{card} names {legacy}: {named:?}"
            );
        }
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
        for (maker, tokens) in MAKER_STILL_TO_COME {
            assert!(
                !CATALOG.contains_key(*maker),
                "{maker} is in the catalog: take its tokens off MAKER_STILL_TO_COME"
            );
            for token in *tokens {
                let card = CATALOG
                    .get(*token)
                    .expect("a token waiting for its maker is in the catalog");
                assert!(card.token, "{token} is not a token");
                assert!(
                    !set_ships(card.set),
                    "{token} is in a shipped set: its maker must exist"
                );
                named.insert(token.to_string());
            }
        }
        let unnamed: Vec<String> = entries()
            .into_iter()
            .filter(|card| card.token && !named.contains(&card.id) && !dealt_by_a_rule(card))
            .map(|card| card.id.clone())
            .collect();
        assert_eq!(unnamed, Vec::<String>::new());
    }

    #[test]
    fn r480_names_the_eight_pancake_tokens_and_the_twenty_five_ai_generated_cards_by_their_tags_from_the_cards_that_make_them()
     {
        // R1178002: Meditative+ #1.1 Base Camp and #1.2 Spa Day, AI generated cards Bob's Plan names,
        // join the AI pool once their set ships (#1187), so the pool pinned here is the shipped sets'.
        // Re-pinned for v0.3.15 (Meditative+ ships, #1187): the AI generated cards are twelve, the ten
        // T-AI cards and M+ #1.1 and #1.2 (§7, R1421); fourteen since #1563 added M+ #1.3 and #1.4 (R1563003);
        // twenty-four since the Stinkyscape release (#1884) added that set's T-AI-1 to T-AI-10, which
        // #1853's Marketplace offers by its own rule, so no card of the set names them; twenty-five since
        // #2521 added M+ #1.5 Star Chart, Bob's Plan's plan for Alteran Reach.
        let indices_of = |tag: Tag| -> Vec<String> {
            entries()
                .into_iter()
                .filter(|card| card.token && card.tags.contains(&tag) && set_ships(card.set))
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
                "T-AI-10", "1.1", "1.2", "1.3", "1.4", "1.5", "T-AI-1", "T-AI-2", "T-AI-3", "T-AI-4",
                "T-AI-5", "T-AI-6", "T-AI-7", "T-AI-8", "T-AI-9", "T-AI-10",
            ])
        );
        // The pins are the shipped sets' cards (R1857000: a held set's are pinned at its release).
        assert_eq!(
            shipped(naming("Pancake token")),
            strings(&["classicplus-012", "classicplus-013"])
        );
        // AI Slop and Claude's Datacenter make them; Scaling Law counts them. Meditative #59's granted
        // Death fuses one and #60 Eschews casts one (§8.8 rows 59 and 60, R1421); #53 Prestige added one
        // until #831's patch. Issue #911 put Claude's Datacenter's "AI Generated card" back in the
        // pool's one spelling.
        assert_eq!(
            shipped(naming("AI generated card")),
            strings(&[
                "classicplus-043",
                "classicplus-078",
                "classicplus-t-ai-02",
                "meditative-059",
                "meditative-060"
            ])
        );
        assert_eq!(naming("AI Generated card"), Vec::<String>::new());
        assert_eq!(
            indices_of(Tag::Fruit),
            strings(&["65.1", "65.2", "65.3", "65.4", "65.5"])
        );
        // Balance patch 1 hid the odds and the reward lists: "Grape" names the five
        // Grapes, and "a random reward" names KY's Gift.
        assert_eq!(
            shipped(naming("Grape")),
            strings(&["classicplus-065", "classicplus-066"])
        );
        assert_eq!(shipped(naming("random reward")), strings(&["classicplus-042"]));
    }

    #[test]
    fn r1857002_terminus_cards_war_doctrines_and_resources_are_named_by_their_pools_and_their_roll() {
        let phrase = |tag: Tag| {
            TOKEN_POOLS
                .iter()
                .find(|pool| pool.tag == tag)
                .map(|pool| pool.phrase)
        };
        assert_eq!(phrase(Tag::Terminus), Some("Terminus card"));
        assert_eq!(phrase(Tag::WarDoctrine), Some("War Doctrine"));
        // Since the release (#1884) the set's makers are shipped cards: #4 Olivian Expanse and #154
        // Dice of Doom name the Terminus pool and #137 Starfleet Commander and #176 Preemptive Strike
        // (#2521) the War Doctrines (Core #100 Ceaseless Void, the other Terminus card, names none and
        // is no token).
        assert_eq!(
            shipped(naming("Terminus card")),
            strings(&["stinkyscape-004", "stinkyscape-154"])
        );
        assert_eq!(
            shipped(naming("War Doctrine")),
            strings(&["stinkyscape-137", "stinkyscape-176"])
        );
        // A Resource is dealt by R1840002's roll, as The Coin by §2.1's setup: it needs no card to
        // name it. A plain token still does.
        let resource: CardDef = serde_json::from_value(serde_json::json!({
            "id": "stinkyscape-t-reknite", "index": "T-reknite", "name": "Reknite", "set": "Stinkyscape",
            "type": "Spell", "tags": ["Token"], "rarity": "Token", "token": true, "cost": 0,
            "resource": {
                "rarity": "Common",
                "base": { "sellMin": 1, "sellMax": 3, "buy": 3 },
                "radiant": { "sellMin": 2, "sellMax": 6, "buy": 6 }
            },
            "base": { "keywords": [], "text": "Draw a card." },
            "radiant": { "keywords": [], "text": "Draw 2 cards." }
        }))
        .expect("a Resource definition");
        assert!(dealt_by_a_rule(&resource));
        assert!(dealt_by_a_rule(&CATALOG["core-t-coin"]));
        assert!(!dealt_by_a_rule(&CATALOG["core-t-sheep"]));
        // Every Stinkyscape Resource the catalog holds yet is a token; and the makers that have landed
        // name their pools: #4 Olivian Expanse and #154 Dice of Doom a Terminus card, #137 a War Doctrine.
        for card in entries().into_iter().filter(|card| card.resource.is_some()) {
            assert!(card.token, "{} is a Resource and a token (R1840000)", card.id);
        }
        for (maker, pool) in [
            ("stinkyscape-004", "Terminus card"),
            ("stinkyscape-154", "Terminus card"),
            ("stinkyscape-137", "War Doctrine"),
        ] {
            if CATALOG.contains_key(maker) {
                assert!(naming(pool).contains(&maker.to_string()), "{maker} names {pool}");
            }
        }
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
    fn r961_reads_jade_counter_as_a_rules_word_the_jade_inside_names_no_card() {
        assert!(!names(
            "Add {jade} to your Jade Counter. Gain {mana} mana.",
            "Jade"
        ));
        assert!(!names(
            "You summon this when your Jade Counter reaches 5.",
            "Jade"
        ));
        // Outside the phrase it still names Jade (Meditative #39.2).
        assert!(names("Your Jade Beauties become Radiant.", "Jade"));
        assert!(names(
            "Add one of these: Dud (20%), Jade (75%) or Red Jade (5%).",
            "Jade"
        ));
        // So neither Jade nor Red Jade lists a `refs` for the Counter alone.
        for id in ["meditative-039-2", "meditative-039-4"] {
            assert_eq!(
                CATALOG
                    .get(id)
                    .and_then(|card| card.refs.clone())
                    .unwrap_or_default(),
                Vec::<String>::new(),
                "{id} names no card"
            );
        }
    }

    #[test]
    fn r2198004_fortify_mind_names_itself_and_not_fortify() {
        let text = "Add a Fortify Mind to your opponent's hand.";
        assert!(names(text, "Fortify Mind"));
        assert!(!names(text, "Fortify"));
        assert!(names("Add 2 Fortify Minds.", "Fortify Mind"));
        // Outside the phrase "Fortify" still names Training Grounds #12.
        assert!(names("Cast a Fortify.", "Fortify"));
    }

    #[test]
    fn r1249_the_patch_that_ships_meditative_adds_its_95_to_the_refs_of_core_95_and_classic_plus_73() {
        let card = |id: &str| CATALOG.get(id).unwrap_or_else(|| panic!("{id} in the catalog"));
        let meditative = card("meditative-095");
        // Core #95 and C+ #73 say "Call to Chaos", which names Meditative #95 once its set ships: since
        // v0.3.5, whose fragment claims both cards' refs (R646). A later set held back (R1420) is named
        // by no shipped card's refs the same way until it ships, which `named_by` reads.
        assert!(set_ships(meditative.set));
        for shipped in ["core-095", "classicplus-073"] {
            let named = named_by(card(shipped));
            assert!(
                named.contains(&shipped.to_string()),
                "{shipped} names its own pool"
            );
            assert!(
                named.contains(&meditative.id),
                "{shipped} names Meditative #95, whose set ships"
            );
            assert!(
                card(shipped).refs.iter().flatten().any(|id| *id == meditative.id),
                "{shipped}'s refs list Meditative #95"
            );
        }
        // Meditative #95 names every card its texts name, of any set.
        let named = named_by(meditative);
        for id in [
            "core-095",
            "classicplus-073",
            "meditative-095",
            "meditative-095-1",
        ] {
            assert!(named.contains(&id.to_string()), "meditative-095 names {id}");
        }
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
    fn r903_coin_and_coins_name_the_coin_and_a_lower_case_coin_does_not() {
        let coin = CATALOG.get("core-t-coin").expect("The Coin in the catalog");
        assert_eq!(coin.name, "The Coin");
        assert!(names_of(coin).contains(&"Coin".to_string()));
        // Alone or plural, by the full name or the alias.
        assert!(names("add a Coin to your hand", "Coin"));
        assert!(names("add Coins to your hand", "Coin"));
        assert!(names("shuffle The Coin into each player's deck", "The Coin"));
        // Case matters: "flip a coin" names nothing.
        assert!(!names("flip a coin", "Coin"));
        assert!(!names("flip a coin", "The Coin"));
        // No other card is named without its "The": every other name stands alone.
        for card in entries() {
            let bare = without_parenthetical(&card.name);
            let mut expected = vec![card.name.clone()];
            if bare != card.name {
                expected.push(bare);
            }
            if card.id == "core-t-coin" {
                expected.push("Coin".to_string());
            }
            assert_eq!(names_of(card), expected, "{}", card.id);
        }
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
