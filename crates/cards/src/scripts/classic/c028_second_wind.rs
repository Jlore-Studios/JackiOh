//! C #28 Second Wind (SPEC §8.6 row 28, §6.2 Replacement, §6.3 Play; R1, R3, R65, R78, R393).
//! Field Spell, cost 0, Legendary.
//!   Base:    "Cry: Exile your deck. Discard your hand.\nAura: You may play cards from your graveyard
//!            that cost ({minCost}) or more. Cards that would go to your graveyard are exiled instead."
//!            (balance patch 1: the base face's permission needs the same minimum price)
//!   Radiant: "Cry: Exile your deck. Discard your hand.\nAura: You may play cards from your graveyard
//!            that cost ({minCost}) or more."
//!   Engine:  "Play from the graveyard (§6.3 Play) for every card type while this is on the field: such
//!            a play costs, chooses and counts as one from hand, fires its Cry (R1) and takes R65's
//!            player discounts. The base face adds a replacement (§6.2 Replacement) at the "would go to
//!            a graveyard" point for cards you own: they are exiled instead. Both faces' play from the
//!            graveyard needs a price of at least (1) as it would be paid, which stops a loop of free
//!            plays. The Cry's own discard lands in the graveyard before the Aura starts exiling, so the
//!            discarded hand is playable (R393), the reading that gives the card its name. With no deck
//!            left, every draw is fatigue (§2.4). Tunes: minimum price 1 ↓."
//!
//! THE CRY exiles your whole deck, top to bottom (`exileMatching` over the library with no cost
//! filter: each card its own exile, R135), then discards your hand (`discardHand`, a discard of each
//! card, R16's "whole hand" needing no choice).
//!
//! THE PERMISSION is `Script.graveyardPlay` (B5 E11): while the card acts on the field its controller
//! may play any card of their own graveyard, which `legalActions` offers and §10.5 takes from there as
//! from a hand — its cost, its choices, R65's player discounts, its Cry, its count as a play. The
//! Radiant face's permission carries a minimum price as it would be paid (`param(ctx, "minCost")`), so
//! a card that would cost less is not offered. It ends when the card leaves the field.
//!
//! THE BASE REPLACEMENT is `Script.replacements` (B5 E5): at the "would go to a graveyard" point, a
//! card its controller owns goes to exile instead — a Spell played from the graveyard is exiled after it
//! resolves, a destroyed Unit is exiled. R393 has the Cry's own discard land in the graveyard before
//! the Aura starts: the card is on the field while its Cry runs, so the Cry marks itself as running
//! (`remember`) around the discard and the replacement declines while the mark is on. A Second Wind
//! that arrives without its Cry (a Recruit, a summon) has no mark, so its Aura is on from the start.

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-028";

/// The instance-memory mark the Cry holds while its own discard lands (R393).
const CRY_RUNNING: &str = "secondWindCry";

/// TS `recalled({ self: ctx.self, data: {} }, key)` (engine/src/query.ts), for the replacement's
/// `when`, which is a pure read with no `EffectContext` to hand `query::recalled`. With an empty data
/// bag there is no fused part path, so `work.partMemoryKey` answers the key itself and the read is the
/// card's own memory under it.
fn recalled_on<'a>(card: &'a CardInstance, key: &str) -> Option<&'a Value> {
    card.memory.get(key)
}

/// Both faces' Cry: exile the deck, then discard the hand with the R393 mark held around it.
fn cry() -> Hook {
    hook(|_ctx| {
        vec![
            remember(json_as(json!({ "key": CRY_RUNNING, "value": true }))),
            exile_matching(json_as(json!({ "zones": ["library"] }))),
            discard_hand(Default::default()),
            remember(json_as(json!({ "key": CRY_RUNNING, "value": false }))),
        ]
    })
}

