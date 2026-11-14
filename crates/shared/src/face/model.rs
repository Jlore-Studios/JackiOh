//! What a card face shows, computed once from a catalog def and the live view (docs/polish/6-cards.md,
//! Surface B). Pure but for the reader's language (R711000); nothing here is a rule (CLAUDE.md rule 7). A
//! face with no `in_play` is the card as printed (SPEC §10.10); with `in_play`, as the view says it stands
//! (R243): its cost, a hand Unit's gained stats (#89 Corpse Eater), `power.rs`'s words, `state.rs`'s states;
//! a match-made def (a Fuse's, R77) prints its own. An Animated Field Spell or Trap that prints attack and
//! health prints the Unit's (B3.1 rule 1; R2280000: it may print none). A copier (Classic #57 Echo, B5 E14,
//! R399, R511) prints the Spell text the view says it has (`InPlay.copies`). ME-CN, R1301, R711000: a
//! Chinese card (`FaceSource.chinese`), or any when the reader reads Chinese, prints name and texts from the
//! Chinese table (`chinese.rs`); a Radiant gold diff compares the Chinese faces (R1302). The English name
//! and text ride along, never printed: art motif and glossary read them.

use indexmap::IndexMap;
use jackioh_engine::config::TRIBAL_TAGS;
use jackioh_engine::wire::{
    CardCost, CardDef, CardFace, CardType, Keyword, KeywordKind, Param, ParamBetter, PreviewValue, Tag,
    keyword_key,
};

use crate::config::TEMPORARY_LAST_COUNT;

use super::chinese::{
    CHINESE_COMMA, all_tribes_word, chinese_def, chinese_keyword, chinese_name, chinese_preview_label,
    is_glitch, keyword_kind_word, tag_word, type_word,
};
use super::data::ShortForm;
use super::diff::radiant_marks;
use super::fused::{DORMANT_STARSHIP_ID, DORMANT_STARSHIP_NAME, holds_dormant_starship};
use super::glitch::{glitch_def, glitch_words};
use super::glossary::{glossary_for, state_terms};
use super::power::{
    HEROIC_POWER_ID, VANILLA_TEXT, concealed_in_play, concealed_text, power_text, printed_values,
};
use super::quest::quest_face_text;
use super::refs::{RADIANT_WORD, find_refs};
use super::short::{ascent_level_of, cut_text, short_form_of, summit_short_text};
use super::spans::{Dressing, rich};
use super::state::{BadgeFacts, state_badges};
use super::text::{Text, copied_text};
use super::tuning::{MINUS, face_tuning, filled_text};
use super::types::{
    CostTone, DefMap, FaceCost, FaceCredits, FaceModel, FaceSource, FaceStats, Grew, InPlay, LiveStats,
    Mention, QuestBox, StarshipFace, StatTone, TagChip, TuningChip,
};
use super::words::Words;

/// The gem of a card nobody can name: no catalog, no live cost.
const UNKNOWN_COST: &str = "?";

/// A card with no def is drawn as a Unit.
const UNKNOWN_TYPE: CardType = CardType::Unit;

/// R1382, R1162 (Meditative #87 Tatches the Totem): what the frame prints for a card with every tribal tag
/// (R1424), in place of the five.
pub const ALL_TRIBES: &str = "All Tribes";
/// R3337001: the card whose label names the Anomaly its art draws (Stinkyscape's Anomaly Found!).
pub const ANOMALY_FOUND_ID: &str = "stinkyscape-t-anomaly-found";

/// What the model looks definitions up in: the match's own, then the catalog.
pub struct Lookup<'a> {
    pub defs: &'a DefMap,
}

impl Lookup<'_> {
    /// The definition `id` names, Glitch's words corrupted as a lookup answers for it (`glitch.rs`).
    pub fn def(&self, id: &str) -> Option<CardDef> {
        let found = jackioh_engine::catalog::find_def(None, id).or_else(|| self.defs.get(id))?;
        Some(if is_glitch(id) {
            glitch_def(found)
        } else {
            found.clone()
        })
    }
}

/// R3045000: the Dormant Starship's catalog name, in the face's language.
fn hull_name(chinese: bool) -> String {
    if chinese {
        chinese_name(DORMANT_STARSHIP_ID, DORMANT_STARSHIP_NAME, None)
    } else {
        DORMANT_STARSHIP_NAME.to_string()
    }
}

/// The face model of a card: `words` is the reader's language.
pub fn face_model(words: Words, source: &FaceSource, lookup: &Lookup<'_>) -> FaceModel {
    face_in(
        words,
        source,
        lookup,
        words.drawn_in_chinese(source.chinese == Some(true)),
    )
}

