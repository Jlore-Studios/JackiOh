//! What a face in play prints where the card in play and the card as printed part ways (SPEC §10.10).
//! The collection prints the catalog definition, both faces in full; a game prints the card as the view
//! says it stands (R243), and three cards differ there:
//! - #98 Heroic Power prints only the power it rolled as it arrived (R43, R151, R752), read off the
//!   view's `power` (a hand card's `CardView.power`, a backrow card's `HeroPowerView`), name and X.
//! - A Call to Chaos card (Core #95, Classic+ #73, Meditative #95) reads "???" in play: what it does is
//!   rolled when it resolves (§8 #95); the collection prints the real text for deckbuilding. The Classic+
//!   and Meditative Radiant faces read "!!!" for their three effects (docs/classic-sets.md B7 #73).
//! - A unit a Vanilla took the text of (§6.3, R115) prints that its text is gone: the definition still
//!   names the keywords and scripts it lost, and the view says so (`UnitView.vanilla`, R243).
//!
//! Presentation only (CLAUDE.md rule 7). The powers' words (`data/power-words.json`) are written from
//! §8 #98's clauses and keyed by the power's name in the view; a test holds them to the engine's own table.

use jackioh_engine::wire::{CardDef, Tag};

use super::data::{POWERS, PowerRow};
use super::types::RolledPower;
use super::words::{Words, fill};

pub const HEROIC_POWER_ID: &str = "core-098";

/// §5: the tag whose cards read [`CONCEALED_TEXT`] in play (#95 Call to Chaos).
pub const CONCEALED_TAG: Tag = Tag::CallToChaos;

pub const CONCEALED_TEXT: &str = "???";

pub const CONCEALED_TEXT_LOUD: &str = "!!!";

/// The Radiant faces of these read [`CONCEALED_TEXT_LOUD`].
const LOUD_ON_RADIANT: [&str; 2] = ["classicplus-073", "meditative-095"];

/// What a card that is concealed in play reads (R436 keeps the real text in the collection).
pub fn concealed_text(def_id: &str, radiant: bool) -> &'static str {
    if radiant && LOUD_ON_RADIANT.contains(&def_id) {
        CONCEALED_TEXT_LOUD
    } else {
        CONCEALED_TEXT
    }
}

/// What a Vanilla unit's rules box reads (§6.3 Vanilla: "remove a unit's text").
pub const VANILLA_TEXT: &str = "Vanilla: its text is gone";

/// A card with the Call to Chaos tag keeps its text a mystery in play.
pub fn concealed_in_play(tags: &[Tag]) -> bool {
    tags.contains(&CONCEALED_TAG)
}

fn power_of(power: &RolledPower) -> Option<&'static PowerRow> {
    POWERS.iter().find(|row| row.name == power.name)
}

/// A Heroic Power's text in play (R752, R366's layout): its keyword line, the rolled power's name, then on
/// its own line its Activate, X and words, `{…}` numbers filled from `values`. `None` for a name this table
/// does not know, which leaves the printed text in place. `translate` (R711000): name and words in the
/// reader's language; the English is what the glossary's terms are found in, so it is never translated.
pub fn power_text(
    power: &RolledPower,
    radiant: bool,
    keyword_line: &str,
    values: &indexmap::IndexMap<String, i32>,
    translate: Option<Words>,
) -> Option<String> {
    let entry = power_of(power)?;
    let say = |english: &str| translate.map_or_else(|| english.to_string(), |words| words.say(english));
    let params: Vec<(&str, String)> = values
        .iter()
        .map(|(key, value)| (key.as_str(), value.to_string()))
        .collect();
    let words = fill(&say(if radiant { entry.radiant } else { entry.base }), &params);
    let title = say(if radiant { entry.radiant_title } else { entry.title });
    let line = match translate {
        Some(reader) => reader.say_with(
            "Activate: Spend ({x}): {words}",
            &[("x", entry.x.to_string()), ("words", words)],
        ),
        None => format!("Activate: Spend ({}): {words}", entry.x),
    };
    let clause = format!("{title}\n{line}");
    Some(if keyword_line.is_empty() {
        clause
    } else {
        format!("{keyword_line}\n{clause}")
    })
}