/// Both faces' Aura: play from the graveyard at a price of at least the declared `minCost`.
fn graveyard_play() -> GraveyardPlayHook {
    Arc::new(|args| {
        vec![GraveyardPlayPermission {
            min_price: Some(param(&args, "minCost")),
            ..GraveyardPlayPermission::default()
        }]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(cry()),
        graveyard_play: Some(graveyard_play()),
        replacements: vec![ReplacementDef {
            id: "second-wind-exile".into(),
            on: ReplacementMoment::ToGraveyard,
            where_: None,
            when: Some(replacement_when(|ctx| {
                let owned = matches!(ctx.event, ReplacedEvent::ToGraveyard { owner, .. } if *owner == ctx.controller);
                owned && recalled_on(ctx.self_, CRY_RUNNING) != Some(&Value::Bool(true))
            })),
            instead: ReplacementInstead {
                to: Some(InsteadTo::Exile),
                ..ReplacementInstead::default()
            },
            then: None,
            by: None,
        }],
        ..Script::default()
    };

    let radiant = Script {
        cry: Some(cry()),
        graveyard_play: Some(graveyard_play()),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C #28 Second Wind — SPEC §8.6 row 28, BUILD M9 Classic row C 28: "Cry: exile your deck, then
// discard your hand (a discard; C #64 sees it); the discards land in your graveyard before the Aura
// starts, so they are playable (R393); Aura: `legalActions` offers `play` for every card in your
// graveyard, any type, at its cost with its choices and R65's player discounts, its Cry firing and the
// play counting as one from hand; base: cards you own that would go to your graveyard are exiled
// instead (a replacement: a Spell played from there is exiled after it resolves); with no deck every
// draw is fatigue (§2.4); both end when it leaves the field; no event carries a deck position;
// radiant: no exile replacement, and only cards whose price as it would be paid is (1) or more are
// offered, so a (0) Cost card never loops; its tuned number (radiant minimum price) reads through
// `param()` (R386)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const WIND: &str = "classic-028";
    const REPLENISH: &str = "core-010"; // (0) Spell
    const STOCKPILE: &str = "core-005"; // (1) Spell: "Draw 2. Heal your hero 2."
    const ECLIPSE: &str = "core-035"; // (1) Spell: "Deal 3 damage to a target."
    const MR_TOKEN: &str = "core-015"; // (1) Unit 1/1: "Cry: Summon a Rush Token."
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const SHEEPISH: &str = "core-041"; // (1) Trap
    const MANA_WELL: &str = "core-006"; // (3) Field Spell
    const MENACE: &str = "core-019"; // (3) Unit 9/9
    const HIT_JOB: &str = "core-016"; // (3) Spell: destroy target Unit
    const COLLATERAL: &str = "core-034"; // (4) Spell: exile target permanent and a random card from their deck
    const TOE_CRACKER: &str = "classic-006"; // (2) Unit: "Aura: Your Traps cost (0)."
    const RUSH_TOKEN: &str = "core-t-rush";

    /// What `reduce` answered a graveyard play: the state and the events it made.
    struct Played {
        state: GameState,
        events: Vec<GameEvent>,
    }

    fn plays_of_card(s: &Scenario, instance_id: &str, player: PlayerId) -> Vec<ActionBody> {
        legal_actions(s.state(), player)
            .into_iter()
            .filter(|action| matches!(action, ActionBody::Play { instance_id: id, .. } if id == instance_id))
            .collect()
    }

    fn graveyard_defs_offered(s: &Scenario) -> Vec<String> {
        let offered: IndexSet<String> = legal_actions(s.state(), P1)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Play { instance_id, .. } => Some(instance_id),
                _ => None,
            })
            .collect();
        s.pile(P1, "graveyard")
            .into_iter()
            .filter(|card| offered.contains(&card.id))
            .map(|card| card.def_id)
            .collect()
    }

    /// A play of a graveyard card (B5 E11), sent to `reduce` as `legalActions` offers it: the harness's
    /// `play` takes a card from a hand, so a graveyard play is reduced here and read back off its result.
    ///
    /// `extra` is TS's `Partial<Play>`, spread over the action's literal.
    fn play_from_graveyard(s: &Scenario, card: &CardInstance, extra: Value) -> Played {
        let mut action = json!({ "type": "play", "playerId": "p1", "instanceId": card.id });
        if let (Some(into), Some(more)) = (action.as_object_mut(), extra.as_object()) {
            for (key, value) in more {
                into.insert(key.clone(), value.clone());
            }
        }
        action["nonce"] = json!(format!("graveyard-{}", card.id));
        let result = reduce(s.state(), &json_as::<Action>(action));
        if let Some(error) = result.error {
            panic!("{error}");
        }
        Played {
            state: result.state,
            events: result.events,
        }
    }

    fn zone_of(state: &GameState, card: &CardInstance) -> Option<ZoneName> {
        find_instance(state, &card.id).map(|found| found.zone.z())
    }

    fn defs_of(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.def_id.clone()).collect()
    }

    /// Second Wind standing, with a graveyard to play from. The Radiant face is set up standing (its Cry
    /// not run). The base face's "would go to your graveyard" replacement would exile any card a setup put
    /// in the graveyard once it stands, so there the graveyard comes the way the card makes it: Second Wind
    /// is played and its Cry discards the hand, which lands before the Aura starts (R393).
    ///
    /// `extra` is TS's `{ hand?, field?, mana? }`.
    fn standing(radiant_face: bool, graveyard: &[&str], extra: Value) -> Scenario {
        let p2 = json!({ "hand": [STOCKPILE, HIT_JOB, COLLATERAL], "field": [MENACE], "library": [STOCKPILE, STOCKPILE] });
        let field = extra.get("field").cloned().unwrap_or_else(|| json!([]));
        if !radiant_face && !graveyard.is_empty() {
            let mut hand = vec![WIND];
            hand.extend_from_slice(graveyard);
            let mut p1 = json!({ "hand": hand, "field": field, "library": [VANILLA, VANILLA, VANILLA] });
            if let Some(mana) = extra.get("mana") {
                p1["mana"] = mana.clone();
            }
            let mut s = scenario(json!({ "p1": p1, "p2": p2 }));
            s.play(WIND, json!({ "zone": 1 }));
            return s;
        }
        let hand = extra.get("hand").cloned().unwrap_or_else(|| json!([STOCKPILE]));
        let mut p1 = json!({
            "hand": hand,
            "backrow": [{ "def": WIND, "radiant": radiant_face }],
            "field": field,
            "graveyard": graveyard,
            "library": [VANILLA, VANILLA, VANILLA],
        });
        if let Some(mana) = extra.get("mana") {
            p1["mana"] = mana.clone();
        }
        scenario(json!({ "p1": p1, "p2": p2 }))
    }

    mod c_n28_second_wind {
        use super::*;

        #[test]
        fn declares_its_one_number_the_radiant_face_s_minimum_price_r386() {
            crate::register_all();
            let def = crate::card_def(WIND);
            assert_eq!(
                serde_json::to_value(&def.params).expect("params serialise"),
                json!([{ "key": "minCost", "base": 1, "radiant": 1, "better": "down", "step": 1, "min": 1 }])
            );
            let scripts = script();
            let moments: Vec<ReplacementMoment> = scripts.base.replacements.iter().map(|entry| entry.on).collect();
            assert_eq!(moments, vec![ReplacementMoment::ToGraveyard]);
            assert!(scripts.radiant.replacements.is_empty());
        }

        mod base {
            use super::*;

            #[test]
            fn cry_exiles_your_whole_deck_then_discards_your_hand() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [WIND, ECLIPSE, MR_TOKEN], "library": [VANILLA, MENACE, STOCKPILE] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                let library = s.pile(P1, "library");
                s.play(WIND, json!({ "zone": 1 }));
                assert!(s.pile(P1, "library").is_empty());
                for card in &library {
                    s.expect_in_zone(card, "exile");
                }
                assert!(s.hand(P1).is_empty());
                assert_eq!(
                    s.events().iter().filter(|event| matches!(event, GameEvent::Discarded { .. })).count(),
                    2
                );
                s.expect_events(json!(["exiled", "discarded"]));
            }

            #[test]
            fn r393_the_discards_land_in_your_graveyard_before_the_aura_starts_so_they_are_playable() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [WIND, ECLIPSE, MR_TOKEN], "library": [VANILLA] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                s.play(WIND, json!({ "zone": 1 }));
                assert_eq!(defs_of(&s.pile(P1, "graveyard")), vec![ECLIPSE, MR_TOKEN]);
                assert_eq!(graveyard_defs_offered(&s), vec![ECLIPSE, MR_TOKEN]);
            }

            #[test]
            fn e11_aura_legalactions_offers_every_card_in_your_graveyard_any_type_at_its_cost_with_its_choices() {
                crate::register_all();
                let s = standing(false, &[STOCKPILE, MR_TOKEN, SHEEPISH, MANA_WELL, ECLIPSE, MENACE], json!({}));
                assert_eq!(
                    graveyard_defs_offered(&s),
                    vec![STOCKPILE, MR_TOKEN, SHEEPISH, MANA_WELL, ECLIPSE, MENACE]
                );
                // A targeted Spell carries its targets, each hero and each unit.
                let eclipse = s
                    .pile(P1, "graveyard")
                    .into_iter()
                    .find(|card| card.def_id == ECLIPSE)
                    .expect("setup");
                let targets: Vec<Option<Vec<Selection>>> = plays_of_card(&s, &eclipse.id, P1)
                    .into_iter()
                    .map(|action| match action {
                        ActionBody::Play { targets, .. } => targets,
                        _ => None,
                    })
                    .collect();
                assert!(targets.contains(&Some(vec![Selection::Hero { player: P2 }])));
            }

            #[test]
            fn r1_a_unit_played_from_the_graveyard_fires_its_cry_pays_its_cost_and_counts_as_played() {
                crate::register_all();
                let s = standing(false, &[MR_TOKEN], json!({}));
                let unit = s.card(MR_TOKEN).clone();
                let after = play_from_graveyard(&s, &unit, json!({}));
                assert_eq!(
                    card_at(&after.state, ZoneRef { player: P1, row: Row::Units, lane: 1 }).map(|card| card.id.clone()),
                    Some(unit.id.clone())
                );
                assert_eq!(
                    card_at(&after.state, ZoneRef { player: P1, row: Row::Units, lane: 2 }).map(|card| card.def_id.clone()),
                    Some(RUSH_TOKEN.to_string())
                );
                assert_eq!(unspent_mana_of(&after.state, P1), 3);
                let played = after
                    .events
                    .iter()
                    .find(|event| matches!(event, GameEvent::CardPlayed { instance_id, .. } if *instance_id == unit.id))
                    .expect("the play is announced");
                let played = serde_json::to_value(played).expect("events serialise");
                assert_eq!(played["from"], json!("graveyard"));
                assert_eq!(played["costPaid"], json!(1));
            }

            #[test]
            fn r65_a_play_from_the_graveyard_takes_the_player_s_discounts_but_never_below_the_1_minimum_price() {
                crate::register_all();
                let s = standing(false, &[SHEEPISH], json!({ "field": [TOE_CRACKER] }));
                let trap = s.card(SHEEPISH).clone();
                // The discount takes it to (0): below the minimum price, so nothing is offered and the play is refused.
                assert!(plays_of_card(&s, &trap.id, P1).is_empty());
                expect_throw(|| {
                    play_from_graveyard(&s, &trap, json!({ "zone": { "row": "backrow", "lane": 2 } }));
                });
            }

            #[test]
            fn a_card_it_can_t_afford_is_not_offered() {
                crate::register_all();
                let s = standing(false, &[MENACE], json!({ "mana": 2 }));
                assert!(graveyard_defs_offered(&s).is_empty());
            }

            #[test]
            fn r3_with_no_deck_every_draw_is_fatigue() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [WIND, ECLIPSE], "library": [VANILLA, VANILLA] },
                    "p2": { "hand": [STOCKPILE, STOCKPILE], "library": [STOCKPILE, STOCKPILE] },
                }));
                s.play(WIND, json!({ "zone": 1 }));
                s.end_turn();
                s.end_turn();
                assert_eq!(s.state().active, P1);
                s.expect_health(P1, 29);
                assert!(s.events().iter().any(|event| matches!(event, GameEvent::Fatigue { .. })));
            }

            #[test]
            fn its_aura_ends_when_it_leaves_the_field_nothing_in_the_graveyard_is_offered_any_more() {
                crate::register_all();
                let mut s = standing(false, &[STOCKPILE, VANILLA], json!({}));
                assert_eq!(graveyard_defs_offered(&s), vec![STOCKPILE, VANILLA]);
                s.end_turn();
                let wind = s.card(WIND).id.clone();
                s.play(COLLATERAL, json!({ "targets": [{ "pick": "instance", "instanceId": wind }] }));
                s.expect_in_zone(WIND, "exile");
                s.end_turn();
                assert!(graveyard_defs_offered(&s).is_empty());
            }

            #[test]
            fn no_event_carries_a_deck_position() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [WIND, ECLIPSE], "library": [VANILLA, MENACE, STOCKPILE] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                s.play(WIND, json!({ "zone": 1 }));
                let exiled: Vec<&GameEvent> = s
                    .events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::Exiled { .. }))
                    .collect();
                assert_eq!(exiled.len(), 3);
                for event in exiled {
                    let json = serde_json::to_value(event).expect("events serialise");
                    let mut keys: Vec<String> = json.as_object().expect("an object").keys().cloned().collect();
                    keys.sort();
                    assert_eq!(keys, vec!["defId", "instanceId", "owner", "type"]);
                }
                for viewer in [P1, P2] {
                    for event in &s.view(viewer).events {
                        let json = serde_json::to_value(event).expect("events serialise");
                        assert!(json.get("position").is_none());
                    }
                }
            }

            #[test]
            fn e5_a_spell_played_from_your_graveyard_is_exiled_after_it_resolves() {
                crate::register_all();
                let s = standing(false, &[STOCKPILE], json!({}));
                let spell = s.pile(P1, "graveyard").into_iter().next().expect("setup");
                let after = play_from_graveyard(&s, &spell, json!({}));
                assert_eq!(zone_of(&after.state, &spell), Some(ZoneName::Exile));
            }

            #[test]
            fn e5_a_card_of_yours_that_would_go_to_your_graveyard_is_exiled_instead_a_discard_a_destroyed_unit() {
                crate::register_all();
                let mut s = standing(false, &[], json!({ "hand": [STOCKPILE], "field": [VANILLA] }));
                let vanilla = s.card(VANILLA).clone();
                s.end_turn();
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": vanilla.id }] }));
                s.expect_in_zone(&vanilla, "exile");
                assert!(s.pile(P1, "graveyard").is_empty());
            }

            #[test]
            fn e5_the_replacement_ends_when_it_leaves_the_field_your_destroyed_unit_goes_to_your_graveyard_again() {
                crate::register_all();
                let mut s = standing(false, &[], json!({ "field": [VANILLA] }));
                let vanilla = s.card(VANILLA).clone();
                s.end_turn();
                let wind = s.card(WIND).id.clone();
                s.play(COLLATERAL, json!({ "targets": [{ "pick": "instance", "instanceId": wind }] }));
                s.expect_in_zone(WIND, "exile");
                s.end_turn();
                s.end_turn();
                assert_eq!(s.state().active, P2);
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": vanilla.id }] }));
                s.expect_in_zone(&vanilla, "graveyard");
            }

            #[test]
            fn e5_the_opponent_s_cards_go_to_their_graveyard_as_usual() {
                crate::register_all();
                let mut s = standing(false, &[], json!({ "field": [VANILLA] }));
                s.end_turn();
                let hit_job = s.card(HIT_JOB).clone();
                let vanilla = s.card(VANILLA).id.clone();
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": vanilla }] }));
                s.expect_in_zone(&hit_job, "graveyard");
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn the_same_cry_exiles_your_deck_discards_your_hand_into_your_graveyard() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": WIND, "radiant": true }, ECLIPSE, MR_TOKEN], "library": [VANILLA, MENACE] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                s.play(WIND, json!({ "zone": 1 }));
                assert!(s.pile(P1, "library").is_empty());
                assert_eq!(defs_of(&s.pile(P1, "exile")), vec![VANILLA, MENACE]);
                assert_eq!(defs_of(&s.pile(P1, "graveyard")), vec![ECLIPSE, MR_TOKEN]);
            }

            #[test]
            fn only_cards_whose_price_as_it_would_be_paid_is_1_or_more_are_offered() {
                crate::register_all();
                let s = standing(true, &[REPLENISH, STOCKPILE, VANILLA, MANA_WELL], json!({}));
                assert_eq!(graveyard_defs_offered(&s), vec![STOCKPILE, VANILLA, MANA_WELL]);
            }

            #[test]
            fn the_price_is_read_as_it_would_be_paid_a_1_trap_toe_cracker_makes_0_is_not_offered() {
                crate::register_all();
                let s = standing(true, &[SHEEPISH, STOCKPILE], json!({ "field": [TOE_CRACKER] }));
                assert_eq!(graveyard_defs_offered(&s), vec![STOCKPILE]);
            }

            #[test]
            fn no_exile_replacement_a_spell_played_from_the_graveyard_lands_in_it_again() {
                crate::register_all();
                let s = standing(true, &[STOCKPILE], json!({}));
                let spell = s.pile(P1, "graveyard").into_iter().next().expect("setup");
                let after = play_from_graveyard(&s, &spell, json!({}));
                assert_eq!(zone_of(&after.state, &spell), Some(ZoneName::Graveyard));
                // And the (1) Stockpile is offered again: paid (1) each time, it is no free loop.
                let graveyard: Vec<String> = zone_cards(&after.state, P1, OffFieldZone::Graveyard)
                    .iter()
                    .map(|card| card.id.clone())
                    .collect();
                assert_eq!(graveyard, vec![spell.id.clone()]);
                assert!(
                    legal_actions(&after.state, P1)
                        .iter()
                        .any(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == spell.id))
                );
            }

            #[test]
            fn r1_a_unit_played_from_the_graveyard_fires_its_cry() {
                crate::register_all();
                let s = standing(true, &[MR_TOKEN], json!({}));
                let unit = s.card(MR_TOKEN).clone();
                let after = play_from_graveyard(&s, &unit, json!({}));
                assert_eq!(
                    card_at(&after.state, ZoneRef { player: P1, row: Row::Units, lane: 2 }).map(|card| card.def_id.clone()),
                    Some(RUSH_TOKEN.to_string())
                );
            }

            #[test]
            fn r386_a_degrade_of_the_minimum_price_makes_it_2_a_1_cost_card_is_no_longer_offered() {
                crate::register_all();
                let mut s = standing(true, &[STOCKPILE, MANA_WELL], json!({}));
                step_param(s.card_mut(WIND), "minCost", 1);
                assert_eq!(graveyard_defs_offered(&s), vec![MANA_WELL]);
            }
        }
    }
}