fn face_in(words: Words, source: &FaceSource, lookup: &Lookup<'_>, chinese: bool) -> FaceModel {
    let given = source.def.clone().or_else(|| lookup.def(&source.def_id));
    let filled = with_view_params(
        given,
        source.in_play.as_ref().and_then(|play| play.params.as_ref()),
    );
    // R1301: a Chinese card's name and texts are the table's; all else stays the definition's.
    let def: Option<CardDef> = match (&filled, chinese) {
        (Some(filled), true) => Some(chinese_def(filled)),
        _ => filled.clone(),
    };
    let radiant = source.radiant;
    let printed: Option<&CardFace> = def
        .as_ref()
        .map(|def| if radiant { &def.radiant } else { &def.base });
    // B2.7: a face may carry its own type (Classic+ #22 Blood Moon's Radiant face is a Field Trap), and the
    // card's type is its face's (§5.2); in play the view's word for it comes first.
    let view_type = if source.in_play.is_some() {
        source.type_
    } else {
        None
    };
    let type_ = view_type
        .or(printed.and_then(|face| face.type_))
        .or(def.as_ref().map(|def| def.type_))
        .or(source.type_)
        .unwrap_or(UNKNOWN_TYPE);
    let in_play = source.in_play.as_ref();
    let vanilla = in_play.is_some_and(|play| play.vanilla == Some(true));
    let printed_text = text_of(def.as_ref(), radiant, None);
    let own_text = match in_play.and_then(|play| play.params.as_ref()) {
        None => printed_text.clone(),
        Some(params) => text_of(def.as_ref(), radiant, Some(params)),
    };
    // B5 E14, R511: a copier's own words are the copied Spell's text (in Chinese on a Chinese copier, R1301).
    let copies = in_play.and_then(|play| play.copies.as_ref()).map(|copies| {
        let mut copies = copies.clone();
        if chinese {
            copies.def = chinese_def(&copies.def);
        }
        copies
    });
    let live_text = match &copies {
        None => own_text,
        Some(copies) => {
            let sentence: Vec<char> = def
                .as_ref()
                .map_or_else(Vec::new, |def| filled_text(def, false, None).0.chars().collect());
            copied_text(
                &own_text,
                &sentence,
                &text_of(Some(&copies.def), copies.radiant, copies.params.as_ref()),
            )
        }
    };
    let text = match in_play {
        None => printed_text.clone(),
        Some(play) => text_in_play(words, def.as_ref(), radiant, &live_text, play, chinese),
    };
    let keywords: Vec<Keyword> = source
        .live
        .as_ref()
        .map(|live| live.keywords.clone())
        .or_else(|| in_play.and_then(|play| play.hand_keywords.clone()))
        .or_else(|| printed.map(|face| face.keywords.clone()))
        .unwrap_or_default();
    // The values belong to the card's own words, its numbers as they stand included: a formula play does
    // not print (Vanilla, "???", a rolled power) has no value to show.
    let prints_its_text = text.full == live_text.full;
    // B3.4, R386: what Degrade and Upgrade changed; the keywords they added are marked as such, so the line
    // of keywords gained since printing leaves them out.
    let tuning = in_play.and_then(|play| {
        face_tuning(
            words,
            def.as_ref(),
            radiant,
            play.tuning.as_ref(),
            play.params.as_ref(),
        )
    });
    let tuned_keys: Vec<String> = tuning
        .iter()
        .flat_map(|tuning| tuning.added.iter().map(keyword_key))
        .collect();
    // R280, R1301: a Chinese card's preview labels are the table's, filled with the numbers its face is.
    let preview: &[PreviewValue] = in_play.and_then(|play| play.preview.as_deref()).unwrap_or(&[]);
    let values: Vec<PreviewValue> = match (&filled, chinese) {
        (Some(filled), true) => preview
            .iter()
            .map(|entry| PreviewValue {
                label: chinese_preview_label(
                    filled,
                    radiant,
                    &entry.label,
                    in_play.and_then(|play| play.params.as_ref()),
                ),
                ..entry.clone()
            })
            .collect(),
        _ => preview.to_vec(),
    };
    let english = if chinese {
        Some(face_in(
            Words::new(super::types::Locale::En),
            source,
            lookup,
            false,
        ))
    } else {
        None
    };
    let english = english.map(|face| (face.name, face.text.full));

    let glitch = is_glitch(&source.def_id);
    let cost = if glitch {
        FaceCost {
            text: glitch_words().cost,
            ..cost_of(
                def.as_ref(),
                source.live_cost,
                source.live_embiggen_cost,
                source.fixed_x,
            )
        }
        .with_base()
    } else {
        cost_of(
            def.as_ref(),
            source.live_cost,
            source.live_embiggen_cost,
            source.fixed_x,
        )
    };
    let live_for_stats: Option<LiveStats> = source
        .live
        .clone()
        .or_else(|| hand_live(in_play.and_then(|play| play.hand_stats), printed));
    let stats = stats_of(
        type_,
        def.is_some(),
        printed,
        live_for_stats.as_ref(),
        grew_of(def.as_ref(), source),
    );
    let tags: Vec<Tag> = match (&source.in_play, &source.tags) {
        (Some(_), Some(tags)) => tags.clone(),
        _ => def.as_ref().map(|def| def.tags.clone()).unwrap_or_default(),
    };
    let refs: Vec<String> = copies
        .iter()
        .flat_map(|copies| copies.def.refs.clone().unwrap_or_default())
        .chain(def.iter().flat_map(|def| def.refs.clone().unwrap_or_default()))
        .collect();
    let ref_defs: Vec<CardDef> = refs.iter().filter_map(|id| lookup.def(id)).collect();
    // The named cards: the text's names that point at a card other than this one (R279).
    let mention_defs: Vec<CardDef> = refs
        .iter()
        .filter(|id| **id != source.def_id)
        .filter_map(|id| lookup.def(id))
        .collect();
    let mut mentions: Vec<Mention> = Vec::new();
    for found in find_refs(&text.full, &mention_defs, RADIANT_WORD) {
        let mention = Mention {
            def_id: found.id,
            radiant: found.radiant,
        };
        if !mentions.contains(&mention) {
            mentions.push(mention);
        }
    }
    let gained: Vec<Keyword> = match in_play {
        Some(play) if source.live.is_some() || play.hand_keywords.is_some() => {
            let base: &[Keyword] = if vanilla {
                &[]
            } else {
                printed.map_or(&[], |face| face.keywords.as_slice())
            };
            gained_keywords(&keywords, base)
                .into_iter()
                .filter(|keyword| !tuned_keys.contains(&keyword_key(keyword)))
                .collect()
        }
        _ => Vec::new(),
    };
    let concealed = def.as_ref().is_some_and(|def| concealed_in_play(&def.tags));
    let printed_out: Option<&Text> = match in_play {
        Some(_) if text.full != printed_text.full && !concealed => Some(&printed_text),
        _ => None,
    };
    let temporary = if in_play.is_some_and(|play| play.in_hand == Some(true)) {
        temporary_left(&keywords)
    } else {
        None
    };
    let animated = in_play.and_then(|play| play.animated);
    let dormant = in_play.and_then(|play| play.dormant);
    let starship = in_play
        .and_then(|play| play.starship.as_ref())
        .map(|ship| StarshipFace {
            pieces: ship.pieces,
            launched: ship.name.is_some(),
            list: ship.list.clone().unwrap_or_default(),
            term: super::glossary::entry("Starship"),
        });
    // R3337001: an Anomaly Found! that holds a board (R2247000) draws that board's picture.
    let anomaly = in_play
        .and_then(|play| play.label.clone())
        .filter(|_| source.def_id == ANOMALY_FOUND_ID);
    let enchantments = in_play
        .and_then(|play| play.enchantments.clone())
        .unwrap_or_default();
    let quest = in_play.and_then(|play| play.quest.clone());
    let berserk = in_play.is_some_and(|play| play.berserk == Some(true));
    let frozen = in_play.is_some_and(|play| play.frozen == Some(true));
    let frozen_forever = in_play.is_some_and(|play| play.frozen_forever == Some(true));
    let created = in_play.is_some_and(|play| play.created == Some(true));
    let evolved = in_play.is_some_and(|play| play.evolved == Some(true));
    let brittle = in_play.and_then(|play| play.brittle);

    // The states the face draws, and the glossary entries they and the face name.
    let badges = state_badges(&BadgeFacts {
        words,
        radiant,
        type_,
        chinese,
        brittle,
        temporary,
        quest: quest.as_ref(),
        tuning: tuning.as_ref(),
        enchantments: &enchantments,
        animated: animated.as_ref(),
        berserk,
        frozen,
        frozen_forever,
        dormant: dormant.as_ref(),
        starship: starship.as_ref(),
        created,
        evolved,
    });
    let english_chars: Vec<char> = english
        .as_ref()
        .map_or_else(|| text.full.clone(), |(_, text)| text.chars().collect());
    let kinds: Vec<KeywordKind> = keywords.iter().map(Keyword::kind).collect();
    let states = state_terms(
        brittle,
        tuning.as_ref(),
        &enchantments,
        animated.map(|animated| animated.home),
    );
    let glossary = glossary_for(&english_chars, &kinds, &states, chinese);

    // The formula values the face prints (R280): only beside its own words.
    let face_values: Vec<PreviewValue> = if prints_its_text { values } else { Vec::new() };
    let dressing = Dressing {
        refs: &ref_defs,
        chinese,
        values: &face_values,
    };
    // What the rules box prints in place of its whole text (issues #814, #1017; R2450000).
    let quest_box = quest_face_text(words, &tags, &text, quest.as_ref()).map(|found| QuestBox {
        text: rich(&found.text, &dressing, words),
        less: found.less,
    });
    let ascent = ascent_level_of(&face_values);
    let summit = summit_short_text(&source.def_id, &text.full, ascent).map(|cut| Text {
        full: cut,
        marks: text.marks.clone(),
        tuned: text.tuned.clone(),
    });
    let form = if text.full.is_empty() || printed_out.is_some() {
        None
    } else {
        short_form_of(&source.def_id, radiant)
    };
    let short = form.and_then(|form| cut_text(&text.full, form)).map(|cut| Text {
        full: cut,
        marks: text.marks.clone(),
        tuned: text.tuned.clone(),
    });

    // The collection's text beside a face that prints something else is marked as its face is, with no
    // values and no moved numbers (R277, R279).
    let printed_dressing = Dressing {
        refs: &ref_defs,
        chinese,
        values: &[],
    };
    // R3045000: a Starship keeps the hull's name whatever was fused into it (the engine joins the names): the name it
    // launched under, else the Dormant Starship's. The view says a unit is a Starship; a fused definition says so
    // too where no view does (a card the log or the graveyard names).
    let holds_hull = in_play.is_some_and(|play| play.starship.is_some())
        || source.def.as_ref().is_some_and(holds_dormant_starship);
    let hull = holds_hull.then(|| hull_name(chinese));
    let name = in_play
        .and_then(|play| play.starship.as_ref())
        .and_then(|ship| ship.name.clone())
        .or(hull)
        .or_else(|| def.as_ref().map(|def| def.name.clone()))
        .or_else(|| source.name.clone())
        .unwrap_or_else(|| source.def_id.clone());

    let keyword_word = |keyword: &Keyword| {
        if chinese {
            chinese_keyword(keyword)
        } else {
            keyword_key(keyword)
        }
    };
    let kind_word = |kind: KeywordKind| {
        if chinese {
            keyword_kind_word(kind).to_string()
        } else {
            kind.as_str().to_string()
        }
    };
    let comma = if chinese { CHINESE_COMMA } else { ", " };
    let gained_words: Vec<String> = gained.iter().map(keyword_word).collect();
    let grants: Vec<String> = in_play.and_then(|play| play.grants.clone()).unwrap_or_default();
    let credits = credits_of(def.as_ref(), radiant);

    let tuning_chips: Vec<TuningChip> = tuning
        .iter()
        .flat_map(|tuning| {
            let added = tuning.added.iter().map(|keyword| {
                let text = keyword_word(keyword);
                TuningChip {
                    sign: "+".to_string(),
                    title: words.say_with("Gained {keyword}", &[("keyword", text.clone())]),
                    text,
                    added: true,
                }
            });
            let removed = tuning.removed.iter().map(|kind| {
                let text = kind_word(*kind);
                TuningChip {
                    sign: MINUS.to_string(),
                    title: words.say_with("Lost {keyword}", &[("keyword", text.clone())]),
                    text,
                    added: false,
                }
            });
            added.chain(removed).collect::<Vec<_>>()
        })
        .collect();

    let token = def.as_ref().is_some_and(|def| def.token);
    let type_line = if glitch {
        glitch_words().type_
    } else if chinese {
        let word = type_word(type_);
        if token {
            format!("{}{word}", tag_word(Tag::Token))
        } else {
            word.to_string()
        }
    } else if token {
        format!("Token {}", type_.as_str())
    } else {
        type_.as_str().to_string()
    };
    let tag_chips: Vec<TagChip> = frame_tags(&tags)
        .into_iter()
        .map(|tag| {
            let text = match &tag {
                FrameTag::AllTribes => {
                    if chinese {
                        all_tribes_word().to_string()
                    } else {
                        ALL_TRIBES.to_string()
                    }
                }
                FrameTag::Tag(tag) => {
                    if chinese {
                        tag_word(*tag).to_string()
                    } else {
                        tag.as_str().to_string()
                    }
                }
            };
            TagChip {
                tag: tag.name(),
                text,
            }
        })
        .collect();

    FaceModel {
        def_id: source.def_id.clone(),
        known: def.is_some(),
        name,
        type_,
        tags,
        rarity: def.as_ref().map(|def| def.rarity),
        printed_rarity: def.as_ref().and_then(|def| def.printed_rarity),
        token,
        index: def.as_ref().map(|def| def.index.clone()),
        set: def.as_ref().map(|def| def.set),
        radiant,
        cost,
        stats,
        text: rich(&text, &dressing, words),
        short: short.as_ref().map(|cut| rich(cut, &dressing, words)),
        summit: summit.as_ref().map(|cut| rich(cut, &dressing, words)),
        ascent,
        clamp: form == Some(ShortForm::Clamp),
        has_short_form: form.is_some(),
        quest_box,
        refs,
        mentions,
        values: face_values,
        keyword_words: keywords.iter().map(keyword_word).collect(),
        keyword_join: comma.to_string(),
        keywords,
        in_play: in_play.is_some(),
        vanilla,
        gained_line: super::spans::rich_words(&gained_words.join(comma), words),
        gained,
        grants: grants
            .iter()
            .map(|grant| super::spans::rich_words(grant, words))
            .collect(),
        printed: printed_out.map(|text| rich(text, &printed_dressing, words)),
        brittle,
        temporary,
        element: in_play.and_then(|play| play.element),
        tuning,
        tuning_chips,
        enchantments,
        animated,
        berserk,
        frozen,
        dormant,
        frozen_forever,
        starship,
        anomaly,
        created,
        created_by: in_play.and_then(|play| play.created_by.clone()),
        loc: def.as_ref().and_then(|def| def.loc),
        credits_line: credits.map_or_else(String::new, |credits| {
            words.say_with(
                "Credits: {min}–{max}",
                &[("min", credits.min.to_string()), ("max", credits.max.to_string())],
            )
        }),
        credits,
        marks: in_play.and_then(|play| play.marks.clone()).unwrap_or_default(),
        copying: copies.as_ref().map(|copies| super::types::Copying {
            def_id: copies.def.id.clone(),
            name: copies.def.name.clone(),
            radiant: copies.radiant,
        }),
        quest,
        type_line,
        tag_chips,
        badges,
        glossary,
        chinese: english.as_ref().map(|_| true),
        flagged: (source.chinese == Some(true)).then_some(true),
        english_name: english.as_ref().map(|(name, _)| name.clone()),
        english_text: english.map(|(_, text)| text),
        attack_reason: in_play.and_then(|play| play.attack_reason.clone()),
        switch_reason: in_play.and_then(|play| play.switch_reason.clone()),
        tallies: in_play.and_then(|play| play.tallies.clone()),
        play_reason: in_play.and_then(|play| play.play_reason.clone()),
    }
}