/// A definition's declared numbers as printed on a face (B3.4 rule 5), by key.
pub fn printed_values(def: &CardDef, radiant: bool) -> indexmap::IndexMap<String, i32> {
    def.params
        .iter()
        .flatten()
        .map(|param| {
            (
                param.key.clone(),
                if radiant { param.radiant } else { param.base },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::types::Locale;
    use jackioh_engine::subsystems::HERO_POWERS;

    fn power(name: &str) -> RolledPower {
        RolledPower {
            name: name.to_string(),
        }
    }

    #[test]
    fn the_table_is_the_engines_thirteen_powers() {
        assert_eq!(POWERS.len(), HERO_POWERS.len());
        for engine in HERO_POWERS {
            let row = POWERS
                .iter()
                .find(|row| row.name == engine.name.as_str())
                .unwrap_or_else(|| panic!("{}", engine.name.as_str()));
            assert_eq!(
                (row.x, row.title, row.radiant_title, row.base, row.radiant),
                (
                    engine.x,
                    engine.title,
                    engine.radiant_title,
                    engine.label,
                    engine.radiant_label
                ),
                "{}",
                engine.name.as_str()
            );
        }
    }

    #[test]
    fn it_prints_the_keyword_line_then_the_one_power_and_its_activate() {
        let none = indexmap::IndexMap::new();
        assert_eq!(
            power_text(&power("recruit"), false, "Indestructible", &none, None).as_deref(),
            Some("Indestructible\nExpedition Map\nActivate: Spend (3): Recruit a permanent.")
        );
        assert_eq!(
            power_text(&power("discover"), true, "Indestructible", &none, None).as_deref(),
            Some("Indestructible\nWitness Value\nActivate: Spend (2): Discover a Radiant Unit.")
        );
        let armor = [("armor".to_string(), 4)].into_iter().collect();
        assert_eq!(
            power_text(&power("armor"), true, "", &armor, None).as_deref(),
            Some("Tank Up\nActivate: Spend (1): Your hero gains 4 Armor, then this power refreshes.")
        );
        assert_eq!(power_text(&power("not-a-power"), false, "", &none, None), None);
    }

    #[test]
    fn a_reader_of_chinese_reads_the_name_and_the_words() {
        let none = indexmap::IndexMap::new();
        let text = power_text(
            &power("recruit"),
            false,
            "",
            &none,
            Some(Words::new(Locale::ZhCn)),
        )
        .expect("a power");
        assert_eq!(text, "探险地图\n激活：消耗(3)：招募一个永久物。");
    }

    #[test]
    fn each_power_writes_the_numbers_the_catalog_gives_that_power_and_only_those() {
        jackioh_cards::register_all();
        let def = jackioh_engine::catalog::find_def(None, HEROIC_POWER_ID).expect("#98 is in the catalog");
        assert_eq!(def.index, "98");
        for row in POWERS {
            let written: std::collections::BTreeSet<&str> = [row.base, row.radiant]
                .iter()
                .flat_map(|text| text.split('{').skip(1).filter_map(|rest| rest.split('}').next()))
                .collect();
            let own: std::collections::BTreeSet<&str> = def
                .params
                .iter()
                .flatten()
                .filter(|param| param.power.as_deref() == Some(row.name))
                .map(|param| param.key.as_str())
                .collect();
            assert_eq!(written, own, "{}", row.name);
        }
    }

    #[test]
    fn call_to_chaos_reads_three_marks_in_play_and_the_loud_editions_four_on_radiant() {
        assert_eq!(concealed_text("core-095", true), "???");
        assert_eq!(concealed_text("classicplus-073", false), "???");
        assert_eq!(concealed_text("classicplus-073", true), "!!!");
        assert_eq!(concealed_text("meditative-095", true), "!!!");
    }
}
