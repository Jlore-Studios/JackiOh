//! Meditative #40 Feng Shui (SPEC §8.8 row 40, R980–R987, BUILD M10 row M 40). (2) Field
//! Spell, CN, Legendary.
//!   Base:    "Aura: Every card has an element: 水, 木, 火, 金 or 土. / Auspicious behavior will be
//!            rewarded and inauspicious behavior will be punished. / Your last element and your
//!            opponent's last element are shown here. / Aura: You have Luck {luck}."
//!   Radiant: line 2 becomes "Auspicious behavior will be rewarded for you, and inauspicious behavior
//!            will be punished heavily for your opponent."
//!   Engine:  ME-ELEMENT (`engine/src/subsystems/feng_shui.rs`): the card judges every face-up play at
//!            §10.5 step 3 by its element against its player's last (R981–R984), shows elements on
//!            readable cards while it acts (R980), and shows both records through `preview` (R982);
//!            ME-LUCK: "You have Luck {luck}" (R987). Tunes: luck 1 ↑ (never below 1).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-040";

/// The declared number `luck` (R987): "Aura: You have Luck {luck}".
const LUCK: i32 = 1;

/// R982: the two "last element" preview labels — exact substrings of line 3 (R280).
const YOUR_LAST: &str = "Your last element";
const THEIR_LAST: &str = "your opponent's last element";

/// R982: both records through `preview` (R280), the glyph as their `display` (R372), and "—" before
/// any play. No zone guard: the records read the same wherever the card is read.
fn preview() -> PreviewHook {
    condition_hook(|ctx| {
        vec![
            subsystems::feng_shui::element_preview(
                YOUR_LAST,
                subsystems::feng_shui::last_element(ctx.state, ctx.controller),
            ),
            subsystems::feng_shui::element_preview(
                THEIR_LAST,
                subsystems::feng_shui::last_element(ctx.state, opponent_of(ctx.controller)),
            ),
        ]
    })
}

fn face() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            feng_shui: Some(true),
            luck: Some(LUCK),
            ..StaticFlags::default()
        }),
        preview: Some(preview()),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(),
        radiant: face(),
    }
}

// Meditative #40 Feng Shui — SPEC §8.8 row 40, R980–R987, BUILD M10 row M 40: "(2) Field Spell, CN,
// Legendary. Aura: every card has an element (水, 木, 火, 金 or 土); auspicious behavior rewarded
// (Radiant at §10.5 step 3) and inauspicious punished (Brittle 2 there, 10 to the hero once the play
// has resolved, 20 on the Radiant face); the last elements shown here; Luck 1."
//
// Every game is built with the testkit's `scenario()` after `crate::register_all()`, with the
// Meditative set previewed (the record is written only while it is open, R982) and mana 10. The
// elements ride the indices: core-003 is 木, core-032 火, core-020 土, core-011 水, core-019 金.
#[cfg(test)]
mod tests {
    use super::{ID, THEIR_LAST, YOUR_LAST};
    use crate::js;
    use jackioh_engine::testkit::*;
    use jackioh_engine::wire::PlayerId::{P1, P2};

    /// (1) Unit, index 3: 木.
    const WOOD: &str = "core-003";
    /// (1) Unit, index 11: 水.
    const WATER: &str = "core-011";
    /// (2) Unit, index 20: 土.
    const EARTH: &str = "core-020";
    /// (2) Unit, index 32: 火.
    const FIRE: &str = "core-032";

    /// Lane 1 of p1's backrow as `viewer` sees it, failing unless it is the public Feng Shui.
    fn feng_shui_in(s: &Scenario, viewer: PlayerId) -> PublicBackrowView {
        let view = s.view(viewer);
        let side = if viewer == P1 { view.you } else { view.opponent };
        match side.backrow.into_iter().next().flatten() {
            Some(BackrowView::Public(public)) if !public.face_down => {
                assert_eq!(public.def_id, ID);
                public
            }
            _ => panic!("Feng Shui must be public in lane 1 (§10.8)"),
        }
    }

    /// A hand card's element as `viewer` reads it, or `None` for no chip.
    fn hand_element(s: &Scenario, viewer: PlayerId, def_id: &str) -> Option<CardElement> {
        match s.view(viewer).you.hand {
            HandView::Cards(cards) => cards
                .iter()
                .find(|card| card.def_id == def_id)
                .and_then(|card| card.element),
            _ => None,
        }
    }