/// #2437, R2179001: a card's Temporary X, the largest of its Temporary entries (a plain one counts
/// `TEMPORARY_LAST_COUNT`); `None` with none.
fn temporary_left(keywords: &[Keyword]) -> Option<i32> {
    keywords
        .iter()
        .filter(|keyword| keyword.kind() == KeywordKind::Temporary)
        .map(|keyword| keyword.n().unwrap_or(TEMPORARY_LAST_COUNT))
        .max()
}

/// R243: a hand card's stats are its face plus what it gained in hand, with nothing on the field's layers.
fn hand_live(stats: Option<super::types::HandStats>, printed: Option<&CardFace>) -> Option<LiveStats> {
    stats.map(|stats| LiveStats {
        attack: stats.attack,
        health: stats.health,
        max_health: stats.health,
        keywords: printed.map(|face| face.keywords.clone()).unwrap_or_default(),
    })
}

/// B3.4: a fused definition (R77, R102) carries no `params`, though its text writes its ingredients' `{key}`s:
/// in play the view's values fill them and count as printed, so none is marked as moved.
fn with_view_params(def: Option<CardDef>, values: Option<&IndexMap<String, i32>>) -> Option<CardDef> {
    let mut def = def?;
    let Some(values) = values else { return Some(def) };
    if def.params.is_some() || values.is_empty() {
        return Some(def);
    }
    def.params = Some(
        values
            .iter()
            .map(|(key, value)| Param {
                key: key.clone(),
                base: *value,
                radiant: *value,
                better: ParamBetter::Up,
                step: None,
                min: None,
                max: None,
                tuned_on: None,
                power: None,
                cry: None,
            })
            .collect(),
    );
    Some(def)
}

