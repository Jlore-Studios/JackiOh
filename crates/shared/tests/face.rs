//! The face model (`src/face/`, #3106): a card's face as the web's tests held it before the model moved here,
//! in both locales, through the one function behind the `face` body (`apply`), as a host calls it: JSON in,
//! JSON out. A face reads as it always has; these tests are what says so.

use jackioh_engine::catalog::registered_catalog;
use jackioh_engine::wire::PlayerId;
use jackioh_shared::face::{FaceAnswer, FaceCall, FaceModel, Locale, Span, apply};
use serde_json::{Value, json};

fn ready() {
    jackioh_cards::register_all();
}

fn call(request: Value) -> FaceAnswer {
    let request: FaceCall = serde_json::from_value(request).expect("a FaceCall");
    apply(&request)
}

fn face_of(answer: FaceAnswer) -> FaceModel {
    match answer {
        FaceAnswer::Face { face } => *face,
        other => panic!("expected a face, got {other:?}"),
    }
}

fn locale_name(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "en",
        Locale::ZhCn => "zh-CN",
    }
}

/// A card as printed, the way the collection asks for it.
fn printed(id: &str, radiant: bool, locale: Locale) -> FaceModel {
    ready();
    face_of(call(
        json!({ "call": "card", "locale": locale_name(locale), "source": { "defId": id, "radiant": radiant } }),
    ))
}

/// A card in a game: `in_play` is what the view says of it.
fn in_play(id: &str, radiant: bool, locale: Locale, extra: Value) -> FaceModel {
    ready();
    let mut source = json!({ "defId": id, "radiant": radiant, "inPlay": {} });
    for (key, value) in extra.as_object().expect("an object") {
        if key == "inPlay" {
            source["inPlay"] = value.clone();
        } else {
            source[key] = value.clone();
        }
    }
    face_of(call(
        json!({ "call": "card", "locale": locale_name(locale), "source": source }),
    ))
}

/// The plain words of spans: every kind's text, nested.
fn words(spans: &[Span]) -> String {
    spans
        .iter()
        .map(|span| match span {
            Span::Text { text } | Span::Number { text, .. } | Span::Value { text, .. } => text.clone(),
            Span::Keyword { children, .. } | Span::Card { children, .. } | Span::Mark { children } => {
                words(children)
            }
        })
        .collect()
}

/// What stands inside the first `mark` span of each top-level mark, in order.
fn marked(spans: &[Span]) -> Vec<String> {
    let mut out = Vec::new();
    for span in spans {
        match span {
            Span::Mark { children } => out.push(words(children)),
            Span::Keyword { children, .. } | Span::Card { children, .. } => out.extend(marked(children)),
            _ => {}
        }
    }
    out
}