    /// How many `fengShui` judgements the game has emitted so far.
    fn judged(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| event.event_type().as_str() == "fengShui")
            .count()
    }

    #[test]
    fn r985_r986_a_2_cost_legendary_cn_field_spell_whose_only_param_is_luck() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.cost, CardCost::Fixed(2));
        assert_eq!(def.type_, CardType::FieldSpell);
        assert_eq!(def.tags, vec![Tag::Cn]);
        assert_eq!(def.rarity, Rarity::Legendary);
        assert!(!def.token);
        let params = def.params.clone().unwrap_or_default();
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].key, "luck");
        assert_eq!([params[0].base, params[0].radiant], [1, 1]);
    }

    #[test]
    fn r980_a_hand_card_shows_its_element_while_feng_shui_stands() {
        crate::register_all();
        let _open = preview_sets(&[SetName::Meditative]);
        let s = scenario(json!({
            "seed": "meditative-040-r980-judge",
            "p1": { "mana": 10, "backrow": [ID], "hand": [WOOD] },
            "p2": { "hand": ["core-005"] },
        }));
        assert_eq!(hand_element(&s, P1, WOOD), Some(CardElement::Wood));
        let bare = scenario(json!({
            "seed": "meditative-040-r980-bare",
            "p1": { "mana": 10, "hand": [WOOD] },
            "p2": { "hand": ["core-005"] },
        }));
        assert_eq!(hand_element(&bare, P1, WOOD), None);
    }

    #[test]
    fn r981_r983_a_generating_play_resolves_radiant() {
        crate::register_all();
        let _open = preview_sets(&[SetName::Meditative]);
        let mut s = scenario(json!({
            "seed": "meditative-040-generating",
            "p1": { "mana": 10, "backrow": [ID], "hand": [WOOD, FIRE] },
            "p2": { "hand": ["core-005"] },
        }));
        // Wood first: a first play is neutral, and writes the record.
        s.play(WOOD, json!({}));
        assert_eq!(judged(&s), 0);
        // Wood generates Fire: auspicious, rewarded.
        s.play(FIRE, json!({}));
        assert!(s.card(FIRE).radiant);
        assert_eq!(judged(&s), 1);
    }

    #[test]
    fn r981_r983_an_overcoming_play_gets_brittle_2_and_costs_10() {
        crate::register_all();
        let _open = preview_sets(&[SetName::Meditative]);
        let mut s = scenario(json!({
            "seed": "meditative-040-overcoming",
            "p1": { "mana": 10, "backrow": [ID], "hand": [WOOD, EARTH] },
            "p2": { "hand": ["core-005"] },
        }));
        s.play(WOOD, json!({}));
        // Wood is overcome by Earth: inauspicious, punished.
        s.play(EARTH, json!({}));
        let earth = s.card(EARTH).clone();
        assert!(!earth.radiant);
        assert_eq!(earth.brittle.as_ref().map(|brittle| brittle.count), Some(2));
        s.expect_health(P1, 20);
        assert_eq!(judged(&s), 1);
    }

    #[test]
    fn r981_a_neutral_pair_does_nothing() {
        crate::register_all();
        let _open = preview_sets(&[SetName::Meditative]);
        let mut s = scenario(json!({
            "seed": "meditative-040-neutral",
            "p1": { "mana": 10, "backrow": [ID], "hand": [WOOD, WATER] },
            "p2": { "hand": ["core-005"] },
        }));
        s.play(WOOD, json!({}));
        s.play(WATER, json!({}));
        assert!(!s.card(WATER).radiant);
        assert_eq!(judged(&s), 0);
        s.expect_health(P1, 30);
    }

    #[test]
    fn r983_feng_shui_does_not_judge_its_own_play() {
        crate::register_all();
        let _open = preview_sets(&[SetName::Meditative]);
        let mut s = scenario(json!({
            "seed": "meditative-040-own-play",
            "p1": { "mana": 10, "hand": [WOOD, ID] },
            "p2": { "hand": ["core-005"] },
        }));
        s.play(WOOD, json!({}));
        // Wood is overcome by Earth (Feng Shui itself is 土), which would punish — but the judge is
        // not on the field at step 3 of its own play.
        s.play(ID, json!({}));
        assert_eq!(judged(&s), 0);
        let judge = s.card(ID).clone();
        assert!(!judge.radiant);
        assert_eq!(judge.brittle.as_ref().map(|brittle| brittle.count), None);
        s.expect_health(P1, 30);
    }

    #[test]
    fn r984_the_base_face_judges_the_opponent() {
        crate::register_all();
        let _open = preview_sets(&[SetName::Meditative]);
        let mut s = scenario(json!({
            "seed": "meditative-040-base-opponent",
            "active": "p2",
            "p1": { "backrow": [ID] },
            "p2": { "mana": 10, "hand": [WOOD, EARTH] },
        }));
        s.play(WOOD, json!({}));
        s.play(EARTH, json!({}));
        assert_eq!(judged(&s), 1);
        assert_eq!(
            s.card(EARTH).brittle.as_ref().map(|brittle| brittle.count),
            Some(2)
        );
        s.expect_health(P2, 20);
    }

    #[test]
    fn r984_the_radiant_face_rewards_you_and_punishes_the_opponent_for_20() {
        crate::register_all();
        let _open = preview_sets(&[SetName::Meditative]);
        let mut s = scenario(json!({
            "seed": "meditative-040-radiant-reward",
            "p1": { "mana": 10, "backrow": [{ "def": ID, "radiant": true }], "hand": [WOOD, FIRE] },
            "p2": { "hand": ["core-005"] },
        }));
        s.play(WOOD, json!({}));
        s.play(FIRE, json!({}));
        assert!(s.card(FIRE).radiant);
        assert_eq!(judged(&s), 1);
        let mut s = scenario(json!({
            "seed": "meditative-040-radiant-punish",
            "active": "p2",
            "p1": { "backrow": [{ "def": ID, "radiant": true }] },
            "p2": { "mana": 10, "hand": [WOOD, EARTH] },
        }));
        s.play(WOOD, json!({}));
        s.play(EARTH, json!({}));
        assert_eq!(
            s.card(EARTH).brittle.as_ref().map(|brittle| brittle.count),
            Some(2)
        );
        s.expect_health(P2, 10);
        assert_eq!(judged(&s), 1);
    }

    #[test]
    fn r280_r982_the_last_element_lines_show_a_dash_then_the_glyph() {
        crate::register_all();
        let _open = preview_sets(&[SetName::Meditative]);
        for radiant in [false, true] {
            let mut s = scenario(json!({
                "seed": format!("meditative-040-lines-{radiant}"),
                "p1": { "mana": 10, "backrow": [{ "def": ID, "radiant": radiant }], "hand": [WOOD] },
                "p2": { "hand": ["core-005"] },
            }));
            let def = crate::card_def(ID);
            let text = if radiant {
                &def.radiant.text
            } else {
                &def.base.text
            };
            for viewer in [P1, P2] {
                let preview = feng_shui_in(&s, viewer).preview;
                assert_eq!(
                    js(&preview),
                    json!([
                        { "label": YOUR_LAST, "value": 0, "display": "—" },
                        { "label": THEIR_LAST, "value": 0, "display": "—" },
                    ]),
                    "{viewer} reads two dashes before any play"
                );
                for entry in preview.unwrap_or_default() {
                    assert!(text.contains(&entry.label), "{}", entry.label);
                }
            }
            s.play(WOOD, json!({}));
            assert_eq!(
                js(&feng_shui_in(&s, P1).preview),
                json!([
                    { "label": YOUR_LAST, "value": 3, "display": "木" },
                    { "label": THEIR_LAST, "value": 0, "display": "—" },
                ])
            );
        }
    }

    #[test]
    fn r987_luck_1_for_its_controller_alone_tuned_by_its_param() {
        crate::register_all();
        let _open = preview_sets(&[SetName::Meditative]);
        let mut s = scenario(json!({
            "seed": "meditative-040-luck",
            "p1": { "mana": 10, "backrow": [ID] },
            "p2": { "hand": ["core-005"] },
        }));
        assert_eq!(luck_of(s.state(), P1), 1);
        assert_eq!(luck_of(s.state(), P2), 0);
        step_param(s.card_mut(ID), "luck", 1);
        assert_eq!(luck_of(s.state(), P1), 2);
    }
}