/// R2521009: a Resource's sale prices as its definition prints them (`CardDef.resource`, R1840000), worked out
/// by nothing (CLAUDE.md rule 7); `None` for a card that is no Resource.
fn credits_of(def: Option<&CardDef>, radiant: bool) -> Option<FaceCredits> {
    let resource = def?.resource.as_ref()?;
    let price = if radiant { resource.radiant } else { resource.base };
    Some(FaceCredits {
        min: price.sell_min,
        max: price.sell_max,
    })
}

/// The keywords a card has now that its printed face does not, each once, in order. Lucky X stacks: it is
/// gained when its sum differs from the printed one, as one entry at the sum (R1438).
fn gained_keywords(live: &[Keyword], printed: &[Keyword]) -> Vec<Keyword> {
    let printed_keys: Vec<String> = printed.iter().map(keyword_key).collect();
    let luck = luck_of(live);
    let mut luck_gained = luck != luck_of(printed);
    let mut seen: Vec<String> = Vec::new();
    let mut gained = Vec::new();
    for keyword in live {
        if keyword.kind() == KeywordKind::Lucky {
            if luck_gained {
                gained.push(Keyword::Lucky { n: luck });
            }
            luck_gained = false;
            continue;
        }
        let key = keyword_key(keyword);
        if printed_keys.contains(&key) || seen.contains(&key) {
            continue;
        }
        seen.push(key);
        gained.push(keyword.clone());
    }
    gained
}

