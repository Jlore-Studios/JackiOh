//! Meditative #40 Feng Shui (SPEC §8.8 row 40, R980–R987, BUILD M10 row M 40). (2) Field
//! Spell, CN, Legendary.
//!   Base:    "Aura: Cards have elements (水, 木, 火, 金, 土); good feng shui is rewarded, bad punished. You
//!            have Luck {luck}. / Your last element, your opponent's last element."
//!   Radiant: line 1's "good feng shui is rewarded, bad punished" becomes "good feng shui rewards you,
//!            bad hurts your opponent heavily"
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
    use jackioh_engine::effects::cast_new;
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

    /// `effects` applied for p1 on a sink over the scenario's state, then settled; their events.
    fn apply_for_p1(s: &mut Scenario, effects: &[Effect]) -> Vec<GameEvent> {
        let mut rng = create_rng(&s.state().seed, s.state().rng_cursor);
        let mut events: Vec<GameEvent> = Vec::new();
        {
            let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
            {
                let mut ctx = make_context(
                    &mut sink,
                    None,
                    HookOptions {
                        controller: Some(P1),
                        ..HookOptions::default()
                    },
                );
                apply_effects(effects, &mut ctx);
            }
            settle(&mut sink, SettleOptions::default());
        }
        s.state_mut().rng_cursor = rng.cursor();
        events
    }

    fn last(s: &Scenario, player: PlayerId) -> Option<CardElement> {
        jackioh_engine::subsystems::feng_shui::last_element(s.state(), player)
    }

    /// p1 holds `hand` with Feng Shui (base) standing in its backrow, mana 10.
    fn judged_game(seed: &str, hand: Value) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": { "mana": 10, "backrow": [ID], "hand": hand, "library": ["core-005", "core-005"] },
            "p2": { "hand": ["core-005"], "library": ["core-005", "core-005"] },
        }))
    }

    #[test]
    fn r980_elements_follow_the_hetu_rule_on_the_index_s_last_digit() {
        use jackioh_engine::subsystems::feng_shui::{element_of, element_of_index};
        crate::register_all();
        // 1 and 6 水, 2 and 7 火, 3 and 8 木, 4 and 9 金, 5 and 0 土.
        let hetu = [
            ("1", CardElement::Water),
            ("6", CardElement::Water),
            ("2", CardElement::Fire),
            ("17", CardElement::Fire),
            ("3", CardElement::Wood),
            ("28", CardElement::Wood),
            ("4", CardElement::Metal),
            ("39", CardElement::Metal),
            ("5", CardElement::Earth),
            ("40", CardElement::Earth),
            ("39.2", CardElement::Fire),
        ];
        for (index, element) in hetu {
            assert_eq!(element_of_index(index), element, "index {index}");
        }
        let s = judged_game("meditative-040-hetu", json!([WOOD]));
        for (def_id, element) in [
            (WOOD, CardElement::Wood),
            (WATER, CardElement::Water),
            (EARTH, CardElement::Earth),
            (FIRE, CardElement::Fire),
            (ID, CardElement::Earth),
        ] {
            assert_eq!(element_of(s.state(), def_id), element, "{def_id}");
        }
    }

    #[test]
    fn r981_r982_a_cast_is_judged_and_recorded() {
        // Wood is recorded; a cast Conjure KY (index 57: 火) is judged against it and rewarded.
        const CONJURE_KY: &str = "core-057";
        let mut s = judged_game("meditative-040-cast", json!([WOOD, "core-005"]));
        s.play(WOOD, json!({}));
        let events = apply_for_p1(&mut s, &[cast_new(json_as(json!({ "def": CONJURE_KY })))]);
        let verdicts: Vec<FengShuiOutcome> = events
            .iter()
            .filter_map(|event| match event {
                GameEvent::FengShui {
                    outcome, player: P1, ..
                } => Some(*outcome),
                _ => None,
            })
            .collect();
        assert_eq!(verdicts, vec![FengShuiOutcome::Positive]);
        assert_eq!(last(&s, P1), Some(CardElement::Fire), "the cast is recorded");
    }

    #[test]
    fn r982_the_record_is_written_with_nothing_previewed_since_the_set_shipped() {
        assert!(set_ships(SetName::Meditative));
        let mut s = judged_game("meditative-040-shipped", json!([WOOD, FIRE]));
        assert_eq!(last(&s, P1), None);
        s.play(WOOD, json!({}));
        assert_eq!(last(&s, P1), Some(CardElement::Wood));
        s.play(FIRE, json!({}));
        assert_eq!(last(&s, P1), Some(CardElement::Fire));
        assert_eq!(judged(&s), 1);
    }

    #[test]
    fn r981_a_first_play_does_nothing() {
        let mut s = judged_game("meditative-040-first", json!([EARTH, "core-005"]));
        s.play(EARTH, json!({}));
        assert_eq!(judged(&s), 0);
        let earth = s.card(EARTH).clone();
        assert!(!earth.radiant);
        assert_eq!(earth.brittle.as_ref().map(|brittle| brittle.count), None);
        s.expect_health(P1, 30);
        assert_eq!(last(&s, P1), Some(CardElement::Earth), "but it is recorded");
    }

    #[test]
    fn r983_the_hit_lands_at_step_7_after_card_resolved() {
        let mut s = judged_game("meditative-040-order", json!([WOOD, EARTH]));
        s.play(WOOD, json!({}));
        let mark = s.events().len();
        s.play(EARTH, json!({}));
        let events = &s.events()[mark..];
        let resolved = events
            .iter()
            .rposition(|event| matches!(event, GameEvent::CardResolved { .. }))
            .expect("the play resolved");
        let hit = events
            .iter()
            .position(|event| matches!(event, GameEvent::Damage { .. }))
            .expect("the hit");
        assert!(resolved < hit, "{events:?}");
        s.expect_health(P1, 20);
    }

    #[test]
    fn r982_a_face_down_trap_play_is_neither_judged_nor_recorded() {
        // Sheepish, index 41 (水), would be generated by nothing and overcome nothing from 木 — but
        // it is set face-down, so it is not judged at all and the record stays 木.
        const SHEEPISH: &str = "core-041";
        let mut s = judged_game("meditative-040-trap", json!([WOOD, SHEEPISH]));
        s.play(WOOD, json!({}));
        s.play(SHEEPISH, json!({ "zone": 2, "row": "backrow" }));
        assert_eq!(judged(&s), 0);
        assert_eq!(last(&s, P1), Some(CardElement::Wood));
    }

    #[test]
    fn r983_a_countered_play_keeps_step_3_but_is_not_recorded_and_takes_no_damage() {
        // p2's face-down Counterspell counters p1's Spells.
        const COUNTERSPELL: &str = "classic-017";
        const CONJURE_KY: &str = "core-057"; // index 57: 火, generated by 木.
        const STOCKPILE: &str = "core-005"; // index 5: 土, overcome by 木.
        for (spell, rewarded) in [(CONJURE_KY, true), (STOCKPILE, false)] {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": format!("meditative-040-countered-{spell}"),
                "p1": { "mana": 10, "backrow": [ID], "hand": [WOOD, spell, "core-008"], "library": ["core-008"] },
                "p2": {
                    "hand": ["core-008"],
                    "backrow": [{ "def": COUNTERSPELL, "lane": 1 }],
                    "library": ["core-008"],
                },
            }));
            s.play(WOOD, json!({}));
            let played = s
                .hand(P1)
                .into_iter()
                .find(|card| card.def_id == spell)
                .expect("the Spell")
                .id;
            s.play(&played, json!({}));
            assert!(
                s.events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::Countered { .. })),
                "{spell} is countered"
            );
            let card = s.card(&played).clone();
            if rewarded {
                assert!(card.radiant, "the reward stays on the countered card");
            } else {
                assert_eq!(card.brittle.as_ref().map(|brittle| brittle.count), Some(2));
            }
            assert_eq!(last(&s, P1), Some(CardElement::Wood), "not recorded");
            s.expect_health(P1, 30);
        }
    }

    #[test]
    fn r984_two_feng_shuis_punish_twice() {
        crate::register_all();
        let mut s = scenario(json!({
            "seed": "meditative-040-twice",
            "p1": { "mana": 10, "backrow": [ID, ID], "hand": [WOOD, EARTH] },
            "p2": { "hand": ["core-005"] },
        }));
        s.play(WOOD, json!({}));
        s.play(EARTH, json!({}));
        assert_eq!(judged(&s), 2);
        s.expect_health(P1, 10);
    }

    #[test]
    fn r982_the_feng_shui_event_is_public() {
        let mut s = judged_game("meditative-040-public", json!([WOOD, FIRE]));
        let judge = s.backrow(P1, 1).expect("the judge").id;
        s.play(WOOD, json!({}));
        s.play(FIRE, json!({}));
        let fire = s.card(FIRE).id.clone();
        let wanted = GameEvent::FengShui {
            instance_id: fire,
            source_id: judge,
            player: P1,
            outcome: FengShuiOutcome::Positive,
        };
        for viewer in [P1, P2] {
            assert!(
                s.view(viewer).events.contains(&wanted),
                "{viewer} reads the judgement whole"
            );
        }
    }

    #[test]
    fn r985_the_hidden_numbers_are_engine_config() {
        assert_eq!((FENG_SHUI_DAMAGE.base, FENG_SHUI_DAMAGE.radiant), (10, 20));
        assert_eq!(FENG_SHUI_BRITTLE, 2);
        assert_eq!(HETU_ELEMENTS.len(), 10);
        // The card's text states none of them: its only number is `luck`.
        crate::register_all();
        let def = crate::card_def(ID);
        for text in [&def.base.text, &def.radiant.text] {
            assert!(
                !text.contains("10") && !text.contains("20") && !text.contains("Brittle"),
                "{text}"
            );
        }
    }

    #[test]
    fn r986_elements_match_no_tag_filter() {
        for glyph in [
            "水", "火", "木", "金", "土", "Water", "Fire", "Wood", "Metal", "Earth",
        ] {
            assert!(glyph.parse::<Tag>().is_err(), "{glyph} is no tag");
        }
        // While it stands, a card's tags are its printed ones.
        let s = judged_game("meditative-040-tags", json!([WOOD]));
        let wood = s.card(WOOD).clone();
        assert_eq!(tags_of(s.state(), &wood), crate::card_def(WOOD).tags);
    }

    #[test]
    fn r987_luck_adds_a_roll_to_best_of_rolls_but_not_to_a_pool_pick() {
        crate::register_all();
        const ROCK: &str = "meditative-039-1";
        const ROLLS: [&str; 3] = ["meditative-039-3", "meditative-039-2", "meditative-039-4"];
        let rank = |s: &Scenario| -> usize {
            s.hand(P1)
                .iter()
                .find_map(|card| ROLLS.iter().position(|id| *id == card.def_id))
                .expect("a roll")
        };
        let (mut worse_never, mut better_once) = (true, false);
        for n in 0..30 {
            let seed = format!("meditative-040-luck-roll-{n}");
            let mut plain = scenario(
                json!({ "seed": seed, "p1": { "hand": [ROCK, "core-005"] }, "p2": { "hand": ["core-005"] } }),
            );
            plain.play(ROCK, json!({}));
            let mut lucky = scenario(json!({
                "seed": seed,
                "p1": { "hand": [ROCK, "core-005"], "backrow": [ID] },
                "p2": { "hand": ["core-005"] },
            }));
            lucky.play(ROCK, json!({}));
            worse_never &= rank(&lucky) >= rank(&plain);
            better_once |= rank(&lucky) > rank(&plain);
        }
        assert!(
            worse_never && better_once,
            "Luck 1 is a second roll, the better kept"
        );
        // A pool pick is no roll: H1B Printer prints the same card with Feng Shui standing or not.
        const PRINTER: &str = "meditative-038";
        let printed = |with_judge: bool| -> Vec<String> {
            let mut backrow = vec![json!({ "def": PRINTER, "lane": 1 })];
            if with_judge {
                backrow.push(json!({ "def": ID, "lane": 2 }));
            }
            let mut s = scenario(json!({
                "seed": "meditative-040-luck-pool",
                "p1": { "hand": ["core-005"], "backrow": backrow, "library": ["core-005", "core-005"] },
                "p2": { "hand": ["core-005"], "library": ["core-005", "core-005"] },
            }));
            s.start_turn();
            s.events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::AddedToHand { def_id, .. } => Some(def_id.clone()),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(printed(true), printed(false));
    }

    #[test]
    fn r984_the_radiant_face_never_punishes_you_nor_rewards_the_opponent() {
        crate::register_all();
        let mut s = scenario(json!({
            "seed": "meditative-040-radiant-only-mine",
            "p1": { "mana": 10, "backrow": [{ "def": ID, "radiant": true }], "hand": [WOOD, EARTH] },
            "p2": { "hand": ["core-005"] },
        }));
        s.play(WOOD, json!({}));
        s.play(EARTH, json!({}));
        assert_eq!(judged(&s), 0, "your overcoming play goes unpunished");
        assert_eq!(s.card(EARTH).brittle.as_ref().map(|brittle| brittle.count), None);
        s.expect_health(P1, 30);
        let mut s = scenario(json!({
            "seed": "meditative-040-radiant-only-theirs",
            "active": "p2",
            "p1": { "backrow": [{ "def": ID, "radiant": true }] },
            "p2": { "mana": 10, "hand": [WOOD, FIRE] },
        }));
        s.play(WOOD, json!({}));
        s.play(FIRE, json!({}));
        assert_eq!(judged(&s), 0, "the opponent's generating play goes unrewarded");
        assert!(!s.card(FIRE).radiant);
    }
}