#[test]
fn a_unit_as_printed_has_its_name_type_stats_keywords_and_text() {
    let face = printed("core-020", false, Locale::En);
    assert_eq!(
        (face.name.as_str(), face.type_line.as_str(), face.known),
        ("Pointmaster", "Unit", true)
    );
    // The Stinkyscape balance pass (#3125) gave Core #20 Rush and one attack less.
    assert_eq!(face.text.full, "Rush, First Strike");
    let terms: Vec<&str> = flatten(&face.text.spans)
        .into_iter()
        .filter_map(|span| match span {
            Span::Keyword { entry, .. } => Some(entry.id.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(terms, vec!["Rush", "First Strike"]);
    let stats = face.stats.expect("a unit has stats");
    assert_eq!((stats.attack, stats.health, stats.max_health), (6, 1, 1));
    assert!(!face.in_play && face.printed.is_none() && face.badges.is_empty());
}

#[test]
fn a_radiant_face_marks_what_the_base_face_lacks() {
    let face = printed("core-011", true, Locale::En);
    assert_eq!(face.text.full, "Charge, First Strike");
    assert_eq!(marked(&face.text.spans), vec!["Charge"]);
    assert_eq!(
        face.stats
            .expect("stats")
            .grew
            .map(|grew| (grew.attack, grew.health)),
        Some((true, true))
    );
}

#[test]
fn every_catalog_card_has_a_face_in_both_locales_whose_spans_say_its_text() {
    ready();
    for (id, def) in registered_catalog() {
        // Glitch's words are corrupted wherever a lookup answers for it (`face/glitch.rs`).
        if id == jackioh_engine::config::GLITCH_DEF_ID {
            continue;
        }
        for radiant in [false, true] {
            for locale in [Locale::En, Locale::ZhCn] {
                let face = printed(id, radiant, locale);
                assert_eq!(
                    words(&face.text.spans),
                    face.text.full,
                    "{id} {radiant} {locale:?}"
                );
                assert!(face.known && !face.name.is_empty(), "{id}");
                if locale == Locale::En {
                    let written = if radiant {
                        &def.radiant.text
                    } else {
                        &def.base.text
                    };
                    let filled = jackioh_engine::wire::fill_params(
                        def,
                        if radiant {
                            jackioh_engine::wire::FaceKind::Radiant
                        } else {
                            jackioh_engine::wire::FaceKind::Base
                        },
                        None,
                    );
                    assert_eq!(face.text.full, filled, "{id}: {written}");
                    assert_eq!(face.name, def.name, "{id}");
                }
            }
        }
    }
}

#[test]
fn a_reader_of_chinese_reads_every_card_from_the_chinese_table() {
    let face = printed("core-001", false, Locale::ZhCn);
    assert_eq!(face.name, "大D守卫");
    assert_eq!(face.chinese, Some(true));
    assert_eq!(face.english_name.as_deref(), Some("Big D-fender"));
    assert_eq!(face.type_line, "单位");
    // The English face rides along, never printed: the glossary finds its terms there.
    assert_eq!(
        face.english_text.as_deref(),
        Some("Aura: Your Units in Defense Position have +2 Armor.")
    );
    assert_eq!(
        face.glossary
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        vec!["Aura", "Armor"]
    );
    assert_eq!(face.glossary[0].label, "光环");
}

#[test]
fn a_card_the_view_flags_chinese_is_chinese_for_a_reader_of_english() {
    let face = in_play("core-001", false, Locale::En, json!({ "chinese": true }));
    assert_eq!(face.name, "大D守卫");
    assert_eq!(face.flagged, Some(true));
    // The reader's own words around it stay English.
    let vanilla = in_play(
        "core-001",
        false,
        Locale::En,
        json!({ "chinese": true, "inPlay": { "vanilla": true } }),
    );
    assert_eq!(vanilla.text.full, "Vanilla: its text is gone");
}

#[test]
fn a_cost_reads_against_the_printed_one() {
    let down = in_play("core-001", false, Locale::En, json!({ "liveCost": 1 }));
    assert_eq!((down.cost.text.as_str(), down.cost.value.as_str()), ("1", "1"));
    assert_eq!(serde_json::to_value(down.cost.tone).unwrap(), "down");
    let up = in_play("core-001", false, Locale::En, json!({ "liveCost": 3 }));
    assert_eq!(serde_json::to_value(up.cost.tone).unwrap(), "up");
    let printed = printed("core-001", false, Locale::En);
    assert_eq!(serde_json::to_value(printed.cost.tone).unwrap(), "base");
    assert_eq!(printed.cost.alt, None);
}

#[test]
fn a_card_nobody_can_name_is_drawn_by_its_id() {
    let face = printed("core-999", false, Locale::En);
    assert!(!face.known);
    assert_eq!(
        (
            face.name.as_str(),
            face.cost.text.as_str(),
            face.rarity,
            face.index
        ),
        ("core-999", "?", None, None)
    );
}

#[test]
fn a_heroic_power_prints_only_the_power_it_rolled() {
    let face = in_play(
        "core-098",
        false,
        Locale::En,
        json!({ "inPlay": { "power": { "name": "recruit" } } }),
    );
    assert_eq!(
        face.text.full,
        "Indestructible\nExpedition Map\nActivate: Spend (3): Recruit a permanent."
    );
    assert!(face.printed.is_some(), "the collection's text is shown beside it");
    let radiant = in_play(
        "core-098",
        true,
        Locale::En,
        json!({ "inPlay": { "power": { "name": "recruit" } } }),
    );
    assert_eq!(marked(&radiant.text.spans), vec!["Make it Radiant"]);
}

#[test]
fn call_to_chaos_reads_question_marks_in_play_and_keeps_its_text_in_the_collection() {
    let hidden = in_play("core-095", false, Locale::En, json!({}));
    assert_eq!(hidden.text.full, "???");
    assert!(hidden.printed.is_none());
    assert_ne!(printed("core-095", false, Locale::En).text.full, "???");
    assert_eq!(
        in_play("classicplus-073", true, Locale::En, json!({})).text.full,
        "!!!"
    );
}

#[test]
fn a_vanilla_unit_says_its_text_is_gone_and_prints_its_keywords_as_gained() {
    let face = in_play(
        "core-020",
        false,
        Locale::En,
        json!({ "live": { "attack": 7, "health": 1, "maxHealth": 1, "keywords": [{ "kind": "First Strike" }] }, "inPlay": { "vanilla": true } }),
    );
    assert_eq!(face.text.full, "Vanilla: its text is gone");
    assert!(face.vanilla);
    assert_eq!(face.gained_line.full, "First Strike");
    assert_eq!(
        in_play(
            "core-020",
            false,
            Locale::ZhCn,
            json!({ "inPlay": { "vanilla": true } })
        )
        .text
        .full,
        "白板化：它的文字已消失"
    );
}

#[test]
fn a_number_the_view_moved_is_a_span_that_says_which_way() {
    ready();
    let (id, def) = registered_catalog()
        .iter()
        .find(|(_, def)| {
            def.params.as_ref().is_some_and(|params| !params.is_empty()) && def.base.text.contains('{')
        })
        .expect("a card with a declared number");
    let param = &def.params.as_ref().expect("params")[0];
    let face = in_play(
        id,
        false,
        Locale::En,
        json!({ "inPlay": { "params": { param.key.clone(): param.base + 1 } } }),
    );
    let numbers: Vec<&Span> = flatten(&face.text.spans)
        .into_iter()
        .filter(|span| matches!(span, Span::Number { .. }))
        .collect();
    assert_eq!(numbers.len(), 1, "{id}");
    assert!(matches!(numbers[0], Span::Number { printed, .. } if *printed == param.base));
    assert!(face.tuning.is_some());
}

fn flatten(spans: &[Span]) -> Vec<&Span> {
    let mut out = Vec::new();
    for span in spans {
        out.push(span);
        if let Span::Keyword { children, .. } | Span::Card { children, .. } | Span::Mark { children } = span {
            out.extend(flatten(children));
        }
    }
    out
}

#[test]
fn a_formula_value_prints_after_its_label() {
    let face = in_play(
        "core-020",
        false,
        Locale::En,
        json!({ "inPlay": { "preview": [{ "label": "First Strike", "value": 3 }] } }),
    );
    let value = flatten(&face.text.spans)
        .into_iter()
        .find(|span| matches!(span, Span::Value { .. }))
        .expect("a value");
    assert!(matches!(value, Span::Value { text, value, .. } if text == "{3}" && value == "3"));
    assert_eq!(face.values.len(), 1);
    assert_eq!(words(&face.text.spans), "Rush, First Strike {3}");
}

#[test]
fn a_dormant_frozen_card_wears_its_badges_in_rail_order() {
    let face = in_play(
        "core-020",
        false,
        Locale::En,
        json!({ "inPlay": { "brittle": 2, "frozen": true, "dormant": { "turns": 2 } } }),
    );
    let kinds: Vec<String> = face
        .badges
        .iter()
        .map(|badge| {
            serde_json::to_value(badge.kind)
                .unwrap()
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    assert_eq!(kinds, vec!["brittle", "frozen", "dormant"]);
    assert_eq!(face.badges[0].words, "Brittle 2: crumbles at 0");
    assert_eq!(face.badges[2].text.as_deref(), Some("\u{263E}\u{FE0E}2"));
    let chinese = in_play(
        "core-020",
        false,
        Locale::ZhCn,
        json!({ "inPlay": { "brittle": 2 } }),
    );
    assert_eq!(chinese.badges[0].words, "易碎2：降至0时碎裂");
    // A badge's states name their glossary terms, after the text's and the keywords'.
    assert_eq!(
        face.glossary
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        vec!["Rush", "First Strike", "Brittle"]
    );
}

#[test]
fn a_copier_prints_the_spell_text_the_view_says_it_has() {
    ready();
    let spell = registered_catalog().get("core-044").expect("True Strike");
    let face = in_play(
        "classic-057",
        false,
        Locale::En,
        json!({ "inPlay": { "copies": { "def": spell, "radiant": false } } }),
    );
    let copied = jackioh_engine::wire::fill_params(spell, jackioh_engine::wire::FaceKind::Base, None);
    assert!(face.text.full.contains(&copied), "{}", face.text.full);
    assert_eq!(
        face.copying.as_ref().map(|copying| copying.def_id.as_str()),
        Some("core-044")
    );
}

#[test]
fn a_quest_card_prints_the_stage_it_stands_at() {
    let face = in_play("stinkyscape-007", false, Locale::En, json!({}));
    let quest = face.quest_box.expect("a quest box");
    assert!(quest.less);
    assert!(
        quest.text.full.contains("Stage 1 of 3, Initiation. Quest: "),
        "{}",
        quest.text.full
    );
}

#[test]
fn the_summit_prints_its_header_and_the_next_ascent_line() {
    let face = in_play(
        "meditative-008",
        false,
        Locale::En,
        json!({ "inPlay": { "preview": [{ "label": "Ascent level", "value": 2 }] } }),
    );
    assert_eq!(face.ascent, Some(2));
    let summit = face.summit.expect("a summit text");
    assert!(
        summit.full.contains("Ascent 2:") && !summit.full.contains("Ascent 1:"),
        "{}",
        summit.full
    );
    assert!(face.has_short_form);
}

#[test]
fn a_text_with_terms_is_spans_with_the_terms_in_bold() {
    let answer = call(json!({ "call": "text", "text": "Cry: Armor 2", "locale": "en" }));
    let FaceAnswer::Rich { text } = answer else {
        panic!("rich text")
    };
    assert!(matches!(&text.spans[0], Span::Keyword { entry, .. } if entry.id == "Cry"));
    assert_eq!(text.full, "Cry: Armor 2");
}

#[test]
fn a_word_diff_counts_in_utf16_units() {
    let FaceAnswer::Diff { added, removed } =
        call(json!({ "call": "wordDiff", "before": "Deal 3 damage.", "after": "Deal 4 damage." }))
    else {
        panic!("a diff")
    };
    assert_eq!((added[0].start, added[0].end, removed[0].start), (5, 6, 5));
}

/// A game, as a host's server would present it: the first seat's view after the game begins.
fn a_view() -> jackioh_engine::wire::PlayerView {
    ready();
    let deck: Vec<String> = (1..=25).map(|n| format!("core-{n:03}")).collect();
    let state = jackioh_engine::create_game(&jackioh_engine::CreateGameOptions {
        seed: "face-test".to_string(),
        decks: (deck.clone(), deck),
        catalog: None,
        handicaps: None,
        dealt: None,
        last_boards: None,
        glitch_boards: None,
        sandbox: None,
        deck_marks: None,
        win_rates: None,
    });
    let state = jackioh_engine::begin_game(&state).state;
    jackioh_engine::view_for(&state, PlayerId::P1)
}

#[test]
fn a_view_gives_a_face_to_each_card_it_lists_and_none_to_a_card_it_hides() {
    let view = a_view();
    let FaceAnswer::Faces { faces } = apply(&FaceCall::View {
        view: Box::new(view.clone()),
        locale: Locale::En,
    }) else {
        panic!("faces")
    };
    let hand = match &view.you.hand {
        jackioh_engine::wire::HandView::Cards(cards) => cards,
        other => panic!("the viewer sees their own hand, got {other:?}"),
    };
    assert!(!hand.is_empty());
    for card in hand {
        let face = faces
            .get(&card.instance_id)
            .unwrap_or_else(|| panic!("a face for {}", card.instance_id));
        assert_eq!(face.def_id, card.def_id);
        assert!(face.in_play);
        assert!(
            face.badges
                .iter()
                .all(|badge| serde_json::to_value(badge.kind).unwrap() != "created")
        );
    }
    // The other seat's hand is a count: no instance of it has a face, so none can be named.
    assert!(matches!(
        view.opponent.hand,
        jackioh_engine::wire::HandView::Count { .. }
    ));
    assert_eq!(faces.len(), hand.len());
}

#[test]
fn a_card_the_view_names_has_a_face_unless_it_is_the_sentinel() {
    let view = a_view();
    let named = |def_id: &str| {
        apply(&FaceCall::Named {
            view: Box::new(view.clone()),
            named: jackioh_shared::face::NamedCard {
                def_id: def_id.to_string(),
                radiant: false,
                instance_id: None,
                cost: Some(2),
            },
            locale: Locale::En,
        })
    };
    assert!(matches!(named("hidden"), FaceAnswer::NoFace));
    assert!(matches!(named(""), FaceAnswer::NoFace));
    let FaceAnswer::Face { face } = named("core-020") else {
        panic!("a face")
    };
    assert_eq!(
        (face.name.as_str(), face.cost.text.as_str()),
        ("Pointmaster", "2")
    );
}

#[test]
fn a_name_a_client_cannot_link_is_plain_again_with_its_terms_in_bold() {
    // Core #15 summons a Rush Token: the reference holds "Rush Token", which names a card, not the keyword.
    let face = printed("core-015", false, Locale::En);
    let has = |spans: &[Span], kind: &str| {
        flatten(spans).into_iter().any(|span| match span {
            Span::Card { .. } => kind == "card",
            Span::Keyword { entry, .. } => entry.id == kind,
            _ => false,
        })
    };
    assert!(has(&face.text.spans, "card") && !has(&face.text.spans, "Rush"));
    let unlinked = face
        .text
        .unlinked
        .expect("a face that names a card also gives its words with no reference");
    assert!(has(&unlinked, "Rush") && !has(&unlinked, "card"));
    assert_eq!(words(&unlinked), face.text.full);
    // A face that names no card has no second reading.
    assert!(printed("core-020", false, Locale::En).text.unlinked.is_none());
}

// ---- The Stinkyscape balance pass (#3125, ported from the web's TypeScript face code) --------------------

/// The catalog's definition of `id`.
fn catalog_def(id: &str) -> jackioh_engine::wire::CardDef {
    ready();
    registered_catalog()
        .get(id)
        .unwrap_or_else(|| panic!("the catalog has no {id}"))
        .clone()
}

/// `ingredients` fused as the engine's Fuse builds one: the id spells them (a fusion in parentheses, R179), the
/// name joins theirs, and the definition lists them (R468).
fn fused(n: u32, ingredients: &[&jackioh_engine::wire::CardDef]) -> jackioh_engine::wire::CardDef {
    let spelled: Vec<String> = ingredients
        .iter()
        .map(|def| {
            if def.ingredients.is_some() {
                format!("({})", def.id)
            } else {
                def.id.clone()
            }
        })
        .collect();
    let mut def = ingredients[0].clone();
    def.id = format!("t-{n}:{}", spelled.join("+"));
    def.index = def.id.clone();
    def.name = ingredients
        .iter()
        .map(|def| def.name.as_str())
        .collect::<Vec<_>>()
        .join(" + ");
    def.ingredients = Some(
        ingredients
            .iter()
            .map(|def| jackioh_engine::wire::FusedIngredient {
                def_id: def.id.clone(),
                radiant: None,
            })
            .collect(),
    );
    def
}

/// A fused definition as a face source: the view carries the definition it names (R243).
fn face_of_def(
    def: &jackioh_engine::wire::CardDef,
    radiant: bool,
    locale: Locale,
    extra: Value,
) -> FaceModel {
    ready();
    let mut source = json!({
        "defId": def.id,
        "def": serde_json::to_value(def).expect("a definition"),
        "radiant": radiant,
    });
    for (key, value) in extra.as_object().expect("an object") {
        source[key] = value.clone();
    }
    face_of(call(
        json!({ "call": "card", "locale": locale_name(locale), "source": source }),
    ))
}

const HULL: &str = jackioh_shared::face::fused::DORMANT_STARSHIP_ID;
const MINER: &str = "stinkyscape-062";
const BEAM: &str = "stinkyscape-050";

#[test]
fn r3045000_the_hull_and_its_name_are_the_catalogs() {
    let hull = catalog_def(HULL);
    assert_eq!(hull.name, jackioh_shared::face::fused::DORMANT_STARSHIP_NAME);
    let once = fused(1, &[&hull, &catalog_def(MINER)]);
    assert_eq!(once.name, "Dormant Starship + Miner v1");
}

#[test]
fn r3045000_a_starship_keeps_the_hulls_name_in_english() {
    let hull = catalog_def(HULL);
    let once = fused(1, &[&hull, &catalog_def(MINER)]);
    let twice = fused(2, &[&once, &catalog_def(BEAM)]);
    let pieces = json!([{ "defId": MINER, "radiant": false }, { "defId": BEAM, "radiant": false }]);
    for def in [&once, &twice] {
        // The view says it is a Starship: the hull's name; a launched one keeps the name it launched under.
        let dormant = face_of_def(
            def,
            false,
            Locale::En,
            json!({ "inPlay": { "starship": { "pieces": 2, "name": null, "list": pieces } } }),
        );
        assert_eq!(dormant.name, "Dormant Starship");
        assert!(dormant.starship.as_ref().is_some_and(|ship| !ship.launched));
        let launched = face_of_def(
            def,
            false,
            Locale::En,
            json!({ "inPlay": { "starship": { "pieces": 2, "name": "Ember of Departure", "list": pieces } } }),
        );
        assert_eq!(launched.name, "Ember of Departure");
        assert!(launched.starship.as_ref().is_some_and(|ship| ship.launched));
        // A fused definition holding the hull is named for it where no view says it is a Starship.
        assert_eq!(
            face_of_def(def, false, Locale::En, json!({ "inPlay": {} })).name,
            "Dormant Starship"
        );
        assert_eq!(
            face_of_def(def, true, Locale::En, json!({})).name,
            "Dormant Starship"
        );
    }
}

#[test]
fn r3045000_a_view_that_says_starship_names_even_a_definition_that_hides_its_pieces() {
    let hull = catalog_def(HULL);
    let once = fused(1, &[&hull, &catalog_def(MINER)]);
    let mut hidden = fused(2, &[&once, &catalog_def(BEAM)]);
    hidden.id = "t-9:#0123456789abcdef".to_string();
    hidden.ingredients = Some(vec![
        jackioh_engine::wire::FusedIngredient {
            def_id: "t-8:#fedcba9876543210".to_string(),
            radiant: None,
        },
        jackioh_engine::wire::FusedIngredient {
            def_id: BEAM.to_string(),
            radiant: None,
        },
    ]);
    let in_view = face_of_def(
        &hidden,
        false,
        Locale::En,
        json!({ "inPlay": { "starship": { "pieces": 2, "name": null } } }),
    );
    assert_eq!(in_view.name, "Dormant Starship");
    // With no view to say so, a definition that hides its pieces is named as the engine names it.
    let unseen = face_of_def(&hidden, false, Locale::En, json!({ "inPlay": {} }));
    assert_eq!(unseen.name, hidden.name);
}

#[test]
fn r3045000_the_hull_itself_and_every_other_fusion_are_named_as_they_were() {
    let hull = catalog_def(HULL);
    assert_eq!(
        face_of_def(&hull, false, Locale::En, json!({ "inPlay": {} })).name,
        "Dormant Starship"
    );
    let equipped = fused(3, &[&catalog_def(BEAM), &catalog_def("core-020")]);
    assert_eq!(
        face_of_def(&equipped, false, Locale::En, json!({ "inPlay": {} })).name,
        "Beam v3 + Pointmaster"
    );
    assert_eq!(printed("core-020", false, Locale::En).name, "Pointmaster");
}

#[test]
fn r3045000_a_chinese_face_names_a_fused_dormant_starship_for_the_hull_in_chinese() {
    let hull = catalog_def(HULL);
    let once = fused(1, &[&hull, &catalog_def(MINER)]);
    let twice = fused(2, &[&once, &catalog_def(BEAM)]);
    let hull_zh = printed(HULL, false, Locale::ZhCn).name;
    assert_ne!(hull_zh, "Dormant Starship");
    for def in [&once, &twice] {
        let dormant = face_of_def(
            def,
            false,
            Locale::En,
            json!({ "chinese": true, "inPlay": { "starship": { "pieces": 2, "name": null } } }),
        );
        assert_eq!(dormant.name, hull_zh);
        assert_eq!(dormant.english_name.as_deref(), Some("Dormant Starship"));
        let launched = face_of_def(
            def,
            false,
            Locale::En,
            json!({ "chinese": true, "inPlay": { "starship": { "pieces": 2, "name": "Ember of Departure" } } }),
        );
        assert_eq!(launched.name, "Ember of Departure");
        assert_eq!(
            face_of_def(def, false, Locale::En, json!({ "chinese": true, "inPlay": {} })).name,
            hull_zh
        );
    }
    // The ingredients' Chinese words still join in its text, and any other fusion is joined from the names.
    let first = face_of_def(&once, false, Locale::ZhCn, json!({ "inPlay": {} }));
    let parts = [
        printed(HULL, false, Locale::ZhCn),
        printed(MINER, false, Locale::ZhCn),
    ];
    assert_eq!(
        first.text.full,
        format!("{}\n{}", parts[0].text.full, parts[1].text.full)
    );
    let equipped = fused(3, &[&catalog_def(BEAM), &catalog_def("core-020")]);
    let joined = face_of_def(&equipped, false, Locale::ZhCn, json!({ "inPlay": {} }));
    assert_eq!(
        joined.name,
        format!(
            "{} + {}",
            printed(BEAM, false, Locale::ZhCn).name,
            printed("core-020", false, Locale::ZhCn).name
        )
    );
}

#[test]
fn r3038000_r3039000_the_starship_and_dormant_rules_say_what_the_spec_says() {
    use jackioh_shared::face::glossary::entry;
    let starship = entry("Starship").expect("the Starship term");
    assert_eq!(
        starship.rule,
        "A tag: your Starship, Dormant or launched, built from the Starship Pieces fused into it. Every Starship has Flying and Ranged"
    );
    let dormant = entry("Dormant").expect("the Dormant term");
    assert!(
        dormant
            .rule
            .ends_with("and its auras and triggers are off. An awakened card is never Dormant again"),
        "{}",
        dormant.rule
    );
}

#[test]
fn r3134002_start_of_game_is_no_term_so_no_card_prints_it_as_one() {
    use jackioh_shared::face::glossary::{entry, terms_in};
    assert!(entry("Start of game").is_none());
    let text: Vec<char> = "Start of game: gain a power".chars().collect();
    assert!(terms_in(&text).is_empty());
    // The Chinese table lost its entry with the English one (crates/cards/chinese-terms.json).
    assert!(jackioh_shared::face::glossary::row_of("Start of game").is_none());
}

#[test]
fn r3134001_the_faces_the_balance_pass_put_over_the_limit_clamp() {
    use jackioh_shared::face::{data::ShortForm, short::short_form_of};
    // Issues #3138, #3139 and #3140, and R3134001's Radiant Cry and Aura.
    let both = [
        "core-065",
        "core-089",
        "classicplus-037",
        "meditative-020",
        "meditative-040",
        "meditative-054",
        "meditative-081",
        "meditativeplus-022",
    ];
    for id in both {
        for radiant in [false, true] {
            assert_eq!(
                short_form_of(id, radiant),
                Some(ShortForm::Clamp),
                "{id} {radiant}"
            );
        }
    }
    for id in ["core-052", "meditative-009"] {
        assert_eq!(short_form_of(id, true), Some(ShortForm::Clamp), "{id}");
        assert_eq!(short_form_of(id, false), None, "{id}");
    }
    // Training Grounds #38's text is one short line since issue #3139.
    assert_eq!(short_form_of("traininggrounds-038", true), None);
}

#[test]
fn r3337001_an_anomaly_found_holding_a_board_names_it() {
    const ANOMALY_FOUND: &str = "stinkyscape-t-anomaly-found";
    let held = in_play(
        ANOMALY_FOUND,
        false,
        Locale::En,
        json!({ "inPlay": { "label": "WH" } }),
    );
    assert_eq!(
        held.anomaly.as_deref(),
        Some("WH"),
        "its art draws the board it holds"
    );
    let rich = in_play(
        ANOMALY_FOUND,
        true,
        Locale::En,
        json!({ "inPlay": { "label": "FO" } }),
    );
    assert_eq!(rich.anomaly.as_deref(), Some("FO"));
    assert_eq!(
        printed(ANOMALY_FOUND, false, Locale::En).anomaly,
        None,
        "a printed face holds none"
    );
    assert_eq!(
        in_play(ANOMALY_FOUND, false, Locale::En, json!({})).anomaly,
        None,
        "nor one that holds no board (an opponent's, R202)"
    );
    // Any other card with a label (Heroic Power's word, say) draws its own art.
    let other = in_play(
        "stinkyscape-166",
        false,
        Locale::En,
        json!({ "inPlay": { "label": "WH" } }),
    );
    assert_eq!(other.anomaly, None);
}