/// Lucky X's sum over a list of keywords (§6.1: Lucky stacks).
fn luck_of(keywords: &[Keyword]) -> i32 {
    keywords
        .iter()
        .map(|keyword| if let Keyword::Lucky { n } = keyword { *n } else { 0 })
        .sum()
}

fn grew_of(def: Option<&CardDef>, source: &FaceSource) -> Option<Grew> {
    let def = def?;
    if !source.radiant
        || source.live.is_some()
        || source
            .in_play
            .as_ref()
            .is_some_and(|play| play.hand_stats.is_some())
    {
        return None;
    }
    Some(Grew {
        attack: def.radiant.attack.unwrap_or(0) > def.base.attack.unwrap_or(0),
        health: def.radiant.health.unwrap_or(0) > def.base.health.unwrap_or(0),
    })
}

/// What the rules box prints in play (`power.rs`): a #98 Heroic Power prints the power it rolled.
fn text_in_play(
    words: Words,
    def: Option<&CardDef>,
    radiant: bool,
    printed_text: &Text,
    in_play: &InPlay,
    chinese: bool,
) -> Text {
    // R711000: a face drawn in Chinese for a reader of Chinese says it in the reader's words; the English
    // face (`english_text`, what the glossary's terms are found in) keeps the English.
    let reader = chinese && words.is_zh_cn();
    if in_play.vanilla == Some(true) {
        return Text::plain(&if reader {
            words.say(VANILLA_TEXT)
        } else {
            VANILLA_TEXT.to_string()
        });
    }
    let Some(def) = def else {
        return printed_text.clone();
    };
    if concealed_in_play(&def.tags) {
        return Text::plain(concealed_text(&def.id, radiant));
    }
    // R2247000: a card that holds a label (Anomaly Found!'s "FO") prints it as its text.
    if let Some(label) = &in_play.label {
        return Text::plain(label);
    }
    // R2249000: a Starship prints no text; its face lists the pieces fused into it instead.
    if in_play.starship.is_some() {
        return Text::plain("");
    }
    if let Some(power) = &in_play.power
        && def.id == HEROIC_POWER_ID
    {
        let face = if radiant { &def.radiant } else { &def.base };
        // B3.4: Steady Shot's {shot} is the card's number as it stands (the view's), else as printed.
        let values = in_play
            .params
            .clone()
            .unwrap_or_else(|| printed_values(def, radiant));
        let kinds = |list: &[Keyword]| -> String {
            if reader {
                list.iter()
                    .map(chinese_keyword)
                    .collect::<Vec<_>>()
                    .join(CHINESE_COMMA)
            } else {
                list.iter().map(keyword_key).collect::<Vec<_>>().join(", ")
            }
        };
        let translate = reader.then_some(words);
        if let Some(power_words) = power_text(power, radiant, &kinds(&face.keywords), &values, translate) {
            // R277: a Radiant power is marked against the same power's base words.
            let base_words = if radiant {
                power_text(
                    power,
                    false,
                    &kinds(&def.base.keywords),
                    &printed_values(def, false),
                    translate,
                )
            } else {
                None
            };
            let full: Vec<char> = power_words.chars().collect();
            let marks = base_words.map_or_else(Vec::new, |base| {
                radiant_marks(&base.chars().collect::<Vec<_>>(), &full)
            });
            return Text {
                full,
                marks,
                tuned: Vec::new(),
            };
        }
    }
    printed_text.clone()
}

/// A face's text with its `{key}` numbers filled in (B3.4 rule 5): printed values, or `values` in play. A
/// Radiant face marks what the base text lacks (R277); a fused one's lines against its ingredients' (`diff.rs`).
fn text_of(def: Option<&CardDef>, radiant: bool, values: Option<&IndexMap<String, i32>>) -> Text {
    let Some(def) = def else { return Text::default() };
    let (full, tuned) = filled_text(def, radiant, values);
    let full: Vec<char> = full.chars().collect();
    if !radiant {
        return Text {
            full,
            marks: Vec::new(),
            tuned,
        };
    }
    let base: Vec<char> = filled_text(def, false, None).0.chars().collect();
    let marks = radiant_marks(&base, &full);
    Text { full, marks, tuned }
}

fn cost_tone(live: Option<i32>, printed: i32) -> CostTone {
    match live {
        Some(live) if live < printed => CostTone::Down,
        Some(live) if live > printed => CostTone::Up,
        _ => CostTone::Base,
    }
}

fn cost_of(
    def: Option<&CardDef>,
    live_cost: Option<i32>,
    live_embiggen_cost: Option<i32>,
    fixed_x: Option<i32>,
) -> FaceCost {
    let live = live_cost.map(|cost| cost.to_string());
    let Some(def) = def else {
        let text = live.unwrap_or_else(|| UNKNOWN_COST.to_string());
        return FaceCost {
            value: text.clone(),
            text,
            tone: CostTone::Base,
            alt: None,
        };
    };
    match def.cost {
        CardCost::X => match fixed_x {
            // R1848003: an X card whose X is set costs that X, and the gem prints it.
            Some(x) => FaceCost {
                text: x.to_string(),
                value: live.unwrap_or_else(|| x.to_string()),
                tone: CostTone::Base,
                alt: None,
            },
            // An X card costs exactly X (R65): the gem says X, and `data-cost` carries whatever the view says.
            None => FaceCost {
                text: "X".to_string(),
                value: live.unwrap_or_else(|| "X".to_string()),
                tone: CostTone::Base,
                alt: None,
            },
        },
        CardCost::Fixed(printed) => {
            let text = live.unwrap_or_else(|| printed.to_string());
            FaceCost {
                value: text.clone(),
                text,
                tone: cost_tone(live_cost, printed),
                alt: None,
            }
        }
        // "A embiggen B": with no live number, or the live number at A, the gem shows A with B (the view's
        // `embiggenCost` where it has one, R1181003) beside it. Otherwise it shows the view's number and drops
        // B: the client never works out a discounted embiggen price (CLAUDE.md rule 7). A card whose live
        // cost is B was paid B, its printed price, so the tone is base.
        CardCost::Embiggen { base, embiggen } => match live_cost {
            None => FaceCost {
                text: base.to_string(),
                value: base.to_string(),
                tone: CostTone::Base,
                alt: Some(live_embiggen_cost.unwrap_or(embiggen).to_string()),
            },
            Some(cost) if cost == base => FaceCost {
                text: base.to_string(),
                value: base.to_string(),
                tone: CostTone::Base,
                alt: Some(live_embiggen_cost.unwrap_or(embiggen).to_string()),
            },
            Some(cost) => FaceCost {
                text: cost.to_string(),
                value: cost.to_string(),
                tone: if cost == embiggen {
                    CostTone::Base
                } else {
                    cost_tone(Some(cost), base)
                },
                alt: None,
            },
        },
    }
}

impl FaceCost {
    /// Glitch's gem shows glyphs, never a number; `data-cost` still carries the view's.
    fn with_base(self) -> FaceCost {
        FaceCost {
            tone: CostTone::Base,
            alt: None,
            ..self
        }
    }
}

fn stat_tone(value: i32, printed: Option<i32>) -> StatTone {
    match printed {
        Some(printed) if value > printed => StatTone::Buffed,
        Some(printed) if value < printed => StatTone::Reduced,
        _ => StatTone::Base,
    }
}

fn stats_of(
    type_: CardType,
    known: bool,
    printed: Option<&CardFace>,
    live: Option<&LiveStats>,
    grew: Option<Grew>,
) -> Option<FaceStats> {
    if let Some(live) = live {
        // A unit nobody can name has no printed face to compare against, so nothing is colored.
        if !known {
            return Some(FaceStats {
                attack: live.attack,
                health: live.health,
                max_health: live.max_health,
                attack_tone: StatTone::Base,
                health_tone: StatTone::Base,
                grew: None,
            });
        }
        return Some(FaceStats {
            attack: live.attack,
            health: live.health,
            max_health: live.max_health,
            attack_tone: stat_tone(live.attack, printed.and_then(|face| face.attack)),
            health_tone: if live.health < live.max_health {
                StatTone::Damaged
            } else {
                stat_tone(live.max_health, printed.and_then(|face| face.health))
            },
            grew: None,
        });
    }
    // B3.1 rule 1: an Animated Field Spell or Trap that prints stats prints those of the Unit it becomes
    // (R2280000: it may print none); no other card that is not a Unit prints any.
    let (attack, health) = (
        printed.and_then(|face| face.attack)?,
        printed.and_then(|face| face.health)?,
    );
    if type_ != CardType::Unit && !prints_unit_stats(printed) {
        return None;
    }
    Some(FaceStats {
        attack,
        health,
        max_health: health,
        attack_tone: StatTone::Base,
        health_tone: StatTone::Base,
        grew,
    })
}

/// B3.1: an Animated face (either kind) may print a Unit's attack and health (R2280000).
fn prints_unit_stats(printed: Option<&CardFace>) -> bool {
    printed.is_some_and(|face| {
        face.keywords
            .iter()
            .any(|keyword| matches!(keyword, Keyword::Animated | Keyword::AnimatedOnYourTurn))
    })
}

/// A tag the frame prints: one of the card's, or "All Tribes" in place of every tribal tag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrameTag {
    AllTribes,
    Tag(Tag),
}

impl FrameTag {
    pub fn name(&self) -> String {
        match self {
            FrameTag::AllTribes => ALL_TRIBES.to_string(),
            FrameTag::Tag(tag) => tag.as_str().to_string(),
        }
    }
}

/// The tags the frame prints, `ALL_TRIBES` in place of every tribal tag. The Token tag is never a chip
/// (R1177001): a token's type line says it ("Token Unit"), where no crowded tag row can cut it.
pub fn frame_tags(tags: &[Tag]) -> Vec<FrameTag> {
    let printed: Vec<Tag> = tags.iter().copied().filter(|tag| *tag != Tag::Token).collect();
    if !TRIBAL_TAGS.iter().all(|tribe| printed.contains(tribe)) {
        return printed.into_iter().map(FrameTag::Tag).collect();
    }
    std::iter::once(FrameTag::AllTribes)
        .chain(
            printed
                .into_iter()
                .filter(|tag| !TRIBAL_TAGS.contains(tag))
                .map(FrameTag::Tag),
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::types::{HandStats, Locale};

    fn catalog(id: &str) -> CardDef {
        jackioh_cards::register_all();
        jackioh_cards::CATALOG
            .get(id)
            .unwrap_or_else(|| panic!("the catalog's {id}"))
            .clone()
    }

    /// #2437, R2179001: a card's Temporary count is the largest of its Temporary entries, a plain one counting
    /// `TEMPORARY_LAST_COUNT`; a card with none has none.
    #[test]
    fn r2179001_a_temporary_count_is_the_largest_entry() {
        assert_eq!(temporary_left(&[Keyword::Taunt]), None);
        let three = Keyword::Temporary { n: Some(3) };
        let plain = Keyword::Temporary { n: None };
        assert_eq!(temporary_left(&[plain.clone(), Keyword::Taunt, three]), Some(3));
        assert_eq!(temporary_left(&[plain]), Some(TEMPORARY_LAST_COUNT));
    }

    /// R243: a hand card's stats are what the view says, its keywords its printed face's.
    #[test]
    fn r243_a_hand_cards_stats_are_the_views_and_its_keywords_its_faces() {
        let face = catalog("core-020").base;
        let live = hand_live(Some(HandStats { attack: 7, health: 8 }), Some(&face)).expect("hand stats");
        assert_eq!((live.attack, live.health, live.max_health), (7, 8, 8));
        assert_eq!(live.keywords, face.keywords);
        assert!(hand_live(None, Some(&face)).is_none());
    }

    /// R2521009: a Resource's sale prices are its definition's, on the face it is played on.
    #[test]
    fn r2521009_a_resources_sale_prices_are_its_definitions() {
        let reknite = catalog("stinkyscape-t-reknite");
        let base = credits_of(Some(&reknite), false).expect("a Resource sells");
        let radiant = credits_of(Some(&reknite), true).expect("a Resource sells");
        assert_eq!((base.min, base.max, radiant.min, radiant.max), (1, 3, 2, 6));
        assert!(credits_of(Some(&catalog("core-020")), false).is_none());
    }

    /// R1438: a keyword is gained once, however often the card has it, and only where the printed face lacks it;
    /// Lucky is gained at its sum only where the sum differs from the printed one.
    #[test]
    fn r1438_a_keyword_is_gained_once_and_lucky_at_its_sum() {
        assert_eq!(
            gained_keywords(&[Keyword::Taunt, Keyword::Taunt], &[]),
            vec![Keyword::Taunt]
        );
        assert!(gained_keywords(&[Keyword::Taunt], &[Keyword::Taunt]).is_empty());
        assert!(gained_keywords(&[Keyword::Lucky { n: 2 }], &[Keyword::Lucky { n: 2 }]).is_empty());
        assert_eq!(
            gained_keywords(
                &[Keyword::Lucky { n: 1 }, Keyword::Lucky { n: 2 }],
                &[Keyword::Lucky { n: 1 }]
            ),
            vec![Keyword::Lucky { n: 3 }]
        );
        assert_eq!(
            luck_of(&[Keyword::Lucky { n: 2 }, Keyword::Taunt, Keyword::Lucky { n: 3 }]),
            5
        );
    }

    /// R275: a Radiant face as printed grows where its stats pass the base face's, and not where they only match;
    /// a face in play (on the field or in a hand) is the view's, so none grows.
    #[test]
    fn r275_a_radiant_face_grows_only_past_the_base_faces_stats() {
        let mut def = catalog("core-020");
        def.base.attack = Some(2);
        def.base.health = Some(3);
        def.radiant.attack = Some(2);
        def.radiant.health = Some(4);
        let radiant = FaceSource {
            radiant: true,
            ..FaceSource::default()
        };
        let grew = grew_of(Some(&def), &radiant).expect("a printed Radiant face");
        assert_eq!((grew.attack, grew.health), (false, true));
        def.radiant.attack = Some(3);
        def.radiant.health = Some(3);
        let grew = grew_of(Some(&def), &radiant).expect("a printed Radiant face");
        assert_eq!((grew.attack, grew.health), (true, false));
        let on_the_field = FaceSource {
            live: Some(LiveStats {
                attack: 1,
                health: 1,
                max_health: 1,
                keywords: Vec::new(),
            }),
            ..radiant.clone()
        };
        assert!(grew_of(Some(&def), &on_the_field).is_none());
        let in_hand = FaceSource {
            in_play: Some(InPlay {
                hand_stats: Some(HandStats { attack: 1, health: 1 }),
                ..InPlay::default()
            }),
            ..radiant
        };
        assert!(grew_of(Some(&def), &in_hand).is_none());
        assert!(grew_of(Some(&def), &FaceSource::default()).is_none());
    }

    /// R711000: a card whose text is gone says so in English to a reader of Chinese unless the card is drawn in
    /// Chinese, and in Chinese when it is.
    #[test]
    fn r711000_a_vanilla_text_is_in_chinese_only_on_a_chinese_card_for_a_chinese_reader() {
        let zh = Words::new(Locale::ZhCn);
        let vanilla = InPlay {
            vanilla: Some(true),
            ..InPlay::default()
        };
        let printed = Text::plain("printed");
        assert_eq!(
            text_in_play(zh, None, false, &printed, &vanilla, false).string(),
            VANILLA_TEXT
        );
        assert_eq!(
            text_in_play(zh, None, false, &printed, &vanilla, true).string(),
            zh.say(VANILLA_TEXT)
        );
        assert_ne!(
            zh.say(VANILLA_TEXT),
            VANILLA_TEXT,
            "PREMISE: the table says it in Chinese"
        );
    }

    /// R65: a cost below the printed one is down, above it up, at it base.
    #[test]
    fn r65_a_cost_tone_reads_against_the_printed_cost() {
        assert_eq!(cost_tone(Some(2), 3), CostTone::Down);
        assert_eq!(cost_tone(Some(3), 3), CostTone::Base);
        assert_eq!(cost_tone(Some(4), 3), CostTone::Up);
        assert_eq!(cost_tone(None, 3), CostTone::Base);
    }

    /// R1181003, CLAUDE.md rule 7: "A embiggen B" shows A with B beside it at its base price, and otherwise the view's
    /// number alone: base at B, its printed embiggen price, and read against A elsewhere.
    #[test]
    fn r1181003_an_embiggen_cost_shows_its_second_price_only_at_its_first() {
        let mut def = catalog("core-046");
        def.cost = CardCost::Embiggen { base: 2, embiggen: 5 };
        let at = |live| {
            let cost = cost_of(Some(&def), live, None, None);
            (cost.text, cost.alt, cost.tone)
        };
        assert_eq!(
            at(Some(2)),
            ("2".to_string(), Some("5".to_string()), CostTone::Base)
        );
        assert_eq!(at(Some(5)), ("5".to_string(), None, CostTone::Base));
        assert_eq!(at(Some(4)), ("4".to_string(), None, CostTone::Up));
    }

    /// Issue #170: Glitch's gem is plain, whatever its cost would say.
    #[test]
    fn glitchs_cost_is_always_base_and_alone() {
        let cost = FaceCost {
            text: "2".to_string(),
            value: "2".to_string(),
            tone: CostTone::Up,
            alt: Some("5".to_string()),
        }
        .with_base();
        assert_eq!((cost.tone, cost.alt), (CostTone::Base, None));
    }

    /// R386: a stat above its printed value is buffed, below it reduced, at it (or with none printed) base.
    #[test]
    fn r386_a_stat_tone_reads_against_the_printed_value() {
        assert_eq!(stat_tone(4, Some(3)), StatTone::Buffed);
        assert_eq!(stat_tone(3, Some(3)), StatTone::Base);
        assert_eq!(stat_tone(2, Some(3)), StatTone::Reduced);
        assert_eq!(stat_tone(3, None), StatTone::Base);
    }

    /// R243, B3.1: a known unit's stats are toned against its printed face, its health damaged below its max; a
    /// card that is no Unit prints stats only where its face is Animated.
    #[test]
    fn r243_a_known_units_stats_are_toned_and_only_an_animated_face_off_type_prints_them() {
        let mut face = catalog("core-020").base;
        face.attack = Some(2);
        face.health = Some(3);
        let live = |attack, health, max_health| LiveStats {
            attack,
            health,
            max_health,
            keywords: Vec::new(),
        };
        let tones = |stats: Option<FaceStats>| {
            let stats = stats.expect("stats");
            (stats.attack_tone, stats.health_tone)
        };
        let known = stats_of(CardType::Unit, true, Some(&face), Some(&live(4, 3, 3)), None);
        assert_eq!(tones(known), (StatTone::Buffed, StatTone::Base));
        let damaged = stats_of(CardType::Unit, true, Some(&face), Some(&live(2, 2, 3)), None);
        assert_eq!(tones(damaged), (StatTone::Base, StatTone::Damaged));
        let unknown = stats_of(CardType::Unit, false, Some(&face), Some(&live(4, 2, 3)), None);
        assert_eq!(tones(unknown), (StatTone::Base, StatTone::Base));
        face.keywords = vec![Keyword::Taunt];
        assert!(stats_of(CardType::Spell, true, Some(&face), None, None).is_none());
        face.keywords = vec![Keyword::Animated];
        assert!(stats_of(CardType::FieldSpell, true, Some(&face), None, None).is_some());
    }

    /// R1177001: the frame prints a card's tags, never the Token tag, and "All Tribes" in place of every tribal tag
    /// where it has them all.
    #[test]
    fn r1177001_the_frame_prints_all_tribes_for_every_tribe_and_never_token() {
        assert_eq!(
            frame_tags(&[Tag::Human, Tag::Token, Tag::Fruit]),
            vec![FrameTag::Tag(Tag::Human), FrameTag::Tag(Tag::Fruit)]
        );
        let every: Vec<Tag> = TRIBAL_TAGS
            .iter()
            .copied()
            .chain([Tag::Fruit, Tag::Token])
            .collect();
        assert_eq!(
            frame_tags(&every),
            vec![FrameTag::AllTribes, FrameTag::Tag(Tag::Fruit)]
        );
        assert_eq!(FrameTag::AllTribes.name(), ALL_TRIBES);
        assert_eq!(FrameTag::Tag(Tag::Fruit).name(), "Fruit");
    }
}
